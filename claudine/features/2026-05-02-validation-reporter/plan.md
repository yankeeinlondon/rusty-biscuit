---
phases: 6
created: 2026-05-02
start_phase: 1
spec: claudine/features/2026-05-02-validation-reporter/spec.md
owner: claudine
source_files_during_phase_1:
  - claudine/lib/src/harness/model.rs
  - claudine/lib/src/harness/parse.rs
  - claudine/lib/src/harness/validate.rs
  - claudine/lib/src/harness/report.rs
  - claudine/lib/src/harness/audit.rs
  - claudine/lib/src/composition/preflight.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/harness/parse.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/harness/validate.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/lib/src/harness/report.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
  - claudine
  - claudine-cli
---

# Plan — Improve Harness Validation Reporter Quality

Source spec: [`spec.md`](./spec.md).

## Summary

Today `report_check_outcomes` in
[`claudine/lib/src/harness/report.rs`](../../lib/src/harness/report.rs)
collapses every check (pass or fail) to a single `Status` line driven by the
rule's `markup`, and silently throws away `failure_message`. The result for a
failing rule is a positive assertion paired with a red glyph
("✗ the file Cargo.toml exists") with no source location, no rule body, and no
diagnostic.

The fix is in four pieces:

1. **Model** — add an optional `RuleSource { file, line_range, yaml_snippet }`
   on `ValidationRule`, cloned forward onto `ValidationCheckOutcome`.
2. **Parser** — populate `RuleSource` during `parse_harness_plan`. Carry the
   raw frontmatter YAML text through so we can attempt a line-range lookup;
   `yaml_snippet` is produced by re-serializing the rule's value with
   `serde_yaml_ng`.
3. **Reporter** — in `report_check_outcomes`, branch on `outcome.passed`:
   keep today's compact line for passes; render a four-section failure block
   (status header, source line, YAML snippet, reason) for failures.
4. **YAML rendering** — reuse darkmatter's
   [`YamlBlock`](../../../darkmatter/lib/src/markdown/yaml_block.rs) so
   the snippet is syntax-highlighted by the same path that already renders
   ` ```yaml ` fences in markdown.

Open Question 1 (line-range recovery) is resolved pragmatically: do a
best-effort textual scan of the raw frontmatter text for the rule's anchor
key. If we cannot pin a range, `line_range = None` and the source line drops
the `:<range>` suffix. This keeps the first pass scoped without blocking the
file/snippet/reason improvements.

Open Questions 2–4 are accepted as-recommended in the spec: one block per
failure (no batching), muted "Reason:" styling, always-on for failures.

## Touch List

| File | Why |
|---|---|
| `claudine/lib/src/harness/model.rs` | Add `RuleSource`, extend `ValidationRule`, extend `ValidationCheckOutcome` |
| `claudine/lib/src/harness/parse.rs` | Populate `RuleSource` (file + snippet + best-effort range); accept raw frontmatter text |
| `claudine/lib/src/harness/validate.rs` | Plumb `source` clone into `ValidationCheckOutcome` |
| `claudine/lib/src/harness/report.rs` | Add `render_failure_block`; branch in `report_check_outcomes` |
| `claudine/cli/src/commands/wrap/composition.rs` | Pass raw frontmatter text into `parse_harness_plan` |
| `claudine/lib/src/harness/parse.rs` (tests) | Cover `RuleSource` population |
| `claudine/lib/src/harness/report.rs` (tests) | Cover failure-block path and pass-path unchanged |

## Phases

Each phase ends in a green build + targeted tests for the touched modules.

---

### Phase 1 — Model: introduce `RuleSource` and propagate fields

**Goal.** Data-model changes only. No behavior change. The reporter, parser,
and validate engine compile cleanly with `source: None` everywhere.

**Steps.**

1. In [`claudine/lib/src/harness/model.rs`](../../lib/src/harness/model.rs):
   - Add a new `RuleSource` struct near `ValidationRule`:
     ```rust
     #[derive(Debug, Clone)]
     pub struct RuleSource {
         pub file: PathBuf,
         pub line_range: Option<std::ops::RangeInclusive<usize>>,
         pub yaml_snippet: String,
     }
     ```
   - Add field `pub source: Option<RuleSource>` on `ValidationRule`.
   - Add field `pub source: Option<RuleSource>` on `ValidationCheckOutcome`.
   - Re-export `RuleSource` from `harness::mod` if it isn't picked up via
     `model::*` already (check the existing re-export pattern).

2. Fix every existing `ValidationRule { ... }` literal in the workspace to
   set `source: None`. Known sites:
   - `parse.rs` — `inline_writability_pre_check`
   - `parse.rs` — the `Ok(ValidationRule { ... })` in `parse_single_validation`
     (line ~423)
   - `validate.rs` test helper `make_rule`
   - any other test fixtures uncovered by `cargo build -p claudine`

3. Fix every `ValidationCheckOutcome { ... }` literal:
   - `validate.rs::run_checks` push site
   - `report.rs` test fixtures (multiple)

4. `cargo build -p claudine && cargo build -p claudine-cli`.

**Validation checkpoint.**
- `cargo build -p claudine` clean.
- `cargo build -p claudine-cli` clean.
- `cargo test -p claudine harness::` passes (existing tests untouched
  semantically — they all should still pass with `source: None`).
- `cargo clippy -p claudine -- -D warnings` clean for the touched files.

---

### Phase 2 — Parser: populate `RuleSource` (file + yaml_snippet)

**Goal.** Every author-declared `pre_checks` / `post_checks` rule comes out of
`parse_harness_plan` with `source: Some(RuleSource { file, line_range: None,
yaml_snippet })`. The system-owned inline writability rule keeps `source: None`
(it has no markdown origin).

**Steps.**

1. In `parse.rs::parse_validation_kind` we already have access to the rule
   `value: &Value`. Add a helper:
   ```rust
   fn build_rule_source(
       source_path: &Path,
       rule_name: &str,
       value: &serde_json::Value,
   ) -> Option<RuleSource> {
       // Re-serialize the single-rule object so the snippet matches the
       // YAML form authors actually wrote (key + value).
       let mut map = serde_yaml_ng::Mapping::new();
       map.insert(
           serde_yaml_ng::Value::String(rule_name.to_string()),
           serde_yaml_ng::to_value(value).ok()?,
       );
       let yaml_snippet = serde_yaml_ng::to_string(
           &serde_yaml_ng::Value::Mapping(map),
       ).ok()?;
       Some(RuleSource {
           file: source_path.to_path_buf(),
           line_range: None,
           yaml_snippet,
       })
   }
   ```
   (Phase 5 may revisit `line_range` recovery; for Phase 2 leave it `None`.)

2. In `parse_single_validation`, after the `parse_validation_kind` call,
   compute `let source = build_rule_source(source_path, name, value);` and
   assign it on the constructed `ValidationRule`.

3. Confirm `inline_writability_pre_check` still constructs with `source: None`.

4. Add a unit test in `parse.rs` `mod tests`:
   ```rust
   #[test]
   fn parse_rules_carry_source_with_yaml_snippet() {
       // Build minimal frontmatter with one file_exists rule and assert
       // that plan.pre_checks[0].source.is_some() and that the snippet
       // contains "file_exists" and the file path token.
   }
   ```

**Validation checkpoint.**
- `cargo test -p claudine harness::parse::tests` passes.
- New `parse_rules_carry_source_with_yaml_snippet` passes.
- `cargo build -p claudine-cli` clean.
- No serde_yaml_ng dep added if not already in `claudine/lib/Cargo.toml` —
  `biscuit-file` re-exports it (`biscuit_file::serde_yaml_ng`); use that
  re-export to avoid a new direct dep.

---

### Phase 3 — Validate: propagate `source` onto outcomes

**Goal.** `ValidationCheckOutcome` instances produced by `run_checks` carry a
clone of `rule.source`.

**Steps.**

1. In [`validate.rs::run_checks`](../../lib/src/harness/validate.rs), where we
   push `ValidationCheckOutcome`, add `source: rule.source.clone()` to the
   struct literal.

2. No other touches in `validate.rs` for this phase.

**Validation checkpoint.**
- `cargo test -p claudine harness::validate::tests` passes (the test fixture
  rules all have `source: None`, so outcomes also have `source: None` — the
  reporter still falls back to legacy single-line output, which Phase 4 will
  exercise).

---

### Phase 4 — Reporter: failure block renderer

**Goal.** Failing checks render the spec's four-section block. Passing checks
keep today's single-line `Status` row. Outcomes with `source: None` fall back
to the legacy single-line failure rendering (so existing tests and
programmatically constructed rules still produce sensible output).

**Steps.**

1. In `report.rs`, add a `FailurePhase`-aware status header helper:
   ```rust
   fn failure_header_text(phase: FailurePhase) -> &'static str {
       match phase {
           FailurePhase::PreCheck => "Pre-validation failed",
           FailurePhase::PostCheck => "Post-validation failed",
           FailurePhase::Agent => "Agent execution failed",
           FailurePhase::ShellAudit => "Shell audit failed",
       }
   }
   ```

2. Add the failure-block renderer:
   ```rust
   fn render_failure_block(
       outcome: &ValidationCheckOutcome,
       phase: FailurePhase,
       term: &Terminal,
   ) {
       // Section 1: status header (red glyph + plain summary)
       emit_status(failure_header_text(phase), StatusState::Failure, term);

       // Section 2: source location, OSC8-linked when present
       if let Some(src) = &outcome.source {
           let abs = src.file.display().to_string();
           let display = relative_or_abs(&src.file);
           let display_escaped = prose_escape(&display);
           let abs_escaped = prose_escape(&abs);
           let suffix = match &src.line_range {
               Some(r) => format!(":{}-{}", r.start(), r.end()),
               None => String::new(),
           };
           emit_status(
               &format!(
                   "in <a href=\"{abs_escaped}\">{display_escaped}{suffix}</a>",
                   suffix = prose_escape(&suffix),
               ),
               StatusState::Info,
               term,
           );
       }

       // Section 3: YAML snippet via darkmatter::markdown::YamlBlock
       if let Some(src) = &outcome.source
           && let Ok(block) = darkmatter::markdown::YamlBlock::new(
               src.yaml_snippet.trim_end(),
           )
       {
           let rendered = block.render(term);
           // Indent two spaces past the status glyph column.
           for line in rendered.lines() {
               eprintln!("    {line}");
           }
       }

       // Section 4: reason line (muted; the glyph already carries severity)
       if let Some(reason) = &outcome.failure_message {
           let escaped = prose_escape(reason);
           eprintln!(
               "  Reason: <gray-500>{escaped}</gray-500>"
           );
           // Note: emit through Prose if `gray-500` is not raw-printable;
           // see step 4 below for the exact rendering helper.
       }
   }
   ```
   (Pseudocode — finalize the exact `Prose` / `BlockQuote` integration in step
   4 once the helper signature lands.)

3. Replace `report_check_outcomes` body so it dispatches:
   ```rust
   pub fn report_check_outcomes(report: &ValidationPhaseReport, term: &Terminal) {
       for outcome in &report.outcomes {
           if outcome.passed {
               emit_status(&outcome.markup, StatusState::Success, term);
           } else if outcome.source.is_some() {
               render_failure_block(outcome, report.phase, term);
           } else {
               // Legacy fallback: no source available (e.g. system-owned
               // inline writability rule, or programmatically constructed
               // test rules).
               emit_status(&outcome.markup, StatusState::Failure, term);
               if let Some(reason) = &outcome.failure_message {
                   eprintln!("  Reason: {}", reason);
               }
           }
       }
   }
   ```

4. Render the muted "Reason:" line through the same `Prose`/`Renderable`
   surface used elsewhere in `report.rs` rather than raw `eprintln!`. Use
   whatever style key the existing reporter already has for muted text
   (search `report.rs` and adjacent presentation modules for `gray`,
   `muted`, or a `BlockQuote` precedent before inventing one). Keep
   the styling subdued — the spec calls out that the glyph already carries
   severity.

5. Add a small helper `relative_or_abs(path: &Path) -> String` that produces
   a `cwd`-relative display string when possible, falling back to absolute.
   Re-use any existing helper in the harness/report layer if one exists; do
   not invent a new one if there is a pre-existing convention.

**Validation checkpoint.**
- `cargo build -p claudine` clean.
- `cargo test -p claudine harness::report::tests` passes (existing tests
  still cover the pass path and the legacy-fallback failure path; new tests
  arrive in Phase 5).
- Manual smoke: hand-construct a failing `ValidationCheckOutcome` with
  `source: Some(...)` in a scratch test and confirm the four-section block
  renders without ANSI corruption when `Terminal::new_optimistic(80)` is
  used.

---

### Phase 5 — Tests: failure-block coverage and pass-path regression

**Goal.** Lock in the new behavior and prevent regressions of the two key
invariants: pass-path stays compact; failure-path emits source + YAML +
reason when source is present.

**Steps.**

1. Add to `report.rs::tests`:
   - `report_check_outcomes_failure_with_source_emits_block` — construct an
     outcome with `passed: false`, `source: Some(RuleSource { file: ...,
     yaml_snippet: "file_exists: \"Cargo.toml\"\n", line_range: None })`,
     and `failure_message: Some("file does not exist: ...".into())`.
     Assert no panic and (where capturable) inspect that
     `render_failure_block` did not regress to the legacy single-line path.
     Capturing stderr in unit tests is fragile here — the assertion is
     primarily smoke (no panic, terminal-safe under
     `Terminal::new_optimistic(80)`).
   - `report_check_outcomes_failure_without_source_uses_legacy_path` —
     same shape but `source: None` and `failure_message: Some(...)`.
     Confirms the fallback emits both the markup line and the reason.
   - `report_check_outcomes_pass_path_unchanged` — explicit regression
     guard: `passed: true` produces exactly one `Status` line via
     `emit_status` (validate by absence of indentation/Reason in any
     captured output, or by test that the function returns without
     entering the failure branches — depending on how capture is wired).

2. Add to `parse.rs::tests`:
   - `parse_rules_source_field_uses_source_path` — verify
     `plan.pre_checks[0].source.as_ref().unwrap().file == source()`.
   - `parse_rules_yaml_snippet_round_trips` — verify the snippet
     deserializes back to a YAML mapping whose only key is the rule name.

3. Add to `validate.rs::tests`:
   - `outcome_carries_rule_source_clone` — build a `ValidationRule` with
     `source: Some(RuleSource { ... })`, run `run_checks` against it, and
     assert the produced `ValidationCheckOutcome.source` is `Some` with
     equal contents.

4. Run the full harness module test slice:
   ```
   cargo test -p claudine harness::
   ```

**Validation checkpoint.**
- All new tests pass.
- All pre-existing `harness::*` tests pass without modification.
- `cargo clippy -p claudine -- -D warnings` clean.

---

### Phase 6 — End-to-end check + docs

**Goal.** Confirm the new failure block looks right when produced by the real
composition pipeline, and update the topic docs / skill notes to reflect the
new reporter behavior.

**Steps.**

1. Build the CLI and exercise the failure path manually:
   ```
   cargo build -p claudine-cli
   ```
   Then run a `claudine compose` (or `inline-compose`) against a temp
   markdown harness whose frontmatter contains a deliberately failing
   `pre_checks: { file_exists: "/definitely/not/here.toml" }`. Verify on
   stderr:
   - red ✗ glyph + "Pre-validation failed"
   - source line: `in <relative path>` (OSC8-linked when terminal supports it)
   - syntax-highlighted YAML block
   - `Reason: file does not exist: /definitely/not/here.toml`
   - subsequent passing checks remain compact one-liners.

2. Run the full workspace check chain for the touched packages:
   ```
   cargo build -p claudine -p claudine-cli
   cargo test  -p claudine -p claudine-cli
   cargo clippy -p claudine -p claudine-cli -- -D warnings
   cargo fmt --all
   ```

3. Doc updates:
   - Update
     [`claudine/docs/topics/composition.md`](../../docs/topics/composition.md)
     where it describes pre/post check reporting (search for
     "Pre-validation" / "post_checks" sections) to mention the new
     failure block format.
   - Update
     [`.claude/skills/claudine/validations-and-handlers.md`](../../../.claude/skills/claudine/validations-and-handlers.md)
     (or the `SKILL.md` if validations-and-handlers does not yet describe
     reporter output) with a one-paragraph note: failures now show source
     file (OSC8-linked), YAML snippet, and the underlying diagnostic.
   - No README changes unless a top-level CLI behavior reference exists
     for harness reporting.

4. Mark the spec status. Either flip `status: draft → status: complete` in
   `spec.md`'s frontmatter, or move the whole `2026-05-02-validation-reporter/`
   directory into `claudine/features/_completed/` per repo convention
   (check `claudine/features/_completed/` to confirm the move pattern).

**Validation checkpoint.**
- Manual stderr inspection matches the spec's "Failure (new)" example shape.
- Workspace `build`/`test`/`clippy`/`fmt` all clean for `claudine` +
  `claudine-cli`.
- Docs updated.

---

## Risks and Open Items

- **Line-range recovery is deferred.** `RuleSource.line_range` is always
  `None` after Phase 2. A follow-up that re-parses the source frontmatter
  with a span-preserving YAML parser (e.g. `saphyr`) can populate it without
  any reporter changes — the field is already there.
- **YamlBlock new() can fail.** If the re-serialized snippet ever fails
  validation (it shouldn't, since it just round-tripped through
  `serde_yaml_ng`), the renderer silently drops Section 3 rather than
  panicking. The `Reason:` line still surfaces the diagnostic.
- **`gray-500` styling.** The exact muted style for "Reason:" must match
  whatever the existing reporter convention is — if no precedent exists,
  default to plain (no color) rather than introducing a new palette token
  in this feature.
- **No JSON exporter touch.** Per spec non-goals, downstream consumers of
  `ValidationCheckOutcome` are unchanged: the new `source` field is
  additive and `Option`, and `failure_message` / `markup` / `passed` keep
  their current meaning.

## Parallelism Notes

- Phases 1 → 2 → 3 are strictly sequential (each adds fields the next phase
  depends on).
- Phase 4 (reporter) depends on Phase 3.
- Phase 5 (tests) is mostly written alongside Phases 2, 3, 4 but the
  block-renderer tests can only land after Phase 4. Author Phase 5
  test-skeletons during Phases 2–3 if convenient, but only run them once
  Phase 4 is in.
- Phase 6 is final and not parallelizable (it consumes the whole stack).
