//! Quick Accent keyboard logic, independent of Windows so it can be tested.
//!
//! Hold a letter, press an activation key (Space, Left or Right) and the
//! accent bar appears; the activation keys then move the selection; releasing
//! the letter types the selected character in place of the letter.
//!
//! The letter itself is never blocked when first pressed (normal typing must
//! not lag): on commit, the letters it already typed are erased first. The
//! bar only appears once the letter has been held a moment, so fast typists
//! rolling from a letter onto Space ("a ") never trigger it.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// How long a letter must be held before an activation key opens the bar.
pub const ACTIVATION_DELAY: Duration = Duration::from_millis(250);

pub use crate::platform::{HookEvent as KeyEvent, HookKey as Key};

/// Which keys open the bar and move through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccentTrigger {
    #[default]
    SpaceAndArrows,
    Space,
    Arrows,
}

impl AccentTrigger {
    fn accepts(self, key: Key) -> bool {
        match self {
            AccentTrigger::SpaceAndArrows => matches!(key, Key::Space | Key::Left | Key::Right),
            AccentTrigger::Space => key == Key::Space,
            AccentTrigger::Arrows => matches!(key, Key::Left | Key::Right),
        }
    }
}

/// What the bar should do after a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Show the bar for `letter` with `index` selected.
    Show { letter: char, index: usize, uppercase: bool },
    Select { index: usize },
    /// Shift changed while the bar is open: show the other case.
    ChangeCase { uppercase: bool },
    /// Erase `erase` typed letters and type the selected character.
    Commit { letter: char, index: usize, erase: u32, uppercase: bool },
    /// Close the bar without typing anything.
    Hide,
}

/// The machine's answer to one key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Decision {
    /// Keep the key from reaching the application.
    pub swallow: bool,
    pub command: Option<Command>,
}

impl Decision {
    const PASS: Decision = Decision { swallow: false, command: None };

    fn swallow(command: Option<Command>) -> Self {
        Decision { swallow: true, command }
    }

    fn pass(command: Option<Command>) -> Self {
        Decision { swallow: false, command }
    }
}

#[derive(Debug, Clone, Copy)]
struct HeldLetter {
    letter: char,
    since: Instant,
    /// Letters typed by this key so far (first press plus auto-repeats).
    typed: u32,
}

#[derive(Debug, Clone, Copy)]
struct OpenBar {
    letter: char,
    count: usize,
    index: usize,
}

#[derive(Debug, Default)]
struct Modifiers {
    shift: bool,
    control: bool,
    alt: bool,
    windows: bool,
}

impl Modifiers {
    /// Ctrl, Alt (and AltGr) or Windows turn letters into shortcuts.
    fn shortcut(&self) -> bool {
        self.control || self.alt || self.windows
    }
}

#[derive(Debug, Default)]
pub struct AccentMachine {
    held: Option<HeldLetter>,
    bar: Option<OpenBar>,
    modifiers: Modifiers,
    /// Activation keys whose press was swallowed: their release is too.
    swallowed_down: Vec<Key>,
    caps_lock: bool,
}

impl AccentMachine {
    pub fn is_open(&self) -> bool {
        self.bar.is_some()
    }

    /// Forgets everything (settings changed, the notch opened...).
    pub fn reset(&mut self) -> Option<Command> {
        self.held = None;
        self.swallowed_down.clear();
        self.bar.take().map(|_| Command::Hide)
    }

    /// `count` tells how many characters a letter offers (0: none).
    pub fn handle(
        &mut self,
        event: KeyEvent,
        now: Instant,
        trigger: AccentTrigger,
        caps_lock: bool,
        count: impl Fn(char) -> usize,
    ) -> Decision {
        self.caps_lock = caps_lock;
        match event.key {
            Key::Shift => self.on_shift(event.down),
            Key::Control => {
                self.modifiers.control = event.down;
                Decision::PASS
            }
            Key::Alt => {
                self.modifiers.alt = event.down;
                Decision::PASS
            }
            Key::Windows => {
                self.modifiers.windows = event.down;
                Decision::PASS
            }
            Key::Letter(letter) if event.down => self.on_letter_down(letter, now),
            Key::Letter(letter) => self.on_letter_up(letter),
            Key::Escape if self.bar.is_some() || self.swallowed_down.contains(&Key::Escape) => {
                self.on_escape(event.down)
            }
            key if trigger.accepts(key) => self.on_activation_key(key, event.down, now, count),
            _ if event.down && self.bar.is_some() => {
                // Any other key means the user moved on: close without typing.
                self.held = None;
                Decision::pass(self.bar.take().map(|_| Command::Hide))
            }
            _ => Decision::PASS,
        }
    }

    fn uppercase(&self) -> bool {
        self.modifiers.shift != self.caps_lock
    }

    fn on_shift(&mut self, down: bool) -> Decision {
        let changed = self.modifiers.shift != down;
        self.modifiers.shift = down;
        if changed && self.bar.is_some() {
            Decision::pass(Some(Command::ChangeCase { uppercase: self.uppercase() }))
        } else {
            Decision::PASS
        }
    }

    fn on_letter_down(&mut self, letter: char, now: Instant) -> Decision {
        if self.modifiers.shortcut() {
            self.held = None;
            return Decision::PASS;
        }
        match self.held {
            Some(ref mut held) if held.letter == letter => {
                // Auto-repeat: harmless before the bar opens, noise after.
                if self.bar.is_some() {
                    return Decision::swallow(None);
                }
                held.typed += 1;
                Decision::PASS
            }
            _ => {
                self.held = Some(HeldLetter { letter, since: now, typed: 1 });
                Decision::pass(self.bar.take().map(|_| Command::Hide))
            }
        }
    }

    fn on_letter_up(&mut self, letter: char) -> Decision {
        let Some(held) = self.held.filter(|held| held.letter == letter) else { return Decision::PASS };
        self.held = None;
        match self.bar.take() {
            Some(bar) => Decision::pass(Some(Command::Commit {
                letter: bar.letter,
                index: bar.index,
                erase: held.typed,
                uppercase: self.uppercase(),
            })),
            None => Decision::PASS,
        }
    }

    fn on_escape(&mut self, down: bool) -> Decision {
        if down {
            self.swallowed_down.push(Key::Escape);
            self.held = None;
            Decision::swallow(self.bar.take().map(|_| Command::Hide))
        } else {
            self.swallowed_down.retain(|key| *key != Key::Escape);
            Decision::swallow(None)
        }
    }

    fn on_activation_key(&mut self, key: Key, down: bool, now: Instant, count: impl Fn(char) -> usize) -> Decision {
        if !down {
            if let Some(position) = self.swallowed_down.iter().position(|swallowed| *swallowed == key) {
                self.swallowed_down.remove(position);
                return Decision::swallow(None);
            }
            return Decision::PASS;
        }

        if let Some(bar) = self.bar.as_mut() {
            // Shift+Space steps back only once open: Shift is also held to type capitals.
            let backwards = key == Key::Left || (key == Key::Space && self.modifiers.shift);
            bar.index = if backwards { (bar.index + bar.count - 1) % bar.count } else { (bar.index + 1) % bar.count };
            let index = bar.index;
            self.remember_swallowed(key);
            return Decision::swallow(Some(Command::Select { index }));
        }

        let Some(held) = self.held else { return Decision::PASS };
        if self.modifiers.shortcut() || now.saturating_duration_since(held.since) < ACTIVATION_DELAY {
            return Decision::PASS;
        }
        let available = count(held.letter);
        if available == 0 {
            return Decision::PASS;
        }
        let index = if key == Key::Left { available - 1 } else { 0 };
        self.bar = Some(OpenBar { letter: held.letter, count: available, index });
        self.remember_swallowed(key);
        Decision::swallow(Some(Command::Show { letter: held.letter, index, uppercase: self.uppercase() }))
    }

    fn remember_swallowed(&mut self, key: Key) {
        if !self.swallowed_down.contains(&key) {
            self.swallowed_down.push(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELD: Duration = Duration::from_millis(400);

    struct Harness {
        machine: AccentMachine,
        trigger: AccentTrigger,
        caps_lock: bool,
    }

    impl Harness {
        fn new() -> Self {
            Harness { machine: AccentMachine::default(), trigger: AccentTrigger::SpaceAndArrows, caps_lock: false }
        }

        fn key(&mut self, key: Key, down: bool, now: Instant) -> Decision {
            // 'e' offers 4 characters, 'k' none.
            let count = |letter: char| if letter == 'e' { 4 } else { 0 };
            self.machine.handle(KeyEvent { key, down }, now, self.trigger, self.caps_lock, count)
        }

        /// Presses `letter` and returns the moment it has been held long enough.
        fn hold(&mut self, letter: char) -> Instant {
            assert!(!self.key(Key::Letter(letter), true, Instant::now()).swallow);
            Instant::now() + HELD
        }
    }

    #[test]
    fn holding_a_letter_then_space_opens_the_bar_on_the_first_accent() {
        let mut harness = Harness::new();
        let later = harness.hold('e');
        let decision = harness.key(Key::Space, true, later);
        assert!(decision.swallow);
        assert_eq!(decision.command, Some(Command::Show { letter: 'e', index: 0, uppercase: false }));
        assert!(harness.key(Key::Space, false, later).swallow, "the release of a swallowed key is swallowed too");
    }

    #[test]
    fn left_opens_on_the_last_accent_and_keys_cycle_both_ways() {
        let mut harness = Harness::new();
        let later = harness.hold('e');
        assert_eq!(harness.key(Key::Left, true, later).command, Some(Command::Show { letter: 'e', index: 3, uppercase: false }));
        assert_eq!(harness.key(Key::Right, true, later).command, Some(Command::Select { index: 0 }));
        assert_eq!(harness.key(Key::Left, true, later).command, Some(Command::Select { index: 3 }));
        assert_eq!(harness.key(Key::Space, true, later).command, Some(Command::Select { index: 0 }));
    }

    #[test]
    fn releasing_the_letter_types_the_selection_in_place_of_the_letter() {
        let mut harness = Harness::new();
        let later = harness.hold('e');
        harness.key(Key::Space, true, later);
        harness.key(Key::Space, true, later);
        let decision = harness.key(Key::Letter('e'), false, later);
        assert!(!decision.swallow);
        assert_eq!(decision.command, Some(Command::Commit { letter: 'e', index: 1, erase: 1, uppercase: false }));
        assert!(!harness.machine.is_open());
    }

    #[test]
    fn fast_typing_never_opens_the_bar() {
        let mut harness = Harness::new();
        let now = Instant::now();
        harness.key(Key::Letter('e'), true, now);
        let decision = harness.key(Key::Space, true, now + Duration::from_millis(60));
        assert_eq!(decision, Decision::PASS);
    }

    #[test]
    fn letters_without_accents_and_shortcuts_are_left_alone() {
        let mut harness = Harness::new();
        let later = harness.hold('k');
        assert_eq!(harness.key(Key::Space, true, later), Decision::PASS);

        let mut harness = Harness::new();
        harness.key(Key::Control, true, Instant::now());
        let later = harness.hold('e');
        assert_eq!(harness.key(Key::Space, true, later), Decision::PASS);
    }

    #[test]
    fn auto_repeats_are_erased_on_commit_and_swallowed_once_open() {
        let mut harness = Harness::new();
        let later = harness.hold('e');
        harness.key(Key::Letter('e'), true, later); // auto-repeat before the bar
        harness.key(Key::Space, true, later);
        assert!(harness.key(Key::Letter('e'), true, later).swallow, "repeats after opening are blocked");
        let decision = harness.key(Key::Letter('e'), false, later);
        assert_eq!(decision.command, Some(Command::Commit { letter: 'e', index: 0, erase: 2, uppercase: false }));
    }

    #[test]
    fn escape_closes_without_typing() {
        let mut harness = Harness::new();
        let later = harness.hold('e');
        harness.key(Key::Space, true, later);
        let decision = harness.key(Key::Escape, true, later);
        assert!(decision.swallow);
        assert_eq!(decision.command, Some(Command::Hide));
        assert!(harness.key(Key::Escape, false, later).swallow);
        assert_eq!(harness.key(Key::Letter('e'), false, later), Decision::PASS);
    }

    #[test]
    fn shift_and_caps_lock_choose_the_case() {
        let mut harness = Harness::new();
        harness.key(Key::Shift, true, Instant::now());
        let later = harness.hold('e');
        assert_eq!(harness.key(Key::Space, true, later).command, Some(Command::Show { letter: 'e', index: 0, uppercase: true }));
        assert_eq!(harness.key(Key::Shift, false, later).command, Some(Command::ChangeCase { uppercase: false }));

        let mut harness = Harness::new();
        harness.caps_lock = true;
        let later = harness.hold('e');
        assert_eq!(harness.key(Key::Space, true, later).command, Some(Command::Show { letter: 'e', index: 0, uppercase: true }));
    }

    #[test]
    fn shift_space_goes_back() {
        let mut harness = Harness::new();
        let later = harness.hold('e');
        harness.key(Key::Space, true, later);
        harness.key(Key::Shift, true, later);
        assert_eq!(harness.key(Key::Space, true, later).command, Some(Command::Select { index: 3 }));
    }

    #[test]
    fn the_trigger_setting_limits_the_activation_keys() {
        let mut harness = Harness::new();
        harness.trigger = AccentTrigger::Arrows;
        let later = harness.hold('e');
        assert_eq!(harness.key(Key::Space, true, later), Decision::PASS);
        assert!(harness.key(Key::Right, true, later).swallow);
    }

    #[test]
    fn another_key_closes_the_bar() {
        let mut harness = Harness::new();
        let later = harness.hold('e');
        harness.key(Key::Space, true, later);
        let decision = harness.key(Key::Other, true, later);
        assert!(!decision.swallow);
        assert_eq!(decision.command, Some(Command::Hide));
    }
}
