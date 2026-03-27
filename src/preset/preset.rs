//! Preset structure and serialization

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::engine::{PlayMode, StealMode, VoiceParams};
use crate::modules::{FilterMode, Waveform};

/// Serializable waveform type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaveformPreset {
    Sine,
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
            Waveform::Wavetable { .. } => WaveformPreset::Saw, // Default for wavetable
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
}

impl From<FilterModePreset> for FilterMode {
    fn from(preset: FilterModePreset) -> Self {
        match preset {
            FilterModePreset::LowPass => FilterMode::LowPass,
            FilterModePreset::HighPass => FilterMode::HighPass,
            FilterModePreset::BandPass => FilterMode::BandPass,
            FilterModePreset::Notch => FilterMode::Notch,
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

/// Filter settings in preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPreset {
    pub cutoff: f32,
    pub resonance: f32,
    pub mode: FilterModePreset,
    pub env_amount: f32,
    pub key_track: f32,
}

impl Default for FilterPreset {
    fn default() -> Self {
        Self {
            cutoff: 8000.0,
            resonance: 0.3,
            mode: FilterModePreset::LowPass,
            env_amount: 0.5,
            key_track: 0.5,
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
}

impl Default for EnvelopePreset {
    fn default() -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.3,
        }
    }
}

/// Complete synthesizer preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub author: Option<String>,
    pub category: Option<String>,
    pub osc1: OscillatorPreset,
    pub osc2: OscillatorPreset,
    pub osc_mix: f32,
    pub filter: FilterPreset,
    pub amp_env: EnvelopePreset,
    pub filter_env: EnvelopePreset,
    #[serde(default)]
    pub effects: EffectsPreset,
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
    pub reverb_mix: f32,
    #[serde(default)]
    pub chorus_rate: f32,
    #[serde(default)]
    pub chorus_depth: f32,
    #[serde(default)]
    pub chorus_mix: f32,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            name: "Init".to_string(),
            author: None,
            category: None,
            osc1: OscillatorPreset::default(),
            osc2: OscillatorPreset {
                waveform: WaveformPreset::Saw,
                detune: 5.0,
                octave: 0,
            },
            osc_mix: 0.5,
            filter: FilterPreset::default(),
            amp_env: EnvelopePreset::default(),
            filter_env: EnvelopePreset {
                attack: 0.01,
                decay: 0.2,
                sustain: 0.3,
                release: 0.5,
            },
            effects: EffectsPreset::default(),
        }
    }
}

impl Preset {
    /// Create preset from voice parameters
    pub fn from_params(name: &str, params: &VoiceParams) -> Self {
        Self {
            name: name.to_string(),
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
            filter: FilterPreset {
                cutoff: params.filter_cutoff,
                resonance: params.filter_resonance,
                mode: FilterModePreset::from(params.filter_mode),
                env_amount: params.filter_env_amount,
                key_track: params.filter_key_track,
            },
            amp_env: EnvelopePreset {
                attack: params.amp_attack,
                decay: params.amp_decay,
                sustain: params.amp_sustain,
                release: params.amp_release,
            },
            filter_env: EnvelopePreset {
                attack: params.filter_attack,
                decay: params.filter_decay,
                sustain: params.filter_sustain,
                release: params.filter_release,
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
            filter_cutoff: self.filter.cutoff,
            filter_resonance: self.filter.resonance,
            filter_mode: self.filter.mode.into(),
            filter_env_amount: self.filter.env_amount,
            filter_key_track: self.filter.key_track,
            amp_attack: self.amp_env.attack,
            amp_decay: self.amp_env.decay,
            amp_sustain: self.amp_env.sustain,
            amp_release: self.amp_env.release,
            filter_attack: self.filter_env.attack,
            filter_decay: self.filter_env.decay,
            filter_sustain: self.filter_env.sustain,
            filter_release: self.filter_env.release,
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
            filter: FilterPreset {
                cutoff: 500.0,
                resonance: 0.4,
                mode: FilterModePreset::LowPass,
                env_amount: 0.7,
                key_track: 0.3,
            },
            amp_env: EnvelopePreset {
                attack: 0.001,
                decay: 0.3,
                sustain: 0.6,
                release: 0.2,
            },
            filter_env: EnvelopePreset {
                attack: 0.001,
                decay: 0.4,
                sustain: 0.2,
                release: 0.3,
            },
            effects: EffectsPreset::default(),
        },
        // Soft Pad
        Preset {
            name: "Soft Pad".to_string(),
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
            filter: FilterPreset {
                cutoff: 2000.0,
                resonance: 0.2,
                mode: FilterModePreset::LowPass,
                env_amount: 0.3,
                key_track: 0.5,
            },
            amp_env: EnvelopePreset {
                attack: 0.5,
                decay: 0.3,
                sustain: 0.8,
                release: 1.0,
            },
            filter_env: EnvelopePreset {
                attack: 0.8,
                decay: 0.5,
                sustain: 0.5,
                release: 1.0,
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
        // Pluck
        Preset {
            name: "Pluck".to_string(),
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
            filter: FilterPreset {
                cutoff: 4000.0,
                resonance: 0.5,
                mode: FilterModePreset::LowPass,
                env_amount: 0.8,
                key_track: 0.7,
            },
            amp_env: EnvelopePreset {
                attack: 0.001,
                decay: 0.5,
                sustain: 0.0,
                release: 0.3,
            },
            filter_env: EnvelopePreset {
                attack: 0.001,
                decay: 0.3,
                sustain: 0.0,
                release: 0.2,
            },
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
        assert!(presets.len() >= 4);
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
}
