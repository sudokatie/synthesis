//! Main synthesis engine with full integration

use super::context::ProcessContext;
use super::module::ModuleGraph;
use super::synth_voice::{PlayMode, StealMode, SynthVoiceManager, VoiceParams};
use crate::audio::{AudioConfig, AudioOutput};
use crate::effects::{Chorus, Compressor, Delay, Distortion, Limiter, SchroederReverb};
use crate::midi::{list_midi_inputs, MidiInputManager, MidiMessage, MidiState};
use crate::preset::Preset;
use crate::Result;

use std::sync::Arc;
use parking_lot::Mutex;

/// Engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub sample_rate: u32,
    pub buffer_size: u32,
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
    midi_input: Option<MidiInputManager>,
    audio_output: Option<AudioOutput>,
    // Effects
    delay: Delay,
    reverb: SchroederReverb,
    chorus: Chorus,
    distortion: Distortion,
    compressor: Compressor,
    limiter: Limiter,
    // Mix levels
    delay_mix: f32,
    reverb_mix: f32,
    chorus_mix: f32,
    distortion_mix: f32,
    master_volume: f32,
    // Global state
    bpm: f32,
    // Module graph (optional modular routing)
    module_graph: Option<ModuleGraph>,
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
            distortion: Distortion::new(),
            compressor: Compressor::new(config.sample_rate),
            limiter: Limiter::new(config.sample_rate),
            config,
            context,
            voices,
            midi_state: MidiState::new(),
            midi_input: None,
            audio_output: None,
            delay_mix: 0.0,
            reverb_mix: 0.0,
            chorus_mix: 0.0,
            distortion_mix: 0.0,
            master_volume: 0.8,
            bpm: 120.0,
            module_graph: None,
        }
    }

    /// List available MIDI input devices
    pub fn list_midi_devices() -> Result<Vec<String>> {
        list_midi_inputs()
    }

    /// Connect to a MIDI input device
    pub fn connect_midi(&mut self, device_name: &str) -> Result<()> {
        let mut midi_input = MidiInputManager::new()?;
        midi_input.connect(device_name)?;
        self.midi_input = Some(midi_input);
        Ok(())
    }

    /// Disconnect MIDI input
    pub fn disconnect_midi(&mut self) {
        if let Some(ref mut midi) = self.midi_input {
            midi.disconnect();
        }
        self.midi_input = None;
    }

    /// Check if MIDI is connected
    pub fn midi_connected(&self) -> bool {
        self.midi_input.as_ref().map_or(false, |m| m.is_connected())
    }

    /// List available audio output devices
    pub fn list_audio_devices() -> Result<Vec<String>> {
        AudioOutput::list_devices()
    }

    /// Start audio output with default device
    pub fn start_audio(&mut self) -> Result<()> {
        let audio_config = AudioConfig {
            sample_rate: self.config.sample_rate,
            buffer_size: self.config.buffer_size,
            channels: 2,
        };

        let mut audio_output = AudioOutput::new(audio_config)?;
        
        // Get actual sample rate from device
        if let Ok(device_rate) = audio_output.device_sample_rate() {
            if device_rate != self.config.sample_rate {
                // Reinitialize with device sample rate
                self.config.sample_rate = device_rate;
                self.voices = SynthVoiceManager::new(self.config.max_voices, device_rate);
                self.delay = Delay::new(2.0, device_rate);
                self.reverb = SchroederReverb::new(device_rate);
                self.chorus = Chorus::new(device_rate);
                self.compressor = Compressor::new(device_rate);
                self.limiter = Limiter::new(device_rate);
            }
        }

        // Create shared state for audio callback
        let engine_state = Arc::new(Mutex::new(EngineState {
            voices: self.voices.clone(),
            delay: self.delay.clone(),
            reverb: self.reverb.clone(),
            chorus: self.chorus.clone(),
            distortion: self.distortion.clone(),
            compressor: self.compressor.clone(),
            limiter: self.limiter.clone(),
            delay_mix: self.delay_mix,
            reverb_mix: self.reverb_mix,
            chorus_mix: self.chorus_mix,
            distortion_mix: self.distortion_mix,
            master_volume: self.master_volume,
            pending_notes: Vec::new(),
        }));

        let state_clone = Arc::clone(&engine_state);

        audio_output.start(move |buffer| {
            let mut state = state_clone.lock();
            
            // Process pending notes
            let notes: Vec<_> = state.pending_notes.drain(..).collect();
            for (note, velocity, is_on) in notes {
                if is_on {
                    state.voices.note_on(note, velocity);
                } else {
                    state.voices.note_off(note);
                }
            }
            
            // Generate audio
            state.process(buffer);
        })?;

        self.audio_output = Some(audio_output);
        Ok(())
    }

    /// Start audio output with specific device
    pub fn start_audio_device(&mut self, device_name: &str) -> Result<()> {
        let audio_config = AudioConfig {
            sample_rate: self.config.sample_rate,
            buffer_size: self.config.buffer_size,
            channels: 2,
        };

        let audio_output = AudioOutput::with_device(audio_config, device_name)?;
        // Similar setup as start_audio...
        self.audio_output = Some(audio_output);
        Ok(())
    }

    /// Stop audio output
    pub fn stop_audio(&mut self) {
        if let Some(ref mut audio) = self.audio_output {
            audio.stop();
        }
        self.audio_output = None;
    }

    /// Check if audio is running
    pub fn audio_running(&self) -> bool {
        self.audio_output.as_ref().map_or(false, |a| a.is_running())
    }

    /// Poll MIDI input and process messages
    pub fn poll_midi(&mut self) {
        if let Some(ref midi) = self.midi_input {
            for msg in midi.poll_all() {
                self.process_midi_message(msg);
            }
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
        self.reverb.set_damping(preset.effects.reverb_damping);
        self.reverb.set_pre_delay(preset.effects.reverb_pre_delay);
        self.reverb_mix = preset.effects.reverb_mix;

        self.chorus.set_rate(preset.effects.chorus_rate);
        self.chorus.set_depth(preset.effects.chorus_depth);
        self.chorus_mix = preset.effects.chorus_mix;

        self.distortion.set_drive(preset.effects.distortion_drive.max(1.0));
        self.distortion_mix = preset.effects.distortion_mix;
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

    /// Set BPM for tempo-synced LFOs
    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(20.0, 300.0);
        self.voices.set_bpm(self.bpm);
    }

    /// Get current BPM
    pub fn bpm(&self) -> f32 {
        self.bpm
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

    /// Set distortion effect parameters
    pub fn set_distortion(&mut self, drive: f32, mix: f32) {
        self.distortion.set_drive(drive);
        self.distortion_mix = mix.clamp(0.0, 1.0);
    }

    /// Set compressor parameters
    pub fn set_compressor(&mut self, threshold: f32, ratio: f32, attack: f32, release: f32) {
        self.compressor.set_threshold(threshold);
        self.compressor.set_ratio(ratio);
        self.compressor.set_attack(attack);
        self.compressor.set_release(release);
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

        // Apply effects chain
        for sample in output.iter_mut() {
            // Distortion (before other effects)
            if self.distortion_mix > 0.001 {
                let dist_out = self.distortion.process_sample(*sample);
                *sample = *sample * (1.0 - self.distortion_mix) + dist_out * self.distortion_mix;
            }

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

            // Compressor
            *sample = self.compressor.process_sample(*sample);

            // Master volume
            *sample *= self.master_volume;

            // Limiter (always on)
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
    pub fn buffer_size(&self) -> u32 {
        self.config.buffer_size
    }

    /// Get active voice count
    pub fn active_voices(&self) -> usize {
        self.voices.active_count()
    }

    /// Get process context
    pub fn context(&self) -> &ProcessContext {
        &self.context
    }

    /// Get mutable process context
    pub fn context_mut(&mut self) -> &mut ProcessContext {
        &mut self.context
    }

    /// Get module graph (if configured)
    pub fn module_graph(&self) -> Option<&ModuleGraph> {
        self.module_graph.as_ref()
    }

    /// Set module graph for modular routing
    pub fn set_module_graph(&mut self, graph: ModuleGraph) {
        self.module_graph = Some(graph);
    }

    /// Reset all state
    pub fn reset(&mut self) {
        self.delay.reset();
        self.reverb.reset();
        self.chorus.reset();
        self.distortion.reset();
        self.compressor.reset();
        self.limiter.reset();
        self.midi_state.reset();
        if let Some(ref mut graph) = self.module_graph {
            graph.reset();
        }
    }
}

/// Shared engine state for audio callback
#[derive(Clone)]
struct EngineState {
    voices: SynthVoiceManager,
    delay: Delay,
    reverb: SchroederReverb,
    chorus: Chorus,
    distortion: Distortion,
    compressor: Compressor,
    limiter: Limiter,
    delay_mix: f32,
    reverb_mix: f32,
    chorus_mix: f32,
    distortion_mix: f32,
    master_volume: f32,
    pending_notes: Vec<(u8, f32, bool)>,
}

impl EngineState {
    fn process(&mut self, output: &mut [f32]) {
        // Generate voices
        self.voices.process(output);

        // Apply effects
        for sample in output.iter_mut() {
            if self.distortion_mix > 0.001 {
                let dist_out = self.distortion.process_sample(*sample);
                *sample = *sample * (1.0 - self.distortion_mix) + dist_out * self.distortion_mix;
            }

            if self.chorus_mix > 0.001 {
                let chorus_out = self.chorus.process_sample(*sample);
                *sample = *sample * (1.0 - self.chorus_mix) + chorus_out * self.chorus_mix;
            }

            if self.delay_mix > 0.001 {
                let delay_out = self.delay.process_sample(*sample);
                *sample = *sample * (1.0 - self.delay_mix) + delay_out * self.delay_mix;
            }

            if self.reverb_mix > 0.001 {
                let reverb_out = self.reverb.process_sample(*sample);
                *sample = *sample * (1.0 - self.reverb_mix) + reverb_out * self.reverb_mix;
            }

            *sample = self.compressor.process_sample(*sample);
            *sample *= self.master_volume;
            *sample = self.limiter.process_sample(*sample);
        }
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
        assert!(buffer.iter().all(|&s| s <= 1.0));
    }

    #[test]
    fn test_engine_note_on_off() {
        let mut engine = Engine::new(EngineConfig::default());
        engine.note_on(60, 0.8);
        assert_eq!(engine.active_voices(), 1);

        engine.note_off(60);
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
        engine.process_midi(&[0x90, 60, 100]);
        assert_eq!(engine.active_voices(), 1);
    }

    #[test]
    fn test_engine_effects() {
        let mut engine = Engine::new(EngineConfig::default());
        engine.set_delay(0.25, 0.5, 0.3);
        engine.set_reverb(0.8, 0.5, 0.3);
        engine.set_chorus(1.0, 0.5, 0.3);
        engine.set_distortion(5.0, 0.2);

        engine.note_on(60, 0.8);
        let mut buffer = vec![0.0; 256];
        engine.process(&mut buffer);

        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_engine_master_volume() {
        let mut engine = Engine::new(EngineConfig::default());
        engine.set_master_volume(0.5);
        engine.note_on(60, 1.0);

        let mut buffer = vec![0.0; 256];
        engine.process(&mut buffer);

        let peak = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(peak < 0.6);
    }

    #[test]
    fn test_engine_bpm() {
        let mut engine = Engine::new(EngineConfig::default());
        engine.set_bpm(140.0);
        assert_eq!(engine.bpm(), 140.0);
    }

    #[test]
    fn test_engine_list_midi_devices() {
        // Should not panic
        let _ = Engine::list_midi_devices();
    }

    #[test]
    fn test_engine_list_audio_devices() {
        // Should not panic
        let _ = Engine::list_audio_devices();
    }

    #[test]
    fn test_engine_midi_not_connected() {
        let engine = Engine::new(EngineConfig::default());
        assert!(!engine.midi_connected());
    }

    #[test]
    fn test_engine_audio_not_running() {
        let engine = Engine::new(EngineConfig::default());
        assert!(!engine.audio_running());
    }

    #[test]
    fn test_engine_context() {
        let engine = Engine::new(EngineConfig::default());
        assert_eq!(engine.context().sample_rate, 44100);
    }

    #[test]
    fn test_engine_reset() {
        let mut engine = Engine::new(EngineConfig::default());
        engine.note_on(60, 0.8);
        engine.process(&mut vec![0.0; 256]);
        engine.reset();
        // Should not panic
    }

    #[test]
    fn test_engine_compressor() {
        let mut engine = Engine::new(EngineConfig::default());
        engine.set_compressor(-20.0, 4.0, 0.01, 0.1);
        engine.note_on(60, 1.0);
        
        let mut buffer = vec![0.0; 256];
        engine.process(&mut buffer);
        assert!(buffer.iter().any(|&s| s != 0.0));
    }
}
