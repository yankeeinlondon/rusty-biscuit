---
phases: 5
created: 2026-05-15
updated: 2026-05-25
start_phase: 1
---

# Execution Plan: Schema Support in Claudine

This plan outlines the steps required to incorporate schema support into Claudine, leveraging Darkmatter's `SimplifiedSchema` definitions (the established Darkmatter term; avoid `SimpleSchema` in code/docs).

## Phase 1: Foundation & Infrastructure

In this phase, we prepare the configuration and error handling systems to support schema-related logic.

- [ ] **Task 1: Update Configuration Schema**
    - Modify `claudine/lib/src/config/claudine_config.rs` (or appropriate config file) to add `prompt_for_missing: bool`.
    - Ensure it defaults to `true` if not present in the user's config file.
    - Scope: user config only (`~/.claudine/config.json` / JSON5); repo-scoped config must reject this field.
    - Serialization should omit the field when it is `true` if that matches existing config style.
    - Add `claudine config set prompt-for-missing true/false` support in the non-interactive config setter.
    - Add boolean switch in the interactive config TUI.
- [ ] **Task 2: Define Schema Error Types**
    - Add error variants to the error enum in `claudine/lib/src/error.rs`:
        - `SchemaLoad`: schema resolution or compilation failures.
        - `SchemaValidation`: present-but-invalid values.
        - `MissingProperties`: required values absent and cannot be collected interactively.
        - `UnsupportedInteractiveSchema`: missing required values that cannot be mapped to a widget.
    - `MissingProperties` fields:
        - Path to the prompt file.
        - List of missing property names (in declaration order).
        - Map of property types and descriptions (from schema).
        - Frontmatter `description` when present.
    - All errors should implement or wrap `biscuit_terminal::errors::BlockError` for consistent CLI rendering.
- [ ] **Task 3: Implement Non-TTY Error Reporting**
    - Update `Display` or specialized reporting logic for `MissingProperties`.
    - Use `biscuit-terminal` / `Prose` to render the error.
    - Include OSC8 hyperlink to the prompt file when terminal supports it.
    - Render missing property names in declaration order with type and description.
    - Render frontmatter `description` as `<i><dim>{description}</dim></i>` when present.
    - Include remediation hint: `Pass key=value, use --set, or set prompt_for_missing to true in an interactive terminal.`
    - If schema metadata unavailable (raw JSON Schema), list JSON pointer paths from Darkmatter's validation problems.

## Phase 2: Schema Validation Logic

In this phase, we implement the core logic for validating composition properties against the schema defined in the prompt frontmatter.

- [ ] **Task 1: Integrate `SimplifiedSchema` into Composition**
    - Update the composition parsing logic (e.g., in `claudine/lib/src/composition/prepare.rs` or `resolve.rs`) to detect the `$schema` property.
    - Use Darkmatter's schema API to load, resolve, and parse the referenced schema.
    - Support inline SimplifiedSchema mappings, referenced YAML/JSON schema files, and root-level unions per Darkmatter behavior.
    - `$schema` references resolve relative to the prompt document's parent directory.
    - `file` property values validated using Darkmatter's `file` semantics (resolve from validation-time CWD).
    - Remote `http://` / `https://` references remain unsupported (Darkmatter v1 rejects them).
    - Raw JSON Schema accepted for validation but does not provide typed property metadata for Interactive Mode or completions.
- [ ] **Task 2: Implement Validation Timing**
    - Validate after Darkmatter composition produces effective frontmatter and before provider/model resolution.
    - Validation target: `PreparedComposition::effective_frontmatter` with run-scoped overrides applied.
    - Ordering:
        1. Parse `--set` and shorthand positional `key=value` setters (JSON5-first).
        2. For `sequence`, merge per-step overlay values after caller setters (reserved keys win).
        3. Compose the prompt through Darkmatter.
        4. Build effective schema through Darkmatter for the composed document.
        5. Validate effective frontmatter.
        6. If required properties missing and Interactive Mode allowed: collect values, re-apply as overrides, re-compose, re-validate.
        7. Continue to provider/model resolution only after validation succeeds.
- [ ] **Task 3: Implement Property Validation**
    - Create a validation function that checks provided properties against the schema.
    - Categorize properties as: Present and Valid, Present but Invalid, or Missing.
    - Required property outcomes:
        - Present and valid: continue.
        - Missing: prompt in Interactive Mode when allowed; otherwise return `MissingProperties`.
        - Present but invalid: return hard `SchemaValidation` error. Do not prompt.
    - Optional property outcomes:
        - Missing: continue.
        - Present and valid: continue.
        - Present but invalid: drop value from prompt context, emit warning, re-compose and re-validate.
- [ ] **Task 4: Implement Non-Interactive Missing Property Detection**
    - Interactive Mode allowed only when ALL true:
        - `prompt_for_missing` is `true` or absent.
        - At least one required property is missing.
        - No invalid required properties.
        - Command has interactive input and error surface available.
        - `--silent` is not active.
    - Use stdin plus stderr TTY detection (stdout may be piped).
    - When Interactive Mode not allowed, return `MissingProperties`.

## Phase 3: Interactive Mode (TUI)

This phase implements the interactive user experience for fulfilling missing required properties.

- [ ] **Task 1: Implement Diagnostic Status Reporting**
    - Before prompting, render schema status report to stderr using `biscuit-terminal::Prose`:
        - Header: `- The [{prompt-relative-path}]({prompt-absolute-path}) prompt has the following schema:`
        - Required properties:
            - valid: `<green>✓</green> <inverse>{property}</inverse>: {type} <i><dim>- was defined correctly</dim></i>`
            - invalid: `! <inverse>{property}</inverse>: {type} <i><dim>- was defined but with the wrong type</dim></i>`
            - missing: `<red>⍉</red> <inverse>{property}</inverse>: {type} <i><dim>- was not defined but is required</dim></i>`
        - Optional properties:
            - valid: `<green>✓</green> <dim><i><inverse>{property}</inverse>: {type}</i></dim>`
            - missing: `<grey>⍉</grey> <dim><i><inverse>{property}</inverse>: {type}</i></dim>`
            - invalid: `<yellow>!</yellow> <dim><i><inverse>{property}</inverse>: {type}</i></dim>`
        - If any optional values invalid, also render: `- **Note:** _optional properties with invalid values will be dropped and the prompt will execute without them_`
    - Tests should assert semantic status data where practical instead of matching terminal glyphs exactly.
- [ ] **Task 2: Integrate `biscuit-tui` for Prompts**
    - In the CLI layer (`claudine/cli/src`), implement the interactive loop for missing properties.
    - Trigger only when `prompt_for_missing` is `true`, stdin is a TTY, stderr is a TTY, no invalid required properties, and `--silent` is not active.
    - Must not attempt to open TUI when Interactive Mode not allowed; return `MissingProperties` instead.
- [ ] **Task 3: Map Schema Types to TUI Widgets**
    - Use `biscuit-tui` components based on SimplifiedSchema property type:
        - `enum(...)` -> `choose_one`
        - `enum(...)[]` -> `choose_many`
        - `boolean` or `boolish` -> `boolean_switch`
        - `string`, `date`, `datetime`, `time`, `url`, `email`, `file` -> `text_input`
        - `number`, `numberlike` -> `text_input` with parse-and-retry
    - For `file`, accept `biscuit-file::FileReference` string; let Darkmatter validation report resolution/glob failures.
    - Unsupported shapes (raw JSON Schema, root-level unions without projection, property-level unions, `object`, `any`): if unsupported required property is missing, return `MissingProperties` with note that property cannot be collected interactively.
- [ ] **Task 4: Implement Numeric Validation and Retry**
    - For `number` / `numberlike` types in `text_input`, add validation logic to convert string.
    - If conversion fails, re-render prompt with inline error instead of aborting.

## Phase 4: Shell Completions

Update the shell completion system to be aware of schemas.

- [ ] **Task 1: Extract Schema Properties for Completions**
    - Update `claudine/cli/src/completion/frontmatter.rs` (or similar) to parse the schema when generating completions.
    - Applies to `key=value` positional setters for: `claudine compose`, `claudine inline-compose`, `claudine sequence`.
    - After prompt file identified, resolve effective schema through Darkmatter without executing shell directives or launching providers.
    - If Darkmatter cannot produce effective schema without executing shell directives, use raw document schema plus CLI setters only.
- [ ] **Task 2: Prioritize Required Properties**
    - Complete known property names before `=`.
    - List required properties before optional properties while preserving declaration order within each group.
    - Do not complete properties already supplied in current command line.
    - For `property=<TAB>`, use Darkmatter's completion metadata:
        - enum values complete enum members.
        - file values complete filesystem paths filtered by `match(...)` patterns.
        - date/time/url/email values show format hint when shell completion protocol supports descriptions.
    - If schema cannot be loaded during completion, return no schema-aware candidates and fall back to existing completion behavior.
    - Completion must remain side-effect free.

## Phase 5: Sequence & Inline Compose Behavior

Handle composition-specific validation rules.

- [ ] **Task 1: Sequence Validation**
    - `claudine sequence` validates prepared composition for each step.
    - Caller setters apply to every step.
    - Reserved per-step overlay values win over caller setters.
    - Missing required values collected during Phase 1a, before any provider session starts.
    - If multiple steps need same missing property with same schema type and description: prompt once and reuse answer for later steps unless step overlay supplies different value.
    - If any step has invalid required value, abort before launching any provider session.
    - Aggregate non-interactive `MissingProperties` errors by step so user can fix full sequence in one edit.
- [ ] **Task 2: Inline Compose Validation**
    - `inline-compose` validates effective frontmatter after `prompt` property extracted and composed into temporary document.
    - Existing `prompt` frontmatter requirement remains separate:
        - Missing `prompt` returns `PromptPropertyMissing`.
        - Non-string `prompt` returns `PromptPropertyWrongType`.
        - Schema validation runs after those inline-specific checks.
    - `$schema` property preserved with original frontmatter during inline rewrite.
    - Interactive values collected for run are not written back to source file.

## Phase 6: Documentation & Validation

Finalize the feature with documentation and comprehensive tests.

- [ ] **Task 1: Update Documentation**
    - Update `claudine/docs/topics/composition.md`
    - Update `claudine/docs/topics/execution-flow.md`
    - Update `claudine/docs/topics/shell-completions.md`
    - Update `.claude/skills/claudine/SKILL.md`
    - Update `docs/dependencies.md` and `claudine/docs/dependencies.md` if new crates added
    - Do not add dependency for schema parsing unless Darkmatter does not already expose required API.
- [ ] **Task 2: Unit Testing**
    - Add unit tests for schema validation logic in `claudine/lib`.
    - Direct `compose` validates effective frontmatter after `--set` and shorthand setters.
    - `inline-compose` keeps `prompt` validation behavior then validates schema.
    - `sequence` validates every step before provider launch.
    - Invalid required values abort without prompting.
    - Invalid optional values are dropped with warning.
    - Raw JSON Schema validates but does not enable Interactive Mode.
    - Repo config rejects `prompt_for_missing`.
- [ ] **Task 3: Integration Testing**
    - Add integration tests for:
        - Non-interactive mode (verifying `MissingProperties` error).
        - Missing required properties prompt only when stdin and stderr are TTYs.
        - Shell completion orders required properties before optional properties.
        - Enum and file completion use Darkmatter completion metadata.
        - Mocked interactive mode (if possible within test harness).

## Validation Checkpoints

1. **Infrastructure Check:** Verify `prompt_for_missing` is correctly loaded from config, defaults to `true`, and is rejected in repo-scoped config.
2. **Error Presentation Check:** Trigger a `MissingProperties` error in a non-TTY environment and verify the OSC8 link, property ordering, and formatting.
3. **Validation Logic Check:** Verify that wrong-type required properties cause a hard `SchemaValidation` abort; wrong-type optional properties are dropped with warning and re-validated.
4. **Interactive Flow Check:** Verify that missing required properties trigger the status report and subsequent TUI prompts; invalid required properties abort without prompting.
5. **Completion Check:** Verify that `claudine <prompt> key=<TAB>` suggests schema properties (required first), enum members, and file paths.
6. **Sequence Check:** Verify that sequence validates all steps before launch, aggregates missing properties by step, and reuses answers for identical missing properties across steps.
7. **Inline Compose Check:** Verify that `prompt` validation runs before schema validation and `$schema` is preserved during inline rewrite.
