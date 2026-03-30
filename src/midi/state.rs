//! MIDI state tracking

use super::parser::{cc, cc_to_float, parse_midi, pitch_bend_semitones, velocity_to_float, MidiMessage};

/// Track state of held notes
#[derive(Debug, Clone)]
pub struct NoteState {
    /// Note number
    pub note: u8,
    /// Velocity 0.0-1.0
    pub velocity: f32,
    /// MIDI channel (0-15)
    pub channel: u8,
}

/// Complete MIDI state tracker
#[derive(Debug, Clone)]
pub struct MidiState {
    /// Currently held notes
    notes_held: Vec<NoteState>,
    /// CC values (0.0-1.0 for each controller)
    cc: [f32; 128],
    /// Pitch bend in semitones
    pitch_bend: f32,
    /// Pitch bend range in semitones
    pitch_bend_range: f32,
    /// Sustain pedal state
    sustain: bool,
    /// Channel pressure (aftertouch)
    pressure: f32,
    /// Mod wheel value
    mod_wheel: f32,
}

impl MidiState {
    pub fn new() -> Self {
        let mut cc = [0.0; 128];
        // Default CC values
        cc[cc::VOLUME as usize] = 1.0;
        cc[cc::EXPRESSION as usize] = 1.0;
        cc[cc::PAN as usize] = 0.5;

        Self {
            notes_held: Vec::new(),
            cc,
            pitch_bend: 0.0,
            pitch_bend_range: 2.0,
            sustain: false,
            pressure: 0.0,
            mod_wheel: 0.0,
        }
    }

    /// Set pitch bend range in semitones
    pub fn set_pitch_bend_range(&mut self, semitones: f32) {
        self.pitch_bend_range = semitones.clamp(0.0, 24.0);
    }

    /// Process raw MIDI bytes
    pub fn process_bytes(&mut self, data: &[u8]) {
        let msg = parse_midi(data);
        self.process_message(msg);
    }

    /// Process a parsed MIDI message
    pub fn process_message(&mut self, msg: MidiMessage) {
        match msg {
            MidiMessage::NoteOn { channel, note, velocity } => {
                let vel = velocity_to_float(velocity);
                // Remove existing note if present
                self.notes_held.retain(|n| n.note != note);
                self.notes_held.push(NoteState { note, velocity: vel, channel });
            }
            MidiMessage::NoteOff { note, .. } => {
                if !self.sustain {
                    self.notes_held.retain(|n| n.note != note);
                }
            }
            MidiMessage::ControlChange { controller, value, .. } => {
                let val = cc_to_float(value);
                self.cc[controller as usize] = val;

                match controller {
                    cc::SUSTAIN => {
                        self.sustain = value >= 64;
                    }
                    cc::MOD_WHEEL => {
                        self.mod_wheel = val;
                    }
                    cc::ALL_NOTES_OFF | cc::ALL_SOUND_OFF => {
                        self.notes_held.clear();
                    }
                    cc::RESET_ALL => {
                        self.reset();
                    }
                    _ => {}
                }
            }
            MidiMessage::PitchBend { value, .. } => {
                self.pitch_bend = pitch_bend_semitones(value, self.pitch_bend_range);
            }
            MidiMessage::Aftertouch { pressure, .. } => {
                self.pressure = cc_to_float(pressure);
            }
            MidiMessage::PolyAftertouch { note, pressure, .. } => {
                // Could track per-note pressure, but for now just use channel pressure
                if self.notes_held.iter().any(|n| n.note == note) {
                    self.pressure = cc_to_float(pressure);
                }
            }
            _ => {}
        }
    }

    /// Get held notes
    /// Get held notes
    pub fn notes_held(&self) -> &[NoteState] {
        &self.notes_held
    }

    /// Get CC value
    pub fn cc(&self, controller: u8) -> f32 {
        self.cc[controller as usize]
    }

    /// Get pitch bend in semitones
    pub fn pitch_bend(&self) -> f32 {
        self.pitch_bend
    }

    /// Get sustain state
    pub fn sustain(&self) -> bool {
        self.sustain
    }

    /// Get channel pressure
    pub fn pressure(&self) -> f32 {
        self.pressure
    }

    /// Get mod wheel value
    pub fn mod_wheel(&self) -> f32 {
        self.mod_wheel
    }

    /// Get volume (CC7 * CC11)
    pub fn volume(&self) -> f32 {
        self.cc[cc::VOLUME as usize] * self.cc[cc::EXPRESSION as usize]
    }

    /// Get pan (-1.0 to 1.0)
    pub fn pan(&self) -> f32 {
        self.cc[cc::PAN as usize] * 2.0 - 1.0
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.notes_held.clear();
        self.cc = [0.0; 128];
        self.cc[cc::VOLUME as usize] = 1.0;
        self.cc[cc::EXPRESSION as usize] = 1.0;
        self.cc[cc::PAN as usize] = 0.5;
        self.pitch_bend = 0.0;
        self.sustain = false;
        self.pressure = 0.0;
        self.mod_wheel = 0.0;
    }
}

impl Default for MidiState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_state_new() {
        let state = MidiState::new();
        assert!(state.notes_held.is_empty());
        assert_eq!(state.pitch_bend, 0.0);
    }

    #[test]
    fn test_note_on() {
        let mut state = MidiState::new();
        state.process_message(MidiMessage::NoteOn {
            channel: 0,
            note: 60,
            velocity: 100,
        });
        assert_eq!(state.notes_held.len(), 1);
        assert_eq!(state.notes_held[0].note, 60);
    }

    #[test]
    fn test_note_off() {
        let mut state = MidiState::new();
        state.process_message(MidiMessage::NoteOn {
            channel: 0,
            note: 60,
            velocity: 100,
        });
        state.process_message(MidiMessage::NoteOff {
            channel: 0,
            note: 60,
            velocity: 0,
        });
        assert!(state.notes_held.is_empty());
    }

    #[test]
    fn test_sustain_holds_notes() {
        let mut state = MidiState::new();
        // Sustain on
        state.process_message(MidiMessage::ControlChange {
            channel: 0,
            controller: cc::SUSTAIN,
            value: 127,
        });
        // Note on
        state.process_message(MidiMessage::NoteOn {
            channel: 0,
            note: 60,
            velocity: 100,
        });
        // Note off
        state.process_message(MidiMessage::NoteOff {
            channel: 0,
            note: 60,
            velocity: 0,
        });
        // Note should still be held
        assert_eq!(state.notes_held.len(), 1);
    }

    #[test]
    fn test_pitch_bend() {
        let mut state = MidiState::new();
        state.set_pitch_bend_range(2.0);
        state.process_message(MidiMessage::PitchBend {
            channel: 0,
            value: 8192,
        });
        assert!((state.pitch_bend - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_cc() {
        let mut state = MidiState::new();
        state.process_message(MidiMessage::ControlChange {
            channel: 0,
            controller: cc::MOD_WHEEL,
            value: 64,
        });
        assert!((state.mod_wheel - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_aftertouch() {
        let mut state = MidiState::new();
        state.process_message(MidiMessage::Aftertouch {
            channel: 0,
            pressure: 100,
        });
        assert!(state.pressure > 0.5);
    }

    #[test]
    fn test_reset() {
        let mut state = MidiState::new();
        state.process_message(MidiMessage::NoteOn {
            channel: 0,
            note: 60,
            velocity: 100,
        });
        state.reset();
        assert!(state.notes_held.is_empty());
    }

    #[test]
    fn test_volume() {
        let mut state = MidiState::new();
        assert!((state.volume() - 1.0).abs() < 0.01);
        
        state.process_message(MidiMessage::ControlChange {
            channel: 0,
            controller: cc::VOLUME,
            value: 64,
        });
        assert!((state.volume() - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_pan() {
        let mut state = MidiState::new();
        assert!((state.pan() - 0.0).abs() < 0.01); // Center

        state.process_message(MidiMessage::ControlChange {
            channel: 0,
            controller: cc::PAN,
            value: 127,
        });
        assert!(state.pan() > 0.9); // Right
    }
}
