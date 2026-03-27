//! Distortion and waveshaping effects

use crate::dsp::fast_tanh;

/// Distortion types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistortionType {
    Soft,      // Soft clipping (tanh)
    Hard,      // Hard clipping
    Foldback,  // Wave folding
    BitCrush,  // Bit reduction
}

/// Waveshaper distortion
#[derive(Debug, Clone)]
pub struct Distortion {
    drive: f32,       // 1.0 to 100.0
    tone: f32,        // 0.0 to 1.0 (lowpass after distortion)
    mix: f32,
    dist_type: DistortionType,
    // Tone filter state
    tone_state: f32,
}

impl Distortion {
    pub fn new() -> Self {
        Self {
            drive: 1.0,
            tone: 0.7,
            mix: 1.0,
            dist_type: DistortionType::Soft,
            tone_state: 0.0,
        }
    }

    pub fn set_drive(&mut self, drive: f32) {
        self.drive = drive.clamp(1.0, 100.0);
    }

    pub fn set_tone(&mut self, tone: f32) {
        self.tone = tone.clamp(0.0, 1.0);
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    pub fn set_type(&mut self, dist_type: DistortionType) {
        self.dist_type = dist_type;
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        let driven = input * self.drive;

        let distorted = match self.dist_type {
            DistortionType::Soft => fast_tanh(driven),
            DistortionType::Hard => driven.clamp(-1.0, 1.0),
            DistortionType::Foldback => {
                let mut x = driven;
                while x > 1.0 || x < -1.0 {
                    if x > 1.0 {
                        x = 2.0 - x;
                    }
                    if x < -1.0 {
                        x = -2.0 - x;
                    }
                }
                x
            }
            DistortionType::BitCrush => {
                let bits = 16.0 - (self.drive - 1.0) * 0.15; // 16 bits to ~2 bits
                let bits = bits.clamp(2.0, 16.0);
                let levels = 2.0_f32.powf(bits);
                (driven * levels).round() / levels
            }
        };

        // Simple tone control (lowpass)
        let cutoff = 0.1 + self.tone * 0.9;
        self.tone_state += cutoff * (distorted - self.tone_state);
        let shaped = self.tone_state;

        // Mix dry/wet
        input * (1.0 - self.mix) + shaped * self.mix
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    pub fn reset(&mut self) {
        self.tone_state = 0.0;
    }
}

impl Default for Distortion {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distortion_new() {
        let dist = Distortion::new();
        assert_eq!(dist.drive, 1.0);
    }

    #[test]
    fn test_distortion_soft_clip() {
        let mut dist = Distortion::new();
        dist.set_drive(10.0);
        dist.set_type(DistortionType::Soft);
        dist.set_mix(1.0);
        dist.set_tone(1.0);

        let out = dist.process_sample(0.5);
        assert!(out.abs() <= 1.0);
    }

    #[test]
    fn test_distortion_hard_clip() {
        let mut dist = Distortion::new();
        dist.set_drive(10.0);
        dist.set_type(DistortionType::Hard);
        dist.set_mix(1.0);
        dist.set_tone(1.0);

        // With high drive, should clip at 1.0
        for _ in 0..10 {
            let out = dist.process_sample(0.5);
            assert!(out.abs() <= 1.1); // Small tolerance for filter
        }
    }

    #[test]
    fn test_distortion_foldback() {
        let mut dist = Distortion::new();
        dist.set_drive(5.0);
        dist.set_type(DistortionType::Foldback);
        dist.set_mix(1.0);
        dist.set_tone(1.0);

        // Foldback keeps signal in range
        for _ in 0..10 {
            let out = dist.process_sample(0.8);
            assert!(out.abs() <= 1.1);
        }
    }

    #[test]
    fn test_distortion_bitcrush() {
        let mut dist = Distortion::new();
        dist.set_drive(50.0);
        dist.set_type(DistortionType::BitCrush);
        dist.set_mix(1.0);
        dist.set_tone(1.0);

        let out = dist.process_sample(0.3);
        assert!(out.is_finite());
    }

    #[test]
    fn test_distortion_mix() {
        let mut dist = Distortion::new();
        dist.set_drive(10.0);
        dist.set_mix(0.0);

        let out = dist.process_sample(0.5);
        assert!((out - 0.5).abs() < 0.01); // Dry signal
    }

    #[test]
    fn test_distortion_tone() {
        let mut dist = Distortion::new();
        dist.set_drive(5.0);
        dist.set_tone(0.0); // Dark
        dist.reset();

        // Low tone = darker sound (filtered)
        let mut dark_sum = 0.0;
        for _ in 0..1000 {
            dark_sum += dist.process_sample(0.5).abs();
        }

        dist.reset();
        dist.set_tone(1.0); // Bright

        let mut bright_sum = 0.0;
        for _ in 0..1000 {
            bright_sum += dist.process_sample(0.5).abs();
        }

        // Bright should respond faster (less filtered)
        assert!(bright_sum > dark_sum * 0.8);
    }
}
