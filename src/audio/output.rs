//! Audio output via cpal

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::sync::Arc;
use parking_lot::Mutex;

use crate::{Error, Result};

/// Audio configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub buffer_size: u32,
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

/// Audio callback type
pub type AudioCallback = Arc<Mutex<dyn FnMut(&mut [f32]) + Send>>;

/// Audio output stream manager
pub struct AudioOutput {
    config: AudioConfig,
    device: Option<Device>,
    stream: Option<Stream>,
    stream_config: Option<StreamConfig>,
}

impl AudioOutput {
    /// Create new audio output with config
    pub fn new(config: AudioConfig) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| Error::Audio("No output device found".to_string()))?;

        Ok(Self {
            config,
            device: Some(device),
            stream: None,
            stream_config: None,
        })
    }

    /// Create with specific device name
    pub fn with_device(config: AudioConfig, device_name: &str) -> Result<Self> {
        let host = cpal::default_host();
        let name_lower = device_name.to_lowercase();
        
        let device = host
            .output_devices()
            .map_err(|e| Error::Audio(e.to_string()))?
            .find(|d| {
                d.name()
                    .map(|n| n.to_lowercase().contains(&name_lower))
                    .unwrap_or(false)
            })
            .ok_or_else(|| Error::Audio(format!("Device not found: {}", device_name)))?;

        Ok(Self {
            config,
            device: Some(device),
            stream: None,
            stream_config: None,
        })
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.stream_config
            .as_ref()
            .map(|c| c.sample_rate.0)
            .unwrap_or(self.config.sample_rate)
    }

    /// Get buffer size
    pub fn buffer_size(&self) -> u32 {
        self.config.buffer_size
    }

    /// Get channel count
    pub fn channels(&self) -> u16 {
        self.stream_config
            .as_ref()
            .map(|c| c.channels)
            .unwrap_or(self.config.channels)
    }

    /// Start the audio stream with a callback
    pub fn start<F>(&mut self, callback: F) -> Result<()>
    where
        F: FnMut(&mut [f32]) + Send + 'static,
    {
        let device = self
            .device
            .as_ref()
            .ok_or_else(|| Error::Audio("No device".to_string()))?;

        let supported_config = device
            .default_output_config()
            .map_err(|e| Error::Audio(e.to_string()))?;

        let sample_format = supported_config.sample_format();
        let stream_config: StreamConfig = supported_config.into();
        
        let channels = stream_config.channels as usize;
        let callback = Arc::new(Mutex::new(callback));
        let callback_clone = Arc::clone(&callback);

        let stream = match sample_format {
            SampleFormat::F32 => {
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        // Process mono then duplicate to stereo
                        let mono_len = data.len() / channels;
                        let mut mono_buffer = vec![0.0f32; mono_len];
                        
                        {
                            let mut cb = callback_clone.lock();
                            cb(&mut mono_buffer);
                        }
                        
                        // Copy to interleaved stereo
                        for (i, &sample) in mono_buffer.iter().enumerate() {
                            for ch in 0..channels {
                                data[i * channels + ch] = sample;
                            }
                        }
                    },
                    |err| eprintln!("Audio error: {}", err),
                    None,
                )
            }
            SampleFormat::I16 => {
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        let mono_len = data.len() / channels;
                        let mut mono_buffer = vec![0.0f32; mono_len];
                        
                        {
                            let mut cb = callback_clone.lock();
                            cb(&mut mono_buffer);
                        }
                        
                        for (i, &sample) in mono_buffer.iter().enumerate() {
                            let int_sample = (sample * i16::MAX as f32) as i16;
                            for ch in 0..channels {
                                data[i * channels + ch] = int_sample;
                            }
                        }
                    },
                    |err| eprintln!("Audio error: {}", err),
                    None,
                )
            }
            SampleFormat::U16 => {
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        let mono_len = data.len() / channels;
                        let mut mono_buffer = vec![0.0f32; mono_len];
                        
                        {
                            let mut cb = callback_clone.lock();
                            cb(&mut mono_buffer);
                        }
                        
                        for (i, &sample) in mono_buffer.iter().enumerate() {
                            let int_sample = ((sample + 1.0) * 0.5 * u16::MAX as f32) as u16;
                            for ch in 0..channels {
                                data[i * channels + ch] = int_sample;
                            }
                        }
                    },
                    |err| eprintln!("Audio error: {}", err),
                    None,
                )
            }
            _ => {
                return Err(Error::Audio(format!("Unsupported format: {:?}", sample_format)));
            }
        }
        .map_err(|e| Error::Audio(e.to_string()))?;

        stream.play().map_err(|e| Error::Audio(e.to_string()))?;
        
        self.stream = Some(stream);
        self.stream_config = Some(stream_config);

        Ok(())
    }

    /// Stop the audio stream
    pub fn stop(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
        }
    }

    /// Check if stream is running
    pub fn is_running(&self) -> bool {
        self.stream.is_some()
    }

    /// List available devices
    pub fn list_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();
        let devices: Vec<String> = host
            .output_devices()
            .map_err(|e| Error::Audio(e.to_string()))?
            .filter_map(|d| d.name().ok())
            .collect();

        Ok(devices)
    }

    /// Get default device name
    pub fn default_device_name() -> Result<String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| Error::Audio("No default device".to_string()))?;
        
        device.name().map_err(|e| Error::Audio(e.to_string()))
    }

    /// Get device sample rate
    pub fn device_sample_rate(&self) -> Result<u32> {
        let device = self
            .device
            .as_ref()
            .ok_or_else(|| Error::Audio("No device".to_string()))?;

        let config = device
            .default_output_config()
            .map_err(|e| Error::Audio(e.to_string()))?;

        Ok(config.sample_rate().0)
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        self.stop();
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
        // May fail on headless systems
        let _ = AudioOutput::new(AudioConfig::default());
    }

    #[test]
    fn test_list_devices() {
        // Should not panic
        let _ = AudioOutput::list_devices();
    }

    #[test]
    fn test_default_device_name() {
        let _ = AudioOutput::default_device_name();
    }

    #[test]
    fn test_output_not_running() {
        if let Ok(output) = AudioOutput::new(AudioConfig::default()) {
            assert!(!output.is_running());
        }
    }

    #[test]
    fn test_output_sample_rate() {
        if let Ok(output) = AudioOutput::new(AudioConfig::default()) {
            let rate = output.sample_rate();
            assert!(rate > 0);
        }
    }

    #[test]
    fn test_output_device_sample_rate() {
        if let Ok(output) = AudioOutput::new(AudioConfig::default()) {
            if let Ok(rate) = output.device_sample_rate() {
                assert!(rate >= 44100);
            }
        }
    }
}
