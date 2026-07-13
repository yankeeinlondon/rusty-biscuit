//! End-to-end tests for the `agent-errors` deterministic gate
//! (`claudine-gen agent-errors check`, spec D10).
//!
//! These exercise the IO path — read the facts seed + schema-validated research
//! frontmatter, evaluate, and reconcile the findings file — inside a synthetic
//! area so the fleet's stale-first / write-on-failure contract is proven
//! without running a provider. The pure check logic is unit-tested in
//! `gen/src/agent_errors_check.rs`.
//!
//! The resume half of the pattern (one resume → correction, budget exhaustion,
//! and the unsupported-wrapper gate) is the composition lifecycle's, already
//! covered by `claudine-cli`'s
//! `commands::wrap::harness_orch::loop_control::tests` (`dispatch_resume_*`,
//! `dispatch_retry_exhausts_after_budget`, `check_resume_support`). This gate
//! supplies the mechanical findings those tests consume.

use std::fs;
use std::path::{Path, PathBuf};

use claudine_gen::check_agent_errors;

/// The real claudine package area (parent of this crate's manifest dir) — the
/// source of the committed `_schema.yaml` sidecar the fixtures validate against.
fn real_area() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen crate lives under the claudine package area")
        .to_path_buf()
}

/// Builds a synthetic area with the real sidecar, a facts seed, and a research
/// document, returning `(area_root, findings_path)`.
fn scaffold_area(dir: &Path, slug: &str, facts: &str, research_doc: &str) -> PathBuf {
    let topic = dir.join("docs/research/agent-errors");
    fs::create_dir_all(&topic).unwrap();
    let facts_dir = dir.join("docs/providers/facts");
    fs::create_dir_all(&facts_dir).unwrap();

    let sidecar = real_area().join("docs/research/agent-errors/_schema.yaml");
    fs::copy(&sidecar, topic.join("_schema.yaml")).expect("copy real sidecar");

    fs::write(facts_dir.join(format!("{slug}.yaml")), facts).unwrap();
    fs::write(topic.join(format!("{slug}.md")), research_doc).unwrap();

    topic.join(".findings").join(format!("{slug}.json"))
}

const SEED_FACTS: &str = "\
error_vocabulary:
  kind_buckets:
  - kind: api_remote
    needles: [rate, quota]
  msg_buckets:
  - kind: configuration
    needles: [api key]
";

/// A research doc that preserves the seed, cites a documented capacity
/// addition, and covers the motivating class — the gate should find nothing.
const CLEAN_DOC: &str = "\
---
$schema: ./_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: opencode
model: default
kind_buckets:
  - kind: api_remote
    needles:
      - text: rate
        evidence: seed
      - text: quota
        evidence: seed
      - text: overloaded
        evidence: documented
        source: https://example.com/docs
msg_buckets:
  - kind: configuration
    needles:
      - text: api key
        evidence: seed
changes: []
requires_claudine_update: false
---
# Clean
";

/// The clean doc with the seeded `quota` needle dropped — a seed-preservation
/// failure the gate must flag.
const DROPPED_SEED_DOC: &str = "\
---
$schema: ./_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: opencode
model: default
kind_buckets:
  - kind: api_remote
    needles:
      - text: rate
        evidence: seed
      - text: overloaded
        evidence: documented
        source: https://example.com/docs
msg_buckets:
  - kind: configuration
    needles:
      - text: api key
        evidence: seed
changes: []
requires_claudine_update: false
---
# Dropped seed
";

#[test]
fn clean_document_writes_no_findings() {
    let dir = tempfile::tempdir().unwrap();
    let findings = scaffold_area(dir.path(), "codex", SEED_FACTS, CLEAN_DOC);

    let report = check_agent_errors(dir.path(), "codex", &findings).expect("gate runs");
    assert!(report.is_clean(), "unexpected findings: {:?}", report.findings);
    assert!(!findings.exists(), "a clean run must not write a findings file");
}

#[test]
fn failing_document_writes_findings_and_persists_across_reruns() {
    let dir = tempfile::tempdir().unwrap();
    let findings = scaffold_area(dir.path(), "codex", SEED_FACTS, DROPPED_SEED_DOC);

    // First run: the dropped seed is flagged and the file is written.
    let report = check_agent_errors(dir.path(), "codex", &findings).expect("gate runs");
    assert!(!report.is_clean(), "expected findings for a dropped seed");
    assert!(findings.exists(), "findings file must be written on failure");

    // Non-convergence: re-running against the still-bad document keeps the
    // machine-visible failure in place (budget exhaustion leaves this artifact).
    let rerun = check_agent_errors(dir.path(), "codex", &findings).expect("gate reruns");
    assert!(!rerun.is_clean());
    assert!(findings.exists(), "a still-failing document keeps its findings file");
}

#[test]
fn corrected_document_removes_stale_findings() {
    let dir = tempfile::tempdir().unwrap();
    let findings = scaffold_area(dir.path(), "codex", SEED_FACTS, DROPPED_SEED_DOC);

    // Fail once so a stale findings file exists.
    check_agent_errors(dir.path(), "codex", &findings).unwrap();
    assert!(findings.exists());

    // The model "corrects" the document (simulated by overwriting it); the next
    // run must remove the stale findings file so success is unambiguous.
    let doc = dir.path().join("docs/research/agent-errors/codex.md");
    fs::write(&doc, CLEAN_DOC).unwrap();
    let report = check_agent_errors(dir.path(), "codex", &findings).expect("gate reruns");
    assert!(report.is_clean(), "corrected doc should be clean: {:?}", report.findings);
    assert!(!findings.exists(), "the corrected run must remove the stale findings file");
}

#[test]
fn schema_invalid_document_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    // `evidence: guessed` is not a provenance enum member — schema validation
    // (not the gate's own checks) must reject the document.
    let bad_doc = "\
---
$schema: ./_schema.yaml
last_updated: 2026-07-12
agent: opencode
model: default
msg_buckets:
  - kind: api_remote
    needles:
      - text: rate limit
        evidence: guessed
changes: []
requires_claudine_update: false
---
# Bad
";
    let findings = scaffold_area(dir.path(), "codex", SEED_FACTS, bad_doc);
    let err = check_agent_errors(dir.path(), "codex", &findings).unwrap_err();
    assert!(
        err.to_string().contains("schema validation") || err.to_string().contains("guessed"),
        "expected a schema-validation error, got: {err}"
    );
}
