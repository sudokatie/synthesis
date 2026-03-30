//! Step sequencer for modulation and note patterns

/// Sequencer playback direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SequencerDirection {
    Forward,
    Backward,
    PingPong,
    Random,
}

impl Default for SequencerDirection {
    fn default() -> Self {
        SequencerDirection::Forward
    }
}

/// Step sequencer for modulation
#[derive(Debug, Clone)]
pub struct Sequencer {
    steps: Vec<f32>,
    current_step: usize,
    direction: SequencerDirection,
    ping_pong_forward: bool,
    // Timing
    bpm: f32,
    division: f32,     // 1.0 = quarter note, 0.5 = eighth, etc.
    sample_rate: f32,
    samples_per_step: f32,
    sample_counter: f32,
    // Swing
    swing: f32,        // 0.0-1.0, affects even steps
    // Gate
    gate_length: f32,  // 0.0-1.0, portion of step that gate is high
    gate_active: bool,
    // Output
    current_value: f32,
    gate_value: f32,
}

impl Sequencer {
    /// Create new sequencer with given number of steps
    pub fn new(num_steps: usize, sample_rate: u32) -> Self {
        let mut seq = Self {
            steps: vec![0.0; num_steps.max(1)],
            current_step: 0,
            direction: SequencerDirection::Forward,
            ping_pong_forward: true,
            bpm: 120.0,
            division: 1.0,
            sample_rate: sample_rate as f32,
            samples_per_step: 0.0,
            sample_counter: 0.0,
            swing: 0.0,
            gate_length: 0.5,
            gate_active: true,
            current_value: 0.0,
            gate_value: 1.0,  // Start with gate high
        };
        seq.update_timing();
        seq
    }

    /// Create sequencer with initial values
    pub fn with_steps(steps: Vec<f32>, sample_rate: u32) -> Self {
        let mut seq = Self::new(steps.len(), sample_rate);
        seq.steps = steps;
        seq.current_value = seq.steps.first().copied().unwrap_or(0.0);
        seq.gate_value = 1.0;  // Gate starts high
        seq
    }

    fn update_timing(&mut self) {
        // Samples per beat = sample_rate * 60 / bpm
        // Samples per step = samples per beat * division
        let samples_per_beat = self.sample_rate * 60.0 / self.bpm;
        self.samples_per_step = samples_per_beat * self.division;
    }

    /// Set BPM
    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(20.0, 300.0);
        self.update_timing();
    }

    /// Set step division (1.0 = quarter, 0.5 = eighth, 0.25 = sixteenth)
    pub fn set_division(&mut self, division: f32) {
        self.division = division.clamp(0.0625, 4.0);
        self.update_timing();
    }

    /// Set playback direction
    pub fn set_direction(&mut self, direction: SequencerDirection) {
        self.direction = direction;
    }

    /// Set swing amount (0.0-1.0)
    pub fn set_swing(&mut self, swing: f32) {
        self.swing = swing.clamp(0.0, 1.0);
    }

    /// Set gate length (0.0-1.0)
    pub fn set_gate_length(&mut self, length: f32) {
        self.gate_length = length.clamp(0.0, 1.0);
    }

    /// Set step value
    pub fn set_step(&mut self, index: usize, value: f32) {
        if index < self.steps.len() {
            self.steps[index] = value.clamp(-1.0, 1.0);
        }
    }

    /// Get step value
    pub fn get_step(&self, index: usize) -> f32 {
        self.steps.get(index).copied().unwrap_or(0.0)
    }

    /// Get number of steps
    pub fn num_steps(&self) -> usize {
        self.steps.len()
    }

    /// Get current step index
    pub fn current_step(&self) -> usize {
        self.current_step
    }

    /// Reset to beginning
    pub fn reset(&mut self) {
        self.current_step = 0;
        self.sample_counter = 0.0;
        self.ping_pong_forward = true;
        self.current_value = self.steps.first().copied().unwrap_or(0.0);
        self.gate_active = true;
        self.gate_value = 1.0;
    }

    /// Advance to next step
    fn advance_step(&mut self) {
        let len = self.steps.len();
        if len == 0 {
            return;
        }

        match self.direction {
            SequencerDirection::Forward => {
                self.current_step = (self.current_step + 1) % len;
            }
            SequencerDirection::Backward => {
                if self.current_step == 0 {
                    self.current_step = len - 1;
                } else {
                    self.current_step -= 1;
                }
            }
            SequencerDirection::PingPong => {
                if self.ping_pong_forward {
                    if self.current_step >= len - 1 {
                        self.ping_pong_forward = false;
                        if self.current_step > 0 {
                            self.current_step -= 1;
                        }
                    } else {
                        self.current_step += 1;
                    }
                } else {
                    if self.current_step == 0 {
                        self.ping_pong_forward = true;
                        if len > 1 {
                            self.current_step = 1;
                        }
                    } else {
                        self.current_step -= 1;
                    }
                }
            }
            SequencerDirection::Random => {
                self.current_step = (rand::random::<f32>() * len as f32) as usize % len;
            }
        }

        self.current_value = self.steps[self.current_step];
        self.gate_active = true;
        self.gate_value = 1.0;
    }

    /// Process a single sample, returns (value, gate)
    pub fn process_sample(&mut self) -> (f32, f32) {
        // Calculate swing-adjusted step length
        let is_even_step = self.current_step % 2 == 0;
        let swing_factor = if is_even_step {
            1.0 + self.swing * 0.5
        } else {
            1.0 - self.swing * 0.5
        };
        let adjusted_step_length = self.samples_per_step * swing_factor;

        // Update gate
        let gate_samples = adjusted_step_length * self.gate_length;
        if self.sample_counter >= gate_samples {
            self.gate_active = false;
            self.gate_value = 0.0;
        }

        // Advance counter
        self.sample_counter += 1.0;

        // Check for step advance
        if self.sample_counter >= adjusted_step_length {
            self.sample_counter -= adjusted_step_length;
            self.advance_step();
        }

        (self.current_value, self.gate_value)
    }

    /// Get current value (without advancing)
    pub fn value(&self) -> f32 {
        self.current_value
    }

    /// Get current gate value (without advancing)
    pub fn gate(&self) -> f32 {
        self.gate_value
    }

    /// Process buffer, returns value output
    pub fn process(&mut self, output: &mut [f32]) {
        for sample in output.iter_mut() {
            let (value, _gate) = self.process_sample();
            *sample = value;
        }
    }

    /// Process buffer with separate gate output
    pub fn process_with_gate(&mut self, value_out: &mut [f32], gate_out: &mut [f32]) {
        for (v, g) in value_out.iter_mut().zip(gate_out.iter_mut()) {
            let (value, gate) = self.process_sample();
            *v = value;
            *g = gate;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequencer_new() {
        let seq = Sequencer::new(8, 44100);
        assert_eq!(seq.num_steps(), 8);
        assert_eq!(seq.current_step(), 0);
    }

    #[test]
    fn test_sequencer_with_steps() {
        let steps = vec![0.0, 0.5, 1.0, 0.5];
        let seq = Sequencer::with_steps(steps, 44100);
        assert_eq!(seq.num_steps(), 4);
        assert_eq!(seq.get_step(2), 1.0);
    }

    #[test]
    fn test_sequencer_set_step() {
        let mut seq = Sequencer::new(4, 44100);
        seq.set_step(1, 0.75);
        assert_eq!(seq.get_step(1), 0.75);
    }

    #[test]
    fn test_sequencer_forward() {
        let mut seq = Sequencer::with_steps(vec![0.0, 0.25, 0.5, 0.75], 44100);
        seq.set_bpm(120.0);
        seq.set_division(1.0);
        seq.set_direction(SequencerDirection::Forward);
        
        // Process enough samples to advance several steps
        let samples_per_beat = 44100.0 * 60.0 / 120.0; // ~22050
        
        assert_eq!(seq.current_step(), 0);
        
        // Process one step worth of samples
        for _ in 0..(samples_per_beat as usize + 1) {
            seq.process_sample();
        }
        
        assert_eq!(seq.current_step(), 1);
    }

    #[test]
    fn test_sequencer_backward() {
        let mut seq = Sequencer::with_steps(vec![0.0, 0.25, 0.5, 0.75], 44100);
        seq.set_direction(SequencerDirection::Backward);
        seq.reset();
        
        // Manually trigger advances
        seq.advance_step();
        assert_eq!(seq.current_step(), 3);
        seq.advance_step();
        assert_eq!(seq.current_step(), 2);
    }

    #[test]
    fn test_sequencer_ping_pong() {
        let mut seq = Sequencer::with_steps(vec![0.0, 0.25, 0.5, 0.75], 44100);
        seq.set_direction(SequencerDirection::PingPong);
        seq.reset();
        
        // Forward: 0 -> 1 -> 2 -> 3 -> 2 -> 1 -> 0 -> 1...
        let expected = [0, 1, 2, 3, 2, 1, 0, 1, 2];
        for &exp in &expected {
            assert_eq!(seq.current_step(), exp, "Expected step {}", exp);
            seq.advance_step();
        }
    }

    #[test]
    fn test_sequencer_gate() {
        let mut seq = Sequencer::with_steps(vec![1.0, 1.0], 44100);
        seq.set_bpm(120.0);
        seq.set_division(1.0);
        seq.set_gate_length(0.5);
        
        let (_, gate) = seq.process_sample();
        assert_eq!(gate, 1.0); // Gate starts high
    }

    #[test]
    fn test_sequencer_reset() {
        let mut seq = Sequencer::with_steps(vec![0.0, 0.5, 1.0], 44100);
        seq.advance_step();
        seq.advance_step();
        assert_eq!(seq.current_step(), 2);
        
        seq.reset();
        assert_eq!(seq.current_step(), 0);
    }

    #[test]
    fn test_sequencer_bpm() {
        let mut seq = Sequencer::new(4, 44100);
        seq.set_bpm(60.0);
        
        // At 60 BPM, 1 beat = 1 second = 44100 samples
        // With division 1.0 (quarter note), each step = 44100 samples
        assert!((seq.samples_per_step - 44100.0).abs() < 1.0);
    }

    #[test]
    fn test_sequencer_division() {
        let mut seq = Sequencer::new(4, 44100);
        seq.set_bpm(60.0);
        seq.set_division(0.5); // Eighth notes
        
        // Each step = 22050 samples
        assert!((seq.samples_per_step - 22050.0).abs() < 1.0);
    }

    #[test]
    fn test_sequencer_swing() {
        let mut seq = Sequencer::new(4, 44100);
        seq.set_swing(0.5);
        assert_eq!(seq.swing, 0.5);
    }

    #[test]
    fn test_sequencer_process() {
        let mut seq = Sequencer::with_steps(vec![1.0, -1.0], 44100);
        let mut buffer = vec![0.0; 256];
        seq.process(&mut buffer);
        
        // Should have non-zero values
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_sequencer_random() {
        let mut seq = Sequencer::with_steps(vec![0.0, 0.25, 0.5, 0.75], 44100);
        seq.set_direction(SequencerDirection::Random);
        
        // Advance several times, should get different steps
        let mut steps_seen = std::collections::HashSet::new();
        for _ in 0..100 {
            seq.advance_step();
            steps_seen.insert(seq.current_step());
        }
        
        // Should have seen multiple different steps
        assert!(steps_seen.len() > 1);
    }
}
