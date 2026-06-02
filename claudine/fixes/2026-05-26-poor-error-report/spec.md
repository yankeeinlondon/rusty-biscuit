---
created: 2026-05-26
package: darkmatter
component: schemas / format validators
severity: usability
related_code:
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
---

# Schema Validation Error for `file(required)` Misleads Users on Missing Files

## The Problem We Are Solving

When a `compose` (or any darkmatter schema-validated) frontmatter property is
declared as `file(required)` and the supplied path does not exist on disk,
the error surfaced to the user is:

```
 MarkdownError: schema validation failed
┃ /Users/ken/.claudine/worktrees/rusty-biscuit/renderable/prompts/implement-plan.md
┃ invalid plan: "features/2026-05-26-block-extensions/plan.md" is not a "darkmatter-file"
Correct the frontmatter so it satisfies the declared $schema (or baseline schema).
```

This wording strongly implies *the file was located but is not a valid
darkmatter document* (i.e., bad frontmatter, wrong shape, not Markdown). In
the failing case above the file simply does not exist — the user had a
typo (`block-extensions` plural; the real directory is `block-extension`
singular). Debugging followed the implied path: inspect the file's
frontmatter, check the schema, dig into the validator — none of which is
the actual problem.

The hint line ("Correct the frontmatter so it satisfies the declared
$schema") reinforces the same wrong direction.

## Why the Message Reads This Way

`darkmatter-file` is registered as a [`jsonschema`
`Format`](https://docs.rs/jsonschema/latest/jsonschema/struct.ValidationOptions.html#method.with_format)
in [`darkmatter/lib/src/markdown/schemas/format.rs:57-72`][format.rs]. The
`Format` trait is a `fn(&str) -> bool`, so three structurally different
failures collapse to a single `false`:

```rust
fn validate_file_reference(value: &str) -> bool {
    let Ok(reference) = FileReference::new(value) else {
        return false;                                 // (1) parse failure
    };
    matches!(reference.resolve(), Ok(Some(path)) if path.exists())
    //                            ^^^^^^^^^^^^^   ^^^^^^^^^^^^^^^^^
    //                            (2) unresolvable (3) does not exist
}
```

`jsonschema` then renders the failure with its default format-mismatch
template — `"<value>" is not a "<format-name>"` — and
[`build_problem`][validate.rs] in `validate.rs:236-262` passes that string
through verbatim as `ValidationProblem::message`. The renderer in
[`blocks.rs:232-235`][blocks.rs] formats it as `invalid <prop>: <message>`.

Nothing along this chain knows that the underlying failure was simply
"file not found at the resolved path."

This is also misleading in a second way: even on success the validator
does *not* confirm the file is actually a darkmatter document (it does
not parse frontmatter, does not check that it's Markdown). All it checks
is `FileReference` parse → resolution → `path.exists()`. The format
*name* `darkmatter-file` is overstating what the validator verifies.

## What We Are Building

### Goal

Replace the single bool-returning `Format` validator with an error path
that distinguishes the three failure modes and produces messages a user
can act on. The user should see *file not found at `<resolved-path>`*
when the path doesn't exist, not a generic "is not a 'darkmatter-file'."

### Non-Goals

- **Do not expand what the validator checks.** Today it verifies
  parseable-as-`FileReference` + resolves + path exists. Keep that
  scope; do not start parsing the target file or asserting frontmatter
  shape. (A future enhancement could verify Markdown/frontmatter, but
  that is out of scope here — and naming would then need to be revisited
  separately.)
- **Do not change the `$schema` surface syntax.** `file(required)` in
  the simplified schema must continue to compile to the same JSON
  Schema fragment; only the validator's internal implementation and its
  error reporting change.
- **Do not change schemas that depend on the format string
  `darkmatter-file`.** External consumers (if any) writing
  `"format": "darkmatter-file"` directly continue to work.

### Required Behavior Changes

1. **Switch `darkmatter-file` from a `Format` to a `Keyword`.**
   - Today `register_darkmatter_formats` in
     [`format.rs:57`][format.rs] uses `options.with_format(...)` which
     bottlenecks on `fn(&str) -> bool`.
   - The same file already demonstrates the richer `Keyword` pattern
     for `x-darkmatter-match` and `x-darkmatter-url-scheme`, both of
     which produce `ValidationError::custom("...")` strings tailored to
     the failure.
   - A new `DarkmatterFileKeyword` (or equivalent) replaces the
     `Format`. Register it via `with_keyword("format", ...)`-style
     wiring **only when the format atom is `darkmatter-file`** — or
     more cleanly: emit a sentinel keyword like `x-darkmatter-file`
     from `simplified::convert` instead of `format: darkmatter-file`,
     and register the `Keyword` factory for that. Either approach is
     acceptable; the second avoids overloading the standard `format`
     keyword and matches the existing `x-darkmatter-*` family.
   - `simplified/convert.rs:359` (the site that emits
     `format: darkmatter-file`) is the one place that needs updating
     if we choose the sentinel-keyword approach.

2. **Distinguish three failure modes with distinct messages.**

   | Failure | Today's message | Replacement |
   |---|---|---|
   | `FileReference::new(value)` returns `Err` | `"<v>" is not a "darkmatter-file"` | `` `<v>` is not a valid file reference: <inner-error> `` |
   | `reference.resolve()` returns `Ok(None)` or `Err(_)` | same | `` could not resolve file reference `<v>`: <inner-error> `` |
   | `resolve()` returned a path but `path.exists()` is `false` | same | `` file not found: `<resolved-path>` (from reference `<v>`) `` |

   The resolved path is the most actionable piece of information when
   the user typed a relative path — surface it.

3. **Preserve the rest of the error envelope.**
   - The `MarkdownError: schema validation failed` header and the
     bulleted `invalid <prop>: <message>` line in `blocks.rs` stay as
     they are. Only `<message>` changes — it now carries the new
     wording from item 2 instead of the upstream jsonschema template.
   - `ValidationProblem::kind` remains `Invalid` for all three
     sub-cases (do not introduce a fourth `ValidationProblemKind`
     variant just to disambiguate file failures — the message carries
     the disambiguation).
   - Line/column attribution via `PositionMap` continues to work
     because the path-to-frontmatter key wiring in
     [`build_problem`][validate.rs] does not depend on the message
     string.

4. **Fix the hint line for file-not-found cases.**
   - The current hint ("Correct the frontmatter so it satisfies the
     declared $schema (or baseline schema)") is technically true but
     reinforces the wrong mental model when the actual issue is a
     missing file.
   - Option A — leave the hint generic but make the bullet so explicit
     ("file not found") that the hint can't mislead.
   - Option B — render block in [`blocks.rs:241-245`][blocks.rs]
     accepts a custom hint or omits it when any problem's message
     begins with `file not found` (cheap heuristic).
   - Preference: A. The hint is still correct in the broader sense;
     the bullet just needs to be unambiguous.

### What Must Not Change

- The `file(required)` / `file(match(...))` / `file(required, match(...))`
  surface syntax in simplified schemas.
- The `FileReference` API and its resolution semantics. The fix is
  purely about how validator failures are *reported*.
- The two existing `x-darkmatter-*` keyword validators
  (`x-darkmatter-match`, `x-darkmatter-url-scheme`). Those already
  produce good custom messages and should be left alone.
- The standalone behavior of `validate_file_reference` if it is kept
  as a helper (private fn). Whether it returns `bool` or a richer
  `Result` is an implementation detail of the new keyword.

## Files Most Likely to Change

- `darkmatter/lib/src/markdown/schemas/format.rs` — replace
  `validate_file_reference` + `register_darkmatter_formats` with a
  `Keyword` implementation that returns
  `ValidationError::custom(...)` with one of the three messages above.
- `darkmatter/lib/src/markdown/schemas/simplified/convert.rs:359` — if
  going the sentinel-keyword route, emit
  `"x-darkmatter-file": true` (or `<the value>`) instead of (or in
  addition to) `"format": "darkmatter-file"`.
- `darkmatter/lib/src/markdown/schemas/validate.rs` — no changes
  required; `build_problem` already passes `err.to_string()` through.
  Confirm that `ValidationError::custom` messages survive that round
  trip cleanly (they do).
- Tests in `darkmatter/lib/src/markdown/schemas/format.rs` (existing
  `file_format_accepts_existing_file` / `file_format_rejects_missing_file`)
  — extend to assert on the specific message text for each of the
  three failure modes.
- Any snapshot/golden tests under `darkmatter/lib/tests/` or
  `darkmatter/lib/src/markdown/errors/blocks.rs` tests that capture
  the old wording — update.

## Acceptance Criteria

A composition like:

```sh
c compose prompts/implement-plan.md \
  plan=features/2026-05-26-block-extensions/plan.md \
  phase=1 total_phases=4 -y --claude
```

(where the path has a typo and the file does not exist) produces an
error whose bullet reads, in substance:

```
 MarkdownError: schema validation failed
┃ /…/prompts/implement-plan.md
┃ invalid plan: file not found: `/…/features/2026-05-26-block-extensions/plan.md`
┃                (from reference `features/2026-05-26-block-extensions/plan.md`)
Correct the frontmatter so it satisfies the declared $schema (or baseline schema).
```

Specifically:

- The phrase `is not a "darkmatter-file"` no longer appears for the
  file-not-found case.
- The resolved absolute path is shown so the user can see where
  resolution landed (and immediately notice the typo).
- A malformed `FileReference` string produces a *different* message
  ("is not a valid file reference") so the failure mode is
  unambiguous.
- Existing positive cases (file exists, resolves cleanly) still
  validate without diagnostics.
- All existing `format::tests` continue to pass; new assertions cover
  the three distinct messages.

## References

- Source of the misleading message: [`darkmatter/lib/src/markdown/schemas/format.rs:67-72`][format.rs]
- Pass-through of upstream message: [`darkmatter/lib/src/markdown/schemas/validate.rs:236-262`][validate.rs]
- Renderer that frames it: [`darkmatter/lib/src/markdown/errors/blocks.rs:200-245`][blocks.rs]
- Pattern to mimic for custom messages: `DarkmatterMatchKeyword` and
  `DarkmatterUrlSchemeKeyword` in the same `format.rs`.

[format.rs]: ../../../darkmatter/lib/src/markdown/schemas/format.rs
[validate.rs]: ../../../darkmatter/lib/src/markdown/schemas/validate.rs
[blocks.rs]: ../../../darkmatter/lib/src/markdown/errors/blocks.rs
