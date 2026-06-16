---
reviewed: true
status: "ready for planning and implementation"
---

# Context Variables and Expression Function Additions

This feature adds a small agent context group plus new expression functions for
path shaping, numeric predicates, string mutation, Markdown link generation,
terminal Prose rendering, and skill availability checks.

Reader's note: this review turns the draft inventory into a contract. The
important decisions are that path-shape helpers are context-aware
`FS_FUNCTIONS`, `ctx.agent` and `ctx.model` are always present strings, and the
new helpers follow Darkmatter's existing null-propagation and type-mismatch
rules unless explicitly called out below. The path helpers also separate
resolution from rendering: they resolve through `FileReference` so magic paths
and document-relative behavior stay correct, then render path-shaped output
with stable `/` separators for Markdown.

## Goals

- Add `ctx.agent` and `ctx.model` without triggering repo, hardware, or OS
  capture.
- Add expression functions that work on every expression surface where the
  existing read-side functions work.
- Keep file-reference behavior aligned with `biscuit_file::FileReference` and
  Darkmatter's existing `ResolutionContext`.
- Keep function dispatch, descriptor catalogs, and public docs in sync.

## Non-Goals

- Do not add new shell side effects.
- Do not check file existence for the new path-shape helpers.
- Do not fetch remote URLs for path-shape helpers.
- Do not infer or validate model names against an external provider catalog.

## Context Variables

Add a new demand-driven `Agent` capture group.

| Variable | Type | Source | Default |
|----------|------|--------|---------|
| `agent` | `String` | `AGENT` environment variable | `"unknown"` |
| `model` | `String` | `MODEL` environment variable | `"default"` |

Rules:

- Both keys must always be present when the `Agent` group is captured.
- Trim surrounding ASCII whitespace from the environment value.
- Treat a missing or empty `AGENT` as `"unknown"`.
- Treat a missing or empty `MODEL` as `"default"`.
- Preserve non-empty environment values as supplied after trimming. Do not apply
  a model allowlist; an allowlist would drift as providers add models.
- Add descriptors for both keys in
  `darkmatter/lib/src/markdown/compose/context/catalog.rs`.
- Update `ContextGroup::for_key()` and `ContextGroup::all()` so lazy context
  capture sees `ctx.agent` and `ctx.model`.

## Function Contract

Unless a function below says otherwise:

- Function names are canonical snake_case with the existing underscore-free
  aliases where that convention already applies.
- A wrong argument count returns an evaluation error.
- A `null` argument propagates to `null`.
- A type mismatch returns an evaluation error.
- Each function must have a matching `ExpressionFunctionDescriptor`.
- Dispatch and descriptor parity tests are part of the acceptance criteria.

## Filesystem Functions

Register these as context-aware `FS_FUNCTIONS`, not pure functions, because
they need the current document's `ResolutionContext` and magic paths.

Shared path rules:

- `file` must be a JSON string.
- Normalize with the existing `normalize_path_arg()` behavior before parsing.
- Resolve through `biscuit_file::FileReference::new(...)`, forward
  `ResolutionContext.magic_paths`, then call `resolve_from(ctx.base_dir)`.
- Use the resolved absolute path for validation and filesystem-aware splitting.
- For returned path strings, use the same display policy as `relative(file)`:
  repo-root relative when inside the repo, otherwise `ctx.base_dir` relative,
  otherwise `~`-aliased when inside the user's home, otherwise absolute.
  `basename`, `basename_without_index`, `ext`, and `parent_dir` return only
  path components, so this policy affects only the larger path shape being
  inspected.
- If parsing or resolution errors, return an evaluation error.
- If resolution returns `Ok(None)`, return an evaluation error.
- Do not call `Path::exists()` or otherwise require the target to exist.
- HTTP(S) URL strings are not local file references for these helpers and must
  return an evaluation error.
- Use platform path semantics from `std::path` for splitting, but return `/` as
  the separator in composed strings for stable Markdown output.
- Examples below use display paths for readability; they must hold exactly when
  `ctx.base_dir` is the directory above `foo`.

Indexed filename grammar:

- A file is indexed only when the final path component's stem ends with a
  hyphen-delimited ASCII decimal suffix.
- Regex-like rule for the basename stem: `(?P<base>.+)-(?P<digits>[0-9]+)`.
- `review-1.md`, `review-100.md`, and `review-001.md` are indexed.
- `review1.md`, `review_1.md`, `review-.md`, and `review--1.md` are not indexed.
- The extension, when present, is everything after the final `.` in the basename.

Functions:

- `is_indexed_file(file) -> boolean`
  - Returns `true` when the resolved basename matches the indexed filename grammar.
- `file_index(file) -> number`
  - Returns the parsed index number.
  - Returns `-1` for a non-indexed filename.
- `increment_file_index(file) -> string`
  - `review-1.md` becomes `review-2.md`.
  - `review-001.md` becomes `review-002.md`.
  - A non-indexed filename starts at index `2`: `review.md` becomes
    `review-2.md`.
  - Added indexes use no zero padding.
- `decrement_file_index(file) -> string`
  - Decrements the index and clamps at `0`.
  - `review-001.md` becomes `review-000.md`.
  - A non-indexed filename starts at index `0`: `review.md` becomes
    `review-0.md`.
- `basename(file) -> string`
  - Returns the final path component including extension.
- `basename_without_index(file) -> string`
  - Removes an indexed suffix from the basename stem.
  - `foo/review-1.md` becomes `review.md`.
  - Non-indexed basenames are returned unchanged.
- `dir(file) -> string`
  - Returns the directory portion of the display path.
- `ext(file) -> string`
  - Returns the final extension without `.`.
  - Returns `""` when no extension exists.
- `parent_dir(file) -> string`
  - Returns the directory segment immediately above the basename.
  - `foo/bar/baz/test.md` becomes `baz`.
  - If there is no parent directory, returns an empty string.
- `file_trailing(file) -> string`
  - Returns the last directory segment and basename.
  - `foo/bar/baz/test.md` becomes `baz/test.md`.
  - If there is no directory segment, returns the basename.
- `dir_leading(file) -> string`
  - Returns the directory path before the last directory segment.
  - `foo/bar/baz/test.md` becomes `foo/bar`.
  - If there is no leading directory, which happens when the file path has no
    directories or only one parent directory, returns an empty string.
- `join(left, right) -> string`
  - Joins two local path strings lexically, then validates the joined result
    through the shared path rules.
  - `left` and `right` must be JSON strings.
  - `left` may be relative, absolute, or a magic path reference.
  - Strip leading separators from `right` before joining so `right` appends to
    `left` rather than replacing it. `join("foo/bar/", "/baz/bax.md")`
    returns `foo/bar/baz/bax.md`.
  - Collapse duplicate separators in the returned string and emit `/`
    separators.
  - Reject HTTP(S) URL arguments.
  - Do not check whether the joined path exists.

## Type Predicate Functions

Register these as pure functions.

- `is_positive(val) -> boolean | Error`
  - Uses the existing `to_number()` coercion accepted by `number()`.
  - Returns `true` only when the coerced value is greater than `0`.
  - Returns an error when coercion fails.
  - `0` is neither positive nor negative.
- `is_negative(val) -> boolean | Error`
  - Same coercion and error behavior as `is_positive`.
  - Returns `true` only when the coerced value is less than `0`.
- `is_integer(val) -> boolean`
  - Inspecting predicate: never errors and does not null-propagate.
  - Returns `true` only for JSON numbers whose numeric value has no fractional
    component.
  - Returns `false` for numberlike strings, booleans, arrays, objects, and null.

## String Mutation and Rendering Functions

Register `without_date`, `ensure_leading`, `ensure_trailing`, and `terminal` as
pure functions. Register `link` as a context-aware `FS_FUNCTIONS` entry.

- `without_date(string) -> string`
  - Requires a JSON string.
  - Removes strict calendar-date substrings in `YYYY-MM-DD` form when they
    parse as real calendar dates.
  - `2026-02-30` is not removed because it is not a real calendar date.
  - Full datetimes are not removed as a single token; only their valid
    `YYYY-MM-DD` substring is removed.
  - Remove only the matched date substring. Do not collapse leftover whitespace,
    punctuation, or duplicate separators; callers can compose this with existing
    string helpers when they want additional cleanup.
  - Compact dates, ordinal dates, and parser-discovered variants are out of scope.
- `ensure_leading(var, prefix) -> string | number`
  - `var` and `prefix` may be JSON strings or JSON numbers.
  - A `null` argument propagates to `null`.
  - Arrays, objects, and booleans raise an error.
  - If the string form of `var` already starts with the string form of `prefix`,
    return `var` unchanged, preserving its original JSON type.
  - If `var` is a JSON number and `prefix` is a JSON number or numberlike string,
    prepend and return a JSON number when the result is representable.
  - Otherwise prepend and return a JSON string.
  - Examples: `ensure_leading("foobar", "foo") -> "foobar"`,
    `ensure_leading("bar", "foo") -> "foobar"`,
    `ensure_leading(123, 4) -> 4123`,
    `ensure_leading("123", 4) -> "4123"`.
- `ensure_trailing(var, postfix) -> string | number`
  - Same type and preservation rules as `ensure_leading`, but for suffixes.
- `link(file) -> string | Error`
  - One-argument `link(x)` is file-only.
  - HTTP(S) URL strings raise an error because URL links require an explicit
    description.
  - Resolve the file through the shared filesystem path rules.
  - The description is `relative(file)` style output from the existing helper.
  - The destination is the resolved absolute path.
  - Use the same destination escaping rules as the two-argument form.
- `link(target, desc) -> string | Error`
  - `desc` must be a JSON string.
  - Accepts either an HTTP(S) URL string or a local file reference.
  - Local file references use the shared filesystem path rules.
  - HTTP(S) destinations are emitted exactly as supplied after URL parsing
    confirms they are valid HTTP(S) URLs.
  - Output is valid Markdown link syntax: `[desc](destination)`.
  - Escape `[` and `]` in link text.
  - Emit destinations in a CommonMark-safe form when spaces, `)`, `<`, `>`, or
    other special characters would otherwise break parsing. Angle-bracket
    destinations and percent encoding are both acceptable.
- `terminal(string) -> string`
  - Requires a JSON string.
  - Render through `biscuit_terminal::components::prose::Prose`.
  - Use a deterministic non-interactive terminal configuration; do not probe or
    mutate the user's live terminal during expression evaluation.
  - Return the rendered terminal string, including ANSI SGR sequences when
    `Prose` emits them.
  - Treat the argument as Prose markup, not untrusted plain text. Callers that
    want literal angle brackets must escape them before calling this function.

## Context Functions

Register these as context-aware functions rather than pure functions. They read
the filesystem and need the document base directory for local-scoped skill roots.

- `has_skill(name) -> boolean`
  - `name` must be a JSON string.
  - Returns whether a direct child directory with that exact basename exists in
    any known user-scoped or local-scoped skill root for the executing agent.
  - Reject names containing path separators or `..`; skill lookup is by
    basename only.
- `has_local_skill(name) -> boolean`
  - Same as `has_skill`, but checks only local-scoped roots.

Skill root discovery:

- Derive the agent from `ctx.agent` when available; otherwise read `AGENT` with
  the same defaulting rules.
- Recognize these agent names and aliases:
  - Claude: `claude`, `claude_code`, `claude-code`
  - OpenCode: `opencode`, `open_code`, `open-code`
  - Codex: `codex`
- User-scoped roots:
  - Claude: `~/.claude/skills`
  - OpenCode: `~/.config/opencode/skill`
  - Codex: `~/.codex/skills`
- Local-scoped roots, resolved from the nearest git root when one is available,
  otherwise from `ResolutionContext.base_dir`:
  - `.claude/skills`
  - `.opencode/skill`
  - `.codex/skills`
  - `.agents/skills`
- Unknown agents check only generic local roots `.agents/skills` and
  `.codex/skills`, then return `false` if no match exists.
- Only direct child directories count. Nested directories and files named like a
  skill do not count.
- Missing roots are normal and return `false`, not an error.
- The library implementation must make user-home and local-root discovery
  injectable for tests. Tests must not mutate or depend on the developer's real
  home directory.

## CLI Documentation

The Claudine CLI lets users see all of the expression engine's functions:
`claudine context --expressions`.

- This is done by an existing anti-drift implementation in Darkmatter that is
  exposed to Claudine.
- This feature must represent the new functions in that shared catalog so
  Claudine picks them up with the rest of the expression surface.
- Add or update a regression test that proves the new descriptor entries appear
  in the exported expression catalog consumed by Claudine. Do not add a
  Claudine-only hardcoded list.

## Documentation and Tests

Update:

- `darkmatter/docs/topics/context-variables.md`
- `darkmatter/docs/topics/darkmatter-expressions.md`
- `.claude/skills/darkmatter/SKILL.md` compose/context summary if this feature
  changes the authoritative public context/function inventory.
- Context variable descriptor catalog
- Expression function descriptor catalog

Required tests:

- Context capture tests for `AGENT`, `MODEL`, missing values, and descriptor
  parity.
- Unit tests for every new function's success, null, type-mismatch, and arity
  behavior.
- File helper tests covering relative paths, magic paths, missing files,
  invalid file references, extensionless names, zero-padded indexes, non-indexed
  names, `join` with leading/trailing separators, display-path rendering, and
  remote URL rejection.
- Link tests covering one-argument file links, two-argument file links,
  two-argument HTTP(S) links, link-text escaping, and destination escaping.
- Skill tests using temporary directory roots; tests must not depend on the
  developer's real home directory.
- End-to-end compose tests proving representative functions work in
  interpolation and `when=` conditions.

## Open Questions

No open questions remain after this review. The main ambiguities in the draft
were resolved here so planning and implementation can proceed without guessing.
