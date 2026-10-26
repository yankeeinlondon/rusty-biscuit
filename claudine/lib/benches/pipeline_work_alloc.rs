//! Allocation and work-count measurements for the public scenarios of the
//! `2026-10-25-pipeline-work-reduction` spike (Claudine side).
//!
//! Each scenario prints one machine-readable `SPIKE` row: the item, its size,
//! the median wall time, the allocations, bytes, and peak live bytes of the
//! last (warm) and first (cold) run, and every nonzero work counter. The
//! crate-private scenarios (C1, C3, C6, C7, A4's group step) are ignored unit
//! tests in `src/pipeline_spike.rs`, which print the same rows.
//!
//! ```text
//! cargo bench -p claudine --features work-counters --bench pipeline_work_alloc
//! ```
//!
//! Pass a substring to run only the matching scenarios, for example `-- C2`.

use std::collections::HashMap;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
use darkmatter::markdown::compose::RequestSnapshot;
use darkmatter::markdown::compose::expression::{EvaluationLookup, evaluate, parse_condition};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use serde_json::{Map, Value, json};
use tempfile::TempDir;
use test_toolkit::alloc_count::{self, AllocationSnapshot, CountingAllocator};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

struct Measured {
    median: Duration,
    cold: AllocationSnapshot,
    warm: AllocationSnapshot,
    claudine: claudine::work_counters::WorkSnapshot,
    darkmatter: darkmatter::work_counters::WorkSnapshot,
}

/// Run `scenario` `runs` times; keep the median time and the first and last
/// run's allocation and counter deltas.
fn measure<T>(runs: usize, mut scenario: impl FnMut() -> T) -> Measured {
    let mut times = Vec::with_capacity(runs);
    let mut cold = None;
    let mut warm = AllocationSnapshot::default();
    let mut claudine = claudine::work_counters::WorkSnapshot::default();
    let mut darkmatter = darkmatter::work_counters::WorkSnapshot::default();
    for _ in 0..runs.max(1) {
        alloc_count::reset_peak();
        let allocations = alloc_count::snapshot();
        let claudine_before = claudine::work_counters::snapshot();
        let darkmatter_before = darkmatter::work_counters::snapshot();
        let started = Instant::now();
        let output = scenario();
        let elapsed = started.elapsed();
        warm = alloc_count::snapshot().since(allocations);
        claudine = claudine::work_counters::snapshot().since(&claudine_before);
        darkmatter = darkmatter::work_counters::snapshot().since(&darkmatter_before);
        black_box(output);
        cold.get_or_insert(warm);
        times.push(elapsed);
    }
    times.sort();
    Measured {
        median: times[times.len() / 2],
        cold: cold.unwrap_or_default(),
        warm,
        claudine,
        darkmatter,
    }
}

fn report(item: &str, size: &str, measured: &Measured) {
    let mut work: Vec<String> = measured
        .claudine
        .nonzero()
        .into_iter()
        .map(|(work, count)| format!("{work:?}:{count}"))
        .collect();
    work.extend(
        measured
            .darkmatter
            .nonzero()
            .into_iter()
            .map(|(work, count)| format!("dm.{work:?}:{count}")),
    );
    println!(
        "SPIKE item={item} size={size} time_ns={} allocs={} bytes={} peak={} cold_allocs={} cold_bytes={} cold_peak={} work={}",
        measured.median.as_nanos(),
        measured.warm.allocations,
        measured.warm.bytes_allocated,
        measured.warm.peak_live_bytes,
        measured.cold.allocations,
        measured.cold.bytes_allocated,
        measured.cold.peak_live_bytes,
        if work.is_empty() {
            "-".to_string()
        } else {
            work.join(",")
        },
    );
}

fn text_of(bytes: usize) -> String {
    "x".repeat(bytes)
}

/// A JSON tree of about `bytes` bytes: an array of 1 KB-string records, the
/// shape of a tool response or a group's result dictionary.
fn payload_of(bytes: usize) -> Value {
    Value::Array(
        (0..bytes.div_ceil(1024))
            .map(|index| json!({"index": index, "text": text_of(1000)}))
            .collect(),
    )
}

/// A frontmatter mapping of about `bytes` bytes, as YAML text lines.
fn frontmatter_lines(bytes: usize) -> String {
    (0..bytes.div_ceil(1024))
        .map(|index| format!("field_{index}: {}\n", text_of(1000)))
        .collect()
}

/// A request rooted at `dir` with an empty home, so no ambient schema root,
/// repository, or user configuration participates.
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

/// C2: one dotted `hook.<leaf>` read builds the whole hook object.
fn c2_hook_leaf_reads() {
    for bytes in [10 * 1024, 100 * 1024, 1024 * 1024] {
        let meta = hook_meta(bytes);
        let lookup = EventMetaExpressionLookup::new(&meta);
        let size = format!("tool_response={}KB", bytes / 1024);
        report(
            "C2",
            &format!("{size}/hook.tool_name"),
            &measure(21, || lookup.get("hook.tool_name")),
        );
        report(
            "C2",
            &format!("{size}/unprefixed-tool_name"),
            &measure(21, || lookup.get("tool_name")),
        );
        report(
            "C2",
            &format!("{size}/hook.tool_input.command"),
            &measure(21, || lookup.get("hook.tool_input.command")),
        );
    }
}

/// The ten hook leaves A5's templates read.
const A5_TEMPLATE: &str = "{{ hook.provider }} {{ hook.event }} {{ hook.session_id }} {{ hook.cwd }} {{ hook.tool_name }} \
     {{ hook.tool_input.command }} {{ hook.extra.flag }} {{ hook.prompt }} {{ hook.os.name }} {{ hook.hardware.arch }}";

/// The five loop scalars A5's conditions read.
const A5_CONDITION: &str = "loop.count > 0 && loop.is_first == false && loop.is_last == false && loop.last_exit_code == 0 && loop.count < 100";

fn loop_ambient(last_output_bytes: usize) -> LoopAmbient {
    LoopAmbient::new(3, false, false, payload_of(last_output_bytes), 0)
}

/// C4: one scalar `loop.<member>` read builds the whole `loop` object.
fn c4_loop_scalar_reads(sandbox: &Sandbox) {
    let context =
        capture_file_resolution_context(&sandbox.snapshot(HashMap::new())).expect("context");
    let frontmatter = Map::new();
    let source = sandbox.path().join("loop.md");
    for bytes in [10 * 1024, 100 * 1024, 1024 * 1024] {
        let ambient = loop_ambient(bytes);
        let lookup = LoopExpressionLookup::new(&frontmatter, &ambient, &context, &source);
        let size = format!("last_output={}KB", bytes / 1024);
        report(
            "C4",
            &format!("{size}/loop.count"),
            &measure(21, || lookup.get("loop.count")),
        );
        report(
            "C4",
            &format!("{size}/loop.is_last"),
            &measure(21, || lookup.get("loop.is_last")),
        );
    }
}

/// C5: every pass parses the unchanged condition again.
fn c5_loop_condition_passes(sandbox: &Sandbox) {
    let context =
        capture_file_resolution_context(&sandbox.snapshot(HashMap::new())).expect("context");
    let frontmatter = Map::new();
    let source = sandbox.path().join("loop.md");
    let ambient = loop_ambient(1024);
    let condition =
        LoopCondition::Until("loop.count >= 1000000 || loop.last_exit_code != 0".into());
    for passes in [10usize, 100, 1000] {
        let measured = measure(5, || {
            let lookup = LoopExpressionLookup::new(&frontmatter, &ambient, &context, &source);
            (0..passes)
                .filter(|_| evaluate_condition(&condition, &lookup).expect("condition evaluates"))
                .count()
        });
        report(
            "C5",
            &format!("passes={passes}/evaluate_condition"),
            &measured,
        );
    }
    // The parse alone, which is the part a retained tree removes.
    let parse = measure(101, || {
        parse_condition("loop.count >= 1000000 || loop.last_exit_code != 0").expect("parses")
    });
    report("C5", "one-parse", &parse);
    // The anchor: one pass of a trivial shell task is at least one process
    // spawn.
    let spawn = measure(21, || {
        let status = if cfg!(windows) {
            std::process::Command::new("cmd")
                .args(["/C", "exit 0"])
                .status()
        } else {
            std::process::Command::new("sh")
                .args(["-c", "true"])
                .status()
        };
        status.expect("spawn").success()
    });
    report("C5", "anchor/one-trivial-shell-spawn", &spawn);
}

/// C8: entering a document copies environment maps it then replaces.
fn c8_document_entry() {
    for variables in [50usize, 500] {
        let sandbox = Sandbox::new();
        let env: HashMap<String, String> = (0..variables)
            .map(|index| {
                (
                    format!("VARIABLE_{index}"),
                    format!("value-{index}-{}", text_of(32)),
                )
            })
            .collect();
        let invocation =
            InvocationContext::capture(&sandbox.snapshot(env.clone())).expect("invocation");
        for tasks in [10usize, 100, 1000] {
            let measured = measure(5, || {
                (0..tasks)
                    .map(|_| invocation.enter_document(DocumentEnvironment::new(env.clone())))
                    .map(|document| document.document_environment().vars().len())
                    .sum::<usize>()
            });
            report(
                "C8",
                &format!("vars={variables}/tasks={tasks}/enter_document"),
                &measured,
            );
        }
        // The environment each task receives is a fresh map in production
        // too; measure it alone so it can be subtracted.
        report(
            "C8",
            &format!("vars={variables}/one-map-clone"),
            &measure(21, || env.clone()),
        );
    }
}

/// D3: the optional scrub and the launch check each rediscover triggers.
fn d3_scrub_and_check() {
    for triggers in [1usize, 10, 100] {
        let sandbox = Sandbox::new();
        let schemas = sandbox.path().join("trigger-schemas");
        for index in 0..triggers {
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
        let source =
            resolve_composition_source_in_context(&document.display().to_string(), &context)
                .expect("source");
        let findings = TriggerFindings::default();
        let measured = measure(9, || {
            pre_validate_schema(&source, None, None, &context, &findings).expect("pre-validates")
        });
        report(
            "D3",
            &format!("schema-root-files={triggers}/pre_validate_schema"),
            &measured,
        );
        // One discovery alone: what sharing the registry would save per
        // pre-validation.
        let document_context = context.for_source(&source.resolved_path);
        let discovery = measure(9, || {
            DarkmatterSchemas::new(document_context.clone())
                .with_embedded_trigger_bundle(claudine_schema_bundle())
                .and_then(DarkmatterSchemas::with_trigger_discovery)
                .expect("discovers")
        });
        report(
            "D3",
            &format!("schema-root-files={triggers}/one-discovery"),
            &discovery,
        );
    }
}

/// A5: 100 events, each rendering a template of ten hook leaves and
/// evaluating a condition of five loop scalars, with a 1 MB tool response and
/// a 100 KB previous loop result.
fn a5_event_dispatch(sandbox: &Sandbox) {
    let context =
        capture_file_resolution_context(&sandbox.snapshot(HashMap::new())).expect("context");
    let frontmatter = Map::new();
    let source = sandbox.path().join("loop.md");
    let metas: Vec<EventMeta> = (0..100).map(|_| hook_meta(1024 * 1024)).collect();
    let ambient = loop_ambient(100 * 1024);
    let measured = measure(5, || {
        metas
            .iter()
            .map(|meta| {
                let rendered = interpolate(A5_TEMPLATE, meta);
                let lookup = LoopExpressionLookup::new(&frontmatter, &ambient, &context, &source);
                let condition = parse_condition(A5_CONDITION).expect("parses");
                let value = evaluate(&condition, &lookup).expect("evaluates");
                rendered.len() + usize::from(value == Value::Bool(true))
            })
            .sum::<usize>()
    });
    report(
        "A5",
        "events=100/tool_response=1MB/last_output=100KB",
        &measured,
    );
}

/// Load `name` as a sequence and build its preflight graph.
fn preflight(sandbox: &Sandbox, name: &str) -> impl FnMut() -> usize {
    let context =
        capture_file_resolution_context(&sandbox.snapshot(HashMap::new())).expect("context");
    let path = sandbox.path().join(name).display().to_string();
    move || {
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

/// C7 inside preflight: `tasks` prompt tasks share one prompt document with
/// 100 KB of frontmatter and a literal `agent:`.
fn c7_preflight_scaling() {
    for tasks in [10usize, 100, 1000] {
        let sandbox = Sandbox::new();
        sandbox.write(
            "prompt.md",
            &format!(
                "---\nagent: codex\n{}---\n\nDo the work.\n",
                frontmatter_lines(100 * 1024)
            ),
        );
        let steps: Vec<Value> = (0..tasks)
            .map(|index| json!({"name": format!("task-{index}"), "prompt": "prompt.md"}))
            .collect();
        sandbox.write("seq.md", &sequence_document(&steps));
        let measured = measure(3, preflight(&sandbox, "seq.md"));
        report(
            "C7",
            &format!("tasks={tasks}/fm=100KB/build_preflight_graph"),
            &measured,
        );
    }
}

/// A4: a 50-step sequence: 49 prompt tasks, each its own prompt document with
/// 100 KB of frontmatter and a literal `agent:`, and one 200-leg group at
/// `max_parallel: 4`.
fn a4_sequence_preflight() {
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
    report(
        "A4",
        "preflight/steps=50/legs=200/fm=100KB",
        &measure(5, preflight(&sandbox, "seq.md")),
    );
}

/// A6: a 100-step sequence whose steps run template commands with ten holes
/// each, every hole reading a typed frontmatter value.
fn a6_shell_preparation() {
    for step_count in [10usize, 100] {
        a6_with_steps(step_count);
    }
}

fn a6_with_steps(step_count: usize) {
    let sandbox = Sandbox::new();
    let holes: String = (0..10)
        .map(|hole| format!(" {{{{ arg_{hole} }}}}"))
        .collect();
    let steps: Vec<Value> =
        (0..step_count).map(|index| json!({"name": format!("step-{index}"), "shell": format!("echo step-{index}{holes}")})).collect();
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
    report(
        "A6",
        &format!("steps={step_count}/holes=10"),
        &measure(5, preflight(&sandbox, "seq.md")),
    );
}

fn main() {
    let filter: Option<String> = std::env::args()
        .skip(1)
        .find(|argument| !argument.starts_with('-'));
    let selected = |item: &str| filter.as_deref().is_none_or(|filter| item.contains(filter));
    let sandbox = Sandbox::new();
    if selected("C2") {
        c2_hook_leaf_reads();
    }
    if selected("C4") {
        c4_loop_scalar_reads(&sandbox);
    }
    if selected("C5") {
        c5_loop_condition_passes(&sandbox);
    }
    if selected("C7") {
        c7_preflight_scaling();
    }
    if selected("C8") {
        c8_document_entry();
    }
    if selected("D3") {
        d3_scrub_and_check();
    }
    if selected("A4") {
        a4_sequence_preflight();
    }
    if selected("A5") {
        a5_event_dispatch(&sandbox);
    }
    if selected("A6") {
        a6_shell_preparation();
    }
}
