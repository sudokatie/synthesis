//! Low Frequency Oscillator

use crate::modules::Waveform;

/// LFO polarity
#[derive(Debug, Clone, Copy)]
pub enum Polarity {
    Unipolar, // 0.0 to 1.0
    Bipolar,  // -1.0 to 1.0
}

/// Low Frequency Oscillator
#[derive(Debug, Clone)]
pub struct Lfo {
    waveform: Waveform,
    frequency: f32,
    phase: f32,
    polarity: Polarity,
    sample_rate: f32,
}

impl Lfo {
    pub fn new(waveform: Waveform, frequency: f32, sample_rate: u32) -> Self {
        Self {
            waveform,
            frequency: frequency.clamp(0.01, 100.0),
            phase: 0.0,
            polarity: Polarity::Bipolar,
            sample_rate: sample_rate as f32,
        }
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.frequency = freq.clamp(0.01, 100.0);
    }

    pub fn set_polarity(&mut self, polarity: Polarity) {
        self.polarity = polarity;
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    pub fn value(&self) -> f32 {
        self.generate_sample(self.phase)
    }

    fn generate_sample(&self, phase: f32) -> f32 {
        let raw = match &self.waveform {
            Waveform::Sine => (phase * std::f32::consts::TAU).sin(),
            Waveform::Saw => 2.0 * phase - 1.0,
            Waveform::Square { pulse_width } => {
                if phase < *pulse_width {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Triangle => {
                if phase < 0.5 {
                    4.0 * phase - 1.0
                } else {
                    3.0 - 4.0 * phase
                }
            }
            Waveform::Noise => rand::random::<f32>() * 2.0 - 1.0,
        };

        match self.polarity {
            Polarity::Bipolar => raw,
            Polarity::Unipolar => (raw + 1.0) * 0.5,
        }
    }

    pub fn process_sample(&mut self) -> f32 {
        let sample = self.generate_sample(self.phase);

        self.phase += self.frequency / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sample
    }

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
    fn test_lfo_new() {
        let lfo = Lfo::new(Waveform::Sine, 1.0, 44100);
        assert_eq!(lfo.frequency, 1.0);
    }

    #[test]
    fn test_lfo_bipolar_range() {
        let mut lfo = Lfo::new(Waveform::Sine, 10.0, 44100);
        lfo.set_polarity(Polarity::Bipolar);
        for _ in 0..4410 {
            let s = lfo.process_sample();
            assert!(s >= -1.0 && s <= 1.0);
        }
    }

    #[test]
    fn test_lfo_unipolar_range() {
        let mut lfo = Lfo::new(Waveform::Sine, 10.0, 44100);
        lfo.set_polarity(Polarity::Unipolar);
        for _ in 0..4410 {
            let s = lfo.process_sample();
            assert!(s >= 0.0 && s <= 1.0);
        }
    }

    #[test]
    fn test_lfo_reset() {
        let mut lfo = Lfo::new(Waveform::Sine, 1.0, 44100);
        lfo.process_sample();
        lfo.reset();
        assert_eq!(lfo.phase, 0.0);
    }
}
