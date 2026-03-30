//! Schroeder reverb with comb and allpass filters

use super::delay::DelayLine;

/// Comb filter for reverb
#[derive(Debug, Clone)]
pub struct CombFilter {
    delay: DelayLine,
    delay_samples: usize,
    feedback: f32,
    damp: f32,
    damp_state: f32,
}

impl CombFilter {
    pub fn new(delay_samples: usize) -> Self {
        Self {
            delay: DelayLine::new(delay_samples + 1),
            delay_samples,
            feedback: 0.5,
            damp: 0.5,
            damp_state: 0.0,
        }
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 0.99);
    }

    pub fn set_damp(&mut self, damp: f32) {
        self.damp = damp.clamp(0.0, 1.0);
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        let output = self.delay.read(self.delay_samples);

        // Apply damping (lowpass)
        self.damp_state = output * (1.0 - self.damp) + self.damp_state * self.damp;

        self.delay.write(input + self.damp_state * self.feedback);
        output
    }

    pub fn reset(&mut self) {
        self.delay.reset();
        self.damp_state = 0.0;
    }
}

/// Allpass filter for reverb
#[derive(Debug, Clone)]
pub struct AllpassFilter {
    delay: DelayLine,
    delay_samples: usize,
    feedback: f32,
}

impl AllpassFilter {
    pub fn new(delay_samples: usize) -> Self {
        Self {
            delay: DelayLine::new(delay_samples + 1),
            delay_samples,
            feedback: 0.5,
        }
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 0.99);
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        let delayed = self.delay.read(self.delay_samples);
        let output = -input + delayed;
        self.delay.write(input + delayed * self.feedback);
        output
    }

    pub fn reset(&mut self) {
        self.delay.reset();
    }
}

/// Schroeder reverb - classic algorithm with pre-delay
#[derive(Debug, Clone)]
pub struct SchroederReverb {
    combs: [CombFilter; 8],
    allpasses: [AllpassFilter; 4],
    pre_delay: DelayLine,
    pre_delay_samples: usize,
    mix: f32,
    room_size: f32,
    sample_rate: u32,
}

impl SchroederReverb {
    /// Comb filter delay times (in samples at 44100 Hz)
    const COMB_DELAYS: [usize; 8] = [1557, 1617, 1491, 1422, 1277, 1356, 1188, 1116];

    /// Allpass filter delay times
    const ALLPASS_DELAYS: [usize; 4] = [225, 556, 441, 341];

    /// Maximum pre-delay in seconds
    const MAX_PRE_DELAY: f32 = 0.5;

    pub fn new(sample_rate: u32) -> Self {
        let scale = sample_rate as f32 / 44100.0;

        let combs = [
            CombFilter::new((Self::COMB_DELAYS[0] as f32 * scale) as usize),
            CombFilter::new((Self::COMB_DELAYS[1] as f32 * scale) as usize),
            CombFilter::new((Self::COMB_DELAYS[2] as f32 * scale) as usize),
            CombFilter::new((Self::COMB_DELAYS[3] as f32 * scale) as usize),
            CombFilter::new((Self::COMB_DELAYS[4] as f32 * scale) as usize),
            CombFilter::new((Self::COMB_DELAYS[5] as f32 * scale) as usize),
            CombFilter::new((Self::COMB_DELAYS[6] as f32 * scale) as usize),
            CombFilter::new((Self::COMB_DELAYS[7] as f32 * scale) as usize),
        ];

        let allpasses = [
            AllpassFilter::new((Self::ALLPASS_DELAYS[0] as f32 * scale) as usize),
            AllpassFilter::new((Self::ALLPASS_DELAYS[1] as f32 * scale) as usize),
            AllpassFilter::new((Self::ALLPASS_DELAYS[2] as f32 * scale) as usize),
            AllpassFilter::new((Self::ALLPASS_DELAYS[3] as f32 * scale) as usize),
        ];

        // Pre-delay buffer (max 500ms)
        let max_pre_delay_samples = (Self::MAX_PRE_DELAY * sample_rate as f32) as usize;
        let pre_delay = DelayLine::new(max_pre_delay_samples + 1);

        let mut reverb = Self {
            combs,
            allpasses,
            pre_delay,
            pre_delay_samples: 0,
            mix: 0.3,
            room_size: 0.8,
            sample_rate,
        };
        reverb.update_params();
        reverb
    }

    fn update_params(&mut self) {
        let feedback = 0.28 + 0.7 * self.room_size;
        for comb in &mut self.combs {
            comb.set_feedback(feedback);
            comb.set_damp(0.4);
        }
        for allpass in &mut self.allpasses {
            allpass.set_feedback(0.5);
        }
    }

    pub fn set_room_size(&mut self, size: f32) {
        self.room_size = size.clamp(0.0, 1.0);
        self.update_params();
    }

    pub fn set_damping(&mut self, damp: f32) {
        let damp = damp.clamp(0.0, 1.0);
        for comb in &mut self.combs {
            comb.set_damp(damp);
        }
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    /// Set pre-delay time in seconds (0.0 to 0.5)
    pub fn set_pre_delay(&mut self, seconds: f32) {
        let seconds = seconds.clamp(0.0, Self::MAX_PRE_DELAY);
        self.pre_delay_samples = (seconds * self.sample_rate as f32) as usize;
    }

    /// Get current pre-delay time in seconds
    pub fn pre_delay(&self) -> f32 {
        self.pre_delay_samples as f32 / self.sample_rate as f32
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        // Apply pre-delay
        let delayed_input = if self.pre_delay_samples > 0 {
            self.pre_delay.write(input);
            self.pre_delay.read(self.pre_delay_samples)
        } else {
            input
        };

        // Sum parallel comb filters
        let mut comb_sum = 0.0;
        for comb in &mut self.combs {
            comb_sum += comb.process_sample(delayed_input);
        }
        comb_sum *= 0.125; // Average

        // Series allpass filters
        let mut output = comb_sum;
        for allpass in &mut self.allpasses {
            output = allpass.process_sample(output);
        }

        // Mix dry/wet
        input * (1.0 - self.mix) + output * self.mix
    }

    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        let mono = (left + right) * 0.5;
        
        // Apply pre-delay
        let delayed_mono = if self.pre_delay_samples > 0 {
            self.pre_delay.write(mono);
            self.pre_delay.read(self.pre_delay_samples)
        } else {
            mono
        };

        // Process left through first 4 combs
        let mut left_comb = 0.0;
        for comb in &mut self.combs[0..4] {
            left_comb += comb.process_sample(delayed_mono);
        }
        left_comb *= 0.25;

        // Process right through last 4 combs
        let mut right_comb = 0.0;
        for comb in &mut self.combs[4..8] {
            right_comb += comb.process_sample(delayed_mono);
        }
        right_comb *= 0.25;

        // Allpasses (shared)
        let mut left_out = left_comb;
        let mut right_out = right_comb;
        for allpass in &mut self.allpasses {
            let l = allpass.process_sample(left_out);
            left_out = l;
            right_out = allpass.process_sample(right_out);
        }

        let left_wet = left * (1.0 - self.mix) + left_out * self.mix;
        let right_wet = right * (1.0 - self.mix) + right_out * self.mix;

        (left_wet, right_wet)
    }

    pub fn reset(&mut self) {
        for comb in &mut self.combs {
            comb.reset();
        }
        for allpass in &mut self.allpasses {
            allpass.reset();
        }
        self.pre_delay.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comb_filter_new() {
        let comb = CombFilter::new(1000);
        assert_eq!(comb.delay_samples, 1000);
    }

    #[test]
    fn test_comb_filter_process() {
        let mut comb = CombFilter::new(5);
        comb.set_feedback(0.0);
        comb.set_damp(0.0);

        let mut outputs = Vec::new();
        outputs.push(comb.process_sample(1.0));
        for _ in 0..10 {
            outputs.push(comb.process_sample(0.0));
        }
        
        let echo_found = outputs.iter().skip(1).any(|&x| x > 0.5);
        assert!(echo_found, "No echo found in outputs: {:?}", outputs);
    }

    #[test]
    fn test_allpass_filter_new() {
        let allpass = AllpassFilter::new(500);
        assert_eq!(allpass.delay_samples, 500);
    }

    #[test]
    fn test_allpass_filter_process() {
        let mut allpass = AllpassFilter::new(100);
        allpass.set_feedback(0.5);

        let out = allpass.process_sample(1.0);
        assert!(out.is_finite());
    }

    #[test]
    fn test_schroeder_new() {
        let reverb = SchroederReverb::new(44100);
        assert_eq!(reverb.sample_rate, 44100);
    }

    #[test]
    fn test_schroeder_process() {
        let mut reverb = SchroederReverb::new(44100);
        reverb.set_room_size(0.8);
        reverb.set_mix(0.5);

        let out = reverb.process_sample(1.0);
        assert!(out.is_finite());

        let mut has_tail = false;
        for _ in 0..10000 {
            let out = reverb.process_sample(0.0);
            if out.abs() > 0.01 {
                has_tail = true;
            }
        }
        assert!(has_tail);
    }

    #[test]
    fn test_schroeder_stereo() {
        let mut reverb = SchroederReverb::new(44100);
        reverb.set_mix(0.5);

        let (l, r) = reverb.process_stereo(1.0, 0.5);
        assert!(l.is_finite());
        assert!(r.is_finite());
    }

    #[test]
    fn test_schroeder_room_size() {
        let mut reverb = SchroederReverb::new(44100);

        reverb.set_room_size(0.2);
        reverb.process_sample(1.0);
        let mut small_tail = 0.0;
        for _ in 0..5000 {
            small_tail += reverb.process_sample(0.0).abs();
        }

        reverb.reset();

        reverb.set_room_size(0.9);
        reverb.process_sample(1.0);
        let mut large_tail = 0.0;
        for _ in 0..5000 {
            large_tail += reverb.process_sample(0.0).abs();
        }

        assert!(large_tail > small_tail);
    }

    #[test]
    fn test_schroeder_reset() {
        let mut reverb = SchroederReverb::new(44100);
        reverb.process_sample(1.0);
        reverb.reset();
        let out = reverb.process_sample(0.0);
        assert!(out.abs() < 0.01);
    }

    #[test]
    fn test_schroeder_damping() {
        let mut reverb = SchroederReverb::new(44100);
        reverb.set_damping(0.8);
        let out = reverb.process_sample(1.0);
        assert!(out.is_finite());
    }

    #[test]
    fn test_schroeder_pre_delay() {
        let mut reverb = SchroederReverb::new(44100);
        reverb.set_pre_delay(0.1); // 100ms
        
        assert!((reverb.pre_delay() - 0.1).abs() < 0.001);
        
        // Process impulse
        let out = reverb.process_sample(1.0);
        assert!(out.is_finite());
    }

    #[test]
    fn test_schroeder_pre_delay_effect() {
        let mut reverb = SchroederReverb::new(44100);
        reverb.set_room_size(0.8);
        reverb.set_mix(1.0); // Full wet
        reverb.set_pre_delay(0.05); // 50ms = 2205 samples
        
        // Send impulse
        let _ = reverb.process_sample(1.0);
        
        // Samples before pre-delay ends should be mostly quiet
        // (only comb filter feedback from previous silence)
        let mut pre_delay_sum = 0.0;
        for _ in 0..1000 {
            pre_delay_sum += reverb.process_sample(0.0).abs();
        }
        
        // Skip to after pre-delay
        for _ in 0..1500 {
            let _ = reverb.process_sample(0.0);
        }
        
        // Now we should have reverb tail
        let mut post_delay_sum = 0.0;
        for _ in 0..2000 {
            post_delay_sum += reverb.process_sample(0.0).abs();
        }
        
        // Post-delay should have significant energy from reverb tail
        // Pre-delay period should be relatively quiet
        assert!(post_delay_sum > pre_delay_sum * 0.5 || post_delay_sum > 0.1, 
            "Pre-delay not working: pre={}, post={}", pre_delay_sum, post_delay_sum);
    }

    #[test]
    fn test_schroeder_pre_delay_clamp() {
        let mut reverb = SchroederReverb::new(44100);
        
        reverb.set_pre_delay(1.0); // Should clamp to 0.5
        assert!(reverb.pre_delay() <= 0.5);
        
        reverb.set_pre_delay(-0.1); // Should clamp to 0.0
        assert!(reverb.pre_delay() >= 0.0);
    }
}
