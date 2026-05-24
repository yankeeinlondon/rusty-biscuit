---
name: YAML Code Block and Frontmatter Generation
phases: 2
created: 2026-04-25
start_phase: 1
---

# Execution Plan: YAML Code Block and Frontmatter Generation

Adds two inherent methods to `Yaml` (`as_yaml_code_block`, `as_frontmatter_block`) plus a validation error variant. All implementation lives in `biscuit-file/lib/src/yaml/types.rs`. No new dependencies, no CLI changes.

## Source Documents

- Spec: `biscuit-file/features/_unscheduled/yaml-code-block/spec.md`
- Tech Design: `biscuit-file/features/_unscheduled/yaml-code-block/tech-design.md`

## Working Directory

All `cargo`/`just` commands run from `biscuit-file/` unless noted.

## Phase 1 — Library Implementation

Goal: ship `Yaml::as_yaml_code_block()` and `Yaml::as_frontmatter_block()` with a new `YamlError::FrontmatterNotMapping { found }` variant, fully tested, lints clean.

All steps in this phase touch the same file (`biscuit-file/lib/src/yaml/types.rs`) and must run sequentially.

### Step 1.1 — Add `YamlError::FrontmatterNotMapping` variant

- File: `biscuit-file/lib/src/yaml/types.rs`
- Insert into the `YamlError` enum (after the existing `MaxDepthExceeded` variant or alongside related validation errors):

  ```rust
  /// Frontmatter must be a YAML mapping at the top level.
  #[error("Frontmatter must be a YAML mapping, found {found}")]
  FrontmatterNotMapping { found: &'static str },
  ```

- Do not mark the enum `#[non_exhaustive]` (crate is `0.1.0`; tech design defers that).
- Validation: `cargo check -p biscuit-file --features yaml` from repo root succeeds.

### Step 1.2 — Add private helpers

- File: `biscuit-file/lib/src/yaml/types.rs`
- Add three private free functions in module scope, near the existing `yaml_type_name` helper:

  ```rust
  fn yaml_kind(value: &serde_yaml_ng::Value) -> &'static str { /* per tech-design */ }

  fn longest_backtick_run(input: &str) -> usize { /* per tech-design */ }

  fn ensure_trailing_newline(output: &mut String) {
      if !output.ends_with('\n') {
          output.push('\n');
      }
  }
  ```

- All seven `serde_yaml_ng::Value` variants must be covered in `yaml_kind` (`Null`, `Bool`, `Number`, `String`, `Sequence`, `Mapping`, `Tagged`).
- Helpers stay private — no `pub`.
- Validation: `cargo check -p biscuit-file --features yaml` succeeds; no dead-code warnings (will be consumed in Step 1.3).

### Step 1.3 — Implement the two public methods

- File: `biscuit-file/lib/src/yaml/types.rs`
- Add inside the existing `impl Yaml` block, placed after `as_toml_with` and before `validate`:

  - `pub fn as_yaml_code_block(&self) -> Result<String, YamlError>`
    1. `let body = serde_yaml_ng::to_string(&self.value)?;`
    2. `let fence_len = longest_backtick_run(&body).saturating_add(1).max(3);`
    3. Build a fence `String` of `fence_len` backticks.
    4. Compose: `{fence}yaml\n{body}` then `ensure_trailing_newline`, then `{fence}\n`.

  - `pub fn as_frontmatter_block(&self) -> Result<String, YamlError>`
    1. Reject non-mapping top-level via `matches!(self.value, serde_yaml_ng::Value::Mapping(_))`; otherwise return `YamlError::FrontmatterNotMapping { found: yaml_kind(&self.value) }`.
    2. `let body = serde_yaml_ng::to_string(&self.value)?;`
    3. Compose: `"---\n"` + body + `ensure_trailing_newline` + `"---\n"`.

- Doc comments:
  - Use `## Errors` (H2) section, no H1.
  - `as_yaml_code_block`: "Returns an error if the YAML value cannot be serialized."
  - `as_frontmatter_block`: "Returns an error if the YAML value is not a mapping or cannot be serialized."

- Tracing: add `#[instrument(level = "trace", skip(self), fields(source = ?self.source))]` to both methods to match the style of `as_json` / `as_toml`.

- Do not use `unwrap()` or `expect()`.
- Validation: `cargo build -p biscuit-file --features yaml` from repo root succeeds.

### Step 1.4 — Add unit tests

- File: `biscuit-file/lib/src/yaml/types.rs` (inside the existing `#[cfg(test)] mod tests`)
- Add at the end of the test module, grouped under a `// ===== YAML Code Block / Frontmatter =====` header.

  Code-block tests:
  - `test_as_yaml_code_block_basic` — mapping with nested values; assert output starts with `` "```yaml\n" `` and ends with `` "```\n" ``; assert contains `"key: value"` (or equivalent serialized output).
  - `test_as_yaml_code_block_increases_fence_for_triple_backticks` — mapping whose string value contains three backticks; assert output starts with `` "````yaml\n" `` and ends with `` "````\n" ``.
  - `test_as_yaml_code_block_uses_one_more_than_longest_run` — string containing four contiguous backticks; assert opening and closing fences contain five backticks.
  - `test_as_yaml_code_block_allows_scalar` — `Yaml::from_str("\"just a string\"")`; assert it produces a fenced YAML block (no error).
  - `test_as_yaml_code_block_allows_sequence` — `Yaml::from_str("- one\n- two")`; assert it produces a fenced YAML block (no error).

  Frontmatter tests:
  - `test_as_frontmatter_block_mapping` — top-level mapping; assert output starts with `"---\n"`, contains serialized keys, and ends with `"---\n"`.
  - `test_as_frontmatter_block_allows_empty_mapping` — `Yaml::from_str("{}")`; assert structurally valid (`starts_with("---\n")` and `ends_with("---\n")`).
  - `test_as_frontmatter_block_rejects_scalar` — `"just a string"`; assert `matches!(err, YamlError::FrontmatterNotMapping { found: "string" })`.
  - `test_as_frontmatter_block_rejects_sequence` — `"- one"`; assert `matches!(err, YamlError::FrontmatterNotMapping { found: "sequence" })`.
  - `test_as_frontmatter_block_rejects_tagged_value` — input such as `"!Custom\nfoo: bar"` (or simpler tagged scalar). Skip if `serde_yaml_ng` does not parse it as `Value::Tagged` at the top level; otherwise assert `found == "tagged"`.

- Use `assert!`, `assert_eq!`, and `matches!`. No `unwrap` outside test code (already permitted in tests in this file).
- Validation: `cargo test -p biscuit-file --features yaml yaml::types` from repo root passes.

### Step 1.5 — Lint and full test sweep

- From `biscuit-file/`:
  - `just test` → all tests pass.
  - `just lint` → no warnings or errors.
- If `just lint` flags doc-comment H1s or missing `## Errors`, fix in `types.rs` and re-run.

## Phase 2 — Documentation Sync

Goal: update the four documents called out by the tech design so the new public surface is discoverable.

All four steps in this phase are independent and can run in parallel.

### Step 2.1 — Update `biscuit-file/lib/README.md` *(parallel)*

- Add `as_yaml_code_block()` and `as_frontmatter_block()` to the `Yaml` example block.
- Show one usage example for each (example input → expected fenced/frontmatter output, mirroring spec examples).
- Validation: file renders correctly when previewed; no broken code fences in the README itself (the inner triple-backtick example needs careful escaping — use a four-backtick outer fence in the README to demonstrate collision avoidance).

### Step 2.2 — Update `biscuit-file/README.md` *(parallel)*

- Under the YAML / functional overview section, add a one-line mention that `Yaml` can render itself as a Markdown YAML code block or as Markdown frontmatter.
- No code samples required — keep the addition to a sentence or two.
- Validation: section reads naturally and doesn't duplicate the lib README.

### Step 2.3 — Update `.claude/skills/biscuit-file/references/api.md` *(parallel)*

- File: `.claude/skills/biscuit-file/references/api.md`
- Under the `## Yaml` → `### Conversion` block (after the TOML conversion line, around line 73), append:

  ```rust
  // Markdown wrappers (no new format, just packaging)
  let block: String = yaml.as_yaml_code_block()?;   // ```yaml ... ``` (auto-grows fence)
  let fm: String = yaml.as_frontmatter_block()?;    // --- ... --- (mapping required)
  ```

- Add a short note immediately after the Conversion block:

  > `as_frontmatter_block` returns `YamlError::FrontmatterNotMapping { found }` when the top-level YAML value is not a mapping (scalar, sequence, or tagged).

- Validation: file is valid markdown; example syntax compiles in spirit (matches Step 1.3 signatures).

### Step 2.4 — Update `.claude/skills/biscuit-file/references/format-conversion.md` *(parallel)*

- File: `.claude/skills/biscuit-file/references/format-conversion.md`
- Add a short subsection (e.g. `### Markdown Wrappers (YAML)`) clarifying these helpers are *rendering helpers*, not new data-format conversions: they wrap YAML output for embedding in Markdown.
- Mention the auto-growing fence and the mapping-only frontmatter constraint.
- Validation: the new content references existing terminology used elsewhere in the file.

### Step 2.5 — Final verification checkpoint

After Steps 2.1–2.4 complete, run from `biscuit-file/`:

- `just test` → still green.
- `just lint` → still clean.

Then verify documentation drift:

- `git diff --stat` shows only the files listed in this plan plus `plan.md`.
- No additions to `Cargo.toml`, no new feature flags, no CLI files modified.

## Validation Checkpoints Summary

| Checkpoint | Command | Phase |
|---|---|---|
| Compiles after enum change | `cargo check -p biscuit-file --features yaml` | 1.1 |
| Compiles after helpers + methods | `cargo build -p biscuit-file --features yaml` | 1.3 |
| Targeted tests pass | `cargo test -p biscuit-file --features yaml yaml::types` | 1.4 |
| Full area tests pass | `just test` (in `biscuit-file/`) | 1.5, 2.5 |
| Lints clean | `just lint` (in `biscuit-file/`) | 1.5, 2.5 |
| No unintended files changed | `git diff --stat` | 2.5 |

## Out of Scope (Reaffirmed from Tech Design)

- No CLI flags or `bf` subcommands.
- No new dependencies.
- No new feature flag — these methods live behind the existing `yaml` feature.
- No alternate frontmatter delimiters (e.g. `+++`).
- No frontmatter *extraction* changes.
- No `#[non_exhaustive]` on `YamlError` (deferred until crate stabilizes).
