//! Low Frequency Oscillator

use crate::modules::Waveform;

/// LFO polarity
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Polarity {
    Unipolar, // 0.0 to 1.0
    Bipolar,  // -1.0 to 1.0
}

/// LFO sync mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LfoSync {
    /// Free running, not synced to anything
    Free,
    /// Reset phase on each note trigger
    KeySync,
    /// Sync to BPM with division (e.g., 1.0 = quarter note, 0.5 = eighth note)
    BpmSync { division: f32 },
}

impl Default for LfoSync {
    fn default() -> Self {
        LfoSync::Free
    }
}

impl LfoSync {
    /// Calculate LFO frequency from BPM and division
    pub fn frequency_from_bpm(bpm: f32, division: f32) -> f32 {
        // division 1.0 = quarter note = 1 beat
        // frequency = bpm / 60 / division
        (bpm / 60.0) / division
    }
}

/// Low Frequency Oscillator
#[derive(Debug, Clone)]
pub struct Lfo {
    waveform: Waveform,
    frequency: f32,
    phase: f32,
    polarity: Polarity,
    sync: LfoSync,
    sample_rate: f32,
    /// Current BPM for tempo sync
    bpm: f32,
}

impl Lfo {
    pub fn new(waveform: Waveform, frequency: f32, sample_rate: u32) -> Self {
        Self {
            waveform,
            frequency: frequency.clamp(0.01, 100.0),
            phase: 0.0,
            polarity: Polarity::Bipolar,
            sync: LfoSync::Free,
            sample_rate: sample_rate as f32,
            bpm: 120.0,
        }
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.frequency = freq.clamp(0.01, 100.0);
    }

    pub fn set_polarity(&mut self, polarity: Polarity) {
        self.polarity = polarity;
    }
    
    /// Set sync mode
    pub fn set_sync(&mut self, sync: LfoSync) {
        self.sync = sync;
    }
    
    /// Get current sync mode
    pub fn sync(&self) -> LfoSync {
        self.sync
    }
    
    /// Set BPM for tempo sync
    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(20.0, 300.0);
    }
    
    /// Get effective frequency (considering sync mode)
    pub fn effective_frequency(&self) -> f32 {
        match self.sync {
            LfoSync::Free => self.frequency,
            LfoSync::KeySync => self.frequency,
            LfoSync::BpmSync { division } => LfoSync::frequency_from_bpm(self.bpm, division),
        }
    }
    
    /// Called on note trigger - resets phase if KeySync mode
    pub fn note_on(&mut self) {
        if self.sync == LfoSync::KeySync {
            self.reset();
        }
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    pub fn value(&self) -> f32 {
        self.generate_sample(self.phase)
    }

    fn generate_sample(&self, phase: f32) -> f32 {
        let raw = match &self.waveform {
            Waveform::Sine => (phase * std::f32::consts::TAU).sin(),
            Waveform::Saw => 2.0 * phase - 1.0,
            Waveform::Square { pulse_width } => {
                if phase < *pulse_width {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Triangle => {
                if phase < 0.5 {
                    4.0 * phase - 1.0
                } else {
                    3.0 - 4.0 * phase
                }
            }
            Waveform::Noise => rand::random::<f32>() * 2.0 - 1.0,
            Waveform::Wavetable { table, .. } => {
                if table.is_empty() {
                    0.0
                } else {
                    let index = (phase * table.len() as f32) as usize % table.len();
                    table[index]
                }
            }
            Waveform::MultiWavetable { tables, position } => {
                if tables.is_empty() {
                    0.0
                } else if tables.len() == 1 {
                    let table = &tables[0];
                    if table.is_empty() { 0.0 } else {
                        let index = (phase * table.len() as f32) as usize % table.len();
                        table[index]
                    }
                } else {
                    // Blend between tables based on position
                    let float_index = position * (tables.len() - 1) as f32;
                    let index_a = (float_index.floor() as usize).min(tables.len() - 1);
                    let index_b = (index_a + 1).min(tables.len() - 1);
                    let blend = float_index.fract();
                    
                    let table_a = &tables[index_a];
                    let table_b = &tables[index_b];
                    
                    let sample_a = if table_a.is_empty() { 0.0 } else {
                        let idx = (phase * table_a.len() as f32) as usize % table_a.len();
                        table_a[idx]
                    };
                    let sample_b = if table_b.is_empty() { 0.0 } else {
                        let idx = (phase * table_b.len() as f32) as usize % table_b.len();
                        table_b[idx]
                    };
                    
                    sample_a * (1.0 - blend) + sample_b * blend
                }
            }
        };

        match self.polarity {
            Polarity::Bipolar => raw,
            Polarity::Unipolar => (raw + 1.0) * 0.5,
        }
    }

    pub fn process_sample(&mut self) -> f32 {
        let sample = self.generate_sample(self.phase);

        let freq = self.effective_frequency();
        self.phase += freq / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sample
    }

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
    fn test_lfo_new() {
        let lfo = Lfo::new(Waveform::Sine, 1.0, 44100);
        assert_eq!(lfo.frequency, 1.0);
    }

    #[test]
    fn test_lfo_bipolar_range() {
        let mut lfo = Lfo::new(Waveform::Sine, 10.0, 44100);
        lfo.set_polarity(Polarity::Bipolar);
        for _ in 0..4410 {
            let s = lfo.process_sample();
            assert!(s >= -1.0 && s <= 1.0);
        }
    }

    #[test]
    fn test_lfo_unipolar_range() {
        let mut lfo = Lfo::new(Waveform::Sine, 10.0, 44100);
        lfo.set_polarity(Polarity::Unipolar);
        for _ in 0..4410 {
            let s = lfo.process_sample();
            assert!(s >= 0.0 && s <= 1.0);
        }
    }

    #[test]
    fn test_lfo_reset() {
        let mut lfo = Lfo::new(Waveform::Sine, 1.0, 44100);
        lfo.process_sample();
        lfo.reset();
        assert_eq!(lfo.phase, 0.0);
    }

    #[test]
    fn test_lfo_frequency_range() {
        let mut lfo = Lfo::new(Waveform::Sine, 1.0, 44100);
        lfo.set_frequency(200.0);
        assert_eq!(lfo.frequency, 100.0); // Clamped to max
        lfo.set_frequency(0.001);
        assert_eq!(lfo.frequency, 0.01); // Clamped to min
    }

    #[test]
    fn test_lfo_triangle() {
        let mut lfo = Lfo::new(Waveform::Triangle, 10.0, 44100);
        for _ in 0..4410 {
            let s = lfo.process_sample();
            assert!(s >= -1.0 && s <= 1.0);
        }
    }

    #[test]
    fn test_lfo_saw() {
        let mut lfo = Lfo::new(Waveform::Saw, 10.0, 44100);
        for _ in 0..4410 {
            let s = lfo.process_sample();
            assert!(s >= -1.0 && s <= 1.0);
        }
    }

    #[test]
    fn test_lfo_square() {
        let mut lfo = Lfo::new(Waveform::Square { pulse_width: 0.5 }, 10.0, 44100);
        for _ in 0..4410 {
            let s = lfo.process_sample();
            assert!(s >= -1.0 && s <= 1.0);
        }
    }

    #[test]
    fn test_lfo_process_buffer() {
        let mut lfo = Lfo::new(Waveform::Sine, 10.0, 44100);
        let mut buffer = vec![0.0; 256];
        lfo.process(&mut buffer);
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_lfo_value() {
        let lfo = Lfo::new(Waveform::Sine, 1.0, 44100);
        let v = lfo.value();
        assert!(v >= -1.0 && v <= 1.0);
    }
    
    #[test]
    fn test_lfo_sync_default() {
        let lfo = Lfo::new(Waveform::Sine, 1.0, 44100);
        assert_eq!(lfo.sync(), LfoSync::Free);
    }
    
    #[test]
    fn test_lfo_sync_free() {
        let mut lfo = Lfo::new(Waveform::Sine, 5.0, 44100);
        lfo.set_sync(LfoSync::Free);
        assert_eq!(lfo.effective_frequency(), 5.0);
    }
    
    #[test]
    fn test_lfo_sync_key() {
        let mut lfo = Lfo::new(Waveform::Sine, 5.0, 44100);
        lfo.set_sync(LfoSync::KeySync);
        
        // Process some samples
        for _ in 0..1000 {
            lfo.process_sample();
        }
        assert!(lfo.phase > 0.0);
        
        // Trigger note - should reset phase
        lfo.note_on();
        assert_eq!(lfo.phase, 0.0);
    }
    
    #[test]
    fn test_lfo_sync_bpm() {
        let mut lfo = Lfo::new(Waveform::Sine, 5.0, 44100);
        lfo.set_bpm(120.0);
        lfo.set_sync(LfoSync::BpmSync { division: 1.0 }); // Quarter note
        
        // At 120 BPM, quarter note = 2 Hz
        let freq = lfo.effective_frequency();
        assert!((freq - 2.0).abs() < 0.001);
    }
    
    #[test]
    fn test_lfo_sync_bpm_eighth() {
        let mut lfo = Lfo::new(Waveform::Sine, 5.0, 44100);
        lfo.set_bpm(120.0);
        lfo.set_sync(LfoSync::BpmSync { division: 0.5 }); // Eighth note
        
        // At 120 BPM, eighth note = 4 Hz
        let freq = lfo.effective_frequency();
        assert!((freq - 4.0).abs() < 0.001);
    }
    
    #[test]
    fn test_lfo_frequency_from_bpm() {
        // 120 BPM, quarter note division
        let freq = LfoSync::frequency_from_bpm(120.0, 1.0);
        assert!((freq - 2.0).abs() < 0.001);
        
        // 120 BPM, whole note division
        let freq = LfoSync::frequency_from_bpm(120.0, 4.0);
        assert!((freq - 0.5).abs() < 0.001);
    }
}
