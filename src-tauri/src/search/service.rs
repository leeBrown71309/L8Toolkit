use std::cmp::Reverse;
use std::sync::{Mutex, RwLock};

use super::apps::AppEntry;
use super::calculator;
use super::files::{display_parent, FileEntry};
use super::matching::{initials_of, name_bonus, FuzzyQuery};
use super::system_actions::{SystemAction, SYSTEM_ACTIONS};
use super::types::{ResultKind, SearchResult, SearchSection, SectionId};
use super::web;
use crate::i18n::{tr, tr_with, Language, Text};
use crate::settings::WebSearchEngine;

const MAX_APPS: usize = 6;
const MAX_COMMANDS: usize = 3;
const MAX_FOLDERS: usize = 4;
const MAX_FILES: usize = 6;
/// Single-letter file queries match half the disk and are never useful.
const MIN_FILE_QUERY_CHARS: usize = 2;
/// Files compete with apps for the top hit, but apps win on equal relevance.
const FILE_SCORE_PERCENT: u32 = 75;
const KEYWORD_SCORE_PERCENT: u32 = 50;
const POINTS_PER_LAUNCH: u32 = 8;
const MAX_COUNTED_LAUNCHES: u32 = 25;
const PINNED_BONUS: u32 = 60;
const TOP_HIT_MIN_SCORE: u32 = 60;

/// What the launch history knows about a target.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UsageInfo {
    pub launches: u32,
    pub pinned: bool,
}

/// Injected so ranking stays testable without the history store.
pub type UsageLookup<'a> = &'a dyn Fn(&str) -> UsageInfo;

struct Candidate {
    score: u32,
    result: SearchResult,
}

#[derive(Default)]
struct IndexingState {
    running: bool,
    rerun_requested: bool,
}

#[derive(Default)]
pub struct SearchService {
    apps: RwLock<Vec<AppEntry>>,
    files: RwLock<Vec<FileEntry>>,
    file_indexing: Mutex<IndexingState>,
}

impl SearchService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn replace_apps(&self, apps: Vec<AppEntry>) {
        *self.apps.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = apps;
    }

    pub fn replace_files(&self, files: Vec<FileEntry>) {
        *self.files.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = files;
    }

    /// Finds the installed app behind a media session's AppUserModelID
    /// ("Spotify.exe", "Chrome", "Microsoft.ZuneMusic_...!Microsoft.ZuneMusic").
    /// Returns its display name and launch target.
    pub fn find_app_for_source(&self, source_app_id: &str) -> Option<(String, String)> {
        let apps = self.apps.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        let id = source_app_id.to_lowercase();
        let apps_folder_target = format!("shell:appsfolder\\{id}");
        let executable = if id.ends_with(".exe") { id.clone() } else { format!("{id}.exe") };
        let stem = id.split(['.', '!', '_']).next().unwrap_or(&id);

        let by_target = || apps.iter().find(|app| app.launch_target.to_lowercase() == apps_folder_target);
        let by_executable = || {
            apps.iter().find(|app| {
                app.location.as_deref().and_then(|location| std::path::Path::new(location).file_name())
                    .is_some_and(|name| name.to_string_lossy().to_lowercase() == executable)
            })
        };
        let by_name = || (stem.len() >= 3).then(|| apps.iter().find(|app| app.name_lowercase.contains(stem))).flatten();

        by_target()
            .or_else(by_executable)
            .or_else(by_name)
            .map(|app| (app.name.clone(), app.launch_target.clone()))
    }

    /// Drops a deleted path (and its children) without waiting for a reindex.
    pub fn forget_path(&self, path: &str) {
        let prefix = format!("{path}\\");
        self.files
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|entry| entry.path != path && !entry.path.starts_with(&prefix));
    }

    pub fn app_count(&self) -> usize {
        self.apps.read().map(|apps| apps.len()).unwrap_or(0)
    }

    pub fn file_count(&self) -> usize {
        self.files.read().map(|files| files.len()).unwrap_or(0)
    }

    pub fn is_indexing_files(&self) -> bool {
        self.file_indexing.lock().map(|state| state.running).unwrap_or(false)
    }

    /// Returns false (and queues a rerun) when an indexing pass is running.
    pub fn try_begin_file_indexing(&self) -> bool {
        let mut state = self.file_indexing.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.running {
            state.rerun_requested = true;
            return false;
        }
        state.running = true;
        true
    }

    /// Returns true when another pass was requested meanwhile and must run now.
    pub fn finish_file_indexing(&self) -> bool {
        let mut state = self.file_indexing.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.rerun_requested {
            state.rerun_requested = false;
            return true;
        }
        state.running = false;
        false
    }

    pub fn search(
        &self,
        raw_query: &str,
        usage: UsageLookup,
        engine: WebSearchEngine,
        language: Language,
    ) -> Vec<SearchSection> {
        let query = raw_query.trim();
        if query.is_empty() {
            return Vec::new();
        }
        let mut fuzzy = FuzzyQuery::new(query);
        let mut sections = Vec::new();

        if let Some(value) = calculator::evaluate(query) {
            sections.push(section(SectionId::Calculator, vec![calculator_result(query, value)]));
        }

        let mut apps = self.rank_apps(&mut fuzzy, usage, language);
        let mut commands = rank_commands(&mut fuzzy, usage, language);
        let (mut folders, mut files) = self.rank_files(&mut fuzzy, usage);

        if let Some(top_hit) = take_top_hit([&mut apps, &mut commands, &mut folders, &mut files]) {
            sections.push(section(SectionId::TopHit, vec![top_hit]));
        }
        for (id, candidates) in [
            (SectionId::Applications, apps),
            (SectionId::Commands, commands),
            (SectionId::Folders, folders),
            (SectionId::Files, files),
        ] {
            if !candidates.is_empty() {
                sections.push(section(id, candidates.into_iter().map(|candidate| candidate.result).collect()));
            }
        }

        sections.push(section(SectionId::Web, vec![web_result(query, engine, language)]));
        sections
    }

    fn rank_apps(&self, fuzzy: &mut FuzzyQuery, usage: UsageLookup, language: Language) -> Vec<Candidate> {
        let apps = self.apps.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        let scored: Vec<(u32, usize)> = apps
            .iter()
            .enumerate()
            .filter_map(|(index, app)| {
                let base = fuzzy.score(app.searchable_name.slice(..))?;
                let bonus = name_bonus(&app.name_lowercase, &app.initials, &fuzzy.lowercase);
                Some((base + bonus + usage_points(usage(&app.launch_target)), index))
            })
            .collect();

        top_k(scored, MAX_APPS)
            .into_iter()
            .map(|(score, index)| {
                let app = &apps[index];
                let mut result = app_result(app, fuzzy.indices(&app.name), language);
                result.pinned = usage(&app.launch_target).pinned;
                Candidate { score, result }
            })
            .collect()
    }

    fn rank_files(&self, fuzzy: &mut FuzzyQuery, usage: UsageLookup) -> (Vec<Candidate>, Vec<Candidate>) {
        if fuzzy.lowercase.chars().count() < MIN_FILE_QUERY_CHARS {
            return (Vec::new(), Vec::new());
        }
        let files = self.files.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut folder_scores = Vec::new();
        let mut file_scores = Vec::new();
        for (index, entry) in files.iter().enumerate() {
            let Some(base) = fuzzy.score(entry.searchable_name.slice(..)) else { continue };
            let initials = initials_of(&entry.name);
            let raw = base + name_bonus(&entry.name_lowercase, &initials, &fuzzy.lowercase);
            let score = raw * FILE_SCORE_PERCENT / 100 + usage_points(usage(&entry.path));
            if entry.is_directory {
                folder_scores.push((score, index));
            } else {
                file_scores.push((score, index));
            }
        }

        let mut build = |scores: Vec<(u32, usize)>, limit: usize| -> Vec<Candidate> {
            top_k(scores, limit)
                .into_iter()
                .map(|(score, index)| {
                    let entry = &files[index];
                    let mut result = file_result(entry, fuzzy.indices(&entry.name));
                    result.pinned = usage(&entry.path).pinned;
                    Candidate { score, result }
                })
                .collect()
        };
        let folders = build(folder_scores, MAX_FOLDERS);
        let files_found = build(file_scores, MAX_FILES);
        (folders, files_found)
    }
}

/// Commands are shown in the current language but answer to words from
/// every language, so "lock" and "verrouiller" both find Lock Screen.
fn rank_commands(fuzzy: &mut FuzzyQuery, usage: UsageLookup, language: Language) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = SYSTEM_ACTIONS
        .iter()
        .filter_map(|action| {
            let title = tr(language, action.title);
            let all_languages_words = [Language::En, Language::Fr]
                .map(|other| format!("{} {}", tr(other, action.title), tr(other, action.keywords)))
                .join(" ");
            let title_score = fuzzy.score_text(title);
            let keyword_score =
                fuzzy.score_text(&all_languages_words).map(|score| score * KEYWORD_SCORE_PERCENT / 100);
            let base = title_score.into_iter().chain(keyword_score).max()?;
            let bonus = name_bonus(&title.to_lowercase(), &initials_of(title), &fuzzy.lowercase);
            let score = base + bonus + usage_points(usage(action.id));
            let indices = if title_score.is_some() { fuzzy.indices(title) } else { Vec::new() };
            Some(Candidate { score, result: command_result(action, indices, language) })
        })
        .collect();
    candidates.sort_by_key(|candidate| Reverse(candidate.score));
    candidates.truncate(MAX_COMMANDS);
    candidates
}

/// Keeps the `limit` best (score, index) pairs, best first, without sorting
/// every match (a two-letter query can match tens of thousands of files).
fn top_k(mut scored: Vec<(u32, usize)>, limit: usize) -> Vec<(u32, usize)> {
    let by_score_desc = |left: &(u32, usize), right: &(u32, usize)| right.0.cmp(&left.0).then(left.1.cmp(&right.1));
    if scored.len() > limit {
        scored.select_nth_unstable_by(limit, by_score_desc);
        scored.truncate(limit);
    }
    scored.sort_by(by_score_desc);
    scored
}

/// Moves the single most relevant candidate across all categories out of its
/// section so it can be shown first as the "Top hit".
fn take_top_hit<const N: usize>(categories: [&mut Vec<Candidate>; N]) -> Option<SearchResult> {
    // On equal scores the earlier category wins (apps before files).
    let (_, best_category) = categories
        .iter()
        .enumerate()
        .filter_map(|(index, candidates)| candidates.first().map(|first| (first.score, index)))
        .filter(|(score, _)| *score >= TOP_HIT_MIN_SCORE)
        .max_by(|left, right| left.0.cmp(&right.0).then(right.1.cmp(&left.1)))?;
    Some(categories[best_category].remove(0).result)
}

fn usage_points(info: UsageInfo) -> u32 {
    let pinned_bonus = if info.pinned { PINNED_BONUS } else { 0 };
    info.launches.min(MAX_COUNTED_LAUNCHES) * POINTS_PER_LAUNCH + pinned_bonus
}

/// Fields shared by every result; constructors fill in the rest.
fn base_result(id: String, kind: ResultKind, title: String, subtitle: String, target: String) -> SearchResult {
    SearchResult {
        id,
        kind,
        title,
        subtitle,
        target,
        icon_target: None,
        location: None,
        pinned: false,
        match_indices: Vec::new(),
    }
}

fn section(id: SectionId, items: Vec<SearchResult>) -> SearchSection {
    SearchSection { id, items }
}

fn app_result(app: &AppEntry, match_indices: Vec<u32>, language: Language) -> SearchResult {
    let subtitle = tr(language, if app.location.is_some() { Text::ResultApplication } else { Text::ResultStoreApp });
    SearchResult {
        icon_target: Some(app.launch_target.clone()),
        location: app.location.clone(),
        match_indices,
        ..base_result(
            format!("app:{}", app.launch_target),
            ResultKind::App,
            app.name.clone(),
            subtitle.to_string(),
            app.launch_target.clone(),
        )
    }
}

fn file_result(entry: &FileEntry, match_indices: Vec<u32>) -> SearchResult {
    SearchResult {
        icon_target: Some(entry.path.clone()),
        location: Some(entry.path.clone()),
        match_indices,
        ..base_result(
            format!("file:{}", entry.path),
            if entry.is_directory { ResultKind::Folder } else { ResultKind::File },
            entry.name.clone(),
            display_parent(&entry.path),
            entry.path.clone(),
        )
    }
}

fn command_result(action: &SystemAction, match_indices: Vec<u32>, language: Language) -> SearchResult {
    SearchResult {
        match_indices,
        ..base_result(
            format!("command:{}", action.id),
            ResultKind::Command,
            tr(language, action.title).to_string(),
            tr(language, action.subtitle).to_string(),
            action.id.to_string(),
        )
    }
}

fn calculator_result(query: &str, value: f64) -> SearchResult {
    let formatted = calculator::format_number(value);
    base_result(
        "calculator".to_string(),
        ResultKind::Calculator,
        formatted.clone(),
        format!("{} =", query.trim_start_matches('=').trim()),
        formatted,
    )
}

fn web_result(query: &str, engine: WebSearchEngine, language: Language) -> SearchResult {
    match web::normalize_url(query) {
        Some(url) => base_result(
            "url".to_string(),
            ResultKind::Url,
            query.to_string(),
            tr(language, Text::ResultOpenInBrowser).to_string(),
            url,
        ),
        None => base_result(
            "web".to_string(),
            ResultKind::Web,
            query.to_string(),
            tr_with(language, Text::ResultSearchWith, &[("engine", web::engine_label(engine))]),
            web::search_url(engine, query),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::InstalledApp;
    use crate::search::apps::build_app_entries;

    fn service_with_apps(names: &[&str]) -> SearchService {
        let service = SearchService::new();
        let installed = names
            .iter()
            .map(|name| InstalledApp {
                name: name.to_string(),
                launch_target: format!("shell:AppsFolder\\{name}"),
                location: None,
            })
            .collect();
        service.replace_apps(build_app_entries(installed));
        service
    }

    fn no_launches(_: &str) -> UsageInfo {
        UsageInfo::default()
    }

    #[test]
    fn acronym_query_puts_the_app_in_top_hit() {
        let service = service_with_apps(&["Visual Studio Code", "Visual Studio Installer", "Notepad"]);
        let sections = service.search("vsc", &no_launches, WebSearchEngine::Google, Language::En);
        assert_eq!(sections[0].id, SectionId::TopHit);
        assert_eq!(sections[0].items[0].title, "Visual Studio Code");
        assert_eq!(sections.last().unwrap().id, SectionId::Web);
    }

    #[test]
    fn calculations_come_first() {
        let service = service_with_apps(&["Calculator"]);
        let sections = service.search("12*4", &no_launches, WebSearchEngine::Google, Language::En);
        assert_eq!(sections[0].id, SectionId::Calculator);
        assert_eq!(sections[0].items[0].title, "48");
    }

    #[test]
    fn usage_breaks_ties() {
        let service = service_with_apps(&["Spotify", "Speedtest"]);
        let counter = |target: &str| UsageInfo { launches: if target.ends_with("Speedtest") { 10 } else { 0 }, pinned: false };
        let sections = service.search("sp", &counter, WebSearchEngine::Google, Language::En);
        assert_eq!(sections[0].items[0].title, "Speedtest");
    }

    #[test]
    fn pinned_items_rank_higher_and_are_flagged() {
        let service = service_with_apps(&["Spotify", "Speedtest"]);
        let lookup = |target: &str| UsageInfo { launches: 0, pinned: target.ends_with("Spotify") };
        let sections = service.search("sp", &lookup, WebSearchEngine::Google, Language::En);
        assert_eq!(sections[0].items[0].title, "Spotify");
        assert!(sections[0].items[0].pinned);
    }

    #[test]
    fn media_sources_resolve_to_installed_apps() {
        let service = SearchService::new();
        service.replace_apps(build_app_entries(vec![
            InstalledApp {
                name: "Spotify".into(),
                launch_target: "shell:AppsFolder\\Spotify".into(),
                location: Some("C:\\Users\\me\\AppData\\Roaming\\Spotify\\Spotify.exe".into()),
            },
            InstalledApp { name: "Google Chrome".into(), launch_target: "shell:AppsFolder\\Chrome".into(), location: None },
        ]));
        assert_eq!(service.find_app_for_source("Spotify.exe").map(|app| app.0), Some("Spotify".into()));
        assert_eq!(service.find_app_for_source("Chrome").map(|app| app.0), Some("Google Chrome".into()));
        assert_eq!(service.find_app_for_source("308046B0AF4A39CB"), None);
    }

    #[test]
    fn forgetting_a_folder_drops_its_children() {
        use crate::search::files::index_roots;
        let root = std::env::temp_dir().join("l8-forget-path-test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("old")).unwrap();
        std::fs::write(root.join("old").join("a.txt"), "x").unwrap();
        std::fs::write(root.join("keep.txt"), "x").unwrap();
        let service = SearchService::new();
        service.replace_files(index_roots(std::slice::from_ref(&root), 100));
        service.forget_path(&root.join("old").to_string_lossy());
        assert_eq!(service.file_count(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_query_returns_nothing() {
        let service = service_with_apps(&["Notepad"]);
        assert!(service.search("   ", &no_launches, WebSearchEngine::Google, Language::En).is_empty());
    }

    #[test]
    fn commands_answer_to_both_languages() {
        let service = SearchService::new();
        let in_french = service.search("lock", &no_launches, WebSearchEngine::Google, Language::Fr);
        assert_eq!(in_french[0].items[0].title, "Verrouiller l'écran");
        let in_english = service.search("verrouiller", &no_launches, WebSearchEngine::Google, Language::En);
        assert_eq!(in_english[0].items[0].title, "Lock Screen");
    }

    #[test]
    fn indexing_queues_a_single_rerun() {
        let service = SearchService::new();
        assert!(service.try_begin_file_indexing());
        assert!(!service.try_begin_file_indexing());
        assert!(service.finish_file_indexing());
        assert!(!service.finish_file_indexing());
        assert!(!service.is_indexing_files());
    }

    #[test]
    fn top_k_keeps_best_scores_in_order() {
        let picked = top_k(vec![(5, 0), (9, 1), (1, 2), (7, 3)], 2);
        assert_eq!(picked, vec![(9, 1), (7, 3)]);
    }
}
