//! Turns raw audio samples into a handful of frequency band levels (0 to 1)
//! for the audio bars of the notch. Pure computation, no OS types.
//!
//! Levels follow loudness on purpose (fixed decibel scale, no automatic
//! gain): quiet passages give short bars, loud ones tall bars.

use std::f32::consts::PI;

/// 2048 samples is ~43 ms at 48 kHz: enough resolution for the bass bands
/// while staying responsive.
pub const FFT_SIZE: usize = 2048;
const MIN_FREQUENCY: f32 = 45.0;
const MAX_FREQUENCY: f32 = 14_000.0;
/// Band levels below this are silence, above the ceiling they are full.
const FLOOR_DB: f32 = -68.0;
const CEILING_DB: f32 = -12.0;
/// Music carries less energy in high frequencies; tilting the response keeps
/// the outer bars alive instead of flat.
const TILT_DB_PER_OCTAVE: f32 = 3.0;
const TILT_REFERENCE_HZ: f32 = 1_000.0;
/// Fraction of the distance to the new level covered each frame: bars jump
/// up quickly and fall back slowly, like a VU meter.
const ATTACK: f32 = 0.7;
const RELEASE: f32 = 0.22;

pub struct SpectrumAnalyzer {
    sample_rate: f32,
    window: Vec<f32>,
    window_sum: f32,
    /// Last `FFT_SIZE` samples, oldest first once rotated by `write_index`.
    history: Vec<f32>,
    write_index: usize,
    /// FFT bin range of each band (start inclusive, end exclusive).
    bands: Vec<(usize, usize)>,
    levels: Vec<f32>,
    real: Vec<f32>,
    imaginary: Vec<f32>,
}

impl SpectrumAnalyzer {
    pub fn new(sample_rate: u32, band_count: usize) -> Self {
        let sample_rate = sample_rate.max(1) as f32;
        // Hann window: tames the leakage that would light every band at once.
        let window: Vec<f32> =
            (0..FFT_SIZE).map(|index| 0.5 - 0.5 * (2.0 * PI * index as f32 / (FFT_SIZE - 1) as f32).cos()).collect();
        let window_sum = window.iter().sum();
        Self {
            sample_rate,
            window,
            window_sum,
            history: vec![0.0; FFT_SIZE],
            write_index: 0,
            bands: band_bins(sample_rate, band_count),
            levels: vec![0.0; band_count],
            real: vec![0.0; FFT_SIZE],
            imaginary: vec![0.0; FFT_SIZE],
        }
    }

    /// Appends mono samples in [-1, 1].
    pub fn push(&mut self, samples: &[f32]) {
        for &sample in samples {
            self.history[self.write_index] = sample;
            self.write_index = (self.write_index + 1) % FFT_SIZE;
        }
    }

    /// Nothing was played for `frames` samples (the loopback stream goes
    /// quiet instead of sending zeros).
    pub fn push_silence(&mut self, frames: usize) {
        for _ in 0..frames.min(FFT_SIZE) {
            self.history[self.write_index] = 0.0;
            self.write_index = (self.write_index + 1) % FFT_SIZE;
        }
    }

    /// Smoothed level of every band, from low to high frequencies.
    pub fn analyze(&mut self) -> &[f32] {
        for index in 0..FFT_SIZE {
            let sample = self.history[(self.write_index + index) % FFT_SIZE];
            self.real[index] = sample * self.window[index];
            self.imaginary[index] = 0.0;
        }
        fft_in_place(&mut self.real, &mut self.imaginary);

        let bin_width = self.sample_rate / FFT_SIZE as f32;
        for (band, &(start, end)) in self.bands.iter().enumerate() {
            // Peak bin of the band: a single loud note should show.
            let magnitude = (start..end)
                .map(|bin| (self.real[bin].powi(2) + self.imaginary[bin].powi(2)).sqrt())
                .fold(0.0_f32, f32::max);
            // Scaled so a full-scale sine reads 0 dB.
            let amplitude = magnitude * 2.0 / self.window_sum;
            let center = ((start + end) as f32 / 2.0) * bin_width;
            let decibels = 20.0 * (amplitude + 1e-9).log10() + TILT_DB_PER_OCTAVE * (center / TILT_REFERENCE_HZ).log2();
            let target = ((decibels - FLOOR_DB) / (CEILING_DB - FLOOR_DB)).clamp(0.0, 1.0);
            let level = &mut self.levels[band];
            let speed = if target > *level { ATTACK } else { RELEASE };
            *level += (target - *level) * speed;
            if *level < 0.002 {
                *level = 0.0;
            }
        }
        &self.levels
    }
}

/// Logarithmically spaced bands (each one an equal musical interval), each
/// covering at least one bin and never overlapping the previous one.
fn band_bins(sample_rate: f32, band_count: usize) -> Vec<(usize, usize)> {
    let nyquist_bin = FFT_SIZE / 2;
    let max_frequency = MAX_FREQUENCY.min(sample_rate / 2.0);
    let ratio = (max_frequency / MIN_FREQUENCY).powf(1.0 / band_count.max(1) as f32);
    let to_bin = |frequency: f32| ((frequency * FFT_SIZE as f32 / sample_rate).round() as usize).clamp(1, nyquist_bin);

    let mut bands = Vec::with_capacity(band_count);
    let mut previous_end = 1;
    for band in 0..band_count {
        let low = MIN_FREQUENCY * ratio.powi(band as i32);
        let high = low * ratio;
        let start = to_bin(low).max(previous_end).min(nyquist_bin - 1);
        let end = to_bin(high).max(start + 1).min(nyquist_bin);
        bands.push((start, end));
        previous_end = end;
    }
    bands
}

/// Iterative radix-2 Cooley-Tukey FFT; `real.len()` must be a power of two.
fn fft_in_place(real: &mut [f32], imaginary: &mut [f32]) {
    let size = real.len();
    debug_assert!(size.is_power_of_two() && imaginary.len() == size);

    // Bit-reversal permutation.
    let mut reversed = 0;
    for index in 1..size {
        let mut bit = size >> 1;
        while reversed & bit != 0 {
            reversed ^= bit;
            bit >>= 1;
        }
        reversed |= bit;
        if index < reversed {
            real.swap(index, reversed);
            imaginary.swap(index, reversed);
        }
    }

    let mut length = 2;
    while length <= size {
        let angle = -2.0 * PI / length as f32;
        let (step_sin, step_cos) = angle.sin_cos();
        for chunk_start in (0..size).step_by(length) {
            let (mut twiddle_real, mut twiddle_imaginary) = (1.0_f32, 0.0_f32);
            for offset in 0..length / 2 {
                let even = chunk_start + offset;
                let odd = even + length / 2;
                let odd_real = real[odd] * twiddle_real - imaginary[odd] * twiddle_imaginary;
                let odd_imaginary = real[odd] * twiddle_imaginary + imaginary[odd] * twiddle_real;
                real[odd] = real[even] - odd_real;
                imaginary[odd] = imaginary[even] - odd_imaginary;
                real[even] += odd_real;
                imaginary[even] += odd_imaginary;
                let next_real = twiddle_real * step_cos - twiddle_imaginary * step_sin;
                twiddle_imaginary = twiddle_real * step_sin + twiddle_imaginary * step_cos;
                twiddle_real = next_real;
            }
        }
        length <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: u32 = 48_000;
    const BANDS: usize = 15;

    fn sine(frequency: f32, amplitude: f32) -> Vec<f32> {
        (0..FFT_SIZE).map(|index| amplitude * (2.0 * PI * frequency * index as f32 / SAMPLE_RATE as f32).sin()).collect()
    }

    fn settled_levels(samples: &[f32]) -> Vec<f32> {
        let mut analyzer = SpectrumAnalyzer::new(SAMPLE_RATE, BANDS);
        analyzer.push(samples);
        for _ in 0..30 {
            analyzer.analyze();
        }
        analyzer.analyze().to_vec()
    }

    fn band_of(frequency: f32) -> usize {
        let bin = (frequency * FFT_SIZE as f32 / SAMPLE_RATE as f32).round() as usize;
        band_bins(SAMPLE_RATE as f32, BANDS).iter().position(|&(start, end)| (start..end).contains(&bin)).unwrap()
    }

    #[test]
    fn silence_gives_flat_bars() {
        assert!(settled_levels(&vec![0.0; FFT_SIZE]).iter().all(|&level| level == 0.0));
    }

    #[test]
    fn a_tone_lights_its_own_band_most() {
        let levels = settled_levels(&sine(1_000.0, 0.5));
        let loudest = levels.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0;
        assert_eq!(loudest, band_of(1_000.0));
        assert!(levels[loudest] > 0.8);
        assert!(levels[0] < 0.2, "the bass band stays low: {levels:?}");
    }

    #[test]
    fn louder_sound_gives_taller_bars() {
        let band = band_of(440.0);
        let quiet = settled_levels(&sine(440.0, 0.02))[band];
        let loud = settled_levels(&sine(440.0, 0.4))[band];
        assert!(loud > quiet + 0.2, "quiet {quiet}, loud {loud}");
    }

    #[test]
    fn bands_are_ordered_and_never_empty() {
        let bands = band_bins(SAMPLE_RATE as f32, BANDS);
        assert_eq!(bands.len(), BANDS);
        for window in bands.windows(2) {
            assert!(window[0].1 <= window[1].0);
        }
        assert!(bands.iter().all(|&(start, end)| end > start && start >= 1));
    }
}
