//! Main synthesis engine

use super::context::ProcessContext;
use super::voice::VoiceManager;

/// Engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub sample_rate: u32,
    pub buffer_size: usize,
    pub max_voices: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            buffer_size: 256,
            max_voices: 8,
        }
    }
}

/// Main synthesis engine
pub struct Engine {
    config: EngineConfig,
    context: ProcessContext,
    voices: VoiceManager,
}

impl Engine {
    /// Create new engine with config
    pub fn new(config: EngineConfig) -> Self {
        let context = ProcessContext::new(config.sample_rate, config.buffer_size);
        let voices = VoiceManager::new(config.max_voices);

        Self {
            config,
            context,
            voices,
        }
    }

    /// Process audio into output buffer
    pub fn process(&mut self, output: &mut [f32]) {
        output.fill(0.0);
        // TODO: Process all voices and mix
    }

    /// Handle note on
    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.voices.note_on(note, velocity);
    }

    /// Handle note off
    pub fn note_off(&mut self, note: u8) {
        self.voices.note_off(note);
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_new() {
        let engine = Engine::new(EngineConfig::default());
        assert_eq!(engine.sample_rate(), 44100);
    }

    #[test]
    fn test_engine_process() {
        let mut engine = Engine::new(EngineConfig::default());
        let mut buffer = vec![1.0; 256];
        engine.process(&mut buffer);
        assert_eq!(buffer[0], 0.0); // Should be cleared
    }
}
