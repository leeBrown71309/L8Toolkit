use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResultKind {
    App,
    File,
    Folder,
    Calculator,
    Web,
    Url,
    Command,
}

/// One actionable row in the Spotlight list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub kind: ResultKind,
    pub title: String,
    pub subtitle: String,
    /// What gets opened or executed (path, URL, AppsFolder id, command id).
    pub target: String,
    /// Shell parsing name used to fetch a real icon, when one exists.
    pub icon_target: Option<String>,
    /// File or executable on disk: enables "Show in folder", "Properties"...
    #[serde(default)]
    pub location: Option<String>,
    /// Pinned to the Frequent row by the user.
    #[serde(default)]
    pub pinned: bool,
    /// Character positions in `title` that matched the query.
    #[serde(default)]
    pub match_indices: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SectionId {
    Calculator,
    TopHit,
    Applications,
    Commands,
    Folders,
    Files,
    Web,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSection {
    pub id: SectionId,
    pub items: Vec<SearchResult>,
}
