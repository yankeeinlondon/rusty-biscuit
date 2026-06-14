//! Criterion benchmarks for biscuit-file format parsers.
//!
//! Covers the hot paths for TOML, YAML, and JSON5 parsing and conversion.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

// ---------------------------------------------------------------------------
// Stress-input generators
// ---------------------------------------------------------------------------

fn large_toml() -> String {
    let mut out = String::from("[package]\nname = \"example\"\nversion = \"1.0.0\"\n\n");
    for i in 0..200 {
        out.push_str(&format!(
            "[[dependency]]\nname = \"dep-{i}\"\nversion = \"^{i}.0.0\"\noptional = {}\nfeatures = [\"foo\", \"bar\"]\n\n",
            i % 2 == 0
        ));
    }
    out
}

fn large_yaml() -> String {
    let mut out = String::from("name: example\nversion: '1.0.0'\n\n");
    out.push_str("dependencies:\n");
    for i in 0..200 {
        out.push_str(&format!(
            "  dep-{i}:\n    version: \"^{i}.0.0\"\n    optional: {}\n    features:\n      - foo\n      - bar\n",
            i % 2 == 0
        ));
    }
    out
}

fn large_json5() -> String {
    let mut out = String::from("{\n  name: 'example',\n  version: '1.0.0',\n  dependencies: {\n");
    for i in 0..200 {
        out.push_str(&format!(
            "    'dep-{i}': {{ version: '^{i}.0.0', optional: {}, features: ['foo', 'bar'] }},\n",
            i % 2 == 0
        ));
    }
    out.push_str("  }\n}\n");
    out
}

// ---------------------------------------------------------------------------
// TOML benchmarks
// ---------------------------------------------------------------------------

fn bench_toml_parse(c: &mut Criterion) {
    let input = large_toml();
    let mut group = c.benchmark_group("toml");
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("parse", |b| {
        b.iter(|| biscuit_file::Toml::from_str(black_box(&input)).unwrap())
    });
    group.finish();
}

fn bench_toml_to_json(c: &mut Criterion) {
    let input = large_toml();
    let doc = biscuit_file::Toml::from_str(&input).unwrap();
    let mut group = c.benchmark_group("toml");
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("to_json", |b| b.iter(|| black_box(&doc).as_json().unwrap()));
    group.finish();
}

fn bench_toml_to_yaml(c: &mut Criterion) {
    let input = large_toml();
    let doc = biscuit_file::Toml::from_str(&input).unwrap();
    let mut group = c.benchmark_group("toml");
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("to_yaml", |b| b.iter(|| black_box(&doc).as_yaml().unwrap()));
    group.finish();
}

// ---------------------------------------------------------------------------
// YAML benchmarks
// ---------------------------------------------------------------------------

fn bench_yaml_parse(c: &mut Criterion) {
    let input = large_yaml();
    let mut group = c.benchmark_group("yaml");
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("parse", |b| {
        b.iter(|| biscuit_file::Yaml::from_str(black_box(&input)).unwrap())
    });
    group.finish();
}

fn bench_yaml_to_json(c: &mut Criterion) {
    let input = large_yaml();
    let doc = biscuit_file::Yaml::from_str(&input).unwrap();
    let mut group = c.benchmark_group("yaml");
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("to_json", |b| b.iter(|| black_box(&doc).as_json().unwrap()));
    group.finish();
}

fn bench_yaml_to_toml(c: &mut Criterion) {
    let input = large_yaml();
    let doc = biscuit_file::Yaml::from_str(&input).unwrap();
    let mut group = c.benchmark_group("yaml");
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("to_toml", |b| b.iter(|| black_box(&doc).as_toml().unwrap()));
    group.finish();
}

// ---------------------------------------------------------------------------
// JSON5 benchmarks
// ---------------------------------------------------------------------------

fn bench_json5_parse(c: &mut Criterion) {
    let input = large_json5();
    let mut group = c.benchmark_group("json5");
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("parse", |b| {
        b.iter(|| biscuit_file::Json5::from_str(black_box(&input)).unwrap())
    });
    group.finish();
}

fn bench_json5_to_json(c: &mut Criterion) {
    let input = large_json5();
    let doc = biscuit_file::Json5::from_str(&input).unwrap();
    let mut group = c.benchmark_group("json5");
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("to_json", |b| b.iter(|| black_box(&doc).as_json().unwrap()));
    group.finish();
}

fn bench_json5_to_toml(c: &mut Criterion) {
    let input = large_json5();
    let doc = biscuit_file::Json5::from_str(&input).unwrap();
    let mut group = c.benchmark_group("json5");
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("to_toml", |b| b.iter(|| black_box(&doc).as_toml().unwrap()));
    group.finish();
}

criterion_group!(
    benches,
    bench_toml_parse,
    bench_toml_to_json,
    bench_toml_to_yaml,
    bench_yaml_parse,
    bench_yaml_to_json,
    bench_yaml_to_toml,
    bench_json5_parse,
    bench_json5_to_json,
    bench_json5_to_toml,
);
criterion_main!(benches);
