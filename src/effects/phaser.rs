//! Phaser effect - all-pass filter chain with LFO modulation

/// Soft clip function to prevent feedback blowup
fn soft_clip(x: f32) -> f32 {
    // tanh-like soft clipper
    let abs_x = x.abs();
    if abs_x <= 1.0 {
        x
    } else {
        x.signum() * (1.0 + (abs_x - 1.0).tanh() * 0.5)
    }
}

/// All-pass filter stage for phaser
#[derive(Debug, Clone, Copy, Default)]
struct AllPassStage {
    delay: f32,
}

impl AllPassStage {
    fn process(&mut self, input: f32, coeff: f32) -> f32 {
        // First-order all-pass: y[n] = coeff * (x[n] - y[n-1]) + x[n-1]
        // Simplified: y[n] = coeff * x[n] + x[n-1] - coeff * y[n-1]
        let output = coeff * input + self.delay - coeff * self.delay;
        self.delay = input;
        output
    }
    
    fn reset(&mut self) {
        self.delay = 0.0;
    }
}

/// Number of stages in the phaser (affects depth of notches)
pub const PHASER_STAGES: usize = 6;

/// Phaser effect
#[derive(Debug, Clone)]
pub struct Phaser {
    /// All-pass filter stages
    stages: [AllPassStage; PHASER_STAGES],
    /// LFO phase (0.0 to 1.0)
    lfo_phase: f32,
    /// LFO rate in Hz
    rate: f32,
    /// Modulation depth (0.0 to 1.0)
    depth: f32,
    /// Feedback amount (-1.0 to 1.0)
    feedback: f32,
    /// Dry/wet mix (0.0 to 1.0)
    mix: f32,
    /// Minimum frequency for sweep (Hz)
    min_freq: f32,
    /// Maximum frequency for sweep (Hz)
    max_freq: f32,
    /// Sample rate
    sample_rate: f32,
    /// Feedback delay line
    feedback_sample: f32,
}

impl Phaser {
    /// Create a new phaser effect
    pub fn new(sample_rate: u32) -> Self {
        Self {
            stages: [AllPassStage::default(); PHASER_STAGES],
            lfo_phase: 0.0,
            rate: 0.5,
            depth: 0.7,
            feedback: 0.5,
            mix: 0.5,
            min_freq: 100.0,
            max_freq: 3000.0,
            sample_rate: sample_rate as f32,
            feedback_sample: 0.0,
        }
    }
    
    /// Set LFO rate in Hz (0.01 to 10.0)
    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate.clamp(0.01, 10.0);
    }
    
    /// Set modulation depth (0.0 to 1.0)
    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth.clamp(0.0, 1.0);
    }
    
    /// Set feedback amount (-1.0 to 1.0)
    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(-0.99, 0.99);
    }
    
    /// Set dry/wet mix (0.0 = dry, 1.0 = wet)
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }
    
    /// Set frequency range for sweep
    pub fn set_frequency_range(&mut self, min_hz: f32, max_hz: f32) {
        self.min_freq = min_hz.clamp(20.0, 5000.0);
        self.max_freq = max_hz.clamp(100.0, 15000.0);
        if self.min_freq > self.max_freq {
            std::mem::swap(&mut self.min_freq, &mut self.max_freq);
        }
    }
    
    /// Process a single sample
    pub fn process_sample(&mut self, input: f32) -> f32 {
        // Calculate LFO value (sine wave, 0.0 to 1.0)
        let lfo = (self.lfo_phase * std::f32::consts::TAU).sin() * 0.5 + 0.5;
        
        // Calculate sweep frequency
        let sweep = self.min_freq + (self.max_freq - self.min_freq) * lfo * self.depth;
        
        // Convert frequency to all-pass coefficient
        // coeff = (tan(pi * f / fs) - 1) / (tan(pi * f / fs) + 1)
        let w = std::f32::consts::PI * sweep / self.sample_rate;
        let tan_w = w.tan().clamp(-10.0, 10.0); // Prevent extreme values
        let coeff = (tan_w - 1.0) / (tan_w + 1.0);
        
        // Input with feedback (soft-clipped to prevent blowup)
        let feedback_scaled = soft_clip(self.feedback_sample) * self.feedback;
        let input_with_feedback = input + feedback_scaled;
        
        // Process through all-pass stages
        let mut signal = input_with_feedback;
        for stage in &mut self.stages {
            signal = stage.process(signal, coeff);
        }
        
        // Store for feedback (clamp to prevent runaway)
        self.feedback_sample = signal.clamp(-10.0, 10.0);
        
        // Advance LFO
        self.lfo_phase += self.rate / self.sample_rate;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }
        
        // Mix dry and wet
        input * (1.0 - self.mix) + signal * self.mix
    }
    
    /// Process stereo with offset LFO phases
    pub fn process_stereo(&mut self, input: f32) -> (f32, f32) {
        // Process left channel normally
        let left = self.process_sample(input);
        
        // For right channel, use phase-offset LFO
        let right_lfo = ((self.lfo_phase + 0.5) * std::f32::consts::TAU).sin() * 0.5 + 0.5;
        let sweep = self.min_freq + (self.max_freq - self.min_freq) * right_lfo * self.depth;
        let w = std::f32::consts::PI * sweep / self.sample_rate;
        let tan_w = w.tan();
        let coeff = (tan_w - 1.0) / (tan_w + 1.0);
        
        // Simple right channel approximation (phase offset creates width)
        let right = input * (1.0 - self.mix) + self.feedback_sample * self.mix * coeff.abs();
        
        (left, right)
    }
    
    /// Reset all filter states
    pub fn reset(&mut self) {
        for stage in &mut self.stages {
            stage.reset();
        }
        self.lfo_phase = 0.0;
        self.feedback_sample = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_phaser_new() {
        let phaser = Phaser::new(44100);
        assert_eq!(phaser.sample_rate, 44100.0);
        assert_eq!(phaser.rate, 0.5);
    }
    
    #[test]
    fn test_phaser_process() {
        let mut phaser = Phaser::new(44100);
        
        let out = phaser.process_sample(0.5);
        assert!(out.is_finite());
    }
    
    #[test]
    fn test_phaser_sweep() {
        let mut phaser = Phaser::new(44100);
        phaser.set_rate(1.0);
        phaser.set_depth(1.0);
        phaser.set_mix(1.0);
        
        // Process a full LFO cycle
        let mut samples = Vec::new();
        for _ in 0..44100 {
            samples.push(phaser.process_sample(0.5));
        }
        
        // Should have variation due to sweep
        let min = samples.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min > 0.001, "Phaser should produce variation");
    }
    
    #[test]
    fn test_phaser_feedback() {
        let mut phaser = Phaser::new(44100);
        phaser.set_feedback(0.8);
        phaser.set_mix(1.0);
        
        // High feedback should create resonance
        for _ in 0..1000 {
            let out = phaser.process_sample(0.5);
            assert!(out.is_finite(), "Phaser with feedback should remain stable");
        }
    }
    
    #[test]
    fn test_phaser_negative_feedback() {
        let mut phaser = Phaser::new(44100);
        phaser.set_feedback(-0.7);
        
        for _ in 0..1000 {
            let out = phaser.process_sample(0.5);
            assert!(out.is_finite());
        }
    }
    
    #[test]
    fn test_phaser_stereo() {
        let mut phaser = Phaser::new(44100);
        phaser.set_mix(1.0);
        
        let (l, r) = phaser.process_stereo(0.5);
        assert!(l.is_finite());
        assert!(r.is_finite());
    }
    
    #[test]
    fn test_phaser_reset() {
        let mut phaser = Phaser::new(44100);
        
        for _ in 0..1000 {
            phaser.process_sample(1.0);
        }
        
        phaser.reset();
        assert_eq!(phaser.lfo_phase, 0.0);
        assert_eq!(phaser.feedback_sample, 0.0);
    }
    
    #[test]
    fn test_phaser_frequency_range() {
        let mut phaser = Phaser::new(44100);
        phaser.set_frequency_range(200.0, 5000.0);
        
        assert_eq!(phaser.min_freq, 200.0);
        assert_eq!(phaser.max_freq, 5000.0);
    }
    
    #[test]
    fn test_phaser_frequency_range_swap() {
        let mut phaser = Phaser::new(44100);
        // If min > max, should swap
        phaser.set_frequency_range(5000.0, 200.0);
        
        assert!(phaser.min_freq < phaser.max_freq);
    }
    
    #[test]
    fn test_phaser_dry_wet() {
        let mut phaser = Phaser::new(44100);
        
        // Full dry
        phaser.set_mix(0.0);
        let dry = phaser.process_sample(0.5);
        assert!((dry - 0.5).abs() < 0.01, "Dry should pass input unchanged");
        
        // Full wet
        phaser.reset();
        phaser.set_mix(1.0);
        let wet = phaser.process_sample(0.5);
        assert!(wet.is_finite());
    }
    
    #[test]
    fn test_phaser_rate_range() {
        let mut phaser = Phaser::new(44100);
        
        phaser.set_rate(0.001);
        assert_eq!(phaser.rate, 0.01);
        
        phaser.set_rate(100.0);
        assert_eq!(phaser.rate, 10.0);
    }
    
    #[test]
    fn test_allpass_stage() {
        let mut stage = AllPassStage::default();
        
        // Process some samples
        let out1 = stage.process(1.0, 0.5);
        let out2 = stage.process(1.0, 0.5);
        
        assert!(out1.is_finite());
        assert!(out2.is_finite());
        // All-pass should converge to input for DC
    }
}
