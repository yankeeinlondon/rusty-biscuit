//! Criterion benchmarks for biscuit-hash algorithms.
//!
//! Covers the hot paths for xxHash, BLAKE3, and variant hashing.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

// ---------------------------------------------------------------------------
// Input generators
// ---------------------------------------------------------------------------

fn small_text() -> String {
    "Hello, World!".to_string()
}

fn medium_text() -> String {
    "The quick brown fox jumps over the lazy dog. ".repeat(100)
}

fn large_text() -> String {
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(10_000)
}

// ---------------------------------------------------------------------------
// xxHash benchmarks
// ---------------------------------------------------------------------------

fn bench_xx_hash(c: &mut Criterion) {
    let small = small_text();
    let medium = medium_text();
    let large = large_text();

    let mut group = c.benchmark_group("xx_hash");
    group.throughput(Throughput::Bytes(small.len() as u64));
    group.bench_function("small", |b| {
        b.iter(|| biscuit_hash::xx_hash(black_box(&small)))
    });

    group.throughput(Throughput::Bytes(medium.len() as u64));
    group.bench_function("medium", |b| {
        b.iter(|| biscuit_hash::xx_hash(black_box(&medium)))
    });

    group.throughput(Throughput::Bytes(large.len() as u64));
    group.bench_function("large", |b| {
        b.iter(|| biscuit_hash::xx_hash(black_box(&large)))
    });
    group.finish();
}

fn bench_xx_hash_bytes(c: &mut Criterion) {
    let large = large_text();
    let bytes = large.as_bytes();

    let mut group = c.benchmark_group("xx_hash_bytes");
    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_function("large", |b| {
        b.iter(|| biscuit_hash::xx_hash_bytes(black_box(bytes)))
    });
    group.finish();
}

fn bench_xx_hash_variant(c: &mut Criterion) {
    let text = medium_text();
    let variants = vec![
        biscuit_hash::HashVariant::BlockTrimming,
        biscuit_hash::HashVariant::BlankLine,
        biscuit_hash::HashVariant::LeadingWhitespace,
        biscuit_hash::HashVariant::TrailingWhitespace,
    ];

    let mut group = c.benchmark_group("xx_hash_variant");
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("medium_4_variants", |b| {
        b.iter(|| biscuit_hash::xx_hash_variant(black_box(&text), black_box(variants.clone())))
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// BLAKE3 benchmarks
// ---------------------------------------------------------------------------

#[cfg(feature = "blake3")]
fn bench_blake3_hash(c: &mut Criterion) {
    let small = small_text();
    let medium = medium_text();
    let large = large_text();

    let mut group = c.benchmark_group("blake3_hash");
    group.throughput(Throughput::Bytes(small.len() as u64));
    group.bench_function("small", |b| {
        b.iter(|| biscuit_hash::blake3_hash(black_box(&small)))
    });

    group.throughput(Throughput::Bytes(medium.len() as u64));
    group.bench_function("medium", |b| {
        b.iter(|| biscuit_hash::blake3_hash(black_box(&medium)))
    });

    group.throughput(Throughput::Bytes(large.len() as u64));
    group.bench_function("large", |b| {
        b.iter(|| biscuit_hash::blake3_hash(black_box(&large)))
    });
    group.finish();
}

#[cfg(feature = "blake3")]
fn bench_blake3_hash_bytes(c: &mut Criterion) {
    let large = large_text();
    let bytes = large.as_bytes();

    let mut group = c.benchmark_group("blake3_hash_bytes");
    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_function("large", |b| {
        b.iter(|| biscuit_hash::blake3_hash_bytes(black_box(bytes)))
    });
    group.finish();
}

#[cfg(not(feature = "blake3"))]
fn bench_blake3_hash(_c: &mut Criterion) {}

#[cfg(not(feature = "blake3"))]
fn bench_blake3_hash_bytes(_c: &mut Criterion) {}

criterion_group!(
    benches,
    bench_xx_hash,
    bench_xx_hash_bytes,
    bench_xx_hash_variant,
    bench_blake3_hash,
    bench_blake3_hash_bytes,
);
criterion_main!(benches);
