//! Synthesis - Modular synthesizer engine
//!
//! A software modular synthesizer with oscillators, filters, envelopes,
//! LFOs, and a modulation matrix. Real-time audio output with MIDI input.
//!
//! # Example
//! ```no_run
//! use synthesis::prelude::*;
//!
//! let mut engine = Engine::new(EngineConfig::default());
//! engine.note_on(60, 0.8); // Middle C at velocity 0.8
//! ```

pub mod audio;
pub mod dsp;
pub mod effects;
pub mod engine;
pub mod midi;
pub mod modulation;
pub mod modules;
pub mod preset;

/// Prelude with common types
pub mod prelude {
    pub use crate::audio::{AudioConfig, AudioOutput};
    pub use crate::dsp::{db_to_linear, linear_to_db, midi_to_freq};
    pub use crate::effects::{Chorus, Compressor, Delay, Distortion, Effect, EffectProcessor, Limiter, SchroederReverb, StereoDelay};
    pub use crate::engine::{
        Connection, Engine, EngineConfig, InputPort, Module, ModuleContext, ModuleGraph,
        OscModMode, OutputPort, Parameter, PlayMode, PortId, ProcessContext, StealMode,
        SynthVoice, SynthVoiceManager, UnisonConfig, VoiceParams,
    };
    pub use crate::midi::{list_midi_inputs, MidiInputManager, MidiMessage, MidiState, parse_midi};
    pub use crate::modulation::{ModDest, ModSlot, ModSource, ModSources, ModValues, ModulationMatrix, Sequencer, SequencerDirection};
    pub use crate::modules::{Envelope, Filter, FilterMode, Lfo, LfoSync, Oscillator, Polarity, StateVariableFilter, Waveform};
    pub use crate::preset::{builtin_presets, Preset};
}

/// Sample rate type alias
pub type SampleRate = u32;

/// Common result type
pub type Result<T> = std::result::Result<T, Error>;

/// Synthesis error types
#[derive(Debug)]
pub enum Error {
    Audio(String),
    Midi(String),
    Preset(String),
    Parameter(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Audio(msg) => write!(f, "Audio error: {}", msg),
            Error::Midi(msg) => write!(f, "MIDI error: {}", msg),
            Error::Preset(msg) => write!(f, "Preset error: {}", msg),
            Error::Parameter(msg) => write!(f, "Parameter error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
