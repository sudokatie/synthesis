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
    sample_rate: f32,
    // State
    stage: EnvelopeStage,
    value: f32,
    target: f32,
    rate: f32,
}

impl Envelope {
    pub fn new(attack: f32, decay: f32, sustain: f32, release: f32, sample_rate: u32) -> Self {
        Self {
            attack: attack.max(0.001),
            decay: decay.max(0.001),
            sustain: sustain.clamp(0.0, 1.0),
            release: release.max(0.001),
            sample_rate: sample_rate as f32,
            stage: EnvelopeStage::Idle,
            value: 0.0,
            target: 0.0,
            rate: 0.0,
        }
    }

    /// Trigger the envelope
    pub fn trigger(&mut self) {
        self.stage = EnvelopeStage::Attack;
        self.target = 1.0;
        self.rate = 1.0 / (self.attack * self.sample_rate);
    }

    /// Release the envelope
    pub fn release(&mut self) {
        if self.stage != EnvelopeStage::Idle {
            self.stage = EnvelopeStage::Release;
            self.target = 0.0;
            self.rate = self.value / (self.release * self.sample_rate);
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
                self.value += self.rate;
                if self.value >= 1.0 {
                    self.value = 1.0;
                    self.stage = EnvelopeStage::Decay;
                    self.target = self.sustain;
                    self.rate = (1.0 - self.sustain) / (self.decay * self.sample_rate);
                }
            }
            EnvelopeStage::Decay => {
                self.value -= self.rate;
                if self.value <= self.sustain {
                    self.value = self.sustain;
                    self.stage = EnvelopeStage::Sustain;
                }
            }
            EnvelopeStage::Sustain => {
                self.value = self.sustain;
            }
            EnvelopeStage::Release => {
                self.value -= self.rate;
                if self.value <= 0.0 {
                    self.value = 0.0;
                    self.stage = EnvelopeStage::Idle;
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
}
