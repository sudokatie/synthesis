//! Modulation matrix - connects sources to destinations

/// Modulation source types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModSource {
    Envelope(usize),
    Lfo(usize),
    Velocity,
    KeyTrack,
    Aftertouch,
    ModWheel,
    PitchBend,
}

/// Modulation destination types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModDest {
    OscFreq(usize),
    OscPulseWidth(usize),
    FilterCutoff,
    FilterResonance,
    LfoRate(usize),
    EnvAttack(usize),
    Volume,
    Pan,
}

/// Single modulation routing
#[derive(Debug, Clone)]
pub struct ModSlot {
    pub source: ModSource,
    pub destination: ModDest,
    pub amount: f32,  // -1.0 to 1.0
    pub bipolar: bool,
}

impl ModSlot {
    pub fn new(source: ModSource, destination: ModDest, amount: f32) -> Self {
        Self {
            source,
            destination,
            amount: amount.clamp(-1.0, 1.0),
            bipolar: true,
        }
    }

    pub fn unipolar(mut self) -> Self {
        self.bipolar = false;
        self
    }
}

/// Collected modulation source values
#[derive(Debug, Clone, Default)]
pub struct ModSources {
    pub envelopes: [f32; 4],
    pub lfos: [f32; 4],
    pub velocity: f32,
    pub key_track: f32,
    pub aftertouch: f32,
    pub mod_wheel: f32,
    pub pitch_bend: f32,
}

impl ModSources {
    pub fn get(&self, source: ModSource) -> f32 {
        match source {
            ModSource::Envelope(i) => self.envelopes.get(i).copied().unwrap_or(0.0),
            ModSource::Lfo(i) => self.lfos.get(i).copied().unwrap_or(0.0),
            ModSource::Velocity => self.velocity,
            ModSource::KeyTrack => self.key_track,
            ModSource::Aftertouch => self.aftertouch,
            ModSource::ModWheel => self.mod_wheel,
            ModSource::PitchBend => self.pitch_bend,
        }
    }
}

/// Computed modulation values per destination
#[derive(Debug, Clone, Default)]
pub struct ModValues {
    pub osc_freq: [f32; 4],
    pub osc_pw: [f32; 4],
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub lfo_rate: [f32; 4],
    pub env_attack: [f32; 4],
    pub volume: f32,
    pub pan: f32,
}

/// Modulation matrix
pub struct ModulationMatrix {
    slots: Vec<ModSlot>,
    max_slots: usize,
}

impl ModulationMatrix {
    pub fn new(max_slots: usize) -> Self {
        Self {
            slots: Vec::with_capacity(max_slots),
            max_slots,
        }
    }

    pub fn add_route(&mut self, slot: ModSlot) -> bool {
        if self.slots.len() < self.max_slots {
            self.slots.push(slot);
            true
        } else {
            false
        }
    }

    pub fn remove_route(&mut self, index: usize) {
        if index < self.slots.len() {
            self.slots.remove(index);
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
    }

    pub fn slots(&self) -> &[ModSlot] {
        &self.slots
    }

    /// Process all modulation routes and compute destination values
    pub fn process(&self, sources: &ModSources) -> ModValues {
        let mut values = ModValues::default();

        for slot in &self.slots {
            let mut source_val = sources.get(slot.source);
            
            // Convert to unipolar if needed
            if !slot.bipolar {
                source_val = (source_val + 1.0) * 0.5;
            }
            
            let mod_val = source_val * slot.amount;

            match slot.destination {
                ModDest::OscFreq(i) => {
                    if let Some(v) = values.osc_freq.get_mut(i) {
                        *v += mod_val;
                    }
                }
                ModDest::OscPulseWidth(i) => {
                    if let Some(v) = values.osc_pw.get_mut(i) {
                        *v += mod_val;
                    }
                }
                ModDest::FilterCutoff => values.filter_cutoff += mod_val,
                ModDest::FilterResonance => values.filter_resonance += mod_val,
                ModDest::LfoRate(i) => {
                    if let Some(v) = values.lfo_rate.get_mut(i) {
                        *v += mod_val;
                    }
                }
                ModDest::EnvAttack(i) => {
                    if let Some(v) = values.env_attack.get_mut(i) {
                        *v += mod_val;
                    }
                }
                ModDest::Volume => values.volume += mod_val,
                ModDest::Pan => values.pan += mod_val,
            }
        }

        values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_slot_new() {
        let slot = ModSlot::new(ModSource::Lfo(0), ModDest::FilterCutoff, 0.5);
        assert_eq!(slot.amount, 0.5);
        assert!(slot.bipolar);
    }

    #[test]
    fn test_mod_slot_clamp() {
        let slot = ModSlot::new(ModSource::Lfo(0), ModDest::FilterCutoff, 2.0);
        assert_eq!(slot.amount, 1.0);
    }

    #[test]
    fn test_matrix_new() {
        let matrix = ModulationMatrix::new(16);
        assert_eq!(matrix.slots().len(), 0);
    }

    #[test]
    fn test_matrix_add_route() {
        let mut matrix = ModulationMatrix::new(16);
        let slot = ModSlot::new(ModSource::Lfo(0), ModDest::FilterCutoff, 0.5);
        assert!(matrix.add_route(slot));
        assert_eq!(matrix.slots().len(), 1);
    }

    #[test]
    fn test_matrix_remove_route() {
        let mut matrix = ModulationMatrix::new(16);
        matrix.add_route(ModSlot::new(ModSource::Lfo(0), ModDest::FilterCutoff, 0.5));
        matrix.add_route(ModSlot::new(ModSource::Envelope(0), ModDest::Volume, 1.0));
        matrix.remove_route(0);
        assert_eq!(matrix.slots().len(), 1);
    }

    #[test]
    fn test_matrix_process() {
        let mut matrix = ModulationMatrix::new(16);
        matrix.add_route(ModSlot::new(ModSource::Lfo(0), ModDest::FilterCutoff, 0.5));
        
        let mut sources = ModSources::default();
        sources.lfos[0] = 1.0;
        
        let values = matrix.process(&sources);
        assert_eq!(values.filter_cutoff, 0.5);
    }

    #[test]
    fn test_matrix_process_multiple() {
        let mut matrix = ModulationMatrix::new(16);
        matrix.add_route(ModSlot::new(ModSource::Lfo(0), ModDest::FilterCutoff, 0.5));
        matrix.add_route(ModSlot::new(ModSource::Envelope(0), ModDest::FilterCutoff, 0.3));
        
        let mut sources = ModSources::default();
        sources.lfos[0] = 1.0;
        sources.envelopes[0] = 1.0;
        
        let values = matrix.process(&sources);
        assert!((values.filter_cutoff - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_mod_sources_get() {
        let mut sources = ModSources::default();
        sources.velocity = 0.8;
        sources.mod_wheel = 0.5;
        
        assert_eq!(sources.get(ModSource::Velocity), 0.8);
        assert_eq!(sources.get(ModSource::ModWheel), 0.5);
    }
}
