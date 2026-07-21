---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T17:02:40-07:00
spec: 2026-07-13-meta-schema/spec.md
log: darkmatter/features/2026-07-13-meta-schema/log.md
implemented: true
implemented_by: codex/default
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-10.md
previous: 2026-07-13-meta-schema/review-09.md
---

# Review 10 — Meta Schema

## Verdict

This feature is **not ready for production**. Review 9's repository-schema,
documentation, and Level-2 findings are closed, and its block-style last-good
counterexample is fixed on the reported input. The same quote-state defect
remains in the separate flow-mapping scanner, however. A valid flow-style
tagged envelope with `types` before `kind` can lose its lexical activation
claim during an invalid-definition edit, causing DMLS to discard the
last-good schema model instead of retaining completion, hover, and current
diagnostics. That is a user-observable AC9 failure with no Level-1 regression.

The canonical gates themselves are green: Level 1 passed 5,932 Darkmatter,
561 CLI, and 627 DMLS tests; Level 2 passed 18 library, 69 CLI, and 3 DMLS
tests (90/90 total); and the Darkmatter-area build and lint recipes passed.
Those gates do not cover the flow-style counterexample below.

## Findings

### High: Flow-style nested plain scalars can still erase last-good schema state

Review 9 showed that a quote inside an indented block-style definition could
poison the lexical envelope scanner and hide a later `kind: schema`. The repair
correctly makes block scanning ignore nested lines when no top-level quoted
scalar is open and makes its quote opener token-aware
([schema.rs:280](../../dmls/src/overlay/schema.rs#L280),
[schema.rs:362](../../dmls/src/overlay/schema.rs#L362)). The standalone
contract is presentation-independent, though: the authoritative parser accepts
equivalent block and flow mappings, and the lexical claim explicitly promises
both presentations ([schema.rs:251](../../dmls/src/overlay/schema.rs#L251)).

The separate flow scanner still treats every `'` or `"` as the beginning of a
quoted scalar, regardless of whether it appears at a YAML scalar boundary
([schema.rs:434](../../dmls/src/overlay/schema.rs#L434),
[schema.rs:463](../../dmls/src/overlay/schema.rs#L463)). A quote is legal plain
scalar content when it appears mid-token. Therefore this flow-style carrier is
valid YAML and claims a tagged envelope, while its `title` definition is
invalid SimplifiedSchema syntax:

```yaml
{types: {title: foo-"bar}, kind: schema}
```

`parse_standalone_schema_document` recognizes that tagged envelope and returns
the structured invalid-definition error. `standalone_envelope_claim` instead
returns `None`: the quote after `foo-` opens flow quote state, consumes the
remaining nested brace and top-level `kind` entry, and leaves no tagged claim.
When the authoritative parse fails, `OverlayState::for_document` applies
`claim?`, so this `None` discards the overlay and its cached last-good model
([mod.rs:313](../../dmls/src/overlay/mod.rs#L313)). Completion and hover then
disappear, and the current schema diagnostic is not published.

This is the same AC9 user-observable state failure as Review 9, reached through
the other presentation path. Level 1 is the appropriate verification. A
removable integration probe first asserted that the authoritative parser
returned `Err`, then expected `Some(Tagged)` from the claim; it failed on all
four configured Nextest attempts with `left: None`. The probe was removed. The
new LSP regression covers only the repaired block carrier
([lsp_session.rs:3771](../../dmls/tests/lsp_session.rs#L3771)); the existing
flow last-good test puts `kind` first and changes `string` to `str`, so it never
exercises mid-plain-scalar quote handling or `types`-first ordering
([lsp_session.rs:3852](../../dmls/tests/lsp_session.rs#L3852)).

Make flow quote opening token-aware under the same YAML boundary rules as the
block scanner while preserving nested flow-depth handling. Add parser/claim
parity cases for both tagged key orders and for inert ordinary flow YAML. Then
add an in-memory LSP valid-open → malformed-change regression proving current
diagnostics plus last-good completion and hover for the flow carrier above.

## Prior Review Closure

- **Block-style nested quote poisoning — closed on the reported surface.** The
  scanner now isolates nested block payload text, and the new in-memory LSP
  regression proves diagnostics, completion, and hover survive the exact
  `types`-first block edit. The equivalent flow path remains open as the finding
  above.
- **Repository review schemas — closed.** The shipped review schemas now use
  pure `$schema` envelopes. The new repository-root corpus tests classify the
  artifacts, resolve `feature-review.yaml` by bare name, validate a well-formed
  review, and reject an actively type-invalid review. All three tests passed in
  the full Level-1 gate.
- **Semantic-array documentation — closed.** The v1 limitation now explicitly
  excludes `type-definition[]` and `schema[]` and links back to the supported
  nested-sequence representation.
- **Canonical Level-2 gate — closed.** The three code-block tests now stage
  `COLORFGBG` under tmux while still executing the Cargo-built `md` shim. A
  fresh canonical run passed 90/90, including all three formerly red tests.

## Requirement Verification Levels

| Requirement | Appropriate level | Strongest verification present | Assessment |
| --- | --- | --- | --- |
| AC1–6: grammar, lowering, validation, serialization, and semantic arrays | Level 1 | Parser, conversion, validation, serialization, property, trigger, compose, and persistence tests | Appropriate and green. |
| AC7: shared passive semantic/source-aware authority | Level 1 | Parser-parity and structural-sidecar tests across quoting, CRLF, UTF-8, nesting, and unions | Appropriate and green. |
| AC8: base schema and declaration preparation | Level 1 | Base-schema, inline/reference/root-union/raw-JSON, repository-schema, dependency, origin, and cache tests | Appropriate and green. |
| AC9: DMLS completion, hover, diagnostics, activation, and last-good state | Level 1 | In-memory overlay and LSP protocol tests | **Incomplete.** Flow-style `types`-first tagged content can lose activation and all last-good assistance after a mid-plain-scalar quote edit. |
| AC10: passive analysis performs no file/process/shell/network side effects | Level 1 | Sentinel integration tests across diagnostics and provider requests | Appropriate and green for the implemented request paths. |
| AC11–12: recursion bounds and compatibility | Level 1 | Depth-boundary, baseline replay, import-name, shipped-corpus, repository-schema, property, and downstream Claudine tests | Appropriate and green; no new compatibility gap found. |
| AC13: release gates and terminal presentation | Level 2 for real-terminal rendering; Level 1 for protocol semantics | Fresh full Level-1 and canonical Level-2 runs | Satisfied: Level 1 is green and Level 2 is 90/90. |

No requirement concerns OS keyboard encoding, modifier visibility, hotkeys,
paste, IME, or mouse delivery, so Level 3 is not applicable. DMLS completion,
hover, diagnostics, and last-good behavior are in-memory LSP semantics and
belong at Level 1; real-terminal rendering belongs at Level 2.

## Verification Performed

- `biscuit-file` resolved the specification to its canonical Darkmatter feature
  path. The requested `@prompts/./_reviews/.../review-09.md` archive path does
  not exist in this worktree, so the colocated
  `darkmatter/features/2026-07-13-meta-schema/review-9.md` received the previous-
  review lifecycle update.
- `sniff` identified the affected package area as `darkmatter`, containing the
  `darkmatter`, `darkmatter-cli`, and `dmls` packages, and provided the workspace
  dependency map used to scope verification.
- GitNexus reports CRITICAL upstream impact for
  `parse_property_definition` (74 affected symbols) and
  `parse_schema_declaration` (94), plus HIGH impact for
  `standalone_envelope_claim` (41). No Rust production symbol was modified
  during this review.
- A removable Level-1 flow-style counterexample failed on all four Nextest
  attempts after proving the authoritative parser recognized the tagged
  envelope. The probe was removed before final verification.
- `just test` passed: Darkmatter 5,932/5,932, CLI 561/561, and DMLS 627/627.
- `just test-l2 --no-fail-fast` passed: Darkmatter 18/18, CLI 69/69, and DMLS
  3/3 — 90/90 total.
- `just build` and `just lint` passed for all three Darkmatter-area packages on
  macOS. The implementation remains designed for macOS, Windows, and Linux;
  this host did not execute native Windows or Linux binaries.
- No formatting command was run.

## Production Readiness

**Not ready.** Apply the block scanner's token-aware quote-opening discipline
to flow mappings and prove the `types`-first flow edit retains current
diagnostics, completion, and hover through the in-memory LSP protocol. With
that AC9 gap closed, the remaining reviewed requirements and release gates are
green.
