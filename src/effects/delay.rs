//! Delay effects - mono and stereo delay lines

/// Simple delay line buffer
/// 
/// A circular buffer for audio delay. Write samples with `write()`,
/// then read delayed samples with `read(delay)`.
#[derive(Debug, Clone)]
pub struct DelayLine {
    buffer: Vec<f32>,
    write_pos: usize,
}

impl DelayLine {
    pub fn new(max_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; max_samples.max(1)],
            write_pos: 0,
        }
    }

    /// Write a sample and advance the write position
    pub fn write(&mut self, sample: f32) {
        self.buffer[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
    }

    /// Read from `delay_samples` behind the write head
    /// 
    /// read(0) returns the most recently written sample.
    /// read(1) returns the sample written before that, etc.
    pub fn read(&self, delay_samples: usize) -> f32 {
        if self.buffer.is_empty() {
            return 0.0;
        }
        let delay = delay_samples.min(self.buffer.len() - 1);
        // write_pos points to where we'll write NEXT
        // So the most recent sample is at write_pos - 1
        // And delay samples back is at write_pos - 1 - delay
        let read_pos = (self.write_pos + self.buffer.len() - 1 - delay) % self.buffer.len();
        self.buffer[read_pos]
    }

    /// Read with linear interpolation for fractional delays
    pub fn read_interp(&self, delay_samples: f32) -> f32 {
        if self.buffer.is_empty() {
            return 0.0;
        }
        let delay = delay_samples.min(self.buffer.len() as f32 - 1.0).max(0.0);
        let int_delay = delay.floor() as usize;
        let frac = delay - int_delay as f32;

        let s1 = self.read(int_delay);
        let s2 = self.read(int_delay + 1);

        s1 + frac * (s2 - s1)
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

/// Mono delay with feedback
#[derive(Debug, Clone)]
pub struct Delay {
    buffer: DelayLine,
    delay_samples: usize,
    feedback: f32,
    mix: f32,
    sample_rate: f32,
}

impl Delay {
    pub fn new(max_time: f32, sample_rate: u32) -> Self {
        let max_samples = (max_time * sample_rate as f32) as usize;
        Self {
            buffer: DelayLine::new(max_samples),
            delay_samples: max_samples / 2,
            feedback: 0.5,
            mix: 0.5,
            sample_rate: sample_rate as f32,
        }
    }

    pub fn set_time(&mut self, time: f32) {
        let samples = (time * self.sample_rate) as usize;
        self.delay_samples = samples.min(self.buffer.len().saturating_sub(1));
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 0.95);
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        let delayed = self.buffer.read(self.delay_samples);
        self.buffer.write(input + delayed * self.feedback);
        input * (1.0 - self.mix) + delayed * self.mix
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }

    pub fn reset(&mut self) {
        self.buffer.reset();
    }
}

/// Stereo delay with ping-pong mode
#[derive(Debug, Clone)]
pub struct StereoDelay {
    left: DelayLine,
    right: DelayLine,
    left_time: f32,
    right_time: f32,
    feedback: f32,
    cross_feedback: f32, // For ping-pong
    mix: f32,
    sample_rate: f32,
}

impl StereoDelay {
    pub fn new(max_time: f32, sample_rate: u32) -> Self {
        let max_samples = (max_time * sample_rate as f32) as usize;
        Self {
            left: DelayLine::new(max_samples),
            right: DelayLine::new(max_samples),
            left_time: 0.25,
            right_time: 0.375,
            feedback: 0.5,
            cross_feedback: 0.0,
            mix: 0.5,
            sample_rate: sample_rate as f32,
        }
    }

    pub fn set_left_time(&mut self, time: f32) {
        self.left_time = time.max(0.0);
    }

    pub fn set_right_time(&mut self, time: f32) {
        self.right_time = time.max(0.0);
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 0.95);
    }

    pub fn set_cross_feedback(&mut self, cross: f32) {
        self.cross_feedback = cross.clamp(0.0, 0.95);
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    /// Enable ping-pong mode (cross-channel feedback)
    pub fn set_ping_pong(&mut self, enabled: bool) {
        if enabled {
            self.cross_feedback = self.feedback;
            self.feedback = 0.0;
        } else {
            self.feedback = self.cross_feedback;
            self.cross_feedback = 0.0;
        }
    }

    pub fn process_sample(&mut self, left_in: f32, right_in: f32) -> (f32, f32) {
        let left_samples = (self.left_time * self.sample_rate) as usize;
        let right_samples = (self.right_time * self.sample_rate) as usize;

        let left_delayed = self.left.read(left_samples);
        let right_delayed = self.right.read(right_samples);

        // Normal feedback + cross feedback (ping-pong)
        self.left.write(left_in + left_delayed * self.feedback + right_delayed * self.cross_feedback);
        self.right.write(right_in + right_delayed * self.feedback + left_delayed * self.cross_feedback);

        let left_out = left_in * (1.0 - self.mix) + left_delayed * self.mix;
        let right_out = right_in * (1.0 - self.mix) + right_delayed * self.mix;

        (left_out, right_out)
    }

    pub fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_line_new() {
        let dl = DelayLine::new(1000);
        assert_eq!(dl.buffer.len(), 1000);
    }

    #[test]
    fn test_delay_line_write_read() {
        let mut dl = DelayLine::new(100);
        dl.write(0.5);
        assert_eq!(dl.read(0), 0.5);
    }

    #[test]
    fn test_delay_line_delayed_read() {
        let mut dl = DelayLine::new(100);
        dl.write(0.5);
        dl.write(0.7);
        dl.write(0.9);
        assert_eq!(dl.read(2), 0.5);
        assert_eq!(dl.read(1), 0.7);
        assert_eq!(dl.read(0), 0.9);
    }

    #[test]
    fn test_delay_line_interp() {
        let mut dl = DelayLine::new(100);
        dl.write(0.0);
        dl.write(1.0);
        // Interpolate between samples
        let interp = dl.read_interp(0.5);
        assert!((interp - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_delay_new() {
        let delay = Delay::new(1.0, 44100);
        assert_eq!(delay.buffer.buffer.len(), 44100);
    }

    #[test]
    fn test_delay_process() {
        // Test with very simple delay
        let mut delay = Delay::new(0.1, 1000);  // 100ms max at 1kHz = 100 samples
        delay.set_time(0.003);  // 3ms = 3 samples delay
        delay.set_feedback(0.0);
        delay.set_mix(1.0);  // 100% wet

        // Track what comes out
        let mut outputs = Vec::new();
        
        // Send impulse then zeros
        outputs.push(delay.process_sample(1.0));  // Impulse in
        for _ in 0..10 {
            outputs.push(delay.process_sample(0.0));
        }
        
        // With 3 sample delay, the impulse should appear at index 3
        // (index 0 is first output, which reads from empty buffer)
        let echo_found = outputs.iter().skip(1).any(|&x| x > 0.5);
        assert!(echo_found, "No echo found in outputs: {:?}", outputs);
    }

    #[test]
    fn test_delay_feedback() {
        let mut delay = Delay::new(1.0, 44100);
        delay.set_time(0.001);
        delay.set_feedback(0.5);
        delay.set_mix(1.0);

        delay.process_sample(1.0);
        
        // With feedback, echoes should decay
        let mut prev = 1.0;
        for _ in 0..1000 {
            let out = delay.process_sample(0.0);
            if out > 0.01 {
                assert!(out <= prev + 0.1);
                prev = out;
            }
        }
    }

    #[test]
    fn test_stereo_delay_new() {
        let delay = StereoDelay::new(1.0, 44100);
        assert_eq!(delay.left.buffer.len(), 44100);
        assert_eq!(delay.right.buffer.len(), 44100);
    }

    #[test]
    fn test_stereo_delay_process() {
        let mut delay = StereoDelay::new(1.0, 44100);
        delay.set_left_time(0.01);
        delay.set_right_time(0.02);
        delay.set_mix(0.5);

        let (l, r) = delay.process_sample(1.0, 1.0);
        assert!(l.is_finite());
        assert!(r.is_finite());
    }

    #[test]
    fn test_stereo_delay_ping_pong() {
        let mut delay = StereoDelay::new(1.0, 44100);
        delay.set_ping_pong(true);
        assert!(delay.cross_feedback > 0.0);
        assert_eq!(delay.feedback, 0.0);
    }

    #[test]
    fn test_delay_reset() {
        let mut delay = Delay::new(1.0, 44100);
        delay.process_sample(1.0);
        delay.reset();
        assert!(delay.buffer.buffer.iter().all(|&s| s == 0.0));
    }
}
