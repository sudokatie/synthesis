//! Audio output via cpal

mod buffer;
mod output;

pub use buffer::AudioBuffer;
pub use output::{AudioConfig, AudioOutput};
