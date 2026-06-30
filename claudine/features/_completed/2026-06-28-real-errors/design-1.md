# Design 1 — Cause-First Structured Diagnostics

## Goal

Make Darkmatter and Claudine report the user's real mistake as the primary
error, not the mechanism that happened to surface it.

The reference failure is a missing or invalid file reference used inside a
frontmatter interpolation expression. Today that failure is flattened into:

```text
MarkdownError: transform failed
frontmatter key 'iteration': Interpolation evaluation failed for '...': frontmatter() invalid file path: "..."
```

The long-term fix is not a better string for this one path. The fix is to stop
turning structured authoring failures into strings before the render boundary.

## Assumptions

- `BlockError`, `StatusBlock`, `Prose`, `CodeBlock`, and `SourceContext` remain
  the terminal rendering foundation. They are already aligned with the repo's
  terminal rendering conventions.
- Darkmatter owns Markdown composition, expression evaluation, file-reference
  resolution, and source excerpts. It should therefore own the primary typed
  diagnostics for those failures.
- Claudine should preserve and enrich Darkmatter diagnostics, not translate them
  into parallel string-only `CompositionError` variants.
- Breaking changes are acceptable when they remove lossy error boundaries.

## Design Summary

Introduce a cause-first diagnostic spine in Darkmatter:

1. Replace expression and transform `Result<_, String>` boundaries with typed
   errors.
2. Attach scope as data while errors move upward: expression text, frontmatter
   key, source file, source line, function name, referenced path, base directory,
   and resolution attempts.
3. Render from the deepest typed cause via `BlockError`.
4. Let context renderers choose a focused source excerpt instead of dumping no
   YAML or all YAML.
5. Make Claudine preserve Darkmatter errors across crate boundaries and remove
   its string flattening paths over time.

The key principle is:

> Wrapping an error may add fields, but it must not rewrite the cause into
> prose.

## Core Types

Add a typed expression error family under Darkmatter, likely near
`darkmatter/lib/src/markdown/compose/expression/`:

```rust
pub enum ExpressionError {
    Parse {
        expression: String,
        message: String,
    },
    Evaluation {
        expression: String,
        source: Box<ExpressionCause>,
    },
}

pub enum ExpressionCause {
    UnknownFunction {
        name: String,
        suggestions: Vec<ExpressionSuggestion>,
    },
    InvalidArguments {
        function: String,
        message: String,
        expected: Vec<String>,
    },
    InvalidFileReference(FileReferenceDiagnostic),
    Io {
        operation: FileOperation,
        reference: Option<String>,
        path: PathBuf,
        source: std::io::Error,
    },
    Numeric {
        message: String,
    },
    Message {
        message: String,
    },
}
```

The exact enum names can change, but the important split is between:

- `ExpressionError`: where in expression evaluation the failure happened.
- `ExpressionCause`: what actually failed.

File-reference failures should get their own reusable diagnostic:

```rust
pub struct FileReferenceDiagnostic {
    pub function: String,
    pub raw: String,
    pub kind: FileReferenceFailureKind,
    pub base_dir: PathBuf,
    pub fallback_dir: Option<PathBuf>,
    pub resolved_attempts: Vec<PathBuf>,
    pub suggestions: Vec<PathSuggestion>,
}

pub enum FileReferenceFailureKind {
    Malformed,
    NotFound,
    RemoteNotEnabled,
    UnsupportedRemote,
}
```

This type should distinguish malformed input from a well-formed path that does
not exist. That distinction is currently visible in `resolve_arg`, then lost
when both cases become "invalid file path".

## Scope Attachment

Add a second layer for composition/interpolation scope:

```rust
pub enum InterpolationError {
    Expression {
        expression: String,
        source: ExpressionError,
    },
    FrontmatterKey {
        key: String,
        source: Box<InterpolationError>,
    },
    Body {
        source: Box<InterpolationError>,
    },
}
```

This replaces helpers such as `key_scoped_error()` that currently do:

```rust
MarkdownError::Transform(format!("frontmatter key '{key}': {msg}"))
```

The frontmatter key is valuable. It should be a field, not a prefix.

`MarkdownError::Transform(String)` should become either:

```rust
Transform(#[from] TransformError)
```

or be split into named top-level variants such as:

```rust
Interpolation(#[from] InterpolationError)
```

The cleaner long-term shape is a real `TransformError` enum whose variants
delegate to typed sub-errors. That avoids turning `MarkdownError` into a giant
catch-all enum while still letting Darkmatter's `BlockError` implementation
delegate to the true cause.

## Source Context

Every file-origin diagnostic should carry or be able to reach a
`SourceContext`.

The existing `SourceContext` already gives the renderer:

- absolute path for OSC8 links,
- display path for visible labels,
- full source content,
- frontmatter byte range.

The missing piece is plumbing it into the expression/interpolation path. The
composition preparation layer knows the source file and original content before
frontmatter interpolation runs, so it should construct a `SourceContext` and
pass it into the interpolation/evaluation context.

For the reference error, the resulting data should include:

- prompt file `SourceContext`,
- assigned frontmatter key `iteration`,
- expression text,
- function `frontmatter`,
- raw reference `features/2026-06-21-opencode-log-fix/spec.md`,
- base directory and fallback directory used for resolution,
- resolution failure kind,
- suggestions.

The renderer can then say:

```text
MarkdownError: invalid file reference

The file reference `features/.../spec.md` assigned to frontmatter property
`iteration` could not be found while evaluating the `frontmatter()` expression
in <linked prompt file>.
```

The transform/interpolation machinery can still be available in a compact
"while evaluating ..." line, but it is no longer the headline.

## Focused Frontmatter Excerpts

Move focused frontmatter excerpt support into Darkmatter rather than keeping it
Claudine-only.

Claudine's `FrontmatterExcerpt` already proves the value of TTY-gated YAML
appendices, but it is all-or-nothing: it captures the entire frontmatter block
and optionally highlights one line. The new design needs a source-aware excerpt
builder that can render the relevant shape:

```rust
pub struct FrontmatterFocus {
    pub keys: Vec<String>,
    pub include_parents: bool,
    pub highlight: Option<String>,
}
```

For the reference failure, the involved keys are `spec` and `iteration`, and
the structural parent is `$schema`. The excerpt renderer should show those
lines in YAML form with line numbers, preserving enough parent indentation to
make the relationship clear.

This should be Darkmatter-owned because:

- the `md` CLI needs the same quality as Claudine,
- `SourceContext` already lives in `biscuit-terminal`,
- Darkmatter owns frontmatter parsing and source ranges,
- Claudine should not need to parse Darkmatter internals to render Darkmatter
  failures.

Claudine can keep its current `FrontmatterExcerpt` temporarily as a compatibility
appendix, then delegate to Darkmatter's focused excerpt once available.

## File Suggestions

Add bounded file suggestions to Darkmatter's file-reference diagnostics.

Suggested behavior:

- For a relative path, first list siblings in the referenced parent directory if
  that parent exists.
- If the parent directory does not exist, suggest sibling directories from the
  nearest existing ancestor.
- Use the existing `darkmatter::catalog::levenshtein` style rather than adding a
  new fuzzy dependency.
- Cap directory scanning aggressively: for example, no recursion by default,
  stop after a fixed entry count, and return at most three suggestions.
- Keep suggestion generation best-effort. Failure to list a directory must not
  replace the original diagnostic.

Recursive search is tempting, but it can become expensive and surprising in
large repos. A later enhancement can add a bounded repo-indexed search if real
usage shows sibling suggestions are insufficient.

## Rendering

Rendering should be cause-driven.

Each typed cause that can produce a better message implements `BlockError`.
Wrapper errors either:

- delegate to their source's `BlockError`, or
- render their own block only when they add a genuinely more helpful user-level
  frame.

For `InvalidFileReference`, the block should own:

- headline: `invalid file reference` or `file reference not found`,
- linked prompt file,
- frontmatter property, if known,
- expression/function, if useful,
- focused frontmatter excerpt, when a `SourceContext` is available and stderr is
  a TTY,
- did-you-mean suggestions,
- a cause detail line for malformed path, disabled remote access, or I/O.

This avoids generic hints such as "Review the transform pipeline inputs". The
hint comes from the real cause:

- not found: "Check the path or choose one of the suggested files."
- malformed: "Fix the path syntax."
- remote disabled: "Enable remote reads or use a local path."
- unreadable: "Check file permissions or whether the file is still present."

Paths should render through a common helper, preferably based on
`SourceContext::linked_path_prose()` or a small `PathDisplay`/`PathLink` helper
in `biscuit-terminal`, so OSC8 links are a property of path fields rather than a
manual per-call-site choice.

## Claudine Boundary Changes

Claudine should adopt a preservation rule:

> If a lower layer returns a typed error, Claudine may wrap it with `#[source]`,
> but it must not convert it to `String`.

Immediate candidates:

- `claudine/lib/src/composition/resolve.rs`
- `claudine/lib/src/composition/sequence.rs`
- `claudine/cli/src/commands/sequence.rs`
- any `CompositionError::{InvalidReference, FileNotFound, MarkdownLoad,
  SequenceExternalLoad}` path that currently contains a formatted lower-layer
  error.

Some Claudine-specific variants can remain because they represent Claudine
workflow failures, not Darkmatter failures. But their fields should become typed
too:

```rust
InvalidReference {
    raw: String,
    source: biscuit_file::FileReferenceError,
}

FileNotFound {
    raw: String,
    attempted: Vec<PathBuf>,
    suggestions: Vec<PathSuggestion>,
}

MarkdownLoad {
    path: PathBuf,
    source: MarkdownLoadSource,
}
```

The CLI error walker should continue choosing the deepest `BlockError`, but the
walk should become simpler as Darkmatter and Claudine stop hiding causes inside
strings.

## Migration Plan

1. Add typed expression and interpolation error types in Darkmatter while
   preserving existing `Display` text closely enough for non-BlockError callers.
2. Convert `frontmatter()`, `absolute()`, and the shared file-resolution helper
   first. This proves the reference error end to end.
3. Replace `key_scoped_error()` with typed scope attachment.
4. Replace `MarkdownError::Transform(String)` with `TransformError`, or add a
   new typed interpolation variant and leave `Transform(String)` as legacy
   fallback during migration.
5. Implement `BlockError` for the new typed causes and add Darkmatter snapshot
   tests.
6. Add focused frontmatter excerpt extraction in Darkmatter and use it from the
   invalid-file-reference block.
7. Update Claudine bridges to preserve typed Darkmatter errors and remove
   `to_string()`/`format!("{e}")` wrappers at the crate boundary.
8. Add regression tests in both CLIs: `md compose` and `claudine compose` should
   render the same root-cause headline for the reference failure.
9. Add a review lint/check script or documented grep for new stringly error
   boundaries:
   - `Result<_, String>` in composition subsystems,
   - `MarkdownError::Transform(format!(...))`,
   - `map_err(|e| ... e.to_string())`,
   - `CompositionError::* (String)` for lower-layer causes.

## Technical Challenges

The largest challenge is expression error typing. The expression engine has a
large function catalog where both pure and filesystem functions currently share
`Result<Value, String>`. Converting the entire catalog at once would be noisy.
The pragmatic route is to introduce a typed `ExpressionCause` with a catch-all
`Message` variant, convert filesystem functions first, and then migrate pure
functions opportunistically.

The second challenge is keeping `Display` useful without making it the source of
truth. Many tests and callers may assert `to_string()` output. The new typed
errors should provide stable, concise `Display` messages, but rich rendering
must live in `BlockError`.

Focused YAML excerpts are non-trivial. YAML is indentation-sensitive, aliases
and sequences complicate "show these keys", and the existing line locator is a
simple mapping-key scanner. The first implementation should support the common
frontmatter mapping shape and fall back to the existing full-block excerpt when
it cannot confidently slice the structure.

Suggestions need cost controls. Large monorepos can have huge directories, and
recursive search could accidentally make an error path slow. Suggestions must be
bounded, optional, and never allowed to mask the root error.

Cross-crate trait-object discovery is another sharp edge. The current Claudine
walker uses Darkmatter's `as_block_error` registry plus a local downcast for
`CompositionError`. Adding more typed errors should follow the existing
`BlockError` conventions, but wrapper delegation and deepest-cause selection
need tests so a generic wrapper does not shadow the useful leaf.

Finally, there is a product challenge: too much diagnostic detail can become the
same density problem in a different form. The renderer must prioritize one
cause, one location, one focused excerpt, and a small suggestion list.

## Benefits

This design fixes the reference failure by making the invalid file reference the
root typed cause. The headline becomes "invalid file reference" instead of
"transform failed"; the file path can be linked; the assigned frontmatter key can
be highlighted; and suggestions can be generated from real path fields.

It also addresses the broader error-pattern catalog:

- P1: headlines come from the root cause, not the transform wrapper.
- P2: typed errors replace `String` at subsystem boundaries.
- P3: wrappers attach fields instead of prepending prose.
- P4: hints are cause-specific.
- P5: focused frontmatter excerpts replace no-context/full-context extremes.
- P6: source and resolution context are captured before they are lost.
- P7: file suggestions become part of file-reference diagnostics.
- P8: paths render through structured path/link helpers.
- P9: Claudine preserves Darkmatter typed errors with `#[source]`.
- P10: both `md` and `claudine` benefit because Darkmatter owns the core
  diagnostic.
- P11: malformed, missing, disabled-remote, and unreadable references remain
  distinct.

## Limitations and Open Questions

This design does not require a full migration to `miette`. The current
`BlockError` model is already repo-standard and terminal-aware. A future
`Diagnostic` bridge could be added later, but adopting it now would expand scope
without solving the root string-flattening problem.

The focused excerpt API needs more exploration. It should probably start with
frontmatter mapping keys and known parent inclusion, then grow only when real
documents require more YAML-aware behavior.

The right public API break for `MarkdownError::Transform(String)` needs a short
implementation spike. Replacing it outright is cleaner, but adding a typed
variant first may reduce churn while the expression catalog migrates.

The file suggestion strategy should be measured against real repos. Directory
sibling suggestions are cheap and predictable, but they may miss useful matches
when the typo is in an ancestor directory. A later repo-indexed suggestion
provider may be worthwhile if bounded local suggestions are not enough.

## Success Criteria

- The reference failure renders with a root-cause headline naming the invalid
  file reference.
- The rendered error includes the prompt file as an OSC8 link when terminal
  capabilities allow it.
- The rendered error names the frontmatter key that received the bad value.
- The rendered error shows a focused frontmatter excerpt when stderr is a TTY.
- The rendered error suggests likely intended files when cheap suggestions are
  available.
- `md compose` and `claudine compose` preserve the same root-cause diagnostic.
- New code paths do not introduce string-only lower-layer error variants for
  typed Darkmatter or Claudine failures.
