# Improved Sequences — Delta Plan

This document describes the work required to evolve Claudine's sequence functionality from its current state to the desired future state. The current state is documented in `docs/cli/sequence.md`; the future state is documented in `docs/topics/agent-flows/sequences.md`. The parameters feature described in `docs/topics/validations/parameters.md` is part of the target state.

Groups are still in flux and are explicitly excluded from this plan.

---

## 1. Step ID Generation

### Current State

Steps have a zero-based `index` and a `name` (either the string value or the `name` field from an object step). There is no guaranteed-unique `id` property. The `SequenceStep` struct (`composition/types.rs:220`) stores `index`, `name`, and `raw_state` only.

### Future State

Every step gets a unique `id` derived by dasherizing the `name`. If a dasherized name has already been used, an index suffix is appended (e.g., `one-1`, `one-2`). Users may optionally define `id` explicitly on every step in the sequence; if they do, all steps must have an explicit `id` and all must be unique.

### Work Required

- **`SequenceStep` struct** — add an `id: String` field.
- **`normalize_inline_list`** (`composition/sequence.rs:166`) — after extracting `name`, compute `id` by dasherizing it (`name.to_lowercase().replace(' ', "-").replace('_', "-")` etc.). Maintain a `HashSet<String>` of seen IDs to append `-1`, `-2` etc. on collisions. Validate the "all explicit or none" rule if any step provides `id` in its raw state.
- **Overlay** — add `id` to `SequenceStepOverlay` (or expose it through `state.id` since it will be in `raw_state`). Update `RESERVED_KEYS` and `as_set_overrides` if it becomes a top-level overlay variable.
- **Tests** — cover: unique IDs from duplicate names, explicit ID validation (all-or-nothing, uniqueness), dasherization edge cases.

---

## 2. Headless Sequences

### Current State

Every sequence execution requires a Markdown document as the composition source. The `sequence` CLI command (`cli/src/commands/sequence.rs`) always resolves a file reference to a Markdown file, then calls `resolve_sequence_plan` on it. The `SequencePlan` carries no concept of per-step prompt sources — all steps compose the same root document.

### Future State

A "headless" sequence is a YAML file pointed to directly (e.g., `claudine sequence @defn.yaml`). It has no root Markdown document. Instead, each step may define:

- **`prompt`** — a file reference to a Markdown document that is composed and used as the prompt for that step.
- **`shell`** — a string or list of strings representing shell command(s) to run. Success means all commands exit 0.

Steps without `prompt` or `shell` would be an error in headless mode (there is no default prompt source).

### Work Required

#### 2a. CLI Layer

- **`sequence.rs` (CLI)** — detect when the positional file reference resolves to a YAML file instead of Markdown. Currently the code calls `resolve_composition_source` which expects Markdown; for YAML, skip Markdown parsing and call a new code path.
- **`SequenceArgs`** — no structural changes, but the `run_sequence_inner` logic needs branching: Markdown path → headed sequence, YAML path → headless sequence.

#### 2b. Library Layer — Headless Plan Resolution

- **New function** (e.g., `resolve_headless_sequence_plan`) in `composition/sequence.rs` — loads the YAML, parses it into a `SequencePlan` with step-type metadata, validates that each step has at least one of `prompt` or `shell`.
- **`SequenceStep`** — add fields:
    - `prompt_ref: Option<String>` — raw file reference string.
    - `shell_commands: Option<Vec<String>>` — one or more shell commands.
    - `shell_timeout: Option<u64>` — per-step timeout in seconds (default 30s).

#### 2c. Step Execution Router

- **`wrap/sequence.rs`** — the execution loop currently only knows how to compose a Markdown document and hand it to a provider. For headless steps, it needs to:
    - **Prompt steps**: resolve `prompt_ref` to a file path, compose that document (with its own frontmatter/parameters), build a `CompositionExecutionRequest`, and execute it.
    - **Shell steps**: run each command via `std::process::Command` (or the existing harness shell infrastructure), collect exit codes, respect the per-step `timeout`, and report success/failure.
- **Pre-flight** — shell commands defined in step `shell` blocks must be included in Phase 1 pre-flight approval discovery, alongside existing template `::shell` and harness shell commands.

#### 2d. Template Section Enhancement

- The future-state docs show a `template` section that can define properties beyond strings (e.g., `dir: "path/to/some/location"` as a static value). The current `template` handling only supports string values with `{{key}}` interpolation.
- **Enhancement** — allow `template` values to be any JSON type (strings, numbers, booleans, objects, arrays). String values continue to support `{{key}}` interpolation. Non-string values are merged into each step's `raw_state` as-is (only if the key doesn't already exist on the step).

#### 2e. Tests

- Headless YAML with `prompt` steps.
- Headless YAML with `shell` steps (single string and list).
- Headless YAML with mixed `prompt` and `shell` steps.
- Error: headless step with neither `prompt` nor `shell`.
- Shell timeout enforcement.
- Pre-flight includes step `shell` commands.

---

## 3. Parameters

### Current State

There is no parameter schema concept. CLI `key=value` setters and `--set` JSON are free-form; any key can be passed. There is no validation of required vs. optional, no type checking, and no `describe` command.

### Future State

Both Markdown documents and headless sequence YAML files can declare a `parameters` section that defines a typed schema for expected inputs. The parameter types are: `String`, `Filepath`, `Boolean`, `Number`, `Enum<...>`, `Dictionary`, plus `Option<T>` and `Array<T>` variants and default values.

The `claudine describe` command reports parameter schemas. Shell completions provide type-aware tab completion for parameter keys and `Filepath` values.

### Work Required

#### 3a. Parameter Type System (Library)

- **New types** in `composition/types.rs` (or a new `composition/parameters.rs`):
    - `ParameterType` enum with variants: `String`, `Filepath`, `Boolean`, `Number`, `Enum(Vec<String>)`, `Dictionary`.
    - `ParameterVariant` enum: `Required(ParameterType)`, `Optional(ParameterType)`, `Array(ParameterType)`.
    - `ParameterDefinition` struct: `name`, `variant`, `default_value: Option<serde_json::Value>`.
    - `ParameterSchema` struct: `HashMap<String, ParameterDefinition>`.
- **Parsing** — parse a `parameters` frontmatter key (or YAML `parameters` section) into a `ParameterSchema`. The schema format is:

  ```yaml
  parameters:
      name: String
      dir: Filepath
      spec_file: Option<Filepath>
      iteration: [Number, 1]
  ```

  Simple string values map to required types. Tuple values `[Type, default]` set a default. `Option<T>` and `Array<T>` are parsed from the type string.

#### 3b. Validation Engine (Library)

- **New function** (e.g., `validate_parameters`) — given a `ParameterSchema` and the user-provided setters, validate:
    - All required parameters without defaults are present.
    - Each value matches its declared type (e.g., `Filepath` values resolve to existing files, `Number` values parse as numeric, `Enum` values are in the allowed set).
    - `Option<T>` parameters accept `null` or omission.
    - `Array<T>` parameters accept lists of the element type.
- **Error types** — add `ParameterError` variants to `CompositionError` for: missing required parameter, type mismatch, enum value not in set, filepath not found, invalid type string.

#### 3c. Integration into Composition Pipeline

- **`resolve_composition_source`** or `prepare_direct` — after parsing frontmatter, extract `parameters`, then validate the user's `--set` / shorthand setters against the schema. Fail early before any agent execution.
- **Sequence integration** — headless YAML sequences define `parameters` at the top level. These are validated at plan resolution time. The resolved parameter values are merged into each step's `raw_state` alongside `template` values.
- **Prompt References with `params`** — when a step has `prompt: "@doc.md"` and `params: { key: value }`, those params are passed as setters to the referenced document's composition. If the referenced document also declares `parameters`, those are validated against the passed params.

#### 3d. `describe` Command (CLI)

- **New subcommand** `claudine describe <file>` — reads a Markdown document or headless YAML, extracts `parameters` and optional `description` from frontmatter, and renders:
    - File name (blue, bold).
    - Description (dim, italic) if present.
    - One line per parameter: `  - <i>requires</i> <inverse>name</inverse> <i>as</i> <b>Type</b>` (or `optional` for `Option<T>`).
- **`--json` flag** — outputs structured JSON: `{ "description": "...", "parameters": { "name": "String", ... } }`.

#### 3e. Shell Completions

- **Tab completion for parameter keys** — when composing or running a sequence, if the source document defines `parameters`, tab-complete the key names.
- **Tab completion for `Filepath` values** — when a parameter is typed as `Filepath`, tab-complete valid file references.
- This requires teaching the completion scripts to read frontmatter from the referenced file and extract `parameters`.

#### 3f. Tests

- Parse all parameter types and variants.
- Validate required parameter present/missing.
- Validate type mismatches (string where Number expected, etc.).
- Validate `Filepath` resolves to existing file.
- Validate `Enum` value in/out of allowed set.
- Default values fill in when caller omits parameter.
- `Option<T>` accepts omission.
- `Array<T>` accepts list values.
- `describe` command output (text and JSON).
- Error messages are actionable.

---

## 4. Per-Step `params` Property

### Current State

All steps share the same root document as their prompt. There is no way to pass different parameters to different steps, or to pass parameters to an externally referenced prompt document.

### Future State

A step in a headless sequence can define a `params` property — a key/value map that is passed as setters to the step's prompt document (if it has one). Template variables from `parameters` and `template` are available for interpolation within `params` values.

### Work Required

- **`SequenceStep`** — add `params: Option<serde_json::Map<String, serde_json::Value>>`.
- **Step parsing** — extract `params` from object steps in `normalize_inline_list` or the headless plan resolver.
- **Execution** — when executing a prompt step, merge the step's `params` with any top-level parameter values. The step's `params` override top-level values for that step's composition.
- **Template interpolation** — `params` values that contain `{{key}}` syntax should be interpolated using the step's resolved state (which includes `parameters`, `template`, and `state` values).
- **Tests** — params passed through to composed document; params override top-level values; template interpolation within params.

---

## 5. Per-Step `operation` Property

### Current State

The `--operation` CLI flag sets the `OPERATION` environment variable for the entire sequence run. It is applied uniformly to all steps via `env_overrides`.

### Future State

Any step (headed or headless) can define an `operation` property. The value is assigned to the `OPERATION` environment variable for that step's session, overriding any global `--operation` flag.

### Work Required

- **`SequenceStep`** — add `operation: Option<String>`.
- **Step parsing** — extract `operation` from object steps.
- **Execution loop** (`wrap/sequence.rs`) — when building `env_overrides` for a step, check if the step defines `operation`. If so, set `OPERATION` to that value; otherwise fall back to the global `--operation` flag.
- **Tests** — per-step operation overrides global; step without operation inherits global; operation appears in composed session environment.

---

## 6. Preflight Checks for Shell Command Blocks

### Current State

Preflight only covers template `::shell` directives and harness shell commands discovered during composition. There is no concept of step-level shell commands that need preflight approval.

### Future State

All `shell` commands defined in sequence steps are included in Claudine's preflight checks. If any command is not already whitelisted, the interactive approval dialog fires immediately at the start of execution (before any agent launches).

### Work Required

- **Phase 1 preflight loop** (`wrap/sequence.rs:106-192`) — for each step that has `shell_commands`, add those commands to the preflight approval pass. This means the loop needs to inspect step metadata (not just compose the document).
- **Cumulative approval** — shell commands from step definitions should be added to `cumulative_approved` just like template and harness commands.
- **Default timeout** — shell commands have a 30-second default timeout. The `shell_timeout` per-step override applies to each command in that step.

---

## 7. External YAML Format Unification

### Current State

External YAML supports two formats:
1. `sequence: [...]` (plain list, no templates)
2. `kind: sequence` + `list: [...]` + `template: {...}` (templated)

### Future State

The future-state docs use a more streamlined format:

```yaml
template:
    description: "{{name}} ({{age}} years old)"
sequence:
    - name: Bob
      age: 32
```

The `kind: sequence` + `list` form still works but the primary form uses `sequence:` directly alongside `template:`. The `template` section gains broader value types (not just strings).

### Work Required

- **`load_external_sequence`** (`composition/sequence.rs:207`) — relax the restriction that `template` is only valid with `kind: list`. Allow the `sequence:` + `template:` combination directly. Remove the error at line 228-233.
- **Template value types** — expand `template` validation to accept any JSON value type, not just strings. String values still get `{{key}}` interpolation; non-string values are merged as-is.
- **Tests** — update `external_template_rejected_in_plain_sequence_form` test (currently expects an error for this combination); add new test for `sequence:` + `template:` working correctly; add tests for non-string template values.

---

## 8. `describe` Command for Sequences

### Current State

No `describe` command exists.

### Future State

`claudine describe` works for both Markdown documents and headless sequence YAML files. For sequences, it reports the sequence's `parameters` and `description`.

### Work Required

- Covered in section 3d (Parameters — `describe` command). The implementation should handle both file types.

---

## Summary of Affected Files

| File | Changes |
|------|---------|
| `claudine/lib/src/composition/types.rs` | Add `id`, `prompt_ref`, `shell_commands`, `shell_timeout`, `params`, `operation` to `SequenceStep`. Add `SequenceStepOverlay.id`. Add parameter type system types. |
| `claudine/lib/src/composition/sequence.rs` | ID generation logic. Headless plan resolver. Parameter validation. Relax template+sequence restriction. Non-string template values. |
| `claudine/lib/src/composition/parameters.rs` | **New file.** Parameter type parsing, validation engine, error types. |
| `claudine/lib/src/composition/mod.rs` | Export new `parameters` module. |
| `claudine/lib/src/composition/error.rs` | Add `ParameterError` variants. |
| `claudine/cli/src/commands/sequence.rs` | Branch between headed (Markdown) and headless (YAML) paths. |
| `claudine/cli/src/commands/wrap/sequence.rs` | Step-type routing (prompt vs. shell). Per-step `operation` env var. Preflight for step shell commands. Shell command execution with timeout. |
| `claudine/cli/src/commands/describe.rs` | **New file.** `describe` subcommand implementation. |
| `claudine/cli/src/commands/mod.rs` | Register `describe` subcommand. |
| Shell completion scripts | Parameter key and `Filepath` value completions. |

---

## Dependency Order

The recommended implementation order, respecting dependencies:

1. **Step ID Generation** (§1) — foundational, no external dependencies.
2. **External YAML Format Unification** (§7) — small refactor, unblocks template enhancements.
3. **Parameter Type System** (§3a–3b) — core library work, no CLI changes yet.
4. **Parameters — Pipeline Integration** (§3c) — wire validation into compose/sequence flows.
5. **Headless Sequences** (§2) — builds on IDs and unified YAML format.
6. **Per-Step `params`** (§4) — extends headless execution.
7. **Per-Step `operation`** (§5) — small, self-contained.
8. **Shell Command Block Preflight** (§6) — extends the Phase 1 loop.
9. **`describe` Command** (§3d) — depends on parameter type system being stable.
10. **Shell Completions** (§3e) — polish, depends on `describe` being able to extract schemas.
