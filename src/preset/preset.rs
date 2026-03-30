//! Preset structure and serialization

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::engine::{OscModMode, VoiceParams};
use crate::modules::{FilterMode, LfoSync, Waveform};

/// Serializable waveform type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WaveformPreset {
    Sine,
    #[default]
    Saw,
    Square { pulse_width: f32 },
    Triangle,
    Noise,
}

impl From<WaveformPreset> for Waveform {
    fn from(preset: WaveformPreset) -> Self {
        match preset {
            WaveformPreset::Sine => Waveform::Sine,
            WaveformPreset::Saw => Waveform::Saw,
            WaveformPreset::Square { pulse_width } => Waveform::Square { pulse_width },
            WaveformPreset::Triangle => Waveform::Triangle,
            WaveformPreset::Noise => Waveform::Noise,
        }
    }
}

impl From<&Waveform> for WaveformPreset {
    fn from(waveform: &Waveform) -> Self {
        match waveform {
            Waveform::Sine => WaveformPreset::Sine,
            Waveform::Saw => WaveformPreset::Saw,
            Waveform::Square { pulse_width } => WaveformPreset::Square {
                pulse_width: *pulse_width,
            },
            Waveform::Triangle => WaveformPreset::Triangle,
            Waveform::Noise => WaveformPreset::Noise,
            Waveform::Wavetable { .. } => WaveformPreset::Saw,
            Waveform::MultiWavetable { .. } => WaveformPreset::Saw,
        }
    }
}

/// Serializable filter mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterModePreset {
    LowPass,
    HighPass,
    BandPass,
    Notch,
    Peak,
    LowShelf,
    HighShelf,
}

impl From<FilterModePreset> for FilterMode {
    fn from(preset: FilterModePreset) -> Self {
        match preset {
            FilterModePreset::LowPass => FilterMode::LowPass,
            FilterModePreset::HighPass => FilterMode::HighPass,
            FilterModePreset::BandPass => FilterMode::BandPass,
            FilterModePreset::Notch => FilterMode::Notch,
            FilterModePreset::Peak => FilterMode::Peak,
            FilterModePreset::LowShelf => FilterMode::LowShelf,
            FilterModePreset::HighShelf => FilterMode::HighShelf,
        }
    }
}

impl From<FilterMode> for FilterModePreset {
    fn from(mode: FilterMode) -> Self {
        match mode {
            FilterMode::LowPass => FilterModePreset::LowPass,
            FilterMode::HighPass => FilterModePreset::HighPass,
            FilterMode::BandPass => FilterModePreset::BandPass,
            FilterMode::Notch => FilterModePreset::Notch,
            FilterMode::Peak => FilterModePreset::Peak,
            FilterMode::LowShelf => FilterModePreset::LowShelf,
            FilterMode::HighShelf => FilterModePreset::HighShelf,
        }
    }
}

/// Serializable oscillator modulation mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OscModModePreset {
    #[default]
    None,
    Fm,
    Pm,
    Sync,
    Ring,
}

impl From<OscModModePreset> for OscModMode {
    fn from(preset: OscModModePreset) -> Self {
        match preset {
            OscModModePreset::None => OscModMode::None,
            OscModModePreset::Fm => OscModMode::Fm,
            OscModModePreset::Pm => OscModMode::Pm,
            OscModModePreset::Sync => OscModMode::Sync,
            OscModModePreset::Ring => OscModMode::Ring,
        }
    }
}

impl From<OscModMode> for OscModModePreset {
    fn from(mode: OscModMode) -> Self {
        match mode {
            OscModMode::None => OscModModePreset::None,
            OscModMode::Fm => OscModModePreset::Fm,
            OscModMode::Pm => OscModModePreset::Pm,
            OscModMode::Sync => OscModModePreset::Sync,
            OscModMode::Ring => OscModModePreset::Ring,
        }
    }
}

/// Serializable LFO sync mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LfoSyncPreset {
    #[default]
    Free,
    KeySync,
    BpmSync { division: f32 },
}

impl From<LfoSyncPreset> for LfoSync {
    fn from(preset: LfoSyncPreset) -> Self {
        match preset {
            LfoSyncPreset::Free => LfoSync::Free,
            LfoSyncPreset::KeySync => LfoSync::KeySync,
            LfoSyncPreset::BpmSync { division } => LfoSync::BpmSync { division },
        }
    }
}

impl From<LfoSync> for LfoSyncPreset {
    fn from(sync: LfoSync) -> Self {
        match sync {
            LfoSync::Free => LfoSyncPreset::Free,
            LfoSync::KeySync => LfoSyncPreset::KeySync,
            LfoSync::BpmSync { division } => LfoSyncPreset::BpmSync { division },
        }
    }
}

/// Oscillator settings in preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OscillatorPreset {
    pub waveform: WaveformPreset,
    pub detune: f32,
    pub octave: i32,
}

impl Default for OscillatorPreset {
    fn default() -> Self {
        Self {
            waveform: WaveformPreset::Saw,
            detune: 0.0,
            octave: 0,
        }
    }
}

/// Oscillator modulation settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OscModPreset {
    #[serde(default)]
    pub mode: OscModModePreset,
    #[serde(default)]
    pub amount: f32,
}

/// Filter settings in preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPreset {
    pub cutoff: f32,
    pub resonance: f32,
    pub mode: FilterModePreset,
    #[serde(default)]
    pub drive: f32,
    pub env_amount: f32,
    pub key_track: f32,
    #[serde(default)]
    pub gain_db: f32,
}

impl Default for FilterPreset {
    fn default() -> Self {
        Self {
            cutoff: 8000.0,
            resonance: 0.3,
            mode: FilterModePreset::LowPass,
            drive: 0.0,
            env_amount: 0.5,
            key_track: 0.5,
            gain_db: 0.0,
        }
    }
}

/// Envelope settings in preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopePreset {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    #[serde(default)]
    pub curve: f32,
}

impl Default for EnvelopePreset {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.3,
            curve: 0.0,
        }
    }
}

/// LFO settings in preset
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LfoPreset {
    #[serde(default = "default_lfo_waveform")]
    pub waveform: WaveformPreset,
    #[serde(default = "default_lfo_rate")]
    pub rate: f32,
    #[serde(default)]
    pub sync: LfoSyncPreset,
    #[serde(default)]
    pub to_pitch: f32,
    #[serde(default)]
    pub to_filter: f32,
    #[serde(default)]
    pub to_amp: f32,
}

fn default_lfo_waveform() -> WaveformPreset {
    WaveformPreset::Sine
}

fn default_lfo_rate() -> f32 {
    1.0
}

/// Effects settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EffectsPreset {
    #[serde(default)]
    pub delay_time: f32,
    #[serde(default)]
    pub delay_feedback: f32,
    #[serde(default)]
    pub delay_mix: f32,
    #[serde(default)]
    pub reverb_size: f32,
    #[serde(default)]
    pub reverb_damping: f32,
    #[serde(default)]
    pub reverb_pre_delay: f32,
    #[serde(default)]
    pub reverb_mix: f32,
    #[serde(default)]
    pub chorus_rate: f32,
    #[serde(default)]
    pub chorus_depth: f32,
    #[serde(default)]
    pub chorus_mix: f32,
    #[serde(default)]
    pub distortion_drive: f32,
    #[serde(default)]
    pub distortion_mix: f32,
}

/// Preset format version
pub const PRESET_VERSION: u32 = 1;

/// Complete synthesizer preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: u32,
    pub author: Option<String>,
    pub category: Option<String>,
    pub osc1: OscillatorPreset,
    pub osc2: OscillatorPreset,
    pub osc_mix: f32,
    #[serde(default)]
    pub osc_mod: OscModPreset,
    pub filter: FilterPreset,
    pub amp_env: EnvelopePreset,
    pub filter_env: EnvelopePreset,
    #[serde(default)]
    pub lfo: LfoPreset,
    #[serde(default)]
    pub effects: EffectsPreset,
}

fn default_version() -> u32 {
    PRESET_VERSION
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            name: "Init".to_string(),
            version: PRESET_VERSION,
            author: None,
            category: None,
            osc1: OscillatorPreset::default(),
            osc2: OscillatorPreset {
                waveform: WaveformPreset::Saw,
                detune: 5.0,
                octave: 0,
            },
            osc_mix: 0.5,
            osc_mod: OscModPreset::default(),
            filter: FilterPreset::default(),
            amp_env: EnvelopePreset::default(),
            filter_env: EnvelopePreset {
                attack: 0.01,
                decay: 0.2,
                sustain: 0.3,
                release: 0.5,
                curve: 0.0,
            },
            lfo: LfoPreset::default(),
            effects: EffectsPreset::default(),
        }
    }
}

impl Preset {
    /// Create preset from voice parameters
    pub fn from_params(name: &str, params: &VoiceParams) -> Self {
        Self {
            name: name.to_string(),
            version: PRESET_VERSION,
            author: None,
            category: None,
            osc1: OscillatorPreset {
                waveform: WaveformPreset::from(&params.osc1_waveform),
                detune: 0.0,
                octave: 0,
            },
            osc2: OscillatorPreset {
                waveform: WaveformPreset::from(&params.osc2_waveform),
                detune: params.osc2_detune,
                octave: params.osc2_octave,
            },
            osc_mix: params.osc_mix,
            osc_mod: OscModPreset {
                mode: OscModModePreset::from(params.osc_mod_mode),
                amount: params.osc_mod_amount,
            },
            filter: FilterPreset {
                cutoff: params.filter_cutoff,
                resonance: params.filter_resonance,
                mode: FilterModePreset::from(params.filter_mode),
                drive: params.filter_drive,
                env_amount: params.filter_env_amount,
                key_track: params.filter_key_track,
                gain_db: 0.0,
            },
            amp_env: EnvelopePreset {
                attack: params.amp_attack,
                decay: params.amp_decay,
                sustain: params.amp_sustain,
                release: params.amp_release,
                curve: params.amp_curve,
            },
            filter_env: EnvelopePreset {
                attack: params.filter_attack,
                decay: params.filter_decay,
                sustain: params.filter_sustain,
                release: params.filter_release,
                curve: params.filter_curve,
            },
            lfo: LfoPreset {
                waveform: WaveformPreset::from(&params.lfo_waveform),
                rate: params.lfo_rate,
                sync: LfoSyncPreset::from(params.lfo_sync),
                to_pitch: params.lfo_to_pitch,
                to_filter: params.lfo_to_filter,
                to_amp: params.lfo_to_amp,
            },
            effects: EffectsPreset::default(),
        }
    }

    /// Convert preset to voice parameters
    pub fn to_params(&self) -> VoiceParams {
        VoiceParams {
            osc1_waveform: self.osc1.waveform.clone().into(),
            osc2_waveform: self.osc2.waveform.clone().into(),
            osc2_detune: self.osc2.detune,
            osc2_octave: self.osc2.octave,
            osc_mix: self.osc_mix,
            osc_mod_mode: self.osc_mod.mode.into(),
            osc_mod_amount: self.osc_mod.amount,
            filter_cutoff: self.filter.cutoff,
            filter_resonance: self.filter.resonance,
            filter_mode: self.filter.mode.into(),
            filter_drive: self.filter.drive,
            filter_env_amount: self.filter.env_amount,
            filter_key_track: self.filter.key_track,
            amp_attack: self.amp_env.attack,
            amp_decay: self.amp_env.decay,
            amp_sustain: self.amp_env.sustain,
            amp_release: self.amp_env.release,
            amp_curve: self.amp_env.curve,
            filter_attack: self.filter_env.attack,
            filter_decay: self.filter_env.decay,
            filter_sustain: self.filter_env.sustain,
            filter_release: self.filter_env.release,
            filter_curve: self.filter_env.curve,
            lfo_waveform: self.lfo.waveform.clone().into(),
            lfo_rate: self.lfo.rate,
            lfo_sync: self.lfo.sync.into(),
            lfo_to_pitch: self.lfo.to_pitch,
            lfo_to_filter: self.lfo.to_filter,
            lfo_to_amp: self.lfo.to_amp,
        }
    }

    /// Load preset from JSON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, PresetError> {
        let contents = fs::read_to_string(path).map_err(|e| PresetError::Io(e.to_string()))?;
        serde_json::from_str(&contents).map_err(|e| PresetError::Parse(e.to_string()))
    }

    /// Save preset to JSON file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), PresetError> {
        let json = serde_json::to_string_pretty(self).map_err(|e| PresetError::Parse(e.to_string()))?;
        fs::write(path, json).map_err(|e| PresetError::Io(e.to_string()))
    }
}

/// Preset errors
#[derive(Debug)]
pub enum PresetError {
    Io(String),
    Parse(String),
}

impl std::fmt::Display for PresetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PresetError::Io(msg) => write!(f, "IO error: {}", msg),
            PresetError::Parse(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for PresetError {}

/// Built-in preset library
pub fn builtin_presets() -> Vec<Preset> {
    vec![
        Preset::default(),
        // Classic Bass
        Preset {
            name: "Classic Bass".to_string(),
            version: PRESET_VERSION,
            author: Some("Katie".to_string()),
            category: Some("Bass".to_string()),
            osc1: OscillatorPreset {
                waveform: WaveformPreset::Saw,
                detune: 0.0,
                octave: -1,
            },
            osc2: OscillatorPreset {
                waveform: WaveformPreset::Square { pulse_width: 0.5 },
                detune: 3.0,
                octave: -1,
            },
            osc_mix: 0.6,
            osc_mod: OscModPreset::default(),
            filter: FilterPreset {
                cutoff: 500.0,
                resonance: 0.4,
                mode: FilterModePreset::LowPass,
                drive: 0.2,
                env_amount: 0.7,
                key_track: 0.3,
                gain_db: 0.0,
            },
            amp_env: EnvelopePreset {
                attack: 0.001,
                decay: 0.3,
                sustain: 0.6,
                release: 0.2,
                curve: -0.3,
            },
            filter_env: EnvelopePreset {
                attack: 0.001,
                decay: 0.4,
                sustain: 0.2,
                release: 0.3,
                curve: -0.5,
            },
            lfo: LfoPreset::default(),
            effects: EffectsPreset::default(),
        },
        // Soft Pad
        Preset {
            name: "Soft Pad".to_string(),
            version: PRESET_VERSION,
            author: Some("Katie".to_string()),
            category: Some("Pad".to_string()),
            osc1: OscillatorPreset {
                waveform: WaveformPreset::Saw,
                detune: -7.0,
                octave: 0,
            },
            osc2: OscillatorPreset {
                waveform: WaveformPreset::Saw,
                detune: 7.0,
                octave: 0,
            },
            osc_mix: 0.5,
            osc_mod: OscModPreset::default(),
            filter: FilterPreset {
                cutoff: 2000.0,
                resonance: 0.2,
                mode: FilterModePreset::LowPass,
                drive: 0.0,
                env_amount: 0.3,
                key_track: 0.5,
                gain_db: 0.0,
            },
            amp_env: EnvelopePreset {
                attack: 0.5,
                decay: 0.3,
                sustain: 0.8,
                release: 1.0,
                curve: 0.3,
            },
            filter_env: EnvelopePreset {
                attack: 0.8,
                decay: 0.5,
                sustain: 0.5,
                release: 1.0,
                curve: 0.0,
            },
            lfo: LfoPreset {
                waveform: WaveformPreset::Sine,
                rate: 0.3,
                sync: LfoSyncPreset::Free,
                to_pitch: 0.1,
                to_filter: 200.0,
                to_amp: 0.0,
            },
            effects: EffectsPreset {
                reverb_size: 0.8,
                reverb_mix: 0.4,
                chorus_rate: 0.3,
                chorus_depth: 0.5,
                chorus_mix: 0.3,
                ..Default::default()
            },
        },
        // FM Bell
        Preset {
            name: "FM Bell".to_string(),
            version: PRESET_VERSION,
            author: Some("Katie".to_string()),
            category: Some("Bell".to_string()),
            osc1: OscillatorPreset {
                waveform: WaveformPreset::Sine,
                detune: 0.0,
                octave: 0,
            },
            osc2: OscillatorPreset {
                waveform: WaveformPreset::Sine,
                detune: 0.0,
                octave: 2,
            },
            osc_mix: 0.0,
            osc_mod: OscModPreset {
                mode: OscModModePreset::Fm,
                amount: 0.4,
            },
            filter: FilterPreset {
                cutoff: 8000.0,
                resonance: 0.1,
                mode: FilterModePreset::LowPass,
                drive: 0.0,
                env_amount: 0.0,
                key_track: 0.8,
                gain_db: 0.0,
            },
            amp_env: EnvelopePreset {
                attack: 0.001,
                decay: 1.5,
                sustain: 0.0,
                release: 1.0,
                curve: -0.7,
            },
            filter_env: EnvelopePreset::default(),
            lfo: LfoPreset::default(),
            effects: EffectsPreset {
                reverb_size: 0.6,
                reverb_mix: 0.3,
                ..Default::default()
            },
        },
        // Sync Lead
        Preset {
            name: "Sync Lead".to_string(),
            version: PRESET_VERSION,
            author: Some("Katie".to_string()),
            category: Some("Lead".to_string()),
            osc1: OscillatorPreset {
                waveform: WaveformPreset::Saw,
                detune: 0.0,
                octave: 0,
            },
            osc2: OscillatorPreset {
                waveform: WaveformPreset::Saw,
                detune: 0.0,
                octave: 1,
            },
            osc_mix: 0.5,
            osc_mod: OscModPreset {
                mode: OscModModePreset::Sync,
                amount: 0.0,
            },
            filter: FilterPreset {
                cutoff: 3000.0,
                resonance: 0.5,
                mode: FilterModePreset::LowPass,
                drive: 0.3,
                env_amount: 0.6,
                key_track: 0.5,
                gain_db: 0.0,
            },
            amp_env: EnvelopePreset {
                attack: 0.01,
                decay: 0.2,
                sustain: 0.7,
                release: 0.3,
                curve: 0.0,
            },
            filter_env: EnvelopePreset {
                attack: 0.01,
                decay: 0.3,
                sustain: 0.3,
                release: 0.3,
                curve: -0.3,
            },
            lfo: LfoPreset {
                waveform: WaveformPreset::Triangle,
                rate: 5.0,
                sync: LfoSyncPreset::Free,
                to_pitch: 0.1,
                to_filter: 0.0,
                to_amp: 0.0,
            },
            effects: EffectsPreset {
                delay_time: 0.25,
                delay_feedback: 0.3,
                delay_mix: 0.2,
                ..Default::default()
            },
        },
        // Pluck
        Preset {
            name: "Pluck".to_string(),
            version: PRESET_VERSION,
            author: Some("Katie".to_string()),
            category: Some("Pluck".to_string()),
            osc1: OscillatorPreset {
                waveform: WaveformPreset::Triangle,
                detune: 0.0,
                octave: 0,
            },
            osc2: OscillatorPreset {
                waveform: WaveformPreset::Square { pulse_width: 0.3 },
                detune: 0.0,
                octave: 1,
            },
            osc_mix: 0.3,
            osc_mod: OscModPreset::default(),
            filter: FilterPreset {
                cutoff: 4000.0,
                resonance: 0.5,
                mode: FilterModePreset::LowPass,
                drive: 0.0,
                env_amount: 0.8,
                key_track: 0.7,
                gain_db: 0.0,
            },
            amp_env: EnvelopePreset {
                attack: 0.001,
                decay: 0.5,
                sustain: 0.0,
                release: 0.3,
                curve: -0.5,
            },
            filter_env: EnvelopePreset {
                attack: 0.001,
                decay: 0.3,
                sustain: 0.0,
                release: 0.2,
                curve: -0.7,
            },
            lfo: LfoPreset::default(),
            effects: EffectsPreset {
                delay_time: 0.25,
                delay_feedback: 0.3,
                delay_mix: 0.2,
                ..Default::default()
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_preset_default() {
        let preset = Preset::default();
        assert_eq!(preset.name, "Init");
    }

    #[test]
    fn test_preset_to_params() {
        let preset = Preset::default();
        let params = preset.to_params();
        assert_eq!(params.osc_mix, 0.5);
        assert_eq!(params.osc_mod_mode, OscModMode::None);
    }

    #[test]
    fn test_preset_from_params() {
        let params = VoiceParams::default();
        let preset = Preset::from_params("Test", &params);
        assert_eq!(preset.name, "Test");
        assert_eq!(preset.osc_mix, params.osc_mix);
    }

    #[test]
    fn test_preset_save_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        let preset = Preset::default();
        preset.save(&path).unwrap();

        let loaded = Preset::load(&path).unwrap();
        assert_eq!(loaded.name, preset.name);
    }

    #[test]
    fn test_builtin_presets() {
        let presets = builtin_presets();
        assert!(presets.len() >= 5);
        assert_eq!(presets[0].name, "Init");
    }

    #[test]
    fn test_waveform_conversion() {
        let preset_wf = WaveformPreset::Square { pulse_width: 0.3 };
        let waveform: Waveform = preset_wf.into();
        match waveform {
            Waveform::Square { pulse_width } => assert!((pulse_width - 0.3).abs() < 0.01),
            _ => panic!("Wrong waveform type"),
        }
    }

    #[test]
    fn test_osc_mod_mode_conversion() {
        let preset = OscModModePreset::Fm;
        let mode: OscModMode = preset.into();
        assert_eq!(mode, OscModMode::Fm);
    }

    #[test]
    fn test_lfo_sync_conversion() {
        let preset = LfoSyncPreset::BpmSync { division: 0.5 };
        let sync: LfoSync = preset.into();
        assert_eq!(sync, LfoSync::BpmSync { division: 0.5 });
    }

    #[test]
    fn test_envelope_curve_in_preset() {
        let preset = Preset::default();
        assert_eq!(preset.amp_env.curve, 0.0);
        
        let mut params = VoiceParams::default();
        params.amp_curve = -0.5;
        let preset = Preset::from_params("Test", &params);
        assert_eq!(preset.amp_env.curve, -0.5);
    }

    #[test]
    fn test_filter_drive_in_preset() {
        let preset = Preset::default();
        assert_eq!(preset.filter.drive, 0.0);
        
        let mut params = VoiceParams::default();
        params.filter_drive = 0.5;
        let preset = Preset::from_params("Test", &params);
        assert_eq!(preset.filter.drive, 0.5);
    }

    #[test]
    fn test_lfo_preset() {
        let preset = LfoPreset {
            waveform: WaveformPreset::Triangle,
            rate: 2.0,
            sync: LfoSyncPreset::KeySync,
            to_pitch: 0.5,
            to_filter: 1000.0,
            to_amp: 0.2,
        };
        assert_eq!(preset.rate, 2.0);
    }
}
