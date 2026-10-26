//! Criterion timings for the public scenarios of the
//! `2026-10-25-pipeline-work-reduction` spike (Claudine side), under the
//! system allocator. Allocation and work counts come from
//! `pipeline_work_alloc`; run this target without `work-counters` so no
//! counter is compiled in:
//!
//! ```text
//! cargo bench -p claudine --bench pipeline_work
//! cargo bench -p claudine --bench pipeline_work -- A5      # one group
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use claudine::composition::schema::TriggerFindings;
use claudine::composition::schema::bundle::claudine_schema_bundle;
use claudine::composition::{
    LoopAmbient, LoopCondition, LoopExpressionLookup, build_preflight_graph,
    capture_file_resolution_context, evaluate_condition, pre_validate_schema,
    resolve_composition_source_in_context, resolve_sequence_plan,
};
use claudine::dispatch::expression::EventMetaExpressionLookup;
use claudine::dispatch::template::interpolate;
use claudine::events::{AgenticEvent, EventMeta};
use claudine::invocation_context::{DocumentEnvironment, InvocationContext};
use claudine::provider::Provider;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use darkmatter::markdown::compose::RequestSnapshot;
use darkmatter::markdown::compose::expression::{EvaluationLookup, evaluate, parse_condition};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use serde_json::{Map, Value, json};
use tempfile::TempDir;

fn text_of(bytes: usize) -> String {
    "x".repeat(bytes)
}

fn payload_of(bytes: usize) -> Value {
    Value::Array(
        (0..bytes.div_ceil(1024))
            .map(|index| json!({"index": index, "text": text_of(1000)}))
            .collect(),
    )
}

fn frontmatter_lines(bytes: usize) -> String {
    (0..bytes.div_ceil(1024))
        .map(|index| format!("field_{index}: {}\n", text_of(1000)))
        .collect()
}

struct Sandbox {
    dir: TempDir,
    home: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            dir: TempDir::new().expect("temp dir"),
            home: TempDir::new().expect("temp home"),
        }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn snapshot(&self, env: HashMap<String, String>) -> RequestSnapshot {
        RequestSnapshot::new(self.path())
            .with_home(Some(self.home.path().to_path_buf()))
            .with_env(env)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(&path, text).expect("write fixture");
        path
    }

    fn preflight(&self, name: &str) -> usize {
        let context =
            capture_file_resolution_context(&self.snapshot(HashMap::new())).expect("context");
        let path = self.path().join(name).display().to_string();
        let source = resolve_composition_source_in_context(&path, &context).expect("source");
        let plan = resolve_sequence_plan(&source, &context)
            .expect("plan")
            .expect("a sequence");
        build_preflight_graph(&plan, &source, &context)
            .expect("preflight")
            .steps
            .len()
    }
}

fn sequence_document(steps: &[Value]) -> String {
    format!(
        "---\nsequence: {}\n---\n\nBody.\n",
        serde_json::to_string(steps).expect("serialize")
    )
}

fn hook_meta(tool_response_bytes: usize) -> EventMeta {
    let mut meta = EventMeta::new(Provider::Claude, AgenticEvent::AfterTool);
    meta.session_id = Some("session-1".into());
    meta.cwd = Some("/work".into());
    meta.tool_name = Some("Bash".into());
    meta.prompt = Some("run the tests".into());
    meta.tool_input = Some(json!({"command": "cargo test", "description": "tests"}));
    meta.tool_response = Some(payload_of(tool_response_bytes));
    meta.extra.insert("flag".into(), json!(true));
    meta
}

const A5_TEMPLATE: &str = "{{ hook.provider }} {{ hook.event }} {{ hook.session_id }} {{ hook.cwd }} {{ hook.tool_name }} \
     {{ hook.tool_input.command }} {{ hook.extra.flag }} {{ hook.prompt }} {{ hook.os.name }} {{ hook.hardware.arch }}";
const A5_CONDITION: &str = "loop.count > 0 && loop.is_first == false && loop.is_last == false && loop.last_exit_code == 0 && loop.count < 100";

fn configured<'a>(
    c: &'a mut Criterion,
    name: &str,
) -> criterion::BenchmarkGroup<'a, criterion::measurement::WallTime> {
    let mut group = c.benchmark_group(name);
    group
        .sample_size(10)
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_secs(2));
    group
}

/// C2 and C4: one leaf read against a growing unrelated payload.
fn leaf_reads(c: &mut Criterion) {
    let sandbox = Sandbox::new();
    let context =
        capture_file_resolution_context(&sandbox.snapshot(HashMap::new())).expect("context");
    let frontmatter = Map::new();
    let source = sandbox.path().join("loop.md");
    let mut group = configured(c, "C2_hook_leaf");
    for bytes in [10 * 1024, 1024 * 1024] {
        let meta = hook_meta(bytes);
        let lookup = EventMetaExpressionLookup::new(&meta);
        group.bench_function(BenchmarkId::new("hook.tool_name", bytes), |b| {
            b.iter(|| lookup.get(black_box("hook.tool_name")))
        });
    }
    group.finish();
    let mut group = configured(c, "C4_loop_scalar");
    for bytes in [10 * 1024, 1024 * 1024] {
        let ambient = LoopAmbient::new(3, false, false, payload_of(bytes), 0);
        let lookup = LoopExpressionLookup::new(&frontmatter, &ambient, &context, &source);
        group.bench_function(BenchmarkId::new("loop.count", bytes), |b| {
            b.iter(|| lookup.get(black_box("loop.count")))
        });
    }
    group.finish();
}

/// C5: one pass's condition evaluation, and the parse it repeats.
fn loop_condition(c: &mut Criterion) {
    let sandbox = Sandbox::new();
    let context =
        capture_file_resolution_context(&sandbox.snapshot(HashMap::new())).expect("context");
    let frontmatter = Map::new();
    let source = sandbox.path().join("loop.md");
    let ambient = LoopAmbient::new(3, false, false, payload_of(1024), 0);
    let condition =
        LoopCondition::Until("loop.count >= 1000000 || loop.last_exit_code != 0".into());
    let mut group = configured(c, "C5_loop_condition");
    group.bench_function("evaluate_condition", |b| {
        let lookup = LoopExpressionLookup::new(&frontmatter, &ambient, &context, &source);
        b.iter(|| evaluate_condition(black_box(&condition), &lookup).expect("evaluates"))
    });
    group.bench_function("parse_only", |b| {
        b.iter(|| {
            parse_condition(black_box(
                "loop.count >= 1000000 || loop.last_exit_code != 0",
            ))
            .expect("parses")
        })
    });
    group.finish();
}

/// C8: one document entry under a 500-variable environment.
fn document_entry(c: &mut Criterion) {
    let sandbox = Sandbox::new();
    let env: HashMap<String, String> = (0..500)
        .map(|index| {
            (
                format!("VARIABLE_{index}"),
                format!("value-{index}-{}", text_of(32)),
            )
        })
        .collect();
    let invocation =
        InvocationContext::capture(&sandbox.snapshot(env.clone())).expect("invocation");
    let mut group = configured(c, "C8_document_entry");
    group.bench_function("enter_document/vars=500", |b| {
        b.iter(|| invocation.enter_document(DocumentEnvironment::new(env.clone())))
    });
    group.finish();
}

/// D3: pre-validation over ten schema-root files, and one discovery alone.
fn scrub_and_check(c: &mut Criterion) {
    let sandbox = Sandbox::new();
    let schemas = sandbox.path().join("trigger-schemas");
    for index in 0..10 {
        sandbox.write(
            &format!("trigger-schemas/probe-{index}.yaml"),
            &format!("kind: schema-trigger\nmatch:\n    probe_{index}: string(required)\n$schema:\n    probe_{index}: string\n    count: number(optional)\n"),
        );
    }
    let document = sandbox.write(
        "prompt.md",
        "---\nprobe_0: hello\ntitle: Spike\n---\n\nBody.\n",
    );
    let env = HashMap::from([("SCHEMAS_DIR".to_string(), schemas.display().to_string())]);
    let context = capture_file_resolution_context(&sandbox.snapshot(env)).expect("context");
    let source = resolve_composition_source_in_context(&document.display().to_string(), &context)
        .expect("source");
    let findings = TriggerFindings::default();
    let document_context = context.for_source(&source.resolved_path);
    let mut group = configured(c, "D3_scrub_and_check");
    group.bench_function("pre_validate_schema/files=10", |b| {
        b.iter(|| {
            pre_validate_schema(&source, None, None, &context, &findings).expect("pre-validates")
        })
    });
    group.bench_function("one_discovery/files=10", |b| {
        b.iter(|| {
            DarkmatterSchemas::new(document_context.clone())
                .with_embedded_trigger_bundle(claudine_schema_bundle())
                .and_then(DarkmatterSchemas::with_trigger_discovery)
                .expect("discovers")
        })
    });
    group.finish();
}

/// C7: preflight of 100 prompt tasks sharing one 100 KB-frontmatter document.
fn prediction_preflight(c: &mut Criterion) {
    let sandbox = Sandbox::new();
    sandbox.write(
        "prompt.md",
        &format!(
            "---\nagent: codex\n{}---\n\nDo the work.\n",
            frontmatter_lines(100 * 1024)
        ),
    );
    let steps: Vec<Value> = (0..100)
        .map(|index| json!({"name": format!("task-{index}"), "prompt": "prompt.md"}))
        .collect();
    sandbox.write("seq.md", &sequence_document(&steps));
    let mut group = configured(c, "C7_prediction");
    group.throughput(Throughput::Elements(100));
    group.bench_function("build_preflight_graph/tasks=100", |b| {
        b.iter(|| sandbox.preflight("seq.md"))
    });
    group.finish();
}

/// A4: preflight of 49 prompt tasks (100 KB frontmatter each) and a 200-leg
/// group. The group's execution is measured by the crate-private spike.
fn a4_preflight(c: &mut Criterion) {
    let sandbox = Sandbox::new();
    let mut steps = Vec::new();
    for index in 0..49 {
        sandbox.write(
            &format!("prompts/p{index}.md"),
            &format!(
                "---\nagent: codex\nsuccess:\n    - say: \"Task {index} finished\"\n{}---\n\nDo task {index}.\n",
                frontmatter_lines(100 * 1024)
            ),
        );
        steps.push(
            json!({"name": format!("task-{index}"), "prompt": format!("prompts/p{index}.md")}),
        );
        if index == 24 {
            let legs: Vec<Value> = (0..200)
                .map(|leg| json!({"name": format!("leg-{leg}"), "shell": format!("echo {leg}")}))
                .collect();
            steps.push(json!({"name": "fan-out", "group": {"name": "legs", "max_parallel": 4, "tasks": legs}}));
        }
    }
    sandbox.write("seq.md", &sequence_document(&steps));
    let mut group = configured(c, "A4_sequence_preflight");
    group.bench_function("steps=50/legs=200", |b| {
        b.iter(|| sandbox.preflight("seq.md"))
    });
    group.finish();
}

/// A5: 100 events with a 1 MB tool response.
fn a5_dispatch(c: &mut Criterion) {
    let sandbox = Sandbox::new();
    let context =
        capture_file_resolution_context(&sandbox.snapshot(HashMap::new())).expect("context");
    let frontmatter = Map::new();
    let source = sandbox.path().join("loop.md");
    let metas: Vec<EventMeta> = (0..100).map(|_| hook_meta(1024 * 1024)).collect();
    let ambient = LoopAmbient::new(3, false, false, payload_of(100 * 1024), 0);
    let mut group = configured(c, "A5_event_dispatch");
    group.throughput(Throughput::Elements(100));
    group.bench_function("events=100", |b| {
        b.iter(|| {
            metas
                .iter()
                .map(|meta| {
                    let rendered = interpolate(A5_TEMPLATE, meta);
                    let lookup =
                        LoopExpressionLookup::new(&frontmatter, &ambient, &context, &source);
                    let condition = parse_condition(A5_CONDITION).expect("parses");
                    rendered.len()
                        + usize::from(
                            evaluate(&condition, &lookup).expect("evaluates") == Value::Bool(true),
                        )
                })
                .sum::<usize>()
        })
    });
    group.finish();
}

/// A6: preparation of 100 template-command steps with ten holes each.
fn a6_shell_preparation(c: &mut Criterion) {
    let sandbox = Sandbox::new();
    let holes: String = (0..10)
        .map(|hole| format!(" {{{{ arg_{hole} }}}}"))
        .collect();
    let steps: Vec<Value> =
        (0..100).map(|index| json!({"name": format!("step-{index}"), "shell": format!("echo step-{index}{holes}")})).collect();
    let values: String = (0..10)
        .map(|hole| format!("arg_{hole}: value-{hole}\n"))
        .collect();
    let schema: String = (0..10)
        .map(|hole| format!("    arg_{hole}: string\n"))
        .collect();
    sandbox.write(
        "seq.md",
        &format!(
            "---\n$schema:\n{schema}{values}sequence: {}\n---\n\nBody.\n",
            serde_json::to_string(&steps).expect("serialize")
        ),
    );
    let mut group = configured(c, "A6_shell_preparation");
    group.bench_function("steps=100/holes=10", |b| {
        b.iter(|| sandbox.preflight("seq.md"))
    });
    group.finish();
}

criterion_group!(
    benches,
    leaf_reads,
    loop_condition,
    document_entry,
    scrub_and_check,
    prediction_preflight,
    a4_preflight,
    a5_dispatch,
    a6_shell_preparation
);
criterion_main!(benches);
