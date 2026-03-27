//! Synthesis modules (oscillators, filters, envelopes, LFOs)

pub mod envelope;
pub mod filter;
pub mod lfo;
pub mod oscillator;

pub use envelope::{Envelope, EnvelopeStage};
pub use filter::{Filter, FilterMode, MoogLadder, StateVariableFilter};
pub use lfo::{Lfo, Polarity};
pub use oscillator::{generate_wavetable, Oscillator, Waveform};
