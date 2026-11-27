//! Operating-system integration layer.
//!
//! Everything that talks to the OS directly (app enumeration, icons, focus,
//! shell execution, window shape) lives behind this facade so the rest of the
//! app stays portable. Windows is the only implemented target for now; other
//! targets get explicit "unsupported" stubs so the macOS/Linux ports only need
//! to fill in this module.

/// An application as exposed by the OS launcher (Start menu on Windows).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    pub name: String,
    /// Opaque identifier used to launch the app and resolve its icon.
    pub launch_target: String,
    /// Executable on disk, when the app has one (Store apps do not).
    pub location: Option<String>,
}

/// How a target should be opened by the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellVerb {
    Open,
    RunAsAdmin,
}

/// Screen rectangle in physical pixels: (left, top, right, bottom).
pub type ScreenRect = (i32, i32, i32, i32);

/// The media session the OS considers most relevant (a playing one first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSnapshot {
    /// AppUserModelID of the player ("Spotify.exe", "Chrome", ...).
    pub source_app_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    /// Position extrapolated to "now", in milliseconds.
    pub position_ms: u64,
    pub duration_ms: u64,
    pub can_toggle: bool,
    pub can_next: bool,
    pub can_previous: bool,
    pub can_seek: bool,
}

/// Cover art bytes as provided by the player, with their MIME type.
#[derive(Debug, Clone)]
pub struct MediaArtwork {
    pub bytes: Vec<u8>,
    pub content_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCommand {
    TogglePlayPause,
    Next,
    Previous,
    Seek { position_ms: u64 },
}

/// What the clipboard held after a change (Recents).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardContent {
    Text(String),
    /// Files copied in Explorer (paths).
    Files(Vec<String>),
    /// An image in the "PNG" clipboard format (Snipping Tool, browsers).
    Png(Vec<u8>),
    /// An image as a device-independent bitmap (CF_DIB).
    Dib(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardChange {
    pub content: ClipboardContent,
    /// Copied by a password manager, or tagged by the app as not for history.
    pub sensitive: bool,
    /// Executable of the app that copied, lowercase ("bitwarden.exe").
    pub source_app: Option<String>,
}

pub type ClipboardHandler = Box<dyn Fn(ClipboardChange) + Send + 'static>;

/// A drag of files over the notch's drop zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropZoneEvent {
    /// Files came over the zone.
    Enter,
    /// They left without being dropped (or the drag was cancelled).
    Leave,
    /// They were dropped: their paths.
    Drop(Vec<String>),
}

pub type DropZoneHandler = Box<dyn Fn(DropZoneEvent) + Send + 'static>;

/// Keys reported by the system keyboard hook (only those features react to).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookKey {
    /// A letter key, as a lowercase ASCII letter (whatever the layout prints on it).
    Letter(char),
    Space,
    Left,
    Right,
    Escape,
    Shift,
    Control,
    /// Alt, and AltGr (which Windows reports as Ctrl+Alt).
    Alt,
    Windows,
    Other,
}

/// Modifier keys physically held, as Windows itself knows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PressedModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub windows: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HookEvent {
    pub key: HookKey,
    pub down: bool,
}

/// Windows stores colors as 0xAABBGGRR; the web layer wants "#rrggbb".
pub fn abgr_to_hex(abgr: u32) -> String {
    let red = abgr & 0xff;
    let green = (abgr >> 8) & 0xff;
    let blue = (abgr >> 16) & 0xff;
    format!("#{red:02x}{green:02x}{blue:02x}")
}

#[cfg(windows)]
mod win32;
#[cfg(windows)]
pub use win32::*;

#[cfg(not(windows))]
mod unsupported;
#[cfg(not(windows))]
pub use unsupported::*;

#[cfg(test)]
mod tests {
    use super::abgr_to_hex;

    #[test]
    fn converts_windows_color_order() {
        assert_eq!(abgr_to_hex(0xff_d4_78_00), "#0078d4");
    }
}
