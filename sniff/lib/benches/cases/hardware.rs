//! Hardware-detection Criterion benches.
//!
//! Isolates the platform-specific paths the design calls out as
//! historically slow (audio enumeration, GPU detection) and exposes
//! them alongside the cheap SIMD / summary paths.

use criterion::{Criterion, black_box};
use sniff::hardware::{detect_hardware_summary, detect_hardware_with_request, detect_simd};
use sniff::request::HardwareRequest;

use crate::support::util;

pub fn register(c: &mut Criterion) {
    let mut group = util::configure_group(c, "hardware");

    group.bench_function("detect_simd", |b| {
        b.iter(|| {
            let simd = detect_simd();
            black_box(simd);
        });
    });

    group.bench_function("detect_hardware_summary", |b| {
        b.iter(|| {
            let result = detect_hardware_summary().unwrap();
            black_box(result);
        });
    });

    group.bench_function("detect_storage_only", |b| {
        let req = HardwareRequest::summary().include_storage(true);
        b.iter(|| {
            let result = detect_hardware_with_request(black_box(&req)).unwrap();
            black_box(result);
        });
    });

    group.bench_function("detect_gpus_only", |b| {
        let req = HardwareRequest::summary().include_gpu(true);
        b.iter(|| {
            let result = detect_hardware_with_request(black_box(&req)).unwrap();
            black_box(result);
        });
    });

    // Audio enumeration is only a meaningful cost on macOS (Core Audio)
    // and Linux (ALSA/PulseAudio). Non-macOS, non-Linux builds still run
    // the bench but the path is effectively a no-op stub.
    group.bench_function("detect_audio_only", |b| {
        let req = HardwareRequest::summary().include_audio(true);
        b.iter(|| {
            let result = detect_hardware_with_request(black_box(&req)).unwrap();
            black_box(result);
        });
    });

    group.finish();
}
