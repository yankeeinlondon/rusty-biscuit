---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T08:31:34-07:00
spec: 2026-07-13-meta-schema/spec.md
implemented: true
implemented_by: claude/default
log: darkmatter/features/2026-07-13-meta-schema/log.md
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-2.md
previous: 2026-07-13-meta-schema/review-1.md
---

# Review 2 — Meta Schema

## Verdict

This feature is **not ready for production**. The shared library grammar,
semantic types, carrier lowering, base-schema migration, and passive-validation
boundary are substantially implemented, and the focused test populations pass.
However, the DMLS implementation bypasses the source-aware parser authority for
completion and diagnostic ranges, mishandles mixed unions containing a semantic
meta-type, and emits the wrong diagnostic contract for invalid standalone outer
declarations. Local schema references also are classified after trimming but
constructed from the untrimmed value.

## Findings

### High: DMLS reconstructs parser state and ranges from text instead of consuming the required sidecar

The specification requires DMLS to consume the shared structural sidecar for
hover, completion context, and diagnostics and explicitly prohibits searching
decoded text to reconstruct ranges ([spec.md:250](spec.md#L250)). Acceptance
criteria 7 and 9 make that shared parser authority and parser-state completion
part of the production contract ([spec.md:723](spec.md#L723),
[spec.md:732](spec.md#L732)).

The completion provider instead derives context from line indentation and string
searches. `meta_schema_completion` uses `line_prefix`, `enclosing_path`, and
`semantic_value_token`; the latter splits only on `,` and `[`
([frontmatter.rs:78](../../dmls/src/providers/frontmatter.rs#L78),
[frontmatter.rs:171](../../dmls/src/providers/frontmatter.rs#L171)). Constraint
completion uses `partial.rfind('(')` and catalog substring matching rather than
the schema parser's current structural path
([frontmatter.rs:184](../../dmls/src/providers/frontmatter.rs#L184)). Diagnostics
likewise reparse extracted YAML fragments and scan raw bytes for `[]` while
trying to locate the smallest invalid node
([frontmatter.rs:221](../../dmls/src/diagnostics/frontmatter.rs#L221),
[frontmatter.rs:323](../../dmls/src/diagnostics/frontmatter.rs#L323)).

This is already observably incomplete:

- `string(pattern(^a); re` cannot complete a second constraint because the
  innermost `(` belongs to `pattern(...)`, so the heuristic no longer recognizes
  the outer constraint list.
- Array-level completion such as `type-definition[](mi` cannot offer the valid
  array constraints because lookup strips `[]` and filters against the semantic
  atom descriptor, whose accepted constraints are only `default`, `required`,
  and `generated`
  ([about.rs:114](../../lib/src/markdown/schemas/about.rs#L114)).
- Inline-object and nested-union states such as `{ child: str` are not structural
  tokens under the comma/bracket splitter, so the expected type-keyword
  completion is absent.

The existing Level-1 completion tests cover an empty value, a leading keyword,
the first constraint, quoting, array items, and top-level scaffolds, but not a
second constraint, postfix array constraints, or a partially authored inline
object. Replace these heuristics with a tolerant parser-state API backed by the
same grammar/source-map authority, and add Level-1 LSP regressions for each
state above, including UTF-8, CRLF, and quoted scalar projections.

### High: A semantic arm in a valid mixed union produces false diagnostics and suppresses sibling completions

Frontmatter activation intentionally records `type-definition` or `schema` when
any effective union arm contains that type
([schema.rs:159](../../dmls/src/overlay/schema.rs#L159)). The semantic diagnostic
pass then attempts only those recorded semantic kinds and emits a specialized
error whenever they reject the authored value
([frontmatter.rs:221](../../dmls/src/diagnostics/frontmatter.rs#L221)). Although
the function receives the complete `ValidationReport`, it uses that report only
to attach related information; it does not first establish that every union arm
rejected the value. The expression diagnostic path already demonstrates the
correct union-wide gating policy
([frontmatter.rs:521](../../dmls/src/diagnostics/frontmatter.rs#L521)).

Consequently, this valid document is diagnosed incorrectly:

```yaml
---
$schema:
  value: [type-definition, string]
value: hello
---
```

The `string` arm accepts `hello`, but DMLS emits
`dm.schema.invalid_type_definition`. Completion has the parallel defect:
`completion` returns immediately when `meta_schema_completion` activates, so
ordinary sibling-arm candidates are discarded
([frontmatter.rs:47](../../dmls/src/providers/frontmatter.rs#L47)). A property
typed `[type-definition, enum(foo,bar)]`, for example, cannot offer `foo` and
`bar` through the normal union merge.

Add Level-1 LSP tests in both arm orders for valid and invalid values, and merge
semantic completion candidates with every effective sibling arm. Emit the
specialized diagnostic only when the validation report proves the whole union
failed at that instance path.

### High: Invalid standalone outer declarations use the wrong code and whole-document range

The diagnostic contract says malformed inner definitions use
`dm.schema.invalid_type_definition`, while invalid outer declarations retain
`dm.schema.invalid_schema_shape`. The standalone fallback instead ranges the
entire buffer and emits `dm.schema.document_malformed`
([frontmatter.rs:740](../../dmls/src/diagnostics/frontmatter.rs#L740)). Its
specialized helper handles only mapping entries that can be passed to
`parse_property_definition`; invalid outer root-union arms, empty unions, and
invalid reference declarations therefore fall through to the generic code
([frontmatter.rs:759](../../dmls/src/diagnostics/frontmatter.rs#L759)).

This violates acceptance criterion 9's precise-diagnostic requirement and loses
the stable code clients use to distinguish declaration shape from document YAML
failure. Use the source-aware schema-declaration error path to emit
`dm.schema.invalid_schema_shape` over the smallest invalid outer value or union
arm. Add Level-1 standalone LSP cases for an empty root union, an invalid scalar
arm, and an invalid local reference, plus a separate malformed-YAML case that
continues to use `dm.schema.document_malformed`.

### Medium: Trimmed schema references are syntax-checked and resolved with different strings

The specification requires file-reference strings to be trimmed before they are
syntax-checked ([spec.md:351](spec.md#L351)). The classifier computes `trimmed`
for remote and bare-name classification, but constructs `FileReference` from
the original `reference`
([reference.rs:50](../../lib/src/markdown/schemas/reference.rs#L50)). A quoted
declaration such as `" ./schemas/post.yaml "` can therefore pass semantic
classification as path-qualified while later resolving a path that still
contains leading and trailing spaces. Whitespace-only values also reach
`FileReference::new` rather than being rejected as an empty trimmed reference.

Construct and report the canonical local reference consistently from `trimmed`,
and add parser/validator/resolver parity tests for leading/trailing whitespace
and whitespace-only strings. Keep the operation passive; these tests should
continue to prove no existence check or file I/O occurs during classification.

## Prior Review Closure

Review 1's three recorded design rulings are present in the specification and
plan: semantic arrays use ordinary postfix lowering, source-aware parsing uses a
sidecar rather than a second AST, and YAML-native mappings share
`MAX_INLINE_OBJECT_DEPTH`. The library implementation and tests cover those
decisions. The first finding above shows that the sidecar ruling is not closed
end-to-end because DMLS still implements its own text-derived parser state and
range recovery.

The requested previous-review target,
`@prompts/./_reviews/darkmatter/features/2026-07-13-meta-schema/review-1.md`, does
not exist in this worktree or repository history. No canonical
`darkmatter/features/2026-07-13-meta-schema/review-1.md` exists either. Its
`next` and `implemented` properties therefore could not be updated without
inventing a historical artifact. Commit `587236e11` preserves the three Review
1 rulings in the specification and plan, but does not contain the review file.

## Requirement Verification Levels

| Requirement | Appropriate level | Strongest verification present | Assessment |
| --- | --- | --- | --- |
| AC1–6: grammar, carrier lowering, validation, serialization, and semantic arrays | Level 1 | Level-1 library and CLI integration tests | Partial. Core cases pass; AC3/4 reference trimming is inconsistent and AC6's array constraint completion is missing. |
| AC7: shared semantic/source-aware parser parity and structural spans | Level 1 | Level-1 parser-parity and span-projection tests | Library coverage is appropriate, but DMLS does not consume the verified sidecar authority. |
| AC8: base-schema behavior and `$schema` hover identity | Level 1 | Level-1 preparation and in-memory LSP tests | Appropriate and passing. This is protocol content, not terminal rendering. |
| AC9: completion, hover, diagnostics, activation, and last-good behavior | Level 1 | Level-1 in-memory LSP tests | Level is appropriate, but important parser states, mixed semantic unions, and invalid standalone outer declarations are unverified and broken. |
| AC10: no file, process, expression, shell, or network side effects | Level 1 | Level-1 sentinel integration tests | Appropriate and passing. |
| AC11–12: depth bound and compatibility | Level 1 | Level-1 boundary, baseline-replay, and downstream tests | Appropriate and passing for the exercised cases. |
| AC13: CLI terminal presentation and editor integration | Level 2 where presentation is asserted; Level 1 for semantic protocol content | Three `schema about` real-terminal tests and three DMLS Neovim/tmux tests, plus Level-1 semantic tests | Appropriate. The relevant Level-2 tests pass; no meta-schema requirement depends on the terminal's keyboard encoder. |

No requirement concerns keypresses, hotkeys, paste, IME, mouse input, or another
OS keyboard-encoding path, so Level 3 is not applicable. There is no case where
a user-observable requirement is tested only below the level its mechanism
requires; the production blockers are missing cases and incorrect behavior at
the appropriate Level-1 boundary.

## Verification Performed

- `FileReference` resolved the specification to the canonical feature path. It
  could not resolve either the requested previous-review target or a canonical
  Review 1 artifact; repository history confirmed neither file ever existed.
- `sniff` mapped the affected area to `darkmatter`, `darkmatter-cli`, and `dmls`,
  with Claudine packages among the downstream consumers.
- GitNexus reported **CRITICAL** upstream blast radius for both shared parser
  entry points: `parse_property_definition` affects 76 indexed symbols across
  seven modules, and `parse_schema_declaration` affects 69 symbols across six
  modules. `meta_schema_completion` was medium risk with 39 impacted symbols.
- Darkmatter focused meta-schema Level 1: **31/31 passed** across phases 1,
  3–6, source projection, and grammar proptests.
- DMLS Level 1: **61/61 passed** across `lsp_session` and `no_side_effects`.
- Darkmatter CLI Level 1: **14/14 passed** for `schema_about`; one test first
  reported leaked handles and passed on the configured nextest retry, so the
  result is green but not fully deterministic.
- Darkmatter CLI Level 2: **3/3 passed** for real-terminal `schema about`
  rendering.
- DMLS Level 2: **3/3 passed** in Neovim/tmux. These are editor/rendering
  regressions; the meta-schema semantic assertions remain Level 1.
- The implementation record reports all 19 Darkmatter library Level-2 tests
  passing. The canonical full CLI Level-2 gate remains red on two documented,
  pre-existing terminal-theme luminance assertions unrelated to this feature.
- `md get` read back every required Review 2 frontmatter value exactly, and the
  specification passed `md schema validate`. The review cannot currently be
  schema-adjudicated because the repository's existing
  `schemas/feature-review.yaml` is rejected as a standalone tagged schema: its
  `description` and `$schema` keys are unsupported. This is existing schema
  infrastructure drift, not a meta-schema implementation result.
- No formatting command was run and no Rust source was modified during this
  review.

## Production Readiness

**Not ready.** Close all three high-severity DMLS findings, normalize trimmed
schema references end-to-end, and add the specified Level-1 regressions before
setting `ready: true`. The current Level-2 results do not compensate for
incorrect semantic protocol behavior.
