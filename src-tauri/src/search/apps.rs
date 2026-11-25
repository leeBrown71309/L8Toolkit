//! Application index built from the OS launcher entries.

use std::collections::HashSet;

use nucleo_matcher::Utf32String;

use super::matching::initials_of;
use crate::platform::InstalledApp;

/// Start-menu entries that are not apps people want to launch from search.
const NOISE_NAME_FRAGMENTS: &[&str] = &["uninstall", "readme", "release notes"];
const NOISE_TARGET_EXTENSIONS: &[&str] = &[".chm", ".txt", ".pdf", ".rtf", ".html", ".htm", ".ini"];

pub struct AppEntry {
    pub name: String,
    pub name_lowercase: String,
    pub initials: String,
    pub searchable_name: Utf32String,
    pub launch_target: String,
    pub location: Option<String>,
}

pub fn build_app_entries(installed: Vec<InstalledApp>) -> Vec<AppEntry> {
    let mut seen = HashSet::new();
    let mut entries: Vec<AppEntry> = installed
        .into_iter()
        .filter(|app| !is_noise(app))
        .filter(|app| seen.insert(app.launch_target.to_lowercase()))
        .map(|app| AppEntry {
            name_lowercase: app.name.to_lowercase(),
            initials: initials_of(&app.name),
            searchable_name: Utf32String::from(app.name.as_str()),
            name: app.name,
            launch_target: app.launch_target,
            location: app.location,
        })
        .collect();
    entries.sort_by(|left, right| left.name_lowercase.cmp(&right.name_lowercase));
    entries
}

fn is_noise(app: &InstalledApp) -> bool {
    let name = app.name.to_lowercase();
    let target = app.launch_target.to_lowercase();
    NOISE_NAME_FRAGMENTS.iter().any(|fragment| name.contains(fragment))
        || NOISE_TARGET_EXTENSIONS.iter().any(|extension| target.ends_with(extension))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, target: &str) -> InstalledApp {
        InstalledApp { name: name.into(), launch_target: target.into(), location: None }
    }

    #[test]
    fn filters_noise_and_duplicates() {
        let entries = build_app_entries(vec![
            app("Visual Studio Code", "shell:AppsFolder\\Code"),
            app("Uninstall Foo", "shell:AppsFolder\\foo-uninstall"),
            app("Foo Help", "shell:AppsFolder\\C:\\foo\\help.chm"),
            app("Code duplicate", "shell:AppsFolder\\code"),
        ]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].initials, "vsc");
    }
}
