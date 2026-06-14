---
created: 2026-05-26
package: darkmatter
component: schemas / format validators
severity: usability
reviewed: true
status: ready for planning and implementation
related_code:
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
---

# Schema Validation Error for `file(required)` Misleads Users on Missing Files

## The Problem We Are Solving

When a `compose` or other Darkmatter schema-validated frontmatter property is
declared as `file(required)` and the supplied reference does not resolve to an
existing file, the user receives a generic JSON Schema format error:

```text
MarkdownError: schema validation failed
┃ /Users/ken/.claudine/worktrees/rusty-biscuit/renderable/prompts/implement-plan.md
┃ invalid plan: "features/2026-05-26-block-extensions/plan.md" is not a valid "darkmatter-file" format
Correct the frontmatter so it satisfies the declared $schema (or baseline schema).
```

The message does not distinguish an invalid reference, a resolution failure,
and a well-formed reference for which no file was found. In the observed case,
the reference contained a typo (`block-extensions` plural instead of
`block-extension` singular), but the format-oriented wording led debugging
toward the target document's contents and schema instead of its path.

The format name is also easy to over-read. `darkmatter-file` does not parse the
target as Markdown or validate its frontmatter. It verifies only that the value
is a valid [`FileReference`][file-reference] and resolves to an existing regular
file under `FileReference`'s current search semantics.

### Affected Combined Constraint

For `file(match(...))`, a missing file currently fails both validators:

1. `format: darkmatter-file` reports a generic format mismatch.
2. `x-darkmatter-match` reports that the same reference does not match the
   configured globs.

The second diagnostic is false precision: glob matching was never reached
because the file did not resolve. Fixing only the format message would still
leave users with a misleading second cause.

## Why the Message Reads This Way

`darkmatter-file` is registered through `ValidationOptions::with_format` in
[`format.rs`][format.rs]. A custom JSON Schema format returns only `bool`, so
the current validator collapses all failures to `false`:

```rust
fn validate_file_reference(value: &str) -> bool {
    let Ok(reference) = FileReference::new(value) else {
        return false;
    };
    matches!(reference.resolve(), Ok(Some(path)) if path.exists())
}
```

The `jsonschema` crate then creates its standard
`ValidationErrorKind::Format { format: "darkmatter-file" }` error.
[`build_problem`][validate.rs] copies `err.to_string()` into
`ValidationProblem::message`, and [`blocks.rs`][blocks.rs] renders that as
`invalid <property>: <message>`.

`FileReference::resolve()` has a relevant contract boundary: `Ok(Some(path))`
contains an existing file, while `Ok(None)` means no candidate matched. It does
not expose the candidate paths it searched. Therefore Darkmatter cannot
truthfully display the attempted absolute path for a missing reference without
adding a diagnostic API to `biscuit-file` or duplicating its resolution rules.

## What We Are Building

### Goal

Produce actionable, failure-specific diagnostics for `darkmatter-file` while
preserving the public JSON Schema representation and `FileReference` resolution
semantics. A missing file must be reported as a missing match, not as an invalid
format or glob mismatch.

### Design Decision

Keep `format: darkmatter-file` and enrich its error while mapping
`jsonschema::ValidationError` into `ValidationProblem`.

**Reader's note:** an earlier design replaced the format with a private
`x-darkmatter-file` keyword. That would either break direct JSON Schemas that
use `"format": "darkmatter-file"` or require recursively rewriting every
schema and referenced subschema before compilation. The mapper already has the
format kind and rejected instance value, so replacing the public schema
contract is unnecessary.

The implementation should centralize the check in `format.rs`, for example as
a private helper returning a typed result:

```rust
fn resolve_file_reference(value: &str) -> Result<PathBuf, FileReferenceFailure>;
```

The existing bool format validator becomes a thin
`resolve_file_reference(value).is_ok()` adapter. When `build_problem` sees a
`ValidationErrorKind::Format` for `darkmatter-file` and a string instance, it
reruns that helper only on the error path and substitutes the specific message.
All other JSON Schema format errors retain `err.to_string()` unchanged.

This performs a second resolution only for invalid values. That is preferable
to changing the schema vocabulary or carrying mutable side-channel state out of
the validator. If filesystem state changes between the two checks and the
second check succeeds, retain the original generic format message rather than
inventing a failure.

### Required Behavior Changes

#### 1. Distinguish the three file-reference failure modes

Use these message contracts:

| Failure                                          | Replacement message                                                             |
| ------------------------------------------------ | ------------------------------------------------------------------------------- |
| `FileReference::new(value)` returns `Err(error)` | `` `<value>` is not a valid file reference: <error> ``                          |
| `reference.resolve()` returns `Err(error)`       | ``could not resolve file reference `<value>`: <error>``                         |
| `reference.resolve()` returns `Ok(None)`         | `` no existing file matched reference `<value>` while resolving from `<cwd>` `` |

For the `Ok(None)` case, obtain `<cwd>` with `std::env::current_dir()` after the
failed resolution. If it is unavailable, omit the `while resolving from ...`
clause. Do not fabricate a candidate absolute path: `FileReference` may search
multiple roots for implicit, magic, package, vault, and recursive references.

Messages must use the rejected instance string supplied by
`ValidationError::instance()`. They must not parse the JSON pointer or rendered
error text to recover the value.

#### 2. Preserve the public schema contract

- SimplifiedSchema `file(...)` atoms continue to compile to
  `{"type":"string","format":"darkmatter-file"}` plus
  `x-darkmatter-match` when requested.
- Direct JSON Schemas using `"format": "darkmatter-file"` receive the same
  improved diagnostics through Darkmatter's validator.
- `register_darkmatter_formats`, `DARKMATTER_FILE_FORMAT`, and format validation
  remain available with their current roles.
- Built-in formats and the existing `x-darkmatter-url-scheme` keyword are not
  affected.

#### 3. Prevent duplicate or misleading glob diagnostics

Refactor the `x-darkmatter-match` check to distinguish:

- file resolved and matched the globs: valid;
- file resolved but did not match the globs: emit the existing glob diagnostic;
- file-reference parse, resolution, or no-match failure: treat the keyword as
  valid and defer to `format: darkmatter-file`, which owns those diagnostics.

This is an intentional narrowing of the keyword's responsibility. The keyword
validates glob constraints; the format validates that the value resolves to a
file. SimplifiedSchema always emits the format alongside the match keyword.

Add a schema-build guard in `match_keyword_factory`: if the parent schema does
not contain `"format": "darkmatter-file"`, reject the schema with a clear
schema error. This prevents direct JSON Schema authors from using
`x-darkmatter-match` alone and accidentally bypassing existence validation.

#### 4. Preserve the error envelope

- The `MarkdownError: schema validation failed` header remains unchanged.
- `ValidationProblem::kind` remains `Invalid` for all file-reference failures.
- JSON-pointer path, property attribution, line, column, and root-union arm
  attribution remain unchanged.
- The generic hint remains unchanged. Once the problem line says that no file
  matched, the hint no longer obscures the cause.
- Error messages continue to be escaped by the renderer through `Prose`; the
  mapper stores plain text and must not add terminal markup or raw escape codes.

### Non-Goals

- Do not parse the target as Markdown or validate its frontmatter.
- Do not change `FileReference` search order, supported reference forms, or
  regular-file requirement.
- Do not add a `biscuit-file` diagnostic/candidate API in this change.
- Do not change the SimplifiedSchema `file(required)`, `file(match(...))`, or
  `file(required, match(...))` syntax.
- Do not add a new `ValidationProblemKind` variant.
- Do not customize unrelated JSON Schema format diagnostics.

## Files Most Likely to Change

- `darkmatter/lib/src/markdown/schemas/format.rs`
    - Introduce the shared typed file-reference check.
    - Keep the bool format adapter.
    - Make glob validation defer file-reference failures.
    - Require `x-darkmatter-match` to accompany `format: darkmatter-file`.
- `darkmatter/lib/src/markdown/schemas/validate.rs`
    - Specialize only `darkmatter-file` format messages in `build_problem`.
    - Preserve the generic message if the rejected instance is not a string or
      the diagnostic recheck no longer fails.
- `darkmatter/lib/src/markdown/errors/blocks.rs`
    - No behavior change expected; verify that the new plain messages render and
      escape correctly.
- `darkmatter/lib/tests/error_snapshots/markdown_error.rs` and schema tests
    - Update snapshots that intentionally contain the old generic format text.

`darkmatter/lib/src/markdown/schemas/simplified/convert.rs` should not change;
its current output is part of the compatibility requirement.

## Test Requirements

Add focused tests for:

1. An empty reference, `%`, an unclosed interpolation, or another stable parse
   failure produces `is not a valid file reference` and includes the source
   error.
2. A syntactically valid reference with a resolution error, such as an unset
   `{{ENV_VAR}}` or unconfigured `vault:` root, produces
   `could not resolve file reference` and includes the source error.
3. A well-formed missing relative file produces
   `no existing file matched reference` and includes the reference and current
   resolution directory.
4. A missing absolute, magic, package, or recursive reference does not claim a
   fabricated absolute candidate path.
5. An existing file validates successfully.
6. An existing file that violates `file(match(...))` emits only the glob
   diagnostic.
7. A missing file under `file(match(...))` emits only the file-reference
   diagnostic.
8. `x-darkmatter-match` without `format: darkmatter-file` fails schema
   construction with an explanatory message.
9. A direct JSON Schema using `format: darkmatter-file` receives the improved
   message, proving this is not limited to SimplifiedSchema conversion.
10. An unrelated format failure, such as `format: date`, retains the upstream
    `jsonschema` message.
11. Root-union validation and nested/array instance paths retain their existing
    path and arm attribution.
12. Error rendering escapes reference and source-error text and does not treat
    it as `Prose` markup.

Tests that mutate the process working directory or environment must use the
repository's existing serialization/guard pattern and restore state on drop.

## Acceptance Criteria

For a missing relative reference, the rendered problem reads in substance:

```text
MarkdownError: schema validation failed
┃ /…/prompts/implement-plan.md
┃ invalid plan: no existing file matched reference `features/2026-05-26-block-extensions/plan.md`
┃               while resolving from `/…/renderable`
Correct the frontmatter so it satisfies the declared $schema (or baseline schema).
```

Specifically:

- `is not a valid "darkmatter-file" format` no longer appears for
  `darkmatter-file` failures reported through Darkmatter.
- Parse, resolution, and no-match failures have distinct messages.
- Missing files under `file(match(...))` do not also report a glob mismatch.
- Existing files and genuine glob mismatches preserve current validation
  behavior.
- SimplifiedSchema output and direct `format: darkmatter-file` schemas remain
  compatible.
- Existing schema, error-rendering, and snapshot tests pass after intentional
  expectation updates.

## References

- Format registration and glob keyword: [`format.rs`][format.rs]
- Validation-error mapping: [`validate.rs`][validate.rs]
- Error envelope renderer: [`blocks.rs`][blocks.rs]
- `FileReference` resolution contract: [`biscuit-file` file reference module][file-reference]

[format.rs]: ../../../darkmatter/lib/src/markdown/schemas/format.rs
[validate.rs]: ../../../darkmatter/lib/src/markdown/schemas/validate.rs
[blocks.rs]: ../../../darkmatter/lib/src/markdown/errors/blocks.rs
[file-reference]: ../../../biscuit-file/lib/src/file_reference/mod.rs
