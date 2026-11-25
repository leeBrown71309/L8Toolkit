//! What the speakers play, read through WASAPI loopback capture: the same
//! mix the user hears (any app), in shared mode, without a driver or a
//! virtual device. Only levels are derived from it; nothing is recorded.

use std::ptr;

use windows::core::GUID;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX,
    WAVEFORMATEXTENSIBLE,
};
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL};

use super::com::take_com_string;

const WAVE_FORMAT_PCM: u16 = 1;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
const SUBTYPE_PCM: GUID = GUID::from_u128(0x0000_0001_0000_0010_8000_00aa_0038_9b71);
const SUBTYPE_IEEE_FLOAT: GUID = GUID::from_u128(0x0000_0003_0000_0010_8000_00aa_0038_9b71);
/// Bytes of WAVEFORMATEXTENSIBLE beyond the WAVEFORMATEX header.
const EXTENSIBLE_EXTRA_BYTES: u16 = 22;
/// Shared buffer of 200 ms (in 100 ns units): ample for a reader polling
/// every 33 ms.
const BUFFER_DURATION: i64 = 2_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SampleFormat {
    Float32,
    Int16,
    /// 32-bit container; 24-bit audio is left-aligned in it, so it reads the same.
    Int32,
}

pub struct LoopbackCapture {
    enumerator: IMMDeviceEnumerator,
    client: IAudioClient,
    capture: IAudioCaptureClient,
    device_id: String,
    format: SampleFormat,
    channels: usize,
    sample_rate: u32,
}

impl LoopbackCapture {
    /// Starts capturing the default output device.
    pub fn open() -> Result<Self, String> {
        let describe = |error: windows::core::Error| format!("Cannot listen to the audio output: {error}");
        // SAFETY: COM is initialised on this thread by the caller (enter_com);
        // the mix format pointer is read before being freed exactly once.
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(describe)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).map_err(describe)?;
            let device_id = take_com_string(device.GetId().map_err(describe)?);
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(describe)?;

            let mix_format = client.GetMixFormat().map_err(describe)?;
            let Some((format, channels, sample_rate)) = parse_format(mix_format) else {
                CoTaskMemFree(Some(mix_format as *const _));
                return Err("The audio output uses a sample format the audio bars cannot read".to_string());
            };
            let initialized = client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                BUFFER_DURATION,
                0,
                mix_format,
                None,
            );
            CoTaskMemFree(Some(mix_format as *const _));
            initialized.map_err(describe)?;

            let capture: IAudioCaptureClient = client.GetService().map_err(describe)?;
            client.Start().map_err(describe)?;
            Ok(Self { enumerator, client, capture, device_id, format, channels, sample_rate })
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Appends every sample played since the last call, mixed down to mono.
    /// Nothing is appended while the output is completely idle.
    pub fn read_mono(&mut self, output: &mut Vec<f32>) -> Result<(), String> {
        let describe = |error: windows::core::Error| format!("Audio capture interrupted: {error}");
        // SAFETY: the buffer returned by GetBuffer holds `frames` frames of
        // the negotiated format and stays valid until ReleaseBuffer.
        unsafe {
            loop {
                if self.capture.GetNextPacketSize().map_err(describe)? == 0 {
                    return Ok(());
                }
                let mut data = ptr::null_mut();
                let mut frames = 0u32;
                let mut flags = 0u32;
                self.capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None).map_err(describe)?;
                let frame_count = frames as usize;
                let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
                if silent || data.is_null() {
                    output.extend(std::iter::repeat_n(0.0, frame_count));
                } else {
                    let samples = frame_count * self.channels;
                    match self.format {
                        SampleFormat::Float32 => {
                            mix_down(std::slice::from_raw_parts(data as *const f32, samples), self.channels, output, |value| value)
                        }
                        SampleFormat::Int16 => mix_down(
                            std::slice::from_raw_parts(data as *const i16, samples),
                            self.channels,
                            output,
                            |value| f32::from(value) / 32_768.0,
                        ),
                        SampleFormat::Int32 => mix_down(
                            std::slice::from_raw_parts(data as *const i32, samples),
                            self.channels,
                            output,
                            |value| value as f32 / 2_147_483_648.0,
                        ),
                    }
                }
                self.capture.ReleaseBuffer(frames).map_err(describe)?;
            }
        }
    }

    /// True when Windows now plays through another device (headphones
    /// plugged in...), so the capture must follow it.
    pub fn default_device_changed(&self) -> bool {
        // SAFETY: plain COM calls on interfaces owned by this struct.
        unsafe {
            self.enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .and_then(|device| device.GetId())
                .map(|id| take_com_string(id) != self.device_id)
                .unwrap_or(true)
        }
    }
}

impl Drop for LoopbackCapture {
    fn drop(&mut self) {
        // SAFETY: stopping a started client; failure only means it is gone.
        if let Err(error) = unsafe { self.client.Stop() } {
            log::debug!("Cannot stop the audio capture: {error}");
        }
    }
}

/// Reads the mix format; the struct is packed, hence the unaligned reads.
unsafe fn parse_format(format: *const WAVEFORMATEX) -> Option<(SampleFormat, usize, u32)> {
    let header = unsafe { ptr::read_unaligned(format) };
    let is_float = match header.wFormatTag {
        WAVE_FORMAT_IEEE_FLOAT => true,
        WAVE_FORMAT_PCM => false,
        WAVE_FORMAT_EXTENSIBLE if header.cbSize >= EXTENSIBLE_EXTRA_BYTES => {
            let extensible = unsafe { ptr::read_unaligned(format as *const WAVEFORMATEXTENSIBLE) };
            let sub_format = extensible.SubFormat;
            if sub_format == SUBTYPE_IEEE_FLOAT {
                true
            } else if sub_format == SUBTYPE_PCM {
                false
            } else {
                return None;
            }
        }
        _ => return None,
    };
    let sample_format = match (is_float, header.wBitsPerSample) {
        (true, 32) => SampleFormat::Float32,
        (false, 16) => SampleFormat::Int16,
        (false, 32) => SampleFormat::Int32,
        _ => return None,
    };
    let channels = usize::from(header.nChannels);
    (channels > 0).then_some((sample_format, channels, header.nSamplesPerSec))
}

/// Averages the channels of every frame into one mono sample.
fn mix_down<T: Copy>(interleaved: &[T], channels: usize, output: &mut Vec<f32>, to_float: impl Fn(T) -> f32) {
    let scale = 1.0 / channels as f32;
    output.extend(
        interleaved.chunks_exact(channels).map(|frame| frame.iter().map(|&value| to_float(value)).sum::<f32>() * scale),
    );
}

#[cfg(test)]
mod tests {
    use super::mix_down;

    #[test]
    fn stereo_frames_are_averaged() {
        let mut output = Vec::new();
        mix_down(&[1.0_f32, 0.0, -0.5, -0.5], 2, &mut output, |value| value);
        assert_eq!(output, vec![0.5, -0.5]);
    }

    /// Manual check: run with `--ignored` while something plays.
    #[test]
    #[ignore = "needs sound playing on the default output"]
    fn captures_what_the_speakers_play() {
        let _com = super::super::com::enter_com();
        let mut capture = super::LoopbackCapture::open().expect("the capture opens");
        let mut samples = Vec::new();
        let started = std::time::Instant::now();
        while started.elapsed() < std::time::Duration::from_millis(1500) {
            capture.read_mono(&mut samples).expect("the capture keeps running");
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        let peak = samples.iter().fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        println!("{} samples at {} Hz, peak {peak}", samples.len(), capture.sample_rate());
        assert!(peak > 0.0, "nothing was heard");
    }

    #[test]
    fn integer_samples_are_normalised() {
        let mut output = Vec::new();
        mix_down(&[i16::MIN, i16::MIN], 2, &mut output, |value| f32::from(value) / 32_768.0);
        assert_eq!(output, vec![-1.0]);
    }
}
