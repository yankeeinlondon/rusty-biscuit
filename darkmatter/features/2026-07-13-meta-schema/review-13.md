---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T20:29:51-07:00
spec: 2026-07-13-meta-schema/spec.md
log: darkmatter/features/2026-07-13-meta-schema/log.md
implemented: true
implemented_by: codex/default
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-13.md
previous: 2026-07-13-meta-schema/review-12.md
next: 2026-07-13-meta-schema/review-14.md
---

# Review 13 — Meta Schema

## Verdict

This feature is **not ready for production**. Review 12's block-versus-flow
counterexample is fixed, and its parser/claim plus in-memory LSP regressions
pass. A distinct YAML presentation remains unsupported by the source-aware
authority and lexical activation paths: explicit mapping keys (`? key` followed
by `: value`). The source-free YAML and SimplifiedSchema parsers accept this standard
mapping presentation, but the source projector rejects even a valid tagged
schema before DMLS can provide completion or hover. When the definition is
malformed, the lexical claim also returns `None`, so current diagnostics and
last-good assistance disappear. This is a user-observable AC7/AC9 failure with
no Level-1 regression coverage.

The affected gates are otherwise green. Review 12's two focused regressions
passed 2/2, the complete DMLS Level-1 suite passed 631/631, and the DMLS build
and lint gates passed. Review cycle 9's complete terminal gate remains 90/90;
the production changes since then are confined to in-memory DMLS activation
and do not change terminal presentation.

## Findings

### High: Explicit YAML mapping keys disable standalone schema intelligence

The standalone contract accepts a YAML mapping independently of its authored
presentation, but the source locator recognizes only an inline value, a `-`
sequence item, or a same-line `key: value` pair
([source.rs:397](../../lib/src/markdown/schemas/simplified/source.rs#L397),
[source.rs:413](../../lib/src/markdown/schemas/simplified/source.rs#L413)). The
standalone source-aware entry point first parses the semantic declaration and
then requires that locator to recover the envelope payload
([source.rs:168](../../lib/src/markdown/schemas/simplified/source.rs#L168)).
Consequently, this valid tagged standalone schema fails with `could not project
SimplifiedSchema expression spans through YAML source`:

```yaml
? kind
: schema
? types
:
  title: string
```

A removable Level-1 probe established that `serde_yaml_ng` accepts the carrier
and that semantic declaration parsing succeeds before source projection, then
failed on all four configured Nextest attempts because
`parse_standalone_schema_document` returned `SchemaDocument` with the projection
error instead of `Ok(Some(...))`. A removable in-memory LSP probe failed on all
four attempts before its edit step because the valid document produced no
`type-definition` hover.

The malformed-definition path loses activation independently. The block claim
scanner only recognizes one-line `key: value` entries and drops lines without a
same-line colon
([schema.rs:280](../../dmls/src/overlay/schema.rs#L280),
[schema.rs:312](../../dmls/src/overlay/schema.rs#L312)), so the same document
with `title: nope` yields no `Tagged` claim. `OverlayState::for_document` then
exits at `claim?` instead of publishing the current schema diagnostic or
retaining the last-good model
([mod.rs:284](../../dmls/src/overlay/mod.rs#L284),
[mod.rs:313](../../dmls/src/overlay/mod.rs#L313)).

Extend the shared structural source locator to represent explicit mapping
pairs, including their key/value spans and nested payloads, and teach the
malformed-buffer claim scanner the same top-level presentation. Add library
Level-1 parity tests for pure and tagged explicit-key envelopes, asserting the
projected envelope, mapping-key, definition, and type-keyword spans. Add an
in-memory LSP regression that opens the valid tagged document above, verifies
hover and completion, changes `string` to `nope`, and verifies the current
`dm.schema.invalid_type_definition` diagnostic plus retained completion and
hover. This repair crosses CRITICAL shared source-projection code and HIGH DMLS
activation code, so keep the source-free semantic parse product authoritative
and avoid a DMLS-only range reconstruction.

## Prior Review Closure

- **Review 12's block plain-scalar flow indicators — closed.** The scanner now
  distinguishes block plain-scalar content from an actual flow collection. The
  exact parser/claim table and in-memory diagnostics/completion/hover regression
  pass 2/2.
- **Earlier meta-schema findings — remain closed on their reported surfaces.**
  No new gap was found in primitive registration, lowering, custom-keyword
  validation, serialization, semantic arrays, base-schema preparation,
  recursion limits, passivity, schema-file migration, or terminal staging.
  The explicit-key finding is a separate source-presentation gap in the shared
  sidecar and activation layers.

## Requirement Verification Levels

| Requirement | Appropriate level | Strongest verification present | Assessment |
| --- | --- | --- | --- |
| AC1–6: grammar, lowering, validation, serialization, and semantic arrays | Level 1 | Parser, conversion, validation, serialization, property, trigger, compose, and persistence tests | Appropriate and green. |
| AC7: shared passive semantic/source-aware authority | Level 1 | Source-projection tests cover quoted scalars, CRLF, UTF-8, nesting, flow collections, and unions | **Incomplete.** A semantically valid explicit-key mapping is rejected only by source projection. |
| AC8: base schema and declaration preparation | Level 1 | Base-schema, inline/reference/root-union/raw-JSON, repository-schema, dependency, origin, and cache tests | Appropriate and green for covered presentations. |
| AC9: DMLS completion, hover, diagnostics, activation, and last-good state | Level 1 | In-memory overlay and LSP protocol tests | **Incomplete.** Explicit-key standalone schemas provide no valid-buffer assistance, and malformed definitions lose the claim and last-good state. |
| AC10: passive analysis performs no file/process/shell/network side effects | Level 1 | Sentinel integration tests across diagnostics and provider requests | Appropriate and green for implemented request paths. |
| AC11–12: recursion bounds and compatibility | Level 1 | Depth-boundary, baseline replay, import-name, shipped-corpus, repository-schema, property, and downstream tests | Recursion is green; explicit-key standalone compatibility is incomplete because the source sidecar narrows the source-free mapping grammar. |
| AC13: release gates and terminal presentation | Level 2 for real-terminal rendering; Level 1 for protocol semantics | Current DMLS L1 631/631; prior complete L2 aggregate 90/90 | Appropriate for the changed path; the uncovered behavior is Level-1 LSP semantics. |

No requirement concerns OS keyboard encoding, modifier visibility, hotkeys,
paste, IME, or mouse delivery, so Level 3 is not applicable.

## Verification Performed

- The required `darkmatter` skill supplied the meta-schema and review workflow;
  `rust-testing` supplied the L1/L2/L3 classification; `biscuit-file` resolved
  the authored file-reference workflow; and `sniff` identified `darkmatter`,
  `darkmatter-cli`, and `dmls`, with no downstream workspace consumer of
  `dmls`.
- GitNexus reports CRITICAL upstream impact for
  `parse_standalone_schema_payload_with_source` (48 symbols, seven modules) and
  `locate_yaml_value` (30 symbols, six modules), and HIGH impact for
  `standalone_envelope_claim` (41 symbols, four DMLS modules). No production
  symbol was modified during this review.
- Review 12's focused parser/claim and in-memory LSP regressions passed 2/2.
- A removable valid explicit-key parser/source-projection probe failed on all
  four Nextest attempts with the exact projection error above. A removable
  parser/claim probe also failed 4/4 (`None` versus `Some(Tagged)`), and a
  removable in-memory LSP probe failed 4/4 because the valid document produced
  no semantic hover. All probes were removed before final verification.
- The complete affected DMLS Level-1 gate passed 631/631, with three
  higher-tier tests skipped by the Level-1 selector. DMLS build and lint passed
  on macOS. No formatting command was run.
- Review cycle 9's canonical Level-2 gate passed 90/90 (library 18, CLI 69,
  DMLS 3). The current finding concerns in-memory YAML/LSP semantics and does
  not require Level 2 or Level 3 verification.

## Production Readiness

**Not ready.** Support explicit YAML mapping pairs in the shared source sidecar
and malformed-buffer envelope claim, then prove valid-buffer and last-good
diagnostics/completion/hover through library and in-memory LSP Level-1 tests.
The remaining reviewed requirements and affected-package gates are green.
