//! Now playing, through the Windows media session API (the same source as
//! the volume flyout): Spotify, browsers (YouTube, SoundCloud...), Media
//! Player and any app that integrates with Windows media controls.

use std::time::{SystemTime, UNIX_EPOCH};

use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};
use windows::Storage::Streams::DataReader;

use crate::platform::{MediaArtwork, MediaCommand, MediaSnapshot};

/// 100-nanosecond ticks between 1601-01-01 (Windows epoch) and 1970-01-01.
const WINDOWS_TO_UNIX_EPOCH_TICKS: i64 = 116_444_736_000_000_000;
const TICKS_PER_MILLISECOND: i64 = 10_000;
/// Cover art larger than this is not worth sending to the web layer.
const MAX_ARTWORK_BYTES: u32 = 4 * 1024 * 1024;

pub struct MediaSessions {
    manager: SessionManager,
}

impl MediaSessions {
    pub fn connect() -> Result<Self, String> {
        let manager = SessionManager::RequestAsync()
            .and_then(|operation| operation.join())
            .map_err(|error| format!("Media sessions are unavailable: {error}"))?;
        Ok(Self { manager })
    }

    /// A playing session wins over the "current" one, so a paused Spotify
    /// does not hide the YouTube video playing right now.
    fn preferred_session(&self) -> Option<Session> {
        if let Ok(sessions) = self.manager.GetSessions() {
            let playing = sessions.into_iter().find(|session| {
                session
                    .GetPlaybackInfo()
                    .and_then(|info| info.PlaybackStatus())
                    .is_ok_and(|status| status == PlaybackStatus::Playing)
            });
            if playing.is_some() {
                return playing;
            }
        }
        self.manager.GetCurrentSession().ok()
    }

    pub fn snapshot(&self) -> Result<Option<MediaSnapshot>, String> {
        let Some(session) = self.preferred_session() else { return Ok(None) };
        read_session(&session).map(Some).map_err(|error| format!("Cannot read the media session: {error}"))
    }

    pub fn artwork(&self) -> Result<Option<MediaArtwork>, String> {
        let Some(session) = self.preferred_session() else { return Ok(None) };
        read_artwork(&session).map_err(|error| format!("Cannot read the cover art: {error}"))
    }

    pub fn send(&self, command: MediaCommand) -> Result<(), String> {
        let session = self.preferred_session().ok_or("Nothing is playing")?;
        let accepted = match command {
            MediaCommand::TogglePlayPause => session.TryTogglePlayPauseAsync().and_then(|operation| operation.join()),
            MediaCommand::Next => session.TrySkipNextAsync().and_then(|operation| operation.join()),
            MediaCommand::Previous => session.TrySkipPreviousAsync().and_then(|operation| operation.join()),
            MediaCommand::Seek { position_ms } => {
                let start = session.GetTimelineProperties().and_then(|timeline| timeline.StartTime()).map(|time| time.Duration).unwrap_or(0);
                let ticks = start + position_ms as i64 * TICKS_PER_MILLISECOND;
                session.TryChangePlaybackPositionAsync(ticks).and_then(|operation| operation.join())
            }
        }
        .map_err(|error| format!("The player did not respond: {error}"))?;
        if accepted { Ok(()) } else { Err("The player refused this action".to_string()) }
    }
}

fn read_session(session: &Session) -> windows::core::Result<MediaSnapshot> {
    let properties = session.TryGetMediaPropertiesAsync()?.join()?;
    let info = session.GetPlaybackInfo()?;
    let controls = info.Controls()?;
    let playing = info.PlaybackStatus()? == PlaybackStatus::Playing;
    let (position_ms, duration_ms) = read_timeline(session, playing);

    Ok(MediaSnapshot {
        source_app_id: session.SourceAppUserModelId()?.to_string(),
        title: properties.Title()?.to_string(),
        artist: properties.Artist()?.to_string(),
        album: properties.AlbumTitle().map(|album| album.to_string()).unwrap_or_default(),
        playing,
        position_ms,
        duration_ms,
        can_toggle: controls.IsPlayPauseToggleEnabled().unwrap_or(false),
        can_next: controls.IsNextEnabled().unwrap_or(false),
        can_previous: controls.IsPreviousEnabled().unwrap_or(false),
        can_seek: controls.IsPlaybackPositionEnabled().unwrap_or(false),
    })
}

/// Players report the position at their last update; while playing, the
/// time elapsed since then is added so every reading is "as of now".
fn read_timeline(session: &Session, playing: bool) -> (u64, u64) {
    let Ok(timeline) = session.GetTimelineProperties() else { return (0, 0) };
    let start = timeline.StartTime().map(|time| time.Duration).unwrap_or(0);
    let end = timeline.EndTime().map(|time| time.Duration).unwrap_or(0);
    let position = timeline.Position().map(|time| time.Duration).unwrap_or(0) - start;
    let duration = (end - start).max(0);

    let elapsed = if playing {
        timeline
            .LastUpdatedTime()
            .map(|updated| (now_windows_ticks() - updated.UniversalTime).max(0))
            .unwrap_or(0)
    } else {
        0
    };
    let mut current = (position + elapsed).max(0);
    if duration > 0 {
        current = current.min(duration);
    }
    ((current / TICKS_PER_MILLISECOND) as u64, (duration / TICKS_PER_MILLISECOND) as u64)
}

fn read_artwork(session: &Session) -> windows::core::Result<Option<MediaArtwork>> {
    let properties = session.TryGetMediaPropertiesAsync()?.join()?;
    let Ok(reference) = properties.Thumbnail() else { return Ok(None) };
    let stream = reference.OpenReadAsync()?.join()?;
    let size = stream.Size()? as u32;
    if size == 0 || size > MAX_ARTWORK_BYTES {
        return Ok(None);
    }
    let content_type = stream.ContentType().map(|value| value.to_string()).unwrap_or_default();
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
    // The stream may deliver less than its announced size (phone bridges do).
    let loaded = reader.LoadAsync(size)?.join()?.min(size);
    if loaded == 0 {
        return Ok(None);
    }
    let mut bytes = vec![0u8; loaded as usize];
    reader.ReadBytes(&mut bytes)?;
    Ok(Some(MediaArtwork { bytes, content_type }))
}

fn now_windows_ticks() -> i64 {
    let unix_ticks = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| (duration.as_nanos() / 100) as i64)
        .unwrap_or(0);
    unix_ticks + WINDOWS_TO_UNIX_EPOCH_TICKS
}
