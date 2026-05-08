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
    // ---------- leaf (fast) ----------
    //
    // GPU and storage enumeration are consistently fast on every
    // supported platform, so they share the standard leaf group.
    let mut leaf_group = util::configure_group(c, "hardware_leaf");

    leaf_group.bench_function("gpu_enumeration", |b| {
        b.iter(|| {
            let gpus = detect_gpus();
            black_box(gpus);
        });
    });

    leaf_group.bench_function("storage_enumeration", |b| {
        b.iter(|| {
            let storage = detect_storage();
            black_box(storage);
        });
    });

    leaf_group.finish();

    // ---------- leaf (audio, slow) ----------
    //
    // macOS Core Audio and some Linux ALSA setups can take well over
    // 1s per call, so this single-function group uses the slow config
    // (10 samples, 15s measurement budget) to keep Criterion from
    // panicking when a single iteration exceeds the default 10s.
    let mut audio_group = util::configure_slow_group(c, "hardware_leaf_audio");
    audio_group.bench_function("audio_device_enumeration", |b| {
        b.iter(|| {
            let devices = detect_audio_devices();
            black_box(devices);
        });
    });
    audio_group.finish();

    // ---------- request-level ----------
    let mut group = util::configure_group(c, "hardware");

    group.bench_function("simd_feature_detection", |b| {
        b.iter(|| {
            let simd = detect_simd();
            black_box(simd);
        });
    });

    group.bench_function("hardware_summary_cpu_memory", |b| {
        b.iter(|| {
            let result = detect_hardware_summary().unwrap();
            black_box(result);
        });
    });

    group.bench_function("hardware_full_all_subsystems", |b| {
        let req = HardwareRequest::full();
        b.iter(|| {
            let result = detect_hardware_with_request(black_box(&req)).unwrap();
            black_box(result);
        });
    });

    group.bench_function("hardware_request_storage_isolated", |b| {
        let req = HardwareRequest::summary().include_storage(true);
        b.iter(|| {
            let result = detect_hardware_with_request(black_box(&req)).unwrap();
            black_box(result);
        });
    });

    group.bench_function("hardware_request_gpu_isolated", |b| {
        let req = HardwareRequest::summary().include_gpu(true);
        b.iter(|| {
            let result = detect_hardware_with_request(black_box(&req)).unwrap();
            black_box(result);
        });
    });

    // Audio enumeration is only a meaningful cost on macOS (Core Audio)
    // and Linux (ALSA/PulseAudio). Non-macOS, non-Linux builds still run
    // the bench but the path is effectively a no-op stub.
    group.bench_function("hardware_request_audio_isolated", |b| {
        let req = HardwareRequest::summary().include_audio(true);
        b.iter(|| {
            let result = detect_hardware_with_request(black_box(&req)).unwrap();
            black_box(result);
        });
    });

    group.finish();
}
