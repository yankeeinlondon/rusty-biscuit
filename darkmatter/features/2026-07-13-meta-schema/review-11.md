---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T17:46:49-07:00
spec: 2026-07-13-meta-schema/spec.md
log: darkmatter/features/2026-07-13-meta-schema/log.md
implemented: true
implemented_by: codex/default
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-11.md
previous: 2026-07-13-meta-schema/review-10.md
---

# Review 11 — Meta Schema

## Verdict

This feature is **not ready for production**. Review 10's exact flow-mapping
counterexample is fixed and now has both parser/claim parity coverage and an
in-memory LSP regression. The scalar-boundary repair is still incomplete,
however: it treats a hyphen followed by whitespace as a structural boundary
without checking whether the hyphen is already inside a plain scalar. A valid
flow-style tagged envelope can therefore still lose its lexical activation
claim during an invalid-definition edit, causing DMLS to discard the last-good
schema model instead of retaining completion, hover, and current diagnostics.
This is another user-observable AC9 failure with no Level-1 regression.

The affected package gates are otherwise green: the complete DMLS Level-1
suite passed 629/629, the DMLS Level-2 tier passed 3/3, and the DMLS build and
lint gates passed. The canonical Level-2 aggregate progressed without a
failure through all 18 library tests and 41 of 69 CLI tests before it was
stopped at the non-interactive 60-second command ceiling; Review 10's complete
run remains 90/90, and the implementation since then changes only the DMLS
lexical scanner and its tests.

## Findings

### High: A mid-token hyphen still turns a plain-scalar quote into flow quote state

The Review 10 repair adds `at_scalar_start` state to the flow scanner and
correctly keeps the quote in `foo-"bar` inert. Its boundary helper says that
`-` and `:` begin a scalar only at a token boundary, but the implementation
checks only the character after the indicator
([schema.rs:393](../../dmls/src/overlay/schema.rs#L393),
[schema.rs:401](../../dmls/src/overlay/schema.rs#L401)). Consequently, any
mid-token hyphen followed by whitespace sets `at_scalar_start` to true
([schema.rs:435](../../dmls/src/overlay/schema.rs#L435)).

This carrier is valid YAML, claims a tagged standalone schema envelope, and
contains an invalid SimplifiedSchema property definition:

```yaml
{types: {title: foo- "bar}, kind: schema}
```

The quote is plain-scalar content: the hyphen is part of `foo-`, not a block
sequence indicator. The authoritative `parse_standalone_schema_document`
recognizes the tagged envelope and returns its structured invalid-definition
error. `standalone_envelope_claim` instead returns `None`: the mid-scalar
hyphen makes the following quote open flow quote state, which consumes the
nested closing brace and the later top-level `kind` entry. The overlay then
applies `claim?` on the parser error and returns no authoring overlay
([mod.rs:313](../../dmls/src/overlay/mod.rs#L313)). Completion and hover
disappear, and the current schema diagnostic is not published.

A removable Level-1 probe first asserted that the authoritative parser
returned `Err`, then expected `Some(Tagged)` from the lexical claim. The claim
returned `None` on all four configured Nextest attempts. The probe was removed
before final verification. The new regression covers only the adjacent form
`foo-"bar` ([lsp_session.rs:3852](../../dmls/tests/lsp_session.rs#L3852)); it
does not exercise an indicator-looking character that is mid-token but followed
by whitespace.

Make the `-`/`:` boundary decision depend on both sides of the indicator, or
otherwise preserve whether the scanner is already inside a plain scalar.
Add parser/claim parity cases for this carrier in both tagged key orders and
inert ordinary flow YAML. Add an in-memory LSP valid-open → malformed-change
regression proving current diagnostics plus last-good completion and hover.

## Prior Review Closure

- **Review 10's `foo-"bar` flow quote poisoning — closed on the reported
  surface.** The scanner now opens quotes only when `at_scalar_start` is true,
  and tests cover both tagged key orders, inert ordinary flow YAML, and the
  complete LSP diagnostics/completion/hover transition. The mid-token
  hyphen-plus-whitespace path remains open as the finding above.
- **Earlier meta-schema implementation findings — remain closed.** No new gap
  was found in the semantic types, passive parser authority, custom keyword
  lowering, base schema, recursion limit, compatibility, side-effect guards,
  documentation, or terminal staging work closed by Reviews 1–10.

## Requirement Verification Levels

| Requirement | Appropriate level | Strongest verification present | Assessment |
| --- | --- | --- | --- |
| AC1–6: grammar, lowering, validation, serialization, and semantic arrays | Level 1 | Parser, conversion, validation, serialization, property, trigger, compose, and persistence tests | Appropriate and green. |
| AC7: shared passive semantic/source-aware authority | Level 1 | Parser-parity and structural-sidecar tests across quoting, CRLF, UTF-8, nesting, and unions | Appropriate and green. |
| AC8: base schema and declaration preparation | Level 1 | Base-schema, inline/reference/root-union/raw-JSON, repository-schema, dependency, origin, and cache tests | Appropriate and green. |
| AC9: DMLS completion, hover, diagnostics, activation, and last-good state | Level 1 | In-memory overlay and LSP protocol tests | **Incomplete.** A flow-style tagged edit containing `foo- "bar` can lose activation and all last-good assistance. |
| AC10: passive analysis performs no file/process/shell/network side effects | Level 1 | Sentinel integration tests across diagnostics and provider requests | Appropriate and green for the implemented request paths. |
| AC11–12: recursion bounds and compatibility | Level 1 | Depth-boundary, baseline replay, import-name, shipped-corpus, repository-schema, property, and downstream Claudine tests | Appropriate and green; no new compatibility gap found. |
| AC13: release gates and terminal presentation | Level 2 for real-terminal rendering; Level 1 for protocol semantics | Current DMLS L1 629/629 and DMLS L2 3/3; Review 10 full L1 and L2 90/90 | Satisfied for the affected package and previously green full corpus. |

No requirement concerns OS keyboard encoding, modifier visibility, hotkeys,
paste, IME, or mouse delivery, so Level 3 is not applicable. DMLS completion,
hover, diagnostics, and last-good behavior are in-memory LSP semantics and
belong at Level 1; real-terminal rendering belongs at Level 2.

## Verification Performed

- `biscuit-file` guidance was used for the repository file references. The
  requested `@prompts/./_reviews/.../review-10.md` archive path does not exist
  in this worktree, so the canonical colocated
  `darkmatter/features/2026-07-13-meta-schema/review-10.md` received the
  previous-review lifecycle update.
- `sniff` identified the Darkmatter package area and its `darkmatter`,
  `darkmatter-cli`, and `dmls` Rust packages. The change after Review 10 is
  confined to DMLS source/tests.
- GitNexus reports HIGH upstream impact for `standalone_envelope_claim`: 41
  affected symbols, one direct caller, and four affected modules (overlay,
  diagnostics, providers, workspace). No Rust production symbol was modified
  during this review.
- The Review 10 parity and LSP regressions passed 2/2.
- A removable Level-1 counterexample failed on all four Nextest attempts after
  proving the authoritative parser recognized the tagged envelope. The probe
  was removed before final verification.
- The complete affected DMLS Level-1 suite passed 629/629, with 3 higher-tier
  tests skipped by the Level-1 selector.
- DMLS Level 2 passed 3/3. The canonical aggregate additionally passed library
  18/18 and CLI 41/41 reached tests before the session ceiling interrupted the
  remaining 28 unchanged CLI tests.
- `just _build dmls 'Darkmatter Language Server'` and `just _lint dmls` passed
  on macOS. The implementation remains designed for macOS, Windows, and Linux;
  this host did not execute native Windows or Linux binaries.
- No formatting command was run.

## Production Readiness

**Not ready.** Correct the scalar-boundary test so an indicator is structural
only when it is actually at a YAML token boundary, then prove the
hyphen-plus-whitespace carrier retains current diagnostics, completion, and
hover through the in-memory LSP protocol. The remaining reviewed requirements
and affected-package gates are green.
