//! MIDI handling and parsing

pub mod parser;
pub mod state;

pub use parser::{cc, cc_to_float, note_name, parse_midi, pitch_bend_semitones, velocity_to_float, MidiMessage};
pub use state::{MidiState, NoteState};
