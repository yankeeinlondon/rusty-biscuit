---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T00:20:54-07:00
spec: 2026-07-13-meta-schema/spec.md
implemented: true
implemented_by: claude/default
log: darkmatter/features/2026-07-13-meta-schema/log.md
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-5.md
previous: 2026-07-13-meta-schema/review-4.md
---

# Review 5 — Meta Schema

## Verdict

This feature is **not ready for production**. Review 4's complete structural
path matching and reference-graph cycle/dependency findings are substantially
fixed, and all Level-1 tests, builds, and lints pass across the affected package
area. The feature-relevant Level-2 evidence also remains green.

One high-severity DMLS completion gap remains: a valid standalone schema whose
pure envelope is a scalar file reference activates successfully for parsing and
diagnostics, but its top-level `$schema` value cannot receive the required
schema/file-reference completion. The new reference-depth guard is also
untested at its boundary and reports an acyclic over-depth chain as a cycle with
incorrect recovery advice. Finally, the canonical Level-2 gate remains red and
the specification explicitly says its proposed exception is not ratified.

## Findings

### High: Standalone scalar `$schema` references cannot receive completion

The specification requires completion inside `schema` values, including passive
file-reference completion, and says that completion must support standalone
documents recognized by `parse_standalone_schema_document`
([spec.md:531](spec.md#L531), [spec.md:538](spec.md#L538)). A pure standalone
scalar is one of those valid, active documents:

```yaml
$schema: ./other.yaml
```

Parsing and activation handle that form correctly. The Level-1 declaration
parity test explicitly asserts that the scalar document is valid and active
([lsp_session.rs:3967](../../dmls/tests/lsp_session.rs#L3967)). Completion loses
the activation at the consumer boundary. `enclosing_path` deliberately returns
an empty path for any top-level line
([frontmatter.rs:1765](../../dmls/src/providers/frontmatter.rs#L1765)), while
the `Standalone::Pure` branch of `meta_schema_kinds_for_line` requires the first
ancestor to be `$schema` and ignores the current `key`
([frontmatter.rs:170](../../dmls/src/providers/frontmatter.rs#L170)). For the
top-level scalar line, `ancestors` is empty and `key` is `$schema`, so the
provider returns `None` before `schema_value_completions` can offer path
candidates.

The standalone completion regression covers a mapping payload, a block root
union, and a tagged mapping, but not the scalar reference form
([lsp_session.rs:3411](../../dmls/tests/lsp_session.rs#L3411)). This leaves the
user-observable completion requirement in acceptance criterion 9 with no test
at any level, and the implementation fails by inspection. The appropriate
verification is Level 1 because this is LSP protocol behavior, not terminal
rendering or keyboard encoding.

Treat a top-level, non-sequence `$schema` value in a pure standalone envelope as
`MetaSchemaKind::Schema`, just as the frontmatter branch already does. Add
Level-1 in-memory LSP tests for empty, partial, and complete scalar references,
LF and CRLF, and last-good retention during a malformed edit.

### Medium: The delegation depth boundary is untested and reports the wrong failure

The new `ReferenceStack` correctly prevents cycles and caps reference delegation
at 32 open files. However, exceeding that cap returns
`SchemaError::ReferenceCycle`
([resolve.rs:300](../../lib/src/markdown/schemas/resolve.rs#L300),
[resolve.rs:325](../../lib/src/markdown/schemas/resolve.rs#L325)). Its public
error text therefore says “cycle detected,” and terminal rendering tells the
user to “Break the loop,” even when all 33 files are distinct and the graph is
acyclic ([errors.rs:121](../../lib/src/markdown/schemas/errors.rs#L121),
[errors.rs:461](../../lib/src/markdown/schemas/errors.rs#L461)). That diagnosis
and recovery advice are false for the depth-limit case.

The new graph suite tests multi-hop success, self/two-file/root-union cycles,
and a non-cyclic diamond, but never reaches `MAX_REFERENCE_DEPTH`
([meta_schema_reference_graph.rs:32](../../lib/tests/meta_schema_reference_graph.rs#L32)).
The module claims to pin the “cycle/depth guard,” yet an implementation could
move or remove the depth check without failing the suite.

Use a distinct structured depth-limit error, or otherwise render cycle and
depth failures with accurate messages. Add Level-1 boundary tests proving the
largest permitted acyclic chain resolves, the next hop fails without panic,
and the reported diagnostic describes depth rather than a loop. Include a
root-union path so both recursive entry routes remain bounded.

### Medium: The canonical Level-2 gate remains red and its exception is not approved

Acceptance criterion 13 requires `just test-l2` to pass. The post-fix
implementation record reports library 18/18, CLI 66/69, and DMLS 3/3, with the
same three unrelated code-block staging failures named in the specification
([log.md:376](log.md#L376)). The focused `schema about` and DMLS slices have the
correct real-terminal coverage and pass.

The proposed exception is well scoped and its evidence is persuasive, but the
specification explicitly says it is **proposed, awaiting ratification, and does
not relax AC13** ([spec.md:750](spec.md#L750)). A review cannot treat it as
approved. Restore the canonical gate or obtain the documented ratification
before marking the feature production-ready.

## Prior Review Closure

- **Complete structural completion paths — closed.** Completion now matches the
  complete decoded authored path against decoded RFC 6901 pointer segments.
  New Level-1 tests cover nested semantic values, `/` and `~` keys, arrays,
  LF/CRLF, and malformed-buffer last-good behavior.
- **Reference cycles and transitive dependencies — substantially closed.** A
  canonical-path frame stack prevents self, multi-file, and root-union cycles;
  dependencies accumulate across every hop; terminal origin attribution is
  preserved; and DMLS invalidates its cache when the terminal file changes.
  The separate depth-boundary diagnostic and coverage issue remains as the
  second finding.
- **Canonical Level-2 release gate — open.** The same three code-block staging
  tests fail, and the proposed exception remains unratified.

## Requirement Verification Levels

| Requirement | Appropriate level | Strongest verification present | Assessment |
| --- | --- | --- | --- |
| AC1–6: grammar, lowering, validation, serialization, semantic arrays | Level 1 | Library/CLI unit and integration tests, property tests, and filesystem round trips | Appropriate and passing. |
| AC7: shared passive semantic/source-aware authority | Level 1 | Parser-parity, source-projection, structural-path, and malformed-buffer tests | Appropriate and passing. |
| AC8: base schema and referenced declaration preparation | Level 1 | Base-schema, multi-hop, cycle, dependency, origin, and cache-invalidation tests | Core graph behavior passes; the depth boundary is unverified and misdiagnosed. |
| AC9: DMLS completion, hover, diagnostics, activation, last-good state | Level 1 | In-memory LSP tests | Correct tier for protocol behavior, but standalone scalar-reference completion has no verification and fails. |
| AC10: passive analysis performs no file/process/shell/network side effects | Level 1 | Sentinel integration tests | Appropriate and passing. |
| AC11–12: grammar recursion bounds and compatibility | Level 1 | Grammar-depth, baseline replay, import-name, and downstream tests | Appropriate and passing. The separate file-delegation cap is not AC11's semantic-grammar depth limit. |
| AC13: terminal/editor presentation and area release gate | Level 2 for rendering; Level 1 for protocol semantics | 18 library, 3 `schema about`, and 3 DMLS Level-2 tests pass; canonical CLI tier is 66/69 | Feature slices use the right levels, but the declared full gate is red and its exception is unratified. |

No requirement concerns OS keyboard encoding, modifier visibility, hotkeys,
paste, IME, or mouse delivery, so Level 3 is not applicable. The missing scalar
completion and depth-boundary cases belong at Level 1; promoting them to Level
2 or Level 3 would not test a more relevant boundary.

## Verification Performed

- The requested `@prompts/./_reviews/.../review-4.md` path does not exist. The
  canonical previous review is the colocated
  `darkmatter/features/2026-07-13-meta-schema/review-4.md`, which was used for
  the review-chain update.
- `sniff` identified `darkmatter`, `darkmatter-cli`, and `dmls` as the affected
  package-area crates.
- GitNexus reports HIGH upstream impact for the reviewed reference resolver
  (`resolve_reference`: 56 affected symbols; `load_guarded`: 21) and LOW impact
  for `meta_schema_kinds_for_line` (24 affected symbols). No Rust symbol was
  edited during this review.
- Eight bounded `just test` partitions passed: Darkmatter **5,926/5,926**,
  Darkmatter CLI **561/561**, and DMLS **614/614**. Eleven focused regressions
  for structural completion, standalone declaration parity, reference graphs,
  and terminal-file cache invalidation also passed.
- `just build` and `just lint` passed for all three affected crates. No
  formatting command was run.
- The implementation record's post-change Level-2 rerun remains library
  **18/18**, CLI **66/69**, and DMLS **3/3**; the only failures are the three
  documented code-block staging tests. This review did not substitute the
  passing feature slices for the red canonical gate.
- `md schema validate` accepts the specification. Review 5 cannot be validated
  against `feature-review.yaml` because that existing tagged schema envelope
  contains unsupported `description` and `$schema` keys, the same schema
  infrastructure drift recorded by prior reviews.
- No Rust source was modified during this review.

## Production Readiness

**Not ready.** Restore standalone scalar-reference completion and add its
Level-1 regressions, distinguish and test reference-depth exhaustion, and
either restore the canonical Level-2 gate or obtain ratification of the scoped
exception before setting `ready: true`.
