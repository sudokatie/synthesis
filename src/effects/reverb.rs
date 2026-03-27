//! Schroeder reverb

/// Schroeder reverb
pub struct SchroederReverb {
    sample_rate: u32,
}

impl SchroederReverb {
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        // TODO: Implement comb and allpass filters
        input
    }
}
