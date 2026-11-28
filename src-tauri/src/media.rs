//! "Now playing" for the Music widget: watches the OS media sessions and
//! broadcasts changes; forwards playback commands to the player.
//!
//! Polling twice a second is simpler and sturdier than juggling the session
//! events of every player, and costs next to nothing. Position updates are
//! not broadcast: the web layer extrapolates the position itself, so only
//! real changes (track, play/pause, seek) cross the bridge.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::backdrop::image_extension_from_signature;
use crate::i18n::{tr, Language, Text};
use crate::notch;
use crate::platform::{MediaArtwork, MediaCommand, MediaSessions, MediaSnapshot};
use crate::search::SearchService;
use crate::settings::SettingsStore;

pub const MEDIA_CHANGED_EVENT: &str = "media:changed";
const POLL_INTERVAL: Duration = Duration::from_millis(500);
/// First wait before asking again for a cover the player had not published yet.
const ARTWORK_RETRY_FIRST: Duration = Duration::from_secs(1);
/// Longest wait between two tries, for tracks that really have no cover.
const ARTWORK_RETRY_MAX: Duration = Duration::from_secs(30);
/// A position further than this from the extrapolated one is a seek.
const SEEK_DETECTION_TOLERANCE_MS: u64 = 2_000;
/// Display names for players whose id does not match an installed app.
const KNOWN_SOURCES: &[(&str, &str)] = &[("308046b0af4a39cb", "Firefox"), ("msedge", "Microsoft Edge")];

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub source_name: String,
    /// Launch target of the player, for its icon and "open the player".
    pub source_launch_target: Option<String>,
    /// Cover art (or video thumbnail) as a data URL.
    pub artwork: Option<String>,
    pub playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    /// Unix time (ms) at which `position_ms` was read.
    pub updated_at_ms: u64,
    pub can_toggle: bool,
    pub can_next: bool,
    pub can_previous: bool,
    pub can_seek: bool,
}

/// Playback commands accepted from the web layer.
/// `rename_all` only renames variants; `rename_all_fields` is what turns
/// `position_ms` into the `positionMs` the web layer sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "type")]
pub enum PlaybackRequest {
    TogglePlayPause,
    Next,
    Previous,
    Seek {
        #[serde(deserialize_with = "deserialize_milliseconds")]
        position_ms: u64,
    },
}

/// Positions computed in the browser are fractional (a click lands between
/// two milliseconds); any finite number is accepted, rounded and clamped.
fn deserialize_milliseconds<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let value = f64::deserialize(deserializer)?;
    if !value.is_finite() {
        return Err(serde::de::Error::custom("the position must be a finite number"));
    }
    Ok(value.max(0.0).round() as u64)
}

impl From<PlaybackRequest> for MediaCommand {
    fn from(request: PlaybackRequest) -> Self {
        match request {
            PlaybackRequest::TogglePlayPause => MediaCommand::TogglePlayPause,
            PlaybackRequest::Next => MediaCommand::Next,
            PlaybackRequest::Previous => MediaCommand::Previous,
            PlaybackRequest::Seek { position_ms } => MediaCommand::Seek { position_ms },
        }
    }
}

#[derive(Default)]
pub struct MediaService {
    current: Mutex<Option<NowPlaying>>,
    sessions: Mutex<Option<Arc<MediaSessions>>>,
    wake: Mutex<Option<Sender<()>>>,
}

impl MediaService {
    pub fn now_playing(&self) -> Option<NowPlaying> {
        self.current.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    pub fn is_active(&self) -> bool {
        self.current.lock().map(|current| current.is_some()).unwrap_or(false)
    }

    fn sessions(&self) -> Option<Arc<MediaSessions>> {
        self.sessions.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    /// Blocking: waits for the player to accept the command. The technical
    /// cause is logged; the user gets a translated, readable message.
    pub fn send(&self, request: PlaybackRequest, language: Language) -> Result<(), String> {
        let sessions = self.sessions().ok_or_else(|| tr(language, Text::ErrorMediaUnavailable).to_string())?;
        if !self.is_active() {
            return Err(tr(language, Text::ErrorNothingPlaying).to_string());
        }
        let outcome = sessions.send(request.into()).map_err(|error| {
            log::warn!("Playback command {request:?} failed: {error}");
            tr(language, Text::ErrorPlayerRefused).to_string()
        });
        // Reflect the result right away instead of at the next poll.
        if let Some(wake) = self.wake.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).as_ref() {
            let _ = wake.send(());
        }
        outcome
    }
}

pub fn start_watcher(app: &AppHandle) {
    let app = app.clone();
    let spawned = thread::Builder::new().name("media-watcher".into()).spawn(move || {
        let sessions = match MediaSessions::connect() {
            Ok(sessions) => Arc::new(sessions),
            Err(error) => {
                log::warn!("{error}");
                return;
            }
        };
        let (wake, wakeups) = mpsc::channel();
        {
            let service = app.state::<MediaService>();
            *service.sessions.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Arc::clone(&sessions));
            *service.wake.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(wake);
        }

        let mut artwork_cache = ArtworkCache::default();
        loop {
            let enabled = app.state::<SettingsStore>().read(|settings| settings.music.enabled);
            let next = if enabled { read_now_playing(&app, &sessions, &mut artwork_cache) } else { None };
            publish(&app, next);
            match wakeups.recv_timeout(POLL_INTERVAL) {
                Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });
    if let Err(error) = spawned {
        log::error!("Cannot start the media watcher: {error}");
    }
}

/// Cover art only changes with the track; fetching it every poll would waste
/// a stream read twice a second.
///
/// Browsers (YouTube) and phone bridges often publish the title before the
/// cover: a track read in between has no cover yet. Reading it once per track
/// would then keep the placeholder until the next track (or for good when it
/// kept happening), so a missing cover is asked again, less and less often.
#[derive(Default)]
struct ArtworkCache {
    track_key: String,
    artwork: Option<String>,
    /// Reads of the current track that found no cover.
    misses: u32,
    last_attempt: Option<Instant>,
}

impl ArtworkCache {
    /// A new track, or a cover still missing and due for another try.
    fn wants_read(&self, track_key: &str, now: Instant) -> bool {
        if self.track_key != track_key {
            return true;
        }
        self.artwork.is_none()
            && self.last_attempt.is_none_or(|attempt| now.duration_since(attempt) >= artwork_retry_delay(self.misses))
    }
}

/// 1 s, 2 s, 4 s... up to 30 s between tries for a cover that is still missing.
fn artwork_retry_delay(misses: u32) -> Duration {
    let doubled = ARTWORK_RETRY_FIRST.saturating_mul(1 << misses.saturating_sub(1).min(5));
    doubled.min(ARTWORK_RETRY_MAX)
}

fn read_now_playing(app: &AppHandle, sessions: &MediaSessions, cache: &mut ArtworkCache) -> Option<NowPlaying> {
    let snapshot = match sessions.snapshot() {
        Ok(Some(snapshot)) if !snapshot.title.trim().is_empty() => snapshot,
        Ok(_) => return None,
        Err(error) => {
            log::debug!("{error}");
            return None;
        }
    };

    let track_key = format!("{}|{}|{}|{}", snapshot.source_app_id, snapshot.title, snapshot.artist, snapshot.album);
    let now = Instant::now();
    if cache.wants_read(&track_key, now) {
        if cache.track_key != track_key {
            cache.track_key = track_key;
            cache.misses = 0;
        }
        cache.last_attempt = Some(now);
        cache.artwork = match sessions.artwork() {
            Ok(artwork) => artwork.and_then(|artwork| to_data_url(artwork, &snapshot.source_app_id, cache.misses == 0)),
            Err(error) => {
                // Logged in release builds too: a cover that never comes must be traceable.
                if cache.misses == 0 {
                    log::info!("{error}");
                }
                None
            }
        };
        if cache.artwork.is_some() {
            if cache.misses > 0 {
                log::debug!("Cover art from {} arrived after {} tries", snapshot.source_app_id, cache.misses + 1);
            }
        } else {
            cache.misses = cache.misses.saturating_add(1);
        }
    }

    let source = app.state::<SearchService>().find_app_for_source(&snapshot.source_app_id);
    Some(build_now_playing(snapshot, source, cache.artwork.clone(), unix_millis()))
}

fn build_now_playing(
    snapshot: MediaSnapshot,
    source: Option<(String, String)>,
    artwork: Option<String>,
    now_ms: u64,
) -> NowPlaying {
    let (source_name, source_launch_target) = match source {
        Some((name, target)) => (name, Some(target)),
        None => (friendly_source_name(&snapshot.source_app_id), None),
    };
    NowPlaying {
        title: snapshot.title,
        artist: snapshot.artist,
        album: snapshot.album,
        source_name,
        source_launch_target,
        artwork,
        playing: snapshot.playing,
        position_ms: snapshot.position_ms,
        duration_ms: snapshot.duration_ms,
        updated_at_ms: now_ms,
        can_toggle: snapshot.can_toggle,
        can_next: snapshot.can_next,
        can_previous: snapshot.can_previous,
        can_seek: snapshot.can_seek,
    }
}

/// "Spotify.exe" -> "Spotify", "Vivaldi.ABC123" -> "Vivaldi".
fn friendly_source_name(source_app_id: &str) -> String {
    let lowercase = source_app_id.to_lowercase();
    if let Some((_, name)) = KNOWN_SOURCES.iter().find(|(id, _)| lowercase.starts_with(id)) {
        return (*name).to_string();
    }
    let stem = source_app_id.trim_end_matches(".exe").split(['.', '!', '_']).next().unwrap_or(source_app_id);
    let mut characters = stem.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => "Media".to_string(),
    }
}

/// Only bytes that really are an image become a data URL: some players
/// (phone bridges...) hand out thumbnails the webview cannot decode, which
/// would show a broken image instead of the placeholder cover.
/// `report`: log an undecodable cover (only on the first try of a track).
fn to_data_url(artwork: MediaArtwork, source_app_id: &str, report: bool) -> Option<String> {
    let Some(extension) = image_extension_from_signature(&artwork.bytes) else {
        if !report {
            return None;
        }
        let header: Vec<String> = artwork.bytes.iter().take(8).map(|byte| format!("{byte:02x}")).collect();
        log::info!(
            "Cover art from {source_app_id} ignored: not a known image format ({} bytes, type \"{}\", starts with {})",
            artwork.bytes.len(),
            artwork.content_type,
            header.join(" ")
        );
        return None;
    };
    let content_type = match extension {
        "jpg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/bmp",
    };
    Some(format!("data:{content_type};base64,{}", STANDARD.encode(artwork.bytes)))
}

/// Broadcasts only meaningful changes, and refreshes the idle notch when
/// media starts or stops (it may be shown only while something plays).
fn publish(app: &AppHandle, next: Option<NowPlaying>) {
    let service = app.state::<MediaService>();
    let (changed, presence_changed) = {
        let mut current = service.current.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let changed = match (current.as_ref(), next.as_ref()) {
            (None, None) => false,
            (Some(previous), Some(candidate)) => is_meaningful_change(previous, candidate),
            _ => true,
        };
        let presence_changed = current.is_some() != next.is_some();
        if let (Some(previous), Some(candidate)) = (current.as_ref(), next.as_ref()) {
            if previous.title != candidate.title || previous.artist != candidate.artist {
                log::info!("New track from {} (playing: {})", candidate.source_name, candidate.playing);
            }
        }
        if changed {
            *current = next.clone();
        }
        (changed, presence_changed)
    };
    if !changed {
        return;
    }
    if let Err(error) = app.emit(MEDIA_CHANGED_EVENT, &next) {
        log::warn!("Cannot broadcast media state: {error}");
    }
    if presence_changed {
        match next.as_ref() {
            Some(playing) => log::info!("Media session from {} detected", playing.source_name),
            None => log::info!("No media session anymore"),
        }
        notch::sync_idle_visibility(app);
    }
}

/// Everything but the natural progress of the position counts as a change.
fn is_meaningful_change(previous: &NowPlaying, next: &NowPlaying) -> bool {
    let comparable = |state: &NowPlaying| NowPlaying { position_ms: 0, updated_at_ms: 0, ..state.clone() };
    if comparable(previous) != comparable(next) {
        return true;
    }
    let expected = if previous.playing {
        previous.position_ms + next.updated_at_ms.saturating_sub(previous.updated_at_ms)
    } else {
        previous.position_ms
    };
    expected.abs_diff(next.position_ms) > SEEK_DETECTION_TOLERANCE_MS
}

fn unix_millis() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_millis() as u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_covers_are_asked_again_less_and_less_often() {
        assert_eq!(artwork_retry_delay(1), Duration::from_secs(1));
        assert_eq!(artwork_retry_delay(2), Duration::from_secs(2));
        assert_eq!(artwork_retry_delay(3), Duration::from_secs(4));
        assert_eq!(artwork_retry_delay(20), ARTWORK_RETRY_MAX);
    }

    #[test]
    fn a_cover_is_read_for_a_new_track_and_retried_only_while_missing() {
        let start = Instant::now();
        let mut cache = ArtworkCache { track_key: "a".into(), artwork: None, misses: 1, last_attempt: Some(start) };
        assert!(cache.wants_read("b", start), "a new track is always read");
        assert!(!cache.wants_read("a", start + Duration::from_millis(500)), "too soon to ask again");
        assert!(cache.wants_read("a", start + Duration::from_secs(1)), "a missing cover is asked again");
        cache.artwork = Some("data:image/png;base64,".into());
        assert!(!cache.wants_read("a", start + Duration::from_secs(60)), "a cover in hand is kept");
    }

    fn track(position_ms: u64, updated_at_ms: u64, playing: bool) -> NowPlaying {
        NowPlaying {
            title: "Stargazing".into(),
            artist: "Myles Smith".into(),
            album: String::new(),
            source_name: "Spotify".into(),
            source_launch_target: None,
            artwork: None,
            playing,
            position_ms,
            duration_ms: 200_000,
            updated_at_ms,
            can_toggle: true,
            can_next: true,
            can_previous: true,
            can_seek: true,
        }
    }

    #[test]
    fn natural_progress_is_not_a_change() {
        assert!(!is_meaningful_change(&track(10_000, 1_000, true), &track(10_500, 1_500, true)));
        assert!(!is_meaningful_change(&track(10_000, 1_000, false), &track(10_000, 9_000, false)));
    }

    #[test]
    fn seeks_and_state_changes_are_changes() {
        assert!(is_meaningful_change(&track(10_000, 1_000, true), &track(60_000, 1_500, true)));
        assert!(is_meaningful_change(&track(10_000, 1_000, true), &track(10_500, 1_500, false)));
    }

    #[test]
    fn playback_requests_match_the_web_payloads() {
        let seek: PlaybackRequest = serde_json::from_str(r#"{ "type": "seek", "positionMs": 205000 }"#).unwrap();
        assert_eq!(seek, PlaybackRequest::Seek { position_ms: 205_000 });
        let fractional: PlaybackRequest =
            serde_json::from_str(r#"{ "type": "seek", "positionMs": 48943.8202247191 }"#).unwrap();
        assert_eq!(fractional, PlaybackRequest::Seek { position_ms: 48_944 });
        let negative: PlaybackRequest = serde_json::from_str(r#"{ "type": "seek", "positionMs": -12.5 }"#).unwrap();
        assert_eq!(negative, PlaybackRequest::Seek { position_ms: 0 });
        let toggle: PlaybackRequest = serde_json::from_str(r#"{ "type": "togglePlayPause" }"#).unwrap();
        assert_eq!(toggle, PlaybackRequest::TogglePlayPause);
    }

    #[test]
    fn source_names_are_readable() {
        assert_eq!(friendly_source_name("Spotify.exe"), "Spotify");
        assert_eq!(friendly_source_name("Vivaldi.ABC123"), "Vivaldi");
        assert_eq!(friendly_source_name("308046B0AF4A39CB"), "Firefox");
    }
}
