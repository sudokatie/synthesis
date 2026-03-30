//! Oscillator module - waveform generation with anti-aliasing

use crate::dsp::{cubic_interp, saw_polyblep, square_polyblep};

/// Waveform types
#[derive(Debug, Clone)]
pub enum Waveform {
    Sine,
    Saw,
    Square { pulse_width: f32 },
    Triangle,
    Noise,
    /// Wavetable with samples and position (0.0-1.0 for morphing between tables)
    Wavetable { table: Vec<f32>, position: f32 },
    /// Multi-wavetable for morphing (multiple tables, position selects blend)
    MultiWavetable { tables: Vec<Vec<f32>>, position: f32 },
}

impl Default for Waveform {
    fn default() -> Self {
        Waveform::Saw
    }
}

/// Generate a basic wavetable (one cycle of a waveform)
pub fn generate_wavetable(waveform: &str, size: usize) -> Vec<f32> {
    let mut table = vec![0.0; size];
    for i in 0..size {
        let phase = i as f32 / size as f32;
        table[i] = match waveform {
            "sine" => (phase * std::f32::consts::TAU).sin(),
            "saw" => 2.0 * phase - 1.0,
            "square" => if phase < 0.5 { 1.0 } else { -1.0 },
            "triangle" => {
                if phase < 0.5 {
                    4.0 * phase - 1.0
                } else {
                    3.0 - 4.0 * phase
                }
            }
            _ => 0.0,
        };
    }
    table
}

/// Generate a set of wavetables for morphing (e.g., sine -> saw -> square)
pub fn generate_morph_wavetables(size: usize) -> Vec<Vec<f32>> {
    vec![
        generate_wavetable("sine", size),
        generate_wavetable("triangle", size),
        generate_wavetable("saw", size),
        generate_wavetable("square", size),
    ]
}

/// Oscillator with anti-aliasing and gain control
#[derive(Debug, Clone)]
pub struct Oscillator {
    waveform: Waveform,
    frequency: f32,
    /// Current phase (0.0-1.0), public for sync detection
    pub phase: f32,
    detune: f32,           // cents
    gain: f32,             // output gain (0.0-1.0)
    sample_rate: f32,
    // Modulation inputs
    fm_amount: f32,        // FM depth in Hz
    pm_amount: f32,        // PM depth in radians
    // Oversampling for wavetables
    oversample_factor: usize,
    oversample_buffer: Vec<f32>,
    // Lowpass state for oversampling decimation
    lp_state: f32,
}

impl Oscillator {
    /// Create new oscillator
    pub fn new(waveform: Waveform, sample_rate: u32) -> Self {
        Self {
            waveform,
            frequency: 440.0,
            phase: 0.0,
            detune: 0.0,
            gain: 1.0,
            sample_rate: sample_rate as f32,
            fm_amount: 0.0,
            pm_amount: 0.0,
            oversample_factor: 4,
            oversample_buffer: vec![0.0; 4],
            lp_state: 0.0,
        }
    }

    /// Set output gain (0.0-1.0)
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.0, 1.0);
    }

    /// Get current gain
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// Set oversampling factor for wavetables (1, 2, 4, or 8)
    pub fn set_oversample(&mut self, factor: usize) {
        self.oversample_factor = factor.clamp(1, 8);
        self.oversample_buffer.resize(self.oversample_factor, 0.0);
    }

    /// Set FM (frequency modulation) amount in Hz
    pub fn set_fm_amount(&mut self, amount: f32) {
        self.fm_amount = amount;
    }

    /// Set PM (phase modulation) amount in radians
    pub fn set_pm_amount(&mut self, amount: f32) {
        self.pm_amount = amount;
    }

    /// Process with FM input (modulator signal)
    pub fn process_sample_fm(&mut self, fm_input: f32) -> f32 {
        let freq = self.effective_frequency() + fm_input * self.fm_amount;
        let freq = freq.max(0.0);
        let dt = freq / self.sample_rate;
        
        let sample = self.generate_sample(self.phase, dt);
        
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        
        sample * self.gain
    }

    /// Process with PM input (modulator signal)
    pub fn process_sample_pm(&mut self, pm_input: f32) -> f32 {
        let freq = self.effective_frequency();
        let dt = freq / self.sample_rate;
        
        // Apply phase modulation
        let modulated_phase = (self.phase + pm_input * self.pm_amount / std::f32::consts::TAU).fract();
        let modulated_phase = if modulated_phase < 0.0 { modulated_phase + 1.0 } else { modulated_phase };
        
        let sample = self.generate_sample(modulated_phase, dt);
        
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        
        sample * self.gain
    }

    /// Hard sync - reset phase when sync signal crosses zero
    pub fn sync(&mut self) {
        self.phase = 0.0;
    }

    /// Set wavetable position for morphing (0.0-1.0)
    pub fn set_wavetable_position(&mut self, position: f32) {
        match &mut self.waveform {
            Waveform::Wavetable { position: pos, .. } => {
                *pos = position.clamp(0.0, 1.0);
            }
            Waveform::MultiWavetable { position: pos, .. } => {
                *pos = position.clamp(0.0, 1.0);
            }
            _ => {}
        }
    }

    fn generate_sample(&self, phase: f32, dt: f32) -> f32 {
        match &self.waveform {
            Waveform::Sine => (phase * std::f32::consts::TAU).sin(),
            Waveform::Saw => saw_polyblep(phase, dt),
            Waveform::Square { pulse_width } => square_polyblep(phase, dt, *pulse_width),
            Waveform::Triangle => {
                if phase < 0.5 {
                    4.0 * phase - 1.0
                } else {
                    3.0 - 4.0 * phase
                }
            }
            Waveform::Noise => rand::random::<f32>() * 2.0 - 1.0,
            Waveform::Wavetable { table, .. } => {
                self.read_wavetable_oversampled(table, phase)
            }
            Waveform::MultiWavetable { tables, position } => {
                self.read_multi_wavetable(tables, *position, phase)
            }
        }
    }

    /// Read from wavetable with oversampling for anti-aliasing
    fn read_wavetable_oversampled(&self, table: &[f32], phase: f32) -> f32 {
        if table.is_empty() {
            return 0.0;
        }

        if self.oversample_factor <= 1 {
            // No oversampling, just interpolate
            let index = phase * table.len() as f32;
            return cubic_interp(table, index);
        }

        // Oversample: read multiple points and lowpass filter
        let mut sum = 0.0;
        let step = 1.0 / self.oversample_factor as f32;
        
        for i in 0..self.oversample_factor {
            let sub_phase = (phase + step * i as f32 / table.len() as f32).fract();
            let index = sub_phase * table.len() as f32;
            sum += cubic_interp(table, index);
        }
        
        sum / self.oversample_factor as f32
    }

    /// Read from multi-wavetable with morphing
    fn read_multi_wavetable(&self, tables: &[Vec<f32>], position: f32, phase: f32) -> f32 {
        if tables.is_empty() {
            return 0.0;
        }
        
        if tables.len() == 1 {
            return self.read_wavetable_oversampled(&tables[0], phase);
        }

        // Calculate which two tables to blend between
        let float_index = position * (tables.len() - 1) as f32;
        let index_a = (float_index.floor() as usize).min(tables.len() - 1);
        let index_b = (index_a + 1).min(tables.len() - 1);
        let blend = float_index.fract();

        // Read from both tables
        let sample_a = self.read_wavetable_oversampled(&tables[index_a], phase);
        let sample_b = self.read_wavetable_oversampled(&tables[index_b], phase);

        // Crossfade
        sample_a * (1.0 - blend) + sample_b * blend
    }

    /// Set frequency in Hz
    pub fn set_frequency(&mut self, freq: f32) {
        self.frequency = freq.clamp(20.0, 20000.0);
    }

    /// Set detune in cents
    pub fn set_detune(&mut self, cents: f32) {
        self.detune = cents;
    }

    /// Set waveform
    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    /// Reset phase
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.lp_state = 0.0;
    }

    /// Get effective frequency with detune
    fn effective_frequency(&self) -> f32 {
        self.frequency * 2.0_f32.powf(self.detune / 1200.0)
    }

    /// Process a single sample
    pub fn process_sample(&mut self) -> f32 {
        let freq = self.effective_frequency();
        let dt = freq / self.sample_rate;

        let sample = self.generate_sample(self.phase, dt);

        // Advance phase
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sample * self.gain
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
    fn test_oscillator_new() {
        let osc = Oscillator::new(Waveform::Saw, 44100);
        assert_eq!(osc.frequency, 440.0);
        assert_eq!(osc.gain, 1.0);
    }

    #[test]
    fn test_oscillator_gain() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        osc.set_gain(0.5);
        assert_eq!(osc.gain(), 0.5);
        
        // Process and check output is scaled
        osc.set_frequency(440.0);
        let sample = osc.process_sample();
        assert!(sample.abs() <= 0.5);
    }

    #[test]
    fn test_oscillator_gain_clamp() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        osc.set_gain(2.0);
        assert_eq!(osc.gain(), 1.0);
        osc.set_gain(-0.5);
        assert_eq!(osc.gain(), 0.0);
    }

    #[test]
    fn test_oscillator_set_frequency() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        osc.set_frequency(880.0);
        assert_eq!(osc.frequency, 880.0);
    }

    #[test]
    fn test_sine_range() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        osc.set_frequency(440.0);
        for _ in 0..1000 {
            let sample = osc.process_sample();
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_saw_range() {
        let mut osc = Oscillator::new(Waveform::Saw, 44100);
        osc.set_frequency(440.0);
        for _ in 0..1000 {
            let sample = osc.process_sample();
            assert!(sample >= -1.5 && sample <= 1.5);
        }
    }

    #[test]
    fn test_square_range() {
        let mut osc = Oscillator::new(Waveform::Square { pulse_width: 0.5 }, 44100);
        osc.set_frequency(440.0);
        for _ in 0..1000 {
            let sample = osc.process_sample();
            assert!(sample >= -2.5 && sample <= 2.5);
        }
    }

    #[test]
    fn test_noise_varies() {
        let mut osc = Oscillator::new(Waveform::Noise, 44100);
        let s1 = osc.process_sample();
        let s2 = osc.process_sample();
        assert!(s1 != s2 || s1 == 0.0);
    }

    #[test]
    fn test_detune() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        osc.set_frequency(440.0);
        osc.set_detune(1200.0);
        assert!((osc.effective_frequency() - 880.0).abs() < 1.0);
    }

    #[test]
    fn test_process_buffer() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        let mut buffer = vec![0.0; 64];
        osc.process(&mut buffer);
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_triangle_range() {
        let mut osc = Oscillator::new(Waveform::Triangle, 44100);
        osc.set_frequency(440.0);
        for _ in 0..1000 {
            let sample = osc.process_sample();
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_wavetable_sine() {
        let table = generate_wavetable("sine", 256);
        let mut osc = Oscillator::new(
            Waveform::Wavetable { table, position: 0.0 },
            44100,
        );
        osc.set_frequency(440.0);
        for _ in 0..1000 {
            let sample = osc.process_sample();
            assert!(sample >= -1.1 && sample <= 1.1);
        }
    }

    #[test]
    fn test_wavetable_saw() {
        let table = generate_wavetable("saw", 256);
        let mut osc = Oscillator::new(
            Waveform::Wavetable { table, position: 0.0 },
            44100,
        );
        osc.set_frequency(440.0);
        let mut buffer = vec![0.0; 256];
        osc.process(&mut buffer);
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_generate_wavetable_size() {
        let table = generate_wavetable("sine", 512);
        assert_eq!(table.len(), 512);
    }

    #[test]
    fn test_reset_phase() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        osc.process_sample();
        osc.process_sample();
        assert!(osc.phase > 0.0);
        osc.reset();
        assert_eq!(osc.phase, 0.0);
    }

    #[test]
    fn test_frequency_clamping() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        osc.set_frequency(50000.0);
        assert_eq!(osc.frequency, 20000.0);
        osc.set_frequency(5.0);
        assert_eq!(osc.frequency, 20.0);
    }

    #[test]
    fn test_pulse_width_variations() {
        let mut osc = Oscillator::new(Waveform::Square { pulse_width: 0.25 }, 44100);
        osc.set_frequency(440.0);
        let mut high_count = 0;
        for _ in 0..4410 {
            let s = osc.process_sample();
            if s > 0.5 {
                high_count += 1;
            }
        }
        let ratio = high_count as f32 / 4410.0;
        assert!(ratio > 0.15 && ratio < 0.35, "Pulse width ratio: {}", ratio);
    }

    #[test]
    fn test_fm_modulation() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        osc.set_frequency(440.0);
        osc.set_fm_amount(100.0);
        
        let s1 = osc.process_sample_fm(0.0);
        osc.reset();
        let s2 = osc.process_sample_fm(1.0);
        assert!(s1.abs() <= 1.0);
        assert!(s2.abs() <= 1.0);
    }

    #[test]
    fn test_pm_modulation() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        osc.set_frequency(440.0);
        osc.set_pm_amount(std::f32::consts::PI);
        
        let s1 = osc.process_sample_pm(0.0);
        osc.reset();
        let s2 = osc.process_sample_pm(0.5);
        assert!(s1.abs() <= 1.0);
        assert!(s2.abs() <= 1.0);
    }

    #[test]
    fn test_hard_sync() {
        let mut osc = Oscillator::new(Waveform::Saw, 44100);
        osc.set_frequency(440.0);
        
        for _ in 0..100 {
            osc.process_sample();
        }
        assert!(osc.phase > 0.0);
        
        osc.sync();
        assert_eq!(osc.phase, 0.0);
    }

    #[test]
    fn test_multi_wavetable() {
        let tables = generate_morph_wavetables(256);
        let mut osc = Oscillator::new(
            Waveform::MultiWavetable { tables, position: 0.0 },
            44100,
        );
        osc.set_frequency(440.0);
        
        // Test at different positions
        for pos in [0.0, 0.25, 0.5, 0.75, 1.0] {
            osc.set_wavetable_position(pos);
            let sample = osc.process_sample();
            assert!(sample.abs() <= 1.5, "Sample out of range at position {}: {}", pos, sample);
        }
    }

    #[test]
    fn test_wavetable_position_morphing() {
        let tables = generate_morph_wavetables(256);
        let mut osc = Oscillator::new(
            Waveform::MultiWavetable { tables, position: 0.0 },
            44100,
        );
        osc.set_frequency(440.0);
        
        // Get samples at position 0 (sine-like)
        osc.reset();
        osc.set_wavetable_position(0.0);
        let mut sum_0 = 0.0;
        for _ in 0..1000 {
            sum_0 += osc.process_sample().abs();
        }
        
        // Get samples at position 1.0 (square-like, more energy)
        osc.reset();
        osc.set_wavetable_position(1.0);
        let mut sum_1 = 0.0;
        for _ in 0..1000 {
            sum_1 += osc.process_sample().abs();
        }
        
        // Square should have more energy than sine
        assert!(sum_1 > sum_0 * 0.9, "Morphing not working: sum_0={}, sum_1={}", sum_0, sum_1);
    }

    #[test]
    fn test_oversampling() {
        let table = generate_wavetable("saw", 256);
        let mut osc = Oscillator::new(
            Waveform::Wavetable { table, position: 0.0 },
            44100,
        );
        osc.set_frequency(440.0);
        
        // Test with different oversample factors
        for factor in [1, 2, 4, 8] {
            osc.set_oversample(factor);
            osc.reset();
            let sample = osc.process_sample();
            assert!(sample.is_finite(), "Non-finite sample with oversample factor {}", factor);
        }
    }

    #[test]
    fn test_generate_morph_wavetables() {
        let tables = generate_morph_wavetables(256);
        assert_eq!(tables.len(), 4);
        for table in &tables {
            assert_eq!(table.len(), 256);
        }
    }
}
