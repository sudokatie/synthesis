//! Main synthesis engine with full integration

use super::context::ProcessContext;
use super::synth_voice::{SynthVoiceManager, VoiceParams, PlayMode, StealMode};
use crate::effects::{Chorus, Compressor, Delay, Limiter, SchroederReverb};
use crate::midi::{MidiMessage, MidiState};
use crate::preset::Preset;

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
    voices: SynthVoiceManager,
    midi_state: MidiState,
    // Effects
    delay: Delay,
    reverb: SchroederReverb,
    chorus: Chorus,
    limiter: Limiter,
    // Mix levels
    delay_mix: f32,
    reverb_mix: f32,
    chorus_mix: f32,
    master_volume: f32,
}

impl Engine {
    /// Create new engine with config
    pub fn new(config: EngineConfig) -> Self {
        let context = ProcessContext::new(config.sample_rate, config.buffer_size);
        let voices = SynthVoiceManager::new(config.max_voices, config.sample_rate);

        Self {
            delay: Delay::new(2.0, config.sample_rate),
            reverb: SchroederReverb::new(config.sample_rate),
            chorus: Chorus::new(config.sample_rate),
            limiter: Limiter::new(config.sample_rate),
            config,
            context,
            voices,
            midi_state: MidiState::new(),
            delay_mix: 0.0,
            reverb_mix: 0.0,
            chorus_mix: 0.0,
            master_volume: 0.8,
        }
    }

    /// Load a preset
    pub fn load_preset(&mut self, preset: &Preset) {
        self.voices.set_params(preset.to_params());

        // Configure effects from preset
        self.delay.set_time(preset.effects.delay_time);
        self.delay.set_feedback(preset.effects.delay_feedback);
        self.delay_mix = preset.effects.delay_mix;

        self.reverb.set_room_size(preset.effects.reverb_size);
        self.reverb_mix = preset.effects.reverb_mix;

        self.chorus.set_rate(preset.effects.chorus_rate);
        self.chorus.set_depth(preset.effects.chorus_depth);
        self.chorus_mix = preset.effects.chorus_mix;
    }

    /// Set voice parameters directly
    pub fn set_params(&mut self, params: VoiceParams) {
        self.voices.set_params(params);
    }

    /// Get current voice parameters
    pub fn params(&self) -> &VoiceParams {
        self.voices.params()
    }

    /// Set play mode (poly, mono, legato)
    pub fn set_play_mode(&mut self, mode: PlayMode) {
        self.voices.set_play_mode(mode);
    }

    /// Set voice stealing mode
    pub fn set_steal_mode(&mut self, mode: StealMode) {
        self.voices.set_steal_mode(mode);
    }

    /// Set master volume (0.0 to 1.0)
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Set delay effect parameters
    pub fn set_delay(&mut self, time: f32, feedback: f32, mix: f32) {
        self.delay.set_time(time);
        self.delay.set_feedback(feedback);
        self.delay_mix = mix.clamp(0.0, 1.0);
    }

    /// Set reverb effect parameters
    pub fn set_reverb(&mut self, room_size: f32, damping: f32, mix: f32) {
        self.reverb.set_room_size(room_size);
        self.reverb.set_damping(damping);
        self.reverb_mix = mix.clamp(0.0, 1.0);
    }

    /// Set chorus effect parameters
    pub fn set_chorus(&mut self, rate: f32, depth: f32, mix: f32) {
        self.chorus.set_rate(rate);
        self.chorus.set_depth(depth);
        self.chorus_mix = mix.clamp(0.0, 1.0);
    }

    /// Process raw MIDI bytes
    pub fn process_midi(&mut self, data: &[u8]) {
        use crate::midi::parse_midi;
        let msg = parse_midi(data);
        self.process_midi_message(msg);
    }

    /// Process a MIDI message
    pub fn process_midi_message(&mut self, msg: MidiMessage) {
        self.midi_state.process_message(msg);

        match msg {
            MidiMessage::NoteOn { note, velocity, .. } => {
                let vel = velocity as f32 / 127.0;
                self.note_on(note, vel);
            }
            MidiMessage::NoteOff { note, .. } => {
                self.note_off(note);
            }
            _ => {}
        }
    }

    /// Process audio into output buffer
    pub fn process(&mut self, output: &mut [f32]) {
        // Generate voices
        self.voices.process(output);

        // Apply effects
        for sample in output.iter_mut() {
            // Chorus
            if self.chorus_mix > 0.001 {
                let chorus_out = self.chorus.process_sample(*sample);
                *sample = *sample * (1.0 - self.chorus_mix) + chorus_out * self.chorus_mix;
            }

            // Delay
            if self.delay_mix > 0.001 {
                let delay_out = self.delay.process_sample(*sample);
                *sample = *sample * (1.0 - self.delay_mix) + delay_out * self.delay_mix;
            }

            // Reverb
            if self.reverb_mix > 0.001 {
                let reverb_out = self.reverb.process_sample(*sample);
                *sample = *sample * (1.0 - self.reverb_mix) + reverb_out * self.reverb_mix;
            }

            // Master volume
            *sample *= self.master_volume;

            // Limiter
            *sample = self.limiter.process_sample(*sample);
        }
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

    /// Get buffer size
    pub fn buffer_size(&self) -> usize {
        self.config.buffer_size
    }

    /// Get active voice count
    pub fn active_voices(&self) -> usize {
        self.voices.active_count()
    }

    /// Reset all state
    pub fn reset(&mut self) {
        self.delay.reset();
        self.reverb.reset();
        self.chorus.reset();
        self.limiter.reset();
        self.midi_state.reset();
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
        // Buffer should be modified (at least by limiter)
        assert!(buffer.iter().all(|&s| s <= 1.0));
    }

    #[test]
    fn test_engine_note_on_off() {
        let mut engine = Engine::new(EngineConfig::default());
        engine.note_on(60, 0.8);
        assert_eq!(engine.active_voices(), 1);

        engine.note_off(60);
        // Voice still active (in release)
        assert_eq!(engine.active_voices(), 1);
    }

    #[test]
    fn test_engine_polyphony() {
        let mut engine = Engine::new(EngineConfig::default());
        engine.note_on(60, 0.8);
        engine.note_on(64, 0.8);
        engine.note_on(67, 0.8);
        assert_eq!(engine.active_voices(), 3);
    }

    #[test]
    fn test_engine_produces_audio() {
        let mut engine = Engine::new(EngineConfig::default());
        engine.note_on(60, 0.8);

        let mut buffer = vec![0.0; 512];
        engine.process(&mut buffer);

        // Should produce some audio
        assert!(buffer.iter().any(|&s| s.abs() > 0.001));
    }

    #[test]
    fn test_engine_load_preset() {
        let mut engine = Engine::new(EngineConfig::default());
        let preset = Preset::default();
        engine.load_preset(&preset);
        assert_eq!(engine.params().osc_mix, preset.osc_mix);
    }

    #[test]
    fn test_engine_midi_note_on() {
        let mut engine = Engine::new(EngineConfig::default());
        engine.process_midi(&[0x90, 60, 100]); // Note on
        assert_eq!(engine.active_voices(), 1);
    }

    #[test]
    fn test_engine_effects() {
        let mut engine = Engine::new(EngineConfig::default());
        engine.set_delay(0.25, 0.5, 0.3);
        engine.set_reverb(0.8, 0.5, 0.3);
        engine.set_chorus(1.0, 0.5, 0.3);

        engine.note_on(60, 0.8);
        let mut buffer = vec![0.0; 256];
        engine.process(&mut buffer);

        // Should still produce output
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_engine_master_volume() {
        let mut engine = Engine::new(EngineConfig::default());
        engine.set_master_volume(0.5);
        engine.note_on(60, 1.0);

        let mut buffer = vec![0.0; 256];
        engine.process(&mut buffer);

        // Peak should be reduced
        let peak = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(peak < 0.6);
    }
}
