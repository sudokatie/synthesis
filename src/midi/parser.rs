//! MIDI message parsing

/// MIDI message types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MidiMessage {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8, velocity: u8 },
    ControlChange { channel: u8, controller: u8, value: u8 },
    ProgramChange { channel: u8, program: u8 },
    PitchBend { channel: u8, value: i16 },
    Aftertouch { channel: u8, pressure: u8 },
    PolyAftertouch { channel: u8, note: u8, pressure: u8 },
    SystemExclusive(u8),
    Unknown,
}

/// Common MIDI CC numbers
pub mod cc {
    pub const MOD_WHEEL: u8 = 1;
    pub const BREATH: u8 = 2;
    pub const FOOT: u8 = 4;
    pub const PORTAMENTO_TIME: u8 = 5;
    pub const DATA_ENTRY: u8 = 6;
    pub const VOLUME: u8 = 7;
    pub const BALANCE: u8 = 8;
    pub const PAN: u8 = 10;
    pub const EXPRESSION: u8 = 11;
    pub const SUSTAIN: u8 = 64;
    pub const PORTAMENTO: u8 = 65;
    pub const SOSTENUTO: u8 = 66;
    pub const SOFT_PEDAL: u8 = 67;
    pub const LEGATO: u8 = 68;
    pub const ALL_SOUND_OFF: u8 = 120;
    pub const RESET_ALL: u8 = 121;
    pub const ALL_NOTES_OFF: u8 = 123;
}

/// Parse raw MIDI bytes into a message
pub fn parse_midi(data: &[u8]) -> MidiMessage {
    if data.is_empty() {
        return MidiMessage::Unknown;
    }

    let status = data[0];
    let channel = status & 0x0F;
    let msg_type = status & 0xF0;

    match msg_type {
        0x80 if data.len() >= 3 => MidiMessage::NoteOff {
            channel,
            note: data[1],
            velocity: data[2],
        },
        0x90 if data.len() >= 3 => {
            // Note on with velocity 0 is note off
            if data[2] == 0 {
                MidiMessage::NoteOff {
                    channel,
                    note: data[1],
                    velocity: 0,
                }
            } else {
                MidiMessage::NoteOn {
                    channel,
                    note: data[1],
                    velocity: data[2],
                }
            }
        }
        0xA0 if data.len() >= 3 => MidiMessage::PolyAftertouch {
            channel,
            note: data[1],
            pressure: data[2],
        },
        0xB0 if data.len() >= 3 => MidiMessage::ControlChange {
            channel,
            controller: data[1],
            value: data[2],
        },
        0xC0 if data.len() >= 2 => MidiMessage::ProgramChange {
            channel,
            program: data[1],
        },
        0xD0 if data.len() >= 2 => MidiMessage::Aftertouch {
            channel,
            pressure: data[1],
        },
        0xE0 if data.len() >= 3 => {
            let lsb = data[1] as i16;
            let msb = data[2] as i16;
            let value = ((msb << 7) | lsb) - 8192; // Center at 0
            MidiMessage::PitchBend { channel, value }
        }
        0xF0 => MidiMessage::SystemExclusive(data[0]),
        _ => MidiMessage::Unknown,
    }
}

/// Parse note number to name
pub fn note_name(note: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (note as i32 / 12) - 1;
    let name = NAMES[(note % 12) as usize];
    format!("{}{}", name, octave)
}

/// Convert velocity to 0.0-1.0 float
pub fn velocity_to_float(velocity: u8) -> f32 {
    velocity as f32 / 127.0
}

/// Convert CC value to 0.0-1.0 float
pub fn cc_to_float(value: u8) -> f32 {
    value as f32 / 127.0
}

/// Convert pitch bend to semitones (assuming 2 semitone range)
pub fn pitch_bend_semitones(value: i16, range: f32) -> f32 {
    value as f32 / 8192.0 * range
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_note_on() {
        let msg = parse_midi(&[0x90, 60, 100]);
        assert_eq!(
            msg,
            MidiMessage::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100
            }
        );
    }

    #[test]
    fn test_parse_note_off() {
        let msg = parse_midi(&[0x80, 60, 64]);
        assert_eq!(
            msg,
            MidiMessage::NoteOff {
                channel: 0,
                note: 60,
                velocity: 64
            }
        );
    }

    #[test]
    fn test_parse_note_on_vel_zero() {
        let msg = parse_midi(&[0x90, 60, 0]);
        assert_eq!(
            msg,
            MidiMessage::NoteOff {
                channel: 0,
                note: 60,
                velocity: 0
            }
        );
    }

    #[test]
    fn test_parse_cc() {
        let msg = parse_midi(&[0xB0, 1, 64]);
        assert_eq!(
            msg,
            MidiMessage::ControlChange {
                channel: 0,
                controller: 1,
                value: 64
            }
        );
    }

    #[test]
    fn test_parse_pitch_bend() {
        let msg = parse_midi(&[0xE0, 0, 64]); // Center
        if let MidiMessage::PitchBend { channel, value } = msg {
            assert_eq!(channel, 0);
            assert!(value.abs() < 100); // Near center
        } else {
            panic!("Expected PitchBend");
        }
    }

    #[test]
    fn test_note_name() {
        assert_eq!(note_name(60), "C4");
        assert_eq!(note_name(69), "A4");
        assert_eq!(note_name(0), "C-1");
    }

    #[test]
    fn test_velocity_to_float() {
        assert!((velocity_to_float(127) - 1.0).abs() < 0.01);
        assert!((velocity_to_float(0) - 0.0).abs() < 0.01);
        assert!((velocity_to_float(64) - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_pitch_bend_semitones() {
        assert!((pitch_bend_semitones(0, 2.0) - 0.0).abs() < 0.01);
        assert!((pitch_bend_semitones(8192, 2.0) - 2.0).abs() < 0.01);
        assert!((pitch_bend_semitones(-8192, 2.0) - (-2.0)).abs() < 0.01);
    }
}
