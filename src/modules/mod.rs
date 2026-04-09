//! Synthesis modules (oscillators, filters, envelopes, LFOs)

pub mod envelope;
pub mod filter;
pub mod lfo;
pub mod oscillator;

pub use envelope::{Envelope, EnvelopeStage};
pub use filter::{Filter, FilterMode, FormantData, FormantFilter, MoogLadder, StateVariableFilter, Vowel};
pub use lfo::{Lfo, LfoSync, Polarity};
pub use oscillator::{generate_morph_wavetables, generate_wavetable, Oscillator, Waveform};
