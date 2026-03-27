//! Synthesis modules (oscillators, filters, envelopes, LFOs)

mod envelope;
mod filter;
mod lfo;
mod oscillator;

pub use envelope::Envelope;
pub use filter::{Filter, FilterMode, MoogLadder, StateVariableFilter};
pub use lfo::Lfo;
pub use oscillator::{Oscillator, Waveform};
