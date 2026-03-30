//! Unified Effect enum as specified

use super::{Chorus, Compressor, Delay, Distortion, SchroederReverb, StereoDelay};

/// Unified effect type matching spec
#[derive(Debug, Clone)]
pub enum Effect {
    /// Delay effect
    Delay {
        time: f32,
        feedback: f32,
        mix: f32,
        stereo: bool,
    },
    /// Reverb effect
    Reverb {
        size: f32,
        damping: f32,
        mix: f32,
        pre_delay: f32,
    },
    /// Distortion effect
    Distortion {
        drive: f32,
        tone: f32,
        mix: f32,
    },
    /// Chorus effect
    Chorus {
        rate: f32,
        depth: f32,
        mix: f32,
    },
    /// Compressor effect
    Compressor {
        threshold: f32,
        ratio: f32,
        attack: f32,
        release: f32,
    },
}

impl Default for Effect {
    fn default() -> Self {
        Effect::Delay {
            time: 0.25,
            feedback: 0.5,
            mix: 0.3,
            stereo: false,
        }
    }
}

/// Effect processor that can hold any effect type
pub struct EffectProcessor {
    effect_type: Effect,
    // Actual processors
    delay: Delay,
    stereo_delay: StereoDelay,
    reverb: SchroederReverb,
    distortion: Distortion,
    chorus: Chorus,
    compressor: Compressor,
    sample_rate: u32,
}

impl EffectProcessor {
    pub fn new(effect: Effect, sample_rate: u32) -> Self {
        let mut processor = Self {
            effect_type: effect.clone(),
            delay: Delay::new(2.0, sample_rate),
            stereo_delay: StereoDelay::new(2.0, sample_rate),
            reverb: SchroederReverb::new(sample_rate),
            distortion: Distortion::new(),
            chorus: Chorus::new(sample_rate),
            compressor: Compressor::new(sample_rate),
            sample_rate,
        };
        processor.apply_settings(&effect);
        processor
    }

    /// Apply effect settings
    pub fn apply_settings(&mut self, effect: &Effect) {
        self.effect_type = effect.clone();
        match effect {
            Effect::Delay { time, feedback, mix, stereo } => {
                if *stereo {
                    self.stereo_delay.set_left_time(*time);
                    self.stereo_delay.set_right_time(*time * 1.5);
                    self.stereo_delay.set_feedback(*feedback);
                    self.stereo_delay.set_mix(*mix);
                } else {
                    self.delay.set_time(*time);
                    self.delay.set_feedback(*feedback);
                    self.delay.set_mix(*mix);
                }
            }
            Effect::Reverb { size, damping, mix, pre_delay } => {
                self.reverb.set_room_size(*size);
                self.reverb.set_damping(*damping);
                self.reverb.set_mix(*mix);
                self.reverb.set_pre_delay(*pre_delay);
            }
            Effect::Distortion { drive, tone, mix } => {
                self.distortion.set_drive(*drive);
                self.distortion.set_tone(*tone);
                self.distortion.set_mix(*mix);
            }
            Effect::Chorus { rate, depth, mix } => {
                self.chorus.set_rate(*rate);
                self.chorus.set_depth(*depth);
                self.chorus.set_mix(*mix);
            }
            Effect::Compressor { threshold, ratio, attack, release } => {
                self.compressor.set_threshold(*threshold);
                self.compressor.set_ratio(*ratio);
                self.compressor.set_attack(*attack);
                self.compressor.set_release(*release);
            }
        }
    }

    /// Process a single sample
    pub fn process_sample(&mut self, input: f32) -> f32 {
        match &self.effect_type {
            Effect::Delay { stereo, .. } => {
                if *stereo {
                    let (l, r) = self.stereo_delay.process_sample(input, input);
                    (l + r) * 0.5
                } else {
                    self.delay.process_sample(input)
                }
            }
            Effect::Reverb { .. } => self.reverb.process_sample(input),
            Effect::Distortion { .. } => self.distortion.process_sample(input),
            Effect::Chorus { .. } => self.chorus.process_sample(input),
            Effect::Compressor { .. } => self.compressor.process_sample(input),
        }
    }

    /// Process stereo samples
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        match &self.effect_type {
            Effect::Delay { stereo, .. } => {
                if *stereo {
                    self.stereo_delay.process_sample(left, right)
                } else {
                    let l = self.delay.process_sample(left);
                    let r = self.delay.process_sample(right);
                    (l, r)
                }
            }
            Effect::Reverb { .. } => self.reverb.process_stereo(left, right),
            Effect::Distortion { .. } => {
                (self.distortion.process_sample(left), self.distortion.process_sample(right))
            }
            Effect::Chorus { .. } => {
                (self.chorus.process_sample(left), self.chorus.process_sample(right))
            }
            Effect::Compressor { .. } => {
                (self.compressor.process_sample(left), self.compressor.process_sample(right))
            }
        }
    }

    /// Get current effect type
    pub fn effect_type(&self) -> &Effect {
        &self.effect_type
    }

    /// Reset effect state
    pub fn reset(&mut self) {
        self.delay.reset();
        self.stereo_delay.reset();
        self.reverb.reset();
        self.distortion.reset();
        self.chorus.reset();
        self.compressor.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_default() {
        let effect = Effect::default();
        match effect {
            Effect::Delay { time, .. } => assert_eq!(time, 0.25),
            _ => panic!("Wrong default"),
        }
    }

    #[test]
    fn test_effect_processor_delay() {
        let effect = Effect::Delay {
            time: 0.1,
            feedback: 0.5,
            mix: 0.5,
            stereo: false,
        };
        let mut proc = EffectProcessor::new(effect, 44100);
        let out = proc.process_sample(1.0);
        assert!(out.is_finite());
    }

    #[test]
    fn test_effect_processor_reverb() {
        let effect = Effect::Reverb {
            size: 0.8,
            damping: 0.5,
            mix: 0.3,
            pre_delay: 0.05,
        };
        let mut proc = EffectProcessor::new(effect, 44100);
        let out = proc.process_sample(1.0);
        assert!(out.is_finite());
    }

    #[test]
    fn test_effect_processor_distortion() {
        let effect = Effect::Distortion {
            drive: 5.0,
            tone: 0.5,
            mix: 0.5,
        };
        let mut proc = EffectProcessor::new(effect, 44100);
        let out = proc.process_sample(0.5);
        assert!(out.is_finite());
    }

    #[test]
    fn test_effect_processor_chorus() {
        let effect = Effect::Chorus {
            rate: 1.0,
            depth: 0.5,
            mix: 0.3,
        };
        let mut proc = EffectProcessor::new(effect, 44100);
        let out = proc.process_sample(0.5);
        assert!(out.is_finite());
    }

    #[test]
    fn test_effect_processor_compressor() {
        let effect = Effect::Compressor {
            threshold: -20.0,
            ratio: 4.0,
            attack: 0.01,
            release: 0.1,
        };
        let mut proc = EffectProcessor::new(effect, 44100);
        let out = proc.process_sample(0.5);
        assert!(out.is_finite());
    }

    #[test]
    fn test_effect_processor_stereo() {
        let effect = Effect::Delay {
            time: 0.25,
            feedback: 0.5,
            mix: 0.5,
            stereo: true,
        };
        let mut proc = EffectProcessor::new(effect, 44100);
        let (l, r) = proc.process_stereo(1.0, 0.5);
        assert!(l.is_finite());
        assert!(r.is_finite());
    }

    #[test]
    fn test_effect_processor_reset() {
        let effect = Effect::Delay {
            time: 0.1,
            feedback: 0.5,
            mix: 0.5,
            stereo: false,
        };
        let mut proc = EffectProcessor::new(effect, 44100);
        proc.process_sample(1.0);
        proc.reset();
        // Should not panic
    }

    #[test]
    fn test_effect_processor_apply_settings() {
        let mut proc = EffectProcessor::new(Effect::default(), 44100);
        proc.apply_settings(&Effect::Reverb {
            size: 0.9,
            damping: 0.3,
            mix: 0.5,
            pre_delay: 0.1,
        });
        match proc.effect_type() {
            Effect::Reverb { size, .. } => assert_eq!(*size, 0.9),
            _ => panic!("Wrong type"),
        }
    }
}
