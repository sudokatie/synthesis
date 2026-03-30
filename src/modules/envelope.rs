//! ADSR Envelope generator

/// Envelope stages
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// ADSR Envelope
#[derive(Debug, Clone)]
pub struct Envelope {
    attack: f32,  // seconds
    decay: f32,   // seconds
    sustain: f32, // 0.0-1.0
    release: f32, // seconds
    /// Curve shape: -1.0 = exponential, 0.0 = linear, 1.0 = logarithmic
    curve: f32,
    sample_rate: f32,
    // State
    stage: EnvelopeStage,
    value: f32,
    target: f32,
    rate: f32,
    /// Linear position in current stage (0.0 to 1.0)
    stage_pos: f32,
    /// Starting value when stage began
    stage_start: f32,
}

impl Envelope {
    pub fn new(attack: f32, decay: f32, sustain: f32, release: f32, sample_rate: u32) -> Self {
        Self {
            attack: attack.max(0.001),
            decay: decay.max(0.001),
            sustain: sustain.clamp(0.0, 1.0),
            release: release.max(0.001),
            curve: 0.0,
            sample_rate: sample_rate as f32,
            stage: EnvelopeStage::Idle,
            value: 0.0,
            target: 0.0,
            rate: 0.0,
            stage_pos: 0.0,
            stage_start: 0.0,
        }
    }
    
    /// Set curve shape (-1.0 = exponential, 0.0 = linear, 1.0 = logarithmic)
    pub fn set_curve(&mut self, curve: f32) {
        self.curve = curve.clamp(-1.0, 1.0);
    }
    
    /// Get curve shape
    pub fn curve(&self) -> f32 {
        self.curve
    }
    
    /// Apply curve shaping to a linear position
    fn apply_curve(&self, linear_pos: f32) -> f32 {
        if self.curve.abs() < 0.001 {
            // Linear
            linear_pos
        } else if self.curve < 0.0 {
            // Exponential (fast start, slow end)
            let exp = 1.0 + self.curve.abs() * 4.0; // 1.0 to 5.0
            linear_pos.powf(exp)
        } else {
            // Logarithmic (slow start, fast end)
            let exp = 1.0 / (1.0 + self.curve * 4.0); // 1.0 to 0.2
            linear_pos.powf(exp)
        }
    }

    /// Trigger the envelope
    pub fn trigger(&mut self) {
        self.stage = EnvelopeStage::Attack;
        self.target = 1.0;
        self.rate = 1.0 / (self.attack * self.sample_rate);
        self.stage_pos = 0.0;
        self.stage_start = self.value;
    }

    /// Release the envelope
    pub fn release(&mut self) {
        if self.stage != EnvelopeStage::Idle {
            self.stage = EnvelopeStage::Release;
            self.target = 0.0;
            self.rate = 1.0 / (self.release * self.sample_rate);
            self.stage_pos = 0.0;
            self.stage_start = self.value;
        }
    }

    /// Check if envelope is active
    pub fn is_active(&self) -> bool {
        self.stage != EnvelopeStage::Idle
    }

    /// Get current value
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Get current stage
    pub fn stage(&self) -> EnvelopeStage {
        self.stage
    }

    /// Process single sample
    pub fn process_sample(&mut self) -> f32 {
        match self.stage {
            EnvelopeStage::Idle => {
                self.value = 0.0;
            }
            EnvelopeStage::Attack => {
                self.stage_pos += self.rate;
                if self.stage_pos >= 1.0 {
                    self.stage_pos = 1.0;
                    self.value = 1.0;
                    self.stage = EnvelopeStage::Decay;
                    self.target = self.sustain;
                    self.rate = 1.0 / (self.decay * self.sample_rate);
                    self.stage_pos = 0.0;
                    self.stage_start = 1.0;
                } else {
                    // Apply curve to attack
                    let curved = self.apply_curve(self.stage_pos);
                    self.value = self.stage_start + (1.0 - self.stage_start) * curved;
                }
            }
            EnvelopeStage::Decay => {
                self.stage_pos += self.rate;
                if self.stage_pos >= 1.0 {
                    self.stage_pos = 1.0;
                    self.value = self.sustain;
                    self.stage = EnvelopeStage::Sustain;
                } else {
                    // Apply curve to decay (inverted)
                    let curved = self.apply_curve(self.stage_pos);
                    self.value = self.stage_start - (self.stage_start - self.sustain) * curved;
                }
            }
            EnvelopeStage::Sustain => {
                self.value = self.sustain;
            }
            EnvelopeStage::Release => {
                self.stage_pos += self.rate;
                if self.stage_pos >= 1.0 {
                    self.stage_pos = 1.0;
                    self.value = 0.0;
                    self.stage = EnvelopeStage::Idle;
                } else {
                    // Apply curve to release (inverted)
                    let curved = self.apply_curve(self.stage_pos);
                    self.value = self.stage_start * (1.0 - curved);
                }
            }
        }

        self.value
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
    fn test_envelope_new() {
        let env = Envelope::new(0.01, 0.1, 0.7, 0.5, 44100);
        assert_eq!(env.stage(), EnvelopeStage::Idle);
        assert!(!env.is_active());
    }

    #[test]
    fn test_envelope_trigger() {
        let mut env = Envelope::new(0.01, 0.1, 0.7, 0.5, 44100);
        env.trigger();
        assert_eq!(env.stage(), EnvelopeStage::Attack);
        assert!(env.is_active());
    }

    #[test]
    fn test_envelope_attack_rises() {
        let mut env = Envelope::new(0.01, 0.1, 0.7, 0.5, 44100);
        env.trigger();
        let v1 = env.process_sample();
        let v2 = env.process_sample();
        assert!(v2 > v1);
    }

    #[test]
    fn test_envelope_reaches_peak() {
        let mut env = Envelope::new(0.001, 0.1, 0.7, 0.5, 44100);
        env.trigger();
        let mut max_val = 0.0_f32;
        for _ in 0..500 {
            max_val = max_val.max(env.process_sample());
        }
        assert!(max_val > 0.99);
    }

    #[test]
    fn test_envelope_sustain() {
        let mut env = Envelope::new(0.001, 0.01, 0.7, 0.5, 44100);
        env.trigger();
        // Process through attack and decay
        for _ in 0..2000 {
            env.process_sample();
        }
        assert_eq!(env.stage(), EnvelopeStage::Sustain);
        assert!((env.value() - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_envelope_release() {
        let mut env = Envelope::new(0.001, 0.01, 0.7, 0.1, 44100);
        env.trigger();
        for _ in 0..2000 {
            env.process_sample();
        }
        env.release();
        assert_eq!(env.stage(), EnvelopeStage::Release);
        for _ in 0..10000 {
            env.process_sample();
        }
        assert_eq!(env.stage(), EnvelopeStage::Idle);
    }

    #[test]
    fn test_envelope_retrigger() {
        let mut env = Envelope::new(0.01, 0.1, 0.5, 0.5, 44100);
        env.trigger();
        for _ in 0..500 {
            env.process_sample();
        }
        // Retrigger during attack
        env.trigger();
        assert_eq!(env.stage(), EnvelopeStage::Attack);
    }

    #[test]
    fn test_envelope_value_range() {
        let mut env = Envelope::new(0.01, 0.1, 0.7, 0.5, 44100);
        env.trigger();
        for _ in 0..10000 {
            let v = env.process_sample();
            assert!(v >= 0.0 && v <= 1.0, "Value out of range: {}", v);
        }
    }

    #[test]
    fn test_envelope_process_buffer() {
        let mut env = Envelope::new(0.01, 0.1, 0.7, 0.5, 44100);
        let mut buffer = vec![0.0; 256];
        env.trigger();
        env.process(&mut buffer);
        assert!(buffer.iter().any(|&v| v > 0.0));
    }

    #[test]
    fn test_envelope_idle_stays_zero() {
        let mut env = Envelope::new(0.01, 0.1, 0.7, 0.5, 44100);
        // Don't trigger
        for _ in 0..100 {
            let v = env.process_sample();
            assert_eq!(v, 0.0);
        }
    }
    
    #[test]
    fn test_envelope_curve_default() {
        let env = Envelope::new(0.01, 0.1, 0.7, 0.5, 44100);
        assert_eq!(env.curve(), 0.0);
    }
    
    #[test]
    fn test_envelope_set_curve() {
        let mut env = Envelope::new(0.01, 0.1, 0.7, 0.5, 44100);
        env.set_curve(-0.5);
        assert_eq!(env.curve(), -0.5);
        
        env.set_curve(2.0); // Clamped
        assert_eq!(env.curve(), 1.0);
        
        env.set_curve(-2.0); // Clamped
        assert_eq!(env.curve(), -1.0);
    }
    
    #[test]
    fn test_envelope_curve_exponential() {
        let mut env = Envelope::new(0.1, 0.1, 0.5, 0.5, 44100);
        env.set_curve(-1.0); // Exponential
        env.trigger();
        
        // Sample midpoint
        let samples_to_mid = (44100.0 * 0.1 / 2.0) as usize;
        for _ in 0..samples_to_mid {
            env.process_sample();
        }
        
        // With exponential curve, value at midpoint should be < 0.5
        // (fast start, slow end means we haven't reached 0.5 yet)
        let mid_val = env.value();
        assert!(mid_val < 0.5, "Expected < 0.5, got {}", mid_val);
    }
    
    #[test]
    fn test_envelope_curve_logarithmic() {
        let mut env = Envelope::new(0.1, 0.1, 0.5, 0.5, 44100);
        env.set_curve(1.0); // Logarithmic
        env.trigger();
        
        // Sample midpoint
        let samples_to_mid = (44100.0 * 0.1 / 2.0) as usize;
        for _ in 0..samples_to_mid {
            env.process_sample();
        }
        
        // With logarithmic curve, value at midpoint should be > 0.5
        // (slow start, fast end means we've passed 0.5 already)
        let mid_val = env.value();
        assert!(mid_val > 0.5, "Expected > 0.5, got {}", mid_val);
    }
    
    #[test]
    fn test_envelope_curve_still_reaches_target() {
        for curve in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let mut env = Envelope::new(0.01, 0.01, 0.5, 0.01, 44100);
            env.set_curve(curve);
            env.trigger();
            
            // Should reach peak
            let mut max_val = 0.0_f32;
            for _ in 0..2000 {
                max_val = max_val.max(env.process_sample());
            }
            assert!(max_val > 0.99, "Curve {} didn't reach peak: {}", curve, max_val);
        }
    }
}
