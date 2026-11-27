//! The Recents list itself, pure: newest first, no duplicates, bounded.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecentKind {
    Text,
    Image,
    Files,
}

/// How the item arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecentOrigin {
    /// Copied with Ctrl+C (or a menu) in any app.
    Copy,
    /// A screen capture (Win+Shift+S, Print Screen, Snipping Tool...).
    Screenshot,
    /// Dropped on the notch (the shelf).
    Shelf,
}

/// An image kept in the Recents folder: full size and thumbnail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredImage {
    /// File names inside the images folder.
    pub file: String,
    pub thumbnail: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentItem {
    pub id: String,
    pub kind: RecentKind,
    pub origin: RecentOrigin,
    pub created_at_ms: u64,
    /// Text items. Never written to disk in clear for sensitive items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Sensitive text, encrypted for the Windows user (DPAPI, base64): only on disk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected_text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<StoredImage>,
    /// Copied by a password manager: kept, never shown in clear.
    #[serde(default)]
    pub sensitive: bool,
    /// Executable of the app that copied ("bitwarden.exe").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_app: Option<String>,
    /// Identifies the content, to move a repeated copy up instead of adding it twice.
    pub fingerprint: u64,
}

/// FNV-1a: stable across versions (unlike the std hasher), so fingerprints
/// saved by one version still match in the next.
pub fn fingerprint(kind: RecentKind, bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let seed = [kind as u8];
    seed.iter().chain(bytes).fold(OFFSET, |hash, &byte| (hash ^ u64::from(byte)).wrapping_mul(PRIME))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RecentList {
    items: Vec<RecentItem>,
}

impl RecentList {
    pub fn new(items: Vec<RecentItem>) -> Self {
        RecentList { items }
    }

    pub fn items(&self) -> &[RecentItem] {
        &self.items
    }

    pub fn get(&self, id: &str) -> Option<&RecentItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Adds `item` on top. The same content already in the list is replaced
    /// (it moves up). Returns what left the list, so its files can be deleted.
    pub fn push(&mut self, item: RecentItem, limit: usize) -> Vec<RecentItem> {
        let mut removed: Vec<RecentItem> = Vec::new();
        if let Some(index) = self.items.iter().position(|existing| existing.fingerprint == item.fingerprint) {
            removed.push(self.items.remove(index));
        }
        self.items.insert(0, item);
        removed.extend(self.trim(limit));
        removed
    }

    /// Moves the item with this content to the top. False when absent.
    pub fn promote_fingerprint(&mut self, fingerprint: u64, now_ms: u64) -> bool {
        match self.items.iter().position(|item| item.fingerprint == fingerprint) {
            Some(index) => {
                self.move_to_top(index, now_ms);
                true
            }
            None => false,
        }
    }

    /// Moves an item to the top (copied again from Recents).
    pub fn promote(&mut self, id: &str, now_ms: u64) -> bool {
        match self.items.iter().position(|item| item.id == id) {
            Some(index) => {
                self.move_to_top(index, now_ms);
                true
            }
            None => false,
        }
    }

    fn move_to_top(&mut self, index: usize, now_ms: u64) {
        let mut item = self.items.remove(index);
        item.created_at_ms = now_ms;
        self.items.insert(0, item);
    }

    pub fn remove(&mut self, id: &str) -> Option<RecentItem> {
        let index = self.items.iter().position(|item| item.id == id)?;
        Some(self.items.remove(index))
    }

    /// Removes every item matching `predicate`.
    pub fn remove_where(&mut self, predicate: impl Fn(&RecentItem) -> bool) -> Vec<RecentItem> {
        let (removed, kept) = std::mem::take(&mut self.items).into_iter().partition(|item| predicate(item));
        self.items = kept;
        removed
    }

    /// Keeps the `limit` newest items.
    pub fn trim(&mut self, limit: usize) -> Vec<RecentItem> {
        if self.items.len() <= limit {
            return Vec::new();
        }
        self.items.split_off(limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(id: &str, content: &str) -> RecentItem {
        RecentItem {
            id: id.to_string(),
            kind: RecentKind::Text,
            origin: RecentOrigin::Copy,
            created_at_ms: 0,
            text: Some(content.to_string()),
            protected_text: None,
            files: Vec::new(),
            image: None,
            sensitive: false,
            source_app: None,
            fingerprint: fingerprint(RecentKind::Text, content.as_bytes()),
        }
    }

    fn ids(list: &RecentList) -> Vec<&str> {
        list.items().iter().map(|item| item.id.as_str()).collect()
    }

    #[test]
    fn newest_first_and_bounded() {
        let mut list = RecentList::default();
        for (index, content) in ["a", "b", "c"].iter().enumerate() {
            list.push(text(&index.to_string(), content), 2);
        }
        assert_eq!(ids(&list), ["2", "1"]);
    }

    #[test]
    fn copying_the_same_content_moves_it_up() {
        let mut list = RecentList::default();
        list.push(text("1", "hello"), 10);
        list.push(text("2", "world"), 10);
        let removed = list.push(text("3", "hello"), 10);
        assert_eq!(ids(&list), ["3", "2"]);
        assert_eq!(removed.len(), 1, "the older copy leaves (its files can go)");
    }

    #[test]
    fn promote_moves_to_the_top_with_a_fresh_time() {
        let mut list = RecentList::new(vec![text("1", "a"), text("2", "b")]);
        assert!(list.promote("2", 99));
        assert_eq!(ids(&list), ["2", "1"]);
        assert_eq!(list.items()[0].created_at_ms, 99);
        assert!(!list.promote("missing", 1));
        assert!(list.promote_fingerprint(fingerprint(RecentKind::Text, b"a"), 5));
        assert_eq!(ids(&list), ["1", "2"]);
    }

    #[test]
    fn fingerprints_depend_on_the_kind() {
        assert_ne!(fingerprint(RecentKind::Text, b"x"), fingerprint(RecentKind::Files, b"x"));
        assert_eq!(fingerprint(RecentKind::Text, b"x"), fingerprint(RecentKind::Text, b"x"));
    }

    #[test]
    fn sensitive_text_is_never_serialized_in_clear_when_moved_to_protected() {
        let mut item = text("1", "secret");
        item.sensitive = true;
        item.protected_text = Some("ZW5j".to_string());
        item.text = None;
        let json = serde_json::to_string(&item).unwrap();
        assert!(!json.contains("secret"), "{json}");
    }
}
