//! Oscillator module - waveform generation with anti-aliasing

use crate::dsp::{polyblep, saw_polyblep, square_polyblep};

/// Waveform types
#[derive(Debug, Clone)]
pub enum Waveform {
    Sine,
    Saw,
    Square { pulse_width: f32 },
    Triangle,
    Noise,
}

impl Default for Waveform {
    fn default() -> Self {
        Waveform::Saw
    }
}

/// Oscillator with anti-aliasing
#[derive(Debug, Clone)]
pub struct Oscillator {
    waveform: Waveform,
    frequency: f32,
    phase: f32,
    detune: f32, // cents
    sample_rate: f32,
}

impl Oscillator {
    /// Create new oscillator
    pub fn new(waveform: Waveform, sample_rate: u32) -> Self {
        Self {
            waveform,
            frequency: 440.0,
            phase: 0.0,
            detune: 0.0,
            sample_rate: sample_rate as f32,
        }
    }

    /// Set frequency in Hz
    pub fn set_frequency(&mut self, freq: f32) {
        self.frequency = freq.clamp(20.0, 20000.0);
    }

    /// Set detune in cents
    pub fn set_detune(&mut self, cents: f32) {
        self.detune = cents;
    }

    /// Set waveform
    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    /// Reset phase
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Get effective frequency with detune
    fn effective_frequency(&self) -> f32 {
        self.frequency * 2.0_f32.powf(self.detune / 1200.0)
    }

    /// Process a single sample
    pub fn process_sample(&mut self) -> f32 {
        let freq = self.effective_frequency();
        let dt = freq / self.sample_rate;

        let sample = match &self.waveform {
            Waveform::Sine => (self.phase * std::f32::consts::TAU).sin(),
            Waveform::Saw => saw_polyblep(self.phase, dt),
            Waveform::Square { pulse_width } => square_polyblep(self.phase, dt, *pulse_width),
            Waveform::Triangle => {
                // Naive triangle (TODO: integrate square for bandlimited)
                if self.phase < 0.5 {
                    4.0 * self.phase - 1.0
                } else {
                    3.0 - 4.0 * self.phase
                }
            }
            Waveform::Noise => rand::random::<f32>() * 2.0 - 1.0,
        };

        // Advance phase
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sample
    }

    /// Process buffer
    pub fn process(&mut self, output: &mut [f32]) {
        for sample in output.iter_mut() {
            *sample = self.process_sample();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oscillator_new() {
        let osc = Oscillator::new(Waveform::Saw, 44100);
        assert_eq!(osc.frequency, 440.0);
    }

    #[test]
    fn test_oscillator_set_frequency() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        osc.set_frequency(880.0);
        assert_eq!(osc.frequency, 880.0);
    }

    #[test]
    fn test_sine_range() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        osc.set_frequency(440.0);
        for _ in 0..1000 {
            let sample = osc.process_sample();
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_saw_range() {
        let mut osc = Oscillator::new(Waveform::Saw, 44100);
        osc.set_frequency(440.0);
        for _ in 0..1000 {
            let sample = osc.process_sample();
            assert!(sample >= -1.5 && sample <= 1.5);
        }
    }

    #[test]
    fn test_square_range() {
        let mut osc = Oscillator::new(Waveform::Square { pulse_width: 0.5 }, 44100);
        osc.set_frequency(440.0);
        for _ in 0..1000 {
            let sample = osc.process_sample();
            assert!(sample >= -1.5 && sample <= 1.5);
        }
    }

    #[test]
    fn test_noise_varies() {
        let mut osc = Oscillator::new(Waveform::Noise, 44100);
        let s1 = osc.process_sample();
        let s2 = osc.process_sample();
        // Noise should vary (statistically almost always)
        assert!(s1 != s2 || s1 == 0.0); // Allow rare equality
    }

    #[test]
    fn test_detune() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        osc.set_frequency(440.0);
        osc.set_detune(1200.0); // One octave up
        assert!((osc.effective_frequency() - 880.0).abs() < 1.0);
    }

    #[test]
    fn test_process_buffer() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        let mut buffer = vec![0.0; 64];
        osc.process(&mut buffer);
        // Should have non-zero values
        assert!(buffer.iter().any(|&s| s != 0.0));
    }
}
