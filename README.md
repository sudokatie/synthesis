# synthesis

A modular synthesizer engine in Rust. Because sometimes you need to hear what a Moog ladder filter sounds like at 3 AM.

## Why This Exists?

Most audio libraries give you the building blocks but make you figure out how to wire them together. Synthesis gives you a complete signal path: oscillators through filters through effects, with a modulation matrix that doesn't require a PhD to understand. It's opinionated about architecture so you can focus on making sounds instead of fighting abstractions.

## Features

- **Dual oscillators** with saw, square, sine, triangle, noise, and wavetable modes
- **Anti-aliased** via PolyBLEP (no more aliasing artifacts at high frequencies)
- **FM and PM synthesis** for those classic DX7-style sounds
- **Filters**: State variable (LP/HP/BP/Notch) and Moog ladder with self-oscillation
- **ADSR envelopes** for amplitude and filter cutoff
- **LFO** with multiple waveforms and unipolar/bipolar modes
- **Modulation matrix** for routing any source to any destination
- **Voice management**: polyphonic, monophonic, and legato modes
- **Voice stealing**: oldest, lowest, highest, or quietest
- **Effects**: stereo delay, Schroeder reverb, chorus, distortion, limiter
- **MIDI input** with full message parsing and state tracking
- **JSON presets** with built-in starter patches

## Quick Start

```rust
use synthesis::prelude::*;

// Create engine with default settings
let mut engine = Engine::new(EngineConfig::default());

// Load a preset
let preset = builtin_presets()[1]; // Classic Bass
engine.load_preset(&preset);

// Play a note
engine.note_on(60, 0.8); // Middle C at velocity 0.8

// Process audio
let mut buffer = vec![0.0; 256];
engine.process(&mut buffer);
// buffer now contains synthesized audio
```

## Usage

### Basic Synthesis

```rust
use synthesis::prelude::*;

let mut engine = Engine::new(EngineConfig {
    sample_rate: 44100,
    buffer_size: 256,
    max_voices: 8,
});

// Configure the sound
let mut params = VoiceParams::default();
params.osc1_waveform = Waveform::Saw;
params.filter_cutoff = 2000.0;
params.filter_resonance = 0.5;
engine.set_params(params);

// Add effects
engine.set_delay(0.25, 0.4, 0.3);  // time, feedback, mix
engine.set_reverb(0.8, 0.5, 0.2);  // room size, damping, mix
```

### MIDI Input

```rust
use synthesis::prelude::*;

let mut engine = Engine::new(EngineConfig::default());

// Process raw MIDI bytes
engine.process_midi(&[0x90, 60, 100]); // Note on: C4, velocity 100
engine.process_midi(&[0x80, 60, 0]);   // Note off: C4

// Or use parsed messages
engine.process_midi_message(MidiMessage::NoteOn {
    channel: 0,
    note: 60,
    velocity: 100,
});
```

### Presets

```rust
use synthesis::preset::{Preset, builtin_presets};

// Load built-in preset
let presets = builtin_presets();
engine.load_preset(&presets[2]); // "Soft Pad"

// Load from file
let preset = Preset::load("my_preset.json")?;
engine.load_preset(&preset);

// Save current settings
let preset = Preset::from_params("My Sound", engine.params());
preset.save("my_sound.json")?;
```

## Architecture

```
Oscillator 1 ─┬─> Mix ─> Filter ─> Amp Envelope ─┬─> Chorus
Oscillator 2 ─┘                                  ├─> Delay
                                                 ├─> Reverb
Filter Envelope ─> Filter Cutoff                 └─> Limiter ─> Output

LFO ─> Modulation Matrix ─> [any parameter]
```

Each voice contains the full signal path. The engine manages voice allocation, MIDI state, and the global effects chain.

## Philosophy

1. **Complete, not minimal** - Everything you need for subtractive synthesis in one crate
2. **Fast by default** - PolyBLEP anti-aliasing, efficient filter algorithms, no heap allocations in the audio path
3. **Opinionated routing** - The modulation matrix is flexible, but the signal path is fixed. Less rope to hang yourself with.
4. **Presets are data** - JSON presets mean you can version control your sounds

## License

MIT

---

*Built during a series of sleepless nights. The filter resonance is not responsible for any tinnitus.*
