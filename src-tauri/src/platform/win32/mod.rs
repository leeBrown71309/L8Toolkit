//! Windows implementation of the platform layer.

mod apps;
mod com;
mod file_actions;
mod icons;
mod input;
mod locale;
mod loopback;
mod media;
mod screen;
mod shell;
mod system_colors;
mod window;

pub use apps::enumerate_installed_apps;
pub use com::enter_com;
pub use file_actions::{copy_files_to_clipboard, move_to_recycle_bin, open_in_terminal, open_with_dialog, show_properties};
pub use icons::{extract_icon_rgba, extract_thumbnail_rgba};
pub use input::send_paste_shortcut;
pub use locale::system_primary_language;
pub use loopback::LoopbackCapture;
pub use media::MediaSessions;
pub use screen::{capture_screen_rgba, desktop_wallpaper_path};
pub use shell::{lock_workstation, reveal_in_file_manager, shell_open};
pub use system_colors::system_accent_color;
pub use window::{
    cursor_position, force_foreground, foreground_window, fullscreen_foreground_rect,
    restore_foreground,
};
