---
kind: feature
created: 2026-11-18
status: finalized-spec
area: claudine
depends-on:
    - 2026-11-22-better-errors-and-dry-run
related:
    - 2026-11-22-better-errors-and-dry-run
    - 2026-07-02-inline-sections
references:
    spike-isolated-schema.md: >-
        Cross-stack verification of the Simplified Schema `isolated` constraint across all five property forms, sticky-union `EffectiveSchema::isolated_keys()` plumbing across Darkmatter's four child-inheritance sites, host-trigger conflict avoidance, and `just shipped-schemas` regeneration.
    spike-default-model.md: >-
        Verified 10-provider, 4-OS passive config lookup matrix for `get_default_model(provider)`, including `pi`/`kilo`/`kimi` metadata reconciliation, `OverlayPlan` Tier-6 forwarding predicates, cache-key/deduplication design, and `< 10ms` malformed-config resilience.
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed-by: string -> the agent and model used in the spec review
    reviewed-on: date -> the date the spec was reviewed
    review-iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented-by: string -> the agent who implemented the plan
clarified: true
clarified-by: antigravity/gemini-4-neon
reviewed: true
reviewed-by: antigravity/gemini-4-neon
reviewed-on: 2026-11-23
review-iterations: 0
review-note: "the clarification process served as a review"
needs-rulings: false
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
   - Introduce a property-level `isolated` metadata constraint in `simplified-schema/lib` lowering to `"x-darkmatter-isolated": true` at the top level of each property schema (and do **not** add a new `null-is-undefined` constraint).
   - Roll out `isolated` across `simplified-schema`, `darkmatter` (all four child-inheritance sites across `::file`, `as_markdown()`, preflight, and reference graph), and `claudine` sequences using sticky union semantics across baseline, matched triggers (extracted before pruning), and document `$schema`.
   - Ensure `isolated` blocks *implicit inheritance only* (owning-sequence root `fm` propagation and Darkmatter `::file` transclusion), never explicit `with:`.
   - Keep `agent` and `model` non-isolated across Claudine schemas, enforced by an L1 schema-parity test in `claudine-lib`.
5. **Unified Document Classification & Dynamic Target Handling.**
   - Consolidate document classification into one pure classifier in `claudine-lib` inspecting authored frontmatter only (with absent-equals-null semantics), deferring `sections:` recognition until `2026-07-02-inline-sections` lands.
   - Classify statically resolvable target paths upfront; handle dynamic `{{ expr }}` targets by emitting `dm.check.not_checked` (`info`) in `--dry-run`, deferring classification strictly to runtime resolution, and validating `agent` availability via ancestor/task/ordered `set:` or emitting `composition.agent_unverifiable_target` (`warning`).
6. **Unified CLI Provider/Model Flags, Recursive `agent` Pre-Flight & `get_default_model`.**
   - Unify CLI `--<provider>` and `--model` into Caller → Callee `fm` state (eliminating the parallel `explicit_provider` override channel), disambiguating `claudine sequence` strictly by file format (Markdown vs. YAML), and failing with `usage.invalid_argument` (exit `2`) when `--<provider>` + `agent=<val>` or `--model <val>` + `model=<val>` conflict on the same command.
   - Validate `agent` availability recursively at pre-flight across nested sequences, groups, `task:` files, and composition tasks before execution begins, with ordered `set: { agent: ... }` tracking, cycle detection, and TTY review vs. non-TTY/`--silent` fail-fast behavior.
   - Never require authors to specify `model`; apply a deterministic six-tier model precedence chain (without adding `CLAUDINE_MODEL`) backed by a unified, fast, passive `get_default_model(provider: Provider, ctx: &DefaultModelContext)` lookup across all 10 registered providers.
7. **Diagnostic Taxonomy & Three-Way `--dry-run` Exit Codes.**
   - Register additive `composition.*` and `config.*` diagnostic codes in `claudine/lib/src/diagnostics/registry.rs` following `2026-11-22-better-errors-and-dry-run`'s message standard.
   - Exit `0` for clean/info-only `--dry-run`, `1` for warnings-only `--dry-run`, and `2` for errors or CLI usage errors.

### Non-Functional & Cross-Stack Requirements

- **Package ownership boundaries:**
  - `simplified-schema/lib`: owns `isolated` constraint parsing, top-level `"x-darkmatter-isolated": true` lowering/hoisting, validation/narrowing metadata classification, and `PREPARATION_REVISION` bumping.
  - `darkmatter/lib`: owns `EffectiveSchema::isolated_keys()`, `darkmatter_baseline_isolated_keys()`, all four child-inheritance plumbing sites, expression call-binding / schema catalog parity, data-origin defaults layering, and safe conditional file-path provenance (so relative file paths authored in a default layer resolve relative to their authoring file without overwriting child-authored values).
  - `claudine/lib`: owns the unified pure document classifier, Caller → Callee layering, recursive `agent` pre-flight, six-tier model resolution, passive `get_default_model(provider, ctx)` across all 10 registered providers (with zero new crate dependencies), `OverlayPlan::hides_native_config()`, diagnostic registry additions, and the L1 schema-parity test.
  - `claudine/cli`: owns CLI `--<provider>` / `--model` flag wiring, conflict detection (`--<provider>` + `agent=<val>` and `--model <val>` + `model=<val>`), `--dry-run` exit-code mapping (`0`/`1`/`2`), `exec_prep` overlay-aware Tier-6 forwarding, and launch resolution.
- **Performance, passivity & zero new dependencies:** `get_default_model` performs strictly passive local file reads only (zero subprocesses, zero network, zero directory walks), requires zero new crate dependencies (`serde_json`, `biscuit_file::Json5`, `toml`, and `serde_yaml_ng` are already in `claudine/lib/Cargo.toml`), enforces a `1 MiB` (`MAX_CONFIG_FILE_BYTES`) file/env-value cap and a bounded 32-ancestor upward walk, targets `< 10ms` per provider, and is memoized per invocation on `InvocationContext` keyed by `(Provider, ConfigPlatform, home, project_cwd, repo_root, include_project_config, provider_env_fingerprint)`.
- **Cross-OS compatibility:** All behavior, including passive default-model lookup and config-root discovery via explicit `DefaultModelContext` (`HomeBaseline`, `DocumentEnvironment`, `ConfigPlatform`, XDG, `%APPDATA%`, `%ProgramData%` — with zero ambient `std::env::*`, `home_dir()`, or `canonicalize()` calls and Windows ASCII case-insensitive env lookup), must compile and work across macOS, Linux, native Windows, and WSL2.

---

## Design Decisions (Confirmed — Batches 1–3, Refined Post-Spikes)

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

#### 1. The `isolated` Schema Constraint (`simplified-schema/lib`)

- Add a new property-level metadata constraint `Constraint::Isolated` (`"isolated"`) in **`simplified-schema/lib`**, following the implementation pattern of `Constraint::Generated` (except `Isolated` never suppresses static `required` and is rejected on root `$constraints`).
- **Top-level hoisting invariant:** Lower `isolated` to `"x-darkmatter-isolated": true` (`DARKMATTER_ISOLATED_KEYWORD`) strictly at the **top level** of the property schema across all five property forms:
  1. primitives (including semantic `schema`, `null`, `any`/`unknown`, `literal`, `enum` — e.g., `string(isolated)`, `"$schema": "schema(isolated)"`, `kind: "literal(sequence; required; isolated)"`)
  2. arrays (by convention placed after `[]` — e.g., `string[](isolated)`, `policy[](min(1); isolated)@...` — which works uniformly on built-ins, non-union imports, and union imports)
  3. inline objects (`{ ... }(isolated)`) and block objects via `$constraints: { isolated: true }` (round-tripping through `schema/export.rs`)
  4. imported and union references (e.g., `steps(required; isolated)@./sequence.yaml`, `loop(isolated)@./lifecycle-events.yaml`)
  5. multi-arm YAML-list property unions
  - Specifically: hoist `"x-darkmatter-isolated": true` onto outer optional `anyOf` wrappers in `atom_to_schema` and onto outer union objects in `union_property_to_schema`, and strip per-arm `"x-darkmatter-isolated"` in `union_arm_fragments` so annotating one arm vs. every arm lowers identically.
- **Verified consumer wiring points (from [spike-isolated-schema.md](./spike-isolated-schema.md)):**
  - `simplified-schema/lib`: `grammar.rs::validate_semantic_constraints` (so `"$schema": "schema(isolated)"` and `"$state": "schema(isolated)"` parse cleanly); `resolve.rs` (`is_union_marker`, `is_flag` for deduplication when written on both library type and use site); `resolve/recursive.rs` (`is_use_site_marker`, `is_arm_property_marker`); `convert.rs` (`is_reference_annotation`, `value_constraints` filtering, hoisting/stripping); `validate.rs` (`NON_ASSERTING_KEYWORDS` so `"x-darkmatter-isolated"` beside `$ref` / `anyOf` never triggers spurious `allOf` wrapping); `narrowing.rs` (`has_ordinary_constraints`, `type_identity`). `triggers/grammar.rs::is_match_safe_constraint` intentionally excludes `Isolated`.
  - `darkmatter/lib`: `call_binding/translate.rs` (`is_annotation`, `inline_object`, `literal`) so expression call-binding accepts isolated inline objects and literals (`guard_settings`, `kind: literal(sequence; required; isolated)`); `darkmatter/lib/src/markdown/schemas/about.rs` descriptor catalogs and all **4 closed parity tests** (`type_accepted_constraints_match_lowering`, `block_constraint_keys_match_the_lowering`, `constraint_set_matches_descriptor_set`, `match_safe_constraint_descriptor_set_matches_implementation`).
- **Schema bundle revision:** Bump `PREPARATION_REVISION` (`4 → 5`) in `simplified-schema/lib/src/shipped/registry.rs` before running root `just shipped-schemas` so stale downstream artifacts are rejected.

#### 2. Sticky-Union Plumbing & Full Cross-Stack Rollout (All 4 Darkmatter Sites)

`isolated` is wired end-to-end across all three layers in this specification:

1. **`simplified-schema`**: parses and lowers `isolated` → top-level `"x-darkmatter-isolated": true`.
2. **`darkmatter` sticky-union assembly (`darkmatter/lib/src/markdown/schemas/mod.rs`)**:
   - `EffectiveSchema::isolated_keys()` returns the **sticky union** of `"x-darkmatter-isolated": true` keys across the baseline schema, all matched triggers, and the document's own `$schema` (so a user `$schema` can add custom isolated keys without accidentally un-isolating framework controls).
   - **Three trigger-discard traps avoided by design:** Reading the merged `EffectiveSchema.json_schema` alone would lose isolated keys due to (a) whole-property replacement in `merge_baseline` / `merge_around_union`, (b) template trigger pruning via `pending.without_properties(&discarded)` in `resolve_trigger_layers`, and (c) registered-host `checked_layer` pruning in `registered_trigger_layers`. To preserve sticky-union invariants by construction, extract `isolated_keys` onto `TriggerLayer` **before** pruning or merging (using `isolated_keys_in_json` on unpruned payloads and `isolated_keys_in_root` on unresolved `PendingSchema` templates).
   - **Precompiled baseline & fallback safety:** Precompute `darkmatter_baseline_isolated_keys()` once per process via `OnceLock<Arc<BTreeSet<String>>>` from the shipped `fm.yaml` artifact (giving baseline-only documents an O(1) `Arc::clone` fast path). Because `ComposeOptions::new()` disables the baseline schema by default (`baseline_schema: None`), `PreparedSchemas::isolated_keys_for(source)` **always unions** the resolved/best-effort layer set with `darkmatter_baseline_isolated_keys()`, keeping `fm.yaml` controls isolated even for baseline-free library callers and broken or deferred `$schema` fallbacks (while `EffectiveSchema::isolated_keys()` stays pure to its own configured layers).
   - **All 4 Darkmatter child-inheritance sites** replace the hardcoded 3-key `prologue`/`epilogue`/`ctx` list by passing `isolated_keys: &BTreeSet<String>` through `inherited_child_options` / `markdown_child_options`:
     1. `darkmatter/lib/src/markdown/compose/transclusion/engine.rs` (`render_markdown_transclusion`, computed once per transclusion phase in `pipeline/phases.rs`),
     2. `darkmatter/lib/src/markdown/compose/nested.rs` (`as_markdown()`, stored on `Scope::caller_state` via `align_state` in `pipeline/mod.rs`),
     3. `darkmatter/lib/src/markdown/compose/preflight/collect.rs` (body `::file`, `prologue`/`epilogue`, and `as_markdown()` sub-sites), and
     4. `darkmatter/lib/src/markdown/reference/graph.rs` (`inclusion_child_options` across body `::file`, `prologue`, and `epilogue`).
     - Sites 3 and 4 receive `isolated_keys` via `ComposeReport.isolated_keys: Option<Arc<BTreeSet<String>>>` (populated only on inline-pre-only passes where `BlockTransclusion` is disabled, matching `body_data` so normal reports keep `None` and `ComposeReport::PartialEq` tests stay green).
     - **Zero `ComposeOptions` cache-hash changes needed:** `inherited_child_options` writes the already-filtered map into `child_request.external_state`, which `ComposeOptions::classify_options` already hashes into `options_hash(&child_request)`.
3. **`claudine`**: sequence value propagation consults `EffectiveSchema::isolated_keys()` **unioned with `darkmatter_baseline_isolated_keys()`** to decide which owning-sequence `fm` keys are excluded from the defined-value implicit inheritance channel (without restricting explicit `with:`).

#### 3. Schema Layering & `default-values` Propagation

Two host-registration and YAML-root subtleties verified in [spike-isolated-schema.md](./spike-isolated-schema.md) govern exact schema placement:

- **Host-trigger conflict avoidance (`composition.yaml` + `sequence-document.yaml`):** Every Markdown sequence document matches **both** `composition.yaml` (which `$extends: common-attributes@./shared.yaml`) and `sequence-document.yaml`. Because `shared.yaml` already declares `title`, `description`, and `metadata` (via `linking.yaml`), redeclaring them with `isolated` in `sequence-document.yaml` while leaving them non-isolated in `shared.yaml` would fail registered-host merge checking with `dm.host.trigger_conflict`. Therefore, `title`, `description`, and `metadata` (along with `name` and all other linking/document/harness controls) are marked `isolated` directly in `shared.yaml` / `linking.yaml`, and `sequence-document.yaml` adds only the three remaining `fm.yaml` keys absent from `shared.yaml` (`replace`, `tags`, `draft`) plus sequence controls (`kind`, `sequence`, `template`, `$state`, `default-values`, `fail_fast`). Plain Darkmatter documents (validated by `fm.yaml` alone) keep all six (`replace`, `title`, `description`, `tags`, `draft`, `metadata`) non-isolated for `::file` partials.
- **YAML sequence roots (`formal-sequence-file.yaml`):** `EffectiveSchema::for_yaml_document` validates a `kind: sequence` YAML file against `formal-sequence-file.yaml` alone — without `fm.yaml` or `composition.yaml`. Therefore, `formal-sequence-file.yaml` extends `common-attributes@./shared.yaml` (or declares the sequence-valid control subset as `isolated`) **and** Claudine sequence propagation unions `effective.isolated_keys()` with `darkmatter_baseline_isolated_keys()`, ensuring neither Claudine lifecycle/run controls nor Darkmatter controls authored on a YAML sequence root leak into direct prompt tasks.

| Schema File | Marked `isolated` | Explicitly Non-Isolated |
| ----------- | ----------------- | ----------------------- |
| `darkmatter/schemas/partials/fm.yaml` | 16 document-processing controls: `$schema`, `last_updated`, `content_policy`, `hash`, `style`, `change`, `ctx`, `prologue`, `epilogue`, `ignore_invalid`, `interpolate_code_blocks`, and `md` lifecycle events (`initialize`, `composed`, `failed`, `cancelled`, `finalize`) | 6 keys preserved non-isolated for plain Darkmatter `::file` partial compatibility: `replace`, `title`, `description`, `tags`, `draft`, `metadata` |
| `claudine/registration/shared.yaml` (`common-attributes`), `document.yaml`, `linking.yaml`, `composition.yaml`, `inline-composition.yaml` | All Claudine, inline-composition (`prompt`, `sections`), document (`hash`, `last_updated`), linking (`name`, `allowed-tools`, `tools`, `skills`, `license`, `compatibility`, `metadata`, `user-invocable`, `disable-model-invocation`, `argument-hint`, `max_turns`), metadata (`title`, `description`), and harness/lifecycle control keys *(marking library types in `document.yaml`/`linking.yaml` propagates `isolated` automatically to all importers; `name` blocks implicit inheritance while explicit `with: { name: ... }` still works)* | `agent`, `model` (enforced by an L1 schema-parity test in `claudine-lib`) |
| `claudine/registration/sequence-document.yaml` | The 3 `fm.yaml` keys absent from `shared.yaml` (`replace`, `tags`, `draft`) plus sequence controls (`kind`, `sequence`, `template`, `$state`, `default-values`, `fail_fast`); does **not** redeclare `title`, `description`, or `metadata` (already isolated via `composition.yaml`/`shared.yaml`) | User-authored domain frontmatter keys, `agent`, `model` |
| `claudine/registration/formal-sequence-file.yaml` | Extends `common-attributes@./shared.yaml` (or declares the sequence-valid control subset as `isolated`) plus `kind`, `tags`, `draft`, `replace`, `template`, `$state`, `default-values`, `fail_fast`, `sequence` *(and sequence propagation unions `effective.isolated_keys()` with `darkmatter_baseline_isolated_keys()`)* | User-authored domain frontmatter keys, `agent`, `model` |

Running root `just shipped-schemas` (after bumping `PREPARATION_REVISION` in `simplified-schema/lib/src/shipped/registry.rs`) regenerates all four checked-in precompiled schema artifacts atomically (`simplified-schema/lib/generated/shipped.json`, `darkmatter/lib/generated/shipped.json`, `claudine/lib/generated/shipped.json`, and `claudine/lib/generated/globals.lifecycle.json`).

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
| 2 | **Provider-specific model env var (`<PROVIDER>_MODEL`)** | Checked in each provider's declared `model_env_vars` order — 7 providers (`CLAUDE_MODEL`/`ANTHROPIC_MODEL`, `CODEX_MODEL`/`OPENAI_MODEL`, `GEMINI_MODEL`, `GOOSE_MODEL`, `KIMI_MODEL`, `OPENCODE_MODEL`, `QWEN_MODEL`); `kilo`, `pi`, and `antigravity` declare `[]` *(fixing two live generator bugs: `pi` currently leaks `PI_OFFLINE` and `kilo` leaks `KILO_PROVIDER`; pin `model_env_vars: []` in `claudine/docs/providers/overrides/{pi,kilo}.yaml` and regenerate via `cargo run -p claudine-gen -- generate`)* |
| 3 | **Generic `MODEL` env var** | Sole generic env var — **do NOT add `CLAUDINE_MODEL`** |
| 4 | **Document's authored `fm.model` (and owning sequence's authored or runtime-`set` `fm.model` for direct prompt tasks)** | CLI `--model` / `model=` on direct or Markdown runs already sits in Tier 1; for direct prompt tasks, the owning sequence's authored or runtime-`set` `fm.model` outranks the child document's authored `fm.model` (Rank 2 vs. Rank 3 in Decision 2.2) |
| 5 | **`default-values.model`** | `task default-values.model` > `sequence default-values.model` > inherited parent `default-values.model` (including CLI `--model` on YAML sequences) |
| 6 | **Provider's configured default model** | Resolved via `get_default_model(provider, ctx)` in `claudine-lib` (full 10-provider matrix in [spike-default-model.md](./spike-default-model.md)) |

#### 4. Unified `get_default_model(provider: Provider, ctx: &DefaultModelContext)` Operational Contract (`claudine-lib`)

One fast, passive, provider-neutral config lookup function in `claudine-lib` (`claudine/lib/src/provider/default_model.rs`) serves **all 10 registered providers** (`claude`, `codex`, `gemini`, `goose`, `kimi`, `opencode`, `qwen`, `kilo`, `pi`, `antigravity`), retiring `OpencodeWrapper::configured_default_model` and wiring `claudine/lib/src/composition/select.rs` to Tier 6 before failing with `composition.model_required`:

- **All 10 providers passively readable with zero new crate dependencies:** Every registered provider exposes a passively readable local config parsed with crates already in `claudine/lib/Cargo.toml` (`serde_json` for JSON, `biscuit_file::Json5` for JSONC, `toml` for TOML, `serde_yaml_ng` for YAML — full precedence and extraction matrix in [spike-default-model.md](./spike-default-model.md)):
  - **Live metadata fixes (`claudine-gen`):** Pin `model_env_vars: []` in `claudine/docs/providers/overrides/pi.yaml` (removing `PI_OFFLINE`) and `claudine/docs/providers/overrides/kilo.yaml` (removing `KILO_PROVIDER`), then run `cargo run -p claudine-gen -- generate`.
  - **Kimi Code two-generation probe:** Check exclusive `$KIMI_CODE_HOME/config.toml` (if set and absolute) → `~/.kimi-code/config.toml` (Kimi Code 2.x) → legacy `$KIMI_SHARE_DIR` / `~/.kimi/config.{toml,json}` (1.x) **only** when `KIMI_CODE_HOME` is unset.
  - **Goose XDG-primary across all 4 OSes:** Check exclusive `$GOOSE_PATH_ROOT/config/config.yaml` (if set and absolute) → XDG primary (`$XDG_CONFIG_HOME/goose/config.yaml` else `<home>/.config/goose/config.yaml` on all 4 OSes) → legacy OS fallbacks (`%APPDATA%\Block\goose\config\config.yaml` on Windows; `~/Library/Application Support/Block/goose/config/config.yaml` on macOS) → `GOOSE_ADDITIONAL_CONFIG_FILES` → system config.
  - **Pi compound merge & `--no-approve` awareness:** Merge `defaultProvider` and `defaultModel` across project `.pi/settings.json` (only when `include_project_config` is `true`, skipping it under Pi's `--no-approve` structured mode) and user `$PI_CODING_AGENT_DIR/settings.json` / `~/.pi/agent/settings.json`; `Antigravity` reads `<home>/.gemini/antigravity-cli/settings.json`.
- **Explicit 4-OS context & Windows env-lookup fix (zero ambient reads):**
  - Accepts explicit `DefaultModelContext { home, env, cwd, repo_root, platform: ConfigPlatform, include_project_config }` — zero `std::env::*`, zero `home_dir()`, zero `canonicalize()` (keeping `context_construction_guard` and `path_lookup_guard` green with zero new allowlist entries).
  - Adds `ConfigPlatform`-aware Windows ASCII case-insensitive env lookup (`AppData`, `ProgramData`, `UserProfile`, `XDG_CONFIG_HOME`) to prevent misses from `DocumentEnvironment::get`'s exact-case `HashMap::get` on native Windows.
  - Constructs paths component-by-component via `.join()`, validates platform-appropriate absoluteness on path env vars, and treats `ErrorKind::NotFound` and `ErrorKind::NotADirectory` as absent candidates.
- **Latency budget, read caps & memoization:**
  - Strictly passive local file reads only (zero subprocesses, zero network, zero directory walks), targeting `< 10ms` per provider.
  - Enforces a `1 MiB` (`MAX_CONFIG_FILE_BYTES`) cap on config files and inline JSON env vars (`OPENCODE_CONFIG_CONTENT`, `KILO_CONFIG_CONTENT`), rejects non-regular files (FIFOs/sockets/directories) before reading, and bounds upward project walks (`Codex`, `OpenCode`, `Kilo`) to `MAX_ANCESTOR_DEPTH = 32` ancestors stopped at `repo_root`.
  - Memoized per invocation on `InvocationContext` (`Mutex<HashMap<DefaultModelCacheKey, DefaultModelLookup>>`) keyed by `(Provider, ConfigPlatform, home, project_cwd, repo_root, include_project_config, provider_env_fingerprint)`, normalizing `project_cwd` and `repo_root` to `None` for user-only providers (`Goose`, `KimiCode`, `Antigravity`).
- **Context & visibility:** Populates `ctx.model` and `env.MODEL` with the discovered default model name (falling back to `"default"` when `get_default_model` resolves no model), and surfaces both the resolved model and its provenance (tier/source) in `--dry-run` output and execution logs.
- **Child `--model` argv forwarding & `exec_prep` ordering:**
  - **Tiers 1–5** (caller, env var, authored `fm.model`, or `default-values.model`): **always** forward `--model` (or provider equivalent) to the child provider process.
  - **Tier 6** (discovered provider default from `get_default_model`): forward **only** when `(non_interactive && info.model_required_in_non_tty)` (OpenCode) **or** `overlay_hides_native_config` is true, where `OverlayPlan::hides_native_config(&self) -> bool` (in `claudine/lib/src/provider_overlay/plan.rs`) is:
    ```rust
    self.requires_materialization()
        && provider_info(self.provider)
            .overlay_selector
            .is_some_and(|s| !s.additive && s.relocates(OverlayResourceClass::Config))
    ```
  - **Pipeline-ordering requirement (`claudine/cli/src/commands/exec_prep/mod.rs`):** Evaluate Tier-6 `--model` forwarding (`resolve_model_and_validate`) **after** `ensure_provider_overlay` (or explicitly account for pending `NativeRoot` MCP overlays) so Codex and Gemini composition runs that request MCP servers without `--repo` still detect the materialized overlay.
- **OpenCode / `model_required_in_non_tty`:** If a provider with `model_required_in_non_tty` still has no model after checking Tiers 1–5 and `get_default_model` (Tier 6), pre-flight reports `composition.model_required` (`error`).
- **Malformed local provider config fallthrough & deduplication:** `get_default_model` returns `DefaultModelLookup { outcome: DefaultModelOutcome, unreadable: Vec<UnreadableConfig> }`. Any malformed or unreadable higher-precedence candidate emits one deduplicated `config.provider_default_unreadable` **`info`** finding keyed per **`(Provider, DefaultModelSource)`** (exact file path or `$VAR`, tracked on `InvocationContext`) while **falling through** to any valid lower-precedence config; if no valid config resolves, non-OpenCode providers fall back to `"default"` (so third-party config quirks never break CI `--dry-run` exit `0`), while non-TTY OpenCode emits `composition.model_required` (`error`, exit `2`).

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
| `config.provider_default_unreadable` | `info` | Local provider config file or inline env value inspected by `get_default_model` is malformed, oversized (`> 1 MiB`), or unreadable; emitted once per `(Provider, DefaultModelSource)` (exact file path or `$VAR`) while falling through to any valid lower-precedence config, or falling back to `"default"` / non-TTY OpenCode `composition.model_required`. |

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
| `composition.model_required` | `error` | Non-TTY provider with `model_required_in_non_tty` (OpenCode) still has no model after Tiers 1–5 and `get_default_model` (Tier 6). |
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
   - *After:* `darkmatter/schemas/partials/fm.yaml` also isolates `$schema`, `hash`, `last_updated`, `change`, `style`, `content_policy`, `ignore_invalid`, `interpolate_code_blocks`, and `md` lifecycle events. `replace`, `title`, `description`, `tags`, `draft`, and `metadata` remain non-isolated in plain Darkmatter documents (`fm.yaml`) for `::file` partial compatibility; `title`, `description`, and `metadata` are isolated in Claudine's shared/linking layer, and `replace`, `tags`, and `draft` are isolated at the Claudine sequence layer.
5. **`--dry-run` warnings-only exit code changes from `2` to `1`.**
   - *Before (as designed in `2026-11-22-better-errors-and-dry-run`):* Both errors and warnings exited `2`.
   - *After:* Warnings with no errors exit `1`; errors (and CLI usage errors) exit `2`; clean / `info`-only exits `0`.

In-repo migration of the 21 affected research files is pre-authorized for the same implementation PR; see [Pre-Authorized Preparatory & Cross-Cutting Actions](#pre-authorized-preparatory--cross-cutting-actions) below.

---

## Pre-Authorized Preparatory & Cross-Cutting Actions

The following preparatory, migration, and documentation updates are explicitly pre-authorized as part of implementing this specification:

1. **Bump `PREPARATION_REVISION` and regenerate all 4 checked-in schema bundles atomically:**
   - Bump `PREPARATION_REVISION` (`4 → 5`) in `simplified-schema/lib/src/shipped/registry.rs`, then run root `just shipped-schemas` to regenerate:
     - `simplified-schema/lib/generated/shipped.json`
     - `darkmatter/lib/generated/shipped.json`
     - `claudine/lib/generated/shipped.json`
     - `claudine/lib/generated/globals.lifecycle.json`
2. **Fix `pi` and `kilo` `model_env_vars` metadata bugs via `claudine-gen`:**
   - Pin `model_env_vars: []` in `claudine/docs/providers/overrides/pi.yaml` (removing `PI_OFFLINE`) and `claudine/docs/providers/overrides/kilo.yaml` (removing `KILO_PROVIDER`), then run `cargo run -p claudine-gen -- generate` (and verify `cargo run -p claudine-gen -- check`).
3. **Migrate the 21 in-repo research files in the same implementation PR:**
   - The **20 coordinator `claudine/docs/research/*/_fleet.md` files** (19 `_provider-tasks.yaml` coordinators plus `claudine/docs/research/local_runners/_fleet.md`), and
   - **`claudine/docs/research/_TEMPLATE.md`**.
   - Migration scope: move root `agent:` / `model:` under `default-values:` so these coordinator fleets preserve their intended fallback semantics under the new Caller → Callee model. Leave `agent-cli`, `reasoning-level`, and `sandboxing/providers` untouched.
4. **Update canonical docs, caller skills, and repo skills:**
   - Canonical docs under `simplified-schema/docs/`, `darkmatter/docs/`, and `claudine/docs/` (covering `isolated`, Markdown vs. YAML/data sequence forms and the Document Classification Matrix, `compose:`/`inline-compose:`/`prompt:`/`sequence:` rules, Caller → Callee `fm` layering and `default-values:`, unified `--<provider>`/`--model` and six-tier model precedence, recursive `agent` pre-flight, and three-way `--dry-run` exit codes).
   - Caller skills under `claudine/docs/use-skills/use-claudine/{sequences,cli,composition}.md` (and `darkmatter/docs/use-skills/use-darkmatter/authoring.md` if needed), followed by running `just publish-skills`.
   - Repo skills under `.claude/skills/` reflecting the new schema constraint, classifier, and layering architecture.
5. **Leave `2026-11-22-better-errors-and-dry-run` untouched:**
   - Keep `2026-11-22-better-errors-and-dry-run` untouched as its reviewed snapshot; record the two supersessions (retained `prompt:` alongside `compose:`/`inline-compose:` and `--dry-run` `0`/`1`/`2` exit codes) exclusively in this specification.

---

## Acceptance Criteria & Definition of Done

Implementation is complete when all package-level acceptance criteria and cross-cutting gates below pass:

### `simplified-schema/lib`

- Parses property-level `Constraint::Isolated` (`"isolated"`) and lowers it to top-level `"x-darkmatter-isolated": true` across all five property forms (primitives including semantic `schema`, arrays, inline/block `$constraints`, imported/union references such as `steps(required; isolated)@./sequence.yaml`, and multi-arm YAML-list property unions), hoisting onto outer optional `anyOf` wrappers and outer union objects while stripping arm-level duplicates in `union_arm_fragments`.
- Wires `Constraint::Isolated` through `validate_semantic_constraints`, `is_union_marker`, `is_flag`, `resolve/recursive.rs` (`is_use_site_marker`, `is_arm_property_marker`), `convert.rs` (`is_reference_annotation`), `validate.rs` (`NON_ASSERTING_KEYWORDS`), `narrowing.rs`, `serialize.rs`, and `schema/export.rs`, and bumps `PREPARATION_REVISION` (`4 → 5`) in `simplified-schema/lib/src/shipped/registry.rs`.
- Does **not** introduce a `null-is-undefined` constraint, and does **not** add `Isolated` to `triggers/grammar.rs::is_match_safe_constraint`.
- L1 tests cover parsing, top-level hoisting/lowering across all five forms, round-trip YAML export, semantic `$schema`/`$state` properties, and invalid root `$constraints: { isolated: true }` rejection.

### `darkmatter/lib`

- `EffectiveSchema::isolated_keys()` returns the sticky union of `"x-darkmatter-isolated": true` keys across the baseline schema, all matched triggers (extracted onto `TriggerLayer` *before* `without_properties` / `checked_layer` / JSON merge pruning), and the document's own `$schema`, with an O(1) `Arc::clone` fast path for baseline-only documents.
- `darkmatter_baseline_isolated_keys()` is precomputed once via `OnceLock<Arc<BTreeSet<String>>>` from the shipped `fm.yaml` artifact, and `PreparedSchemas::isolated_keys_for` always unions with it so `ComposeOptions::new()` callers without baseline enabled and broken/deferred `$schema` fallbacks stay safe.
- All **4** Darkmatter child-inheritance sites (`transclusion/engine.rs`, `compose/nested.rs`, `compose/preflight/collect.rs`, and `reference/graph.rs::inclusion_child_options`) consume `isolated_keys` (threaded into sites 3 and 4 via `ComposeReport.isolated_keys` with zero `ComposeOptions` cache-hash changes), removing the hardcoded 3-key `prologue`/`epilogue`/`ctx` list.
- `call_binding/translate.rs` accepts `Constraint::Isolated`, and `darkmatter/lib/src/markdown/schemas/about.rs` updates descriptor catalogs and passes all 4 closed parity tests.
- `darkmatter/schemas/partials/fm.yaml` marks the 16 document-processing controls `isolated` while keeping `replace`, `title`, `description`, `tags`, `draft`, and `metadata` non-isolated so existing `::file` partials remain compatible.
- Provides data-origin defaults layering with safe conditional file-path provenance so relative file paths authored in a default layer resolve relative to the file that authored the default without overwriting child-authored values.
- L1 tests cover sticky-union preservation across all three trigger-discard traps, baseline-free `ComposeOptions::new()` isolation, all 4 inheritance sites, existing partial compatibility, and conditional file-path provenance.

### `claudine/lib`

- **Task grammar & options:** `compose:` and `inline-compose:` accept Markdown `<file-ref>` only; `prompt:` accepts plain `<file-ref>` only (rejecting `->` / `::`, rejecting explicit `operation:`/`flow:` when dispatching to a sequence, and silently ignoring inherited group `operation`/`flow` defaults); `sequence:` preserves full `<file-ref> [-> offset] [::op(args)]` grammar; `task:` remains immutable at call sites (`name` and `when` only); inline nested `sequence: [ ... ]` allows `default-values:` and rejects `with:`; zero-executable Markdown body steps reject `with:` and `default-values:`; group legs accept `with:` and `default-values:` identically to tasks; all forbidden options emit `composition.task_option_invalid`.
- **Caller → Callee & split boundary:** Six-rank precedence for direct prompt tasks and uniform `with:` merge semantics (including `with: { agent: null }` clearing pinned values and falling back to `default-values:`) are verified by unit and integration tests. Referenced formal child sequence documents (`is_formal_sequence == true`, nested or root) enforce the split boundary; informal data-file/offset/operator step lists (`is_formal_sequence == false`) keep the owning Markdown document's root `fm`. `"default-values"` is in `SEQUENCE_DECLARATION_KEYS` and rejected on `set:` writes (`composition.lifecycle_invalid`).
- **Schema layering & parity:** `claudine/registration/{shared.yaml,document.yaml,linking.yaml,composition.yaml,inline-composition.yaml}` mark Claudine/document/linking controls (including `title`, `description`, `metadata`, `name`) `isolated` while keeping `agent` and `model` non-isolated; `sequence-document.yaml` isolates `replace`, `tags`, `draft` and sequence controls without triggering `dm.host.trigger_conflict`; `formal-sequence-file.yaml` covers Claudine root controls (`$extends: common-attributes@./shared.yaml` or sequence-valid subset) and sequence propagation unions `effective.isolated_keys()` with `darkmatter_baseline_isolated_keys()`; enforced by an L1 schema-parity test in `claudine-lib`.
- **Unified pure document classifier:** Replaces the three fragmented classification checks (`mismatch.rs`, `preflight/mod.rs::load_prompt`, `commands/wrap/sequence/mod.rs`), inspects authored frontmatter only with absent-equals-null semantics, enforces every cell of the Decision 4.2 matrix (`composition.operation_mismatch`), leaves the `sections:` hook inactive until `2026-07-02-inline-sections`, and defers unresolved dynamic `{{ expr }}` targets to runtime (`dm.check.not_checked` in `--dry-run`).
- **Recursive `agent` pre-flight:** Traverses nested sequences, `prompt:`-dispatched sequences, inline `sequence: [ ... ]`, `group:` (inline/file/catalog), and `task:` files with cycle detection; credits only causally preceding `initialize` and earlier sequential `side_effect` `set: { agent: ... }` (`composition.agent_conditional_set`, `warning`); emits `composition.agent_unverifiable_target` (`warning`) for unresolved dynamic targets without an upstream agent; treats `--silent` as non-interactive fail-fast with full nested `Trail` reporting while preserving interactive TTY `review_sequence`.
- **Six-tier model precedence, provider metadata fixes & `get_default_model`:**
  - Implements the six-tier chain (without `CLAUDINE_MODEL`), honoring declared `model_env_vars` for Tier 2 and authored/runtime-`set` owning-sequence `fm.model` for Tier 4.
  - Fixes `pi` and `kilo` `model_env_vars` via `claudine/docs/providers/overrides/{pi,kilo}.yaml` + `cargo run -p claudine-gen -- generate` (verified with `cargo run -p claudine-gen -- check`).
  - `get_default_model(provider, ctx)` covers all 10 registered providers using strictly passive local reads with zero new crate dependencies, explicit `DefaultModelContext` (zero new `context_construction_guard` or `path_lookup_guard` allowlist entries), `ConfigPlatform`-aware Windows ASCII case-insensitive env lookup (`AppData`, `ProgramData`, `UserProfile`), `1 MiB` file/env cap, 32-ancestor walk cap, `< 10ms` per-provider latency, and `InvocationContext` memoization keyed by `(Provider, ConfigPlatform, home, project_cwd, repo_root, include_project_config, provider_env_fingerprint)`.
  - Implements `OverlayPlan::hides_native_config()`, wires `claudine/lib/src/composition/select.rs` to consult Tier 6 before failing with `composition.model_required`, returns `DefaultModelLookup` supporting malformed-config fallthrough and deduplicated `config.provider_default_unreadable` (`info`) per `(Provider, DefaultModelSource)`, and populates `ctx.model`/`env.MODEL` with provenance in `--dry-run`/logs.
  - L1 tests cover all 10 providers (including `pi` compound merge, `kimi` two-generation probe, and `goose` XDG-primary + legacy fallbacks), `DefaultModelContext` Windows case-insensitive env-lookup parity across all 3 `ConfigPlatform` variants, `> 1 MiB` cap, malformed fallthrough/deduplication, memoization reuse/invalidation, and `OverlayPlan::hides_native_config()`.
- **Diagnostics:** Registers all 6 new diagnostic codes and updates all reused codes in `claudine/lib/src/diagnostics/registry.rs` adhering to What/Where/Why/Hint and preserving locked `err.*` codes.

### `claudine/cli`

- Eliminates the parallel `explicit_provider` override channel and retires `OpencodeWrapper::configured_default_model`; wires `--<provider>` (`--claude`, `--codex`, …, `--provider <name>`) and `--model` into `fm.agent`/`fm.model` for `compose`, `inline-compose`, and Markdown `sequence`, and into `default-values.agent`/`default-values.model` for YAML/data `sequence`.
- Evaluates Tier-6 `--model` forwarding (`OverlayPlan::hides_native_config()` / `model_required_in_non_tty`) in `exec_prep/mod.rs` **after** `ensure_provider_overlay` (or accounting for pending `NativeRoot` MCP overlays for Codex/Gemini), verified by an `exec_prep` overlay-ordering test.
- Rejects both `--<provider>` + `agent=<val>` and `--model <val>` + `model=<val>` conflicts on the same command with `usage.invalid_argument` (exit `2`).
- Maps `--dry-run` exit status to `0` (clean or `info`-only), `1` (warnings with no errors — including `composition.agent_conditional_set` and `composition.agent_unverifiable_target`), and `2` (any `error` or CLI usage error).
- L1/L2 CLI tests verify flag wiring, both conflict errors, `exec_prep` overlay-ordered Tier-6 forwarding, `--dry-run` `0`/`1`/`2` exit codes, `--silent` fail-fast, and model provenance rendering.

### Cross-Cutting Definition of Done

- `PREPARATION_REVISION` is bumped (`4 → 5`) in `simplified-schema/lib/src/shipped/registry.rs`, all 4 checked-in schema bundles (`simplified-schema/lib/generated/shipped.json`, `darkmatter/lib/generated/shipped.json`, `claudine/lib/generated/shipped.json`, `claudine/lib/generated/globals.lifecycle.json`) are freshly regenerated via root `just shipped-schemas` with zero uncommitted drift, and `cargo run -p claudine-gen -- check` passes after regenerating `pi` and `kilo` provider data.
- All canonical docs (`simplified-schema/docs/`, `darkmatter/docs/`, `claudine/docs/`), caller skills (`claudine/docs/use-skills/use-claudine/{sequences,cli,composition}.md` and `darkmatter/docs/use-skills/use-darkmatter/authoring.md` if needed, published with `just publish-skills`), and repo skills (`.claude/skills/`) are updated with zero drift against shipped behavior.
- All 21 migrated research files (20 coordinator `claudine/docs/research/*/_fleet.md` files + `claudine/docs/research/_TEMPLATE.md`) pass `claudine sequence --dry-run` cleanly, and `agent-cli`, `reasoning-level`, and `sandboxing/providers` remain untouched and valid.
- `just test` and `just lint` pass across `simplified-schema`, `darkmatter`, and `claudine` on all supported OS environments.

---

## Completed Pre-Implementation Spikes

Both pre-implementation risk-reduction spikes are complete and their verified findings are incorporated into the design decisions and acceptance criteria above:

1. **Spike 1 — Cross-Stack `isolated` Constraint, Sticky-Union Plumbing & Schema Bundles** ([spike-isolated-schema.md](./spike-isolated-schema.md))
   - Verified top-level `"x-darkmatter-isolated": true` lowering/hoisting across all five Simplified Schema property forms, inventoried every downstream consumer (`validate_semantic_constraints`, `resolve/recursive.rs`, `NON_ASSERTING_KEYWORDS`, `narrowing.rs`, `call_binding/translate.rs`, and `about.rs`'s 4 closed parity tests), and confirmed bumping `PREPARATION_REVISION` (`4 → 5`) before `just shipped-schemas`.
   - Uncovered and resolved three trigger-discard traps in `darkmatter/lib/src/markdown/schemas/mod.rs`, `ComposeOptions::new()`'s baseline-free default via `darkmatter_baseline_isolated_keys()`, all **4** Darkmatter child-inheritance sites via `ComposeReport.isolated_keys` with zero `ComposeOptions` cache-hash changes, the `composition.yaml` + `sequence-document.yaml` host-trigger merge conflict on `title`/`description`/`metadata`, and YAML sequence root isolation (`formal-sequence-file.yaml`).
2. **Spike 2 — Unified Passive `get_default_model(provider: Provider)` Across All 10 Providers & 4 OSes** ([spike-default-model.md](./spike-default-model.md))
   - Verified passively readable local config paths, formats, and precedence rules for all 10 registered providers with zero new crate dependencies, surfacing live `model_env_vars` generator bugs for `pi` (`PI_OFFLINE`) and `kilo` (`KILO_PROVIDER`), Kimi Code's two-generation `~/.kimi-code` → `~/.kimi` probe, and Goose's XDG-primary + OS-legacy fallback paths.
   - Specified explicit `DefaultModelContext` with a `ConfigPlatform`-aware Windows ASCII case-insensitive env-lookup fix, the `OverlayPlan::hides_native_config()` predicate and `exec_prep/mod.rs` overlay-ordering fix, the refined `InvocationContext` memoization key, the `1 MiB` / 32-ancestor `< 10ms` caps, and `DefaultModelLookup` malformed-config fallthrough with `(Provider, DefaultModelSource)` deduplication.
