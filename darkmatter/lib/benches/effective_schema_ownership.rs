//! Measures the ownership costs of assembling and cloning an effective schema.
//!
//! The baseline-only case is the path whose cost changes most when the public
//! JSON schema is owned as `Value` versus shared as `Arc<Value>`. The merge and
//! document-only controls distinguish that ownership cost from schema parsing,
//! validator lookup, and baseline merging.

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, darkmatter_base_json_schema, resolve::merge_baseline,
};
use serde_json::{Map, Value, json};
use std::hint::black_box;

const SYNTHETIC_PROPERTY_COUNT: usize = 512;

fn synthetic_baseline(property_count: usize) -> Value {
    let mut properties = Map::with_capacity(property_count);
    let mut required = Vec::with_capacity(property_count / 4);
    for index in 0..property_count {
        let name = format!("property_{index:04}");
        properties.insert(
            name.clone(),
            json!({
                "type": "string",
                "minLength": 1,
                "maxLength": 160,
                "description": format!("Synthetic benchmark property {index}"),
            }),
        );
        if index % 4 == 0 {
            required.push(Value::String(name));
        }
    }
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": true,
    })
}

fn baseline_only_document() -> Markdown {
    Markdown::from("---\ntitle: Ownership benchmark\n---\n# Benchmark\n")
}

fn document_schema() -> Markdown {
    Markdown::from(
        "---\n$schema:\n  benchmark-owned: 'string(required)'\nbenchmark-owned: value\n---\n# Benchmark\n",
    )
}

fn property_count(schema: &Value) -> u64 {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .map_or(0, |properties| properties.len() as u64)
}

fn bench_baseline(c: &mut Criterion, label: &str, baseline: Value) {
    let properties = property_count(&baseline);
    let baseline_only = baseline_only_document();
    let with_document_schema = document_schema();
    let api = DarkmatterSchemas::new()
        .with_baseline_json_schema(baseline)
        .expect("benchmark baseline must be valid");

    let effective = api
        .effective_for(&baseline_only)
        .expect("baseline-only assembly must succeed")
        .expect("configured baseline must produce an effective schema");
    api.effective_for(&with_document_schema)
        .expect("merged assembly must succeed")
        .expect("configured schemas must produce an effective schema");

    let mut group = c.benchmark_group(format!("effective_schema_ownership/{label}"));
    group.sample_size(100);
    group.throughput(Throughput::Elements(properties));

    group.bench_function("baseline_only", |b| {
        b.iter(|| {
            let effective = black_box(&api)
                .effective_for(black_box(&baseline_only))
                .expect("baseline-only assembly must succeed")
                .expect("configured baseline must produce an effective schema");
            black_box(effective);
        });
    });

    group.bench_function("baseline_plus_document", |b| {
        b.iter(|| {
            let effective = black_box(&api)
                .effective_for(black_box(&with_document_schema))
                .expect("merged assembly must succeed")
                .expect("configured schemas must produce an effective schema");
            black_box(effective);
        });
    });

    group.bench_function("clone_effective", |b| {
        b.iter(|| black_box((*black_box(&effective)).clone()));
    });

    group.finish();
}

fn bench_document_only(c: &mut Criterion) {
    let document = document_schema();
    let api = DarkmatterSchemas::new();
    api.effective_for(&document)
        .expect("document-only assembly must succeed")
        .expect("document schema must produce an effective schema");

    let mut group = c.benchmark_group("effective_schema_ownership/control");
    group.sample_size(100);
    group.throughput(Throughput::Elements(1));
    group.bench_function("document_only", |b| {
        b.iter(|| {
            let effective = black_box(&api)
                .effective_for(black_box(&document))
                .expect("document-only assembly must succeed")
                .expect("document schema must produce an effective schema");
            black_box(effective);
        });
    });
    group.finish();
}

fn bench_default_baseline_initialization(c: &mut Criterion) {
    let baseline = darkmatter_base_json_schema();
    let properties = property_count(&baseline);
    let baseline_only = baseline_only_document();
    let empty_document_schema = json!({"type": "object", "properties": {}});
    let tiny_document_schema = json!({
        "type": "object",
        "properties": {
            "benchmark-owned": {"type": "string"}
        },
        "required": ["benchmark-owned"]
    });

    let mut group = c.benchmark_group("effective_schema_initialization/darkmatter_baseline");
    group.sample_size(100);
    group.throughput(Throughput::Elements(properties));

    group.bench_function("owned_accessor", |b| {
        b.iter(|| black_box(darkmatter_base_json_schema()));
    });

    group.bench_function("configure_default_baseline", |b| {
        b.iter(|| {
            let api = DarkmatterSchemas::new()
                .with_darkmatter_baseline_json_schema()
                .expect("built-in benchmark baseline must be valid");
            black_box(api);
        });
    });

    group.bench_function("configure_and_resolve_default_baseline", |b| {
        b.iter(|| {
            let api = DarkmatterSchemas::new()
                .with_darkmatter_baseline_json_schema()
                .expect("built-in benchmark baseline must be valid");
            let effective = api
                .effective_for(black_box(&baseline_only))
                .expect("baseline-only assembly must succeed")
                .expect("configured baseline must produce an effective schema");
            black_box(effective);
        });
    });

    group.bench_function("merge_empty_document", |b| {
        b.iter_batched(
            || empty_document_schema.clone(),
            |document| {
                let merged = merge_baseline(black_box(&baseline), black_box(document))
                    .expect("empty document schema must merge");
                black_box(merged);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("merge_tiny_document", |b| {
        b.iter_batched(
            || tiny_document_schema.clone(),
            |document| {
                let merged = merge_baseline(black_box(&baseline), black_box(document))
                    .expect("tiny document schema must merge");
                black_box(merged);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_effective_schema_ownership(c: &mut Criterion) {
    bench_baseline(c, "darkmatter_baseline", darkmatter_base_json_schema());
    bench_baseline(
        c,
        "synthetic_512_properties",
        synthetic_baseline(SYNTHETIC_PROPERTY_COUNT),
    );
    bench_document_only(c);
    bench_default_baseline_initialization(c);
}

criterion_group!(benches, bench_effective_schema_ownership);
criterion_main!(benches);
