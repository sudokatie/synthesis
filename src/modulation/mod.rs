//! Modulation matrix and sources

pub mod matrix;
pub mod sequencer;
pub mod sources;

pub use matrix::{ModDest, ModSlot, ModSource, ModSources, ModValues, ModulationMatrix};
pub use sequencer::{Sequencer, SequencerDirection};
