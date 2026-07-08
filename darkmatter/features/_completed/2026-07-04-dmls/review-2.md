---
ready: false
agent: codex/default
created: 2026-07-07T18:55:10
implemented: true
---

# DMLS Review 2

## Verdict

DMLS is not ready for production. The second iteration fixed several Review 1
items, but the implementation still misses committed v1 behavior in
frontmatter, shell-block analysis, and graph substrate coverage.

Verification run:

- `just test` from `darkmatter/` compiled and started nextest, but was
  interrupted after the non-interactive safety window. At interruption,
  1,822/5,064 tests had passed, 111 were skipped, and 3,242 had not run.

## Findings

### High - Nested frontmatter schema intelligence is not implemented

The spec requires Layer 2 completion and hover for nested object keys, enum
values, boolish scaffolds, file paths, and `style.*` keys. The current
frontmatter provider only understands top-level schema properties, plus a
special case for direct children of `style:`. It does not walk inline-object
schema shapes for ordinary nested mappings, so a schema like
`settings: { mode: enum(dev, prod), path: file }` will not offer `mode`, enum
values, file completions, or schema hover under `settings:`.

Evidence:

- `darkmatter/dmls/src/providers/frontmatter.rs:52` routes value completions by
  the current line's leaf key string only.
- `darkmatter/dmls/src/providers/frontmatter.rs:58` calls
  `top_level_key_completions` only when indent is zero.
- `darkmatter/dmls/src/providers/frontmatter.rs:59` handles indented key
  completion only when the immediate parent is literally `style`.
- `darkmatter/dmls/src/providers/frontmatter.rs:96` documents
  `value_completions` as top-level and looks up `shape.properties.get(key)`.
- `darkmatter/dmls/src/providers/frontmatter.rs:194` returns no schema hover
  for `entry.depth != 0`.
- The L2 frontmatter tests cover an inline top-level schema and a top-level
  Claudine extension key, but not nested schema objects.

Impact: users editing nested schema-backed frontmatter get no v1 intelligence
even though diagnostics may still validate those fields. This is a direct
Layer 2 functionality gap.

Required fix: resolve the current YAML path from `FrontmatterAst`, walk
`TypeExpr::InlineObject` / union shapes through the effective schema, and use
that nested property definition for key completion, value completion, hover,
file document links, and definition. Add LSP-session tests for nested key
completion, nested enum value completion, nested `file(...)` navigation, and
nested hover.

Verification level: strongest current coverage is Level 1/LSP-in-memory tests
for top-level frontmatter behavior. The nested user-observable requirement has
no coverage, so this remains a high-severity readiness gap.

### High - `::shell-block` is cataloged but not analyzed for shell policy

The spec's Layer 3 shell-awareness requirement covers `::shell`,
`::shell-block`, and frontmatter `$()`: hover should show a read-only policy
verdict, and diagnostics should flag policy-disallowed commands. The
implementation covers `::shell` and frontmatter `$()`, but `::shell-block`
gets only catalog hover/folding/block-pairing. Its body commands are not parsed
for hover or `dm.security.disallowed_command`.

Evidence:

- `darkmatter/dmls/src/overlay/directives.rs:67` catalogs `::shell-block` as
  policy-gated.
- `darkmatter/lib/src/markdown/compose/directives_api.rs:248` classifies
  `DirectiveKind::ShellBlock` as keyword-only in the span scanner, so no
  command target reaches the provider from the directive line.
- `darkmatter/dmls/src/providers/dsl.rs:233` appends shell policy hover only
  for `DirectiveKind::Shell`.
- `darkmatter/dmls/src/providers/dsl.rs:631` documents shell diagnostics as
  `::shell` + frontmatter `$()`.
- `darkmatter/dmls/src/providers/dsl.rs:636` skips every directive whose kind
  is not `DirectiveKind::Shell`.
- Tests mention `shell-block` only for directive completion/folding validity,
  not for policy hover or security diagnostics.

Impact: a dangerous command inside `::shell-block ... ::end-block` can avoid
the v1 security diagnostic and policy explanation. That violates the passive
analysis contract for one of the shell surfaces authors are explicitly told is
covered.

Required fix: expose shell-block command/body spans from the passive scanner or
block scanner, then reuse `shell_verdict_markdown` and
`dm.security.disallowed_command` diagnostics over those spans. Add tests with a
disallowed shell-block command and a whitelisted/unknown shell-block command.

Verification level: strongest current coverage is in-memory LSP coverage for
`::shell` only. The `::shell-block` user-observable requirement has no matching
test and the implementation is absent.

### High - `::file-links --dir ...` is not materialized as a file-use edge

The design and implementation comments say `uses_file` covers
`::file-links`/`::toc-linking` directive paths. The scanner correctly treats
the documented `::file-links --dir <path>` syntax as option form with no
positional target, but the graph substrate records only `directive.target`.
That drops the directory path for the common `--dir` form.

Evidence:

- `darkmatter/docs/inline/file-links.md` documents
  `::file-links --dir <path> [--depth <u32>]` as supported syntax.
- `darkmatter/lib/src/markdown/compose/directives_api.rs:230` states flag-form
  `::file-links` carries no positional target.
- `darkmatter/dmls/src/graph/substrate.rs:297` immediately skips any directive
  without `directive.target`.
- `darkmatter/dmls/src/graph/substrate.rs:313` records
  `DirectiveKind::FileLinks | DirectiveKind::TocLinking` only from that target.
- `darkmatter/dmls/src/graph/substrate.rs:105` and `:165` claim `file_uses`
  includes `::file-links` paths.
- Existing DMLS tests only assert `::file-links` appears in directive-name
  completion; there is no graph test for `--dir`.

Impact: invalidation fan-out over `uses_file`, dependency analysis, and any
future navigation/rename/code-action work relying on the graph will miss
directory-form file-links dependencies. This also means the graph does not
carry every committed file-use source.

Required fix: when indexing `DirectiveKind::FileLinks`, inspect parsed options
for `--dir` and materialize its value span as a `FileRefFact`; retain the
positional target path for glob form. Add graph tests for both
`::file-links docs/*.md` and `::file-links --dir docs --depth 2`.

Verification level: current coverage is Level 1 graph tests for other file-use
surfaces and directive completion. The `--dir` graph requirement is uncovered
and broken.

### Medium - `file(...)` navigation/document links only handle top-level values

Layer 2 navigation requires `$schema` file references and `file(...)` values to
produce definitions and document links. The implementation uses only
`ast.top_level()` when collecting `file(...)` navigation targets, so nested
schema-backed file properties are ignored even if the effective schema declares
them.

Evidence:

- `darkmatter/dmls/src/providers/frontmatter.rs:279` says nav targets include
  every `file(...)`-typed scalar value.
- `darkmatter/dmls/src/providers/frontmatter.rs:296` narrows that to
  "top-level scalar values".
- `darkmatter/dmls/src/providers/frontmatter.rs:298` iterates
  `ast.top_level()` and looks up `shape.properties.get(&entry.key)`.

Impact: users cannot jump to or click nested file-valued frontmatter entries,
which are valid under SimplifiedSchema inline objects. This is related to the
nested-intelligence finding, but it affects navigation/document-link
capabilities specifically.

Required fix: share the nested schema-path resolver from the frontmatter
completion fix and collect scalar entries at all depths whose schema atom is
`SimplifiedType::File`.

Verification level: current coverage exercises no nested `file(...)` value, so
the strongest test for this requirement is missing.

## Notes

- The current DMLS test suite is valuable, but most user-observable behavior is
  still tested through in-memory LSP sessions. For this LSP server, Level 3
  keyboard injection is not the right bar; however, subprocess stdio tests are
  still important for launch/protocol behavior, and nested/DSL cases above need
  request-level coverage.
- I did not see a need to amend the spec downward. The implementation should
  grow to match the v1 contract, especially because the feature document marks
  all 11 acceptance criteria as delivered.
