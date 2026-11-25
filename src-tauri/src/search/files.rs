//! In-memory index of the user's files and folders.
//!
//! We walk the standard user folders instead of querying the Windows Search
//! service: it is predictable, needs no service to be running, and a cap keeps
//! memory bounded (~200k entries is a few tens of MB).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use nucleo_matcher::Utf32String;
use walkdir::{DirEntry, WalkDir};

pub const MAX_INDEXED_ENTRIES: usize = 200_000;
const MAX_DEPTH: usize = 8;

/// Folders that are huge, generated, or never searched by humans.
const SKIPPED_DIRECTORY_NAMES: &[&str] = &[
    "node_modules", "__pycache__", "target", "venv", ".venv", "bower_components", "obj",
    "$recycle.bin", "appdata",
];

pub struct FileEntry {
    pub name: String,
    pub name_lowercase: String,
    pub searchable_name: Utf32String,
    pub path: String,
    pub is_directory: bool,
}

/// Standard user folders plus the configured extra folders, without
/// duplicates or folders already contained in another root.
pub fn collect_roots(extra_folders: &[String]) -> Vec<PathBuf> {
    let standard = [
        dirs::desktop_dir(),
        dirs::document_dir(),
        dirs::download_dir(),
        dirs::picture_dir(),
        dirs::audio_dir(),
        dirs::video_dir(),
    ];
    let candidates: Vec<PathBuf> = standard
        .into_iter()
        .flatten()
        .chain(extra_folders.iter().map(PathBuf::from))
        .filter(|path| path.is_dir())
        .collect();
    remove_nested_roots(candidates)
}

fn remove_nested_roots(candidates: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = HashSet::new();
    let deduplicated: Vec<PathBuf> =
        candidates.into_iter().filter(|path| unique.insert(path.to_string_lossy().to_lowercase())).collect();
    deduplicated
        .iter()
        .filter(|path| {
            !deduplicated.iter().any(|other| other != *path && path.starts_with(other))
        })
        .cloned()
        .collect()
}

pub fn index_roots(roots: &[PathBuf], limit: usize) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    for root in roots {
        let walker = WalkDir::new(root)
            .max_depth(MAX_DEPTH)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| entry.depth() == 0 || !should_skip(entry));
        for entry in walker {
            if entries.len() >= limit {
                log::warn!("File index capped at {limit} entries");
                return entries;
            }
            let entry = match entry {
                Ok(entry) if entry.depth() > 0 => entry,
                Ok(_) => continue,
                Err(error) => {
                    log::debug!("Skipping unreadable path: {error}");
                    continue;
                }
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            entries.push(FileEntry {
                name_lowercase: name.to_lowercase(),
                searchable_name: Utf32String::from(name.as_str()),
                name,
                path: entry.path().to_string_lossy().into_owned(),
                is_directory: entry.file_type().is_dir(),
            });
        }
    }
    entries
}

fn should_skip(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    if name.starts_with('.') || name.starts_with('~') || name.starts_with('$') {
        return true;
    }
    if entry.file_type().is_dir() && SKIPPED_DIRECTORY_NAMES.contains(&name.to_lowercase().as_str()) {
        return true;
    }
    is_hidden_or_system(entry)
}

#[cfg(windows)]
fn is_hidden_or_system(entry: &DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
    // walkdir caches metadata from the directory listing on Windows, so this
    // does not cost an extra system call per entry.
    entry
        .metadata()
        .map(|metadata| metadata.file_attributes() & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_hidden_or_system(_entry: &DirEntry) -> bool {
    false
}

/// Parent folder shown under a file name, with the home folder shortened to `~`.
pub fn display_parent(path: &str) -> String {
    let parent = Path::new(path).parent().map(|parent| parent.to_string_lossy().into_owned()).unwrap_or_default();
    shorten_home(&parent, dirs::home_dir().as_deref())
}

fn shorten_home(path: &str, home: Option<&Path>) -> String {
    let Some(home) = home.map(|home| home.to_string_lossy().into_owned()) else {
        return path.to_string();
    };
    // `get` avoids panicking when the cut falls inside a multi-byte character.
    match (path.get(..home.len()), path.get(home.len()..)) {
        (Some(prefix), Some(rest)) if prefix.eq_ignore_ascii_case(&home) => format!("~{rest}"),
        _ => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_roots_are_removed() {
        let roots = remove_nested_roots(vec![
            PathBuf::from("C:\\Users\\me\\Documents"),
            PathBuf::from("C:\\Users\\me\\Documents\\Projects"),
            PathBuf::from("C:\\Users\\me\\documents"),
            PathBuf::from("D:\\Work"),
        ]);
        assert_eq!(roots, vec![PathBuf::from("C:\\Users\\me\\Documents"), PathBuf::from("D:\\Work")]);
    }

    #[test]
    fn home_is_shortened() {
        let home = PathBuf::from("C:\\Users\\me");
        assert_eq!(shorten_home("C:\\Users\\me\\Documents", Some(&home)), "~\\Documents");
        assert_eq!(shorten_home("D:\\Work", Some(&home)), "D:\\Work");
    }

    #[test]
    fn indexes_a_real_directory_tree() {
        let root = std::env::temp_dir().join("l8-files-index-test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("project").join("node_modules")).unwrap();
        std::fs::write(root.join("project").join("notes.md"), "x").unwrap();
        std::fs::write(root.join("project").join("node_modules").join("dep.js"), "x").unwrap();
        std::fs::write(root.join(".hidden"), "x").unwrap();

        let entries = index_roots(std::slice::from_ref(&root), 100);
        let names: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
        assert!(names.contains(&"project"));
        assert!(names.contains(&"notes.md"));
        assert!(!names.contains(&"dep.js"));
        assert!(!names.contains(&".hidden"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
