//! Synthesis benchmarks

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use synthesis::engine::{Engine, EngineConfig, VoiceParams};
use synthesis::modules::{Envelope, Filter, Oscillator, StateVariableFilter, Waveform};
use synthesis::effects::{Delay, SchroederReverb, Chorus, Compressor};
use synthesis::preset::builtin_presets;

fn oscillator_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("oscillator");
    group.sample_size(1000);

    group.bench_function("sine_256", |b| {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        let mut buffer = vec![0.0; 256];
        b.iter(|| {
            osc.process(&mut buffer);
        });
    });

    group.bench_function("saw_256", |b| {
        let mut osc = Oscillator::new(Waveform::Saw, 44100);
        let mut buffer = vec![0.0; 256];
        b.iter(|| {
            osc.process(&mut buffer);
        });
    });

    group.bench_function("square_256", |b| {
        let mut osc = Oscillator::new(Waveform::Square { pulse_width: 0.5 }, 44100);
        let mut buffer = vec![0.0; 256];
        b.iter(|| {
            osc.process(&mut buffer);
        });
    });

    group.finish();
}

fn filter_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter");
    group.sample_size(1000);

    group.bench_function("svf_256", |b| {
        let mut filter = StateVariableFilter::new(44100);
        filter.set_cutoff(2000.0);
        filter.set_resonance(0.5);
        b.iter(|| {
            for _ in 0..256 {
                filter.process_sample(0.5);
            }
        });
    });

    group.bench_function("moog_256", |b| {
        use synthesis::modules::MoogLadder;
        let mut filter = MoogLadder::new(44100);
        filter.set_cutoff(2000.0);
        filter.set_resonance(0.5);
        b.iter(|| {
            for _ in 0..256 {
                filter.process_sample(0.5);
            }
        });
    });

    group.finish();
}

fn envelope_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("envelope");
    group.sample_size(1000);

    group.bench_function("adsr_256", |b| {
        let mut env = Envelope::new(0.01, 0.1, 0.7, 0.5, 44100);
        let mut buffer = vec![0.0; 256];
        b.iter(|| {
            env.trigger();
            env.process(&mut buffer);
        });
    });

    group.finish();
}

fn effects_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("effects");
    group.sample_size(500);

    group.bench_function("delay_256", |b| {
        let mut delay = Delay::new(1.0, 44100);
        delay.set_time(0.25);
        delay.set_feedback(0.5);
        let mut buffer = vec![0.5; 256];
        b.iter(|| {
            delay.process(&mut buffer);
        });
    });

    group.bench_function("reverb_256", |b| {
        let mut reverb = SchroederReverb::new(44100);
        reverb.set_room_size(0.8);
        b.iter(|| {
            for _ in 0..256 {
                reverb.process_sample(0.5);
            }
        });
    });

    group.bench_function("chorus_256", |b| {
        let mut chorus = Chorus::new(44100);
        b.iter(|| {
            for _ in 0..256 {
                chorus.process_sample(0.5);
            }
        });
    });

    group.bench_function("compressor_256", |b| {
        let mut comp = Compressor::new(44100);
        b.iter(|| {
            for _ in 0..256 {
                comp.process_sample(0.5);
            }
        });
    });

    group.finish();
}

fn engine_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine");
    group.sample_size(500);

    // Benchmark with different voice counts
    for voices in [1, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("voices", voices),
            &voices,
            |b, &voice_count| {
                let config = EngineConfig {
                    sample_rate: 44100,
                    buffer_size: 256,
                    max_voices: 8,
                };
                let mut engine = Engine::new(config);

                // Trigger voices
                for i in 0..voice_count {
                    engine.note_on(60 + i as u8, 0.8);
                }

                let mut buffer = vec![0.0; 256];
                b.iter(|| {
                    engine.process(&mut buffer);
                });
            },
        );
    }

    group.finish();
}

fn engine_with_effects_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine_effects");
    group.sample_size(200);

    group.bench_function("8_voices_full_effects", |b| {
        let config = EngineConfig {
            sample_rate: 44100,
            buffer_size: 256,
            max_voices: 8,
        };
        let mut engine = Engine::new(config);

        // Load preset with effects
        let presets = builtin_presets();
        if let Some(pad) = presets.iter().find(|p| p.name == "Soft Pad") {
            engine.load_preset(pad);
        }

        // Enable all effects
        engine.set_delay(0.25, 0.5, 0.3);
        engine.set_reverb(0.8, 0.5, 0.4);
        engine.set_chorus(0.5, 0.5, 0.3);
        engine.set_distortion(3.0, 0.2);

        // Trigger 8 voices
        for i in 0..8 {
            engine.note_on(60 + i, 0.8);
        }

        let mut buffer = vec![0.0; 256];
        b.iter(|| {
            engine.process(&mut buffer);
        });
    });

    group.finish();
}

fn latency_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency");
    group.sample_size(1000);

    // Measure time to process different buffer sizes
    for buffer_size in [64, 128, 256, 512, 1024] {
        group.bench_with_input(
            BenchmarkId::new("buffer", buffer_size),
            &buffer_size,
            |b, &size| {
                let config = EngineConfig {
                    sample_rate: 44100,
                    buffer_size: size,
                    max_voices: 8,
                };
                let mut engine = Engine::new(config);

                // Trigger 4 voices
                for i in 0..4 {
                    engine.note_on(60 + i, 0.8);
                }

                let mut buffer = vec![0.0; size];
                b.iter(|| {
                    engine.process(&mut buffer);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    oscillator_benchmark,
    filter_benchmark,
    envelope_benchmark,
    effects_benchmark,
    engine_benchmark,
    engine_with_effects_benchmark,
    latency_benchmark,
);
criterion_main!(benches);
