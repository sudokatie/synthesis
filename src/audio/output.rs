//! Audio output via cpal

use crate::{Error, Result};

/// Audio configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub buffer_size: usize,
    pub channels: u16,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            buffer_size: 256,
            channels: 2,
        }
    }
}

/// Audio output stream
pub struct AudioOutput {
    config: AudioConfig,
    // Stream would be held here in real implementation
}

impl AudioOutput {
    /// Create new audio output with config
    pub fn new(config: AudioConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    /// Get buffer size
    pub fn buffer_size(&self) -> usize {
        self.config.buffer_size
    }

    /// List available devices
    pub fn list_devices() -> Result<Vec<String>> {
        use cpal::traits::{DeviceTrait, HostTrait};

        let host = cpal::default_host();
        let devices: Vec<String> = host
            .output_devices()
            .map_err(|e| Error::Audio(e.to_string()))?
            .filter_map(|d| d.name().ok())
            .collect();

        Ok(devices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, 44100);
        assert_eq!(config.channels, 2);
    }

    #[test]
    fn test_output_new() {
        let output = AudioOutput::new(AudioConfig::default());
        assert!(output.is_ok());
    }

    #[test]
    fn test_list_devices() {
        // This may fail in headless environments
        let _ = AudioOutput::list_devices();
    }
}
