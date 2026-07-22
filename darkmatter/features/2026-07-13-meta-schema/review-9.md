---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T11:20:08-07:00
spec: 2026-07-13-meta-schema/spec.md
log: darkmatter/features/2026-07-13-meta-schema/log.md
implemented: true
implemented_by: codex/default
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-9.md
next: 2026-07-13-meta-schema/review-10.md
previous: 2026-07-13-meta-schema/review-8.md
---

# Review 9 — Meta Schema

## Verdict

This feature is **not ready for production**. Review 8's flow-completion,
pattern-hover arbitration, and false-activation panic findings are closed on
their reported inputs, with passing Level-1 protocol regressions. The hardened
block-envelope scanner still has a false-negative path, however: an indented
invalid definition can mutate the scanner's top-level quote state and hide a
later `kind: schema` entry. DMLS then drops the standalone overlay instead of
retaining last-good completion, hover, and diagnostics.

Acceptance criterion 13 also remains formally unmet. The current implementation
record reports 87/90 Level-2 tests passing, and the three-test exception remains
explicitly proposed rather than ratified. The public schema topic also contains
a blanket array-of-unions limitation that contradicts its own documented
semantic-array example, and two shipped repository schemas no longer load under
the feature's tagged-envelope rules.

## Findings

### High: An indented invalid definition can hide a later tagged-envelope claim

Standalone tagged documents may author `types` before `kind`; mapping order is
not part of the language contract. During malformed edits, the lexical claim is
responsible for retaining the last-good semantic model
([spec.md:468](spec.md#L468), [spec.md:478](spec.md#L478)).

`block_top_level_entries` calls `advance_quote_state` on every physical line
before it rejects indented lines as non-top-level
([schema.rs:281](../../dmls/src/overlay/schema.rs#L281)). Consequently, text
inside the nested `types` payload can alter quote state used to recognize later
top-level keys. The scalar-start tracker compounds this by treating every
hyphen, colon, comma, `[` and `{` as a fresh scalar boundary even when the
character occurs inside a plain scalar
([schema.rs:360](../../dmls/src/overlay/schema.rs#L360)).

This valid YAML carrier demonstrates the failure:

```yaml
types:
  title: foo-"bar
kind: schema
```

The authoritative `parse_standalone_schema_document` recognizes the tagged
envelope and returns a structured error for the invalid type definition. The
lexical `standalone_envelope_claim` instead returns `None`: the hyphen marks a
false scalar boundary, the literal quote opens cross-line quote state, and the
following `kind: schema` line is discarded as a continuation. In
`OverlayState::for_document`, the error arm then applies `claim?` and returns no
overlay at all ([mod.rs:312](../../dmls/src/overlay/mod.rs#L312)). A valid-open
to malformed-change sequence therefore loses its cached semantic model and the
current diagnostic together, violating AC9
([spec.md:732](spec.md#L732)).

This is user-observable LSP state behavior, so Level 1 is the appropriate
verification. A removable Nextest probe first asserted that the authoritative
parser returns `Err`, then asserted `Some(Tagged)` from the lexical claim; it
failed deterministically on all four configured attempts with `left: None`.
The shipped escape tests cover quoted top-level continuations and put `kind`
before their invalid payload, but do not cover an indented plain scalar poisoning
a later envelope key
([schema.rs:1153](../../dmls/src/overlay/schema.rs#L1153)).

Ignore an indented line entirely when no top-level quoted scalar is already
open; only a continuation of an already-open top-level scalar should advance
cross-line quote state. Also make scalar-boundary tracking token-aware rather
than setting it from punctuation inside a plain scalar. Add parser/claim parity
coverage for both tagged key orders and an in-memory LSP valid-open → malformed
change regression proving diagnostics, completion, and hover remain available.

### High: Shipped review schemas are incompatible with the tagged-envelope contract

The repository ships `schemas/feature-review.yaml` and
`schemas/suggestion-review.yaml` as `kind: schema` documents whose payload is
under `$schema`; the feature-review schema also carries a top-level
`description`
([feature-review.yaml:1](../../../schemas/feature-review.yaml#L1),
[suggestion-review.yaml:1](../../../schemas/suggestion-review.yaml#L1)). The
new standalone classifier treats every `kind: schema` document as tagged and
permits exactly `kind` plus a mapping-valued `types`
([standalone.rs:137](../../lib/src/markdown/schemas/simplified/standalone.rs#L137)).
It therefore rejects both shipped schemas before they can validate their
consumers.

This is not merely an unused artifact. The review file required by this
workflow declares `$schema: feature-review.yaml`; running the current workspace
`darkmatter-cli` against Review 9 returns a schema-load error:

```text
tagged schema documents support only `kind` and `types`; found unsupported keys: description, $schema
```

AC12 requires existing schemas outside the two new keywords to remain
compatible except for the enumerated `$schema` metadata correction
([spec.md:745](spec.md#L745)). Neither shipped review schema uses
`type-definition` or `schema`, and this failure is not one of AC12's allowed
deltas. Existing corpus coverage walks `darkmatter/docs/schemas`, so it misses
the repository-level `schemas/` directory and cannot catch the broken workflow.

This is deterministic in-process schema resolution, so Level 1 is sufficient
and currently red. Migrate the two artifacts to a valid pure or tagged envelope
(preserving documentation as comments or supported metadata), or explicitly
provide and test a compatibility form if the legacy envelope is intentional.
Add a repository-schema corpus test and an end-to-end review fixture that must
load `feature-review.yaml` successfully.

### Medium: Public documentation contradicts the semantic-array contract

The public topic correctly explains that `type-definition[]` and `schema[]` are
arrays of independent semantic values and that a union-valued item is authored
as a nested sequence
([schema-definition.md:155](../../docs/topics/schema-definition.md#L155)). Its
v1 limitations later state without qualification that arrays of unions are not
supported and require an external JSON Schema
([schema-definition.md:1461](../../docs/topics/schema-definition.md#L1461)).

That blanket statement is true for ordinary denoted-value unions, but false for
the semantic carrier case introduced by this feature and demonstrated only
pages earlier. It can lead an author to avoid the supported `type-definition[]`
or `schema[]` nested-sequence syntax that AC6 explicitly requires. Qualify the
limitation so it excludes semantic meta-type arrays, and link back to the
semantic-array example.

### High: AC13 remains red and the scoped exception is still unratified

Acceptance criterion 13 requires both `just test` and `just test-l2` to pass,
and the specification still says the exception is proposed and not approved
([spec.md:750](spec.md#L750), [spec.md:755](spec.md#L755)). The post-review-8
implementation reran all three Level-2 tiers and recorded **87/90**: library
18/18, CLI 66/69, and DMLS 3/3. The only failures remain the three named
code-block styling tests ([spec.md:839](spec.md#L839)).

The evidence remains persuasive that these are pre-existing WezTerm staging
defects outside meta-schema execution paths. That evidence does not make the
declared gate green or grant this review authority to ratify the exception.
Restage the three tests on a suitable real-terminal backend, or record the
required ratification before treating AC13 as satisfied.

## Prior Review Closure

- **Flow-style standalone completion — closed on the reported surface.**
  `flow_value_cursor` now supplies structural flow paths, and Level-1 LSP cases
  cover pure, tagged, nested, outer-array, and union-valued-item forms in valid
  and malformed states.
- **Standalone pattern-key hover arbitration — closed.** All four pattern-key
  forms now pass full LSP hover in pure and tagged standalone documents, while
  ordinary Markdown autolink hover has an explicit non-regression test.
- **Escaped-quote false activation and panic — closed on the reported inputs,
  but the scanner remains incomplete.** Escaped double quotes, single-quote
  rules, quoted top-level continuations, and lexical/parser disagreement no
  longer panic or false-activate ordinary documents. The distinct false-negative
  path through an indented nested definition remains open as the first finding.
- **Canonical Level-2 gate — open.** The latest recorded result remains 87/90,
  and the exception is still unratified.

## Requirement Verification Levels

| Requirement | Appropriate level | Strongest verification present | Assessment |
| --- | --- | --- | --- |
| AC1–6: grammar, lowering, validation, serialization, and semantic arrays | Level 1 | Parser, conversion, validation, serialization, property, trigger, and compose tests | Runtime verification is appropriate; the public docs contradict AC6's semantic-array exception as noted above. |
| AC7: shared passive semantic/source-aware authority | Level 1 | Parser-parity, structural-sidecar, quote/CRLF/UTF-8, nesting, and union tests | Appropriate and passing in the recorded feature gates. |
| AC8: base schema and declaration preparation | Level 1 | Base-schema, inline/reference/root-union/raw-JSON preparation, depth, dependency, origin, and cache tests | Appropriate and passing in the recorded feature gates. |
| AC9: DMLS completion, hover, diagnostics, activation, and last-good state | Level 1 | In-memory overlay and LSP tests | **Incomplete.** A `types`-first tagged document can lose its lexical claim and all last-good assistance when an indented invalid definition poisons quote state. |
| AC10: passive analysis performs no file/process/shell/network side effects | Level 1 | Sentinel integration tests | Appropriate for the covered requests; no side-effect gap found. |
| AC11–12: recursion bounds and compatibility | Level 1 | Depth-boundary, baseline replay, import-name, property, and downstream Claudine tests | **Incomplete.** The corpus omits repository-level schemas, and two shipped legacy `kind: schema` artifacts no longer load. |
| AC13: release gates and terminal presentation | Level 2 for real-terminal rendering; Level 1 for protocol semantics | Recorded L1 area gate green; latest recorded L2 total 87/90 | **Not satisfied.** Three L2 assertions fail and the proposed exception is unratified. |

No requirement concerns OS keyboard encoding, modifier visibility, hotkeys,
paste, IME, or mouse delivery, so Level 3 is not applicable. DMLS completion,
hover, diagnostics, and last-good state are in-memory LSP behavior and belong at
Level 1; real-terminal rendering belongs at Level 2.

## Verification Performed

- `biscuit-file` resolved the specification and colocated Review 8 to their
  canonical package-area files. The requested
  `@prompts/./_reviews/.../review-8.md` archive reference does not exist on this
  host, so the canonical colocated Review 8 received the lifecycle update.
- `sniff` identified the `darkmatter` package area (`darkmatter`,
  `darkmatter-cli`, and `dmls`) and its direct workspace consumers. The latest
  code delta is DMLS-only; the feature's existing compatibility tests cover the
  public semantic variants in downstream Claudine format/classification paths.
- GitNexus reports CRITICAL upstream impact for
  `parse_property_definition` (74 affected symbols) and
  `parse_schema_declaration` (93), HIGH impact for
  `standalone_envelope_claim` (41), and LOW impact for `flow_value_cursor` (3).
  No Rust symbol was modified during this review.
- A fresh focused DMLS Level-1 run passed **5/5**: escaped-quote inertness,
  lexical quote parity on the shipped cases, standalone pattern-key hover,
  ordinary Markdown autolink preservation, and structural flow completion.
- A removable Level-1 counterexample for the `types`-first tagged document
  failed on all four Nextest attempts after first proving the authoritative
  parser recognized the envelope. The probe was removed; the worktree returned
  to its original source state.
- The current workspace `darkmatter-cli` validates the specification but cannot
  load Review 9's referenced `schemas/feature-review.yaml`; the tagged-envelope
  incompatibility above is the exact reported error.
- The post-review-8 implementation record reports a green full Darkmatter-area
  `just test` (5,929 library, 561 CLI, 625 DMLS) and `just lint`, plus the fresh
  Level-2 total of 87/90 described above.
- No formatting command was run. The pre-existing unrelated `CLAUDE.md` change
  was preserved.

## Production Readiness

**Not ready.** Prevent nested payload text from poisoning top-level tagged
envelope recognition, prove last-good retention through the LSP protocol,
migrate or support the shipped review schemas, and qualify the public
array-of-unions limitation. Then either restore the Level-2 gate or obtain the
documented exception ratification.
