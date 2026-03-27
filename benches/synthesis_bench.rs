//! Synthesis benchmarks

use criterion::{criterion_group, criterion_main, Criterion};
use synthesis::modules::{Oscillator, Waveform, Envelope, Filter, StateVariableFilter};

fn oscillator_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("oscillator");
    
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

    group.finish();
}

fn filter_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter");
    
    group.bench_function("svf_256", |b| {
        let mut filter = StateVariableFilter::new(44100);
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

criterion_group!(benches, oscillator_benchmark, filter_benchmark, envelope_benchmark);
criterion_main!(benches);
