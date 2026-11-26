//! Quick Accent: hold a letter, press Space or an arrow, pick an accented
//! character from a bar that drops out of the notch, release the letter.
//!
//! - `characters`: which accents each language offers.
//! - `machine`: the keyboard logic, pure and unit tested.
//! - this module: the system keyboard hook, the bar shown in the notch and
//!   the typing of the chosen character.
//!
//! The hook callback only decides (swallow or not) and queues a command; a
//! worker thread does the slow parts (events to the web layer, typing), so
//! the keyboard never lags.

mod characters;
mod machine;

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub use characters::{characters_for, AccentLanguage};
pub use machine::AccentTrigger;
use machine::{AccentMachine, Command, Key};

use crate::fullscreen_pause;
use crate::notch;
use crate::platform::{self, KeyboardHook};
use crate::settings::QuickAccentSettings;

pub const ACCENT_STATE_EVENT: &str = "accent:state";
/// Lets the bar's closing animation play before the window may hide.
const HIDE_ANIMATION: Duration = Duration::from_millis(260);

/// What the notch draws, sent on every change.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccentBarState {
    pub visible: bool,
    pub characters: Vec<String>,
    pub index: usize,
}

impl AccentBarState {
    fn hidden() -> Self {
        AccentBarState { visible: false, characters: Vec::new(), index: 0 }
    }
}

/// One letter and its accents, for the settings preview.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccentPreviewRow {
    pub letter: String,
    pub characters: Vec<String>,
}

/// Every letter that offers accents with these languages, a to z.
pub fn preview(languages: &[AccentLanguage]) -> Vec<AccentPreviewRow> {
    ('a'..='z')
        .filter_map(|letter| {
            let characters = characters_for(letter, languages, false);
            (!characters.is_empty()).then(|| AccentPreviewRow {
                letter: letter.to_string(),
                characters: characters.into_iter().map(String::from).collect(),
            })
        })
        .collect()
}

/// State the hook callback needs, shared with the service.
struct HookState {
    machine: Mutex<AccentMachine>,
    settings: RwLock<QuickAccentSettings>,
}

#[derive(Default)]
pub struct QuickAccentService {
    hook: Mutex<Option<KeyboardHook>>,
    shared: Mutex<Option<Arc<HookState>>>,
}

impl QuickAccentService {
    /// Applies the settings: installs or removes the keyboard hook.
    pub fn configure(&self, app: &AppHandle, settings: &QuickAccentSettings) {
        let mut hook = self.hook.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut shared = self.shared.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

        if !settings.enabled {
            if hook.take().is_some() {
                log::info!("Quick Accent off");
            }
            *shared = None;
            return;
        }
        if let Some(state) = shared.as_ref() {
            *state.settings.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = settings.clone();
            // New languages may change what an open bar would offer: start over.
            let _ = state.machine.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).reset();
            if hook.is_some() {
                return;
            }
        }

        let state = Arc::new(HookState {
            machine: Mutex::new(AccentMachine::default()),
            settings: RwLock::new(settings.clone()),
        });
        let (commands, receiver) = mpsc::channel();
        start_worker(app.clone(), Arc::clone(&state), receiver);
        match platform::install_keyboard_hook(hook_handler(app.clone(), Arc::clone(&state), commands)) {
            Ok(installed) => {
                log::info!("Quick Accent on");
                *hook = Some(installed);
                *shared = Some(state);
            }
            Err(error) => log::warn!("Quick Accent is unavailable: {error}"),
        }
    }
}

/// Runs inside the keyboard hook: decide fast, queue the rest.
fn hook_handler(app: AppHandle, state: Arc<HookState>, commands: Sender<Command>) -> platform::KeyHandler {
    Box::new(move |event| {
        let mut machine = state.machine.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        // The open notch has its own keyboard (search field, music keys), and
        // a fullscreen game gets every key untouched.
        if notch::is_open(&app) || fullscreen_pause::is_active(&app) {
            if let Some(command) = machine.reset() {
                let _ = commands.send(command);
            }
            return false;
        }
        let settings = state.settings.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        let decision = machine.handle(
            event,
            Instant::now(),
            settings.trigger,
            platform::caps_lock_on(),
            platform::pressed_modifiers(),
            |letter| characters_for(letter, &settings.languages, false).len(),
        );
        if matches!(event.key, Key::Space | Key::Left | Key::Right) && event.down && decision.command.is_none() {
            // Why an activation key did nothing (never logs which letter is typed).
            log::debug!("Quick Accent: {:?} passed through ({})", event.key, machine.describe());
        }
        if let Some(command) = decision.command {
            let _ = commands.send(command);
        }
        decision.swallow
    })
}

fn start_worker(app: AppHandle, state: Arc<HookState>, commands: Receiver<Command>) {
    let spawned = thread::Builder::new().name("quick-accent".into()).spawn(move || {
        let mut bar = AccentBarState::hidden();
        let mut letter = 'a';
        // Ends when the hook (the only sender) is dropped.
        while let Ok(command) = commands.recv() {
            log::debug!(
                "Quick Accent: {command:?} (focused window class: {})",
                platform::foreground_window_class()
            );
            let languages = state.settings.read().unwrap_or_else(|poisoned| poisoned.into_inner()).languages.clone();
            let characters = |letter: char, uppercase: bool| -> Vec<String> {
                characters_for(letter, &languages, uppercase).into_iter().map(String::from).collect()
            };
            match command {
                Command::Show { letter: shown, index, uppercase } => {
                    letter = shown;
                    bar = AccentBarState { visible: true, characters: characters(shown, uppercase), index };
                    notch::set_overlay(&app, true);
                    publish(&app, &bar);
                }
                Command::Select { index } => {
                    bar.index = index;
                    publish(&app, &bar);
                }
                Command::ChangeCase { uppercase } => {
                    bar.characters = characters(letter, uppercase);
                    publish(&app, &bar);
                }
                Command::Commit { letter, index, erase, uppercase } => {
                    let chosen = characters_for(letter, &languages, uppercase).get(index).copied();
                    bar = AccentBarState::hidden();
                    publish(&app, &bar);
                    match chosen {
                        Some(character) => match platform::type_replacing(erase, character) {
                            Ok(()) => log::debug!(
                                "Quick Accent: typed into window class {}",
                                platform::foreground_window_class()
                            ),
                            Err(error) => log::warn!("{error}"),
                        },
                        None => log::warn!("Quick Accent: no character at position {index}"),
                    }
                    hide_after_animation(&app, &state);
                }
                Command::Hide => {
                    bar = AccentBarState::hidden();
                    publish(&app, &bar);
                    hide_after_animation(&app, &state);
                }
            }
        }
    });
    if let Err(error) = spawned {
        log::error!("Cannot start Quick Accent: {error}");
    }
}

fn hide_after_animation(app: &AppHandle, state: &HookState) {
    thread::sleep(HIDE_ANIMATION);
    // A new bar may have opened meanwhile.
    if !state.machine.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).is_open() {
        notch::set_overlay(app, false);
    }
}

fn publish(app: &AppHandle, bar: &AccentBarState) {
    if let Err(error) = app.emit(ACCENT_STATE_EVENT, bar) {
        log::warn!("Cannot show the accent bar: {error}");
    }
}
