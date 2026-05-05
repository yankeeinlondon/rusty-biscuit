---
status: complete
owner: claudine
related:
  - claudine/lib/src/harness/validate.rs
  - claudine/lib/src/harness/report.rs
  - claudine/lib/src/harness/model.rs
  - features/2026-04-29-leverage-dm-parser/review-2.md
  - biscuit-terminal
  - biscuit-file
---

# Improve Harness Validation Reporter Quality

## Background

The harness validation pipeline runs author-declared `validate.pre` and
`validate.post` rules against a markdown harness message and emits a per-check
report. Today the reporter is functional but the user-facing output is thin and
sometimes misleading.

The relevant pieces:

- [`evaluate_single`](../../../lib/src/harness/validate.rs) returns
  `Result<(), String>`, where the `String` is a real diagnostic
  (`"file does not exist: /abs/path"`, `"shell command exited with status 1"`,
  etc.).
- [`build_check_markup`](../../../lib/src/harness/validate.rs) produces a single
  short markup line per rule by either rendering the author's `message:` template
  or, when absent, a hard-coded default like `"the file {{file}} exists"`.
- [`ValidationCheckOutcome`](../../../lib/src/harness/model.rs) carries
  `passed`, `markup`, and `failure_message` (the diagnostic from
  `evaluate_single`).
- [`report_check_outcomes`](../../../lib/src/harness/report.rs) walks each
  outcome and emits a `Status` row using `markup` and a pass/fail glyph.
  **It does not surface `failure_message` at all** — the diagnostic is
  collected and then thrown away at render time.

The result is that a failing check renders as something like:

```
✗ the file Cargo.toml exists
```

which reads as a positive assertion with a red glyph rather than a useful
failure description, omits the actual reason for the failure, omits the source
markdown the rule was loaded from, and omits the YAML block the author wrote.

## Goals

A failing pre/post validation check should give the user enough information to
locate and fix the problem without re-reading the harness file:

1. State the failure plainly — not a positive assertion paired with a red glyph.
2. Show the underlying diagnostic from `evaluate_single`.
3. Show the source markdown file the rule came from, ideally as an OSC-8 link
   to the file path (and, where possible, the line range of the rule's YAML
   block).
4. Show the offending YAML rule itself, rendered as a syntax-highlighted YAML
   snippet using the existing terminal rendering primitives in this monorepo.
5. Pass-state output stays compact (one line per check, as today) — the rich
   block is for failures only.

## Non-Goals

- Changing the validation rule schema or adding new validation kinds.
- Changing how author-written `message:` templates are interpolated. That
  surface (frontmatter-backed bare-name lookup) is tracked separately under
  the dm-parser follow-up; this spec assumes the existing template behavior.
- Restructuring `ValidationPhaseReport` consumers outside of the harness
  reporter (e.g. JSON exporters, audit logs). Those should keep working
  off the existing `passed` / `markup` / `failure_message` fields.

## Proposed Output Shape

### Pass (unchanged)

```
✓  the file Cargo.toml exists
```

One-line status row. The `markup` continues to come from
`build_check_markup` — either the author's `message:` template or the
kind-specific default.

### Failure (new)

```
✗  Pre-validation failed
   in features/2026-04-29-leverage-dm-parser/spec.md:42-44

     - file_exists: "Cargo.toml"      ← styled YAML block
       message: "{{source_file}} requires a Cargo.toml"

   Reason: file does not exist: /Users/ken/repo/Cargo.toml
```

Components, top to bottom:

1. **Status header.** `✗` glyph + plain-language failure summary
   ("Pre-validation failed", "Post-validation failed"). Short.
2. **Source location.** `in <relative-path>[:<line-range>]`. The path is an
   OSC-8 hyperlink (biscuit-terminal `Prose`). Line range is best-effort —
   omit if not known.
3. **YAML snippet.** The originating YAML block for the rule, rendered with
   the same syntax-highlighted YAML output used elsewhere in the monorepo
   (biscuit-file / biscuit-terminal). Indented, no surrounding box.
4. **Reason.** `Reason: <failure_message>` — the existing diagnostic
   string from `evaluate_single`, surfaced verbatim.

## Required Changes

### 1. Carry source location on `ValidationRule`

`ValidationRule` (model.rs:133) does not currently know which markdown file
or which line range it came from. Add an optional source-span field, e.g.:

```rust
pub struct ValidationRule {
    // ...existing fields...
    pub source: Option<RuleSource>,
}

pub struct RuleSource {
    pub file: PathBuf,
    pub line_range: Option<RangeInclusive<usize>>,
    pub yaml_snippet: String,
}
```

Population happens in the harness parser (`harness::parse`) during rule
construction. Both `line_range` and `yaml_snippet` are best-effort: if the
parser cannot recover them (e.g. programmatically constructed rules in
tests), `source` is `None` and the reporter falls back to the legacy
single-line output.

### 2. Propagate source to `ValidationCheckOutcome`

`ValidationCheckOutcome` (model.rs:444) gains a clone of the
`Option<RuleSource>` so the reporter does not have to look the rule up by
`rule_id`.

### 3. Failure block renderer in `harness::report`

A new helper (sketch):

```rust
fn render_failure_block(
    outcome: &ValidationCheckOutcome,
    phase: FailurePhase,
    term: &Terminal,
) { /* status header + source line + YAML block + reason */ }
```

`report_check_outcomes` switches on `outcome.passed`:

- `true` → existing `emit_status` path with `markup`.
- `false` → `render_failure_block`.

### 4. YAML rendering helper

Use the existing biscuit-file / biscuit-terminal YAML renderer rather than
hand-rolling SGR sequences. If no such helper currently exists in a form
this reporter can call directly, expose a thin wrapper. Indentation is two
spaces past the column of the status glyph for visual nesting.

### 5. Stop reusing the same `markup` string for both pass and fail

The default-message templates in `default_message` (validate.rs:723) read as
positive assertions ("the file {{file}} exists"). For the failure block the
status header should be the phase-level summary
("Pre-validation failed" / "Post-validation failed"), not the rule's
"what we checked" string. The pass path keeps the assertion phrasing.

## Open Questions

1. **Line-range recovery.** Does the current YAML parser
   (`harness::parse`) preserve enough span information to extract a line
   range per rule? If not, scope this to file-level only for the first
   pass and add line ranges in a follow-up.
2. **Multi-failure batching.** When several pre-checks fail, do we emit
   one rich block per failure, or one combined block listing all
   failures under a single header? First pass: one block per failure;
   revisit if it becomes noisy.
3. **Color/styling parity with the existing reporter.** Should the
   "Reason:" line use the same red Prose styling used elsewhere for
   error chains, or a more muted treatment so the YAML block remains
   the focal point? Recommend muted; the glyph already carries the
   severity signal.
4. **Verbose vs default behavior.** Should the rich failure block be
   gated behind `--verbose`, or always shown for failures? Recommend
   always-on for failures (the user already opted into seeing
   validation results by running the harness), with `--debug` adding
   the `failure_message`'s full backtrace if any.

## Out-of-Scope Follow-Ups

- Author-written `message:` templates resolving frontmatter via bare-name
  lookup (tracked under the dm-parser follow-up).
- Shell expansion syntax in messages (e.g. `$(...)` for command output).
- A machine-readable diagnostics export format (e.g. SARIF / JSON Lines)
  for CI consumers.
