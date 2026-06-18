//! Criterion benchmarks for tree-hugger symbol extraction.
//!
//! Covers parsing and query hot paths for representative source files.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::io::Write;

// ---------------------------------------------------------------------------
// Stress-input generators
// ---------------------------------------------------------------------------

fn rust_source() -> String {
    let mut out = String::from("use std::collections::HashMap;\n\n");
    for i in 0..100 {
        out.push_str(&format!(
            "pub fn function_{i}(x: usize) -> usize {{\n    x + {i}\n}}\n\n"
        ));
    }
    out.push_str("pub struct MyStruct {\n    pub field: String,\n}\n\n");
    out.push_str("impl MyStruct {\n    pub fn new() -> Self {\n        Self { field: String::new() }\n    }\n}\n");
    out
}

fn python_source() -> String {
    let mut out = String::from("import os\nimport sys\n\n");
    for i in 0..100 {
        out.push_str(&format!("def function_{i}(x):\n    return x + {i}\n\n"));
    }
    out.push_str("class MyClass:\n    def __init__(self):\n        self.field = ''\n");
    out
}

fn javascript_source() -> String {
    let mut out = String::from("import {{ useState }} from 'react';\n\n");
    for i in 0..100 {
        out.push_str(&format!(
            "export function function_{i}(x) {{\n    return x + {i};\n}}\n\n"
        ));
    }
    out.push_str(
        "export class MyClass {\n    constructor() {\n        this.field = '';\n    }\n}\n",
    );
    out
}

// ---------------------------------------------------------------------------
// Helper: write source to a temp file and parse it
// ---------------------------------------------------------------------------

fn parse_temp(source: &str, extension: &str) -> tree_hugger::TreeFile {
    let mut tmpfile = tempfile::NamedTempFile::with_suffix(extension).unwrap();
    tmpfile.write_all(source.as_bytes()).unwrap();
    tmpfile.flush().unwrap();
    tree_hugger::TreeFile::new(tmpfile.path()).unwrap()
}

// ---------------------------------------------------------------------------
// Parse benchmarks
// ---------------------------------------------------------------------------

fn bench_parse_rust(c: &mut Criterion) {
    let source = rust_source();
    let mut group = c.benchmark_group("parse");
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("rust", |b| {
        b.iter(|| {
            let file = parse_temp(black_box(&source), ".rs");
            black_box(file);
        })
    });
    group.finish();
}

fn bench_parse_python(c: &mut Criterion) {
    let source = python_source();
    let mut group = c.benchmark_group("parse");
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("python", |b| {
        b.iter(|| {
            let file = parse_temp(black_box(&source), ".py");
            black_box(file);
        })
    });
    group.finish();
}

fn bench_parse_javascript(c: &mut Criterion) {
    let source = javascript_source();
    let mut group = c.benchmark_group("parse");
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("javascript", |b| {
        b.iter(|| {
            let file = parse_temp(black_box(&source), ".js");
            black_box(file);
        })
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Symbol extraction benchmarks
// ---------------------------------------------------------------------------

fn bench_symbols_rust(c: &mut Criterion) {
    let source = rust_source();
    let file = parse_temp(&source, ".rs");
    let mut group = c.benchmark_group("symbols");
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("rust", |b| b.iter(|| black_box(&file).symbols().unwrap()));
    group.finish();
}

fn bench_imports_rust(c: &mut Criterion) {
    let source = rust_source();
    let file = parse_temp(&source, ".rs");
    let mut group = c.benchmark_group("imports");
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("rust", |b| {
        b.iter(|| black_box(&file).imported_symbols().unwrap())
    });
    group.finish();
}

fn bench_exports_rust(c: &mut Criterion) {
    let source = rust_source();
    let file = parse_temp(&source, ".rs");
    let mut group = c.benchmark_group("exports");
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("rust", |b| {
        b.iter(|| black_box(&file).exported_symbols().unwrap())
    });
    group.finish();
}

fn bench_lint_diagnostics_rust(c: &mut Criterion) {
    let source = rust_source();
    let file = parse_temp(&source, ".rs");
    let mut group = c.benchmark_group("lint");
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("rust", |b| b.iter(|| black_box(&file).lint_diagnostics()));
    group.finish();
}

criterion_group!(
    benches,
    bench_parse_rust,
    bench_parse_python,
    bench_parse_javascript,
    bench_symbols_rust,
    bench_imports_rust,
    bench_exports_rust,
    bench_lint_diagnostics_rust,
);
criterion_main!(benches);
