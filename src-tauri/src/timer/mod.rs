//! Timer and Pomodoro. The logic lives in `machine` (pure, tested); this
//! module owns the running instance, wakes up when a period ends, tells the
//! web layer and shows a Windows notification.

mod machine;

use std::sync::{Condvar, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

pub use machine::{PomodoroPhase, TimerFinished, TimerSnapshot};
use machine::{PomodoroPlan, TimerMachine};

use crate::i18n::{tr, tr_with, Language, Text};
use crate::notch;
use crate::settings::{SettingsStore, TimerSettings};

pub const TIMER_STATE_EVENT: &str = "timer:state";
pub const TIMER_FINISHED_EVENT: &str = "timer:finished";
const MINUTE_MS: u64 = 60_000;
/// How long the notch stays up for the alert (the web layer shows it 5 s).
const ALERT_HOLD: Duration = Duration::from_millis(5600);
/// The watcher sleeps at most this long when nothing runs.
const IDLE_WAIT: Duration = Duration::from_secs(3600);

/// What the Timer tab asks for.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum TimerRequest {
    StartCountdown { duration_ms: u64 },
    StartPomodoro,
    Pause,
    Resume,
    Reset,
    AddTime { delta_ms: i64 },
    Skip,
}

#[derive(Default)]
pub struct TimerService {
    machine: Mutex<TimerMachine>,
    /// Wakes the watcher when the deadline changes.
    changed: Condvar,
}

impl TimerService {
    pub fn snapshot(&self) -> TimerSnapshot {
        self.lock().snapshot(now_ms())
    }

    /// Counting down or paused: the notch shows the timer.
    pub fn is_active(&self) -> bool {
        self.lock().is_active()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, TimerMachine> {
        self.machine.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub fn plan_from(settings: &TimerSettings) -> PomodoroPlan {
    PomodoroPlan {
        focus_ms: u64::from(settings.focus_minutes) * MINUTE_MS,
        short_break_ms: u64::from(settings.short_break_minutes) * MINUTE_MS,
        long_break_ms: u64::from(settings.long_break_minutes) * MINUTE_MS,
        rounds: settings.rounds_before_long_break,
    }
}

/// Applies a request from the Timer tab and returns the new state.
pub fn apply(app: &AppHandle, request: TimerRequest) -> TimerSnapshot {
    let plan = app.state::<SettingsStore>().read(|settings| plan_from(&settings.timer));
    let service = app.state::<TimerService>();
    let now = now_ms();
    let snapshot = {
        let mut machine = service.lock();
        match request {
            TimerRequest::StartCountdown { duration_ms } => machine.start_countdown(duration_ms, now),
            TimerRequest::StartPomodoro => machine.start_pomodoro(&plan, now),
            TimerRequest::Pause => machine.pause(now),
            TimerRequest::Resume => machine.resume(now),
            TimerRequest::Reset => machine.reset(),
            TimerRequest::AddTime { delta_ms } => machine.add_time(delta_ms, now),
            TimerRequest::Skip => machine.skip(&plan, now),
        }
        machine.snapshot(now)
    };
    service.changed.notify_all();
    publish(app, &snapshot);
    snapshot
}

/// Stops a running timer when the widget is turned off.
pub fn sync_with_settings(app: &AppHandle) {
    let enabled = app.state::<SettingsStore>().read(|settings| settings.timer.enabled);
    if !enabled && app.state::<TimerService>().is_active() {
        apply(app, TimerRequest::Reset);
    }
}

/// Sleeps until the running period ends (or the timer changes), then reports it.
pub fn start_watcher(app: &AppHandle) {
    let app = app.clone();
    let spawned = thread::Builder::new().name("timer".into()).spawn(move || loop {
        let service = app.state::<TimerService>();
        let mut machine = service.lock();
        let wait = machine
            .deadline()
            .map(|deadline| Duration::from_millis(deadline.saturating_sub(now_ms())))
            .unwrap_or(IDLE_WAIT);
        if !wait.is_zero() {
            machine = service
                .changed
                .wait_timeout(machine, wait)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
        let (plan, auto_continue, notify, in_notch) = app.state::<SettingsStore>().read(|settings| {
            (plan_from(&settings.timer), settings.timer.auto_continue, settings.timer.notify, settings.timer.show_in_notch)
        });
        let now = now_ms();
        let Some(finished) = machine.tick(&plan, auto_continue, now) else { continue };
        let snapshot = machine.snapshot(now);
        drop(machine);

        log::info!("Timer: {:?} {:?} finished", finished.mode, finished.phase);
        // Before publishing: an ended countdown would otherwise hide the notch.
        if in_notch {
            notch::hold_visible(&app, ALERT_HOLD);
        }
        publish(&app, &snapshot);
        if let Err(error) = app.emit(TIMER_FINISHED_EVENT, finished) {
            log::warn!("Cannot announce the end of the timer: {error}");
        }
        if notify {
            show_notification(&app, &finished, &plan);
        }
    });
    if let Err(error) = spawned {
        log::error!("Cannot start the timer: {error}");
    }
}

fn publish(app: &AppHandle, snapshot: &TimerSnapshot) {
    if let Err(error) = app.emit(TIMER_STATE_EVENT, snapshot) {
        log::warn!("Cannot send the timer state: {error}");
    }
    // A running timer keeps the idle notch visible.
    notch::sync_idle_visibility(app);
}

fn show_notification(app: &AppHandle, finished: &TimerFinished, plan: &PomodoroPlan) {
    let language = app.state::<SettingsStore>().language();
    let (title, body) = notification_texts(language, finished, plan);
    let shown = app.notification().builder().title(title).body(body).show();
    if let Err(error) = shown {
        log::warn!("Cannot show the timer notification: {error}");
    }
}

fn notification_texts(language: Language, finished: &TimerFinished, plan: &PomodoroPlan) -> (String, String) {
    let Some(next) = finished.next_phase else {
        return (tr(language, Text::TimerDoneTitle).to_string(), tr(language, Text::TimerDoneBody).to_string());
    };
    let title = match finished.phase {
        Some(PomodoroPhase::Focus) => tr(language, Text::PomodoroFocusDoneTitle),
        _ => tr(language, Text::PomodoroBreakDoneTitle),
    };
    let minutes = (plan.duration_of(next) / MINUTE_MS).to_string();
    let text = match (next, finished.continued) {
        (PomodoroPhase::Focus, true) => Text::PomodoroFocusStarted,
        (PomodoroPhase::Focus, false) => Text::PomodoroFocusReady,
        (_, true) => Text::PomodoroBreakStarted,
        (_, false) => Text::PomodoroBreakReady,
    };
    (title.to_string(), tr_with(language, text, &[("minutes", &minutes)]))
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_millis() as u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::machine::TimerMode;
    use super::*;

    #[test]
    fn requests_use_camel_case_tags() {
        let request: TimerRequest = serde_json::from_str(r#"{ "type": "startCountdown", "durationMs": 60000 }"#).unwrap();
        assert!(matches!(request, TimerRequest::StartCountdown { duration_ms: 60_000 }));
        let request: TimerRequest = serde_json::from_str(r#"{ "type": "addTime", "deltaMs": -60000 }"#).unwrap();
        assert!(matches!(request, TimerRequest::AddTime { delta_ms: -60_000 }));
    }

    #[test]
    fn notifications_name_the_next_break_length() {
        let plan = PomodoroPlan { focus_ms: 25 * MINUTE_MS, short_break_ms: 5 * MINUTE_MS, long_break_ms: 15 * MINUTE_MS, rounds: 4 };
        let finished = TimerFinished {
            mode: TimerMode::Pomodoro,
            phase: Some(PomodoroPhase::Focus),
            next_phase: Some(PomodoroPhase::ShortBreak),
            continued: true,
        };
        let (_, body) = notification_texts(Language::En, &finished, &plan);
        assert!(body.contains('5'), "{body}");
    }
}
