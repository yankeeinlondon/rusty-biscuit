//! Hardware-detection Criterion benches.
//!
//! The suite is split into two layers so regressions can be pinned
//! precisely:
//!
//! - `hardware_leaf` — calls each platform-specific leaf detector
//!   directly (`detect_audio_devices`, `detect_gpus`, `detect_storage`).
//!   These numbers describe raw Core Audio / IOKit / `/sys` / Windows
//!   WMI latency without any CPU or memory setup work.
//! - `hardware` — calls `detect_hardware_with_request` with different
//!   request shapes so the request-level orchestration, CPU info, and
//!   memory discovery cost is still measured end to end.

use criterion::{Criterion, black_box};
use sniff::hardware::{
    detect_audio_devices, detect_gpus, detect_hardware_summary, detect_hardware_with_request,
    detect_simd, detect_storage,
};
use sniff::request::HardwareRequest;

use crate::support::util;

pub fn register(c: &mut Criterion) {
    // ---------- leaf ----------
    let mut leaf_group = util::configure_group(c, "hardware_leaf");

    leaf_group.bench_function("detect_audio_devices", |b| {
        b.iter(|| {
            let devices = detect_audio_devices();
            black_box(devices);
        });
    });

    leaf_group.bench_function("detect_gpus", |b| {
        b.iter(|| {
            let gpus = detect_gpus();
            black_box(gpus);
        });
    });

    leaf_group.bench_function("detect_storage", |b| {
        b.iter(|| {
            let storage = detect_storage();
            black_box(storage);
        });
    });

    leaf_group.finish();

    // ---------- request-level ----------
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

    group.bench_function("detect_hardware_full", |b| {
        let req = HardwareRequest::full();
        b.iter(|| {
            let result = detect_hardware_with_request(black_box(&req)).unwrap();
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
