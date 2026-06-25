---
agent: open_code/zai-coding-plan/glm-5.2
phases: 4
created: 2026-06-20
start_phase: 1
yolo: "true"
spec: 2026-06-14-system-prompt-mode/spec.md
packages: [claudine]
source_files_during_phase_1:
  - claudine/lib/src/system_prompt/prepare.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/system_prompt/prepare.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - claudine/docs/topics/system-prompt.md
  - claudine/docs/topics/frontmatter-properties.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
  - claudine/lib/src/system_prompt/prepare.rs
documentation:
  - claudine/docs/topics/system-prompt.md
  - claudine/docs/topics/frontmatter-properties.md
---

# Plan — System Prompt Mode (Frontmatter-Driven Replace for Discovered Files)

## Context

This plan implements the spec at `claudine/features/2026-06-14-system-prompt-mode/spec.md`: allow a
discovered `system-prompt.md` to control its own delivery mode (`append` | `replace`) via a `mode`
frontmatter property, validated by a baseline `SimplifiedSchema` claudine supplies to the Darkmatter
compose operation.

The change is **surgical and concentrated in one file** (`claudine/lib/src/system_prompt/prepare.rs`)
plus two documentation files. No new error variants, no data-model changes, no CLI flag changes, no
`resolve.rs` changes.

### Pre-flight verification (already completed during planning)

Every Darkmatter API the spec depends on was confirmed against source — these are facts, not
assumptions the implementer must re-derive:

- `ComposeOptions::with_baseline_schema(SimplifiedSchema) -> Self` —
  `darkmatter/lib/src/markdown/compose/context/options.rs:899`
- `Markdown::fm_get::<T>(&self, key) -> MarkdownResult<Option<T>>` —
  `darkmatter/lib/src/markdown/mod.rs:134`
- `Markdown::compose_with(options) -> MarkdownResult<(Markdown, ComposeReport)>` returns the composed
  `Markdown` (frontmatter is readable from it) — `darkmatter/lib/src/markdown/compose/mod.rs:190`
- `MarkdownError::SchemaValidationFailed { path, problems, summary, description, source }` is public —
  `darkmatter/lib/src/markdown/types.rs:132`
- `ClaudineError::SystemPromptComposition(#[from] MarkdownError)` — a `MarkdownError` returned from
  `md.compose_with(options)?` auto-converts via `?` (`claudine/lib/src/error.rs:233`). So invalid
  `mode` → compose error → `SystemPromptComposition` with **zero** new error wiring.
- `parse_yaml_schema(&serde_yaml_ng::Value) -> Result<SimplifiedSchema, SchemaError>` accepts a YAML
  mapping `{ mode: "enum(append, replace; default(append))" }`; both it and `SimplifiedSchema` are
  `pub use`'d from `darkmatter::markdown::schemas` — `darkmatter/lib/src/markdown/schemas/simplified/mod.rs:45`
  and `.../schemas/mod.rs:76`
- The grammar `enum(append, replace; default(append))` parses (matches the verified
  `enum(draft, published, archived; default(draft); required)` test shape) —
  `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs:1465`
- Schema validation is **always-on during compose** when `baseline_schema` is set —
  `darkmatter/lib/src/markdown/compose/schema_validation.rs:60`
- `describe_source` already pattern-matches `StandardDiscovered { path, scope }` with **no** `mode`
  field; `describe_effective` already prints `mode:` from `prepared.mode` —
  `claudine/cli/src/commands/wrap/system_prompt.rs:61` and `:105`. **No CLI change required.**

### Key invariant the implementer must respect

The JSON Schema `default(append)` is **annotation-only**. Darkmatter does **not** backfill an absent
`mode` key into composed frontmatter. So `fm_get::<String>("mode")` returns `Ok(None)` for an absent
key — and the read-back must treat `None` as `Append`. This is the backwards-compatible default and is
exactly what the spec's "absent → Append" row requires.

---

## Phase 1 — Library implementation in `prepare.rs`

The core. All tasks are within `claudine/lib/src/system_prompt/prepare.rs` (no other source files
change). Tasks are ordered by compile-time dependency: the schema helper and the
`compose_prompt_markdown` return-type change land first because the mode read-back and the prepare
restructure depend on them.

- [x] **1.1 — Add the baseline schema helper.** Define a small function in `prepare.rs` (e.g.
  `fn discovered_baseline_schema() -> SimplifiedSchema`) that builds the baseline once via
  `darkmatter::markdown::schemas::parse_yaml_schema` over a YAML mapping with the single entry
  `mode: "enum(append, replace; default(append))"`. Construct the `serde_yaml_ng::Mapping`,
  wrap it in `serde_yaml_ng::Value::Mapping`, call `parse_yaml_schema`, and `.expect(...)` on the
  `Err` with a message naming the constant string (it is a compile-controlled literal, so a parse
  failure is a programmer error, not a runtime condition). Add the imports
  (`darkmatter::markdown::schemas::{SimplifiedSchema, parse_yaml_schema}` and
  `serde_yaml_ng`). Keep it in `prepare.rs` rather than a new `schema.rs` sibling — one private
  function does not justify a new module file (Rule 2: Simplicity First).

- [x] **1.2 — Change `compose_prompt_markdown` to return the composed `Markdown` and attach the
  baseline for discovered sources only.** Today the signature is
  `fn compose_prompt_markdown(...) -> Result<String, ClaudineError>` and it ends with
  `Ok(composed.content().to_string())` (`prepare.rs:25-73`). Change the return type to
  `Result<darkmatter::markdown::Markdown, ClaudineError>` and return `Ok(composed)` (the
  `(Markdown, ComposeReport)` first element). In the `options` builder, after the existing
  `bind_agent_workspace` call, attach the baseline **only** when the source is `StandardDiscovered`:
  `.with_baseline_schema(discovered_baseline_schema())` guarded by
  `matches!(source, SystemPromptSource::StandardDiscovered { .. })`. Do **not** attach it for
  `ExplicitFile`, `NonInteractiveFile`, or `BuiltInNonInteractive`. The `md.compose_with(options)?`
  line stays as-is — the `MarkdownError` still auto-converts to
  `ClaudineError::SystemPromptComposition` via `#[from]`.

- [x] **1.3 — Add `mode_from_composed` read-back helper.** Add
  `fn mode_from_composed(composed: &darkmatter::markdown::Markdown) -> SystemPromptMode` that calls
  `composed.fm_get::<String>("mode")` and maps defensively:
  `Ok(Some("replace"))` → `Replace`; `Ok(Some("append"))` / `Ok(None)` / `Ok(Some(<other string>))`
  → `Append`; `Err(_)` (a non-string value that escaped validation via a document `$schema`
  conflict) → `Append` with an optional `tracing::warn!`. This keeps the common path strictly
  enum-validated while remaining robust to the obscure override case (spec → "Where mode is
  resolved"). Match the "absent" arm, never panic.

- [x] **1.4 — Trim `mode_for_source` to drop `StandardDiscovered`.** Edit `mode_for_source`
  (`prepare.rs:16-23`) so it no longer has a `StandardDiscovered` arm. It stays responsible only for
  `ExplicitFile { mode, .. }` (returns `*mode`), `NonInteractiveFile`, and
  `BuiltInNonInteractive` (both `Append`). The discovered-file mode is now computed post-compose by
  `mode_from_composed`. **This will cause a compile error at the `Disabled` arm in
  `resolve_and_prepare_for_session` (`prepare.rs:282`) — that is expected and fixed in 1.6.**

- [x] **1.5 — Restructure `prepare_system_prompt` and `prepare_system_prompt_with_ctx` to compute
  discovered mode after compose.** In both functions (`prepare.rs:137-155` and `:184-206`): obtain
  `let composed_md = compose_prompt_markdown(...)?;` (now a `Markdown`), compute the string via
  `composed_md.content()` for the empty-body check, and resolve the mode with: `ExplicitFile` /
  `NonInteractiveFile` / `BuiltInNonInteractive` → `mode_for_source(&source)` (unchanged);
  `StandardDiscovered` → `mode_from_composed(&composed_md)`. Structure it as a match on `&source`
  so the discovered arm reads the composed frontmatter and the explicit arm keeps using the
  source-carried mode. The `PreparedSystemPrompt { mode, .. }` construction otherwise stays
  identical.

- [x] **1.6 — Fix the `Disabled` arm in `resolve_and_prepare_for_session`.** The arm at
  `prepare.rs:282` currently calls `mode_for_source(&source)`, which after 1.4 no longer covers
  `StandardDiscovered`. Replace it with a match: `ExplicitFile { mode, .. }` → `*mode`;
  `StandardDiscovered { .. }` → `Append` (an empty discovered body makes the declared mode moot —
  the effective prompt becomes the appendix, which is Append-style content; this matches the spec's
  empty-body edge case where `mode` is irrelevant). `NonInteractiveFile` / `BuiltInNonInteractive`
  cannot reach this arm via `resolve_system_prompt_source`, but include them as `Append` for
  exhaustiveness.

- [x] **1.7 — Update `prepare_non_interactive_appendix_from` for the new return type.** This caller
  (`prepare.rs:159-177`) also uses `compose_prompt_markdown`. Since the function now returns
  `Markdown`, update the call site to `let composed_md = compose_prompt_markdown(...)?;` and then
  `let composed_markdown = composed_md.content().trim().to_string();` (the appendix only needs the
  body string; the baseline schema is **not** attached for `NonInteractiveFile` /
  `BuiltInNonInteractive`, so no `mode` read-back is needed here).

- [x] **1.8 — Validation checkpoint (Phase 1).** Run `just check` (from `claudine/`) —
  `cargo check -p claudine -p claudine-contract -p claudine-cli` must compile cleanly. The existing
  tests `standard_file_always_append` (`prepare.rs:505`) and `explicit_replace_preserves_mode`
  (`prepare.rs:525`) must still pass: a discovered file with no `mode` resolves to `Append`
  (`fm_get` → `Ok(None)` → `Append`), and an explicit replace file keeps `Replace` via
  `mode_for_source`. Gate: green compile + the two pre-existing tests pass under
  `just test-library system_prompt`.

---

## Phase 2 — Test plan in `prepare.rs`

Depends on Phase 1 landing. All tests are unit tests added to the existing `#[cfg(test)] mod tests`
in `claudine/lib/src/system_prompt/prepare.rs`. Use the established helpers (`write_temp_file`,
`prepare_system_prompt`, `resolve_and_prepare_for_session`, `LaunchContext`) and the `#[serial]`
attribute where a test mutates `HOME`.

These map 1:1 to the spec's 12-item test plan. To assert the error channel for the reject cases,
match on `Err(crate::error::ClaudineError::SystemPromptComposition(md_err))` and then on
`darkmatter::markdown::MarkdownError::SchemaValidationFailed { .. }` (both are public and already
imported elsewhere in claudine — `darkmatter::markdown::MarkdownError`).

- [x] **2.1 — Absent `mode` → `Append` (default).** Discovered `system-prompt.md` with no `mode`
  frontmatter resolves to `SystemPromptMode::Append`. Also confirms the absent key is treated as
  `Append` (schema default is not backfilled into composed frontmatter).

- [x] **2.2 — Explicit `mode: append` → `Append`.** Discovered file with `---\nmode: append\n---`
  resolves to `Append`.

- [x] **2.3 — Explicit `mode: replace` → `Replace`.** Discovered file with
  `---\nmode: replace\n---` resolves to `Replace`. This is the headline new capability.

- [x] **2.4 — Invalid string value rejected at compose.** Discovered file with
  `mode: overwrite` fails compose; assert the error is `ClaudineError::SystemPromptComposition`
  wrapping `MarkdownError::SchemaValidationFailed { .. }` — **not** a bespoke variant. Optionally
  assert the summary names `mode`.

- [x] **2.5 — Non-string value rejected at compose.** Discovered file with `mode: 42` fails schema
  validation during compose and surfaces via the same `SystemPromptComposition` path.

- [x] **2.6 — Full pipeline.** `resolve_and_prepare_for_session` with a discovered `mode: replace`
  file produces a `PreparedSystemPrompt` with `mode: Replace` that flows through to provider
  delivery (assert on `ResolvedSystemPrompt::Ready(prepared)` with `prepared.mode == Replace` and
  the body present).

- [x] **2.7 — Explicit flag ignores frontmatter.** `--replace-system-prompt` (`SystemPromptArgs {
  replace_file: Some(..), .. }`) pointing at a file that contains `mode: append` in frontmatter
  still uses `Replace`. The flag wins because the explicit path composes without the baseline schema
  and `resolve_system_prompt_source` returns early before discovery.

- [x] **2.8 — Non-interactive + replace mode.** Discovered file with `mode: replace` in a
  non-interactive session (`resolve_and_prepare_for_session(.., true)`) preserves `Replace` after
  the safety appendix is appended. Use `#[serial]` + a temp `HOME` like the existing
  `non_interactive_session_preserves_replace_mode` test (`prepare.rs:643`).

- [x] **2.9 — Empty body + `mode: replace` → `Disabled`.** A discovered file that is frontmatter-only
  (`---\nmode: replace\n---\n`, no body) composes to an empty body and resolves to
  `ResolvedSystemPrompt::Disabled` regardless of the declared mode — not an error, not an empty
  replacement. Drives `prepare_system_prompt` directly.

- [x] **2.10 — Document `$schema` conflict falls back defensively.** A discovered file whose own
  `$schema` redefines `mode` (e.g. as a free `string`) to allow another value (e.g.
  `mode: overwrite`) composes successfully (document-side-wins merge relaxes the baseline enum), but
  claudine resolves the effective delivery mode to `Append` rather than panicking or inventing a
  mode. If a warning is emitted, assert via tracing only where the harness has a stable capture
  path; otherwise just assert the `Append` outcome.

- [x] **2.11 — Prompt report reflects effective mode.** A discovered `mode: replace` file produces a
  `PreparedSystemPrompt` whose `mode` field is `Replace` (the value `describe_effective` and prompt
  reports read), while `verbosity` frontmatter continues to control only report verbosity. Assert
  on `prepared.mode` directly (this is the source of truth for `describe_effective` at
  `system_prompt.rs:105`).

- [x] **2.12 — Validation checkpoint (Phase 2).** Run
  `just test-library system_prompt::prepare` — all 12 new tests plus the pre-existing prepare tests
  pass. Gate: green.

---

## Phase 3 — Documentation (parallelizable with Phase 2)

Documentation is text-only and independent of the Phase 2 test suite, so it may be authored
concurrently with Phase 2. Test #12 in the spec is a docs-review check that lands here.

- [x] **3.1 — Update `claudine/docs/topics/system-prompt.md`.** (a) Replace/qualify the statement at
  line 74 (`The standard discovered file is always treated as append-mode.`) so it documents the new
  `mode` frontmatter property: absent/`null`/`append` → append (default); `replace` → replace. (b)
  Note that an invalid value is rejected during compose by the baseline `SimplifiedSchema`, surfacing
  as a composition error. (c) Note that explicit `--append-system-prompt` / `--replace-system-prompt`
  files ignore the `mode` key entirely (the flag is authoritative). (d) Note that a document-owned
  `$schema` that redefines `mode` opts out of baseline validation but does not create additional
  delivery modes (falls back to append). Keep the edit surgical — do not reflow unrelated prose
  (Rule 3).

- [x] **3.2 — Add `mode` to `claudine/docs/topics/frontmatter-properties.md`.** Add a new row to the
  authoritative frontmatter catalog. The cleanest home is a short "System Prompt" subsection (the
  existing `## Prompt Reporting` section already lists `verbosity` for system-prompt docs). The row
  must state: `mode` controls delivery mode for **automatically discovered** `system-prompt.md`
  files only; accepts `append` or `replace`; absent/`null` defaults to append; explicit
  `--append-system-prompt` / `--replace-system-prompt` files ignore this key. Link to
  `system-prompt.md`. Reference the source symbols: `SystemPromptMode`, `mode_from_composed`
  (`prepare.rs`), and the baseline schema.

- [x] **3.3 — Validation checkpoint (Phase 3).** Reviewer check (test #12 in the spec): the catalog
  includes the new `mode` row and does **not** claim explicit system-prompt files consult the
  property. Confirm the `system-prompt.md` edits no longer assert "always append" for discovered
  files.

---

## Phase 4 — Validation gate

Depends on Phases 1–3. The full claudine PR-gate minus the L2/L3 PTY tiers (this feature has no
terminal-rendering surface, so L2/L3 are not load-bearing here — but run `test-l2` opportunistically
if the gate is cheap).

- [x] **4.1 — Sanity.** `just sanity` (fast confidence subset, ≤15s, lib + bin).

- [x] **4.2 — Lint.** `just lint` across claudine / claudine-contract / claudine-cli. Address any
  clippy findings introduced by the new code. Do **not** run `cargo fmt` (repo convention: `main` is
  the formatting authority; match surrounding style by hand).

- [x] **4.3 — Doctests.** `just doctest` — confirms any doc-comment additions on the new helpers
  compile. Only add rustdoc to the new helpers if they are `pub`; private helpers need no doc
  (per AGENTS.md comment-quality rules, prefer no comment over a tautological one).

- [x] **4.4 — Full test.** `just test` (claudine + claudine-contract + claudine-cli). All
  Phase 2 tests plus the entire existing suite must pass.

- [x] **4.5 — Cross-platform reasoning check.** This change has no OS-specific surface (pure
  library logic over Darkmatter compose + frontmatter read-back). Confirm there are no new
  filesystem, env, or path assumptions that would break on Windows/Linux. (No new I/O; the
  discovered-file read stays in `resolve.rs`, unchanged.)

- [x] **4.6 — Spec conformance sweep.** Re-read the spec's "In scope", "Out of scope", "Error
  handling", "Interaction with explicit flags", "Non-interactive sessions", "Empty-body edge case",
  and "Affected files" table. Confirm: no new error variant was added; `types.rs` / `resolve.rs` /
  `error.rs` / `cli/.../system_prompt.rs` were **not** modified (or only trivially); the baseline
  schema is attached to `StandardDiscovered` sources only; explicit paths compose without the
  baseline.

---

## Out of scope (explicit non-tasks)

- Do **not** add a `mode` field to `SystemPromptSource::StandardDiscovered` (the original plan to
  carry mode on the source is dropped — mode is read from composed frontmatter).
- Do **not** add a `ClaudineError::InvalidSystemPromptMode` variant (validation is delegated to
  schema validation during compose).
- Do **not** modify `resolve.rs` (`discover_standard_file` keeps its plain `read_to_string`).
- Do **not** modify `claudine/cli/src/commands/wrap/system_prompt.rs` (`describe_source` /
  `describe_effective` already surface the effective mode via `prepared.mode`).
- Do **not** change `--append-system-prompt` / `--replace-system-prompt` behavior.
- Do **not** touch `non-interactive.md` handling or the discovery hierarchy.
- Do **not** forward `mode` (or any frontmatter) into the composed prompt body.

## Risk notes

- **Subtle cascade from trimming `mode_for_source` (task 1.4 → 1.6).** Dropping the
  `StandardDiscovered` arm is intentional and surfaces as a compile error at the `Disabled` arm.
  This is a feature, not a bug: it forces the implementer to decide the empty-body + discovered
  case explicitly rather than inheriting stale behavior. Task 1.6 resolves it to `Append`.
- **Document `$schema` override (test 2.10).** The merge rule is document-side-wins, so a user
  `$schema` can relax the baseline enum. The read-back must stay defensive (task 1.3). This is the
  one case where an "invalid" value is not rejected — by design, because the user explicitly
  overrode the schema.
- **Default is annotation-only.** If a future Darkmatter change ever backfills `default(append)`
  into composed frontmatter, the read-back would see `Ok(Some("append"))` → `Append`, which is
  identical behavior. So the read-back is forward-compatible with either backfill behavior.
