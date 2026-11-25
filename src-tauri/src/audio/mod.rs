//! Audio bars of the folded notch: listens to what the speakers play and
//! broadcasts a few frequency band levels about 30 times a second.
//!
//! The capture only runs while the notch actually shows the bars (the web
//! layer asks for it), so an idle or paused machine pays nothing.

mod spectrum;

use std::sync::{Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::notch;
use crate::platform::{self, LoopbackCapture};
use crate::settings::SettingsStore;
use spectrum::SpectrumAnalyzer;

pub const AUDIO_LEVELS_EVENT: &str = "media:levels";
/// Bands per side: the notch mirrors them, bass in the middle.
const BAND_COUNT: usize = 15;
/// ~30 frames a second; the web layer interpolates in between.
const FRAME_INTERVAL: Duration = Duration::from_millis(33);
const RETRY_DELAY: Duration = Duration::from_secs(3);
const DEVICE_CHECK_INTERVAL: Duration = Duration::from_secs(2);
const COVERED_CHECK_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Default)]
pub struct AudioVisualizer {
    wanted: Mutex<bool>,
    changed: Condvar,
}

impl AudioVisualizer {
    pub fn set_wanted(&self, wanted: bool) {
        *self.wanted.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = wanted;
        self.changed.notify_all();
    }

    fn is_wanted(&self) -> bool {
        *self.wanted.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn wait_until_wanted(&self) {
        let guard = self.wanted.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let _guard = self.changed.wait_while(guard, |wanted| !*wanted).unwrap_or_else(|poisoned| poisoned.into_inner());
    }
}

/// Whether the settings allow the bars at all (checked on every request, so
/// a stale web layer cannot keep the capture running).
pub fn bars_enabled(app: &AppHandle) -> bool {
    app.state::<SettingsStore>()
        .read(|settings| settings.music.enabled && settings.music.show_in_notch && settings.music.show_audio_bars)
}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    let spawned = thread::Builder::new().name("audio-visualizer".into()).spawn(move || {
        let _com = platform::enter_com();
        let mut last_error: Option<String> = None;
        loop {
            app.state::<AudioVisualizer>().wait_until_wanted();
            if notch::is_covered_by_fullscreen_app(&app) {
                thread::sleep(COVERED_CHECK_INTERVAL);
                continue;
            }
            match LoopbackCapture::open() {
                Ok(capture) => {
                    last_error = None;
                    if let Err(error) = run_capture(&app, capture) {
                        log::warn!("{error}");
                    }
                    // Bars fall flat rather than freezing mid-air.
                    emit_levels(&app, &[0; BAND_COUNT]);
                }
                Err(error) => {
                    // Logged once, not every retry.
                    if last_error.as_deref() != Some(error.as_str()) {
                        log::warn!("{error}");
                        last_error = Some(error);
                    }
                    thread::sleep(RETRY_DELAY);
                }
            }
        }
    });
    if let Err(error) = spawned {
        log::error!("Cannot start the audio bars: {error}");
    }
}

/// Captures until the bars are no longer wanted, the output device changes
/// (Ok, the caller reopens) or the capture breaks (Err).
fn run_capture(app: &AppHandle, mut capture: LoopbackCapture) -> Result<(), String> {
    let visualizer = app.state::<AudioVisualizer>();
    let mut analyzer = SpectrumAnalyzer::new(capture.sample_rate(), BAND_COUNT);
    let silence_frames = (capture.sample_rate() as u128 * FRAME_INTERVAL.as_millis() / 1000) as usize;
    let mut samples = Vec::with_capacity(spectrum::FFT_SIZE * 2);
    let mut last_device_check = Instant::now();
    let mut was_silent = false;

    while visualizer.is_wanted() && !notch::is_covered_by_fullscreen_app(app) {
        samples.clear();
        capture.read_mono(&mut samples)?;
        if samples.is_empty() {
            analyzer.push_silence(silence_frames);
        } else {
            analyzer.push(&samples);
        }

        let levels: Vec<u8> = analyzer.analyze().iter().map(|level| (level * 255.0).round() as u8).collect();
        let silent = levels.iter().all(|&level| level == 0);
        // Silence is sent once, not thirty times a second.
        if !(silent && was_silent) {
            emit_levels(app, &levels);
        }
        was_silent = silent;

        if last_device_check.elapsed() >= DEVICE_CHECK_INTERVAL {
            if capture.default_device_changed() {
                log::info!("Audio output changed, the audio bars follow it");
                return Ok(());
            }
            last_device_check = Instant::now();
        }
        thread::sleep(FRAME_INTERVAL);
    }
    Ok(())
}

fn emit_levels(app: &AppHandle, levels: &[u8]) {
    if let Err(error) = app.emit(AUDIO_LEVELS_EVENT, levels) {
        log::debug!("Cannot broadcast the audio levels: {error}");
    }
}
