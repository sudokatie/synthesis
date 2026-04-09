//! Audio effects - delay, reverb, distortion, chorus, phaser, compression

pub mod chorus;
pub mod compressor;
pub mod delay;
pub mod distortion;
pub mod effect;
pub mod phaser;
pub mod reverb;

pub use chorus::Chorus;
pub use compressor::{Compressor, Limiter};
pub use delay::{Delay, DelayLine, StereoDelay};
pub use distortion::{Distortion, DistortionType};
pub use effect::{Effect, EffectProcessor};
pub use phaser::{Phaser, PHASER_STAGES};
pub use reverb::{AllpassFilter, CombFilter, SchroederReverb};
