//! The notch window: a transparent, always-on-top surface pinned to the top
//! centre of a monitor.
//!
//! * Idle: it only paints a small black notch and lets every click through.
//!   A background watcher opens it when the pointer rests on the notch.
//! * Open: it hosts the active widget panel and receives input.
//!
//! The window never resizes while animating (that would stutter); the web
//! layer morphs the notch shape inside a fixed transparent canvas instead.

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, Monitor, PhysicalPosition, WebviewWindow};

use crate::backdrop;
use crate::fullscreen_pause;
use crate::media::MediaService;
use crate::platform::{self, DropZoneEvent};
use crate::recents;
use crate::settings::{Material, SettingsStore};
use crate::timer::TimerService;

pub const NOTCH_WINDOW_LABEL: &str = "notch";
const WINDOW_WIDTH: f64 = 760.0;
const WINDOW_HEIGHT: f64 = 540.0;
/// Must match the idle sizes in `notch-shape.css`. The hover zone uses the
/// wider "live" notch (shown while media plays) so both are covered.
const IDLE_LIVE_NOTCH_WIDTH: f64 = 224.0;
const IDLE_NOTCH_HEIGHT: f64 = 30.0;
const HOVER_ZONE_PADDING: f64 = 10.0;
/// The bubble holding a second activity beside the folded notch. Must match
/// `.notch-bubble` in `notch-shape.css`.
const BUBBLE_GAP: f64 = 8.0;
const BUBBLE_SIZE: f64 = 30.0;
const POINTER_POLL_INTERVAL: Duration = Duration::from_millis(50);
/// ~150 ms of resting on the notch before it opens, so passing by does not.
const HOVER_DWELL_TICKS: u32 = 3;
const FULLSCREEN_CHECK_EVERY_TICKS: u32 = 10;
/// Lets the collapse animation finish before the window moves or hides.
const COLLAPSE_SETTLE_DELAY: Duration = Duration::from_millis(320);
/// The drop zone stays ~300 ms after the button is released: the drop is
/// delivered to it after the release.
const DROP_ZONE_LINGER_TICKS: u32 = 6;
/// How often the drop zone is put back above the notch window while shown.
const DROP_ZONE_RAISE_EVERY_TICKS: u32 = 4;
/// Tells the web layer that files are dragged over the notch (enter, leave, drop).
pub const FILE_DRAG_EVENT: &str = "notch:file-drag";

pub mod events {
    pub const OPENED: &str = "notch:opened";
    pub const CLOSED: &str = "notch:closed";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenSource {
    Hotkey,
    Hover,
    Tray,
    /// Files dragged onto the notch.
    Drop,
}

/// Which part of the folded notch the pointer rested on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HoverZone {
    Notch,
    /// The bubble beside the notch (a second activity).
    Bubble,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenedPayload {
    source: OpenSource,
    zone: HoverZone,
    /// Screen behind the notch (Acrylic material), as a PNG data URL.
    backdrop: Option<String>,
}

/// Where the notch window currently sits, in physical pixels.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub scale: f64,
    pub on_primary_monitor: bool,
}

#[derive(Default)]
pub struct NotchController {
    is_open: AtomicBool,
    fullscreen_app_active: AtomicBool,
    /// The focused window is fullscreen on any monitor, not only the notch's.
    fullscreen_app_focused: AtomicBool,
    previous_foreground: AtomicIsize,
    own_window: AtomicIsize,
    /// A bar shown over the idle notch (Quick Accent) keeps the window visible.
    overlay_active: AtomicBool,
    /// The folded notch shows a bubble beside it (two activities at once).
    bubble: AtomicBool,
    /// Kept visible for a moment (a timer alert peeks in the idle notch).
    held: AtomicBool,
    hold_generation: AtomicU64,
    collapse_generation: AtomicU64,
    placement: Mutex<Placement>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotchSnapshot {
    pub is_open: bool,
}

impl NotchController {
    pub fn snapshot(&self) -> NotchSnapshot {
        NotchSnapshot { is_open: self.is_open.load(Ordering::SeqCst) }
    }

    pub fn set_bubble(&self, shown: bool) {
        self.bubble.store(shown, Ordering::SeqCst);
    }

    fn placement(&self) -> Placement {
        *self.placement.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn set_placement(&self, placement: Placement) {
        *self.placement.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = placement;
    }
}

fn notch_window(app: &AppHandle) -> Option<WebviewWindow> {
    let window = app.get_webview_window(NOTCH_WINDOW_LABEL);
    if window.is_none() {
        log::error!("The notch window does not exist");
    }
    window
}

pub fn initialize(app: &AppHandle) -> Result<(), String> {
    let window = notch_window(app).ok_or("The notch window is missing from tauri.conf.json")?;
    let controller = app.state::<NotchController>();

    #[cfg(windows)]
    match window.hwnd() {
        Ok(hwnd) => controller.own_window.store(hwnd.0 as isize, Ordering::SeqCst),
        Err(error) => log::warn!("Cannot read the notch window handle: {error}"),
    }

    place_on_primary_monitor(app, &window, &controller);
    window.set_ignore_cursor_events(true).map_err(|error| error.to_string())?;
    sync_idle_visibility(app);
    start_pointer_watch(app.clone());
    start_drop_zone(app);
    Ok(())
}

/// Opens the notch with the Spotlight panel focused.
pub fn open(app: &AppHandle, source: OpenSource) {
    open_at(app, source, HoverZone::Notch);
}

/// Opens the notch; `zone` tells the web layer which activity was pointed at.
fn open_at(app: &AppHandle, source: OpenSource, zone: HoverZone) {
    let enabled = app.state::<SettingsStore>().read(|settings| {
        settings.spotlight.enabled || settings.music.enabled || settings.timer.enabled || settings.recents.enabled
    });
    if !enabled {
        return;
    }
    // Store apps installed meanwhile become searchable (no shortcut to watch).
    crate::indexer::refresh_apps_if_stale(app);
    let controller = app.state::<NotchController>();
    if controller.is_open.swap(true, Ordering::SeqCst) {
        return;
    }
    controller.collapse_generation.fetch_add(1, Ordering::SeqCst);
    let Some(window) = notch_window(app) else { return };

    let own_window = controller.own_window.load(Ordering::SeqCst);
    let foreground = platform::foreground_window();
    if foreground != own_window {
        controller.previous_foreground.store(foreground, Ordering::SeqCst);
    }

    // Keyboard and tray opens follow the pointer to its monitor; hover and
    // drop opens already happen on the monitor showing the notch.
    if !matches!(source, OpenSource::Hover | OpenSource::Drop) {
        if let Some(monitor) = cursor_monitor(app) {
            place_on_monitor(&window, &monitor, &controller, is_primary(app, &monitor));
        }
    }

    // Taken before the panel draws, so the snapshot shows what is behind it.
    let material = app.state::<SettingsStore>().read(|settings| settings.appearance.material);
    // Mica also gets one, as a fallback when the wallpaper cannot be read.
    let backdrop = matches!(material, Material::Acrylic | Material::Mica)
        .then(|| capture_backdrop(&controller.placement()))
        .flatten();

    log_failure("show the notch", window.show());
    log_failure("keep the notch on top", window.set_always_on_top(true));
    log_failure("make the notch interactive", window.set_ignore_cursor_events(false));
    log_failure(
        "notify the notch",
        app.emit_to(NOTCH_WINDOW_LABEL, events::OPENED, OpenedPayload { source, zone, backdrop }),
    );
    log_failure("focus the notch", window.set_focus());
    // Windows may refuse focus to a background app (hover opens); force it on
    // the thread that owns the window.
    log_failure(
        "schedule notch focus",
        app.run_on_main_thread(move || platform::force_foreground(own_window)),
    );
}

pub fn toggle_from_hotkey(app: &AppHandle) {
    let is_open = app.state::<NotchController>().is_open.load(Ordering::SeqCst);
    if is_open {
        collapse(app, true);
    } else {
        open(app, OpenSource::Hotkey);
    }
}

/// Returns the notch to its idle state. `restore_focus` hands keyboard focus
/// back to the previously active window (Escape), as opposed to launching
/// something, where the launched app should keep focus.
pub fn collapse(app: &AppHandle, restore_focus: bool) {
    let controller = app.state::<NotchController>();
    if !controller.is_open.swap(false, Ordering::SeqCst) {
        return;
    }
    let generation = controller.collapse_generation.fetch_add(1, Ordering::SeqCst) + 1;

    if let Some(window) = notch_window(app) {
        log_failure("make the notch click-through", window.set_ignore_cursor_events(true));
    }
    log_failure("notify the notch", app.emit_to(NOTCH_WINDOW_LABEL, events::CLOSED, ()));

    let previous = controller.previous_foreground.swap(0, Ordering::SeqCst);
    if restore_focus {
        platform::restore_foreground(previous);
    }

    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(COLLAPSE_SETTLE_DELAY);
        let controller = app.state::<NotchController>();
        let superseded = controller.is_open.load(Ordering::SeqCst)
            || controller.collapse_generation.load(Ordering::SeqCst) != generation;
        if superseded {
            return;
        }
        if let Some(window) = notch_window(&app) {
            if !controller.placement().on_primary_monitor {
                place_on_primary_monitor(&app, &window, &controller);
            }
        }
        sync_idle_visibility(&app);
    });
}

/// Shows or hides the idle notch according to settings and fullscreen apps.
pub fn sync_idle_visibility(app: &AppHandle) {
    let controller = app.state::<NotchController>();
    if controller.is_open.load(Ordering::SeqCst) {
        return;
    }
    let Some(window) = notch_window(app) else { return };
    if controller.overlay_active.load(Ordering::SeqCst) {
        return;
    }
    let visible = idle_notch_wanted(app) && !controller.fullscreen_app_active.load(Ordering::SeqCst);
    let outcome = if visible { window.show() } else { window.hide() };
    log_failure("update notch visibility", outcome);
}

pub fn is_open(app: &AppHandle) -> bool {
    app.state::<NotchController>().is_open.load(Ordering::SeqCst)
}

/// Shows the notch window for a bar drawn over the idle notch (Quick Accent),
/// without taking the focus from the app being typed in; `false` returns to
/// the normal idle visibility.
pub fn set_overlay(app: &AppHandle, active: bool) {
    let controller = app.state::<NotchController>();
    controller.overlay_active.store(active, Ordering::SeqCst);
    if active {
        platform::show_without_activation(controller.own_window.load(Ordering::SeqCst));
    } else {
        sync_idle_visibility(app);
    }
}

/// Keeps the idle notch visible for `duration`, even if nothing else wants
/// it (a countdown that just ended peeks its alert in it).
pub fn hold_visible(app: &AppHandle, duration: Duration) {
    let controller = app.state::<NotchController>();
    let generation = controller.hold_generation.fetch_add(1, Ordering::SeqCst) + 1;
    controller.held.store(true, Ordering::SeqCst);
    sync_idle_visibility(app);
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(duration);
        let controller = app.state::<NotchController>();
        if controller.hold_generation.load(Ordering::SeqCst) == generation {
            controller.held.store(false, Ordering::SeqCst);
            sync_idle_visibility(&app);
        }
    });
}

/// A fullscreen app (game, video) covers the notch's monitor.
pub fn is_covered_by_fullscreen_app(app: &AppHandle) -> bool {
    app.state::<NotchController>().fullscreen_app_active.load(Ordering::SeqCst)
}

/// The focused window fills a monitor (any of them): a game, a video.
pub fn is_fullscreen_app_focused(app: &AppHandle) -> bool {
    app.state::<NotchController>().fullscreen_app_focused.load(Ordering::SeqCst)
}

/// The idle notch shows when the user keeps it, or while an activity (media,
/// a timer) is shown in it.
fn idle_notch_wanted(app: &AppHandle) -> bool {
    let (always, while_playing, while_timing) = app.state::<SettingsStore>().read(|settings| {
        (
            settings.spotlight.enabled && settings.spotlight.show_idle_notch,
            settings.music.enabled && settings.music.show_in_notch,
            settings.timer.enabled && settings.timer.show_in_notch,
        )
    });
    always
        || app.state::<NotchController>().held.load(Ordering::SeqCst)
        || (while_playing && app.state::<MediaService>().is_active())
        || (while_timing && app.state::<TimerService>().is_active())
}

fn place_on_primary_monitor(app: &AppHandle, window: &WebviewWindow, controller: &NotchController) {
    match app.primary_monitor() {
        Ok(Some(monitor)) => place_on_monitor(window, &monitor, controller, true),
        Ok(None) => log::warn!("No primary monitor reported"),
        Err(error) => log::warn!("Cannot read the primary monitor: {error}"),
    }
}

fn place_on_monitor(window: &WebviewWindow, monitor: &Monitor, controller: &NotchController, primary: bool) {
    let placement = compute_placement(monitor, primary);
    // Position first so the size is applied with the target monitor's DPI.
    log_failure("move the notch", window.set_position(PhysicalPosition::new(placement.x, placement.y)));
    log_failure("size the notch", window.set_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT)));
    controller.set_placement(placement);
}

/// Screen area the notch window covers, for the Acrylic material.
fn capture_backdrop(placement: &Placement) -> Option<String> {
    let rect = (placement.x, placement.y, placement.x + placement.width, placement.y + placement.height);
    backdrop::snapshot_data_url(rect)
        .map_err(|error| log::warn!("No Acrylic backdrop: {error}"))
        .ok()
}

fn compute_placement(monitor: &Monitor, primary: bool) -> Placement {
    let scale = monitor.scale_factor();
    let width = (WINDOW_WIDTH * scale).round() as i32;
    let height = (WINDOW_HEIGHT * scale).round() as i32;
    let position = monitor.position();
    let size = monitor.size();
    Placement {
        x: position.x + (size.width as i32 - width) / 2,
        y: position.y,
        width,
        height,
        scale,
        on_primary_monitor: primary,
    }
}

fn cursor_monitor(app: &AppHandle) -> Option<Monitor> {
    let (x, y) = platform::cursor_position()?;
    app.monitor_from_point(x as f64, y as f64).ok().flatten()
}

fn is_primary(app: &AppHandle, monitor: &Monitor) -> bool {
    app.primary_monitor()
        .ok()
        .flatten()
        .is_some_and(|primary| primary.position() == monitor.position())
}

/// True when the pointer is anywhere on the notch window (the open panel).
pub fn is_in_window(placement: &Placement, x: i32, y: i32) -> bool {
    x >= placement.x && x < placement.x + placement.width && y >= placement.y && y < placement.y + placement.height
}

/// True when the pointer is on the idle notch (plus a small forgiving margin).
pub fn is_in_hover_zone(placement: &Placement, x: i32, y: i32) -> bool {
    let center = placement.x + placement.width / 2;
    let half_width = ((IDLE_LIVE_NOTCH_WIDTH / 2.0 + HOVER_ZONE_PADDING) * placement.scale) as i32;
    let bottom = placement.y + ((IDLE_NOTCH_HEIGHT + HOVER_ZONE_PADDING) * placement.scale) as i32;
    x >= center - half_width && x <= center + half_width && y >= placement.y && y <= bottom
}

/// True when the pointer is on the bubble beside the folded notch.
pub fn is_in_bubble_zone(placement: &Placement, x: i32, y: i32) -> bool {
    let center = placement.x + placement.width / 2;
    let left = center + ((IDLE_LIVE_NOTCH_WIDTH / 2.0 + BUBBLE_GAP - HOVER_ZONE_PADDING / 2.0) * placement.scale) as i32;
    let right = center + ((IDLE_LIVE_NOTCH_WIDTH / 2.0 + BUBBLE_GAP + BUBBLE_SIZE + HOVER_ZONE_PADDING) * placement.scale) as i32;
    let bottom = placement.y + ((BUBBLE_SIZE + HOVER_ZONE_PADDING) * placement.scale) as i32;
    x > left && x <= right && y >= placement.y && y <= bottom
}

/// Polls the pointer (cheap: one system call) instead of installing a global
/// mouse hook, which would add latency to every mouse move system-wide.
fn start_pointer_watch(app: AppHandle) {
    let spawned = thread::Builder::new().name("notch-pointer-watch".into()).spawn(move || {
        let mut dwell_ticks = 0u32;
        let mut hovered_zone: Option<HoverZone> = None;
        let mut drop_target = DropTarget::default();
        let mut tick = 0u32;
        loop {
            thread::sleep(POINTER_POLL_INTERVAL);
            tick = tick.wrapping_add(1);
            let controller = app.state::<NotchController>();
            let placement = controller.placement();

            if tick.is_multiple_of(FULLSCREEN_CHECK_EVERY_TICKS) {
                refresh_fullscreen_state(&app, &controller, &placement);
            }
            let idle = !controller.is_open.load(Ordering::SeqCst);
            let unobstructed = !controller.fullscreen_app_active.load(Ordering::SeqCst);
            let notch_shown = idle_notch_wanted(&app);
            let (hover_to_open, shelf_enabled) =
                app.state::<SettingsStore>().read(|settings| (settings.spotlight.hover_to_open, settings.recents.enabled));
            let cursor = platform::cursor_position();
            let button_down = platform::left_button_down();

            // Files can be dropped on the folded notch and on the open one.
            let drop_allowed = shelf_enabled && unobstructed && (notch_shown || !idle);
            let in_drop_zone = drop_allowed
                && cursor.is_some_and(|(x, y)| if idle { is_in_hover_zone(&placement, x, y) } else { is_in_window(&placement, x, y) });
            drop_target.update(button_down, in_drop_zone, &placement);

            // A held button is a drag (a file, a window): the drop zone
            // handles it, resting on the notch must not open it.
            if !(hover_to_open && notch_shown && idle && unobstructed) || button_down {
                dwell_ticks = 0;
                continue;
            }

            let bubble = controller.bubble.load(Ordering::SeqCst);
            // The bubble is checked first: its zone touches the notch's padding.
            let zone = cursor.and_then(|(x, y)| {
                if bubble && is_in_bubble_zone(&placement, x, y) {
                    Some(HoverZone::Bubble)
                } else {
                    is_in_hover_zone(&placement, x, y).then_some(HoverZone::Notch)
                }
            });
            if zone != hovered_zone {
                dwell_ticks = 0;
                hovered_zone = zone;
            }
            dwell_ticks = if zone.is_some() { dwell_ticks + 1 } else { 0 };
            if let (true, Some(zone)) = (dwell_ticks >= HOVER_DWELL_TICKS, zone) {
                dwell_ticks = 0;
                open_at(&app, OpenSource::Hover, zone);
            }
        }
    });
    if let Err(error) = spawned {
        log::error!("Cannot start the notch pointer watcher: {error}");
    }
}

/// Lets files be dropped on the notch (the Recents shelf).
///
/// WebView2 refuses file drops, so a native drop window (see
/// `platform::start_drop_zone`) is laid over the notch window when a drag
/// comes in: the button went down elsewhere and the pointer reached the
/// notch with it still held. A press on the notch itself (dragging a Recents
/// card out, a click) never shows it. It stays a moment after the release so
/// the drop is delivered, then goes away.
#[derive(Default)]
struct DropTarget {
    /// The button went down outside the drop zone and is still held.
    dragging_in: bool,
    was_down: bool,
    shown: bool,
    shown_ticks: u32,
    released_ticks: u32,
}

impl DropTarget {
    fn update(&mut self, button_down: bool, in_zone: bool, placement: &Placement) {
        if button_down && !self.was_down {
            self.dragging_in = !in_zone;
        }
        self.was_down = button_down;
        if button_down {
            self.released_ticks = 0;
            if !self.shown && self.dragging_in && in_zone {
                self.shown = true;
                self.shown_ticks = 0;
                log::debug!("A drag reached the notch: drop zone shown");
                platform::show_drop_zone(placement.x, placement.y, placement.width, placement.height);
            } else if self.shown {
                // The notch window rises when it opens: stay above it.
                self.shown_ticks += 1;
                if self.shown_ticks.is_multiple_of(DROP_ZONE_RAISE_EVERY_TICKS) {
                    platform::raise_drop_zone();
                }
            }
            return;
        }
        self.dragging_in = false;
        if self.shown {
            self.released_ticks += 1;
            if self.released_ticks >= DROP_ZONE_LINGER_TICKS {
                self.shown = false;
                log::debug!("Drag over: drop zone hidden");
                platform::hide_drop_zone();
            }
        }
    }
}

/// Starts the native drop window and routes its drags: files coming in open
/// the notch on Recents, dropped files are kept there.
fn start_drop_zone(app: &AppHandle) {
    let handle = app.clone();
    let started = platform::start_drop_zone(Box::new(move |event| on_drop_zone(&handle, event)));
    if let Err(error) = started {
        log::warn!("{error}");
    }
}

fn on_drop_zone(app: &AppHandle, event: DropZoneEvent) {
    let state = match event {
        DropZoneEvent::Enter => {
            if !app.state::<NotchController>().is_open.load(Ordering::SeqCst) {
                open(app, OpenSource::Drop);
            }
            platform::raise_drop_zone();
            FileDragState::Enter
        }
        DropZoneEvent::Leave => FileDragState::Leave,
        DropZoneEvent::Drop(paths) => {
            recents::add_files(app, paths);
            FileDragState::Drop
        }
    };
    if let Err(error) = app.emit(FILE_DRAG_EVENT, state) {
        log::warn!("Cannot report the file drag: {error}");
    }
}

/// What the web layer is told about a file drag over the notch.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum FileDragState {
    Enter,
    Leave,
    Drop,
}

/// Steps the idle notch aside while a fullscreen app (game, video) covers
/// the monitor it lives on, and pauses the shortcuts while one has the focus
/// on any monitor (a game on a second screen still takes Alt+Space).
fn refresh_fullscreen_state(app: &AppHandle, controller: &NotchController, placement: &Placement) {
    let own_window = controller.own_window.load(Ordering::SeqCst);
    let notch_center = (placement.x + placement.width / 2, placement.y + 1);
    let fullscreen_rect = platform::fullscreen_foreground_rect(own_window);
    let covered = fullscreen_rect.is_some_and(|(left, top, right, bottom)| {
        notch_center.0 >= left && notch_center.0 < right && notch_center.1 >= top && notch_center.1 < bottom
    });
    let previous = controller.fullscreen_app_active.swap(covered, Ordering::SeqCst);
    if previous != covered {
        sync_idle_visibility(app);
    }
    let focused = fullscreen_rect.is_some();
    if controller.fullscreen_app_focused.swap(focused, Ordering::SeqCst) != focused {
        fullscreen_pause::sync(app);
    }
}

fn log_failure<T, E: std::fmt::Display>(action: &str, outcome: Result<T, E>) {
    if let Err(error) = outcome {
        log::warn!("Cannot {action}: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_zone_follows_scale_and_position() {
        let placement = Placement { x: 1000, y: 0, width: 1520, height: 1080, scale: 2.0, on_primary_monitor: true };
        let center = 1000 + 760;
        assert!(is_in_hover_zone(&placement, center, 0));
        assert!(is_in_hover_zone(&placement, center + 200, 70));
        assert!(!is_in_hover_zone(&placement, center + 250, 10));
        assert!(!is_in_hover_zone(&placement, center, 90));
    }

    #[test]
    fn bubble_zone_sits_right_of_the_notch() {
        let placement = Placement { x: 0, y: 0, width: 760, height: 540, scale: 1.0, on_primary_monitor: true };
        let center = 380;
        assert!(is_in_bubble_zone(&placement, center + 135, 15));
        assert!(!is_in_bubble_zone(&placement, center + 60, 15), "the notch itself");
        assert!(!is_in_bubble_zone(&placement, center - 135, 15), "nothing on the left");
        assert!(!is_in_bubble_zone(&placement, center + 135, 60));
    }
}
