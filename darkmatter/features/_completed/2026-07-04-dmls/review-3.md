---
ready: false
agent: codex/default
created: 2026-07-08T11:54:22
implemented: true
---

# DMLS Review 3

## Verdict

DMLS is not ready for production. Review 2's reported gaps appear to have been
addressed, and the focused regression tests for nested frontmatter, shell-block
policy, and `::file-links --dir` now pass. I found two remaining Layer 2
frontmatter gaps around schema unions and file-valued navigation.

Verification run:

- `cargo nextest run -p dmls --no-fail-fast --color=never -E 'test(...)'` for the
  Review 2 regression set: 8 passed.
- `just test dmls`: 258 passed, 26 skipped.
- `cd darkmatter && just lint`: passed for `darkmatter`, `darkmatter-cli`, and
  `dmls`.

## Findings

### High - Frontmatter schema intelligence ignores non-first union arms

The spec requires Layer 2 schema completion, hover, and navigation to reflect
the effective SimplifiedSchema. SimplifiedSchema explicitly supports
property-level unions, and the schema library's own completion helper searches
union arms for a completable atom. DMLS instead resolves the "primary" atom as
the first union arm only, then bases value completions, hover, nested object
descent, and `file(...)` navigation on that single arm.

Evidence:

- `darkmatter/docs/topics/schema-definition.md:351` documents property-level
  unions as ordinary schema syntax.
- `darkmatter/lib/src/markdown/schemas/completion.rs:199` searches a
  `PropertyDef::Union` with `atoms.iter().find(...)`, so later file/enum/etc.
  arms are intentionally meaningful for completion.
- `darkmatter/dmls/src/providers/frontmatter.rs:158` gets value completions via
  `def_at_path(...).and_then(primary_atom)`.
- `darkmatter/dmls/src/providers/frontmatter.rs:250` renders schema hover from
  `primary_atom`.
- `darkmatter/dmls/src/providers/frontmatter.rs:357` collects `file(...)`
  navigation targets only when the primary atom is `file`.
- `darkmatter/dmls/src/providers/frontmatter.rs:467` descends nested inline
  objects only through `primary_atom`.
- `darkmatter/dmls/src/providers/frontmatter.rs:493` defines `primary_atom` as
  `atoms.first()`.
- Current LSP tests cover single-arm scalar and single-arm nested object cases,
  but no property-level union case.

Impact: valid schemas such as this lose v1 editor behavior:

```yaml
$schema:
  asset:
    - string
    - file
  settings:
    - string
    - mode: enum(dev, prod)
asset: other.md
settings:
  mode: dev
```

`asset` will not offer file path completion, definition, or document links when
the `file` arm is second, and `settings.mode` will not offer nested key/value
intelligence when the inline object arm is second. Diagnostics may still
validate because validation uses the schema library, so the editor behavior
silently diverges from the schema authority.

Required fix: replace the single `primary_atom` path with capability-specific
arm selection: first completable atom for value completion, any matching file
arm for navigation/document links, and a deterministic inline-object arm
selection/merge policy for nested key lookup. Add LSP-session tests for union
arms where the completable/file/inline-object arm is not first.

Verification level: the strongest current coverage is Level 1 unit coverage and
in-process LSP session coverage for non-union schemas. The union-arm
user-observable requirement has no coverage and is implemented incorrectly.

### High - Valid extensionless `file(...)` values are dropped by a path-looking heuristic

Layer 2 navigation requires `$schema` file references and `file(...)` values to
produce definitions and document links, and the graph substrate is supposed to
materialize frontmatter `file(...)` values as `uses_file` edges. After DMLS has
already established that a scalar is schema-typed as `file`, it still rejects
values that do not contain `/` or `.`, so valid extensionless local files such as
`LICENSE`, `Makefile`, or `README` are ignored.

Evidence:

- `darkmatter/dmls/src/providers/frontmatter.rs:360` only records a
  `file(...)` navigation target if `looks_like_path(value)` is true.
- `darkmatter/dmls/src/providers/frontmatter.rs:535` recognizes the schema atom
  as `SimplifiedType::File`, but `looks_like_path` still applies afterward.
- `darkmatter/dmls/src/graph/substrate.rs:369` applies the same heuristic before
  adding inline-schema frontmatter file uses to the graph.
- `darkmatter/dmls/src/graph/substrate.rs:487` defines the heuristic as requiring
  a slash or dot.
- Existing tests cover `top.md`, `nested.md`, and `docs` for `::file-links
  --dir`, but no schema-typed frontmatter value like `LICENSE`.

Impact: users cannot go to definition or click document links for valid
schema-typed file values that happen to be extensionless, and the graph misses
their `uses_file` invalidation dependency. This is narrower than the union-arm
issue, but it still violates the "all file-valued entries" Layer 2 contract.

Required fix: once the effective schema says a scalar is `file`, resolve it via
the same `FileReference` semantics used by Darkmatter/biscuit-file instead of a
dot/slash heuristic. Keep URL/inline-object rejection, but do not reject bare
relative filenames. Add provider and graph tests for an extensionless file.

Verification level: the strongest current coverage is Level 1 graph tests and
in-process LSP tests for dotted file names. The extensionless-file case has no
coverage and is broken.

## Notes

- Review 2 items are materially improved: `level2_frontmatter_nested_schema_intelligence`,
  `level2_frontmatter_nested_file_navigation`, shell-block diagnostics/hover
  tests, and `::file-links --dir` graph tests all passed.
- The prompt's terminal Level 2/3 requirements do not map directly to this LSP
  server's protocol behavior; no terminal encoder/decoder UX is under review.
  For DMLS, the meaningful higher-confidence launch check is the real stdio
  subprocess test, and the request/response behaviors are covered through
  in-process LSP sessions. The two findings above need those request-level tests,
  not real-terminal or OS-keyboard injection.
