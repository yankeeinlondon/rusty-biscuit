---
kind: feature
created: 2026-11-18
status: draft-spec
area: claudine
depends-on:
    - 2026-11-22-better-errors-and-dry-run
related:
    - 2026-11-22-better-errors-and-dry-run
    - 2026-07-02-inline-sections
---

# Ergonomic Sequences

## Context & Problem Statement

Today, sequence steps rely on the generic `prompt:` directive to execute both `compose` and `inline-compose` operations, while nested child sequences already exist via `sequence:` but lack ergonomic value-passing and fallback semantics. In practice, sequence authors hit four recurring friction points:

1. **Ambiguous task intent and fragmented classification.** `prompt:` hides whether the author intends a standard `compose`, an `inline-compose`, or a child `sequence` run, and three separate code paths (`mismatch.rs`, `preflight/mod.rs::load_prompt`, and `commands/wrap/sequence/mod.rs`) currently classify target documents inconsistently.
2. **No distinction between overriding values and fallback defaults.** A single `compose` or `inline-compose` document must resolve an `agent` (and optionally a `model`), whereas a YAML sequence needs neither on its own root. Today, authors cannot say "use this value unless the task or child document sets its own," and `--<provider>` flags unconditionally override every task through a separate `explicit_provider` channel.
3. **Hardcoded frontmatter isolation.** `darkmatter` transclusion hardcodes a 3-key exclusion list (`prologue`, `epilogue`, `ctx`), while Claudine sequences need a schema-driven way to prevent document-processing and sequence control keys from leaking into child documents without blocking domain keys or `agent`/`model`.
4. **Late provider failures and opaque default models.** Sequences without a root-level agent fallback can fail mid-run on a later step or inside a nested YAML sequence, and dry-runs/logs often cannot tell the user which provider default model will actually execute.

This specification adds explicit `compose:` and `inline-compose:` task executables, formalizes `prompt:` and `sequence:` dispatch and per-executable option rules, unifies document classification, defines a schema-driven `isolated` constraint and Caller → Callee `fm` layering model, unifies `--<provider>` / `--model` handling with a passive cross-provider `get_default_model` lookup, introduces recursive `agent` pre-flight validation, and establishes the diagnostic taxonomy and three-way `--dry-run` exit-code contract.

---

## Requirements

### Functional Requirements

1. **Prerequisite Foundation.** This feature builds directly on `2026-11-22-better-errors-and-dry-run` (implemented first), reusing its shared task schema in `claudine/registration/sequence.yaml`, its `ExecutableField::noun` table, its `dm.reference.missing_document` diagnostic with nearby-file suggestions, its `dm.check.not_checked` reporting, its `removed(...)` schema annotations, and its What/Where/Why/Hint error standard.
2. **Explicit & Generic Task Executables.**
   - Preserve all six existing task executables (`prompt`, `shell`, `side_effect`, `group`, `task`, `sequence`) plus zero-executable Markdown body-default steps, and preserve `timeout` as a duration string.
   - Add two explicit task executables: `compose` and `inline-compose`, accepting **Markdown document file references only**.
   - Keep `prompt` as generic dispatch accepting a **plain `<file-ref>` only** (Markdown, YAML, or supported data sequence formats; no `-> offset` or `::operator` suffixes) capable of executing `compose`, `inline-compose`, or a child `sequence`.
   - Preserve `sequence:`'s full source grammar (`<file-ref> [-> <offset.path>] [::<operator>(<args>)]`) across YAML, JSON, JSON5, JSONL, NDJSON, and Markdown frontmatter, and keep `task: <file-ref>` immutable at the call site (`name` and `when` only).
   - Keep `with:` and `default-values:` strictly as sibling task options beside the executable; reject both on zero-executable Markdown body steps.
3. **Caller → Callee Layering in `fm` & Child-Sequence Isolation.**
   - Support two distinct value channels — **defined values** (override child-authored values) and **default values** (fill only what nothing else sets, including child-authored values) — within the `fm` workspace without colliding with read-only `seq`, `group`, and `loop` workspaces.
   - Apply uniform `with:` merge semantics with zero special cases: `with:` merges onto the effective `fm` workspace first (so `with: { agent: null }` clears any pinned value), and normal resolution then consults `default-values:` or fails/prompts if required and unresolved.
   - Preserve modular isolation across **referenced formal child sequence documents** (`is_formal_sequence == true`, via `sequence: <file-ref>`, root `sequence: <file-ref>`, or `prompt:` resolving to a formal child sequence) via a split boundary: the parent sequence's authored root `fm` does not leak across the sequence boundary, while `with:`, CLI `key=value`, runtime `set`, and inherited `default-values:` (nearer/child wins) cross cleanly. Informal data-file, offset, or operator step lists feeding a Markdown document's own body steps keep that Markdown document's own root `fm`.
   - Forbid runtime `set:` writes to `default-values` (adding `"default-values"` to `SEQUENCE_DECLARATION_KEYS`).
   - Support `with:` and `default-values:` on group legs identically to tasks; support `default-values:` (and forbid `with:`) on inline nested `sequence: [ ... ]` blocks.
4. **Schema-Driven Frontmatter Isolation (`isolated`).**
   - Introduce a property-level `isolated` metadata constraint in `simplified-schema/lib` lowering to `"x-darkmatter-isolated": true` (and do **not** add a new `null-is-undefined` constraint).
   - Roll out `isolated` across `simplified-schema`, `darkmatter` (`::file` / `as_markdown()` transclusion and preflight), and `claudine` sequences using sticky union semantics across baseline, matched triggers, and document `$schema`.
   - Ensure `isolated` blocks *implicit inheritance only* (owning-sequence root `fm` propagation and Darkmatter `::file` transclusion), never explicit `with:`.
   - Keep `agent` and `model` non-isolated across Claudine schemas, enforced by an L1 schema-parity test in `claudine-lib`.
5. **Unified Document Classification & Dynamic Target Handling.**
   - Consolidate document classification into one pure classifier in `claudine-lib` inspecting authored frontmatter only (with absent-equals-null semantics), deferring `sections:` recognition until `2026-07-02-inline-sections` lands.
   - Classify statically resolvable target paths upfront; handle dynamic `{{ expr }}` targets by emitting `dm.check.not_checked` (`info`) in `--dry-run`, deferring classification strictly to runtime resolution, and validating `agent` availability via ancestor/task/ordered `set:` or emitting `composition.agent_unverifiable_target` (`warning`).
6. **Unified CLI Provider/Model Flags, Recursive `agent` Pre-Flight & `get_default_model`.**
   - Unify CLI `--<provider>` and `--model` into Caller → Callee `fm` state (eliminating the parallel `explicit_provider` override channel), disambiguating `claudine sequence` strictly by file format (Markdown vs. YAML), and failing with `usage.invalid_argument` (exit `2`) when `--<provider>` + `agent=<val>` or `--model <val>` + `model=<val>` conflict on the same command.
   - Validate `agent` availability recursively at pre-flight across nested sequences, groups, `task:` files, and composition tasks before execution begins, with ordered `set: { agent: ... }` tracking, cycle detection, and TTY review vs. non-TTY/`--silent` fail-fast behavior.
   - Never require authors to specify `model`; apply a deterministic six-tier model precedence chain (without adding `CLAUDINE_MODEL`) backed by a unified, fast, passive `get_default_model(provider: Provider)` lookup across all 10 registered providers.
7. **Diagnostic Taxonomy & Three-Way `--dry-run` Exit Codes.**
   - Register additive `composition.*` and `config.*` diagnostic codes in `claudine/lib/src/diagnostics/registry.rs` following `2026-11-22-better-errors-and-dry-run`'s message standard.
   - Exit `0` for clean/info-only `--dry-run`, `1` for warnings-only `--dry-run`, and `2` for errors or CLI usage errors.

### Non-Functional & Cross-Stack Requirements

- **Package ownership boundaries:**
  - `simplified-schema/lib`: owns `isolated` constraint parsing and `"x-darkmatter-isolated": true` lowering.
  - `darkmatter/lib`: owns `EffectiveSchema::isolated_keys()`, data-origin defaults layering, and safe conditional file-path provenance (so relative file paths authored in a default layer resolve relative to their authoring file without overwriting child-authored values).
  - `claudine/lib`: owns the unified pure document classifier, Caller → Callee layering, recursive `agent` pre-flight, six-tier model resolution, passive `get_default_model(provider: Provider)` across all 10 registered providers, diagnostic registry additions, and the L1 schema-parity test.
  - `claudine/cli`: owns CLI `--<provider>` / `--model` flag wiring, conflict detection (`--<provider>` + `agent=<val>` and `--model <val>` + `model=<val>`), `--dry-run` exit-code mapping (`0`/`1`/`2`), and launch resolution.
- **Performance & passivity:** `get_default_model(provider: Provider)` performs strictly passive local file reads only (zero subprocesses, zero network), targets `< 10ms` per provider, and is memoized once per invocation keyed by `(Provider, config_root)`.
- **Cross-OS compatibility:** All behavior, including passive default-model lookup and config-root discovery (`DocumentEnvironment` / `HomeBaseline` / XDG / `%APPDATA%`), must compile and work across macOS, Linux, native Windows, and WSL2.

---

## Design Decisions (Confirmed — Batches 1–3)

### Prerequisite Integration (`2026-11-22-better-errors-and-dry-run`)

`2026-11-22-better-errors-and-dry-run` lands before this specification. This specification builds directly on its infrastructure:

- **Shared task schema:** Extends `claudine/registration/sequence.yaml` — preserving the open `task` shape with `$advise-unknown-keys` for sequence steps and the closed `task-file` / group-leg shape for group legs and standalone `kind: task` files. Executable mutual exclusivity and per-executable option rules remain enforced in the Rust parser.
- **Shared diagnostics machinery:** Extends `ExecutableField::noun`, reuses `dm.reference.missing_document` (with nearby-file suggestions) for missing file targets, reuses `dm.check.not_checked` for unchecked dynamic targets, honors the What/Where/Why/Hint message standard and locked `err.*` lifecycle codes, and keeps `removed(...)` schema declarations for retired keys.
- **Explicit supersessions of `2026-11-22-better-errors-and-dry-run`:** `2026-11-22-better-errors-and-dry-run` is left untouched as its reviewed snapshot; these two supersessions are recorded exclusively here:
  1. **`prompt:` retained alongside `compose:` and `inline-compose:`.** `2026-11-22-better-errors-and-dry-run` assumed in its reconciliation note that this spec would *replace* `prompt:` with `compose:`, `inline-compose:`, and `sequence:`. **This spec supersedes that assumption:** `prompt:` is **retained** as generic dispatch, and `compose:` and `inline-compose:` are added alongside it in `ExecutableField::noun`.
  2. **Three-way `--dry-run` exit codes (`0` / `1` / `2`).** `2026-11-22-better-errors-and-dry-run` combined errors and warnings under exit code `2` and stated that a dry run never exits `1`. **This spec refines that exit-status table:** `--dry-run` exits `0` (no errors or warnings; `info` findings only or none), `1` (no errors, one or more warnings), or `2` (one or more errors, or a CLI usage error).

---

### Decision 1 — Task Executable Grammar, `prompt:` Dispatch & Allowed Options

#### 1. Executables & Target Format Rules

1. **Preserved executables and step shapes:**
   - All six existing task executables remain valid: `prompt`, `shell`, `side_effect`, `group`, `task`, and `sequence`.
   - Zero-executable body-default steps (in Markdown sequences) remain valid.
   - `timeout` remains a duration string (e.g., `"5m"`, `"30s"`). The original draft's `shell-command` and `timeout: number` sketch is dropped.
2. **New explicit executables (Markdown only):**
   - `compose`: explicitly composes the referenced **Markdown** document and sends the composed result as the prompt to an agent; rejects inline-compose documents, sequence documents, and non-Markdown files.
   - `inline-compose`: explicitly executes an `inline-compose` operation on the referenced **Markdown** document; rejects standard compose documents, sequence documents, and non-Markdown files.
   - Both accept file references only in this specification (no bare-string inline prompt text; an explicit inline-text form such as `compose: { text: "..." }` remains a possible future additive extension out of scope here).
3. **Retained generic executable (`prompt:`):**
   - `prompt:` accepts a **plain `<file-ref>` only** (no `-> offset` or `::operator` suffixes — data-slicing stays exclusively on `sequence:`).
   - Accepts Markdown documents and YAML (plus supported data sequence formats: JSON, JSON5, JSONL, NDJSON), dispatching to `compose`, `inline-compose`, or a child `sequence` based on the unified document classifier (Decision 4).
   - When `prompt:` resolves to a formal child sequence file, it follows the **referenced child-sequence split boundary** (Decision 2), **rejects** explicit `operation:` / `flow:` on that task (`composition.task_option_invalid`), and **silently ignores** inherited group `operation` / `flow` defaults.
4. **Immutable `task: <file-ref>` call sites:**
   - `task: <file-ref>` accepts `name` and `when` only at the call site. All inner executable options — including `with:` and `default-values:` — live inside the referenced `kind: task` file.
5. **Zero-executable Markdown body steps:**
   - Explicitly reject `with:` and `default-values:` (in addition to `when:`, `loop:`, container events, `timeout:`, `operation:`, `flow:`, and sequence declarations).
6. **Sibling options only:**
   - `with:` and `default-values:` live strictly as sibling options on the task (or group leg) beside the executable key. The original draft's nested `compose: { file, with, default-values }` long-form sketch is dropped.

#### 2. Conceptual Schema Shape (`claudine/registration/sequence.yaml`)

```yaml
# Conceptual shape on the shared task definition (open for sequence steps with
# $advise-unknown-keys; closed for group legs and standalone kind: task files)
name: "string -> task identifier, exposed as seq.name during execution"
timeout: "duration -> wall-clock limit before the task times out (e.g. 5m, 30s)"

# Exactly one executable (or zero executables on a Markdown body-default step):
compose: "file -> Markdown document executed via compose"
inline-compose: "file -> Markdown document executed via inline-compose"
sequence: "file | sequence-step[] -> referenced child sequence (with optional -> offset / ::operator) or inline nested sequence"
prompt: "file -> generic dispatch via plain file reference (compose, inline-compose, or child sequence)"
shell: "string | shell-spec -> shell command execution"
side_effect: "side-effect-spec -> state-mutating side effect"
group: "group-spec -> parallel or structured group of legs"
task: "file -> standalone kind: task file reference"

# Sibling value-passing options (per-executable legality enforced in Rust parser):
with: "object -> defined overrides applied on top of caller fm state"
default-values: "object -> fallback values applied only where nothing higher sets the key"
```

#### 3. Per-Executable Allowed Task Options Table

Every task option outside the allowed set for its executable produces `composition.task_option_invalid` (`error`) rather than being silently ignored (except inherited group `operation`/`flow` defaults when `prompt:` dispatches to a child sequence, which are silently ignored).

| Executable / Step Form | Target Formats | `name` | `when` | `with` | `default-values` | `operation` / `flow` | `timeout` | `loop`, Container Events (`initialize`..`finalize`), `$schema`, `$state`, `template` | Notes |
| --- | --- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | --- |
| `compose: <file-ref>` | Markdown only | Yes | Yes | Yes | Yes | Yes | No | No | Plain file reference; rejects inline-compose & sequence docs |
| `inline-compose: <file-ref>` | Markdown only | Yes | Yes | Yes | Yes | Yes | No | No | Plain file reference; rejects compose & sequence docs |
| `prompt: <file-ref>` (→ compose / inline-compose) | Markdown | Yes | Yes | Yes | Yes | Yes | No | No | Plain `<file-ref>` only (no `->` or `::`) |
| `prompt: <file-ref>` (→ child sequence) | Markdown, YAML, JSON/JSON5/JSONL/NDJSON | Yes | Yes | Yes | Yes | **No** *(explicit rejected; group default ignored)* | No | No | Plain `<file-ref>` only; uses referenced child-sequence split boundary |
| `sequence: <file-ref>` (referenced) | Markdown, YAML, JSON/JSON5/JSONL/NDJSON | Yes | Yes | Yes | Yes | No | No | No | Full `<file-ref> [-> offset] [::op(args)]` grammar; uses split boundary for formal child sequence documents |
| `sequence: [ ... ]` (inline nested) | Inline task list | Yes | Yes | **No** | Yes | No | No | Yes | Executes inside parent's `fm` scope; `with:` forbidden |
| `shell: <cmd \| spec>` | Command string / spec | Yes | Yes | No | No | No | Yes | No | `timeout` is a duration string |
| `side_effect: <spec>` | Side-effect spec | Yes | Yes | No | No | No | No | No | Mutates live `fm` (forbids `SEQUENCE_DECLARATION_KEYS`) |
| `group: <spec \| ref>` | Inline group object, file, or catalog ref | Yes | Yes | No | No | No | No | No *(live on the `group:` object itself)* | Group itself has no `default-values:`; see Group Leg row below |
| Group leg (entry in `group.tasks`) | Same as leg's own executable | Per leg's executable | Per leg's executable | Per leg's executable | Per leg's executable | Per leg's executable | Per leg's executable | Per leg's executable | Closed schema (no open step data); supports `with:` / `default-values:` identically to top-level tasks |
| `task: <file-ref>` | Standalone `kind: task` file | Yes | Yes | **No** | **No** | No | No | No | Immutable at call site; inner executable options (`with:`, `default-values:`, etc.) live inside the `kind: task` file |
| Zero-executable Markdown body step | Owning Markdown doc body | Yes *(+ open step data)* | **No** | **No** | **No** | No | No | No | Invalid in YAML/data sequences (`composition.no_executable`) |

---

### Decision 2 — Caller → Callee Model in `fm` & Child-Sequence Boundary

The read-only workspaces `seq`, `group`, and `loop` are separate from `fm` (frontmatter) and never compete with it. All value passing described here operates within `fm` under a consistent **Caller → Callee** model.

#### 1. CLI → Root Sequence

- CLI `key=value` / `--set` arguments initialize the root sequence's `fm` with highest priority, overriding authored root frontmatter.
- Runtime `set` writes (from `initialize`, `side_effect`, and lifecycle hooks) mutate that live `fm` state as the sequence executes.
- **Runtime `set:` on `default-values` is forbidden:** `"default-values"` is added to `SEQUENCE_DECLARATION_KEYS` alongside `"$state"` and `"template"`. Consistent with `EffectEngine::set` rejecting dotted keys and `RuntimeState.mutations` sharing upward, no value write — step state, template default, CLI setter, `with:`, or runtime `set:` — may target `default-values` or `default-values.*`.

#### 2. Sequence → Direct Prompt Task (`compose:`, `inline-compose:`, `prompt:` → doc, and Group Legs)

The owning sequence acts as the caller. Precedence for any non-isolated `fm` key on the child document, **highest to lowest**:

| Rank | Source | Channel |
| ---- | ------ | ------- |
| 1 | Task's `with:` | Defined override (call-site) |
| 2 | Owning sequence's live non-isolated `fm` (`authored root fm < CLI key=value < runtime set`) | Defined override (caller state) |
| 3 | Child document's own authored `fm` | Callee authored value |
| 4 | Task's `default-values:` | Default fallback (call-site) |
| 5 | Owning sequence's `default-values:` (inheriting from parent `default-values:`, nearer wins) | Default fallback (inherited) |
| 6 | Child document's `$schema` defaults | Schema fallback |

#### 3. Uniform `with:` Merge Semantics & `isolated` Boundary

- **Uniform `with:` merge semantics (zero special cases):** `with:` merges with the effective `fm` workspace first, and the resulting state proceeds through normal resolution checks (under absent-equals-null semantics from `2026-11-14-absent-equals-null`). Setting `with: { agent: null }` (or `with: { model: null }`) clears the child prompt's (or owning sequence's) pinned value; normal resolution then consults `default-values.agent` (or `default-values.model`) and fails or prompts if the key is required and remains unresolved.
- **`isolated` vs. explicit `with:`:** `isolated` blocks **implicit inheritance only** (owning-sequence root `fm` propagation and Darkmatter `::file` transclusion), **never** explicit `with:`. Explicit `with:` forbids only reserved sequence keys:
  - Read-only lexical roots (`ROOT_OVERLAY_KEYS`): `seq`, `group`, `loop`
  - Generated step fields (`GENERATED_STATE_KEYS`): `id`, `sequence_id`, `is_first`, `is_last`, `index`, `count`, `run_id`, `previous`, `next`, `outputs`
  - Sequence declarations (`SEQUENCE_DECLARATION_KEYS`): `$state`, `template`, `default-values`

#### 4. Referenced Child Sequence Boundary (Nested `sequence: <file-ref>`, `prompt:` → Sequence, and Root `sequence: <file-ref>`)

A referenced formal child sequence preserves **modular isolation** via a **split boundary**.

- **Scope of the split boundary (`is_formal_sequence == true`):** The split boundary applies specifically to **referenced formal child sequence documents** (`is_formal_sequence == true`, such as `_provider-tasks.yaml` or another Markdown/YAML sequence document referenced via a nested task `sequence: <file-ref>`, a `prompt:` task resolving to a sequence, or a root document's `sequence: <file-ref>`).
- **Informal data-list step sources keep the owning Markdown document's root `fm`:** When a Markdown document's `sequence:` points to an **informal data-file, offset, or operator step list** feeding that Markdown document's own body steps (for example, `claudine/docs/research/sandboxing/providers/_fleet.md` reading `claudine/docs/providers.yaml`), the target is step data rather than a formal child sequence (`is_formal_sequence == false`), so it keeps the owning Markdown document's own root `fm` without crossing a split boundary.

Across a split boundary (for `is_formal_sequence == true`):

- **Does NOT cross:** The parent/root sequence's own authored root `fm` (including authored root `agent:` or `model:`) does **not** leak into the referenced formal child sequence document. Authored root properties stay scoped to the parent document and its direct prompt tasks.
- **DOES cross:**
  - **Explicit `with:`** on the calling task (applied as a defined override on top of the child sequence's root `fm`).
  - **Parent runtime `set` and CLI `key=value`** state (including root `initialize` `set:` writes for a root `sequence: <file-ref>`).
  - **Inherited `default-values:`** (propagating recursively into the child sequence, with nearer defaults winning).

Exact precedence inside a referenced formal child sequence, **highest to lowest**:

- **Defined channel (`fm`):** `task with:` > `parent runtime set` > `parent CLI key=value` > `child authored fm`
- **Defaults channel (`default-values:`):** `child sequence default-values:` > `calling task default-values:` > `parent sequence default-values:`

#### 5. Group Legs

- Group legs run child targets like tasks and accept `with:` and `default-values:` with identical semantics whenever their leg executable permits them.
- Groups themselves do **not** gain a `default-values:` property in this specification.

#### 6. Illustrative Examples

```yaml
kind: sequence
default-values:
    agent: claude          # --codex on the CLI replaces this on a YAML sequence
    model: sonnet          # --model haiku on the CLI replaces this on a YAML sequence
sequence:
    - name: find
      compose: ./_find.md                  # _find.md names no agent -> claude, sonnet
    - name: review
      compose: ./_review.md                # _review.md sets `agent: codex` -> codex (prompt wins over default)
    - name: summary
      compose: ./_summary.md
      default-values: { model: opus }      # claude, opus, unless _summary.md names a model
    - name: notes
      compose: ./_notes.md
      with: { agent: gemini }              # gemini, regardless of what _notes.md sets
    - name: unpin
      compose: ./_review.md
      with: { agent: null }                # clears _review.md's `agent: codex` -> falls back to default-values.agent (claude)
    - name: subflow
      sequence: ./sub-sequence.yaml        # receives inherited default-values (sub-sequence's own
      with: { topic: auth }                # default-values win); only explicit `with:` + CLI/runtime set cross
```

A root `agent:` (outside `default-values:`) is a **defined** value, so it overrides direct child prompts unless a task's `with:` overrides or clears it:

```yaml
kind: sequence
agent: claude              # every direct prompt task runs claude unless its task's `with:` says otherwise
sequence:
    - name: find
      compose: ./_find.md
```

#### 7. Package Ownership for Default Layering

- **`darkmatter-lib`** adds a small data-origin defaults capability and safe conditional file-path provenance so relative file paths authored in a default layer resolve from the file that authored the default, without clobbering child-authored values.
- **`claudine-lib`** owns Caller → Callee layering, recursive pre-flight, unified document classification, and model/provider prediction.
- **`claudine-cli`** owns CLI flag wiring (`--<provider>`, `--model`, `key=value`), conflict checking, `--dry-run` exit codes, and launch resolution.

---

### Decision 3 — Schema-Driven `isolated` Constraint, Null Semantics & Inline Nested Sequences

#### 1. The `isolated` Schema Constraint

- Add a new property-level metadata constraint `isolated` in **`simplified-schema/lib`**, following the implementation pattern of `Constraint::Generated`.
- Lower `isolated` to `"x-darkmatter-isolated": true` in JSON Schema across every property form:
  1. primitives (e.g., `string(isolated)`)
  2. arrays
  3. inline and block objects via `$constraints`
  4. imported and union references (e.g., `steps(required; isolated)@./sequence.yaml`)
  5. multi-arm YAML-list property unions
- **Sticky union semantics:** `EffectiveSchema::isolated_keys()` in `darkmatter/lib` returns the **union** of all `"x-darkmatter-isolated": true` keys across the baseline schema, all matched triggers, and the document's own `$schema`. A user document's `$schema` can therefore mark additional custom keys as `isolated` without accidentally un-isolating host or framework controls.

#### 2. Full Cross-Stack Rollout

`isolated` is wired end-to-end across all three layers in this specification:

1. **`simplified-schema`**: parses and lowers `isolated` → `"x-darkmatter-isolated": true`.
2. **`darkmatter`**: `::file` / `as_markdown()` transclusion and preflight (all `inherited_child_options` sites) read `EffectiveSchema::isolated_keys()`, replacing the hardcoded 3-key `prologue`/`epilogue`/`ctx` exclusion list.
3. **`claudine`**: sequence value propagation consults `EffectiveSchema::isolated_keys()` to decide which owning-sequence `fm` keys are excluded from the defined-value implicit inheritance channel (without restricting explicit `with:`).

#### 3. Schema Layering & `default-values` Propagation

| Schema File | Marked `isolated` | Explicitly Non-Isolated |
| ----------- | ----------------- | ----------------------- |
| `darkmatter/schemas/partials/fm.yaml` | Document-processing controls: `prologue`, `epilogue`, `ctx`, `$schema`, `hash`, `last_updated`, `change`, `style`, `content_policy`, `ignore_invalid`, `interpolate_code_blocks`, `md` lifecycle events | `replace`, `title`, `description`, `tags`, `draft`, `metadata` (preserved non-isolated so existing `::file` partials stay compatible) |
| `claudine/registration/sequence-document.yaml` & `claudine/registration/formal-sequence-file.yaml` | `replace`, `title`, `description`, `tags`, `draft`, `metadata`, plus all sequence controls: `kind`, `sequence`, `template`, `$state`, `default-values`, `fail_fast` *(note: `sections` is declared in `claudine/registration/inline-composition.yaml` and `claudine/registration/document.yaml`)* | User-authored domain frontmatter keys, `agent`, `model` |
| `claudine/registration/shared.yaml` (`common-attributes`), `composition.yaml`, `inline-composition.yaml`, `document.yaml`, `linking.yaml` | All Claudine, inline-composition (including `sections`), linking, and harness control keys | `agent`, `model` (enforced by an L1 schema-parity test in `claudine-lib`) |

Running root `just shipped-schemas` regenerates all four checked-in precompiled schema artifacts atomically (`simplified-schema/lib/generated/shipped.json`, `darkmatter/lib/generated/shipped.json`, `claudine/lib/generated/shipped.json`, and `claudine/lib/generated/globals.lifecycle.json`).

> **Why `default-values` is marked `isolated`:** Marking `default-values` as `isolated` prevents the parent sequence's `default-values` object from leaking into child documents through the **defined-value override channel** (where it would wrongly overwrite child-authored values or appear as a raw override). Instead, `default-values` propagates recursively through the dedicated **child-wins defaults inheritance channel** defined in Decision 2.

`template:` remains unrelated to `default-values:`: `template:` supplies defaults for each step's own step-state (`seq.*`), not for the `fm` of documents executed by steps.

#### 4. Null Semantics & Simplified Schema Scope

- **No new `null-is-undefined` constraint in Simplified Schema:** Explicitly considered and ruled out — existing absent-equals-null behavior (`2026-11-14-absent-equals-null`) plus uniform `with:` merge semantics (Decision 2.3) cover all required cases cleanly without adding a new schema constraint.
- An explicit `null` in a child document's frontmatter is treated as absent and falls back to `default-values:`. An explicit `with: { key: null }` merges onto the workspace first, clearing any pinned value, and normal resolution then consults `default-values:` (or fails/prompts if required and unresolved).

#### 5. Inline Nested Sequences (`sequence: [ ... ]`)

- An inline nested `sequence: [ ... ]` block **allows** `default-values:` (scoping fallback defaults to that sub-block, with nearer defaults winning over outer defaults).
- An inline nested `sequence: [ ... ]` block **forbids** `with:` (`composition.task_option_invalid`), since it executes within the same sequence file's `fm` scope rather than crossing a file boundary.

---

### Decision 4 — Document Classification & Sequence Forms

*(This terminology and matrix must also be documented clearly under `claudine/docs/` so authors have a single canonical reference for Markdown vs. YAML/data sequence forms.)*

#### 1. Unified Pure Document Classifier (`claudine-lib`)

One pure classifier in `claudine-lib` replaces the three fragmented checks currently split across `mismatch.rs`, `preflight/mod.rs::load_prompt`, and `commands/wrap/sequence/mod.rs`.

- Inspects **authored frontmatter only** (never inherited parent `fm` or `with:` values).
- Applies **absent-equals-null semantics** (`key: null` is treated identically to `key` absent).
- **`sections:` readiness:** Classification recognition of `sections:` is **deferred until `2026-07-02-inline-sections` ships**. `sections` remains declared and marked `isolated` in `claudine/registration/inline-composition.yaml` and `claudine/registration/document.yaml`, and the classifier leaves a clean one-line extension hook (`has_prompt || has_sections`) for `2026-07-02-inline-sections` to enable when that feature lands.

#### 2. Document Classification Matrix

| Document Class | Detection (Authored Frontmatter Only; `null` = Absent) | `compose:` | `inline-compose:` | `sequence:` | `prompt:` |
| --- | --- | --- | --- | --- | --- |
| **Compose doc** | Markdown; no `sequence`, no `prompt` | **Allowed** (compose) | Error (`composition.operation_mismatch`) | Error (`composition.operation_mismatch`) | **Allowed** (dispatches to compose) |
| **Inline-compose doc** | Markdown; no `sequence`, has `prompt` *(+ `sections` once `2026-07-02-inline-sections` lands)* | Error (`composition.operation_mismatch`) | **Allowed** (inline-compose) | Error (`composition.operation_mismatch`) | **Allowed** (dispatches to inline-compose) |
| **Compose sequence** | Markdown; has `sequence`, no `prompt` (each body-default step composes the document body as its prompt) | Error (`composition.operation_mismatch`) | Error (`composition.operation_mismatch`) | **Allowed** (compose sequence) | **Allowed** (dispatches to child sequence) |
| **Inline sequence** | Markdown; has `sequence`, has `prompt` *(+ `sections` once `2026-07-02-inline-sections` lands)* (each body-default step mutates the document body inline) | Error (`composition.operation_mismatch`) | Error (`composition.operation_mismatch`) | **Allowed** (inline sequence) | **Allowed** (dispatches to child sequence) |
| **Pure / data sequence** | YAML, JSON, JSON5, JSONL, or NDJSON (formal `kind: sequence`, or informal/foreign array or offset sequence) | Error (`composition.operation_mismatch` — Markdown only) | Error (`composition.operation_mismatch` — Markdown only) | **Allowed** (pure sequence) | **Allowed** (plain `<file-ref>` only; dispatches to child sequence) |

#### 3. YAML & Data Sequence Source Grammar

The full live sequence source grammar in `claudine/lib/src/composition/sequence/grammar.rs` is preserved unchanged for `sequence:`:

```text
sequence: <file-ref> [-> <offset.path>] [::<operator>(<args>)]
```

- **Supported source formats:** YAML, JSON, JSON5, JSONL, NDJSON, and Markdown frontmatter.
- **Offset syntax:** Dot-notation offset via `->` — e.g., `./foobar.yaml -> data` *(note: the initial draft's `./foobar.yaml::data` sketch was a typo; `->` is the offset separator and `::` introduces the operator)*.
- **Operators:** `::map(from, to)`, `::name(from)`, and `::template(expr)` (at most one operator per reference, parsed span-aware so `->` and `::` inside quotes or `{{ … }}` remain literal).
- **Strict vs. lenient normalization:** Formal sequence documents (`kind: sequence`) follow strict normalization; informal/foreign data files (e.g., an array of strings or objects, or an offset slice into a data file) follow lenient normalization.
- **`prompt:` restriction:** `prompt:` does **not** accept `-> <offset.path>` or `::<operator>(<args>)`; data-slicing and operator projection remain exclusive to `sequence:`.

#### 4. Dynamic `{{ expr }}` Target Paths

Target references on `compose:`, `inline-compose:`, `sequence:`, and `prompt:` may contain `{{ expr }}` interpolation:

- **Statically resolvable targets** (literal paths or expressions resolvable from static frontmatter/CLI inputs) are classified and pre-flight checked upfront identically to literal references.
- **Unresolved dynamic targets** (paths depending on `seq.*`, `loop.*`, `group.*`, a later step's `set`, `$( ... )`, or an unrun `initialize` during passive `--dry-run`):
  - **In `--dry-run`:** Emit `dm.check.not_checked` (`info`) instead of aborting.
  - **In real pre-flight & `--dry-run` agent validation:** Defer document classification to runtime — strictly enforcing the Classification Matrix the moment the path resolves. For recursive `agent` pre-flight, if an ancestor, calling task, or ordered `set: { agent: ... }` supplies `agent`, pre-flight passes; if nothing above supplies `agent`, pre-flight emits `composition.agent_unverifiable_target` (**`warning`**, exiting `1` in `--dry-run` when no errors are present, matching `composition.agent_conditional_set` and real pre-flight's advisory warning).
  - **Shell pre-approval boundary:** For a dynamically chosen child sequence (`sequence: "{{ next_seq }}"`), inner shell commands inside the target file cannot be discovered or pre-approved before step 1 unless already pre-approved by policy or `--yolo` is set.

---

### Decision 5 — CLI `--<provider>` / `--model`, Model Precedence & `get_default_model`

#### 1. Unifying CLI `--<provider>` into `fm` State

The parallel `explicit_provider` override channel in composition and sequence execution is eliminated; `--<provider>` (`--claude`, `--codex`, …, `--provider <name>`) writes directly into `fm` state:

| Command | Target Format | Where `--<provider>` Writes | Effect |
| ------- | ------------- | --------------------------- | ------ |
| `claudine compose` | Markdown | Target document's `fm.agent` | Same priority as CLI `agent=<provider>` (overrides authored `agent`) |
| `claudine inline-compose` | Markdown | Target document's `fm.agent` | Same priority as CLI `agent=<provider>` (overrides authored `agent`) |
| `claudine sequence` | **Markdown** | Root sequence's `fm.agent` | Defined value on the root sequence (overrides direct prompt tasks unless overridden/cleared by task `with:`) |
| `claudine sequence` | **YAML / data** | Root sequence's `default-values.agent` | Fallback default for tasks/descendants that do not specify an agent; task `with:`, root `agent:`, and child prompt `agent:` all win over it |

- **Disambiguation:** On `claudine sequence`, `--<provider>` disambiguation is strictly by **file format** (Markdown vs. YAML/data).
- **CLI conflict rule:** Passing both `--<provider>` (or `--provider <name>`) and `agent=<val>` on the same command is a **hard CLI usage error** (`usage.invalid_argument`, exit `2`).

#### 2. Caller → Callee Symmetry for `--model`

`--model` follows the exact same Caller → Callee symmetry as `--<provider>` (eliminating any need for a tie-breaker between `--model` and `with.model`):

| Command | Target Format | Where `--model` Writes | Effect |
| ------- | ------------- | ---------------------- | ------ |
| `claudine compose` | Markdown | Target document's `fm.model` | Caller-passed `model` (same rank as CLI `model=<val>`) |
| `claudine inline-compose` | Markdown | Target document's `fm.model` | Caller-passed `model` (same rank as CLI `model=<val>`) |
| `claudine sequence` | **Markdown** | Root sequence's `fm.model` | Caller-passed `model` on the root sequence |
| `claudine sequence` | **YAML / data** | Root sequence's `default-values.model` | Fallback default so multi-provider YAML sequences never crash when `--model` is passed; task `with.model`, child prompt `fm.model`, and provider-specific overrides all work cleanly |

- **CLI conflict rule (symmetric with `--<provider>`):** Passing both `--model <val>` and `model=<val>` on the same command is a **hard CLI usage error** (`usage.invalid_argument`, exit `2`), matching `--<provider>` + `agent=<val>`.
- On a task call inside a sequence, `with: { model: ... }` is simply the parent sequence (acting as caller) setting `model` for that single child call without mutating the parent's own state.

#### 3. Six-Tier Model Precedence Chain

`model` is **never mandatory to author** and is **not `isolated`**. At any document invocation, model resolution checks **highest to lowest**:

| Tier | Source | Notes |
| :---: | --- | --- |
| 1 | **Caller-passed `model`** | CLI `--model` / `model=` on direct or Markdown invocation, or parent `with.model` on a task call |
| 2 | **Provider-specific model env var (`<PROVIDER>_MODEL`)** | Checked in each provider's declared `model_env_vars` order — currently 7 providers (`CLAUDE_MODEL`/`ANTHROPIC_MODEL`, `CODEX_MODEL`/`OPENAI_MODEL`, `GEMINI_MODEL`, `GOOSE_MODEL`, `KIMI_MODEL`, `OPENCODE_MODEL`, `QWEN_MODEL`); `kilo`, `pi`, and `antigravity` declare none |
| 3 | **Generic `MODEL` env var** | Sole generic env var — **do NOT add `CLAUDINE_MODEL`** |
| 4 | **Document's authored `fm.model` (and owning sequence's authored or runtime-`set` `fm.model` for direct prompt tasks)** | CLI `--model` / `model=` on direct or Markdown runs already sits in Tier 1; for direct prompt tasks, the owning sequence's authored or runtime-`set` `fm.model` outranks the child document's authored `fm.model` (Rank 2 vs. Rank 3 in Decision 2.2) |
| 5 | **`default-values.model`** | `task default-values.model` > `sequence default-values.model` > inherited parent `default-values.model` (including CLI `--model` on YAML sequences) |
| 6 | **Provider's configured default model** | Resolved via `get_default_model(provider: Provider)` in `claudine-lib` |

#### 4. Unified `get_default_model(provider: Provider)` Operational Contract (`claudine-lib`)

One fast, passive, provider-neutral config lookup function in `claudine-lib` serves all **10 registered providers** (`claude`, `codex`, `gemini`, `goose`, `kimi`, `opencode`, `qwen`, `kilo`, `pi`, `antigravity`), eliminating OpenCode as a special case:

- **Strictly passive local reads only:** Zero subprocesses and zero network calls. Honors `DocumentEnvironment`, `HomeBaseline`, XDG (`XDG_CONFIG_HOME`), and Windows `%APPDATA%` conventions.
- **Latency & memoization:** Target `< 10ms` per provider, memoized once per invocation keyed by `(Provider, config_root)`.
- **Context & visibility:** Populates `ctx.model` and `env.MODEL` with the discovered default model name (falling back to `"default"` when `get_default_model` returns `None`), and surfaces both the resolved model and its provenance (tier/source) in `--dry-run` output and execution logs.
- **Child `--model` argv forwarding:**
  - **Tiers 1–5** (caller, env var, authored `fm.model`, or `default-values.model`): **always** forward `--model` (or provider equivalent) to the child provider process.
  - **Tier 6** (discovered provider default from `get_default_model`): forward **only** when `ProviderInfo::model_required_in_non_tty` is true (OpenCode) **or** when a shadow-home / MCP overlay hides the provider's native config root.
- **OpenCode / `model_required_in_non_tty`:** If a provider with `model_required_in_non_tty` still has no model after checking tiers 1–5 and `get_default_model` (tier 6), pre-flight reports `composition.model_required` (`error`).
- **Malformed local provider config:** Emit one deduplicated `config.provider_default_unreadable` **`info`** finding and fall back to `"default"`. Third-party config quirks therefore never break CI `--dry-run` exit `0`, while non-TTY OpenCode without a usable model still fails with `composition.model_required`.

---

### Decision 6 — Recursive `agent` Pre-Flight Validation

To prevent sequences from running halfway and failing on a downstream task with no agent:

#### 1. Core Rules

- `agent` is required for every runnable prompt execution (`compose`, `inline-compose`, `prompt`, and Markdown body-default steps). Resolution consults `default-values.agent` when no defined or document-authored `agent` is present.
- For a **YAML/data** sequence, `agent` is **not** required on the sequence root itself.
- Because `default-values` inherits recursively into child sequences, setting `default-values.agent` on any ancestor sequence satisfies all descendants.
- When **no** ancestor provides `default-values.agent` (and no root defined `agent` covers direct prompt tasks), pre-flight validates every composition task (`compose`, `inline-compose`, `sequence`, `prompt`):
  1. The task explicitly supplies `agent` via `with:` or `default-values:`, **OR**
  2. The referenced child file defines `agent` or `default-values.agent` in its authored frontmatter, **OR**
  3. An eligible **ordered runtime `set: { agent: ... }`** covers the task (see §2 below) — **targeting `agent` only** (`default-values` is forbidden as a `set:` target per Decision 2.1) — emitting `composition.agent_conditional_set` (`warning`), **OR**
  4. The child is a YAML/data sequence without root `agent` or `default-values.agent`, in which case pre-flight **recurses into that child sequence's own tasks** applying rules 1–4.

#### 2. Ordered `set: { agent: ... }` Tracking

Pre-flight statically recognizes `set: { agent: ... }` (without evaluating runtime `when:` branches) **only** from causally preceding sources:

- **(a)** The target document's own `initialize` lifecycle event,
- **(b)** Any enclosing ancestor sequence or group's `initialize` lifecycle event (including root `initialize`), or
- **(c)** Any **earlier** sequential step's `side_effect: { set: { agent: ... } }`.

Pre-flight **never** credits a later step or a concurrent sibling group leg. Whenever a task relies solely on runtime `set: { agent: ... }` (from (a), (b), or (c)), pre-flight passes that task with `composition.agent_conditional_set` (`warning`) noting that conditional/flow-control logic could skip the `set:` at runtime.

#### 3. Container Traversal, Cycle Detection & Dynamic Targets

- **Full container traversal:** Pre-flight traverses `group:` (inline objects, file references, and catalog entries), `task:` (`kind: task` files), inline nested `sequence: [ ... ]`, referenced `sequence: <file-ref>`, and `prompt:`-dispatched child sequences.
- **Cycle detection:** Reuses existing `SequenceReferenceCycle` detection across both `sequence:` and `prompt:`-dispatched sequences.
- **Dynamic targets:** Unresolved dynamic `{{ expr }}` target paths follow Decision 4.4 (pass if an ancestor/task/ordered `set:` provides `agent`; otherwise emit `composition.agent_unverifiable_target` (`warning`), exiting `1` in `--dry-run` when no errors are present).

#### 4. TTY Review vs. Non-TTY Fail-Fast & `--silent`

- **Interactive TTY:** Preserves the existing `review_sequence` `InputTable` flow and keeps `preferred_agent` as a picker seed only (never silently selecting a provider without review).
- **Non-TTY & `--silent`:** Treats `--silent` as non-interactive (failing fast instead of opening a TUI screen). Non-TTY failures name the exact nested task, file path, and full container trail (`Trail`) leading to the unresolved task, rendered with the updated defined and default provider-chain links.

---

### Decision 7 — Error Taxonomy & `--dry-run` Exit Codes

#### 1. Diagnostic Codes

All new Claudine diagnostics are added additively to `claudine/lib/src/diagnostics/registry.rs`, follow `2026-11-22-better-errors-and-dry-run`'s **What / Where / Why / Hint** message standard, and preserve all locked `err.*` lifecycle codes.

##### New Diagnostic Codes

| Code | Severity | Meaning & Trigger |
| --- | --- | --- |
| `composition.operation_mismatch` | `error` | Explicit `compose:`, `inline-compose:`, or `sequence:` targets a document of an incompatible class or unsupported format (per the Decision 4.2 matrix). |
| `composition.task_option_invalid` | `error` | A task or group leg declares an option forbidden for its executable (e.g., `with:` on inline `sequence: [...]`, `with:`/`default-values:` on `task:` or a zero-executable body step, explicit `operation:`/`flow:` on `prompt:` targeting a sequence). |
| `composition.default_values_invalid` | `error` | `default-values:` has an invalid shape (not a mapping/object) or contains forbidden reserved sequence keys. |
| `composition.agent_conditional_set` | `warning` | Recursive `agent` pre-flight accepted a task solely because an eligible `initialize` or earlier `side_effect` contains `set: { agent: ... }`, which conditional/flow-control logic could skip at runtime. |
| `composition.agent_unverifiable_target` | `warning` | A dynamic `{{ expr }}` target path cannot be statically inspected and no ancestor, task, or ordered `set:` supplies `agent`. Exits `1` in `--dry-run` when no errors are present, matching `composition.agent_conditional_set` and real pre-flight's advisory warning. |
| `config.provider_default_unreadable` | `info` | Local provider config file inspected by `get_default_model` is malformed or unreadable; emitted once per `(Provider, config_root)` and falls back to `"default"`. |

##### Reused / Updated Diagnostic Codes

| Code | Severity | Role in This Specification |
| --- | --- | --- |
| `composition.lifecycle_invalid` | `error` | Reused when a runtime `set:` or lifecycle write targets `default-values` (now in `SEQUENCE_DECLARATION_KEYS`). |
| `usage.invalid_argument` | `error` | Hard CLI usage error (exit `2`) when `--<provider>` (or `--provider <name>`) + `agent=<val>` or `--model <val>` + `model=<val>` appear on the same command. |
| `dm.reference.missing_document` | `error` | Missing file targets on `compose:`, `inline-compose:`, `sequence:`, and `prompt:` (with nearby-file suggestions). |
| `dm.check.not_checked` | `info` | Dynamic `{{ expr }}` target paths that cannot be resolved passively during `--dry-run`. |
| `composition.no_executable` | `error` / `info` | Updated via `ExecutableField::noun` to include `compose` and `inline-compose` alongside `prompt`. |
| `composition.selection_key_as_data` | `warning` | Updated hint to recommend both `with:` and `default-values:` for `agent` and `model`. |
| `composition.provider_unresolved` | `error` / `warning` | Updated provider selection-chain explanation showing defined and `default-values:` links in precedence order, plus full nested task/container trail. |
| `composition.model_required` | `error` | Non-TTY provider with `model_required_in_non_tty` (OpenCode) still has no model after tiers 1–5 and `get_default_model` (tier 6). |
| `composition.missing_properties` | `error` / `warning` | Reused for unresolved required inputs after applying Caller → Callee `fm` and `default-values:` layering. |

#### 2. `--dry-run` Exit Codes *(Supersedes `2026-11-22-better-errors-and-dry-run`)*

> **Note:** This table refines the exit-status table in `2026-11-22-better-errors-and-dry-run`, which combined errors and warnings under exit `2`. `2026-11-22-better-errors-and-dry-run` is kept untouched as its reviewed snapshot.

| Exit Code | Meaning |
| :---: | --- |
| **`0`** | Clean: no `error` and no `warning` findings (`info` findings only, or none). |
| **`1`** | Warnings only: no `error` findings, one or more `warning` findings. |
| **`2`** | Errors: one or more `error` findings, or a CLI usage error (e.g., unknown flag, `--<provider>` + `agent=<val>`, or `--model <val>` + `model=<val>`). |

---

## Breaking Changes & Migration

The following confirmed behavior changes differ from current behavior:

1. **CLI `--<provider>` and `--model` on YAML sequences become defaults, not global overrides.**
   - *Before:* `--claude` forced every task in a sequence to run Claude, overriding task `with:` and child prompts' own `agent:`.
   - *After:* On a YAML/data sequence, `--<provider>` sets `default-values.agent` and `--model` sets `default-values.model`, so task `with:`, root `agent:`/`model:`, and child prompts' own `agent:`/`model:` win over them. (On a Markdown sequence and on single-document `compose` / `inline-compose`, `--<provider>` and `--model` set `fm.agent` and `fm.model`.)
2. **CLI `--<provider>` + `agent=<val>` and `--model <val>` + `model=<val>` on the same command become hard errors.**
   - *After:* Both conflict pairs emit `usage.invalid_argument` (exit `2`) instead of silently picking a winner.
3. **Root `agent:` and `model:` on a sequence become defined overrides for direct prompt tasks.**
   - *Before:* A root `agent:` acted as a fallback when a task's `with:` and child prompt named no agent, and the child prompt's own `agent:` beat it.
   - *After:* Root `agent:` and `model:` are non-isolated defined values, overriding direct child prompts' authored values (unless overridden or cleared via task `with:`), while still respecting the split boundary for referenced formal child sequence documents. Authors who want fallback behavior move them under `default-values:`.
4. **Schema-driven `isolated` replaces Darkmatter's hardcoded 3-key exclusion list.**
   - *Before:* `::file` / `as_markdown()` only isolated `prologue`, `epilogue`, and `ctx`.
   - *After:* `darkmatter/schemas/partials/fm.yaml` also isolates `$schema`, `hash`, `last_updated`, `change`, `style`, `content_policy`, `ignore_invalid`, `interpolate_code_blocks`, and `md` lifecycle events. `replace`, `title`, `description`, `tags`, `draft`, and `metadata` remain non-isolated in Darkmatter partials for compatibility and are isolated only at the Claudine sequence layer.
5. **`--dry-run` warnings-only exit code changes from `2` to `1`.**
   - *Before (as designed in `2026-11-22-better-errors-and-dry-run`):* Both errors and warnings exited `2`.
   - *After:* Warnings with no errors exit `1`; errors (and CLI usage errors) exit `2`; clean / `info`-only exits `0`.

In-repo migration of the 21 affected research files is pre-authorized for the same implementation PR; see [Pre-Authorized Preparatory & Cross-Cutting Actions](#pre-authorized-preparatory--cross-cutting-actions) below.

---

## Pre-Authorized Preparatory & Cross-Cutting Actions

The following preparatory, migration, and documentation updates are explicitly pre-authorized as part of implementing this specification:

1. **Regenerate the 4 checked-in schema bundles atomically** via root `just shipped-schemas`:
   - `simplified-schema/lib/generated/shipped.json`
   - `darkmatter/lib/generated/shipped.json`
   - `claudine/lib/generated/shipped.json`
   - `claudine/lib/generated/globals.lifecycle.json`
2. **Migrate the 21 in-repo research files in the same implementation PR:**
   - The **20 coordinator `claudine/docs/research/*/_fleet.md` files** (19 `_provider-tasks.yaml` coordinators plus `claudine/docs/research/local_runners/_fleet.md`), and
   - **`claudine/docs/research/_TEMPLATE.md`**.
   - Migration scope: move root `agent:` / `model:` under `default-values:` so these coordinator fleets preserve their intended fallback semantics under the new Caller → Callee model. Leave `agent-cli`, `reasoning-level`, and `sandboxing/providers` untouched.
3. **Update canonical docs, caller skills, and repo skills:**
   - Canonical docs under `simplified-schema/docs/`, `darkmatter/docs/`, and `claudine/docs/` (covering `isolated`, Markdown vs. YAML/data sequence forms and the Document Classification Matrix, `compose:`/`inline-compose:`/`prompt:`/`sequence:` rules, Caller → Callee `fm` layering and `default-values:`, unified `--<provider>`/`--model` and six-tier model precedence, recursive `agent` pre-flight, and three-way `--dry-run` exit codes).
   - Caller skills under `claudine/docs/use-skills/use-claudine/{sequences,cli,composition}.md` (and `darkmatter/docs/use-skills/use-darkmatter/authoring.md` if needed), followed by running `just publish-skills`.
   - Repo skills under `.claude/skills/` reflecting the new schema constraint, classifier, and layering architecture.
4. **Leave `2026-11-22-better-errors-and-dry-run` untouched:**
   - Keep `2026-11-22-better-errors-and-dry-run` untouched as its reviewed snapshot; record the two supersessions (retained `prompt:` alongside `compose:`/`inline-compose:` and `--dry-run` `0`/`1`/`2` exit codes) exclusively in this specification.

---

## Acceptance Criteria & Definition of Done

Implementation is complete when all package-level acceptance criteria and cross-cutting gates below pass:

### `simplified-schema/lib`

- Parses property-level `isolated` constraint and lowers it to `"x-darkmatter-isolated": true` in JSON Schema across all five property forms: primitives, arrays, inline/block `$constraints`, imported/union references (e.g., `steps(required; isolated)@./sequence.yaml`), and multi-arm YAML-list property unions.
- Does **not** introduce a `null-is-undefined` constraint.
- L1 tests cover parsing, lowering, round-trip compilation, and invalid usage errors for `isolated`.

### `darkmatter/lib`

- `EffectiveSchema::isolated_keys()` returns the sticky union of `"x-darkmatter-isolated": true` keys across the baseline schema, matched triggers, and the document's own `$schema` (a user `$schema` can add isolated keys and cannot un-isolate framework keys).
- All `::file` / `as_markdown()` transclusion and preflight `inherited_child_options` sites consume `EffectiveSchema::isolated_keys()`, removing the hardcoded 3-key `prologue`/`epilogue`/`ctx` list.
- `darkmatter/schemas/partials/fm.yaml` marks document-processing controls `isolated` while keeping `replace`, `title`, `description`, `tags`, `draft`, and `metadata` non-isolated so existing `::file` partials remain compatible.
- Provides data-origin defaults layering with safe conditional file-path provenance so relative file paths authored in a default layer resolve relative to the file that authored the default without overwriting child-authored values.
- L1 tests cover sticky union semantics, transclusion isolation, existing partial compatibility, and conditional file-path provenance.

### `claudine/lib`

- **Task grammar & options:** `compose:` and `inline-compose:` accept Markdown `<file-ref>` only; `prompt:` accepts plain `<file-ref>` only (rejecting `->` / `::`, rejecting explicit `operation:`/`flow:` when dispatching to a sequence, and silently ignoring inherited group `operation`/`flow` defaults); `sequence:` preserves full `<file-ref> [-> offset] [::op(args)]` grammar; `task:` remains immutable at call sites (`name` and `when` only); inline nested `sequence: [ ... ]` allows `default-values:` and rejects `with:`; zero-executable Markdown body steps reject `with:` and `default-values:`; group legs accept `with:` and `default-values:` identically to tasks; all forbidden options emit `composition.task_option_invalid`.
- **Caller → Callee & split boundary:** Six-rank precedence for direct prompt tasks and uniform `with:` merge semantics (including `with: { agent: null }` clearing pinned values and falling back to `default-values:`) are verified by unit and integration tests. Referenced formal child sequence documents (`is_formal_sequence == true`, nested or root) enforce the split boundary; informal data-file/offset/operator step lists (`is_formal_sequence == false`) keep the owning Markdown document's root `fm`. `"default-values"` is in `SEQUENCE_DECLARATION_KEYS` and rejected on `set:` writes (`composition.lifecycle_invalid`).
- **Schema parity:** `claudine/registration/{sequence.yaml,sequence-document.yaml,formal-sequence-file.yaml,shared.yaml,composition.yaml,inline-composition.yaml,document.yaml,linking.yaml}` mark sequence/document/linking controls `isolated` while keeping `agent` and `model` non-isolated, enforced by an L1 schema-parity test in `claudine-lib`.
- **Unified pure document classifier:** Replaces the three fragmented classification checks (`mismatch.rs`, `preflight/mod.rs::load_prompt`, `commands/wrap/sequence/mod.rs`), inspects authored frontmatter only with absent-equals-null semantics, enforces every cell of the Decision 4.2 matrix (`composition.operation_mismatch`), leaves the `sections:` hook inactive until `2026-07-02-inline-sections`, and defers unresolved dynamic `{{ expr }}` targets to runtime (`dm.check.not_checked` in `--dry-run`).
- **Recursive `agent` pre-flight:** Traverses nested sequences, `prompt:`-dispatched sequences, inline `sequence: [ ... ]`, `group:` (inline/file/catalog), and `task:` files with cycle detection; credits only causally preceding `initialize` and earlier sequential `side_effect` `set: { agent: ... }` (`composition.agent_conditional_set`, `warning`); emits `composition.agent_unverifiable_target` (`warning`) for unresolved dynamic targets without an upstream agent; treats `--silent` as non-interactive fail-fast with full nested `Trail` reporting while preserving interactive TTY `review_sequence`.
- **Six-tier model precedence & `get_default_model`:** Implements the six-tier chain (without `CLAUDINE_MODEL`), honoring declared `model_env_vars` for Tier 2 and authored/runtime-`set` owning-sequence `fm.model` for Tier 4. `get_default_model(provider: Provider)` covers all 10 registered providers using strictly passive local reads (`< 10ms` target, memoized per `(Provider, config_root)`, cross-OS path discovery), populates `ctx.model`/`env.MODEL` with provenance in `--dry-run`/logs, enforces Tier 1–5 vs. Tier 6 `--model` child argv forwarding (including shadow-home/MCP overlay and `model_required_in_non_tty`), emits deduplicated `config.provider_default_unreadable` (`info`) on malformed configs, and emits `composition.model_required` (`error`) when non-TTY OpenCode still lacks a model.
- **Diagnostics:** Registers all 6 new diagnostic codes and updates all reused codes in `claudine/lib/src/diagnostics/registry.rs` adhering to What/Where/Why/Hint and preserving locked `err.*` codes.

### `claudine/cli`

- Eliminates the parallel `explicit_provider` override channel; wires `--<provider>` (`--claude`, `--codex`, …, `--provider <name>`) and `--model` into `fm.agent`/`fm.model` for `compose`, `inline-compose`, and Markdown `sequence`, and into `default-values.agent`/`default-values.model` for YAML/data `sequence`.
- Rejects both `--<provider>` + `agent=<val>` and `--model <val>` + `model=<val>` conflicts on the same command with `usage.invalid_argument` (exit `2`).
- Maps `--dry-run` exit status to `0` (clean or `info`-only), `1` (warnings with no errors — including `composition.agent_conditional_set` and `composition.agent_unverifiable_target`), and `2` (any `error` or CLI usage error).
- L1/L2 CLI tests verify flag wiring, both conflict errors, `--dry-run` `0`/`1`/`2` exit codes, `--silent` fail-fast, and model provenance rendering.

### Cross-Cutting Definition of Done

- All 4 checked-in schema bundles (`simplified-schema/lib/generated/shipped.json`, `darkmatter/lib/generated/shipped.json`, `claudine/lib/generated/shipped.json`, `claudine/lib/generated/globals.lifecycle.json`) are freshly regenerated via root `just shipped-schemas` with zero uncommitted drift.
- All canonical docs (`simplified-schema/docs/`, `darkmatter/docs/`, `claudine/docs/`), caller skills (`claudine/docs/use-skills/use-claudine/{sequences,cli,composition}.md` and `darkmatter/docs/use-skills/use-darkmatter/authoring.md` if needed, published with `just publish-skills`), and repo skills (`.claude/skills/`) are updated with zero drift against shipped behavior.
- All 21 migrated research files (20 coordinator `claudine/docs/research/*/_fleet.md` files + `claudine/docs/research/_TEMPLATE.md`) pass `claudine sequence --dry-run` cleanly, and `agent-cli`, `reasoning-level`, and `sandboxing/providers` remain untouched and valid.
- `just test` and `just lint` pass across `simplified-schema`, `darkmatter`, and `claudine` on all supported OS environments.

---

## Candidate Spikes (Risk Assessment)

Two technical spikes are flagged as candidates to de-risk implementation before planning:

1. **Cross-Stack `isolated` Constraint & Schema Bundle Spike**
   - *Goal:* Verify lowering `isolated` → `"x-darkmatter-isolated": true` across all five `simplified-schema` property forms (primitives, arrays, inline/block `$constraints`, imported/union references such as `steps(required; isolated)@./sequence.yaml`, and multi-arm YAML-list property unions), wire `EffectiveSchema::isolated_keys()` across all three `inherited_child_options` sites in `darkmatter` transclusion (`::file` / `as_markdown()`) and preflight, and verify the 4 bundles regenerated by `just shipped-schemas` preserve existing `::file` partials.
2. **Unified Passive `get_default_model(provider: Provider)` Spike**
   - *Goal (5-point verification plan):*
     1. **Provider config audit (all 10 registered providers):** Audit native local config formats and precedence rules for `claude`, `codex`, `gemini`, `goose`, `kimi`, `opencode`, `qwen`, `kilo`, `pi`, and `antigravity`.
     2. **Cross-OS config-root discovery (all 4 OSes):** Verify path discovery across macOS, Linux, native Windows, and WSL2 honoring `DocumentEnvironment`, `HomeBaseline`, XDG, and `%APPDATA%`.
     3. **Strict passivity & `< 10ms` performance budget:** Confirm zero subprocesses and zero network I/O, benchmark per-provider read latency under `< 10ms`, and validate `(Provider, config_root)` per-invocation memoization.
     4. **Overlay & argv-forwarding behavior:** Verify detection of shadow-home / MCP overlays hiding a provider's native config root to ensure tier-6 discovered defaults forward `--model` to child processes when and only when required (overlay hidden or `model_required_in_non_tty`).
     5. **Malformed-config resilience:** Verify deduplicated `config.provider_default_unreadable` (`info`) emission, `"default"` fallback, clean CI `--dry-run` exit `0` for non-OpenCode providers, and `composition.model_required` failure for non-TTY OpenCode.
