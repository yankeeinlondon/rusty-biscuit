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

1. **Ambiguous task intent.** `prompt:` hides whether the author intends a standard `compose`, an `inline-compose`, or a child `sequence` run, preventing static validation when a task points at the wrong kind of document.
2. **No distinction between overriding values and fallback defaults.** A single `compose` or `inline-compose` document must resolve an `agent` (and optionally a `model`), whereas a YAML sequence needs neither on its own root. Today, authors cannot say "use this value unless the task or child document sets its own," and `--<provider>` flags unconditionally override every task through a separate `explicit_provider` channel.
3. **Hardcoded frontmatter isolation.** `darkmatter` transclusion hardcodes a 3-key exclusion list (`prologue`, `epilogue`, `ctx`), while Claudine sequences need a schema-driven way to prevent document-processing and sequence control keys from leaking into child documents without blocking domain keys or `agent`/`model`.
4. **Late provider failures and opaque default models.** Sequences without a root-level agent fallback can fail mid-run on a later step or inside a nested YAML sequence, and dry-runs/logs often cannot tell the user which provider default model will actually execute.

This specification adds explicit `compose:` and `inline-compose:` task executables, extends `prompt:` and `sequence:` ergonomics, defines a schema-driven `isolated` constraint and Caller → Callee `fm` layering model, unifies `--<provider>` / `--model` handling, and introduces recursive `agent` pre-flight validation.

---

## Requirements

### Functional Requirements

1. **Prerequisite Foundation.** This feature builds directly on `2026-11-22-better-errors-and-dry-run` (implemented first), reusing its shared task schema in `claudine/schemas/sequence-types.yaml`, its `ExecutableField::noun` table, its `dm.reference.missing_document` diagnostic with nearby-file suggestions, and its `removed(...)` schema annotations.
2. **Explicit & Generic Task Executables.**
   - Preserve all six existing task executables (`prompt`, `shell`, `side_effect`, `group`, `task`, `sequence`) plus zero-executable body-default steps, and preserve `timeout` as a duration string.
   - Add two explicit task executables: `compose` and `inline-compose`.
   - Keep `prompt` as generic dispatch capable of executing `compose`, `inline-compose`, or a child `sequence`.
   - Keep `with:` and `default-values:` strictly as sibling task options beside the executable; `compose:` and `inline-compose:` accept file references only.
3. **Caller → Callee Layering in `fm`.**
   - Support two distinct value channels — **defined values** (override child-authored values) and **default values** (fill only what nothing else sets, including child-authored values) — within the `fm` workspace without colliding with read-only `seq`, `group`, and `loop` workspaces.
   - Preserve modular isolation across referenced child sequences (`sequence: <file-ref>`) so a parent sequence's own authored root `fm` does not leak across the sequence boundary, while `with:`, CLI `key=value` / runtime `set`, and inherited `default-values:` (child-wins) cross cleanly.
   - Support `with:` and `default-values:` on group legs identically to tasks; support `default-values:` (and forbid `with:`) on inline nested `sequence: [ ... ]` blocks.
4. **Schema-Driven Frontmatter Isolation (`isolated`).**
   - Introduce a property-level `isolated` metadata constraint in `simplified-schema/lib` lowering to `"x-darkmatter-isolated": true`.
   - Roll out `isolated` across `simplified-schema`, `darkmatter` (`::file` / `as_markdown()` transclusion and preflight), and `claudine` sequences using sticky union semantics across baseline, matched triggers, and document `$schema`.
   - Keep `agent` and `model` non-isolated across Claudine schemas, enforced by an L1 schema-parity test in `claudine-lib`.
   - Preserve null-fallback semantics aligned with `2026-11-14-absent-equals-null`: explicit `null` in a child document falls back to `default-values:`, whereas `with: { key: null }` writes literal `null` as an explicit override/clear.
5. **Unified CLI Provider Flags, Recursive `agent` Pre-Flight & Model Precedence.**
   - Unify CLI `--<provider>` into `fm` state (eliminating the parallel `explicit_provider` override channel in composition/sequence), disambiguating `claudine sequence` strictly by file format (Markdown vs. YAML).
   - Validate `agent` availability recursively at pre-flight across nested YAML sequences and composition tasks before execution begins, statically recognizing `initialize` `set:` actions with a clear warning about conditional flow risk.
   - Never require authors to specify `model`; apply a deterministic five-tier model precedence chain and surface each provider's actual configured default model in pre-check, `--dry-run`, and real runs where passively detectable.

### Non-Functional & Cross-Stack Requirements

- **Package ownership boundaries:**
  - `simplified-schema/lib`: owns `isolated` constraint parsing and `"x-darkmatter-isolated": true` lowering.
  - `darkmatter/lib`: owns `EffectiveSchema::isolated_keys()`, data-origin defaults layering, and safe conditional file-path provenance (so relative file paths authored in a default layer resolve relative to their authoring file without overwriting child-authored values).
  - `claudine/lib`: owns sequence/composition value layering, recursive `agent` pre-flight, model resolution/prediction, and the L1 schema-parity test.
  - `claudine/cli`: owns CLI `--<provider>` / `--model` flag wiring and launch resolution.
- **Cross-OS compatibility:** All behavior, including passive default-model lookup, must compile and work across macOS, Linux, native Windows, and WSL2.

---

## Design Decisions (Confirmed — Batch 1)

### Prerequisite Integration (`2026-11-22-better-errors-and-dry-run`)

`2026-11-22-better-errors-and-dry-run` lands before this specification. This specification builds directly on its infrastructure:

- **Shared task schema:** Extends `claudine/schemas/sequence-types.yaml` — preserving the open `task` shape with `$advise-unknown-keys` for sequence steps and the closed `task-file` / group-leg shape for group legs and standalone `kind: task` files. Executable mutual exclusivity and per-executable option rules remain enforced in the Rust parser.
- **Shared diagnostics machinery:** Extends `ExecutableField::noun`, reuses `dm.reference.missing_document` (with nearby-file suggestions) for missing file targets, and keeps `removed(...)` schema declarations for retired keys.
- **Explicit supersession note:** `2026-11-22-better-errors-and-dry-run` assumed in its reconciliation note that this spec would *replace* `prompt:` with `compose:`, `inline-compose:`, and `sequence:`. **This spec supersedes that assumption:** `prompt:` is **retained** as generic dispatch, and `compose:` and `inline-compose:` are added alongside it in `ExecutableField::noun`.

---

### Decision 1 — Task Executable Grammar & Syntax Shape

1. **Preserved executables and step shapes:**
   - All six existing task executables remain valid: `prompt`, `shell`, `side_effect`, `group`, `task`, and `sequence`.
   - Zero-executable body-default steps (in Markdown sequences) remain valid.
   - `timeout` remains a duration string (e.g., `"5m"`, `"30s"`). The original draft's `shell-command` and `timeout: number` sketch is dropped.
2. **New explicit executables:**
   - `compose`: explicitly composes the referenced document and sends the composed result as the prompt to an agent; rejects documents intended for `inline-compose` or `sequence` execution.
   - `inline-compose`: explicitly executes an `inline-compose` operation on the referenced document; rejects documents that are not inline-compose targets or that are sequences.
3. **Retained generic executable:**
   - `prompt`: kept as generic dispatch capable of running a `compose`, `inline-compose`, or child `sequence` target.
4. **File references only for `compose:` and `inline-compose:`:**
   - Both `compose:` and `inline-compose:` accept file references only in this specification (no bare-string inline prompt text).
   - An explicit, unambiguous inline-text form (such as `compose: { text: "..." }`) remains a possible future additive extension and is out of scope here.
5. **Sibling options only:**
   - `with:` and `default-values:` live strictly as sibling options on the task (or group leg) beside the executable key. The original draft's nested `compose: { file, with, default-values }` long-form sketch is dropped.

Illustrative task option shape in `claudine/schemas/sequence-types.yaml` (exclusivity and per-executable legality enforced in the Rust parser):

```yaml
# Conceptual shape on the shared task definition (open for sequence steps with
# $advise-unknown-keys; closed for group legs and standalone kind: task files)
name: "string -> task identifier, exposed as seq.name during execution"
timeout: "duration -> wall-clock limit before the task times out (e.g. 5m, 30s)"

# Exactly one executable (or zero executables on a Markdown body-default step):
compose: "file -> Markdown document executed via compose"
inline-compose: "file -> Markdown document executed via inline-compose"
sequence: "file | sequence-step[] -> referenced child sequence or inline nested sequence"
prompt: "file -> generic dispatch (compose, inline-compose, or child sequence)"
shell: "string | shell-spec -> shell command execution"
side_effect: "side-effect-spec -> state-mutating side effect"
group: "group-spec -> parallel or structured group of legs"
task: "file -> standalone kind: task file reference"

# Sibling value-passing options (per-executable rules enforced by parser):
with: "object -> defined overrides applied on top of caller fm state"
default-values: "object -> fallback values applied only where nothing higher sets the key"
```

---

### Decision 2 — Caller → Callee Model in `fm` & Child-Sequence Boundary

The read-only workspaces `seq`, `group`, and `loop` are separate from `fm` (frontmatter) and never compete with it. All value passing described here operates within `fm` under a consistent **Caller → Callee** model.

#### 1. CLI → Root Sequence

- CLI `key=value` / `--set` arguments initialize the root sequence's `fm` with highest priority, overriding authored root frontmatter.
- Runtime `set` writes (from `initialize`, `side_effect`, and lifecycle hooks) mutate that live `fm` state as the sequence executes.

#### 2. Sequence → Direct Prompt Task (`compose:`, `inline-compose:`, `prompt:`, and Group Legs)

The owning sequence acts as the caller. Precedence for any non-isolated `fm` key on the child document, **highest to lowest**:

| Rank | Source | Channel |
| ---- | ------ | ------- |
| 1 | Task's `with:` | Defined override (call-site) |
| 2 | Owning sequence's live non-isolated `fm` (`authored root fm < CLI key=value < runtime set`) | Defined override (caller state) |
| 3 | Child document's own authored `fm` | Callee authored value |
| 4 | Task's `default-values:` | Default fallback (call-site) |
| 5 | Owning sequence's `default-values:` (inheriting from parent `default-values:`, nearer wins) | Default fallback (inherited) |
| 6 | Child document's `$schema` defaults | Schema fallback |

#### 3. Sequence → Referenced Child Sequence (`sequence: <file-ref>`)

A referenced child sequence preserves **modular isolation** via a **split boundary**:

- **Does NOT cross:** The parent sequence's own authored root `fm` does **not** leak into the child sequence. A parent sequence's local frontmatter properties stay scoped to the parent and its direct prompt tasks.
- **DOES cross:**
  - **Explicit `with:`** on the calling task — applied as a defined override on top of the child sequence's root `fm`.
  - **CLI `key=value` and runtime `set`** state — propagates across the invocation.
  - **Inherited `default-values:`** — propagates recursively into the child sequence, where the child sequence's own `default-values:` win over inherited parent `default-values:` (nearer wins, preserving the 2026-11-22 Decision #1).

#### 4. Group Legs

- Group legs run child targets like tasks and accept `with:` and `default-values:` with identical semantics (preserving the 2026-11-22 Decision #2).
- Groups themselves do **not** gain a `default-values:` property in this specification.

#### 5. Illustrative Examples

```yaml
kind: sequence
default-values:
    agent: claude          # --codex on the CLI replaces this on a YAML sequence
    model: sonnet
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
    - name: subflow
      sequence: ./sub-sequence.yaml        # receives inherited default-values (sub-sequence's own
      with: { topic: auth }                # default-values win); only explicit `with:` + CLI/runtime set cross
```

A root `agent:` (outside `default-values:`) is a **defined** value, so it overrides direct child prompts unless a task's `with:` overrides it:

```yaml
kind: sequence
agent: claude              # every direct prompt task runs claude unless its task's `with:` says otherwise
sequence:
    - name: find
      compose: ./_find.md
```

#### 6. Package Ownership for Default Layering

- **`darkmatter-lib`** adds a small data-origin defaults capability and safe conditional file-path provenance so relative file paths authored in a default layer resolve from the file that authored the default, without clobbering child-authored values.
- **`claudine-lib`** owns Caller → Callee layering, recursive pre-flight, and model/provider prediction.
- **`claudine-cli`** owns CLI flag wiring (`--<provider>`, `--model`, `key=value`) and launch resolution.

---

### Decision 3 — Schema-Driven `isolated` Constraint, Null Fallback & Inline Nested Sequences

#### 1. The `isolated` Schema Constraint

- Add a new property-level metadata constraint `isolated` in **`simplified-schema/lib`**, following the implementation pattern of `Constraint::Generated`.
- Lower `isolated` to `"x-darkmatter-isolated": true` in JSON Schema across every property form:
  - primitives (e.g., `string(isolated)`)
  - arrays
  - inline and block objects via `$constraints`
  - imported and union references (e.g., `steps(required; isolated)@./sequence-types.yaml`)
  - multi-arm YAML-list property unions
- **Sticky union semantics:** `EffectiveSchema::isolated_keys()` in `darkmatter/lib` returns the **union** of all `"x-darkmatter-isolated": true` keys across the baseline schema, all matched triggers, and the document's own `$schema`. A user document's `$schema` can therefore mark additional custom keys as `isolated` without accidentally un-isolating host or framework controls.

#### 2. Full Cross-Stack Rollout

`isolated` is wired end-to-end across all three layers in this specification:

1. **`simplified-schema`**: parses and lowers `isolated` → `"x-darkmatter-isolated": true`.
2. **`darkmatter`**: `::file` / `as_markdown()` transclusion and preflight read `EffectiveSchema::isolated_keys()`, replacing the hardcoded 3-key `prologue`/`epilogue`/`ctx` exclusion list.
3. **`claudine`**: sequence value propagation consults `EffectiveSchema::isolated_keys()` to decide which owning-sequence `fm` keys are excluded from the defined-value override channel.

#### 3. Schema Layering & `default-values` Propagation

| Schema File | Marked `isolated` | Explicitly Non-Isolated |
| ----------- | ----------------- | ----------------------- |
| `darkmatter/schemas/partials/fm.yaml` | Document-processing controls: `prologue`, `epilogue`, `ctx`, `$schema`, `hash`, `last_updated`, `change`, `style`, `content_policy`, `ignore_invalid`, `interpolate_code_blocks`, `md` lifecycle events | `replace`, `title`, `description`, `tags`, `draft`, `metadata` (preserved non-isolated so existing `::file` partials stay compatible) |
| `claudine/schemas/sequence.yaml` | `replace`, `title`, `description`, `tags`, `draft`, `metadata`, plus all sequence controls: `kind`, `sequence`, `template`, `$state`, `default-values`, `fail_fast`, `sections` | User-authored domain frontmatter keys, `agent`, `model` |
| `claudine/schemas/claudine.yaml` (plus `document.yaml`, `linking.yaml`) | All Claudine, linking, and harness control keys | `agent`, `model` (enforced by an L1 schema-parity test in `claudine-lib`) |

> **Why `default-values` is marked `isolated`:** Marking `default-values` as `isolated` prevents the parent sequence's `default-values` object from leaking into child documents through the **defined-value override channel** (where it would wrongly overwrite child-authored values or appear as a raw override). Instead, `default-values` propagates recursively through the dedicated **child-wins defaults inheritance channel** defined in Decision 2.

`template:` remains unrelated to `default-values:`: `template:` supplies defaults for each step's own step-state (`seq.*`), not for the `fm` of documents executed by steps.

#### 4. Explicit `null` Fallback Semantics

- An explicit `null` value in a child document's frontmatter falls back to `default-values:` (consistent with `deep_merge` and `2026-11-14-absent-equals-null`).
- An explicit `with: { key: null }` on a calling task writes literal `null` as an intentional override/clear rather than falling back to defaults.

#### 5. Inline Nested Sequences (`sequence: [ ... ]`)

- An inline nested `sequence: [ ... ]` block **allows** `default-values:` (scoping fallback defaults to that sub-block, with nearer defaults winning over outer defaults).
- An inline nested `sequence: [ ... ]` block **forbids** `with:` (since it executes within the same sequence file's `fm` scope rather than crossing a file boundary).

---

### Decision 4 — CLI `--<provider>` / `--model`, Recursive Agent Pre-Flight & Model Priority

#### 1. Unifying CLI `--<provider>` into `fm` State

The parallel `explicit_provider` override channel in composition and sequence execution is eliminated; `--<provider>` (`--claude`, `--codex`, …, `--provider <name>`) writes directly into `fm` state:

| Command | Target Format | Where `--<provider>` Writes | Effect |
| ------- | ------------- | --------------------------- | ------ |
| `claudine compose` | Markdown | Target document's `fm.agent` | Same priority as CLI `agent=<provider>` (overrides authored `agent`) |
| `claudine inline-compose` | Markdown | Target document's `fm.agent` | Same priority as CLI `agent=<provider>` (overrides authored `agent`) |
| `claudine sequence` | **Markdown** | Root sequence's `fm.agent` | Defined value on the root sequence (overrides direct prompt tasks unless overridden by task `with:`) |
| `claudine sequence` | **YAML** | Root sequence's `default-values.agent` | Fallback default for tasks/descendants that do not specify an agent; task `with:`, root `agent:`, and child prompt `agent:` all win over it |

Disambiguation on `claudine sequence` is strictly by **file format** (Markdown vs. YAML).

#### 2. Recursive `agent` Pre-Flight Validation

To prevent sequences from running halfway and failing on a downstream task with no agent:

- `agent` is required for every runnable prompt execution (`compose`, `inline-compose`, and Markdown body-default steps). Resolution consults `default-values.agent` when no defined or document-authored `agent` is present, and prompts interactively when unresolved on a TTY.
- For a **YAML** sequence, `agent` is **not** required on the YAML sequence root itself (same type as on Markdown documents, just optional on YAML roots).
- Because `default-values` inherits recursively into child sequences, setting `default-values.agent` on any ancestor sequence satisfies all descendants.
- When **no** ancestor provides `default-values.agent` (and no root defined `agent` covers direct prompt tasks), pre-flight validates every composition task (`compose`, `inline-compose`, `sequence`, `prompt`):
  1. The task explicitly supplies `agent` via `with:` or `default-values:`, **OR**
  2. The referenced child file defines `agent` or `default-values.agent` in its frontmatter, **OR**
  3. The child document's `initialize` lifecycle event contains a `set:` action targeting `agent` or `default-values.agent` — detected statically (ignoring conditional/flow-control branching). In this case, pre-flight **passes with a clear warning** explaining that conditional logic in `initialize` might skip that `set:` at runtime, letting the caller decide whether to fix it or proceed, **OR**
  4. The child is a YAML sequence without root `agent` or `default-values.agent`, in which case pre-flight **recurses into that child YAML sequence's own tasks** applying rules 1–4.

#### 3. Model Resolution Rules & Default-Model Visibility

- `model` is **never mandatory to author**; when unset, the selected provider's configured default model is used.
- Like `agent`, `model` is **not `isolated`**.
- **Model precedence, highest to lowest:**
  1. CLI `--model <model>` / `model` passed via `with:` in the parent sequence
  2. Model environment variables: provider-specific variables (`CLAUDE_MODEL`/`ANTHROPIC_MODEL`, `CODEX_MODEL`/`OPENAI_MODEL`, `GEMINI_MODEL`, `GOOSE_MODEL`, `KIMI_MODEL`, `OPENCODE_MODEL`, `QWEN_MODEL` in each provider's declared order), then generic `MODEL` *(see Open Questions regarding `CLAUDINE_MODEL`)*
  3. `fm.model` set in the document (and owning sequence's defined `fm.model` for direct prompt tasks)
  4. `fm.default-values.model` (`task default-values.model` > `sequence default-values.model` > inherited parent `default-values.model`)
  5. Selected provider's configured default model
- **Default-model visibility:** During pre-check, `--dry-run`, and real runs, Claudine should identify each provider's actual configured default model via fast passive cross-provider config lookup (falling back to `"default"` where a provider's default cannot be read passively), so logs, dry-runs, and real runs display the actual model that will run rather than an opaque blank.

---

## An Unspoken Linguistic Complication (Draft Context)

> *Preserved from the initial draft as context for Batch 2 clarification. Exact classification rules, YAML offset syntax, and error conditions below are not yet finalized — see [Open Questions](#open-questions-pending-clarification).*

We need to clear up one complication that may have gotten lost in the shuffle:

- Markdown files with no `sequence` property are a single document that will be executed _either_ via a `compose` or `inline-compose` operation. This is determined by whether the document's frontmatter has either a `prompt` or `sections` property:
  - if it _does_, then it will be executed as an `inline-compose` operation;
  - if it _does not_, then it will be executed with the document's body/prose as the prompt during a `compose` operation.
- What about a Markdown file **with** a `sequence` property?
  - This is the area of confusion where we do not yet have clear terminology in docs.
  - It is a **sequence** operation, with two variants:
    - **inline sequences:** if the document has a `prompt` or `sections` property, each step in the sequence is intended to operate on the prose/body of this document as its primary mutation;
    - **compose sequences:** if the document _does not_ have a `prompt` or `sections` property, the prompt for each step in the sequence uses the document's prose/body as the prompt.

In most ways a sequence defined by a YAML document is the clearest variant:

- A YAML-defined sequence is neither an _inline sequence_ nor a _compose sequence_; it is a pure sequence that makes no assumptions about where the prompt comes from, instead declaring an ordered set of steps/tasks.
- Each step in a YAML sequence chooses an operation specifically (`compose`, `inline-compose`, `sequence`, `shell`, …) or generically (`prompt`):
  - When using `compose` or `inline-compose`, pointing to a Markdown file with a `sequence` property is an error.
  - When using `sequence`, the target can be either a YAML or Markdown file:
    - if it is a Markdown file, it must have a `sequence` property or it results in an error;
    - if it is a YAML file, it must be either an **explicit** sequence (e.g., `kind: sequence`) or an **informal** sequence (e.g., an array of strings or an array of objects with `name` defined).
    - Note: the informal syntax supports _offsetting_ into a YAML property *(draft noted `./foobar.yaml::data`; exact operator `->` vs `::` is pending Batch 2 confirmation)*.

This distinction is important context for the spec and must also be documented clearly under `claudine/docs/`.

---

## Breaking Changes & Migration

The following confirmed behavior changes differ from current behavior:

1. **CLI `--<provider>` on YAML sequences becomes a default, not a global override.**
   - *Before:* `--claude` forced every task in a sequence to run Claude, overriding task `with:` and child prompts' own `agent:`.
   - *After:* On a YAML sequence, `--<provider>` sets `default-values.agent`, so task `with:`, a root `agent:`, and a child prompt's own `agent:` all win over it. (On a Markdown sequence and on single-document `compose` / `inline-compose`, `--<provider>` sets `fm.agent`.)
2. **Root `agent:` on a sequence becomes a defined override for direct prompt tasks.**
   - *Before:* A root `agent:` acted as a fallback when a task's `with:` and child prompt named no agent, and the child prompt's own `agent:` beat it.
   - *After:* Root `agent:` is a non-isolated defined value, so it overrides direct child prompts' authored `agent:` (unless overridden by task `with:`). Authors who want fallback behavior move `agent:` under `default-values:`.
3. **Root `model:` follows the same non-isolated defined-value rule as `agent:`.**
   - Root `model:` acts as a defined value on direct prompt tasks (within the confirmed five-tier model precedence where CLI `--model` / `with.model` and model env vars still rank above `fm.model`); fallback defaults move under `default-values.model`.
4. **Schema-driven `isolated` replaces Darkmatter's hardcoded 3-key exclusion list.**
   - *Before:* `::file` / `as_markdown()` only isolated `prologue`, `epilogue`, and `ctx`.
   - *After:* `darkmatter/schemas/partials/fm.yaml` also isolates `$schema`, `hash`, `last_updated`, `change`, `style`, `content_policy`, `ignore_invalid`, `interpolate_code_blocks`, and `md` lifecycle events. `replace`, `title`, `description`, `tags`, `draft`, and `metadata` remain non-isolated in Darkmatter partials for compatibility and are isolated only at the Claudine sequence layer.

*(Exact migration guidance, deprecation messaging, and any breaking-change pre-authorizations remain open for Batch 2+.)*

---

## Diagnostics Alignment

This specification integrates directly with the diagnostic framework established in `2026-11-22-better-errors-and-dry-run`:

- **`ExecutableField::noun`:** Adds rows for `compose` and `inline-compose` while retaining `prompt` (superseding `2026-11-22-better-errors-and-dry-run`'s assumption that `prompt:` would be removed). `composition.no_executable` and reference construct phrasing automatically reflect all eight executables.
- **Missing file references:** `compose:`, `inline-compose:`, `sequence:`, and `prompt:` targets reuse `dm.reference.missing_document` and its nearby-file suggestion engine.
- **Unknown & removed keys:** Sequence steps continue to use `$advise-unknown-keys` and `removed(...)` via the shared `claudine/schemas/sequence-types.yaml` declaration.
- **Provider selection-chain reporting:** When a task has no resolvable provider, the diagnostic lists the updated defined and default links in the precedence order from Decision 2 and Decision 4, and the `selection_key_as_data` hint also recommends `default-values:` as a place for `agent` and `model`.
- **Static `initialize` `set:` warning:** When recursive `agent` pre-flight accepts a child document solely because its `initialize` lifecycle event contains a `set:` targeting `agent` or `default-values.agent`, pre-flight emits a structured warning noting that conditional/flow-control logic was not evaluated and could prevent the `set:` from executing at runtime.

*(Exact diagnostic codes for operation-mismatch errors and `--dry-run` rendering updates are tracked under Open Questions.)*

---

## Open Questions (Pending Clarification)

The following topics have **not** yet been clarified and must be resolved in Batch 2+ before finalizing this specification:

1. **Document Classification & Sequence Syntax Edge Cases**
   - How should `sections` classification behave before `2026-07-02-inline-sections` ships (e.g., recognize `sections` immediately in classification, guard it, or defer)?
   - Exact rules for YAML **explicit** (`kind: sequence`) vs. **informal** sequences, and confirming the offsetting operator syntax (existing `->` offset syntax vs. the draft's `::` sketch).
   - Exact semantics and validation boundaries for Markdown **inline sequences** vs. **compose sequences** when targeted by `compose:`, `inline-compose:`, `sequence:`, or `prompt:`.
   - Validation timing for dynamic `{{ expr }}` file targets in `compose:`, `inline-compose:`, `sequence:`, and `prompt:` (what pre-flight validates vs. defers to runtime, and how that interacts with recursive `agent` pre-flight).
2. **`prompt:` Generic Dispatch & Nested `sequence:` Nuances**
   - Exact dispatch behavior when `prompt:` targets a child sequence (Markdown or YAML, explicit or informal, bare or offset) vs. `sequence:`.
   - Any remaining nuances for nested `sequence:` invocations (e.g., cycle detection, `with:` key validation against child sequence schema, runtime `set` propagation semantics back vs. down).
3. **Model Environment Variable Naming & OpenCode Non-TTY Interaction**
   - The codebase currently checks provider-specific env vars (`CLAUDE_MODEL`/`ANTHROPIC_MODEL`, `CODEX_MODEL`/`OPENAI_MODEL`, `GEMINI_MODEL`, `GOOSE_MODEL`, `KIMI_MODEL`, `OPENCODE_MODEL`, `QWEN_MODEL`) followed by generic `MODEL`; there is no `CLAUDINE_MODEL` variable today. Confirm whether `MODEL` stays the sole generic env var or whether `CLAUDINE_MODEL` should be introduced.
   - How does "model is never mandatory to author" interact with providers whose catalog declares `model_required_in_non_tty` (currently OpenCode in `claudine/lib/src/composition/select.rs`) — does passive default-model lookup satisfy OpenCode when configured locally, and should non-TTY OpenCode without a configured default still fail with a model-required error?
4. **Error Taxonomy, `--dry-run`, Acceptance Criteria, Documentation & Migration**
   - Exact error codes, severities, and messages for operation mismatches (`compose:` targeting inline/sequence doc, `inline-compose:` targeting compose/sequence doc, `sequence:` targeting non-sequence doc, `with:` on inline nested `sequence: [...]`, invalid `default-values:` shape, etc.).
   - `--dry-run` output details for layered `with:` / `default-values:`, resolved default models, and `initialize` `set:` pre-flight warnings.
   - Testable acceptance criteria and Definition of Done across `simplified-schema`, `darkmatter`, and `claudine`.
   - Documentation updates required in `claudine/docs/` (including the Markdown/YAML sequence taxonomy) and `claudine/docs/use-skills/use-claudine/`.
   - Migration scope and any explicit pre-authorizations required for the breaking changes listed above.

---

## Candidate Spikes (Risk Assessment)

Two technical spikes are flagged as candidates to de-risk implementation before planning:

1. **Cross-Stack `isolated` Constraint & Schema Bundle Spike**
   - *Goal:* Verify lowering `isolated` → `"x-darkmatter-isolated": true` across all `simplified-schema` property forms (primitives, arrays, inline/block `$constraints`, imported/union references like `steps(required; isolated)@./sequence-types.yaml`, and multi-arm YAML-list property unions), wiring `EffectiveSchema::isolated_keys()` into `darkmatter` transclusion (`::file` / `as_markdown()`) and preflight, and regenerating `shipped.json` bundles without breaking existing `::file` partials.
2. **Fast Passive Cross-Provider Default-Model Config Lookup Spike**
   - *Goal:* Audit each supported provider's local config format and lookup paths across macOS, Linux, native Windows, and WSL2 to verify which providers can resolve their configured default model via fast, passive file reads (without spawning provider CLIs), measure latency, and define the `"default"` fallback contract for pre-check, `--dry-run`, and real runs.
