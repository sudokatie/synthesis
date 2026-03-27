//! Audio buffer utilities

/// Audio buffer for processing
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    data: Vec<f32>,
    channels: usize,
}

impl AudioBuffer {
    /// Create new buffer
    pub fn new(frames: usize, channels: usize) -> Self {
        Self {
            data: vec![0.0; frames * channels],
            channels,
        }
    }

    /// Get buffer length in frames
    pub fn frames(&self) -> usize {
        self.data.len() / self.channels
    }

    /// Get number of channels
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Clear buffer to zeros
    pub fn clear(&mut self) {
        self.data.fill(0.0);
    }

    /// Get slice of all samples (interleaved)
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    /// Get mutable slice
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data
    }

    /// Get mono channel (mix down if stereo)
    pub fn to_mono(&self) -> Vec<f32> {
        if self.channels == 1 {
            return self.data.clone();
        }

        let frames = self.frames();
        let mut mono = vec![0.0; frames];
        for i in 0..frames {
            let mut sum = 0.0;
            for ch in 0..self.channels {
                sum += self.data[i * self.channels + ch];
            }
            mono[i] = sum / self.channels as f32;
        }
        mono
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_new() {
        let buf = AudioBuffer::new(256, 2);
        assert_eq!(buf.frames(), 256);
        assert_eq!(buf.channels(), 2);
    }

    #[test]
    fn test_buffer_clear() {
        let mut buf = AudioBuffer::new(64, 1);
        buf.as_mut_slice()[0] = 1.0;
        buf.clear();
        assert_eq!(buf.as_slice()[0], 0.0);
    }

    #[test]
    fn test_to_mono() {
        let mut buf = AudioBuffer::new(2, 2);
        let data = buf.as_mut_slice();
        data[0] = 0.5; // L frame 0
        data[1] = 0.5; // R frame 0
        data[2] = 1.0; // L frame 1
        data[3] = 0.0; // R frame 1
        let mono = buf.to_mono();
        assert_eq!(mono[0], 0.5);
        assert_eq!(mono[1], 0.5);
    }
}
