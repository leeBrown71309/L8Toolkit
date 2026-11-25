//! Launch history and pins: boosts what the user opens often and fills the
//! "Frequent" row shown before anything is typed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::json_file;
use crate::search::{ResultKind, SearchResult, UsageInfo};

const SECONDS_PER_WEEK: f64 = 7.0 * 24.0 * 3600.0;
/// Old entries are dropped so the file never grows without bound.
const MAX_STORED_ENTRIES: usize = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageEntry {
    pub result: SearchResult,
    pub count: u32,
    pub last_used: u64,
    #[serde(default)]
    pub pinned_at: Option<u64>,
}

impl UsageEntry {
    fn info(&self) -> UsageInfo {
        UsageInfo { launches: self.count, pinned: self.pinned_at.is_some() }
    }
}

pub struct UsageStore {
    path: PathBuf,
    entries: RwLock<HashMap<String, UsageEntry>>,
}

/// Only things that live on the computer can sit in the Frequent row.
pub fn is_pinnable(kind: ResultKind) -> bool {
    matches!(kind, ResultKind::App | ResultKind::File | ResultKind::Folder)
}

impl UsageStore {
    pub fn load(path: PathBuf) -> Self {
        let entries = json_file::read_or_default::<HashMap<String, UsageEntry>>(&path);
        Self { path, entries: RwLock::new(entries) }
    }

    #[cfg(test)]
    pub fn launch_count(&self, target: &str) -> u32 {
        self.entries
            .read()
            .map(|entries| entries.get(target).map(|entry| entry.count).unwrap_or(0))
            .unwrap_or(0)
    }

    /// Runs `reader` with a lookup bound to the current history.
    pub fn with_lookup<T>(&self, reader: impl FnOnce(&dyn Fn(&str) -> UsageInfo) -> T) -> T {
        let entries = self.entries.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        let lookup = |target: &str| entries.get(target).map(UsageEntry::info).unwrap_or_default();
        reader(&lookup)
    }

    pub fn record(&self, result: &SearchResult) {
        self.update(result, |entry, now| {
            entry.count = entry.count.saturating_add(1);
            entry.last_used = now;
        });
    }

    pub fn set_pinned(&self, result: &SearchResult, pinned: bool) {
        self.update(result, |entry, now| entry.pinned_at = pinned.then_some(now));
    }

    /// Removes a target from the history entirely ("Remove from Frequent").
    pub fn forget(&self, target: &str) {
        let snapshot = {
            let mut entries = self.entries.write().unwrap_or_else(|poisoned| poisoned.into_inner());
            entries.remove(target);
            entries.clone()
        };
        self.save(&snapshot);
    }

    fn update(&self, result: &SearchResult, change: impl FnOnce(&mut UsageEntry, u64)) {
        let snapshot = {
            let mut entries = self.entries.write().unwrap_or_else(|poisoned| poisoned.into_inner());
            let now = unix_seconds();
            let entry = entries.entry(result.target.clone()).or_insert_with(|| UsageEntry {
                result: result.clone(),
                count: 0,
                last_used: now,
                pinned_at: None,
            });
            change(entry, now);
            entry.result = SearchResult { match_indices: Vec::new(), pinned: false, ..result.clone() };
            prune(&mut entries, now);
            entries.clone()
        };
        self.save(&snapshot);
    }

    fn save(&self, snapshot: &HashMap<String, UsageEntry>) {
        if let Err(error) = json_file::write_atomically(&self.path, snapshot) {
            log::warn!("Cannot save launch history: {error}");
        }
    }

    /// Pinned items first (in pin order), then the most relevant past
    /// launches, skipping files that no longer exist.
    pub fn frequent(&self, limit: usize) -> Vec<SearchResult> {
        let entries = self.entries.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = unix_seconds();
        let mut candidates: Vec<&UsageEntry> = entries
            .values()
            .filter(|entry| is_pinnable(entry.result.kind))
            .filter(|entry| entry.pinned_at.is_some() || entry.count > 0)
            .filter(|entry| entry.result.kind == ResultKind::App || Path::new(&entry.result.target).exists())
            .collect();
        candidates.sort_by(|left, right| match (left.pinned_at, right.pinned_at) {
            (Some(left_pin), Some(right_pin)) => left_pin.cmp(&right_pin),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => frecency(right, now).total_cmp(&frecency(left, now)),
        });
        candidates
            .into_iter()
            .take(limit)
            .map(|entry| SearchResult { pinned: entry.pinned_at.is_some(), ..entry.result.clone() })
            .collect()
    }
}

/// Frequency decayed by recency, so last month's habit fades out.
fn frecency(entry: &UsageEntry, now: u64) -> f64 {
    let weeks_since = now.saturating_sub(entry.last_used) as f64 / SECONDS_PER_WEEK;
    entry.count as f64 / (1.0 + weeks_since)
}

/// Keeps pins forever; drops the least relevant unpinned entries.
fn prune(entries: &mut HashMap<String, UsageEntry>, now: u64) {
    if entries.len() <= MAX_STORED_ENTRIES {
        return;
    }
    let mut removable: Vec<(String, f64)> = entries
        .iter()
        .filter(|(_, entry)| entry.pinned_at.is_none())
        .map(|(key, entry)| (key.clone(), frecency(entry, now)))
        .collect();
    removable.sort_by(|left, right| left.1.total_cmp(&right.1));
    let excess = entries.len() - MAX_STORED_ENTRIES;
    for (key, _) in removable.into_iter().take(excess) {
        entries.remove(&key);
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(target: &str) -> SearchResult {
        SearchResult {
            id: format!("app:{target}"),
            kind: ResultKind::App,
            title: target.to_string(),
            subtitle: String::new(),
            target: target.to_string(),
            icon_target: None,
            location: None,
            pinned: false,
            match_indices: vec![1, 2],
        }
    }

    fn temporary_store(name: &str) -> (UsageStore, PathBuf) {
        let path = std::env::temp_dir().join(name);
        let _ = std::fs::remove_file(&path);
        (UsageStore::load(path.clone()), path)
    }

    #[test]
    fn records_counts_and_orders_frequent_items() {
        let (store, path) = temporary_store("l8-usage-test.json");
        store.record(&app("a"));
        store.record(&app("b"));
        store.record(&app("b"));

        assert_eq!(store.launch_count("b"), 2);
        let frequent = store.frequent(5);
        assert_eq!(frequent[0].target, "b");
        assert!(frequent[0].match_indices.is_empty());

        let reloaded = UsageStore::load(path.clone());
        assert_eq!(reloaded.launch_count("a"), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn pins_come_first_and_can_be_removed() {
        let (store, path) = temporary_store("l8-usage-pin-test.json");
        store.record(&app("busy"));
        store.record(&app("busy"));
        store.set_pinned(&app("never-launched"), true);

        let frequent = store.frequent(5);
        assert_eq!(frequent[0].target, "never-launched");
        assert!(frequent[0].pinned);
        assert!(store.with_lookup(|lookup| lookup("never-launched").pinned));

        store.set_pinned(&app("never-launched"), false);
        assert_eq!(store.frequent(5).len(), 1);
        store.forget("busy");
        assert!(store.frequent(5).is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn recent_use_outranks_stale_habits() {
        let stale = UsageEntry { result: app("old"), count: 10, last_used: 0, pinned_at: None };
        let fresh = UsageEntry { result: app("new"), count: 3, last_used: 100 * 604_800, pinned_at: None };
        assert!(frecency(&fresh, 100 * 604_800) > frecency(&stale, 100 * 604_800));
    }
}
