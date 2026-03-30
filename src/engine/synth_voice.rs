//! Complete synthesizer voice with oscillators, filter, envelopes, and modulation

use crate::dsp::midi_to_freq;
use crate::modules::{Envelope, Filter, FilterMode, Lfo, LfoSync, Oscillator, Polarity, StateVariableFilter, Waveform};

/// Oscillator modulation mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OscModMode {
    /// No modulation between oscillators
    None,
    /// Frequency modulation (osc2 modulates osc1 frequency)
    Fm,
    /// Phase modulation (osc2 modulates osc1 phase)
    Pm,
    /// Hard sync (osc1 resets osc2 phase)
    Sync,
    /// Ring modulation (multiply outputs)
    Ring,
}

impl Default for OscModMode {
    fn default() -> Self {
        OscModMode::None
    }
}

/// Voice parameters shared across voices
#[derive(Debug, Clone)]
pub struct VoiceParams {
    pub osc1_waveform: Waveform,
    pub osc2_waveform: Waveform,
    pub osc2_detune: f32,       // cents
    pub osc2_octave: i32,       // -2 to +2
    pub osc_mix: f32,           // 0.0 = osc1 only, 1.0 = osc2 only
    pub osc_mod_mode: OscModMode,
    pub osc_mod_amount: f32,    // 0.0 to 1.0
    pub filter_cutoff: f32,     // Hz
    pub filter_resonance: f32,  // 0.0-1.0
    pub filter_mode: FilterMode,
    pub filter_drive: f32,      // 0.0-1.0
    pub filter_env_amount: f32, // -1.0 to 1.0
    pub filter_key_track: f32,  // 0.0-1.0
    pub amp_attack: f32,
    pub amp_decay: f32,
    pub amp_sustain: f32,
    pub amp_release: f32,
    pub amp_curve: f32,         // -1.0 to 1.0
    pub filter_attack: f32,
    pub filter_decay: f32,
    pub filter_sustain: f32,
    pub filter_release: f32,
    pub filter_curve: f32,      // -1.0 to 1.0
    // LFO
    pub lfo_waveform: Waveform,
    pub lfo_rate: f32,
    pub lfo_sync: LfoSync,
    pub lfo_to_pitch: f32,      // semitones
    pub lfo_to_filter: f32,     // Hz
    pub lfo_to_amp: f32,        // 0.0-1.0
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            osc1_waveform: Waveform::Saw,
            osc2_waveform: Waveform::Saw,
            osc2_detune: 5.0,
            osc2_octave: 0,
            osc_mix: 0.5,
            osc_mod_mode: OscModMode::None,
            osc_mod_amount: 0.0,
            filter_cutoff: 8000.0,
            filter_resonance: 0.3,
            filter_mode: FilterMode::LowPass,
            filter_drive: 0.0,
            filter_env_amount: 0.5,
            filter_key_track: 0.5,
            amp_attack: 0.01,
            amp_decay: 0.1,
            amp_sustain: 0.7,
            amp_release: 0.3,
            amp_curve: 0.0,
            filter_attack: 0.01,
            filter_decay: 0.2,
            filter_sustain: 0.3,
            filter_release: 0.5,
            filter_curve: 0.0,
            lfo_waveform: Waveform::Sine,
            lfo_rate: 1.0,
            lfo_sync: LfoSync::Free,
            lfo_to_pitch: 0.0,
            lfo_to_filter: 0.0,
            lfo_to_amp: 0.0,
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
    age: u64,
    
    // Oscillators
    osc1: Oscillator,
    osc2: Oscillator,
    osc1_last_phase: f32,
    
    // Filter
    filter: StateVariableFilter,
    
    // Envelopes
    amp_env: Envelope,
    filter_env: Envelope,
    
    // LFO
    lfo: Lfo,
    
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
            osc1_last_phase: 0.0,
            filter: StateVariableFilter::new(sample_rate),
            amp_env: Envelope::new(0.01, 0.1, 0.7, 0.3, sample_rate),
            filter_env: Envelope::new(0.01, 0.2, 0.3, 0.5, sample_rate),
            lfo: Lfo::new(Waveform::Sine, 1.0, sample_rate),
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
        self.osc1_last_phase = 0.0;
        
        let osc2_freq = self.base_freq * 2.0_f32.powi(params.osc2_octave);
        self.osc2.set_waveform(params.osc2_waveform.clone());
        self.osc2.set_frequency(osc2_freq);
        self.osc2.set_detune(params.osc2_detune);
        self.osc2.reset();
        
        // Configure FM/PM amounts based on mod mode
        match params.osc_mod_mode {
            OscModMode::Fm => {
                self.osc1.set_fm_amount(params.osc_mod_amount * 1000.0); // Hz
                self.osc1.set_pm_amount(0.0);
            }
            OscModMode::Pm => {
                self.osc1.set_fm_amount(0.0);
                self.osc1.set_pm_amount(params.osc_mod_amount * std::f32::consts::PI);
            }
            _ => {
                self.osc1.set_fm_amount(0.0);
                self.osc1.set_pm_amount(0.0);
            }
        }
        
        // Configure filter
        self.filter.set_cutoff(params.filter_cutoff);
        self.filter.set_resonance(params.filter_resonance);
        self.filter.set_mode(params.filter_mode);
        self.filter.set_drive(params.filter_drive);
        self.filter.reset();
        
        // Configure envelopes
        self.amp_env = Envelope::new(
            params.amp_attack,
            params.amp_decay,
            params.amp_sustain,
            params.amp_release,
            self.sample_rate as u32,
        );
        self.amp_env.set_curve(params.amp_curve);
        
        self.filter_env = Envelope::new(
            params.filter_attack,
            params.filter_decay,
            params.filter_sustain,
            params.filter_release,
            self.sample_rate as u32,
        );
        self.filter_env.set_curve(params.filter_curve);
        
        // Configure LFO
        self.lfo = Lfo::new(params.lfo_waveform.clone(), params.lfo_rate, self.sample_rate as u32);
        self.lfo.set_sync(params.lfo_sync);
        self.lfo.set_polarity(Polarity::Bipolar);
        self.lfo.note_on(); // Reset if KeySync
        
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

    /// Check if voice is still active
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
        
        // Get LFO value
        let lfo_val = self.lfo.process_sample();
        
        // Apply LFO to pitch (semitones -> frequency multiplier)
        let pitch_mod = 2.0_f32.powf(lfo_val * params.lfo_to_pitch / 12.0);
        self.osc1.set_frequency(self.base_freq * pitch_mod);
        let osc2_base = self.base_freq * 2.0_f32.powi(params.osc2_octave);
        self.osc2.set_frequency(osc2_base * pitch_mod);
        
        // Generate oscillator outputs based on modulation mode
        let (osc1_out, osc2_out) = match params.osc_mod_mode {
            OscModMode::None => {
                (self.osc1.process_sample(), self.osc2.process_sample())
            }
            OscModMode::Fm => {
                // Osc2 is modulator, osc1 is carrier
                let modulator = self.osc2.process_sample();
                let carrier = self.osc1.process_sample_fm(modulator);
                (carrier, modulator)
            }
            OscModMode::Pm => {
                // Osc2 modulates osc1 phase
                let modulator = self.osc2.process_sample();
                let carrier = self.osc1.process_sample_pm(modulator);
                (carrier, modulator)
            }
            OscModMode::Sync => {
                // Hard sync: osc1 is master, osc2 is slave
                // When osc1 completes a cycle, reset osc2
                let osc1_out = self.osc1.process_sample();
                
                // Detect zero crossing of osc1 phase (simplified sync detection)
                // In practice, check if phase wrapped
                let current_phase = self.osc1.phase;
                if current_phase < self.osc1_last_phase {
                    self.osc2.sync();
                }
                self.osc1_last_phase = current_phase;
                
                let osc2_out = self.osc2.process_sample();
                (osc1_out, osc2_out)
            }
            OscModMode::Ring => {
                // Ring modulation: multiply outputs
                let o1 = self.osc1.process_sample();
                let o2 = self.osc2.process_sample();
                let ring = o1 * o2;
                (ring, o2)
            }
        };
        
        // Mix oscillators
        let osc_out = osc1_out * (1.0 - params.osc_mix) + osc2_out * params.osc_mix;
        
        // Calculate filter cutoff with modulation
        let key_track_offset = (self.note as f32 - 60.0) * params.filter_key_track * 100.0;
        let env_offset = filter_mod * params.filter_env_amount * 10000.0;
        let lfo_offset = lfo_val * params.lfo_to_filter;
        let cutoff = (params.filter_cutoff + key_track_offset + env_offset + lfo_offset).clamp(20.0, 20000.0);
        self.filter.set_cutoff(cutoff);
        
        // Apply filter
        let filtered = self.filter.process_sample(osc_out);
        
        // Apply LFO to amplitude (tremolo)
        let amp_mod = 1.0 - params.lfo_to_amp * (1.0 - (lfo_val + 1.0) * 0.5);
        
        // Apply amplitude envelope and velocity
        filtered * amp * amp_mod * self.velocity
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

/// Play modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayMode {
    Poly,
    Mono,
    Legato,
}

/// Unison configuration
#[derive(Debug, Clone)]
pub struct UnisonConfig {
    pub voices: usize,
    pub detune: f32,
    pub spread: f32,
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
#[derive(Clone)]
pub struct SynthVoiceManager {
    voices: Vec<SynthVoice>,
    params: VoiceParams,
    steal_mode: StealMode,
    play_mode: PlayMode,
    unison: UnisonConfig,
    age_counter: u64,
    last_note: Option<u8>,
    held_notes: Vec<u8>,
    bpm: f32,
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
            bpm: 120.0,
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

    /// Set BPM for LFO sync
    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(20.0, 300.0);
        for voice in &mut self.voices {
            voice.lfo.set_bpm(bpm);
        }
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
                        self.allocate_mono(prev_note, 0.8, self.play_mode == PlayMode::Legato);
                        self.last_note = Some(prev_note);
                    } else {
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
        
        let voice = &mut self.voices[0];
        
        if legato && voice.gate {
            // Legato: just change pitch, don't retrigger
            voice.note = note;
            voice.osc1.set_frequency(midi_to_freq(note));
            let osc2_freq = midi_to_freq(note) * 2.0_f32.powi(self.params.osc2_octave);
            voice.osc2.set_frequency(osc2_freq);
            voice.base_freq = midi_to_freq(note);
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
    fn test_osc_mod_mode_default() {
        assert_eq!(OscModMode::default(), OscModMode::None);
    }

    #[test]
    fn test_voice_params_default() {
        let params = VoiceParams::default();
        assert_eq!(params.osc_mod_mode, OscModMode::None);
        assert_eq!(params.osc_mod_amount, 0.0);
        assert_eq!(params.filter_drive, 0.0);
        assert_eq!(params.amp_curve, 0.0);
    }

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
        assert!(voice.active);
    }

    #[test]
    fn test_synth_voice_fm() {
        let mut voice = SynthVoice::new(44100);
        let mut params = VoiceParams::default();
        params.osc_mod_mode = OscModMode::Fm;
        params.osc_mod_amount = 0.5;
        voice.trigger(60, 0.8, &params, 0);
        
        let mut buffer = vec![0.0; 256];
        voice.process(&mut buffer, &params);
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_synth_voice_pm() {
        let mut voice = SynthVoice::new(44100);
        let mut params = VoiceParams::default();
        params.osc_mod_mode = OscModMode::Pm;
        params.osc_mod_amount = 0.5;
        voice.trigger(60, 0.8, &params, 0);
        
        let mut buffer = vec![0.0; 256];
        voice.process(&mut buffer, &params);
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_synth_voice_sync() {
        let mut voice = SynthVoice::new(44100);
        let mut params = VoiceParams::default();
        params.osc_mod_mode = OscModMode::Sync;
        voice.trigger(60, 0.8, &params, 0);
        
        let mut buffer = vec![0.0; 256];
        voice.process(&mut buffer, &params);
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_synth_voice_ring() {
        let mut voice = SynthVoice::new(44100);
        let mut params = VoiceParams::default();
        params.osc_mod_mode = OscModMode::Ring;
        voice.trigger(60, 0.8, &params, 0);
        
        let mut buffer = vec![0.0; 256];
        voice.process(&mut buffer, &params);
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_synth_voice_lfo_to_pitch() {
        let mut voice = SynthVoice::new(44100);
        let mut params = VoiceParams::default();
        params.lfo_rate = 5.0;
        params.lfo_to_pitch = 1.0; // 1 semitone vibrato
        voice.trigger(60, 0.8, &params, 0);
        
        let mut buffer = vec![0.0; 256];
        voice.process(&mut buffer, &params);
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_synth_voice_lfo_to_filter() {
        let mut voice = SynthVoice::new(44100);
        let mut params = VoiceParams::default();
        params.lfo_rate = 2.0;
        params.lfo_to_filter = 1000.0; // Hz
        voice.trigger(60, 0.8, &params, 0);
        
        let mut buffer = vec![0.0; 256];
        voice.process(&mut buffer, &params);
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_synth_voice_lfo_to_amp() {
        let mut voice = SynthVoice::new(44100);
        let mut params = VoiceParams::default();
        params.lfo_rate = 5.0;
        params.lfo_to_amp = 0.5; // 50% tremolo
        voice.trigger(60, 0.8, &params, 0);
        
        let mut buffer = vec![0.0; 256];
        voice.process(&mut buffer, &params);
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_synth_voice_process() {
        let mut voice = SynthVoice::new(44100);
        let params = VoiceParams::default();
        voice.trigger(60, 0.8, &params, 0);
        
        let mut buffer = vec![0.0; 256];
        voice.process(&mut buffer, &params);
        assert!(buffer.iter().any(|&s| s != 0.0));
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
        vm.note_on(67, 0.8);
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
        let mut buf = vec![0.0; 100];
        vm.process(&mut buf);
        
        vm.note_on(64, 0.8);
        assert_eq!(vm.active_count(), 1);
        assert_eq!(vm.voices[0].note, 64);
    }

    #[test]
    fn test_voice_manager_process() {
        let mut vm = SynthVoiceManager::new(4, 44100);
        vm.note_on(60, 0.8);
        
        let mut buffer = vec![0.0; 256];
        vm.process(&mut buffer);
        assert!(buffer.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn test_voice_manager_bpm() {
        let mut vm = SynthVoiceManager::new(4, 44100);
        vm.set_bpm(140.0);
        assert_eq!(vm.bpm, 140.0);
    }

    #[test]
    fn test_steal_mode_lowest() {
        let mut vm = SynthVoiceManager::new(2, 44100);
        vm.set_steal_mode(StealMode::Lowest);
        vm.note_on(60, 0.8);
        vm.note_on(72, 0.8);
        vm.note_on(66, 0.8);
        
        let notes: Vec<u8> = vm.voices.iter().filter(|v| v.active).map(|v| v.note).collect();
        assert!(!notes.contains(&60));
    }

    #[test]
    fn test_steal_mode_highest() {
        let mut vm = SynthVoiceManager::new(2, 44100);
        vm.set_steal_mode(StealMode::Highest);
        vm.note_on(60, 0.8);
        vm.note_on(72, 0.8);
        vm.note_on(66, 0.8);
        
        let notes: Vec<u8> = vm.voices.iter().filter(|v| v.active).map(|v| v.note).collect();
        assert!(!notes.contains(&72));
    }
}
