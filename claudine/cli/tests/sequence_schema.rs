#![cfg(unix)]

//! Integration tests for `claudine sequence` schema validation.
//!
//! Split out of `sequence_cli.rs`: covers cross-step required-property
//! aggregation, set-override satisfaction, unsupported-shape typed
//! errors, per-step `step_timeout` override, and post-shell-expansion
//! schema validation.

use predicates::str::contains;
use std::fs;
mod common;
use common::{CliProcessFixture, strip_ansi, write_executable};

// ============================================================================
// Phase 5: schema validation
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_aggregates_missing_required_properties_across_steps() {
    // A non-TTY sequence run with `$schema` declaring a required property
    // that no step supplies must abort BEFORE any provider session is
    // launched, surfacing every failing step in a single error.
    let fixture = CliProcessFixture::named("sequence-unsupported-shape");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  topic: 'string(required)'
sequence:
  - alpha
  - beta
---
Step about {{topic}}.
"#,
    )
    .unwrap();

    // Provider stub records every invocation so we can prove it was
    // never called.
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture
        .command()
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    // Strong assertion: error must be Claudine's typed CompositionError
    // surface AND per-step aggregated (`sequence missing properties`),
    // not Darkmatter's raw MarkdownError or a single-doc MissingProperties.
    // Catches regressions where the per-step preflight short-circuits
    // aggregation.
    assert!(
        plain.contains("CompositionError"),
        "expected typed CompositionError surface (not raw MarkdownError); stderr:\n{plain}"
    );
    assert!(
        plain.to_lowercase().contains("sequence missing properties"),
        "expected aggregated `sequence missing properties` (not single-doc); stderr:\n{plain}"
    );
    assert!(
        !plain.contains("MarkdownError"),
        "raw MarkdownError leaked instead of typed Claudine error; stderr:\n{plain}"
    );
    // Both steps must appear in the aggregated report.
    assert!(
        plain.contains("Step 1"),
        "expected per-step `Step 1` header; stderr:\n{plain}"
    );
    assert!(
        plain.contains("Step 2"),
        "expected per-step `Step 2` header; stderr:\n{plain}"
    );
    assert!(
        plain.contains("alpha") && plain.contains("beta"),
        "expected both step names in the aggregated report; stderr:\n{plain}"
    );
    assert!(
        plain.contains("topic"),
        "expected the `topic` property name; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched; stub recorded a call"
    );
}

#[cfg(unix)]
#[test]
fn sequence_set_override_satisfies_required_schema() {
    // The aggregated path retries cleanly when the user supplies the
    // missing value via `--set` so no error reaches the provider.
    let fixture = CliProcessFixture::named("sequence-set-override");

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  topic: 'string(required)'
sequence:
  - one
  - two
---
Working on {{topic}} ({{state}}).
"#,
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\ncat > /dev/null\nexit 0\n",
    );

    fixture
        .command()
        .args([
            "sequence",
            "--goose",
            md_file.to_str().unwrap(),
            "topic=async",
        ])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn sequence_unsupported_shape_surfaces_typed_error_under_tty_pref() {
    // When the run is non-TTY (stdin/stderr are pipes here), the sequence
    // path MUST emit an aggregated `sequence missing properties` report
    // rather than promoting the first unsupported shape to
    // `UnsupportedInteractiveSchema`. This matches the direct `compose`
    // path's behavior in `pre_validate_with_interactive_collection`,
    // which only short-circuits to the unsupported error when interactive
    // collection is actually allowed.
    //
    // Regression target (review-5 medium): we previously promoted the
    // unsupported shape unconditionally, which forced users to see a
    // shape-specific error even when they could have fixed the sequence
    // by editing two files in one pass via the aggregated report.
    let fixture = CliProcessFixture::named("sequence-step-timeout-override");
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  config: 'object(required)'
sequence:
  - alpha
  - beta
---
Step about {{config}}.
"#,
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture
        .command()
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    // Strong assertions: non-TTY must produce the aggregated
    // sequence-missing surface, NOT the unsupported-shape error.
    assert!(
        plain.to_lowercase().contains("sequence missing properties"),
        "expected aggregated `sequence missing properties` surface; stderr:\n{plain}"
    );
    assert!(
        !plain.to_lowercase().contains("unsupported interactive schema"),
        "non-TTY must NOT short-circuit to UnsupportedInteractiveSchema; stderr:\n{plain}"
    );
    // Both step labels should appear in the aggregated report so a user
    // can fix the full sequence in one pass.
    assert!(
        plain.contains("Step 1") && plain.contains("Step 2"),
        "expected per-step `Step 1` and `Step 2` headers; stderr:\n{plain}"
    );
    assert!(
        plain.contains("alpha") && plain.contains("beta"),
        "expected both step names; stderr:\n{plain}"
    );
    assert!(
        plain.contains("config"),
        "expected the `config` property name in error report; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched; stub recorded a call"
    );
}

// ============================================================================
// Per-step step_timeout override
// ============================================================================

/// Verifies that a step-level `step_timeout` (declared in a sequence step's
/// raw state object) overrides the document-level `step_timeout`. The step
/// overlay passes the step's raw state unchanged under the `state` key, so
/// per-step `step_timeout` is surfaced via `{{ state.step_timeout || ... }}`
/// interpolation in the document frontmatter.
///
/// Test shape:
/// - Document `step_timeout` falls back to `30s` when the step does not
///   declare one, but uses `state.step_timeout` when the step does.
/// - Step 1 has no `step_timeout`, so the effective deadline is `30s`. The
///   fake provider completes quickly and the step succeeds.
/// - Step 2 declares `step_timeout: 0.5s` at the step level. The fake
///   provider emits a start event and then stalls, so the step is killed with
///   a `step_timeout` error well before the 30s document fallback would.
#[cfg(unix)]
#[test]
fn sequence_per_step_step_timeout_override() {
    let fixture = CliProcessFixture::named("sequence-missing-required");
    let count_path = fixture.cwd().join("call-count.txt");

    // Document-level step_timeout defaults to 30s; a step may override by
    // setting `step_timeout` at the step level, which the overlay exposes
    // via `state.step_timeout`. Step 2 sets it to 0.5s: the stall below is
    // unconditional, so the budget only has to outlast the gap between the
    // fixture's two `printf` lines (back-to-back, no sleep) — 0.5s is two
    // orders of magnitude above that even under WSL2 contention, and it
    // costs 0.5s + one 0.1s tick instead of a whole second.
    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        r#"---
step_timeout: '{{ state.step_timeout || "30s" }}'
sequence:
  - name: fast
  - name: slow
    step_timeout: 0.5s
---
Run step {{ state.name }}
"#,
    )
    .unwrap();

    // Fake opencode tracks invocation count. Step 1 emits a structured
    // session quickly and exits; step 2 emits a start event only and then
    // sleeps well past the step-level 0.5s deadline. The wait loop must
    // terminate the silent child without letting the test block for the
    // document-level 30s fallback.
    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
    echo '[]'
    exit 0
fi
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"

if [ "$count" = "1" ]; then
  printf '%s\n' '{"type":"step_start","sessionID":"ses_step1"}'
  printf '%s\n' '{"type":"text","text":"fast step done"}'
  printf '%s\n' '{"type":"step_complete","usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2},"duration_ms":10}'
  exit 0
fi

# Step 2: emit start + finish so both `last_event_at` and
# `provider_status` are populated, then stall so the step-silence
# deadline fires. The OpenCode `provider_status` grace requires at
# least one observed `step_finish` boundary before allowing
# `step_timeout` to fire.
printf '%s\n' '{"type":"step_start","sessionID":"ses_step2"}'
printf '%s\n' '{"type":"step_finish","sessionID":"ses_step2","part":{"reason":"tool-calls","tokens":{"input":1,"output":1,"total":2}}}'
sleep 30
exit 0
"#,
    );

    let run_start = std::time::Instant::now();
    let assert = fixture
        .command()
        .env("CLAUDINE_WATCHDOG_INTERVAL", "0.1s")
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .timeout(std::time::Duration::from_secs(10))
        .args(["sequence", "--opencode", md_file.to_str().unwrap()])
        .assert()
        .failure();
    let elapsed = run_start.elapsed();

    // The step-level 0.5s budget must win well before the 30s document
    // fallback. The bound is 4s rather than the fallback itself so it also
    // catches the budget silently reverting to a whole-second value: the
    // whole run — prep, step 1, step 2's budget + 0.1s tick + termination —
    // lands near 1.4s locally and 1.0–1.5s on the slowest CI environment.
    assert!(
        elapsed < std::time::Duration::from_secs(4),
        "per-step step_timeout override should fire quickly; run took {elapsed:?}"
    );

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "2",
        "both steps should be launched before the stall on step 2 is detected"
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("1 succeeded") && plain.contains("1 failed"),
        "summary should record step 1 success and step 2 failure; stderr: {plain}"
    );
}

// ============================================================================
// Post-shell-expansion schema validation in sequence (review-6 high finding)
// ============================================================================

#[cfg(unix)]
fn write_shell_whitelist(home: &std::path::Path, prefixes: &[&str]) {
    let body: String = prefixes.iter().map(|p| format!("prefix {p}\n")).collect();
    fs::write(home.join(".darkmatter-shell-whitelist"), body).unwrap();
}

#[cfg(unix)]
#[test]
fn sequence_shell_expanded_value_violating_schema_aborts_step() {
    // A sequence step whose post-shell effective frontmatter violates the
    // schema must surface a SchemaValidation error and not launch the
    // provider for that step.
    let fixture = CliProcessFixture::named("sequence-shell-schema-violation");
    write_shell_whitelist(fixture.cwd(), &["echo"]);
    let count_path = fixture.cwd().join("call-count.txt");

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        r#"---
$schema:
  tier: 'enum(small, medium, large; required)'
sequence:
  - alpha
tier: $(echo huge)
---
Step {{state}} with tier {{tier}}.
"#,
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture
        .command()
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.to_lowercase().contains("schema validation"),
        "expected SchemaValidation surface for shell-expanded sequence step; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider launch should occur on post-shell invalid value"
    );
    // Silence unused linter for the predicates import that other tests use.
    let _ = contains("");
}

#[cfg(unix)]
#[test]
fn sequence_malformed_step_document_preserves_frontmatter_mismatch() {
    let fixture = CliProcessFixture::named("sequence-malformed-step-doc");
    let count_path = fixture.cwd().join("call-count.txt");

    let valid_doc = fixture.cwd().join("valid.md");
    fs::write(&valid_doc, "---\ntitle: Valid\n---\n# Valid\n").unwrap();

    let malformed_doc = fixture.cwd().join("bad.md");
    fs::write(
        &malformed_doc,
        "----\ntitle: Bad step document\n----\n# Should not compose\n",
    )
    .unwrap();

    let md_file = fixture.cwd().join("seq.md");
    fs::write(
        &md_file,
        format!(
            r#"---
sequence:
  - name: valid
    doc: {}
  - name: malformed
    doc: {}
loaded_title: "{{{{ frontmatter(state.doc, 'title') }}}}"
---
Step {{{{ state.name }}}}: {{{{ loaded_title }}}}
"#,
            valid_doc.display(),
            malformed_doc.display()
        ),
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = fixture
        .command()
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    // The prose renderer hard-wraps the error inside a box at the terminal
    // width, so the message can be split across rows. Drop the box borders and
    // collapse whitespace runs before matching — the assertion is that the
    // typed fence error reaches the operator, not that it fits on one line.
    let collapsed = plain
        .replace('┃', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        collapsed.contains("frontmatter fence must be exactly")
            || collapsed.contains("FrontmatterFenceMismatch")
            || collapsed.contains("exactly three dashes"),
        "expected typed malformed-fence error from the step document; stderr:\n{plain}"
    );
    assert!(
        collapsed.contains("bad.md"),
        "expected the malformed step document path in the error; stderr:\n{plain}"
    );
    assert!(
        !plain.contains("missing properties")
            && !plain.contains("No model specified")
            && !plain.contains("No runnable providers"),
        "malformed step document must not degrade to a generic sequence/provider error; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should launch when a step document is malformed"
    );
}
