//! End-to-end tests for the `agent-errors` deterministic gate
//! (`claudine-gen agent-errors check`, spec D10).
//!
//! These exercise the IO path — read the immutable seed + schema-validated research
//! frontmatter, evaluate, and atomically replace the outcome report — inside a
//! synthetic area so the fleet's explicit-status contract is proven
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
use std::process::Command;

use claudine_gen::{
    Check, GateErrorScope, GateStatus, check_agent_errors, evaluate_agent_errors,
};
use claudine_gen::agent_errors_check::{read_research, read_seed};
use darkmatter::markdown::compose::conditions::evaluate_condition_against;

/// The real claudine package area (parent of this crate's manifest dir) — the
/// source of the committed `_schema.yaml` sidecar the fixtures validate against.
fn real_area() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen crate lives under the claudine package area")
        .to_path_buf()
}

/// Builds a synthetic area with the real sidecar, an immutable seed baseline, and a research
/// document, returning `(area_root, findings_path)`.
fn scaffold_area(dir: &Path, slug: &str, seed: &str, research_doc: &str) -> PathBuf {
    let topic = dir.join("docs/research/agent-errors");
    fs::create_dir_all(&topic).unwrap();
    let seed_dir = topic.join("_seeds");
    fs::create_dir_all(&seed_dir).unwrap();

    let sidecar = real_area().join("docs/research/agent-errors/_schema.yaml");
    fs::copy(&sidecar, topic.join("_schema.yaml")).expect("copy real sidecar");

    fs::write(seed_dir.join(format!("{slug}.yaml")), seed).unwrap();
    fs::write(topic.join(format!("{slug}.md")), research_doc).unwrap();

    topic.join(".findings").join(format!("{slug}.md"))
}

fn outcome_status(path: &Path) -> String {
    let text = fs::read_to_string(path).expect("outcome report is readable");
    let markdown =
        darkmatter::markdown::Markdown::try_from_content(text).expect("outcome is Markdown");
    markdown
        .frontmatter()
        .as_map()
        .get("status")
        .and_then(serde_json::Value::as_str)
        .expect("outcome has a string status")
        .to_string()
}

fn outcome_error_scope(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).expect("outcome report is readable");
    let markdown =
        darkmatter::markdown::Markdown::try_from_content(text).expect("outcome is Markdown");
    markdown
        .frontmatter()
        .as_map()
        .get("error_scope")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn run_checker(area: &Path, slug: &str, findings: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_claudine-gen"))
        .arg("--area")
        .arg(area)
        .arg("agent-errors")
        .arg("check")
        .arg(slug)
        .arg("--findings")
        .arg(findings)
        .output()
        .expect("run claudine-gen non-interactively")
}

const SEED_BASELINE: &str = "\
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
fn clean_document_writes_explicit_clean_outcome() {
    let dir = tempfile::tempdir().unwrap();
    let findings = scaffold_area(dir.path(), "codex", SEED_BASELINE, CLEAN_DOC);

    let report = check_agent_errors(dir.path(), "codex", &findings).expect("gate runs");
    assert!(report.is_clean(), "unexpected findings: {:?}", report.findings);
    assert_eq!(report.status, GateStatus::Clean);
    assert_eq!(outcome_status(&findings), "clean");
}

#[test]
fn workspace_archived_seed_survives_graduation_and_detects_identity_changes() {
    let area = real_area();
    let facts = fs::read_to_string(area.join("docs/providers/facts/codex.yaml")).unwrap();
    assert!(
        !facts.contains("error_vocabulary:"),
        "graduation must not restore the retired facts key"
    );

    let seed = read_seed(&area, "codex")
        .expect("real seed baseline parses")
        .expect("Codex has an archived Phase-A seed");
    let mut research = read_research(&area, "codex").expect("real research parses");
    let clean = evaluate_agent_errors("codex", Some(&seed), &research);
    assert!(clean.is_clean(), "real archived baseline drifted: {:?}", clean.findings);

    research.kind_buckets[0].kind = "configuration".into();
    research.msg_buckets[0].needles.swap(0, 1);
    let changed = evaluate_agent_errors("codex", Some(&seed), &research);
    assert!(
        changed
            .findings
            .iter()
            .any(|finding| finding.check == Check::SeedRekind && finding.detail.contains("rate")),
        "real seed must detect a semantic re-kind: {:?}",
        changed.findings
    );
    assert!(
        changed.findings.iter().any(|finding| {
            finding.check == Check::SeedReorder && finding.detail.contains("rate limit")
        }),
        "real seed must detect an intra-bucket reorder: {:?}",
        changed.findings
    );
}

#[test]
fn all_archived_seeds_match_their_post_graduation_research_rows() {
    let area = real_area();
    for slug in [
        "antigravity",
        "claude",
        "codex",
        "gemini",
        "kilo",
        "kimi",
        "opencode",
        "pi",
        "qwen",
    ] {
        let seed = read_seed(&area, slug)
            .unwrap_or_else(|error| panic!("{slug}: archived seed must parse: {error}"))
            .unwrap_or_else(|| panic!("{slug}: archived seed must exist"));
        let research = read_research(&area, slug)
            .unwrap_or_else(|error| panic!("{slug}: research must parse: {error}"));
        let report = evaluate_agent_errors(slug, Some(&seed), &research);
        assert!(report.is_clean(), "{slug}: archived seed drifted: {:?}", report.findings);
    }
    assert!(
        read_seed(&area, "goose").expect("missing seed is valid").is_none(),
        "parser-less Goose must not gain a fabricated Phase-A seed"
    );
}

#[test]
fn failing_document_writes_findings_and_persists_across_reruns() {
    let dir = tempfile::tempdir().unwrap();
    let findings = scaffold_area(dir.path(), "codex", SEED_BASELINE, DROPPED_SEED_DOC);

    // First run: the dropped seed is flagged and the file is written.
    let report = check_agent_errors(dir.path(), "codex", &findings).expect("gate runs");
    assert!(!report.is_clean(), "expected findings for a dropped seed");
    assert_eq!(outcome_status(&findings), "findings");

    // Non-convergence: re-running against the still-bad document keeps the
    // machine-visible failure in place (budget exhaustion leaves this artifact).
    let rerun = check_agent_errors(dir.path(), "codex", &findings).expect("gate reruns");
    assert!(!rerun.is_clean());
    assert_eq!(outcome_status(&findings), "findings");
}

#[test]
fn corrected_document_replaces_findings_with_explicit_clean() {
    let dir = tempfile::tempdir().unwrap();
    let findings = scaffold_area(dir.path(), "codex", SEED_BASELINE, DROPPED_SEED_DOC);

    // Fail once so a stale findings file exists.
    check_agent_errors(dir.path(), "codex", &findings).unwrap();
    assert!(findings.exists());

    // The model "corrects" the document (simulated by overwriting it); the next
    // run must replace the stale findings report so success is explicit.
    let doc = dir.path().join("docs/research/agent-errors/codex.md");
    fs::write(&doc, CLEAN_DOC).unwrap();
    let report = check_agent_errors(dir.path(), "codex", &findings).expect("gate reruns");
    assert!(report.is_clean(), "corrected doc should be clean: {:?}", report.findings);
    assert_eq!(outcome_status(&findings), "clean");
}

#[test]
fn schema_invalid_document_writes_gate_error_and_never_reports_clean() {
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
    let findings = scaffold_area(dir.path(), "codex", SEED_BASELINE, bad_doc);
    let report = check_agent_errors(dir.path(), "codex", &findings).expect("gate error persists");
    assert_eq!(report.status, GateStatus::GateError);
    assert_eq!(report.error_scope, Some(GateErrorScope::ResearchDocument));
    assert!(
        report.error.as_deref().unwrap_or_default().contains("schema validation")
            || report.error.as_deref().unwrap_or_default().contains("guessed"),
        "expected a schema-validation error, got: {:?}",
        report.error
    );
    assert_eq!(outcome_status(&findings), "gate_error");
    assert_eq!(outcome_error_scope(&findings).as_deref(), Some("research_document"));
}

#[test]
fn invalid_seed_writes_terminal_gate_input_error_scope() {
    let dir = tempfile::tempdir().unwrap();
    let findings = scaffold_area(
        dir.path(),
        "codex",
        "kind_buckets: [not-a-valid-bucket]\n",
        CLEAN_DOC,
    );

    let report = check_agent_errors(dir.path(), "codex", &findings)
        .expect("authoritative input error persists");
    assert_eq!(report.status, GateStatus::GateError);
    assert_eq!(report.error_scope, Some(GateErrorScope::GateInput));
    assert_eq!(outcome_error_scope(&findings).as_deref(), Some("gate_input"));
}

#[test]
fn checker_process_schema_failure_emits_gate_error_not_clean_success() {
    let dir = tempfile::tempdir().unwrap();
    let bad_doc = CLEAN_DOC.replace("evidence: documented", "evidence: guessed");
    let findings = scaffold_area(dir.path(), "codex", SEED_BASELINE, &bad_doc);

    let output = run_checker(dir.path(), "codex", &findings);
    assert!(output.status.success(), "persisted gate_error is a branchable outcome");
    assert_eq!(outcome_status(&findings), "gate_error");
    assert_eq!(outcome_error_scope(&findings).as_deref(), Some("research_document"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("research clean"), "schema failure must not print clean: {stdout}");
}

#[test]
fn checker_process_report_write_failure_is_nonzero_and_never_prints_clean() {
    let dir = tempfile::tempdir().unwrap();
    let normal = scaffold_area(dir.path(), "codex", SEED_BASELINE, CLEAN_DOC);
    let blocker = normal.parent().unwrap().parent().unwrap().join("blocker");
    fs::write(&blocker, "not a directory").unwrap();
    let unwritable = blocker.join("codex.md");

    let output = run_checker(dir.path(), "codex", &unwritable);
    assert!(!output.status.success(), "report persistence failure must stop lifecycle");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("research clean"), "write failure must not print clean: {stdout}");
    assert!(!unwritable.exists());
}

#[test]
fn fleet_clean_action_requires_explicit_clean_status() {
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("outcome.md");
    let data = serde_json::json!({"findings": report.display().to_string()});
    let fleet_text = fs::read_to_string(real_area().join("docs/research/agent-errors/_fleet.md"))
        .expect("read fleet lifecycle");
    assert!(
        !fleet_text.contains("no_error: true"),
        "report-write failure must stop lifecycle before stale status checks"
    );
    let fleet = darkmatter::markdown::Markdown::try_from_content(fleet_text)
        .expect("fleet is valid Markdown");
    let stack = fleet
        .frontmatter()
        .as_map()
        .get("success")
        .and_then(|value| value.get("stack"))
        .and_then(serde_json::Value::as_array)
        .expect("fleet has a success stack");
    let conditions: Vec<&str> = stack
        .iter()
        .filter_map(|item| item.get("when").and_then(serde_json::Value::as_str))
        .collect();
    let clean_condition = "frontmatter(findings, 'status') == 'clean'";
    let findings_condition = "frontmatter(findings, 'status') == 'findings'";
    let repairable_gate_error_condition = "frontmatter(findings, 'status') == 'gate_error' && frontmatter(findings, 'error_scope') == 'research_document'";
    let gate_input_condition = "frontmatter(findings, 'status') == 'gate_error' && frontmatter(findings, 'error_scope') == 'gate_input'";
    let unknown_scope_condition = "frontmatter(findings, 'status') == 'gate_error' && frontmatter(findings, 'error_scope') != 'research_document' && frontmatter(findings, 'error_scope') != 'gate_input'";
    assert!(conditions.contains(&clean_condition));
    assert!(conditions.contains(&findings_condition));
    assert!(conditions.contains(&repairable_gate_error_condition));
    assert!(conditions.contains(&gate_input_condition));
    assert!(conditions.contains(&unknown_scope_condition));
    assert!(conditions.contains(&"!file_exists(findings)"));

    for (status, scope, expected_clean, expected_findings, expected_repair, expected_input) in [
        ("clean", None, true, false, false, false),
        ("findings", None, false, true, false, false),
        (
            "gate_error",
            Some("research_document"),
            false,
            false,
            true,
            false,
        ),
        ("gate_error", Some("gate_input"), false, false, false, true),
    ] {
        let scope = scope.map(|value| format!("error_scope: {value}\n")).unwrap_or_default();
        fs::write(
            &report,
            format!("---\nstatus: {status}\nprovider: codex\n{scope}---\n# Outcome\n"),
        )
        .unwrap();
        assert_eq!(
            evaluate_condition_against(clean_condition, &data, dir.path()).unwrap(),
            expected_clean
        );
        assert_eq!(
            evaluate_condition_against(findings_condition, &data, dir.path()).unwrap(),
            expected_findings
        );
        assert_eq!(
            evaluate_condition_against(repairable_gate_error_condition, &data, dir.path())
                .unwrap(),
            expected_repair
        );
        assert_eq!(
            evaluate_condition_against(gate_input_condition, &data, dir.path()).unwrap(),
            expected_input
        );
    }

    fs::write(
        &report,
        "---\nstatus: gate_error\nprovider: codex\nerror_scope: unexpected\n---\n# Outcome\n",
    )
    .unwrap();
    assert!(evaluate_condition_against(unknown_scope_condition, &data, dir.path()).unwrap());

    fs::remove_file(&report).unwrap();
    assert!(!report.exists(), "absence is an error branch, never clean");
    assert!(
        !evaluate_condition_against("file_exists(findings)", &data, dir.path()).unwrap()
    );
}
