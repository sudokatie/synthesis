//! Delay effect

/// Simple delay line
pub struct Delay {
    buffer: Vec<f32>,
    write_pos: usize,
    delay_samples: usize,
    feedback: f32,
    mix: f32,
}

impl Delay {
    pub fn new(max_time: f32, sample_rate: u32) -> Self {
        let max_samples = (max_time * sample_rate as f32) as usize;
        Self {
            buffer: vec![0.0; max_samples],
            write_pos: 0,
            delay_samples: max_samples / 2,
            feedback: 0.5,
            mix: 0.5,
        }
    }

    pub fn set_time(&mut self, time: f32, sample_rate: u32) {
        let samples = (time * sample_rate as f32) as usize;
        self.delay_samples = samples.min(self.buffer.len() - 1);
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 0.95);
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        let read_pos = (self.write_pos + self.buffer.len() - self.delay_samples) % self.buffer.len();
        let delayed = self.buffer[read_pos];

        self.buffer[self.write_pos] = input + delayed * self.feedback;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();

        input * (1.0 - self.mix) + delayed * self.mix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_new() {
        let delay = Delay::new(1.0, 44100);
        assert_eq!(delay.buffer.len(), 44100);
    }
}
