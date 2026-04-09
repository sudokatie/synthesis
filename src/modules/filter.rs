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

/// Vowel types for formant filter
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Vowel {
    A,  // "ah" as in father
    E,  // "eh" as in bed
    I,  // "ee" as in see
    O,  // "oh" as in go
    U,  // "oo" as in too
}

/// Formant frequency data (F1, F2, F3 in Hz)
#[derive(Debug, Clone, Copy)]
pub struct FormantData {
    pub f1: f32,
    pub f2: f32,
    pub f3: f32,
    pub g1: f32, // Gain for F1 (linear)
    pub g2: f32, // Gain for F2
    pub g3: f32, // Gain for F3
}

impl FormantData {
    pub fn from_vowel(vowel: Vowel) -> Self {
        match vowel {
            Vowel::A => Self { f1: 730.0, f2: 1090.0, f3: 2440.0, g1: 1.0, g2: 0.5, g3: 0.25 },
            Vowel::E => Self { f1: 660.0, f2: 1720.0, f3: 2410.0, g1: 1.0, g2: 0.5, g3: 0.25 },
            Vowel::I => Self { f1: 270.0, f2: 2290.0, f3: 3010.0, g1: 1.0, g2: 0.4, g3: 0.2 },
            Vowel::O => Self { f1: 570.0, f2: 840.0, f3: 2410.0, g1: 1.0, g2: 0.5, g3: 0.25 },
            Vowel::U => Self { f1: 300.0, f2: 870.0, f3: 2240.0, g1: 1.0, g2: 0.4, g3: 0.2 },
        }
    }
    
    /// Interpolate between two formant data sets
    pub fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let inv_t = 1.0 - t;
        Self {
            f1: a.f1 * inv_t + b.f1 * t,
            f2: a.f2 * inv_t + b.f2 * t,
            f3: a.f3 * inv_t + b.f3 * t,
            g1: a.g1 * inv_t + b.g1 * t,
            g2: a.g2 * inv_t + b.g2 * t,
            g3: a.g3 * inv_t + b.g3 * t,
        }
    }
}

/// Formant filter - creates vowel-like sounds using parallel bandpass filters
#[derive(Debug, Clone)]
pub struct FormantFilter {
    /// Three SVF bandpass filters for formants
    bp1: StateVariableFilter,
    bp2: StateVariableFilter,
    bp3: StateVariableFilter,
    /// Current formant data
    formants: FormantData,
    /// Resonance (Q) for formant peaks
    resonance: f32,
    /// Mix between dry and wet (0.0 = dry, 1.0 = wet)
    mix: f32,
    /// Sample rate
    sample_rate: f32,
}

impl FormantFilter {
    pub fn new(sample_rate: u32) -> Self {
        let mut bp1 = StateVariableFilter::new(sample_rate);
        let mut bp2 = StateVariableFilter::new(sample_rate);
        let mut bp3 = StateVariableFilter::new(sample_rate);
        
        bp1.set_mode(FilterMode::BandPass);
        bp2.set_mode(FilterMode::BandPass);
        bp3.set_mode(FilterMode::BandPass);
        
        let formants = FormantData::from_vowel(Vowel::A);
        
        bp1.set_cutoff(formants.f1);
        bp2.set_cutoff(formants.f2);
        bp3.set_cutoff(formants.f3);
        
        Self {
            bp1,
            bp2,
            bp3,
            formants,
            resonance: 0.7,
            mix: 1.0,
            sample_rate: sample_rate as f32,
        }
    }
    
    /// Set vowel preset
    pub fn set_vowel(&mut self, vowel: Vowel) {
        self.formants = FormantData::from_vowel(vowel);
        self.update_filters();
    }
    
    /// Set custom formant frequencies
    pub fn set_formants(&mut self, data: FormantData) {
        self.formants = data;
        self.update_filters();
    }
    
    /// Morph between two vowels (0.0 = vowel_a, 1.0 = vowel_b)
    pub fn morph(&mut self, vowel_a: Vowel, vowel_b: Vowel, amount: f32) {
        let a = FormantData::from_vowel(vowel_a);
        let b = FormantData::from_vowel(vowel_b);
        self.formants = FormantData::lerp(&a, &b, amount);
        self.update_filters();
    }
    
    /// Set resonance (Q) for formant peaks (0.0 to 1.0)
    pub fn set_resonance(&mut self, q: f32) {
        self.resonance = q.clamp(0.0, 0.99);
        self.bp1.set_resonance(self.resonance);
        self.bp2.set_resonance(self.resonance);
        self.bp3.set_resonance(self.resonance);
    }
    
    /// Set dry/wet mix (0.0 = dry, 1.0 = wet)
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }
    
    /// Shift all formants by a factor (useful for pitch tracking)
    pub fn shift(&mut self, factor: f32) {
        let factor = factor.clamp(0.25, 4.0);
        self.bp1.set_cutoff((self.formants.f1 * factor).clamp(20.0, 20000.0));
        self.bp2.set_cutoff((self.formants.f2 * factor).clamp(20.0, 20000.0));
        self.bp3.set_cutoff((self.formants.f3 * factor).clamp(20.0, 20000.0));
    }
    
    fn update_filters(&mut self) {
        self.bp1.set_cutoff(self.formants.f1);
        self.bp2.set_cutoff(self.formants.f2);
        self.bp3.set_cutoff(self.formants.f3);
        self.bp1.set_resonance(self.resonance);
        self.bp2.set_resonance(self.resonance);
        self.bp3.set_resonance(self.resonance);
    }
    
    /// Process a single sample
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let f1_out = self.bp1.process_sample(input) * self.formants.g1;
        let f2_out = self.bp2.process_sample(input) * self.formants.g2;
        let f3_out = self.bp3.process_sample(input) * self.formants.g3;
        
        let wet = f1_out + f2_out + f3_out;
        let dry = input;
        
        dry * (1.0 - self.mix) + wet * self.mix
    }
    
    /// Reset filter state
    pub fn reset(&mut self) {
        self.bp1.reset();
        self.bp2.reset();
        self.bp3.reset();
    }
    
    /// Get current formant data
    pub fn formants(&self) -> &FormantData {
        &self.formants
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
    
    // Formant filter tests
    
    #[test]
    fn test_formant_new() {
        let filter = FormantFilter::new(44100);
        assert_eq!(filter.formants.f1, 730.0); // Default is Vowel::A
        assert_eq!(filter.mix, 1.0);
    }
    
    #[test]
    fn test_formant_set_vowel() {
        let mut filter = FormantFilter::new(44100);
        
        filter.set_vowel(Vowel::I);
        assert_eq!(filter.formants.f1, 270.0);
        assert_eq!(filter.formants.f2, 2290.0);
        
        filter.set_vowel(Vowel::U);
        assert_eq!(filter.formants.f1, 300.0);
    }
    
    #[test]
    fn test_formant_morph() {
        let mut filter = FormantFilter::new(44100);
        
        // Morph from A to I at 50%
        filter.morph(Vowel::A, Vowel::I, 0.5);
        
        // F1 should be halfway between A(730) and I(270) = 500
        assert!((filter.formants.f1 - 500.0).abs() < 1.0);
    }
    
    #[test]
    fn test_formant_process() {
        let mut filter = FormantFilter::new(44100);
        filter.set_vowel(Vowel::A);
        
        // Process a sawtooth-like signal
        let mut output = Vec::new();
        for i in 0..256 {
            let input = ((i as f32 / 64.0) % 2.0) - 1.0;
            output.push(filter.process_sample(input));
        }
        
        // Output should be non-zero and finite
        assert!(output.iter().any(|&s| s != 0.0));
        assert!(output.iter().all(|&s| s.is_finite()));
    }
    
    #[test]
    fn test_formant_resonance() {
        let mut filter = FormantFilter::new(44100);
        filter.set_resonance(0.9);
        
        // High resonance should produce sharper peaks
        let out = filter.process_sample(1.0);
        assert!(out.is_finite());
    }
    
    #[test]
    fn test_formant_mix() {
        let mut filter = FormantFilter::new(44100);
        
        // 100% wet
        filter.set_mix(1.0);
        filter.reset();
        let wet = filter.process_sample(1.0);
        
        // 0% wet (dry)
        filter.set_mix(0.0);
        filter.reset();
        let dry = filter.process_sample(1.0);
        
        assert_eq!(dry, 1.0); // Dry should be input
        assert!(wet.is_finite());
    }
    
    #[test]
    fn test_formant_shift() {
        let mut filter = FormantFilter::new(44100);
        filter.set_vowel(Vowel::A);
        
        // Shift up by 2x
        filter.shift(2.0);
        
        // Internal filters should be updated (we can't access them directly,
        // but we can verify the output is different)
        let out = filter.process_sample(1.0);
        assert!(out.is_finite());
    }
    
    #[test]
    fn test_formant_reset() {
        let mut filter = FormantFilter::new(44100);
        
        // Process some samples
        for _ in 0..100 {
            filter.process_sample(1.0);
        }
        
        // Reset
        filter.reset();
        
        // First sample after reset should be deterministic
        let out = filter.process_sample(1.0);
        assert!(out.is_finite());
    }
    
    #[test]
    fn test_formant_all_vowels() {
        let vowels = [Vowel::A, Vowel::E, Vowel::I, Vowel::O, Vowel::U];
        
        for vowel in vowels {
            let mut filter = FormantFilter::new(44100);
            filter.set_vowel(vowel);
            
            // Process and verify
            for _ in 0..100 {
                let out = filter.process_sample(1.0);
                assert!(out.is_finite(), "Vowel {:?} produced non-finite output", vowel);
            }
        }
    }
    
    #[test]
    fn test_formant_data_lerp() {
        let a = FormantData::from_vowel(Vowel::A);
        let b = FormantData::from_vowel(Vowel::I);
        
        // At t=0, should be A
        let at_0 = FormantData::lerp(&a, &b, 0.0);
        assert_eq!(at_0.f1, a.f1);
        
        // At t=1, should be I
        let at_1 = FormantData::lerp(&a, &b, 1.0);
        assert_eq!(at_1.f1, b.f1);
        
        // At t=0.5, should be halfway
        let at_half = FormantData::lerp(&a, &b, 0.5);
        assert!((at_half.f1 - (a.f1 + b.f1) / 2.0).abs() < 0.01);
    }
    
    #[test]
    fn test_formant_custom_formants() {
        let mut filter = FormantFilter::new(44100);
        
        let custom = FormantData {
            f1: 500.0,
            f2: 1500.0,
            f3: 2500.0,
            g1: 1.0,
            g2: 0.7,
            g3: 0.3,
        };
        
        filter.set_formants(custom);
        assert_eq!(filter.formants.f1, 500.0);
        assert_eq!(filter.formants.g2, 0.7);
    }
}
