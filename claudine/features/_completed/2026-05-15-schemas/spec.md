---
reviewed: true
status: ready for planning and implementation
---

# Schema Support in Claudine

Darkmatter now has schema support for Markdown frontmatter. This feature
integrates that support into Claudine's `compose`, `inline-compose`, and
`sequence` workflows without creating a second schema language or validation
engine.

Reference: [Darkmatter Schemas](@darkmatter/docs/topics/schema-definition.md).

## Goals

- Reuse Darkmatter's `$schema` contract, schema resolution, validation, and
  completion metadata exactly.
- Validate the effective frontmatter that Claudine already uses for provider
  selection, model selection, lifecycle configuration, harness configuration,
  and prompt rendering.
- Offer schema-aware shell completions for `key=value` positional setters.
- Prompt for missing required values in interactive sessions when configured
  to do so.
- Fail clearly in non-interactive sessions instead of hanging.

## Non-goals

- Do not add a Claudine-specific schema syntax. Claudine consumes
  Darkmatter's SimplifiedSchema and raw JSON Schema support through the
  Darkmatter API.
- Do not mutate prompt files to insert missing values. Values supplied by CLI
  setters or Interactive Mode are run-scoped overrides only.
- Do not coerce values after validation. If a value is invalid for the schema,
  Claudine reports the validation failure instead of rewriting it.
- Do not prompt for optional values in v1.
- Do not implement interactive prompting for raw JSON Schema or root-level
  union schemas unless Darkmatter exposes a SimplifiedSchema projection for
  them later.

## Schema Source and Resolution

Prompt authors declare schemas with the standard Darkmatter `$schema`
frontmatter property:

```yaml
---
$schema:
  review: "file(match('*.md'); required) -> File to review"
  strict: "boolean(default(false)) -> Whether to require actionable findings"
---
```

Claudine must call Darkmatter's schema API to resolve and validate schemas.
That means:

- inline SimplifiedSchema mappings, referenced YAML/JSON schema files, and
  root-level unions follow Darkmatter's documented behavior
- `$schema` references resolve relative to the prompt document's parent
  directory
- `file` property values are validated using Darkmatter's `file` semantics,
  which resolve from the validation-time current working directory
- remote `http://` and `https://` `$schema` references remain unsupported
  because Darkmatter v1 rejects them
- raw JSON Schema is accepted for validation but does not provide the typed
  property metadata needed for Interactive Mode or setter completions

Reader note: the original draft referred to `SimpleSchema`; the established
Darkmatter term is `SimplifiedSchema`. Claudine should use Darkmatter's type
names in code and docs to avoid implying a separate local schema contract.

## Validation Timing

Claudine must validate schemas after Darkmatter composition has produced the
effective frontmatter and before provider/model resolution or provider launch.

The validation target is `PreparedComposition::effective_frontmatter`, with
run-scoped overrides already applied:

1. Parse `--set` and shorthand positional `key=value` setters using
   Claudine's existing JSON5-first setter behavior.
2. For `sequence`, merge per-step overlay values (`state`, `step`,
   `total_steps`, `is_first`, `is_last`, `previous_state`, `next_state`) after
   caller-supplied setters, preserving the existing rule that reserved
   sequence keys win.
3. Compose the prompt through Darkmatter.
4. Build the effective schema through Darkmatter for the composed document.
5. Validate the effective frontmatter.
6. If required properties are missing and Interactive Mode is allowed, collect
   the missing values, re-apply them as run-scoped overrides, re-compose, and
   re-validate.
7. Continue to provider/model resolution only after validation succeeds.

This ordering is required because the existing composition pipeline already
allows templates, transclusions, and sequence overlays to change frontmatter.
Validating raw source frontmatter would reject prompts that are valid after
composition and would miss invalid values injected during composition.

## Required Properties

Required properties come from Darkmatter's effective schema. A property is
considered fulfilled when the effective frontmatter contains it and Darkmatter
validation accepts its value.

Required property outcomes:

| Outcome | Behavior |
|---------|----------|
| Present and valid | Continue. |
| Missing | Prompt in Interactive Mode when allowed; otherwise return `MissingProperties`. |
| Present but invalid | Return a hard schema validation error. Do not prompt. |

Optional property outcomes:

| Outcome | Behavior |
|---------|----------|
| Missing | Continue. |
| Present and valid | Continue. |
| Present but invalid | Drop the value from the prompt context for this run and emit a warning, then re-compose and re-validate. |

Dropping invalid optional values is a deliberate Claudine behavior, not a
Darkmatter behavior. It keeps optional prompt variables from breaking a run
while preserving a strict contract for required inputs.

## Configuration

Add a user-scoped configuration field:

```json5
{
  prompt_for_missing: true
}
```

Rules:

- field name: `prompt_for_missing`
- type: boolean
- default when absent: `true`
- scope: user config only (`~/.claudine/config.json` / JSON5)
- repo-scoped config must not accept this field
- serialization should omit the field when it is `true` if that matches
  Claudine's existing config style for default values

The interactive config TUI should expose this as a boolean switch. The
non-interactive config setter should also support it:

```sh
claudine config set prompt-for-missing true
claudine config set prompt-for-missing false
```

Reader note: the existing config type uses `preferred_agent` on disk with a
`favorite_agent` read alias. This feature should not add another alias unless
there is a migration reason.

## Interactive Mode

Interactive Mode is entered only when all of these are true:

- `prompt_for_missing` is `true` or absent
- at least one required property is missing
- there are no invalid required properties
- the command has an interactive input and error surface available
- `--silent` is not active

Use stdin plus stderr TTY detection for the prompt UI. Stdout may be piped for
machine-readable output, so stdout alone must not decide whether prompting is
allowed.

When Interactive Mode is not allowed, return `MissingProperties`.

### Status Report

Before prompting, render a schema status report to stderr using
`biscuit-terminal::Prose` and related terminal renderables:

- `- The [{prompt-relative-path}]({prompt-absolute-path}) prompt has the following schema:`
- Required properties:
  - valid: `<green>✓</green> <inverse>{property}</inverse>: {type} <i><dim>- was defined correctly</dim></i>`
  - invalid: `! <inverse>{property}</inverse>: {type} <i><dim>- was defined but with the wrong type</dim></i>`
  - missing: `<red>⍉</red> <inverse>{property}</inverse>: {type} <i><dim>- was not defined but is required</dim></i>`
- Optional properties:
  - valid: `<green>✓</green> <dim><i><inverse>{property}</inverse>: {type}</i></dim>`
  - missing: `<grey>⍉</grey> <dim><i><inverse>{property}</inverse>: {type}</i></dim>`
  - invalid: `<yellow>!</yellow> <dim><i><inverse>{property}</inverse>: {type}</i></dim>`

If any optional values are invalid, also render:

`- **Note:** _optional properties with invalid values will be dropped and the prompt will execute without them_`

The glyphs above are presentational. Tests should assert semantic status data
where practical instead of matching terminal glyphs exactly.

### Widget Mapping

Use biscuit-tui components based on the SimplifiedSchema property type:

| Schema type | Widget |
|-------------|--------|
| `enum(...)` | `choose_one` |
| `enum(...)[]` | `choose_many` |
| `boolean` or `boolish` | `boolean_switch` |
| `string`, `date`, `datetime`, `time`, `url`, `email`, `file` | `text_input` |
| `number`, `numberlike` | `text_input` with parse-and-retry |

For numeric values, failed parsing should re-render the prompt with an inline
error instead of aborting. For `file`, Claudine should accept a
`biscuit-file::FileReference` string and let Darkmatter validation report
resolution or glob failures.

Unsupported interactive shapes:

- raw JSON Schema
- root-level unions without a SimplifiedSchema projection
- property-level unions where more than one widget kind is plausible
- `object`
- `any`

If an unsupported required property is missing, Claudine must return
`MissingProperties` with a note that the property cannot be collected
interactively from the available schema metadata.

## Non-TTY Fallback

When `prompt_for_missing` is `true` but Interactive Mode is not allowed,
Claudine must not attempt to open a TUI. It must return `MissingProperties`.

The error should include:

- an OSC8 link to the prompt file when the terminal supports hyperlinks
- the missing property names in declaration order when available
- each missing property's type and description when available
- the prompt file's frontmatter `description`, rendered as
  `<i><dim>{description}</dim></i>` when present
- a remediation hint:
  `Pass key=value, use --set, or set prompt_for_missing to true in an interactive terminal.`

If schema metadata is unavailable because the document uses raw JSON Schema,
the error should still list JSON pointer paths from Darkmatter's validation
problems.

## Shell Completions

Schema-aware completion applies to `key=value` positional setters for:

- `claudine compose`
- `claudine inline-compose`
- `claudine sequence`

Completion behavior:

- After a prompt file has been identified, resolve its effective schema through
  Darkmatter without executing shell directives or launching providers.
- Complete known property names before the `=`.
- List required properties before optional properties while preserving
  declaration order within each group.
- Do not complete properties already supplied in the current command line.
- For `property=<TAB>`, use Darkmatter's completion metadata:
  - enum values complete enum members
  - file values complete filesystem paths filtered by `match(...)` patterns
  - date/time/url/email values show a format hint when the shell completion
    protocol can display descriptions
- If the schema cannot be loaded during completion, return no schema-aware
  candidates and fall back to the existing completion behavior.

Completion must remain side-effect free. If Darkmatter cannot produce an
effective schema without executing shell directives, Claudine should use the
raw document schema plus CLI setters only and skip schema-aware completions
that depend on composed frontmatter.

## Sequence Behavior

`claudine sequence` validates the prepared composition for each step.

Rules:

- caller setters apply to every step
- reserved per-step overlay values win over caller setters
- missing required values are collected during Phase 1a, before any provider
  session starts
- if multiple steps need the same missing property with the same schema type
  and description, prompt once and reuse the answer for later steps unless a
  step overlay supplies a different value
- if any step has an invalid required value, abort before launching any
  provider session
- aggregate non-interactive `MissingProperties` errors by step so the user can
  fix the full sequence in one edit

This preserves the existing sequence contract that preflight work happens
before execution and avoids prompting halfway through a multi-step run.

## Inline Compose Behavior

`inline-compose` validates the effective frontmatter after the `prompt`
property has been extracted and composed into the temporary document. The
existing `prompt` frontmatter requirement remains separate:

- missing `prompt` still returns `PromptPropertyMissing`
- non-string `prompt` still returns `PromptPropertyWrongType`
- schema validation runs after those inline-specific checks

The `$schema` property is preserved with the original frontmatter during the
inline rewrite, just like other original frontmatter properties. Interactive
values collected for the run are not written back to the source file.

## Error Types

Add or extend typed composition errors so callers can distinguish:

- `SchemaLoad` for schema resolution or schema compilation failures
- `SchemaValidation` for present-but-invalid values
- `MissingProperties` for required values that are absent and cannot be
  collected interactively
- `UnsupportedInteractiveSchema` for missing required values that cannot be
  mapped to a widget

These errors should implement or wrap `biscuit_terminal::errors::BlockError`
so CLI rendering stays consistent with Darkmatter schema errors and existing
Claudine composition errors.

## Testing

Required coverage:

- direct `compose` validates effective frontmatter after `--set` and shorthand
  setters
- `inline-compose` keeps `prompt` validation behavior and then validates schema
- `sequence` validates every step before provider launch
- missing required properties prompt only when stdin and stderr are TTYs
- non-TTY sessions return `MissingProperties` without trying to enter a TUI
- invalid required values abort without prompting
- invalid optional values are dropped with a warning
- raw JSON Schema validates but does not enable Interactive Mode
- shell completion orders required properties before optional properties
- enum and file completion use Darkmatter completion metadata
- repo config rejects `prompt_for_missing`

## Documentation Updates

Update alongside implementation:

- `claudine/docs/topics/composition.md`
- `claudine/docs/topics/execution-flow.md`
- `claudine/docs/topics/shell-completions.md`
- `.claude/skills/claudine/SKILL.md`
- `docs/dependencies.md` and `claudine/docs/dependencies.md` if new crates are
  added

No dependency should be added for schema parsing unless Darkmatter does not
already expose the required API.

## Open Questions

None. The major design choices are resolved in this spec:

- Darkmatter remains the schema authority.
- Validation uses effective frontmatter, not raw source frontmatter.
- Interactive collection is limited to SimplifiedSchema shapes with an
  unambiguous widget mapping.
- Sequence prompting happens before provider execution, not mid-run.
