//! MIDI handling and parsing

pub mod input;
pub mod parser;
pub mod state;

pub use input::{list_midi_inputs, MidiInputManager};
pub use parser::{cc, cc_to_float, note_name, parse_midi, pitch_bend_semitones, velocity_to_float, MidiMessage};
pub use state::{MidiState, NoteState};
