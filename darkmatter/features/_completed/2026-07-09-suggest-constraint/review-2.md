---
ready: false
agent: codex/default
created: 2026-07-10T11:02:11
implemented: true
---

# Review: Suggested Values for SimplifiedSchema (Iteration #2)

## Summary

The iteration-1 fixes are present: root-union selection now carries the selected
arm's provenance, inline span projection is scoped to the `$schema` subtree,
numeric prefix matching uses decoded candidate text, and the in-memory LSP
sessions now run as Level 1 tests. The maintained suggestion-focused suite
passes.

The feature is still not production-ready. Three focused regressions expose
unimplemented or broken acceptance behavior: nested inline-object suggestions
cannot be source-projected, lint identity can collide across properties and
hide valid completions, and completion at a block-array dash fails until a
space has already been inserted.

## Findings

### High: nested inline-object suggestions fail source-aware schema parsing

`Projector::atom` descends into an inline object's child shape before projecting
the outer YAML scalar (`simplified/source.rs:87-109`). For a value such as:

```yaml
$schema:
  settings: "{ mode: string(min(5); suggest(no, valid)) }"
```

the nested `mode` atom is therefore projected while the only available scalar
is the complete `{ mode: ... }` expression. `scalar_matches_candidates` parses
that scalar but inspects only the returned outer atom's constraints
(`source.rs:125-142`), so it cannot find the nested `suggest(...)` and returns a
projection error.

A focused Level 1 regression calling
`parse_standalone_schema_document` failed with
`SchemaDocument { ... "could not project SimplifiedSchema expression spans
through YAML source" }`. This means a structurally valid standalone schema can
fail to load, violating acceptance criteria 11, 12, 14, 15, and 16. On the
inline DMLS path, `inline_lints` converts the same parse failure to
`SuggestionState::Inactive`, so the author receives no invalid-suggestion
warning.

Project nested candidates through the containing scalar's decoded-to-raw map,
retaining the nested expression offset rather than searching for each nested
atom as an independent YAML scalar. Add Level 1 regressions for invalid nested
suggestions in inline Markdown and both standalone envelopes, asserting both
successful schema resolution and exact candidate ranges.

### High: lint matching omits property identity and can suppress valid completion

`SuggestionLintProblem` already carries its dotted `property`, but
`is_lint_problem` matches only root-arm index, property-union index, candidate
value, and candidate span (`simplified/query.rs:202-236`). Candidate spans from
the ordinary parser are expression-relative, so two different properties can
have the same arm indices, interpreted candidate, and relative span.

For example:

```yaml
bad: number(max(0); suggest(1))
good: number(max(1); suggest(1))
```

only `bad` has a lint problem, but a focused Level 1 regression querying
`suggestions_for_path(&schema, &["good"])` returned zero items. The lint from
`bad` was treated as if it belonged to `good`. This violates acceptance
criteria 18 and 19 and can make valid suggestions disappear based on an
unrelated sibling property.

Carry the selected property's identity through the same traversal that now
returns root/property-arm provenance, and include it in lint matching. A
structured path is preferable to a joined dotted string so property names
containing `.` cannot collide. Add sibling, nested, and root-union Level 1
regressions where identical candidate spellings have different constraints.

### High: block-array completion fails at a literal dash before the space exists

`block_array_suggestions` recognizes both `"- "` and `"-"`, but always sets
the edit start to `line_start + indent + 2` (`providers/frontmatter.rs:273-290`).
For a literal `-`, that start is one byte past the cursor. The source-map range
conversion fails and every completion item is dropped.

The maintained `suggest_phase1_bare_block_array_dash` test uses `"- "` and a
cursor after the space (`dmls/tests/suggest_constraint_phase1.rs:374-389`), so
it does not verify the state before the space exists. Changing only that fixture
to `"-"` and moving the cursor back one character produced an empty completion
list in the Level 1 in-memory LSP session.

Compute the edit start from the syntax actually matched: one byte after a bare
dash, two bytes after dash-space. Keep separate Level 1 cases for `-`, `- `, and
`- partial`. This is a user-observable completion requirement under acceptance
criteria 18 and 21, so the current test mismatch is production-blocking.

### Medium: the acceptance matrix still overstates executable coverage

The matrix says every criterion has a passing executable owner, but the three
failures above are absent. In particular, its block-array owner is mislabeled
as covering a bare dash, exact nested source ranges are untested, and no query
test isolates lint provenance between separate properties. The responsible
paths for criteria 19 and 20 also name
`lib/src/markdown/schemas/completion.rs`, which does not exist; the
implementation is `lib/src/markdown/schemas/simplified/query.rs`.

The specification's Definition of Done still says relevant DMLS integration
tests pass with `just test-l2`, while the corrected matrix accurately classifies
these `Connection::memory()` sessions as Level 1. Update the Definition of Done
to require them under `just test`; no real terminal is involved.

Add the missing regressions and correct the implementation paths before the
matrix claims full acceptance coverage. Also add whole-file referenced-envelope
completion tests for both pure and tagged envelopes, which remain absent from
the end-to-end LSP suite.

## Test Rigor Assessment

| User-facing requirement | Strongest observed verification | Assessment |
|---|---:|---|
| Grammar, interpretation, normalization, generated metadata, non-validating behavior, and numeric boundaries (AC 1-13, 23) | Level 1 unit/integration/snapshot | Appropriate level, but source-aware nested schemas violate AC 11-12. |
| Exact inline/standalone diagnostics and envelope behavior (AC 14-17) | Level 1 in-memory LSP plus temp-file resolver tests | Appropriate level, but nested candidate projection is broken and referenced-envelope completion is not exercised. |
| Scalar, nested, array, prefix, insertion, and union completion (AC 18-22) | Level 1 in-memory LSP | Appropriate level, but cross-property lint identity and literal-dash completion fail. |
| Cross-platform behavior (AC 24) | Level 1 CRLF/UTF-8/temp-file tests on macOS | Reasonable host coverage; no OS-specific implementation was introduced, but Windows and Linux were not executed in this review. |

No requirement needs Level 2 real-terminal capture or Level 3 OS keyboard
injection. This feature exposes library and LSP protocol behavior and specifies
no terminal rendering, key encoding, mouse, paste, or IME behavior.

## Verification

- Maintained suggestion-focused nextest filter: 100 passed, 5,646 skipped.
- Review regression for nested source-aware inline-object suggestions: failed
  with a schema-document projection error.
- Review regression for cross-property lint identity: failed; the valid
  property's completion query returned zero items.
- Review regression for a literal block-array `-`: failed in the Level 1
  in-memory LSP session; completion returned no items.
- Temporary review-only test changes were removed after each run.
- `git diff --check` passed.
- Full `just test` and `just lint` were not run in this review.

## Verdict

Not ready for production. Fix nested source projection, make suggestion-lint
identity property-specific, and support completion at a literal block-array
dash. Then add the missing Level 1 regressions and bring the acceptance matrix
and Definition of Done back in sync with the executable suite.
