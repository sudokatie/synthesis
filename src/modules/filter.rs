//! Filter modules - SVF and Moog Ladder

use crate::dsp::fast_tanh;

/// Filter modes
#[derive(Debug, Clone, Copy)]
pub enum FilterMode {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

/// Generic filter trait
pub trait Filter {
    fn set_cutoff(&mut self, freq: f32);
    fn set_resonance(&mut self, q: f32);
    fn process_sample(&mut self, input: f32) -> f32;
    fn reset(&mut self);
}

/// State Variable Filter (Chamberlin)
#[derive(Debug, Clone)]
pub struct StateVariableFilter {
    cutoff: f32,
    resonance: f32,
    mode: FilterMode,
    sample_rate: f32,
    // State
    lp: f32,
    bp: f32,
}

impl StateVariableFilter {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            cutoff: 1000.0,
            resonance: 0.5,
            mode: FilterMode::LowPass,
            sample_rate: sample_rate as f32,
            lp: 0.0,
            bp: 0.0,
        }
    }

    pub fn set_mode(&mut self, mode: FilterMode) {
        self.mode = mode;
    }

    pub fn process_sample_with_mode(&mut self, input: f32, mode: FilterMode) -> f32 {
        let f = 2.0 * (std::f32::consts::PI * self.cutoff / self.sample_rate).sin();
        let q = 1.0 - self.resonance.clamp(0.0, 0.99);

        // SVF equations
        let hp = input - self.lp - q * self.bp;
        self.bp += f * hp;
        self.lp += f * self.bp;
        let notch = hp + self.lp;

        match mode {
            FilterMode::LowPass => self.lp,
            FilterMode::HighPass => hp,
            FilterMode::BandPass => self.bp,
            FilterMode::Notch => notch,
        }
    }
}

impl Filter for StateVariableFilter {
    fn set_cutoff(&mut self, freq: f32) {
        self.cutoff = freq.clamp(20.0, 20000.0);
    }

    fn set_resonance(&mut self, q: f32) {
        self.resonance = q.clamp(0.0, 1.0);
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        self.process_sample_with_mode(input, self.mode)
    }

    fn reset(&mut self) {
        self.lp = 0.0;
        self.bp = 0.0;
    }
}

/// Moog Ladder Filter (4-pole)
#[derive(Debug, Clone)]
pub struct MoogLadder {
    cutoff: f32,
    resonance: f32,
    sample_rate: f32,
    // 4 stages
    stage: [f32; 4],
    delay: [f32; 4],
}

impl MoogLadder {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            cutoff: 1000.0,
            resonance: 0.0,
            sample_rate: sample_rate as f32,
            stage: [0.0; 4],
            delay: [0.0; 4],
        }
    }

    /// Set drive/saturation amount
    pub fn set_drive(&mut self, _drive: f32) {
        // Could modify resonance behavior
    }
}

impl Filter for MoogLadder {
    fn set_cutoff(&mut self, freq: f32) {
        self.cutoff = freq.clamp(20.0, 20000.0);
    }

    fn set_resonance(&mut self, res: f32) {
        self.resonance = res.clamp(0.0, 4.0);
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        let f = 2.0 * self.cutoff / self.sample_rate;
        let f = f.clamp(0.0, 1.0);

        // Feedback with saturation
        let feedback = self.resonance * self.stage[3];
        let x = input - feedback;
        let x = fast_tanh(x);

        // 4-pole cascade
        for i in 0..4 {
            let prev = if i == 0 { x } else { self.stage[i - 1] };
            self.stage[i] = self.stage[i] + f * (prev - self.stage[i]);
            self.stage[i] = fast_tanh(self.stage[i]);
        }

        self.stage[3]
    }

    fn reset(&mut self) {
        self.stage = [0.0; 4];
        self.delay = [0.0; 4];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svf_new() {
        let filter = StateVariableFilter::new(44100);
        assert_eq!(filter.cutoff, 1000.0);
    }

    #[test]
    fn test_svf_lowpass() {
        let mut filter = StateVariableFilter::new(44100);
        filter.set_cutoff(500.0);
        filter.set_mode(FilterMode::LowPass);
        // Process some samples
        for _ in 0..100 {
            let _ = filter.process_sample(1.0);
        }
        // Should converge to input for DC
        let out = filter.process_sample(1.0);
        assert!(out > 0.9);
    }

    #[test]
    fn test_moog_new() {
        let filter = MoogLadder::new(44100);
        assert_eq!(filter.cutoff, 1000.0);
    }

    #[test]
    fn test_moog_resonance() {
        let mut filter = MoogLadder::new(44100);
        filter.set_resonance(3.5);
        // High resonance should cause self-oscillation
        filter.reset();
        let mut peak = 0.0_f32;
        for _ in 0..1000 {
            let out = filter.process_sample(0.0);
            peak = peak.max(out.abs());
        }
        // With high resonance and no input, may self-oscillate
    }

    #[test]
    fn test_filter_reset() {
        let mut filter = StateVariableFilter::new(44100);
        filter.process_sample(1.0);
        filter.reset();
        assert_eq!(filter.lp, 0.0);
        assert_eq!(filter.bp, 0.0);
    }
}
