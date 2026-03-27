//! Complete synthesizer voice with oscillators, filter, and envelopes

use crate::dsp::midi_to_freq;
use crate::modules::{Envelope, Filter, FilterMode, Oscillator, StateVariableFilter, Waveform};

/// Voice parameters shared across voices
#[derive(Debug, Clone)]
pub struct VoiceParams {
    pub osc1_waveform: Waveform,
    pub osc2_waveform: Waveform,
    pub osc2_detune: f32,       // cents
    pub osc2_octave: i32,       // -2 to +2
    pub osc_mix: f32,           // 0.0 = osc1 only, 1.0 = osc2 only
    pub filter_cutoff: f32,     // Hz
    pub filter_resonance: f32,  // 0.0-1.0
    pub filter_mode: FilterMode,
    pub filter_env_amount: f32, // -1.0 to 1.0
    pub filter_key_track: f32,  // 0.0-1.0
    pub amp_attack: f32,
    pub amp_decay: f32,
    pub amp_sustain: f32,
    pub amp_release: f32,
    pub filter_attack: f32,
    pub filter_decay: f32,
    pub filter_sustain: f32,
    pub filter_release: f32,
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            osc1_waveform: Waveform::Saw,
            osc2_waveform: Waveform::Saw,
            osc2_detune: 5.0,
            osc2_octave: 0,
            osc_mix: 0.5,
            filter_cutoff: 8000.0,
            filter_resonance: 0.3,
            filter_mode: FilterMode::LowPass,
            filter_env_amount: 0.5,
            filter_key_track: 0.5,
            amp_attack: 0.01,
            amp_decay: 0.1,
            amp_sustain: 0.7,
            amp_release: 0.3,
            filter_attack: 0.01,
            filter_decay: 0.2,
            filter_sustain: 0.3,
            filter_release: 0.5,
        }
    }
}

/// Complete synthesizer voice
#[derive(Debug, Clone)]
pub struct SynthVoice {
    pub note: u8,
    pub velocity: f32,
    pub gate: bool,
    pub active: bool,
    age: u64,  // For voice stealing
    
    // Oscillators
    osc1: Oscillator,
    osc2: Oscillator,
    
    // Filter
    filter: StateVariableFilter,
    
    // Envelopes
    amp_env: Envelope,
    filter_env: Envelope,
    
    // Sample rate
    sample_rate: f32,
    
    // Base frequency
    base_freq: f32,
}

impl SynthVoice {
    /// Create a new voice
    pub fn new(sample_rate: u32) -> Self {
        Self {
            note: 0,
            velocity: 0.0,
            gate: false,
            active: false,
            age: 0,
            osc1: Oscillator::new(Waveform::Saw, sample_rate),
            osc2: Oscillator::new(Waveform::Saw, sample_rate),
            filter: StateVariableFilter::new(sample_rate),
            amp_env: Envelope::new(0.01, 0.1, 0.7, 0.3, sample_rate),
            filter_env: Envelope::new(0.01, 0.2, 0.3, 0.5, sample_rate),
            sample_rate: sample_rate as f32,
            base_freq: 440.0,
        }
    }

    /// Trigger the voice with a note
    pub fn trigger(&mut self, note: u8, velocity: f32, params: &VoiceParams, age: u64) {
        self.note = note;
        self.velocity = velocity;
        self.gate = true;
        self.active = true;
        self.age = age;
        
        // Set base frequency
        self.base_freq = midi_to_freq(note);
        
        // Configure oscillators
        self.osc1.set_waveform(params.osc1_waveform.clone());
        self.osc1.set_frequency(self.base_freq);
        self.osc1.reset();
        
        let osc2_freq = self.base_freq * 2.0_f32.powi(params.osc2_octave);
        self.osc2.set_waveform(params.osc2_waveform.clone());
        self.osc2.set_frequency(osc2_freq);
        self.osc2.set_detune(params.osc2_detune);
        self.osc2.reset();
        
        // Configure filter
        self.filter.set_cutoff(params.filter_cutoff);
        self.filter.set_resonance(params.filter_resonance);
        self.filter.set_mode(params.filter_mode);
        self.filter.reset();
        
        // Configure envelopes
        self.amp_env = Envelope::new(
            params.amp_attack,
            params.amp_decay,
            params.amp_sustain,
            params.amp_release,
            self.sample_rate as u32,
        );
        self.filter_env = Envelope::new(
            params.filter_attack,
            params.filter_decay,
            params.filter_sustain,
            params.filter_release,
            self.sample_rate as u32,
        );
        
        // Trigger envelopes
        self.amp_env.trigger();
        self.filter_env.trigger();
    }

    /// Release the voice
    pub fn release(&mut self) {
        self.gate = false;
        self.amp_env.release();
        self.filter_env.release();
    }

    /// Check if voice is still active (envelope not finished)
    pub fn is_active(&self) -> bool {
        self.amp_env.is_active()
    }

    /// Get voice age for stealing
    pub fn age(&self) -> u64 {
        self.age
    }

    /// Process a single sample
    pub fn process_sample(&mut self, params: &VoiceParams) -> f32 {
        if !self.active {
            return 0.0;
        }

        // Get envelope values
        let amp = self.amp_env.process_sample();
        let filter_mod = self.filter_env.process_sample();
        
        // Check if voice has finished
        if !self.amp_env.is_active() {
            self.active = false;
            return 0.0;
        }
        
        // Generate oscillator outputs
        let osc1_out = self.osc1.process_sample();
        let osc2_out = self.osc2.process_sample();
        
        // Mix oscillators
        let osc_out = osc1_out * (1.0 - params.osc_mix) + osc2_out * params.osc_mix;
        
        // Calculate filter cutoff with modulation
        let key_track_offset = (self.note as f32 - 60.0) * params.filter_key_track * 100.0;
        let env_offset = filter_mod * params.filter_env_amount * 10000.0;
        let cutoff = (params.filter_cutoff + key_track_offset + env_offset).clamp(20.0, 20000.0);
        self.filter.set_cutoff(cutoff);
        
        // Apply filter
        let filtered = self.filter.process_sample(osc_out);
        
        // Apply amplitude envelope and velocity
        filtered * amp * self.velocity
    }

    /// Process a buffer
    pub fn process(&mut self, output: &mut [f32], params: &VoiceParams) {
        for sample in output.iter_mut() {
            *sample += self.process_sample(params);
        }
    }
}

/// Voice allocation modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StealMode {
    Oldest,
    Lowest,
    Highest,
    Quietest,
}

/// Voice allocation modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayMode {
    Poly,
    Mono,
    Legato,
}

/// Unison configuration
#[derive(Debug, Clone)]
pub struct UnisonConfig {
    pub voices: usize,      // 1-8
    pub detune: f32,        // cents spread
    pub spread: f32,        // stereo spread 0.0-1.0
}

impl Default for UnisonConfig {
    fn default() -> Self {
        Self {
            voices: 1,
            detune: 0.0,
            spread: 0.0,
        }
    }
}

/// Voice manager with allocation
pub struct SynthVoiceManager {
    voices: Vec<SynthVoice>,
    params: VoiceParams,
    steal_mode: StealMode,
    play_mode: PlayMode,
    unison: UnisonConfig,
    age_counter: u64,
    last_note: Option<u8>,
    held_notes: Vec<u8>,
}

impl SynthVoiceManager {
    /// Create new voice manager
    pub fn new(max_voices: usize, sample_rate: u32) -> Self {
        Self {
            voices: (0..max_voices).map(|_| SynthVoice::new(sample_rate)).collect(),
            params: VoiceParams::default(),
            steal_mode: StealMode::Oldest,
            play_mode: PlayMode::Poly,
            unison: UnisonConfig::default(),
            age_counter: 0,
            last_note: None,
            held_notes: Vec::new(),
        }
    }

    /// Set voice parameters
    pub fn set_params(&mut self, params: VoiceParams) {
        self.params = params;
    }

    /// Get voice parameters
    pub fn params(&self) -> &VoiceParams {
        &self.params
    }

    /// Set steal mode
    pub fn set_steal_mode(&mut self, mode: StealMode) {
        self.steal_mode = mode;
    }

    /// Set play mode
    pub fn set_play_mode(&mut self, mode: PlayMode) {
        self.play_mode = mode;
    }

    /// Set unison config
    pub fn set_unison(&mut self, config: UnisonConfig) {
        self.unison = config;
    }

    /// Handle note on
    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.held_notes.push(note);
        
        match self.play_mode {
            PlayMode::Poly => self.allocate_poly(note, velocity),
            PlayMode::Mono => self.allocate_mono(note, velocity, false),
            PlayMode::Legato => self.allocate_mono(note, velocity, true),
        }
        
        self.last_note = Some(note);
    }

    /// Handle note off
    pub fn note_off(&mut self, note: u8) {
        self.held_notes.retain(|&n| n != note);
        
        match self.play_mode {
            PlayMode::Poly => {
                for voice in &mut self.voices {
                    if voice.note == note && voice.gate {
                        voice.release();
                    }
                }
            }
            PlayMode::Mono | PlayMode::Legato => {
                if self.last_note == Some(note) {
                    if let Some(&prev_note) = self.held_notes.last() {
                        // Retrigger previous note
                        self.allocate_mono(prev_note, 0.8, self.play_mode == PlayMode::Legato);
                        self.last_note = Some(prev_note);
                    } else {
                        // Release all
                        for voice in &mut self.voices {
                            if voice.gate {
                                voice.release();
                            }
                        }
                        self.last_note = None;
                    }
                }
            }
        }
    }

    fn allocate_poly(&mut self, note: u8, velocity: f32) {
        self.age_counter += 1;
        
        // Look for inactive voice
        if let Some(voice) = self.voices.iter_mut().find(|v| !v.active) {
            voice.trigger(note, velocity, &self.params, self.age_counter);
            return;
        }
        
        // Voice stealing
        let voice_idx = self.find_steal_voice();
        if let Some(idx) = voice_idx {
            self.voices[idx].trigger(note, velocity, &self.params, self.age_counter);
        }
    }

    fn allocate_mono(&mut self, note: u8, velocity: f32, legato: bool) {
        self.age_counter += 1;
        
        // In mono/legato mode, use voice 0
        let voice = &mut self.voices[0];
        
        if legato && voice.gate {
            // Legato: just change pitch, don't retrigger
            voice.note = note;
            voice.osc1.set_frequency(midi_to_freq(note));
            let osc2_freq = midi_to_freq(note) * 2.0_f32.powi(self.params.osc2_octave);
            voice.osc2.set_frequency(osc2_freq);
        } else {
            voice.trigger(note, velocity, &self.params, self.age_counter);
        }
    }

    fn find_steal_voice(&self) -> Option<usize> {
        if self.voices.is_empty() {
            return None;
        }

        match self.steal_mode {
            StealMode::Oldest => {
                self.voices
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, v)| v.age())
                    .map(|(i, _)| i)
            }
            StealMode::Lowest => {
                self.voices
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v.active)
                    .min_by_key(|(_, v)| v.note)
                    .map(|(i, _)| i)
            }
            StealMode::Highest => {
                self.voices
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v.active)
                    .max_by_key(|(_, v)| v.note)
                    .map(|(i, _)| i)
            }
            StealMode::Quietest => {
                self.voices
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v.active)
                    .min_by(|(_, a), (_, b)| {
                        a.velocity.partial_cmp(&b.velocity).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)
            }
        }
    }

    /// Process all voices
    pub fn process(&mut self, output: &mut [f32]) {
        output.fill(0.0);
        for voice in &mut self.voices {
            if voice.active {
                voice.process(output, &self.params);
            }
        }
    }

    /// Get active voice count
    pub fn active_count(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synth_voice_new() {
        let voice = SynthVoice::new(44100);
        assert!(!voice.active);
        assert!(!voice.gate);
    }

    #[test]
    fn test_synth_voice_trigger() {
        let mut voice = SynthVoice::new(44100);
        let params = VoiceParams::default();
        voice.trigger(60, 0.8, &params, 0);
        assert!(voice.active);
        assert!(voice.gate);
        assert_eq!(voice.note, 60);
    }

    #[test]
    fn test_synth_voice_release() {
        let mut voice = SynthVoice::new(44100);
        let params = VoiceParams::default();
        voice.trigger(60, 0.8, &params, 0);
        voice.release();
        assert!(!voice.gate);
        assert!(voice.active); // Still active until envelope ends
    }

    #[test]
    fn test_synth_voice_process() {
        let mut voice = SynthVoice::new(44100);
        let params = VoiceParams::default();
        voice.trigger(60, 0.8, &params, 0);
        
        let mut buffer = vec![0.0; 256];
        voice.process(&mut buffer, &params);
        
        // Should have audio
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_synth_voice_envelope_controls() {
        let mut voice = SynthVoice::new(44100);
        let params = VoiceParams::default();
        voice.trigger(60, 0.8, &params, 0);
        
        // Process attack
        let s1 = voice.process_sample(&params);
        let s2 = voice.process_sample(&params);
        
        // During attack, envelope should rise
        assert!(s2.abs() >= s1.abs() * 0.9 || s1.abs() < 0.001);
    }

    #[test]
    fn test_voice_manager_new() {
        let vm = SynthVoiceManager::new(8, 44100);
        assert_eq!(vm.active_count(), 0);
    }

    #[test]
    fn test_voice_manager_note_on() {
        let mut vm = SynthVoiceManager::new(4, 44100);
        vm.note_on(60, 0.8);
        assert_eq!(vm.active_count(), 1);
    }

    #[test]
    fn test_voice_manager_note_off() {
        let mut vm = SynthVoiceManager::new(4, 44100);
        vm.note_on(60, 0.8);
        vm.note_off(60);
        // Voice still active (in release)
        assert_eq!(vm.active_count(), 1);
    }

    #[test]
    fn test_voice_manager_polyphony() {
        let mut vm = SynthVoiceManager::new(4, 44100);
        vm.note_on(60, 0.8);
        vm.note_on(64, 0.8);
        vm.note_on(67, 0.8);
        assert_eq!(vm.active_count(), 3);
    }

    #[test]
    fn test_voice_manager_voice_stealing() {
        let mut vm = SynthVoiceManager::new(2, 44100);
        vm.note_on(60, 0.8);
        vm.note_on(64, 0.8);
        vm.note_on(67, 0.8); // Should steal oldest
        assert_eq!(vm.active_count(), 2);
    }

    #[test]
    fn test_voice_manager_mono_mode() {
        let mut vm = SynthVoiceManager::new(4, 44100);
        vm.set_play_mode(PlayMode::Mono);
        vm.note_on(60, 0.8);
        vm.note_on(64, 0.8);
        assert_eq!(vm.active_count(), 1);
        assert_eq!(vm.voices[0].note, 64);
    }

    #[test]
    fn test_voice_manager_legato() {
        let mut vm = SynthVoiceManager::new(4, 44100);
        vm.set_play_mode(PlayMode::Legato);
        vm.note_on(60, 0.8);
        // Process a bit
        let mut buf = vec![0.0; 100];
        vm.process(&mut buf);
        
        vm.note_on(64, 0.8);
        // Voice should still be active from first note's envelope
        assert_eq!(vm.active_count(), 1);
        assert_eq!(vm.voices[0].note, 64);
    }

    #[test]
    fn test_voice_manager_process() {
        let mut vm = SynthVoiceManager::new(4, 44100);
        vm.note_on(60, 0.8);
        
        let mut buffer = vec![0.0; 256];
        vm.process(&mut buffer);
        
        // Should have audio
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_steal_mode_lowest() {
        let mut vm = SynthVoiceManager::new(2, 44100);
        vm.set_steal_mode(StealMode::Lowest);
        vm.note_on(60, 0.8);  // C4
        vm.note_on(72, 0.8);  // C5
        vm.note_on(66, 0.8);  // Should steal lowest (60)
        
        let notes: Vec<u8> = vm.voices.iter().filter(|v| v.active).map(|v| v.note).collect();
        assert!(!notes.contains(&60)); // 60 should be stolen
    }

    #[test]
    fn test_steal_mode_highest() {
        let mut vm = SynthVoiceManager::new(2, 44100);
        vm.set_steal_mode(StealMode::Highest);
        vm.note_on(60, 0.8);  // C4
        vm.note_on(72, 0.8);  // C5
        vm.note_on(66, 0.8);  // Should steal highest (72)
        
        let notes: Vec<u8> = vm.voices.iter().filter(|v| v.active).map(|v| v.note).collect();
        assert!(!notes.contains(&72)); // 72 should be stolen
    }
}
