---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T09:15:55-07:00
spec: 2026-07-13-meta-schema/spec.md
log: darkmatter/features/2026-07-13-meta-schema/log.md
implemented: true
implemented_by: claude/default
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-8.md
previous: 2026-07-13-meta-schema/review-7.md
---

# Review 8 — Meta Schema

## Verdict

This feature is **not ready for production**. Review 7's nested-owner hover
gap is closed, and flow-style standalone documents now retain last-good hover
and diagnostics. The implementation itself records, however, that completion
still does not activate in flow mappings and that three standalone pattern-key
forms are intercepted by the Markdown hover provider. Both are required DMLS
behaviors with no Level-1 protocol verification.

The new flow-envelope recognizer also mishandles escaped quotes. It can
misclassify valid ordinary YAML/raw-JSON-shaped input as a pure SimplifiedSchema
envelope and then reach an `expect_err` panic. Finally, AC13 remains explicitly
red: the latest recorded real-terminal run is 87/90, and its proposed three-test
exception is still unratified.

## Findings

### High: Flow-style standalone schemas still have no completion

The authoritative standalone parser accepts block and flow mappings equally,
and the specification requires completion for every standalone document it
recognizes ([spec.md:468](spec.md#L468), [spec.md:538](spec.md#L538)). The
completion entry point still derives the cursor from `line_prefix`, indentation,
`value_cursor`, and `enclosing_path`
([frontmatter.rs:99](../../dmls/src/providers/frontmatter.rs#L99)). A one-line
flow mapping has no block-style `key:` line or indentation ancestry, so
`meta_schema_completion` returns no semantic candidates in both valid and
malformed states.

The new LSP test explicitly excludes completion and documents that it does not
activate even in a valid flow mapping
([lsp_session.rs:3738](../../dmls/tests/lsp_session.rs#L3738)). Hover and
diagnostics therefore prove only part of the required last-good contract;
completion still disappears for flow-authored pure and tagged envelopes.

This is user-observable LSP protocol behavior, so Level 1 is appropriate. The
strongest verification present for flow presentation covers hover and
diagnostics only; completion has no passing Level-1 verification and is known
to be absent.

Teach the semantic completion router to locate the cursor structurally within
flow mappings instead of requiring block indentation. Add in-memory LSP cases
for valid and malformed pure/tagged flow envelopes, including nested mappings,
outer arrays, and union-valued array items.

### High: Standalone pattern-key hover is still claimed by the Markdown provider

Within every activated schema shape, hover must identify a complete property
definition as `type-definition` and render its denoted type
([spec.md:496](spec.md#L496)). Region projection now correctly includes all four
pattern-key forms, but hover arbitration is first-non-empty-wins and registers
the Markdown substrate before the frontmatter provider
([mod.rs:327](../../dmls/src/providers/mod.rs#L327),
[mod.rs:398](../../dmls/src/providers/mod.rs#L398)). In a standalone YAML
document, `<starting::…>`, `<ending::…>`, and `<pattern::…>` are interpreted as
Markdown autolinks first, so the semantic hover never runs.

The new test acknowledges this collision and limits standalone protocol
coverage to `<string>` while testing the other forms only in Markdown
frontmatter ([lsp_session.rs:3477](../../dmls/tests/lsp_session.rs#L3477)). The
projection unit test proves spans exist, but it does not verify the
user-observable hover returned by the provider registry.

This behavior also belongs at Level 1. The strongest verification for the three
affected standalone forms is below the required protocol surface, so AC9
remains incomplete.

Make activated standalone schema regions authoritative over generic Markdown
link hover, either through region-aware arbitration or by suppressing substrate
link claims inside those regions. Add full LSP hover cases for all four
pattern-key forms in both pure and tagged standalone documents.

### High: Escaped quotes can false-activate ordinary YAML and panic overlay construction

`standalone_envelope_claim` promises that quoted scalars cannot expose their
internal commas or colons and that ordinary YAML/raw JSON Schema remain inert
([schema.rs:231](../../dmls/src/overlay/schema.rs#L231)). Its flow scanner closes
the active quote whenever it sees the same quote character, without honoring a
backslash-escaped double quote or YAML's doubled single quote
([schema.rs:308](../../dmls/src/overlay/schema.rs#L308)). For example, valid JSON
such as:

```json
{"$schema":"https://example.com/quo\"ted","type":"object"}
```

is scanned as one `$schema` entry because the escaped quote terminates scanner
quote state early and the actual closing quote starts a new quoted region that
swallows the top-level comma. The recognizer therefore returns `Pure` even
though the authoritative parser returns `Ok(None)` for the two-key document.
That supposedly impossible combination reaches an `expect_err` on a successful
YAML parse and panics ([mod.rs:312](../../dmls/src/overlay/mod.rs#L312)).

The inert-input tests cover ordinary quoted flow YAML and a standard raw JSON
Schema, but no escape-bearing scalar
([schema.rs:868](../../dmls/src/overlay/schema.rs#L868)). This is a Level-1
robustness and activation contract with no verification for the failing input.

Track double-quoted escapes and doubled single quotes in the bounded scanner,
and remove the panic assumption: a lexical/authoritative disagreement should
deactivate or return a diagnostic, never crash. Add parser/claim parity cases
and an `OverlayState::for_document` regression proving valid ordinary YAML and
raw-JSON-shaped documents remain inactive without panicking.

### High: AC13 remains red and the scoped exception is still unratified

Acceptance criterion 13 requires `just test-l2` to pass, and the specification
still labels its exception **proposed — not approved**
([spec.md:750](spec.md#L750), [spec.md:755](spec.md#L755)). The latest recorded
post-review-7 run is **87/90**: library 18/18, CLI 66/69, and DMLS 3/3. The same
three code-block styling tests still fail with deterministic value mismatches
([spec.md:829](spec.md#L829)).

Those failures are outside the meta-schema execution path, and the spec gives
credible evidence that they are WezTerm staging defects. That does not make the
declared gate green or authorize this review to ratify the exception. Restage
the three tests on a suitable real-terminal backend, or record the required
ratification before treating AC13 as satisfied.

## Prior Review Closure

- **Nested semantic-owner hover — closed.** Hover now shares the longest-owner
  routing used by completion, and Level-1 LSP tests cover nested entries under
  ordinary `schema` and mapping-valued `type-definition` owners.
- **Pattern-key region projection — partially closed.** All pattern keys now
  become semantic regions and inline frontmatter hover works. Standalone
  provider arbitration still breaks three forms, so the user-facing half
  remains open as a finding above.
- **Flow-style last-good activation — partially closed.** The lexical claim now
  recognizes flow envelopes, and Level-1 LSP tests prove retained hover and
  current-buffer diagnostics. Completion remains absent, and the new scanner
  has an escape-handling panic path.
- **Canonical Level-2 gate — open.** The recorded result remains 87/90 and the
  exception remains unratified.

## Requirement Verification Levels

| Requirement | Appropriate level | Strongest verification present | Assessment |
| --- | --- | --- | --- |
| AC1–6: grammar, lowering, validation, serialization, and semantic arrays | Level 1 | Parser, conversion, validation, serialization, property, trigger, compose, and filesystem tests | Appropriate for these in-process semantics; no gap found in the reviewed surface. |
| AC7: shared passive semantic/source-aware authority | Level 1 | Parser-parity, structural-sidecar, quote/CRLF/UTF-8, nesting, and union tests | Appropriate and passing in the recorded feature gates. |
| AC8: base schema and declaration preparation | Level 1 | Base-schema, inline/reference/root-union/raw-JSON preparation, depth, dependency, origin, and cache tests | Appropriate and passing in the recorded feature gates. |
| AC9: DMLS completion, hover, diagnostics, activation, and last-good state | Level 1 | In-memory overlay and LSP tests | **Incomplete.** Flow completion is absent; three standalone pattern-key hovers are intercepted; escape-bearing flow input can false-activate and panic. |
| AC10: passive analysis performs no file/process/shell/network side effects | Level 1 | Sentinel integration tests | Appropriate for covered requests; the escape-triggered panic is a robustness failure, not an external side effect. |
| AC11–12: recursion bounds and compatibility | Level 1 | Depth-boundary, baseline replay, import-name, property, and downstream Claudine tests | Appropriate for the tested surface; no new mismatch found. |
| AC13: release gates and terminal presentation | Level 2 for real-terminal rendering; Level 1 for protocol semantics | Recorded L1 area gate green; latest recorded L2 total 87/90 | **Not satisfied.** Three L2 assertions fail and the proposed exception is unratified. |

No requirement concerns OS keyboard encoding, modifier visibility, hotkeys,
paste, IME, or mouse delivery, so Level 3 is not applicable. DMLS completion
and hover are in-memory LSP protocol behavior and correctly belong at Level 1;
real-terminal rendering belongs at Level 2.

## Verification Performed

- `biscuit-file` resolved the specification to the canonical package-area file.
  The requested `@prompts/./_reviews/.../review-7.md` archive reference does not
  exist on this host, so the colocated canonical review was updated, matching
  the handling recorded by review 7.
- `sniff` identified `darkmatter`, `darkmatter-cli`, and `dmls` as the affected
  package-area crates. Source and dependency inspection identifies `claudine`
  and `claudine-cli` as the exhaustive downstream consumers of the new public
  `SimplifiedType` variants.
- GitNexus reports CRITICAL upstream impact for
  `parse_property_definition` (74 affected symbols) and
  `parse_schema_declaration` (93), LOW impact for `meta_schema_hover` and
  `semantic_type_regions` (9 each), and HIGH impact for
  `standalone_envelope_claim` (41). No Rust symbol was modified.
- The post-review-7 implementation record reports a green full Level-1 area
  run and lint, plus L2 87/90. A fresh focused DMLS Level-1 run passed **4/4**:
  nested-owner hover, pattern-key hover, block pure/tagged completion, and
  flow-envelope last-good hover/diagnostics.
- A removable Nextest regression probe for escaped-quote raw-JSON-shaped input
  failed on all four configured attempts: `standalone_envelope_claim` returned
  `Some(Pure)` instead of `None`. The probe was removed after confirming the
  defect; the subsequent `OverlayState` panic chain is explicit in the source.
- Darkmatter parsed every requested lifecycle property with the exact value,
  and the specification validates successfully. Review validation remains
  blocked by the pre-existing `schemas/feature-review.yaml` tagged envelope,
  whose unsupported top-level `description` and `$schema` keys prevent the
  schema itself from loading.
- Static path analysis confirmed the two explicitly documented DMLS gaps and
  the escaped-quote scanner/`expect_err` panic chain. No formatting command was
  run, and the pre-existing unrelated `CLAUDE.md` change was preserved.

## Production Readiness

**Not ready.** Add flow-aware completion, make standalone semantic hover win
over Markdown autolinks, harden the flow-envelope recognizer against escaped
quotes and lexical disagreement, and either restore the Level-2 gate or obtain
the documented exception ratification.
