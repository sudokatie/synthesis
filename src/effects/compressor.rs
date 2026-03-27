//! Dynamics processing - compressor and limiter

use crate::dsp::{db_to_linear, linear_to_db};

/// Compressor with attack/release
#[derive(Debug, Clone)]
pub struct Compressor {
    threshold: f32,    // dB
    ratio: f32,        // e.g., 4.0 = 4:1
    attack: f32,       // seconds
    release: f32,      // seconds
    makeup_gain: f32,  // dB
    knee: f32,         // dB (soft knee width)
    sample_rate: f32,
    // State
    envelope: f32,     // Current envelope level
}

impl Compressor {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            threshold: -20.0,
            ratio: 4.0,
            attack: 0.01,
            release: 0.1,
            makeup_gain: 0.0,
            knee: 6.0,
            sample_rate: sample_rate as f32,
            envelope: 0.0,
        }
    }

    pub fn set_threshold(&mut self, db: f32) {
        self.threshold = db.clamp(-60.0, 0.0);
    }

    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio.clamp(1.0, 20.0);
    }

    pub fn set_attack(&mut self, seconds: f32) {
        self.attack = seconds.clamp(0.001, 1.0);
    }

    pub fn set_release(&mut self, seconds: f32) {
        self.release = seconds.clamp(0.01, 5.0);
    }

    pub fn set_makeup_gain(&mut self, db: f32) {
        self.makeup_gain = db.clamp(0.0, 40.0);
    }

    pub fn set_knee(&mut self, db: f32) {
        self.knee = db.clamp(0.0, 24.0);
    }

    fn compute_gain(&self, input_db: f32) -> f32 {
        let over_threshold = input_db - self.threshold;

        if self.knee > 0.0 {
            // Soft knee
            let half_knee = self.knee / 2.0;
            if over_threshold < -half_knee {
                // Below knee - no compression
                input_db
            } else if over_threshold > half_knee {
                // Above knee - full compression
                self.threshold + over_threshold / self.ratio
            } else {
                // In knee - interpolate
                let t = (over_threshold + half_knee) / self.knee;
                let comp_amount = over_threshold / self.ratio;
                input_db - t * t * (over_threshold - comp_amount)
            }
        } else {
            // Hard knee
            if over_threshold <= 0.0 {
                input_db
            } else {
                self.threshold + over_threshold / self.ratio
            }
        }
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        let input_abs = input.abs();
        let input_db = if input_abs > 1e-10 {
            linear_to_db(input_abs)
        } else {
            -120.0
        };

        // Envelope follower
        let target = input_db;
        let coeff = if target > self.envelope {
            1.0 - (-1.0 / (self.attack * self.sample_rate)).exp()
        } else {
            1.0 - (-1.0 / (self.release * self.sample_rate)).exp()
        };
        self.envelope += coeff * (target - self.envelope);

        // Compute gain reduction
        let output_db = self.compute_gain(self.envelope);
        let gain_db = output_db - self.envelope + self.makeup_gain;
        let gain = db_to_linear(gain_db);

        input * gain
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    /// Get current gain reduction in dB
    pub fn gain_reduction(&self) -> f32 {
        let output_db = self.compute_gain(self.envelope);
        self.envelope - output_db
    }

    pub fn reset(&mut self) {
        self.envelope = 0.0;
    }
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new(44100)
    }
}

/// Simple brick-wall limiter
#[derive(Debug, Clone)]
pub struct Limiter {
    ceiling: f32,      // Linear ceiling
    release: f32,      // seconds
    sample_rate: f32,
    gain: f32,         // Current gain
}

impl Limiter {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            ceiling: 0.95,
            release: 0.1,
            sample_rate: sample_rate as f32,
            gain: 1.0,
        }
    }

    pub fn set_ceiling(&mut self, linear: f32) {
        self.ceiling = linear.clamp(0.1, 1.0);
    }

    pub fn set_ceiling_db(&mut self, db: f32) {
        self.ceiling = db_to_linear(db).clamp(0.1, 1.0);
    }

    pub fn set_release(&mut self, seconds: f32) {
        self.release = seconds.clamp(0.01, 1.0);
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        let abs_in = input.abs();

        // Calculate required gain
        let target_gain = if abs_in > self.ceiling {
            self.ceiling / abs_in
        } else {
            1.0
        };

        // Fast attack (instant), slow release
        if target_gain < self.gain {
            self.gain = target_gain;
        } else {
            let release_coeff = 1.0 - (-1.0 / (self.release * self.sample_rate)).exp();
            self.gain += release_coeff * (target_gain - self.gain);
        }

        input * self.gain
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    pub fn reset(&mut self) {
        self.gain = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressor_new() {
        let comp = Compressor::new(44100);
        assert_eq!(comp.threshold, -20.0);
        assert_eq!(comp.ratio, 4.0);
    }

    #[test]
    fn test_compressor_below_threshold() {
        let mut comp = Compressor::new(44100);
        comp.set_threshold(-10.0);
        comp.set_makeup_gain(0.0);

        // -40dB input is below threshold
        let input = 0.01; // ~-40dB
        let out = comp.process_sample(input);
        // Should pass through relatively unchanged
        assert!((out - input).abs() < 0.01);
    }

    #[test]
    fn test_compressor_above_threshold() {
        let mut comp = Compressor::new(44100);
        comp.set_threshold(-20.0);
        comp.set_ratio(4.0);
        comp.set_attack(0.0001);
        comp.set_makeup_gain(0.0);

        // Process loud signal
        let mut out = 0.0;
        for _ in 0..1000 {
            out = comp.process_sample(0.9);
        }
        // Should be compressed
        assert!(out < 0.9);
    }

    #[test]
    fn test_compressor_makeup_gain() {
        let mut comp = Compressor::new(44100);
        comp.set_makeup_gain(12.0);
        comp.set_attack(0.0001); // Fast attack

        // Process enough for envelope to settle
        let mut out = 0.0;
        for _ in 0..1000 {
            out = comp.process_sample(0.1);
        }
        // With 12dB makeup, 0.1 should be boosted significantly
        assert!(out > 0.1, "Output {} should be > 0.1", out);
    }

    #[test]
    fn test_compressor_soft_knee() {
        let mut comp = Compressor::new(44100);
        comp.set_knee(12.0);
        let out = comp.process_sample(0.5);
        assert!(out.is_finite());
    }

    #[test]
    fn test_compressor_gain_reduction() {
        let mut comp = Compressor::new(44100);
        comp.set_threshold(-20.0);
        comp.set_attack(0.0001);

        // Process loud signal
        for _ in 0..1000 {
            comp.process_sample(0.9);
        }

        let gr = comp.gain_reduction();
        assert!(gr > 0.0);
    }

    #[test]
    fn test_limiter_new() {
        let lim = Limiter::new(44100);
        assert_eq!(lim.ceiling, 0.95);
    }

    #[test]
    fn test_limiter_ceiling() {
        let mut lim = Limiter::new(44100);
        lim.set_ceiling(0.5);

        // Loud input
        let out = lim.process_sample(1.0);
        assert!(out <= 0.51); // Should be limited
    }

    #[test]
    fn test_limiter_no_limit() {
        let mut lim = Limiter::new(44100);
        lim.set_ceiling(0.9);

        // Quiet input
        let out = lim.process_sample(0.3);
        assert!((out - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_limiter_reset() {
        let mut lim = Limiter::new(44100);
        lim.process_sample(2.0); // Triggers limiting
        lim.reset();
        assert_eq!(lim.gain, 1.0);
    }
}
