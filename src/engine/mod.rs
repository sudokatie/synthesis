//! Audio engine

mod context;
mod engine;
mod module;
mod synth_voice;
mod voice;

pub use context::ProcessContext;
pub use engine::{Engine, EngineConfig};
pub use module::{
    Connection, InputPort, Module, ModuleContext, ModuleGraph, OutputPort, Parameter, PortId,
};
pub use synth_voice::{
    OscModMode, PlayMode, StealMode, SynthVoice, SynthVoiceManager, UnisonConfig, VoiceParams,
};
pub use voice::{UnisonSettings, Voice, VoiceManager};
