//! Global keyboard shortcuts.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_global_shortcut::{Builder, GlobalShortcutExt, Shortcut, ShortcutState};

use crate::i18n::{tr_with, Text};
use crate::notch;
use crate::settings::SettingsStore;

#[derive(Default)]
pub struct HotkeyState {
    spotlight: Mutex<Option<Shortcut>>,
    last_error: Mutex<Option<String>>,
    /// Released to a fullscreen app: still known, not held by Windows.
    suspended: AtomicBool,
}

impl HotkeyState {
    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    fn set_error(&self, error: Option<String>) {
        *self.last_error.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = error;
    }

    fn current(&self) -> Option<Shortcut> {
        *self.spotlight.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn is_spotlight(&self, shortcut: &Shortcut) -> bool {
        self.spotlight
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .is_some_and(|registered| registered.id() == shortcut.id())
    }
}

pub fn plugin() -> TauriPlugin<Wry> {
    Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            if app.state::<HotkeyState>().is_spotlight(shortcut) {
                notch::toggle_from_hotkey(app);
            }
        })
        .build()
}

pub fn parse_accelerator(accelerator: &str) -> Result<Shortcut, String> {
    accelerator
        .parse::<Shortcut>()
        .map_err(|error| format!("'{accelerator}' is not a valid shortcut: {error}"))
}

/// Replaces the Spotlight shortcut. On failure (usually another app already
/// owns the combination) the previous shortcut is restored.
pub fn register_spotlight(app: &AppHandle, accelerator: &str) -> Result<(), String> {
    let state = app.state::<HotkeyState>();
    let language = app.state::<SettingsStore>().language();
    let next = match parse_accelerator(accelerator) {
        Ok(shortcut) => shortcut,
        Err(error) => {
            log::warn!("{error}");
            let message = tr_with(language, Text::ErrorShortcutInvalid, &[("shortcut", accelerator)]);
            state.set_error(Some(message.clone()));
            return Err(message);
        }
    };
    if state.suspended.load(Ordering::SeqCst) {
        // Windows gets it when the fullscreen app loses the focus.
        *state.spotlight.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(next);
        state.set_error(None);
        log::info!("Spotlight shortcut set to {accelerator} (paused while a fullscreen app has the focus)");
        return Ok(());
    }
    let manager = app.global_shortcut();
    let previous = state.spotlight.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
    if let Some(previous) = previous {
        if let Err(error) = manager.unregister(previous) {
            log::warn!("Cannot release the previous shortcut: {error}");
        }
    }

    match manager.register(next) {
        Ok(()) => {
            *state.spotlight.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(next);
            state.set_error(None);
            log::info!("Spotlight shortcut set to {accelerator}");
            Ok(())
        }
        Err(error) => {
            log::warn!("Cannot register {accelerator}: {error}");
            let message = tr_with(language, Text::ErrorShortcutInUse, &[("shortcut", accelerator)]);
            if let Some(previous) = previous {
                if manager.register(previous).is_ok() {
                    *state.spotlight.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(previous);
                }
            }
            state.set_error(Some(message.clone()));
            Err(message)
        }
    }
}

pub fn unregister_spotlight(app: &AppHandle) {
    let state = app.state::<HotkeyState>();
    let previous = state.spotlight.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
    let held_by_windows = !state.suspended.load(Ordering::SeqCst);
    if let Some(previous) = previous.filter(|_| held_by_windows) {
        if let Err(error) = app.global_shortcut().unregister(previous) {
            log::warn!("Cannot release the Spotlight shortcut: {error}");
        }
    }
    state.set_error(None);
}

/// Hands the shortcut to the focused fullscreen app (`true`), so the game
/// receives the keys, or takes it back (`false`). Settings keep showing it.
///
/// Never called with the `spotlight` lock held: registering runs on the main
/// thread, whose shortcut handler takes that lock.
pub fn set_suspended(app: &AppHandle, suspended: bool) {
    let state = app.state::<HotkeyState>();
    if state.suspended.swap(suspended, Ordering::SeqCst) == suspended {
        return;
    }
    let Some(shortcut) = state.current() else { return };
    let manager = app.global_shortcut();
    if suspended {
        if let Err(error) = manager.unregister(shortcut) {
            log::warn!("Cannot release the Spotlight shortcut to the fullscreen app: {error}");
        }
        return;
    }
    match manager.register(shortcut) {
        Ok(()) => state.set_error(None),
        Err(error) => {
            // Another app took the combination while it was released.
            log::warn!("Cannot take the Spotlight shortcut back: {error}");
            let store = app.state::<SettingsStore>();
            let accelerator = store.read(|settings| settings.spotlight.hotkey.clone());
            let message = tr_with(store.language(), Text::ErrorShortcutInUse, &[("shortcut", &accelerator)]);
            state.set_error(Some(message));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_accelerator;

    #[test]
    fn parses_common_accelerators() {
        assert!(parse_accelerator("Alt+Space").is_ok());
        assert!(parse_accelerator("Ctrl+Shift+K").is_ok());
        assert!(parse_accelerator("Super+Alt+Space").is_ok());
        assert!(parse_accelerator("NotAKey+Q").is_err());
    }
}
