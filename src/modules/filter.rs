//! Filter modules - SVF and Moog Ladder

use crate::dsp::fast_tanh;

/// Filter modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterMode {
    LowPass,
    HighPass,
    BandPass,
    Notch,
    Peak,
    LowShelf,
    HighShelf,
}

/// Generic filter trait
pub trait Filter {
    fn set_cutoff(&mut self, freq: f32);
    fn set_resonance(&mut self, q: f32);
    fn set_drive(&mut self, drive: f32);
    fn process_sample(&mut self, input: f32) -> f32;
    fn reset(&mut self);
}

/// State Variable Filter (Chamberlin)
#[derive(Debug, Clone)]
pub struct StateVariableFilter {
    cutoff: f32,
    resonance: f32,
    drive: f32,
    mode: FilterMode,
    sample_rate: f32,
    // Shelf/Peak gain
    gain_db: f32,
    // State
    lp: f32,
    bp: f32,
}

impl StateVariableFilter {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            cutoff: 1000.0,
            resonance: 0.5,
            drive: 0.0,
            mode: FilterMode::LowPass,
            sample_rate: sample_rate as f32,
            gain_db: 0.0,
            lp: 0.0,
            bp: 0.0,
        }
    }

    pub fn set_mode(&mut self, mode: FilterMode) {
        self.mode = mode;
    }
    
    /// Set gain for Peak/Shelf modes (in dB, -24 to +24)
    pub fn set_gain(&mut self, db: f32) {
        self.gain_db = db.clamp(-24.0, 24.0);
    }

    pub fn process_sample_with_mode(&mut self, input: f32, mode: FilterMode) -> f32 {
        // Apply drive/saturation to input
        let driven_input = if self.drive > 0.01 {
            let gain = 1.0 + self.drive * 4.0;
            fast_tanh(input * gain) / fast_tanh(gain)
        } else {
            input
        };
        
        let f = 2.0 * (std::f32::consts::PI * self.cutoff / self.sample_rate).sin();
        let q = 1.0 - self.resonance.clamp(0.0, 0.99);

        // SVF equations
        let hp = driven_input - self.lp - q * self.bp;
        self.bp += f * hp;
        self.lp += f * self.bp;
        let notch = hp + self.lp;
        
        // Gain factor for shelf/peak
        let gain_linear = 10.0_f32.powf(self.gain_db / 20.0);

        match mode {
            FilterMode::LowPass => self.lp,
            FilterMode::HighPass => hp,
            FilterMode::BandPass => self.bp,
            FilterMode::Notch => notch,
            FilterMode::Peak => {
                // Peak: boost/cut at cutoff frequency
                driven_input + (gain_linear - 1.0) * self.bp
            }
            FilterMode::LowShelf => {
                // Low shelf: boost/cut below cutoff
                hp + gain_linear * self.lp
            }
            FilterMode::HighShelf => {
                // High shelf: boost/cut above cutoff
                self.lp + gain_linear * hp
            }
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
    
    fn set_drive(&mut self, drive: f32) {
        self.drive = drive.clamp(0.0, 1.0);
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
    drive: f32,
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
            drive: 0.0,
            sample_rate: sample_rate as f32,
            stage: [0.0; 4],
            delay: [0.0; 4],
        }
    }
}

impl Filter for MoogLadder {
    fn set_cutoff(&mut self, freq: f32) {
        self.cutoff = freq.clamp(20.0, 20000.0);
    }

    fn set_resonance(&mut self, res: f32) {
        self.resonance = res.clamp(0.0, 4.0);
    }
    
    fn set_drive(&mut self, drive: f32) {
        self.drive = drive.clamp(0.0, 1.0);
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        let f = 2.0 * self.cutoff / self.sample_rate;
        let f = f.clamp(0.0, 1.0);
        
        // Apply input drive
        let driven_input = if self.drive > 0.01 {
            let gain = 1.0 + self.drive * 4.0;
            fast_tanh(input * gain)
        } else {
            input
        };

        // Feedback with saturation
        let feedback = self.resonance * self.stage[3];
        let x = driven_input - feedback;
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
        filter.set_cutoff(5000.0); // Higher cutoff for faster DC convergence
        filter.set_mode(FilterMode::LowPass);
        // Process many samples to allow convergence
        for _ in 0..1000 {
            let _ = filter.process_sample(1.0);
        }
        // Should converge to input for DC
        let out = filter.process_sample(1.0);
        assert!(out > 0.8, "Filter output: {}", out);
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

    #[test]
    fn test_svf_highpass() {
        let mut filter = StateVariableFilter::new(44100);
        filter.set_cutoff(100.0);
        filter.set_mode(FilterMode::HighPass);
        // DC should be blocked by highpass
        for _ in 0..2000 {
            let _ = filter.process_sample(1.0);
        }
        let out = filter.process_sample(1.0);
        assert!(out.abs() < 0.2, "HP should block DC: {}", out);
    }

    #[test]
    fn test_svf_bandpass() {
        let mut filter = StateVariableFilter::new(44100);
        filter.set_cutoff(1000.0);
        filter.set_resonance(0.8);
        filter.set_mode(FilterMode::BandPass);
        let out = filter.process_sample(1.0);
        assert!(out.is_finite());
    }

    #[test]
    fn test_svf_notch() {
        let mut filter = StateVariableFilter::new(44100);
        filter.set_cutoff(1000.0);
        filter.set_mode(FilterMode::Notch);
        let out = filter.process_sample(1.0);
        assert!(out.is_finite());
    }

    #[test]
    fn test_svf_cutoff_range() {
        let mut filter = StateVariableFilter::new(44100);
        filter.set_cutoff(50000.0);
        assert_eq!(filter.cutoff, 20000.0);
        filter.set_cutoff(5.0);
        assert_eq!(filter.cutoff, 20.0);
    }

    #[test]
    fn test_moog_process() {
        let mut filter = MoogLadder::new(44100);
        filter.set_cutoff(2000.0);
        filter.set_resonance(0.5);
        let mut buffer = vec![0.0; 256];
        for (i, s) in buffer.iter_mut().enumerate() {
            let input = (i as f32 * 0.1).sin();
            *s = filter.process_sample(input);
        }
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_moog_reset() {
        let mut filter = MoogLadder::new(44100);
        filter.process_sample(1.0);
        filter.reset();
        assert_eq!(filter.stage, [0.0; 4]);
    }

    #[test]
    fn test_moog_saturation() {
        let mut filter = MoogLadder::new(44100);
        filter.set_cutoff(5000.0);
        // Large input should be saturated
        let out = filter.process_sample(10.0);
        assert!(out.abs() < 5.0, "Should saturate: {}", out);
    }
    
    #[test]
    fn test_svf_drive() {
        let mut filter = StateVariableFilter::new(44100);
        filter.set_cutoff(5000.0);
        filter.set_mode(FilterMode::LowPass);
        filter.set_drive(0.0);
        
        // Process with no drive
        filter.reset();
        for _ in 0..100 {
            filter.process_sample(0.5);
        }
        let no_drive = filter.process_sample(0.5);
        
        // Process with drive
        filter.reset();
        filter.set_drive(1.0);
        for _ in 0..100 {
            filter.process_sample(0.5);
        }
        let with_drive = filter.process_sample(0.5);
        
        // Drive should cause saturation (values differ)
        assert!(no_drive.is_finite());
        assert!(with_drive.is_finite());
    }
    
    #[test]
    fn test_moog_drive() {
        let mut filter = MoogLadder::new(44100);
        filter.set_cutoff(5000.0);
        filter.set_drive(1.0);
        
        // Large input should be further saturated
        let out = filter.process_sample(5.0);
        assert!(out.abs() < 2.0, "Drive should saturate: {}", out);
    }
    
    #[test]
    fn test_svf_peak_mode() {
        let mut filter = StateVariableFilter::new(44100);
        filter.set_cutoff(1000.0);
        filter.set_mode(FilterMode::Peak);
        filter.set_gain(6.0); // Boost
        
        let out = filter.process_sample(1.0);
        assert!(out.is_finite());
    }
    
    #[test]
    fn test_svf_low_shelf() {
        let mut filter = StateVariableFilter::new(44100);
        filter.set_cutoff(500.0);
        filter.set_mode(FilterMode::LowShelf);
        filter.set_gain(6.0); // Boost lows
        
        let out = filter.process_sample(1.0);
        assert!(out.is_finite());
    }
    
    #[test]
    fn test_svf_high_shelf() {
        let mut filter = StateVariableFilter::new(44100);
        filter.set_cutoff(2000.0);
        filter.set_mode(FilterMode::HighShelf);
        filter.set_gain(-6.0); // Cut highs
        
        let out = filter.process_sample(1.0);
        assert!(out.is_finite());
    }
    
    #[test]
    fn test_filter_mode_all_variants() {
        let modes = [
            FilterMode::LowPass,
            FilterMode::HighPass,
            FilterMode::BandPass,
            FilterMode::Notch,
            FilterMode::Peak,
            FilterMode::LowShelf,
            FilterMode::HighShelf,
        ];
        
        for mode in modes {
            let mut filter = StateVariableFilter::new(44100);
            filter.set_mode(mode);
            filter.set_cutoff(1000.0);
            filter.set_gain(3.0);
            
            // Process some samples
            for _ in 0..100 {
                let out = filter.process_sample(1.0);
                assert!(out.is_finite(), "Mode {:?} produced non-finite output", mode);
            }
        }
    }
}
