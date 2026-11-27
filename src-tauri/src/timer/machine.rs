//! Timer and Pomodoro logic, pure: every method takes the current time (Unix
//! milliseconds), so it is unit tested without waiting.

use serde::{Deserialize, Serialize};

const MINUTE_MS: u64 = 60_000;
/// A countdown never goes past this (and never below one second).
pub const MAX_COUNTDOWN_MS: u64 = 24 * 60 * MINUTE_MS;
const MIN_COUNTDOWN_MS: u64 = 1_000;
pub const DEFAULT_COUNTDOWN_MS: u64 = 5 * MINUTE_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimerMode {
    #[default]
    Countdown,
    Pomodoro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PomodoroPhase {
    Focus,
    ShortBreak,
    LongBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TimerStatus {
    /// Nothing counts down; the last countdown length is kept for a restart.
    #[default]
    Idle,
    Running,
    Paused,
}

/// Pomodoro lengths, from the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PomodoroPlan {
    pub focus_ms: u64,
    pub short_break_ms: u64,
    pub long_break_ms: u64,
    /// Focus sessions before a long break.
    pub rounds: u32,
}

impl PomodoroPlan {
    pub fn duration_of(&self, phase: PomodoroPhase) -> u64 {
        match phase {
            PomodoroPhase::Focus => self.focus_ms,
            PomodoroPhase::ShortBreak => self.short_break_ms,
            PomodoroPhase::LongBreak => self.long_break_ms,
        }
    }

    /// What follows `phase`, given the focus sessions completed so far
    /// (including the one that just ended).
    fn next_phase(&self, phase: PomodoroPhase, completed_focus: u32) -> PomodoroPhase {
        match phase {
            PomodoroPhase::Focus if completed_focus.is_multiple_of(self.rounds.max(1)) => PomodoroPhase::LongBreak,
            PomodoroPhase::Focus => PomodoroPhase::ShortBreak,
            PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak => PomodoroPhase::Focus,
        }
    }
}

/// What the UI draws. While running, the remaining time is derived from
/// `ends_at_ms` so the display stays exact without an event per second.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerSnapshot {
    pub mode: TimerMode,
    pub status: TimerStatus,
    /// Pomodoro only.
    pub phase: Option<PomodoroPhase>,
    /// Length of the current period.
    pub duration_ms: u64,
    /// Exact while idle or paused; at `updated_at_ms` while running.
    pub remaining_ms: u64,
    /// Unix time at which the period ends, while running.
    pub ends_at_ms: Option<u64>,
    /// Focus sessions completed since the Pomodoro started.
    pub completed_focus: u32,
    /// Focus sessions before a long break.
    pub rounds: u32,
    pub updated_at_ms: u64,
}

/// A period came to its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerFinished {
    pub mode: TimerMode,
    /// The Pomodoro phase that ended.
    pub phase: Option<PomodoroPhase>,
    /// The Pomodoro phase that follows.
    pub next_phase: Option<PomodoroPhase>,
    /// The next phase started by itself.
    pub continued: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimerMachine {
    mode: TimerMode,
    status: TimerStatus,
    phase: Option<PomodoroPhase>,
    duration_ms: u64,
    /// Valid while idle or paused.
    remaining_ms: u64,
    /// Valid while running.
    ends_at_ms: u64,
    completed_focus: u32,
    rounds: u32,
}

impl Default for TimerMachine {
    fn default() -> Self {
        TimerMachine {
            mode: TimerMode::Countdown,
            status: TimerStatus::Idle,
            phase: None,
            duration_ms: DEFAULT_COUNTDOWN_MS,
            remaining_ms: DEFAULT_COUNTDOWN_MS,
            ends_at_ms: 0,
            completed_focus: 0,
            rounds: 4,
        }
    }
}

impl TimerMachine {
    /// Something is counting down or paused: the notch shows it.
    pub fn is_active(&self) -> bool {
        self.status != TimerStatus::Idle
    }

    /// When the running period ends.
    pub fn deadline(&self) -> Option<u64> {
        (self.status == TimerStatus::Running).then_some(self.ends_at_ms)
    }

    fn remaining_at(&self, now_ms: u64) -> u64 {
        match self.status {
            TimerStatus::Running => self.ends_at_ms.saturating_sub(now_ms),
            TimerStatus::Idle | TimerStatus::Paused => self.remaining_ms,
        }
    }

    pub fn snapshot(&self, now_ms: u64) -> TimerSnapshot {
        TimerSnapshot {
            mode: self.mode,
            status: self.status,
            phase: self.phase,
            duration_ms: self.duration_ms,
            remaining_ms: self.remaining_at(now_ms),
            ends_at_ms: self.deadline(),
            completed_focus: self.completed_focus,
            rounds: self.rounds,
            updated_at_ms: now_ms,
        }
    }

    pub fn start_countdown(&mut self, duration_ms: u64, now_ms: u64) {
        let duration_ms = duration_ms.clamp(MIN_COUNTDOWN_MS, MAX_COUNTDOWN_MS);
        *self = TimerMachine { rounds: self.rounds, ..TimerMachine::default() };
        self.duration_ms = duration_ms;
        self.run_from(duration_ms, now_ms);
    }

    pub fn start_pomodoro(&mut self, plan: &PomodoroPlan, now_ms: u64) {
        self.mode = TimerMode::Pomodoro;
        self.completed_focus = 0;
        self.rounds = plan.rounds.max(1);
        self.begin_phase(PomodoroPhase::Focus, plan, now_ms, true);
    }

    pub fn pause(&mut self, now_ms: u64) {
        if self.status == TimerStatus::Running {
            self.remaining_ms = self.remaining_at(now_ms);
            self.status = TimerStatus::Paused;
        }
    }

    pub fn resume(&mut self, now_ms: u64) {
        if self.status == TimerStatus::Paused {
            self.run_from(self.remaining_ms, now_ms);
        }
    }

    /// Stops everything; a countdown keeps its length for the next start.
    pub fn reset(&mut self) {
        let duration_ms = if self.mode == TimerMode::Countdown { self.duration_ms } else { DEFAULT_COUNTDOWN_MS };
        *self = TimerMachine { rounds: self.rounds, ..TimerMachine::default() };
        self.duration_ms = duration_ms;
        self.remaining_ms = duration_ms;
    }

    /// Adds (or removes) time to the running or paused period.
    pub fn add_time(&mut self, delta_ms: i64, now_ms: u64) {
        if !self.is_active() {
            return;
        }
        let remaining = self.remaining_at(now_ms);
        let next = remaining.saturating_add_signed(delta_ms).clamp(MIN_COUNTDOWN_MS, MAX_COUNTDOWN_MS);
        let elapsed = self.duration_ms.saturating_sub(remaining);
        self.duration_ms = (elapsed + next).min(MAX_COUNTDOWN_MS);
        match self.status {
            TimerStatus::Running => self.ends_at_ms = now_ms + next,
            TimerStatus::Paused => self.remaining_ms = next,
            TimerStatus::Idle => {}
        }
    }

    /// Pomodoro: ends the current phase now and moves to the next one,
    /// keeping the running or paused state.
    pub fn skip(&mut self, plan: &PomodoroPlan, now_ms: u64) {
        let Some(phase) = self.phase.filter(|_| self.mode == TimerMode::Pomodoro && self.is_active()) else {
            return;
        };
        let running = self.status == TimerStatus::Running;
        let next = self.advance(phase, plan);
        self.begin_phase(next, plan, now_ms, running);
    }

    /// Called when time passes; reports the end of a period once.
    pub fn tick(&mut self, plan: &PomodoroPlan, auto_continue: bool, now_ms: u64) -> Option<TimerFinished> {
        if self.status != TimerStatus::Running || now_ms < self.ends_at_ms {
            return None;
        }
        match (self.mode, self.phase) {
            (TimerMode::Pomodoro, Some(phase)) => {
                let next = self.advance(phase, plan);
                self.begin_phase(next, plan, now_ms, auto_continue);
                Some(TimerFinished {
                    mode: TimerMode::Pomodoro,
                    phase: Some(phase),
                    next_phase: Some(next),
                    continued: auto_continue,
                })
            }
            _ => {
                self.reset();
                Some(TimerFinished { mode: TimerMode::Countdown, phase: None, next_phase: None, continued: false })
            }
        }
    }

    /// Counts a completed focus session and picks the next phase.
    fn advance(&mut self, phase: PomodoroPhase, plan: &PomodoroPlan) -> PomodoroPhase {
        if phase == PomodoroPhase::Focus {
            self.completed_focus += 1;
        }
        PomodoroPlan { rounds: self.rounds, ..*plan }.next_phase(phase, self.completed_focus)
    }

    /// Enters `phase`, running, or paused at its full length (waiting for the user).
    fn begin_phase(&mut self, phase: PomodoroPhase, plan: &PomodoroPlan, now_ms: u64, running: bool) {
        let duration_ms = plan.duration_of(phase).clamp(MIN_COUNTDOWN_MS, MAX_COUNTDOWN_MS);
        self.phase = Some(phase);
        self.duration_ms = duration_ms;
        if running {
            self.run_from(duration_ms, now_ms);
        } else {
            self.status = TimerStatus::Paused;
            self.remaining_ms = duration_ms;
        }
    }

    fn run_from(&mut self, remaining_ms: u64, now_ms: u64) {
        self.status = TimerStatus::Running;
        self.ends_at_ms = now_ms + remaining_ms;
        self.remaining_ms = remaining_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAN: PomodoroPlan = PomodoroPlan { focus_ms: 25 * MINUTE_MS, short_break_ms: 5 * MINUTE_MS, long_break_ms: 15 * MINUTE_MS, rounds: 2 };

    #[test]
    fn countdown_runs_pauses_and_resumes_exactly() {
        let mut timer = TimerMachine::default();
        timer.start_countdown(10 * MINUTE_MS, 1_000);
        assert_eq!(timer.snapshot(61_000).remaining_ms, 9 * MINUTE_MS);
        timer.pause(61_000);
        assert_eq!(timer.snapshot(500_000).remaining_ms, 9 * MINUTE_MS, "paused time does not count");
        timer.resume(500_000);
        assert_eq!(timer.deadline(), Some(500_000 + 9 * MINUTE_MS));
    }

    #[test]
    fn countdown_finishes_once_and_keeps_its_length() {
        let mut timer = TimerMachine::default();
        timer.start_countdown(MINUTE_MS, 0);
        assert_eq!(timer.tick(&PLAN, true, MINUTE_MS - 1), None);
        let finished = timer.tick(&PLAN, true, MINUTE_MS).expect("the countdown ends");
        assert_eq!(finished.mode, TimerMode::Countdown);
        assert_eq!(timer.tick(&PLAN, true, MINUTE_MS + 10), None, "reported once");
        assert!(!timer.is_active());
        assert_eq!(timer.snapshot(0).duration_ms, MINUTE_MS, "ready to start again");
    }

    #[test]
    fn countdown_length_is_clamped() {
        let mut timer = TimerMachine::default();
        timer.start_countdown(0, 0);
        assert_eq!(timer.snapshot(0).duration_ms, MIN_COUNTDOWN_MS);
        timer.start_countdown(u64::MAX / 2, 0);
        assert_eq!(timer.snapshot(0).duration_ms, MAX_COUNTDOWN_MS);
    }

    #[test]
    fn adding_time_extends_the_period() {
        let mut timer = TimerMachine::default();
        timer.start_countdown(MINUTE_MS, 0);
        timer.add_time(MINUTE_MS as i64, 30_000);
        let snapshot = timer.snapshot(30_000);
        assert_eq!(snapshot.remaining_ms, 90_000);
        assert_eq!(snapshot.duration_ms, 2 * MINUTE_MS, "the progress keeps its elapsed part");
        timer.add_time(-10 * MINUTE_MS as i64, 30_000);
        assert_eq!(timer.snapshot(30_000).remaining_ms, MIN_COUNTDOWN_MS, "never below a second");
    }

    #[test]
    fn pomodoro_alternates_focus_and_breaks_with_a_long_break_after_the_rounds() {
        let mut timer = TimerMachine::default();
        timer.start_pomodoro(&PLAN, 0);
        let mut now = 0;
        let mut phases = Vec::new();
        for _ in 0..4 {
            now = timer.deadline().unwrap();
            let finished = timer.tick(&PLAN, true, now).unwrap();
            assert!(finished.continued);
            phases.push(finished.next_phase.unwrap());
        }
        assert_eq!(
            phases,
            [PomodoroPhase::ShortBreak, PomodoroPhase::Focus, PomodoroPhase::LongBreak, PomodoroPhase::Focus]
        );
        assert_eq!(timer.snapshot(now).completed_focus, 2);
    }

    #[test]
    fn pomodoro_waits_for_the_user_without_auto_continue() {
        let mut timer = TimerMachine::default();
        timer.start_pomodoro(&PLAN, 0);
        let finished = timer.tick(&PLAN, false, PLAN.focus_ms).unwrap();
        assert!(!finished.continued);
        let snapshot = timer.snapshot(PLAN.focus_ms + 99_000);
        assert_eq!(snapshot.status, TimerStatus::Paused);
        assert_eq!(snapshot.phase, Some(PomodoroPhase::ShortBreak));
        assert_eq!(snapshot.remaining_ms, PLAN.short_break_ms, "the break has not started");
    }

    #[test]
    fn skipping_keeps_the_paused_state() {
        let mut timer = TimerMachine::default();
        timer.start_pomodoro(&PLAN, 0);
        timer.pause(1_000);
        timer.skip(&PLAN, 2_000);
        let snapshot = timer.snapshot(2_000);
        assert_eq!(snapshot.phase, Some(PomodoroPhase::ShortBreak));
        assert_eq!(snapshot.status, TimerStatus::Paused);
        assert_eq!(snapshot.completed_focus, 1);
    }

    #[test]
    fn reset_returns_to_idle() {
        let mut timer = TimerMachine::default();
        timer.start_pomodoro(&PLAN, 0);
        timer.reset();
        assert!(!timer.is_active());
        assert_eq!(timer.snapshot(0).mode, TimerMode::Countdown);
        assert_eq!(timer.deadline(), None);
    }
}
