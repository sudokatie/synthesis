//! Voice management for polyphony

/// Single voice state
#[derive(Debug, Clone)]
pub struct Voice {
    pub note: u8,
    pub velocity: f32,
    pub gate: bool,
    pub active: bool,
}

impl Voice {
    pub fn new() -> Self {
        Self {
            note: 0,
            velocity: 0.0,
            gate: false,
            active: false,
        }
    }

    pub fn trigger(&mut self, note: u8, velocity: f32) {
        self.note = note;
        self.velocity = velocity;
        self.gate = true;
        self.active = true;
    }

    pub fn release(&mut self) {
        self.gate = false;
    }
}

impl Default for Voice {
    fn default() -> Self {
        Self::new()
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

/// Unison voice settings for thicker sounds
#[derive(Debug, Clone, Copy)]
pub struct UnisonSettings {
    /// Number of unison voices (1-8)
    pub voices: usize,
    /// Detune amount in cents between voices
    pub detune: f32,
    /// Stereo spread (0.0 = mono, 1.0 = full stereo)
    pub spread: f32,
}

impl Default for UnisonSettings {
    fn default() -> Self {
        Self {
            voices: 1,
            detune: 0.0,
            spread: 0.0,
        }
    }
}

impl UnisonSettings {
    /// Create new unison settings
    pub fn new(voices: usize, detune: f32, spread: f32) -> Self {
        Self {
            voices: voices.clamp(1, 8),
            detune: detune.clamp(0.0, 100.0),
            spread: spread.clamp(0.0, 1.0),
        }
    }
    
    /// Get detune offset for a specific unison voice index
    /// Returns detune in cents, spread symmetrically around center
    pub fn detune_for_voice(&self, voice_idx: usize) -> f32 {
        if self.voices <= 1 {
            return 0.0;
        }
        let normalized = (voice_idx as f32 / (self.voices - 1) as f32) * 2.0 - 1.0;
        normalized * self.detune
    }
    
    /// Get pan position for a specific unison voice index
    /// Returns pan from -1.0 (left) to 1.0 (right)
    pub fn pan_for_voice(&self, voice_idx: usize) -> f32 {
        if self.voices <= 1 {
            return 0.0;
        }
        let normalized = (voice_idx as f32 / (self.voices - 1) as f32) * 2.0 - 1.0;
        normalized * self.spread
    }
}

/// Voice manager for polyphony
pub struct VoiceManager {
    voices: Vec<Voice>,
    steal_mode: StealMode,
    unison: UnisonSettings,
}

impl VoiceManager {
    pub fn new(max_voices: usize) -> Self {
        Self {
            voices: (0..max_voices).map(|_| Voice::new()).collect(),
            steal_mode: StealMode::Oldest,
            unison: UnisonSettings::default(),
        }
    }
    
    /// Set unison settings
    pub fn set_unison(&mut self, settings: UnisonSettings) {
        self.unison = settings;
    }
    
    /// Get unison settings
    pub fn unison(&self) -> &UnisonSettings {
        &self.unison
    }
    
    /// Set steal mode
    pub fn set_steal_mode(&mut self, mode: StealMode) {
        self.steal_mode = mode;
    }

    /// Get an available voice for a new note
    pub fn note_on(&mut self, note: u8, velocity: f32) -> Option<usize> {
        // First look for inactive voice
        if let Some((idx, voice)) = self
            .voices
            .iter_mut()
            .enumerate()
            .find(|(_, v)| !v.active)
        {
            voice.trigger(note, velocity);
            return Some(idx);
        }

        // Voice stealing based on steal_mode
        let steal_idx = self.find_steal_voice();
        if let Some(idx) = steal_idx {
            self.voices[idx].trigger(note, velocity);
        }
        steal_idx
    }
    
    /// Find voice to steal based on steal_mode
    fn find_steal_voice(&self) -> Option<usize> {
        if self.voices.is_empty() {
            return None;
        }
        
        match self.steal_mode {
            StealMode::Oldest => {
                // Steal the first voice (oldest since we allocate sequentially)
                Some(0)
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

    /// Release all voices playing this note
    pub fn note_off(&mut self, note: u8) {
        for voice in &mut self.voices {
            if voice.note == note && voice.gate {
                voice.release();
            }
        }
    }

    /// Get active voices
    pub fn active_voices(&self) -> impl Iterator<Item = &Voice> {
        self.voices.iter().filter(|v| v.active)
    }

    /// Get mutable active voices
    pub fn active_voices_mut(&mut self) -> impl Iterator<Item = &mut Voice> {
        self.voices.iter_mut().filter(|v| v.active)
    }

    /// Number of active voices
    pub fn active_count(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_new() {
        let voice = Voice::new();
        assert!(!voice.active);
        assert!(!voice.gate);
    }

    #[test]
    fn test_voice_trigger() {
        let mut voice = Voice::new();
        voice.trigger(60, 0.8);
        assert!(voice.active);
        assert!(voice.gate);
        assert_eq!(voice.note, 60);
    }

    #[test]
    fn test_voice_release() {
        let mut voice = Voice::new();
        voice.trigger(60, 0.8);
        voice.release();
        assert!(!voice.gate);
        assert!(voice.active); // Still active until envelope ends
    }

    #[test]
    fn test_voice_manager_new() {
        let vm = VoiceManager::new(8);
        assert_eq!(vm.active_count(), 0);
    }

    #[test]
    fn test_voice_manager_note_on() {
        let mut vm = VoiceManager::new(4);
        let idx = vm.note_on(60, 0.8);
        assert!(idx.is_some());
        assert_eq!(vm.active_count(), 1);
    }

    #[test]
    fn test_voice_manager_note_off() {
        let mut vm = VoiceManager::new(4);
        vm.note_on(60, 0.8);
        vm.note_off(60);
        // Voice is still active (in release), but gate is off
        assert_eq!(vm.active_count(), 1);
    }

    #[test]
    fn test_voice_manager_polyphony() {
        let mut vm = VoiceManager::new(4);
        vm.note_on(60, 0.8);
        vm.note_on(64, 0.8);
        vm.note_on(67, 0.8);
        assert_eq!(vm.active_count(), 3);
    }
    
    #[test]
    fn test_unison_settings_default() {
        let unison = UnisonSettings::default();
        assert_eq!(unison.voices, 1);
        assert_eq!(unison.detune, 0.0);
        assert_eq!(unison.spread, 0.0);
    }
    
    #[test]
    fn test_unison_settings_new() {
        let unison = UnisonSettings::new(4, 10.0, 0.5);
        assert_eq!(unison.voices, 4);
        assert_eq!(unison.detune, 10.0);
        assert_eq!(unison.spread, 0.5);
    }
    
    #[test]
    fn test_unison_settings_clamp() {
        let unison = UnisonSettings::new(100, 200.0, 2.0);
        assert_eq!(unison.voices, 8);
        assert_eq!(unison.detune, 100.0);
        assert_eq!(unison.spread, 1.0);
    }
    
    #[test]
    fn test_unison_detune_for_voice() {
        let unison = UnisonSettings::new(3, 10.0, 1.0);
        assert!((unison.detune_for_voice(0) - (-10.0)).abs() < 0.001);
        assert!((unison.detune_for_voice(1) - 0.0).abs() < 0.001);
        assert!((unison.detune_for_voice(2) - 10.0).abs() < 0.001);
    }
    
    #[test]
    fn test_unison_pan_for_voice() {
        let unison = UnisonSettings::new(3, 10.0, 1.0);
        assert!((unison.pan_for_voice(0) - (-1.0)).abs() < 0.001);
        assert!((unison.pan_for_voice(1) - 0.0).abs() < 0.001);
        assert!((unison.pan_for_voice(2) - 1.0).abs() < 0.001);
    }
    
    #[test]
    fn test_unison_single_voice() {
        let unison = UnisonSettings::new(1, 10.0, 1.0);
        assert_eq!(unison.detune_for_voice(0), 0.0);
        assert_eq!(unison.pan_for_voice(0), 0.0);
    }
    
    #[test]
    fn test_voice_manager_unison() {
        let mut vm = VoiceManager::new(8);
        vm.set_unison(UnisonSettings::new(4, 15.0, 0.8));
        assert_eq!(vm.unison().voices, 4);
        assert_eq!(vm.unison().detune, 15.0);
    }
}
