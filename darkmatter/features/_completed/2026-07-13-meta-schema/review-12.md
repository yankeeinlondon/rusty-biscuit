---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T20:00:12-07:00
spec: 2026-07-13-meta-schema/spec.md
log: darkmatter/features/2026-07-13-meta-schema/log.md
implemented: true
implemented_by: codex/default
next: 2026-07-13-meta-schema/review-13.md
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-12.md
previous: 2026-07-13-meta-schema/review-11.md
---

# Review 12 — Meta Schema

## Verdict

This feature is **not ready for production**. Review 11's exact flow-style
`foo- "bar` counterexample is fixed, with parser/claim parity and in-memory LSP
coverage passing. The same lexical activation scanner still applies flow-only
indicator rules while scanning block-style plain scalars, however. A valid
block YAML scalar containing a mid-token `{` followed by a quote can therefore
hide a later `kind: schema` key, discard the last-good standalone model, and
remove current diagnostics, completion, and hover. This is another
user-observable AC9 failure with no Level-1 regression.

The affected DMLS gates are otherwise green: Review 11's focused regressions
passed 2/2, the complete DMLS Level-1 suite passed 629/629, DMLS Level 2 passed
3/3, and the DMLS build and lint gates passed. The canonical Level-2 aggregate
passed all 18 library tests and the first 46 CLI tests before it was interrupted
at the required non-interactive 60-second ceiling; the prior complete aggregate
remains 90/90, and Review 11 changed only the DMLS lexical scanner and its
Level-1 tests.

## Findings

### High: Flow-only indicators still open quote state inside block plain scalars

`block_top_level_entries` advances cross-line quote state for every top-level
line ([schema.rs:311](../../dmls/src/overlay/schema.rs#L311)). Its shared
boundary helper unconditionally treats `[`, `{`, and `,` as flow indicators
that begin a fresh scalar position
([schema.rs:401](../../dmls/src/overlay/schema.rs#L401),
[schema.rs:404](../../dmls/src/overlay/schema.rs#L404)). That is valid while
scanning a flow collection, but not after a block-style plain scalar has already
begun. In block context these characters can be scalar content.

This document is valid YAML, claims a tagged standalone schema envelope, and
contains an invalid SimplifiedSchema definition:

```yaml
description: foo{ "bar
kind: schema
types:
  title: nope
```

The authoritative YAML parser accepts `foo{ "bar` as one plain scalar, and
`parse_standalone_schema_document` recognizes the tagged envelope and returns
its structured schema-authoring error. The lexical scanner instead makes `{`
set `at_scalar_start`, keeps that state across the following space, and opens
quote state at `"` ([schema.rs:382](../../dmls/src/overlay/schema.rs#L382),
[schema.rs:388](../../dmls/src/overlay/schema.rs#L388)). The unclosed lexical
quote then causes the later `kind` and `types` lines to be skipped as scalar
continuations ([schema.rs:292](../../dmls/src/overlay/schema.rs#L292)).
`standalone_envelope_claim` returns `None`, and the overlay's parser-error path
exits at `claim?` ([mod.rs:284](../../dmls/src/overlay/mod.rs#L284),
[mod.rs:314](../../dmls/src/overlay/mod.rs#L314)).

A removable Level-1 parser/claim probe proved that the carrier is valid YAML
and that the authoritative standalone parser recognizes it, then failed because
the lexical claim returned `None` on all four configured Nextest attempts. A
second removable in-memory LSP probe opened a valid tagged schema and changed it
to the carrier above; all four attempts failed because the current schema
diagnostic disappeared before the completion and hover assertions could run.
Both probes were removed before final verification.

Make scalar-boundary handling presentation-aware. In block context, a
mid-token `[`, `{`, or `,` must remain plain-scalar content; when a real flow
collection begins at a valid node boundary, its nested delimiters still need
flow semantics. Add table-driven parser/claim parity cases for block plain
scalars containing each flow-only indicator, including an inert ordinary-YAML
case, and add an in-memory LSP valid-open → malformed-change regression proving
current diagnostics plus last-good completion and hover. Avoid another
character-specific patch: the scanner should model whether it is in block
plain-scalar content or a genuinely opened flow collection.

## Prior Review Closure

- **Review 11's mid-token hyphen followed by whitespace — closed on the
  reported surface.** `-` now begins a scalar position only when the scanner
  was already at a scalar boundary, and the exact parser/claim and LSP
  regressions pass 2/2. The distinct block-versus-flow context defect remains
  open as the finding above.
- **Earlier meta-schema implementation findings — remain closed.** No new gap
  was found in the semantic types, passive parser authority, custom keyword
  lowering, base schema, recursion limit, compatibility, side-effect guards,
  documentation, or terminal staging work closed by Reviews 1–11.

## Requirement Verification Levels

| Requirement | Appropriate level | Strongest verification present | Assessment |
| --- | --- | --- | --- |
| AC1–6: grammar, lowering, validation, serialization, and semantic arrays | Level 1 | Parser, conversion, validation, serialization, property, trigger, compose, and persistence tests | Appropriate and green. |
| AC7: shared passive semantic/source-aware authority | Level 1 | Parser-parity and structural-sidecar tests across quoting, CRLF, UTF-8, nesting, and unions | Appropriate and green. |
| AC8: base schema and declaration preparation | Level 1 | Base-schema, inline/reference/root-union/raw-JSON, repository-schema, dependency, origin, and cache tests | Appropriate and green. |
| AC9: DMLS completion, hover, diagnostics, activation, and last-good state | Level 1 | In-memory overlay and LSP protocol tests | **Incomplete.** A block plain scalar containing `foo{ "bar` can hide the later envelope tag and remove diagnostics plus last-good assistance. |
| AC10: passive analysis performs no file/process/shell/network side effects | Level 1 | Sentinel integration tests across diagnostics and provider requests | Appropriate and green for the implemented request paths. |
| AC11–12: recursion bounds and compatibility | Level 1 | Depth-boundary, baseline replay, import-name, shipped-corpus, repository-schema, property, and downstream Claudine tests | Appropriate and green; no new compatibility gap found. |
| AC13: release gates and terminal presentation | Level 2 for real-terminal rendering; Level 1 for protocol semantics | Current DMLS L1 629/629 and DMLS L2 3/3; current aggregate L2 library 18/18 plus CLI 46/46 reached; prior complete aggregate 90/90 | Satisfied for the affected package and previously green full terminal corpus. |

No requirement concerns OS keyboard encoding, modifier visibility, hotkeys,
paste, IME, or mouse delivery, so Level 3 is not applicable. DMLS completion,
hover, diagnostics, and last-good behavior are in-memory LSP semantics and
belong at Level 1; real-terminal rendering belongs at Level 2.

## Verification Performed

- The `darkmatter` skill supplied the meta-schema and review-frontmatter
  authority; `rust-testing` supplied the L1/L2/L3 classification; `sniff`
  identified the Darkmatter package area and the `darkmatter`,
  `darkmatter-cli`, and `dmls` package surfaces. The Review 11 production change
  is confined to DMLS source/tests, and `dmls` has no downstream workspace
  package consumer.
- GitNexus reports HIGH upstream impact for `standalone_envelope_claim`: 41
  affected symbols, one direct caller, and four affected modules (overlay,
  diagnostics, providers, and workspace). `flow_top_level_entries` is also
  HIGH at 23 affected symbols; `advance_quote_state` is LOW at three. No Rust
  production symbol was modified during this review.
- Review 11's parser/claim and in-memory LSP regressions passed 2/2.
- A removable parser/claim counterexample failed on all four Nextest attempts
  after proving that the carrier is valid YAML and recognized by the
  authoritative standalone parser. A removable in-memory LSP counterexample
  also failed on all four attempts because current diagnostics disappeared.
  Both probes were removed before final verification.
- The complete affected DMLS Level-1 suite passed 629/629, with three
  higher-tier tests skipped by the Level-1 selector.
- DMLS Level 2 passed 3/3. The canonical aggregate additionally passed library
  18/18 and CLI 46/46 reached tests before the non-interactive ceiling required
  interruption; the remaining 23 CLI tests and DMLS tier were not reached in
  that aggregate invocation.
- `just _build dmls 'Darkmatter Language Server'` and `just _lint dmls` passed
  on macOS. The implementation remains designed for macOS, Windows, and Linux;
  this host did not execute native Windows or Linux binaries.
- No formatting command was run.

## Production Readiness

**Not ready.** Make the lexical envelope scanner distinguish block plain-scalar
content from actual flow-collection context, then prove the block
flow-indicator carrier retains current diagnostics, completion, and hover
through the in-memory LSP protocol. The remaining reviewed requirements and
affected-package gates are green.
