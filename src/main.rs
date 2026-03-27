//! Synthesis CLI

use clap::{Parser, Subcommand};
use synthesis::audio::AudioOutput;

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
    /// Run synthesizer
    Run {
        /// MIDI device name
        #[arg(short, long)]
        midi: Option<String>,
        /// Audio output device
        #[arg(short, long)]
        output: Option<String>,
        /// Preset file
        #[arg(short, long)]
        preset: Option<String>,
    },
    /// Run benchmarks
    Bench,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Devices => {
            println!("Audio devices:");
            match AudioOutput::list_devices() {
                Ok(devices) => {
                    for device in devices {
                        println!("  - {}", device);
                    }
                }
                Err(e) => {
                    eprintln!("Error listing devices: {}", e);
                }
            }
        }
        Commands::Run { midi, output, preset } => {
            println!("Starting synthesizer...");
            if let Some(m) = midi {
                println!("  MIDI: {}", m);
            }
            if let Some(o) = output {
                println!("  Output: {}", o);
            }
            if let Some(p) = preset {
                println!("  Preset: {}", p);
            }
            // TODO: Start audio engine
        }
        Commands::Bench => {
            println!("Run: cargo bench");
        }
    }
}
