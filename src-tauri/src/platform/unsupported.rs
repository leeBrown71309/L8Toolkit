//! Placeholder implementations for platforms that are not supported yet.
//! They fail loudly (returning errors) instead of silently doing nothing.

use super::{HookEvent, InstalledApp, PressedModifiers, MediaArtwork, MediaCommand, MediaSnapshot, ScreenRect, ShellVerb};

const UNSUPPORTED: &str = "This feature is not implemented on this platform yet";

fn unsupported<T>() -> Result<T, String> {
    Err(UNSUPPORTED.to_string())
}

pub struct ComApartment;

pub struct MediaSessions;

impl MediaSessions {
    pub fn connect() -> Result<Self, String> {
        unsupported()
    }

    pub fn snapshot(&self) -> Result<Option<MediaSnapshot>, String> {
        unsupported()
    }

    pub fn artwork(&self) -> Result<Option<MediaArtwork>, String> {
        unsupported()
    }

    pub fn send(&self, _command: MediaCommand) -> Result<(), String> {
        unsupported()
    }
}

pub type KeyHandler = Box<dyn Fn(HookEvent) -> bool + Send + Sync>;

pub struct KeyboardHook;

pub fn install_keyboard_hook(_handler: KeyHandler) -> Result<KeyboardHook, String> {
    unsupported()
}

pub fn pressed_modifiers() -> PressedModifiers {
    PressedModifiers::default()
}

pub fn caps_lock_on() -> bool {
    false
}

pub fn type_replacing(_erase: u32, _character: char) -> Result<(), String> {
    unsupported()
}

pub fn show_without_activation(_handle: isize) {}

pub fn system_prefers_light() -> bool {
    false
}

pub fn foreground_window_class() -> String {
    String::new()
}

pub struct LoopbackCapture;

impl LoopbackCapture {
    pub fn open() -> Result<Self, String> {
        unsupported()
    }

    pub fn sample_rate(&self) -> u32 {
        0
    }

    pub fn read_mono(&mut self, _output: &mut Vec<f32>) -> Result<(), String> {
        unsupported()
    }

    pub fn default_device_changed(&self) -> bool {
        false
    }
}

pub fn enter_com() -> ComApartment {
    ComApartment
}

pub fn enumerate_installed_apps() -> Result<Vec<InstalledApp>, String> {
    unsupported()
}

pub fn extract_icon_rgba(_target: &str, _size: i32) -> Result<(Vec<u8>, u32, u32), String> {
    unsupported()
}

pub fn shell_open(_target: &str, _verb: ShellVerb) -> Result<(), String> {
    unsupported()
}

pub fn reveal_in_file_manager(_path: &str) -> Result<(), String> {
    unsupported()
}

pub fn lock_workstation() -> Result<(), String> {
    unsupported()
}

pub fn show_properties(_path: &str) -> Result<(), String> {
    unsupported()
}

pub fn open_with_dialog(_path: &str) -> Result<(), String> {
    unsupported()
}

pub fn move_to_recycle_bin(_path: &str) -> Result<(), String> {
    unsupported()
}

pub fn copy_files_to_clipboard(_paths: &[String]) -> Result<(), String> {
    unsupported()
}

pub fn open_in_terminal(_directory: &str) -> Result<(), String> {
    unsupported()
}

pub fn send_paste_shortcut() -> Result<(), String> {
    unsupported()
}

pub fn extract_thumbnail_rgba(_path: &str, _size: i32) -> Result<(Vec<u8>, u32, u32), String> {
    unsupported()
}

pub fn capture_screen_rgba(_rect: ScreenRect) -> Result<(Vec<u8>, u32, u32), String> {
    unsupported()
}

pub fn desktop_wallpaper_path() -> Option<String> {
    None
}

pub fn system_accent_color() -> Option<String> {
    None
}

pub fn system_primary_language() -> Option<u16> {
    None
}

pub fn foreground_window() -> isize {
    0
}

pub fn restore_foreground(_handle: isize) {}

pub fn force_foreground(_handle: isize) {}

pub fn cursor_position() -> Option<(i32, i32)> {
    None
}

pub fn fullscreen_foreground_rect(_own_window: isize) -> Option<ScreenRect> {
    None
}
