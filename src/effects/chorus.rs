//! Chorus effect - modulated delay

use super::delay::DelayLine;

/// Chorus effect
#[derive(Debug, Clone)]
pub struct Chorus {
    delays: [DelayLine; 2],
    lfo_phase: f32,
    rate: f32,           // Hz
    depth: f32,          // 0.0-1.0
    mix: f32,
    base_delay: f32,     // seconds
    sample_rate: f32,
}

impl Chorus {
    pub fn new(sample_rate: u32) -> Self {
        // Max delay ~50ms
        let max_samples = (0.05 * sample_rate as f32) as usize;
        Self {
            delays: [DelayLine::new(max_samples), DelayLine::new(max_samples)],
            lfo_phase: 0.0,
            rate: 0.5,
            depth: 0.5,
            mix: 0.5,
            base_delay: 0.015, // 15ms
            sample_rate: sample_rate as f32,
        }
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate.clamp(0.1, 10.0);
    }

    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth.clamp(0.0, 1.0);
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        // Two LFOs, 90 degrees apart for stereo effect
        let lfo1 = (self.lfo_phase * std::f32::consts::TAU).sin();
        let lfo2 = ((self.lfo_phase + 0.25) * std::f32::consts::TAU).sin();

        // Modulated delay times
        let delay_mod = self.depth * 0.005 * self.sample_rate; // Up to 5ms modulation
        let delay1 = self.base_delay * self.sample_rate + lfo1 * delay_mod;
        let delay2 = self.base_delay * self.sample_rate + lfo2 * delay_mod;

        // Write to both delay lines
        self.delays[0].write(input);
        self.delays[1].write(input);

        // Read with interpolation
        let wet1 = self.delays[0].read_interp(delay1);
        let wet2 = self.delays[1].read_interp(delay2);

        // Advance LFO
        self.lfo_phase += self.rate / self.sample_rate;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }

        // Mix (mono output, stereo would return tuple)
        let wet = (wet1 + wet2) * 0.5;
        input * (1.0 - self.mix) + wet * self.mix
    }

    pub fn process_stereo(&mut self, input: f32) -> (f32, f32) {
        let lfo1 = (self.lfo_phase * std::f32::consts::TAU).sin();
        let lfo2 = ((self.lfo_phase + 0.25) * std::f32::consts::TAU).sin();

        let delay_mod = self.depth * 0.005 * self.sample_rate;
        let delay1 = self.base_delay * self.sample_rate + lfo1 * delay_mod;
        let delay2 = self.base_delay * self.sample_rate + lfo2 * delay_mod;

        self.delays[0].write(input);
        self.delays[1].write(input);

        let wet1 = self.delays[0].read_interp(delay1);
        let wet2 = self.delays[1].read_interp(delay2);

        self.lfo_phase += self.rate / self.sample_rate;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }

        let left = input * (1.0 - self.mix) + wet1 * self.mix;
        let right = input * (1.0 - self.mix) + wet2 * self.mix;

        (left, right)
    }

    pub fn reset(&mut self) {
        self.delays[0].reset();
        self.delays[1].reset();
        self.lfo_phase = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chorus_new() {
        let chorus = Chorus::new(44100);
        assert_eq!(chorus.sample_rate, 44100.0);
    }

    #[test]
    fn test_chorus_process() {
        let mut chorus = Chorus::new(44100);
        chorus.set_rate(1.0);
        chorus.set_depth(0.5);
        chorus.set_mix(0.5);

        let out = chorus.process_sample(0.5);
        assert!(out.is_finite());
    }

    #[test]
    fn test_chorus_modulation() {
        let mut chorus = Chorus::new(44100);
        chorus.set_rate(1.0);
        chorus.set_depth(1.0);
        chorus.set_mix(1.0);

        // Process enough samples for LFO to cycle
        let mut samples = Vec::new();
        for _ in 0..44100 {
            samples.push(chorus.process_sample(0.5));
        }

        // Should have variation due to modulation
        let min = samples.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min > 0.01);
    }

    #[test]
    fn test_chorus_stereo() {
        let mut chorus = Chorus::new(44100);
        chorus.set_mix(1.0);

        let (l, r) = chorus.process_stereo(0.5);
        assert!(l.is_finite());
        assert!(r.is_finite());
    }

    #[test]
    fn test_chorus_reset() {
        let mut chorus = Chorus::new(44100);
        chorus.process_sample(1.0);
        chorus.reset();
        assert_eq!(chorus.lfo_phase, 0.0);
    }
}
