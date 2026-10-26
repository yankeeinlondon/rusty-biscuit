//! Anchor A3 of the `2026-10-25-pipeline-work-reduction` spike: one DMLS
//! edit loop on the repository's largest lifecycle document, frozen in
//! `darkmatter/lib/benches/fixtures/pipeline-work/`, driven in-process the way
//! DMLS L1 tests drive the providers.
//!
//! One loop is an edit (a new frontmatter text, so nothing keyed by text is
//! reused), the document's overlay, one diagnostics publish, one completion,
//! and two hovers (a lifecycle verb and an expression in a `when`). The
//! server-lifetime state (repository contexts, overlay state, workspace
//! graph, provider registry) persists across loops, as in a running server.
//! The document is typed by Claudine's trigger files from the frozen bundle
//! in `darkmatter/lib/benches/fixtures/claudine-schemas/`, and its events
//! are Claudine's through the bundle's lifecycle artifact, as in the
//! repository.
//!
//! Prints one tab-separated row per phase and per loop total, at 1× and 10×
//! the document's lifecycle stacks, warm (process caches kept) and cold
//! (emptied before the loop):
//!
//! ```text
//! case  phase  darkmatter_work  dmls_work  allocations  bytes_allocated  peak_live_bytes  median_ns
//! ```
//!
//! Wall times include the counting allocator's overhead.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use darkmatter::markdown::compose::RequestSnapshot;
use darkmatter::markdown::schemas::reset_process_caches_for_bench;
use darkmatter::work_counters::{self, WorkSnapshot};
use dmls::capabilities::ClientProfile;
use dmls::config::DmlsConfig;
use dmls::context::RepositoryContexts;
use dmls::graph::{NoContexts, WorkspaceGraph};
use dmls::overlay::OverlayState;
use dmls::providers::{
    DocumentContext, DslProvider, FrontmatterProvider, Provider, ProviderRegistry, SubstrateProvider, WikiProvider,
};
use dmls::source_map::{PositionEncoding, SourceMap};
use dmls::work_counters::{WorkCounts, work_counts};
use lsp_types::{InitializeParams, Uri};
use serde_yaml_ng::Value as YamlValue;
use test_toolkit::alloc_count::{self, AllocationSnapshot, CountingAllocator};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const A3_DOCUMENT: &str = include_str!("../../lib/benches/fixtures/pipeline-work/implement-plan.md");

const CLAUDINE_SCHEMAS: [(&str, &str); 9] = [
    ("action.yaml", include_str!("../../lib/benches/fixtures/claudine-schemas/action.yaml")),
    ("claudine.yaml", include_str!("../../lib/benches/fixtures/claudine-schemas/claudine.yaml")),
    ("globals.yaml", include_str!("../../lib/benches/fixtures/claudine-schemas/globals.yaml")),
    ("inline-compose.yaml", include_str!("../../lib/benches/fixtures/claudine-schemas/inline-compose.yaml")),
    ("lifecycle.yaml", include_str!("../../lib/benches/fixtures/claudine-schemas/lifecycle.yaml")),
    ("linking.yaml", include_str!("../../lib/benches/fixtures/claudine-schemas/linking.yaml")),
    ("review.yaml", include_str!("../../lib/benches/fixtures/claudine-schemas/review.yaml")),
    ("sequence-types.yaml", include_str!("../../lib/benches/fixtures/claudine-schemas/sequence-types.yaml")),
    ("sequence.yaml", include_str!("../../lib/benches/fixtures/claudine-schemas/sequence.yaml")),
];

/// Claudine's host lifecycle artifact, frozen beside the A3 document; fresh
/// for the frozen bundle, whose `globals.yaml` it is keyed by.
const CLAUDINE_LIFECYCLE_ARTIFACT: &str = include_str!("../../lib/benches/fixtures/pipeline-work/globals.lifecycle.json");

/// Timed loops after the counted one; the median is reported.
const TIMED_RUNS: usize = 5;

/// The phrase each edit changes, so every loop analyzes new text.
const EDITED: &str = "Implements a plan phase by phase.";

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("create directory");
    std::fs::write(path, text).expect("write file");
}

/// A repository whose `claudine/schemas/` holds the frozen bundle, as the
/// DMLS L1 Claudine fixture lays it out, with an empty home.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let repo = root.join("repo");
        std::fs::create_dir_all(repo.join(".git/objects")).expect("objects");
        std::fs::create_dir_all(repo.join(".git/refs/heads")).expect("refs");
        write(&repo.join(".git/HEAD"), "ref: refs/heads/main\n");
        write(&repo.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
        write(&repo.join("Cargo.toml"), "[workspace]\nmembers = [\"claudine/lib\"]\n");
        write(&repo.join("claudine/lib/Cargo.toml"), "[package]\nname = \"claudine\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
        write(&repo.join("claudine/lib/src/lib.rs"), "");
        for (name, text) in CLAUDINE_SCHEMAS {
            write(&repo.join("claudine/schemas").join(name), text);
        }
        write(&repo.join("claudine/schemas/globals.lifecycle.json"), CLAUDINE_LIFECYCLE_ARTIFACT);
        std::fs::create_dir_all(root.join("home")).expect("home");
        Self { _dir: dir, root }
    }

    fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    fn snapshot(&self) -> RequestSnapshot {
        RequestSnapshot::new(&self.root).with_home(Some(self.root.join("home")))
    }
}

/// The document with every lifecycle stack (and the loop gate) repeated
/// `factor` times; comments are dropped when `factor > 1`.
fn scaled(factor: usize) -> String {
    if factor == 1 {
        return A3_DOCUMENT.to_string();
    }
    let (yaml, body) = A3_DOCUMENT.strip_prefix("---\n").and_then(|rest| rest.split_once("\n---\n")).expect("frontmatter");
    let mut value: YamlValue = serde_yaml_ng::from_str(yaml).expect("YAML");
    let repeat = |stack: &mut YamlValue| {
        if let YamlValue::Sequence(items) = stack {
            let original = items.clone();
            for _ in 1..factor {
                items.extend(original.iter().cloned());
            }
        }
    };
    if let YamlValue::Mapping(map) = &mut value {
        for event in ["initialize", "start", "success"] {
            if let Some(stack) = map.get_mut(event) {
                repeat(stack);
            }
        }
        if let Some(YamlValue::Mapping(block)) = map.get_mut("loop")
            && let Some(gate) = block.get_mut("gate")
        {
            repeat(gate);
        }
    }
    format!("---\n{}---\n{body}", serde_yaml_ng::to_string(&value).expect("YAML serializes"))
}

/// What one phase did.
#[derive(Default)]
struct Measured {
    darkmatter: WorkSnapshot,
    dmls: WorkCounts,
    allocated: AllocationSnapshot,
    nanos: u128,
}

/// Runs `phase`, measuring its work, allocations, and time.
fn measure<T>(phase: impl FnOnce() -> T) -> (T, Measured) {
    let darkmatter = work_counters::snapshot();
    let dmls = work_counts();
    alloc_count::reset_peak();
    let allocated = alloc_count::snapshot();
    let start = Instant::now();
    let value = phase();
    let nanos = start.elapsed().as_nanos();
    let measured = Measured {
        allocated: alloc_count::snapshot().since(allocated),
        darkmatter: work_counters::snapshot().since(&darkmatter),
        dmls: work_counts().since(dmls),
        nanos,
    };
    (value, measured)
}

/// The server-lifetime state one session keeps.
struct Server {
    fixture: Fixture,
    config: DmlsConfig,
    contexts: RepositoryContexts,
    registry: ProviderRegistry,
    graph: WorkspaceGraph,
    profile: ClientProfile,
    overlays: OverlayState,
    path: PathBuf,
    uri: Uri,
    edits: usize,
}

const PHASES: [&str; 5] = ["overlay", "diagnostics", "completion", "hover_verb", "hover_expression"];

impl Server {
    fn new() -> Self {
        let fixture = Fixture::new();
        let path = fixture.repo().join("claudine/lib/tasks/implement-plan.md");
        let uri: Uri = url::Url::from_file_path(&path).expect("file URL").to_string().parse().expect("URI");
        Self {
            contexts: RepositoryContexts::new(fixture.snapshot()),
            config: DmlsConfig::default(),
            registry: ProviderRegistry::with_substrate(),
            graph: WorkspaceGraph::build(&BTreeMap::new(), 1, &NoContexts),
            profile: ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16),
            overlays: OverlayState::default(),
            fixture,
            path,
            uri,
            edits: 0,
        }
    }

    /// One edit loop over `base`; returns each phase's measurement.
    fn edit_loop(&mut self, base: &str) -> Vec<Measured> {
        self.edits += 1;
        let text = base.replacen(EDITED, &format!("Implements a plan phase by phase (edit {}).", self.edits), 1);
        assert_ne!(text, base, "the edit changes the frontmatter");
        write(&self.path, &text);
        let completion_at = text.find("warn:").expect("a `warn` verb") + 2;
        let hover_verb = text.find("ensure_file").expect("an `ensure_file` verb") + 2;
        let hover_expression = text.find("file_exists(").expect("a `file_exists` call") + 2;
        let repo = [self.fixture.repo()];
        let resolution = self.contexts.for_document(&self.path);
        let (overlay, overlay_cost) =
            measure(|| self.overlays.for_document(&self.uri, &text, &self.path, &self.config, &repo, &resolution));
        let source_map = SourceMap::new(self.uri.clone(), self.edits as i32, PositionEncoding::Utf16, Arc::from(text.as_str()));
        let ctx = DocumentContext {
            uri: &self.uri,
            path: &self.path,
            text: &text,
            source_map: &source_map,
            graph: &self.graph,
            doc_id: None,
            config: &self.config,
            profile: &self.profile,
            overlay: overlay.as_ref(),
            resolution: &resolution,
        };
        let (diagnostics, diagnostics_cost) = measure(|| self.registry.diagnostics(&ctx));
        assert!(overlay.is_some(), "the document has a frontmatter overlay");
        std::hint::black_box(diagnostics);
        let (_, completion_cost) = measure(|| self.registry.completion(&ctx, completion_at));
        let (_, verb_cost) = measure(|| self.registry.hover(&ctx, hover_verb));
        let (_, expression_cost) = measure(|| self.registry.hover(&ctx, hover_expression));
        // `A3_PROVIDER_BREAKDOWN=1` attributes each hover to the provider
        // that pays for it; the registry stops at the first non-empty answer.
        if std::env::var_os("A3_PROVIDER_BREAKDOWN").is_some() {
            let providers: [&dyn Provider; 4] = [&SubstrateProvider, &WikiProvider, &FrontmatterProvider, &DslProvider];
            for (label, offset) in [("hover_verb", hover_verb), ("hover_expression", hover_expression)] {
                for provider in providers {
                    let (answer, cost) = measure(|| provider.hover(&ctx, offset));
                    eprintln!(
                        "PROVIDER {label} {} answered={} allocs={} bytes={} peak={} ns={}",
                        provider.name(),
                        answer.is_some(),
                        cost.allocated.allocations,
                        cost.allocated.bytes_allocated,
                        cost.allocated.peak_live_bytes,
                        cost.nanos
                    );
                }
            }
        }
        vec![overlay_cost, diagnostics_cost, completion_cost, verb_cost, expression_cost]
    }
}

fn work_text(snapshot: &WorkSnapshot) -> String {
    let parts: Vec<String> = snapshot.nonzero().into_iter().map(|(kind, count)| format!("{kind:?}={count}")).collect();
    if parts.is_empty() { "-".to_string() } else { parts.join(",") }
}

fn dmls_text(counts: &WorkCounts) -> String {
    format!(
        "reads={},hashes={},parses={},enumerations={},registry_scans={}",
        counts.reads, counts.hashes, counts.parses, counts.enumerations, counts.registry_scans
    )
}

fn main() {
    println!("case\tphase\tdarkmatter_work\tdmls_work\tallocations\tbytes_allocated\tpeak_live_bytes\tmedian_ns");
    for factor in [1usize, 10] {
        let base = scaled(factor);
        for cold in [false, true] {
            let case = format!("A3/edit_loop/a3x{factor}-{}", if cold { "cold" } else { "warm" });
            let mut server = Server::new();
            // A first loop fills what a running server already holds.
            server.edit_loop(&base);
            if cold {
                reset_process_caches_for_bench();
            }
            let counted = server.edit_loop(&base);
            let mut timings: Vec<Vec<u128>> = vec![Vec::new(); PHASES.len() + 1];
            for _ in 0..TIMED_RUNS {
                if cold {
                    reset_process_caches_for_bench();
                }
                let phases = server.edit_loop(&base);
                for (index, phase) in phases.iter().enumerate() {
                    timings[index].push(phase.nanos);
                }
                timings[PHASES.len()].push(phases.iter().map(|phase| phase.nanos).sum());
            }
            let median = |values: &mut Vec<u128>| {
                values.sort_unstable();
                values[values.len() / 2]
            };
            let mut total = Measured::default();
            for (index, phase) in counted.iter().enumerate() {
                println!(
                    "{case}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    PHASES[index],
                    work_text(&phase.darkmatter),
                    dmls_text(&phase.dmls),
                    phase.allocated.allocations,
                    phase.allocated.bytes_allocated,
                    phase.allocated.peak_live_bytes,
                    median(&mut timings[index]),
                );
                total.allocated.allocations += phase.allocated.allocations;
                total.allocated.bytes_allocated += phase.allocated.bytes_allocated;
                total.allocated.peak_live_bytes = total.allocated.peak_live_bytes.max(phase.allocated.peak_live_bytes);
            }
            let darkmatter_total = counted.iter().fold(WorkSnapshot::default(), |sum, phase| sum.plus(&phase.darkmatter));
            let dmls_total = counted.iter().fold(WorkCounts::default(), |sum, phase| WorkCounts {
                reads: sum.reads + phase.dmls.reads,
                hashes: sum.hashes + phase.dmls.hashes,
                parses: sum.parses + phase.dmls.parses,
                enumerations: sum.enumerations + phase.dmls.enumerations,
                registry_scans: sum.registry_scans + phase.dmls.registry_scans,
            });
            println!(
                "{case}\ttotal\t{}\t{}\t{}\t{}\t{}\t{}",
                work_text(&darkmatter_total),
                dmls_text(&dmls_total),
                total.allocated.allocations,
                total.allocated.bytes_allocated,
                total.allocated.peak_live_bytes,
                median(&mut timings[PHASES.len()]),
            );
        }
    }
}
