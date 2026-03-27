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
#[derive(Debug, Clone, Copy)]
pub enum StealMode {
    Oldest,
    Lowest,
    Highest,
}

/// Voice manager for polyphony
pub struct VoiceManager {
    voices: Vec<Voice>,
    steal_mode: StealMode,
}

impl VoiceManager {
    pub fn new(max_voices: usize) -> Self {
        Self {
            voices: (0..max_voices).map(|_| Voice::new()).collect(),
            steal_mode: StealMode::Oldest,
        }
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

        // TODO: Voice stealing based on steal_mode
        None
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
}
