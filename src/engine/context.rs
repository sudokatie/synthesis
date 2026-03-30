//! Processing context

/// Context passed to modules during processing
#[derive(Debug, Clone)]
pub struct ProcessContext {
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub tempo: f32,
}

impl ProcessContext {
    pub fn new(sample_rate: u32, buffer_size: u32) -> Self {
        Self {
            sample_rate,
            buffer_size,
            tempo: 120.0,
        }
    }

    /// Samples per millisecond
    pub fn samples_per_ms(&self) -> f32 {
        self.sample_rate as f32 / 1000.0
    }

    /// Time per sample in seconds
    pub fn sample_time(&self) -> f32 {
        1.0 / self.sample_rate as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_context_new() {
        let ctx = ProcessContext::new(44100, 256);
        assert_eq!(ctx.sample_rate, 44100);
        assert_eq!(ctx.buffer_size, 256);
    }

    #[test]
    fn test_samples_per_ms() {
        let ctx = ProcessContext::new(44100, 256);
        assert_relative_eq!(ctx.samples_per_ms(), 44.1, epsilon = 0.1);
    }

    #[test]
    fn test_sample_time() {
        let ctx = ProcessContext::new(44100, 256);
        assert_relative_eq!(ctx.sample_time(), 1.0 / 44100.0, epsilon = 0.0001);
    }
}
