//! Synthesis CLI

use clap::{Parser, Subcommand};
use synthesis::audio::AudioOutput;
use synthesis::engine::{Engine, EngineConfig};
use synthesis::midi::list_midi_inputs;
use synthesis::preset::{builtin_presets, Preset};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "synthesis")]
#[command(about = "Modular synthesizer engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List audio devices
    Devices,
    /// List MIDI input devices
    Midi,
    /// List built-in presets
    Presets,
    /// Run synthesizer
    Run {
        /// MIDI device name (partial match)
        #[arg(short, long)]
        midi: Option<String>,
        /// Audio output device (partial match)
        #[arg(short, long)]
        output: Option<String>,
        /// Preset file or built-in preset name
        #[arg(short, long)]
        preset: Option<String>,
        /// BPM for tempo-synced LFOs
        #[arg(short, long, default_value = "120")]
        bpm: f32,
    },
    /// Run benchmarks
    Bench,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Devices => {
            println!("Audio output devices:");
            match AudioOutput::list_devices() {
                Ok(devices) => {
                    if devices.is_empty() {
                        println!("  (no devices found)");
                    }
                    for device in devices {
                        println!("  - {}", device);
                    }
                }
                Err(e) => {
                    eprintln!("Error listing devices: {}", e);
                }
            }
            
            if let Ok(name) = AudioOutput::default_device_name() {
                println!("\nDefault device: {}", name);
            }
        }
        
        Commands::Midi => {
            println!("MIDI input devices:");
            match list_midi_inputs() {
                Ok(devices) => {
                    if devices.is_empty() {
                        println!("  (no devices found)");
                    }
                    for device in devices {
                        println!("  - {}", device);
                    }
                }
                Err(e) => {
                    eprintln!("Error listing MIDI devices: {}", e);
                }
            }
        }
        
        Commands::Presets => {
            println!("Built-in presets:");
            for preset in builtin_presets() {
                let category = preset.category.as_deref().unwrap_or("General");
                println!("  - {} [{}]", preset.name, category);
            }
        }
        
        Commands::Run { midi, output, preset, bpm } => {
            let running = Arc::new(AtomicBool::new(true));
            let r = running.clone();
            
            ctrlc::set_handler(move || {
                r.store(false, Ordering::SeqCst);
            }).expect("Error setting Ctrl-C handler");

            println!("Starting synthesizer...");
            
            let mut engine = Engine::new(EngineConfig::default());
            engine.set_bpm(bpm);
            println!("  BPM: {}", bpm);

            // Load preset
            if let Some(preset_name) = preset {
                // Try built-in preset first
                let builtin = builtin_presets();
                if let Some(p) = builtin.iter().find(|p| p.name.to_lowercase() == preset_name.to_lowercase()) {
                    engine.load_preset(p);
                    println!("  Loaded preset: {}", p.name);
                } else {
                    // Try file
                    match Preset::load(&preset_name) {
                        Ok(p) => {
                            engine.load_preset(&p);
                            println!("  Loaded preset: {}", p.name);
                        }
                        Err(e) => {
                            eprintln!("Warning: Could not load preset '{}': {}", preset_name, e);
                        }
                    }
                }
            }

            // Connect MIDI
            if let Some(midi_name) = midi {
                match engine.connect_midi(&midi_name) {
                    Ok(()) => println!("  MIDI: Connected to '{}'", midi_name),
                    Err(e) => eprintln!("Warning: Could not connect to MIDI '{}': {}", midi_name, e),
                }
            }

            // Start audio
            let audio_result = if let Some(ref output_name) = output {
                engine.start_audio_device(output_name)
            } else {
                engine.start_audio()
            };
            
            match audio_result {
                Ok(()) => {
                    println!("  Audio: Started (sample rate: {}Hz)", engine.sample_rate());
                }
                Err(e) => {
                    eprintln!("Error starting audio: {}", e);
                    return;
                }
            }

            println!("\nSynthesizer running. Press Ctrl-C to stop.");
            println!("Type note commands: 'c4' to play, 'c4 off' to release, 'q' to quit\n");

            // Main loop
            while running.load(Ordering::SeqCst) {
                // Poll MIDI
                engine.poll_midi();

                // Simple interactive input (non-blocking would be better)
                // For now, just sleep and poll
                thread::sleep(Duration::from_millis(10));
            }

            println!("\nStopping...");
            engine.stop_audio();
            engine.disconnect_midi();
            println!("Done.");
        }
        
        Commands::Bench => {
            println!("Running benchmarks...\n");
            
            let config = EngineConfig {
                sample_rate: 44100,
                buffer_size: 256,
                max_voices: 8,
            };
            let mut engine = Engine::new(config);
            
            // Load a preset with effects
            let presets = builtin_presets();
            if let Some(pad) = presets.iter().find(|p| p.name == "Soft Pad") {
                engine.load_preset(pad);
            }
            
            // Trigger all 8 voices
            for i in 0..8 {
                engine.note_on(60 + i, 0.8);
            }
            
            // Warm up
            let mut buffer = vec![0.0f32; 256];
            for _ in 0..100 {
                engine.process(&mut buffer);
            }
            
            // Benchmark
            let iterations = 10000;
            let start = std::time::Instant::now();
            
            for _ in 0..iterations {
                engine.process(&mut buffer);
            }
            
            let elapsed = start.elapsed();
            let samples_per_iter = 256;
            let total_samples = iterations * samples_per_iter;
            let samples_per_sec = total_samples as f64 / elapsed.as_secs_f64();
            let realtime_ratio = samples_per_sec / 44100.0;
            
            println!("Results:");
            println!("  Iterations: {}", iterations);
            println!("  Buffer size: {} samples", samples_per_iter);
            println!("  Total samples: {}", total_samples);
            println!("  Time: {:?}", elapsed);
            println!("  Throughput: {:.0} samples/sec", samples_per_sec);
            println!("  Real-time ratio: {:.1}x (need >1.0)", realtime_ratio);
            println!("  Voices: 8");
            
            // CPU estimate (very rough)
            let cpu_pct = 100.0 / realtime_ratio;
            println!("  Estimated CPU: {:.1}%", cpu_pct);
            
            if realtime_ratio > 1.0 {
                println!("\n✓ Performance target met!");
            } else {
                println!("\n✗ Below real-time - optimization needed");
            }
        }
    }
}
