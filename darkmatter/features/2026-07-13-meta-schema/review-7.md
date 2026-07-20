---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T07:41:25-07:00
spec: 2026-07-13-meta-schema/spec.md
log: darkmatter/features/2026-07-13-meta-schema/log.md
implemented: true
implemented_by: claude/default
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-7.md
previous: 2026-07-13-meta-schema/review-6.md
next: 2026-07-13-meta-schema/review-8.md
---

# Review 7 — Meta Schema

## Verdict

This feature is **not ready for production**. Review 6's Level-2 test-binary
finding is fixed: all eight error-rendering tests now execute the Cargo-built
`md` shim and pass through a real WezTerm pane. The meta-schema parser,
lowering, validation, serialization, reference-depth handling, and principal
DMLS paths remain well covered by passing Level-1 tests.

Two DMLS behaviors required by acceptance criterion 9 remain incomplete.
Hover does not cover every schema shape activated by a semantic `schema` or
`type-definition` value, and flow-style standalone schemas lose their
last-good model during a malformed edit. Acceptance criterion 13 is also still
unmet: the fresh CLI Level-2 run is 66/69, and the specification explicitly
states that the exception for the three remaining failures is proposed rather
than ratified.

## Findings

### High: Meta-schema hover omits valid activated schema regions

The completion path correctly finds the longest semantic owner prefix, so an
entry nested inside any frontmatter property declared as `schema` or
`type-definition` receives type-definition completion
([frontmatter.rs:210](../../dmls/src/providers/frontmatter.rs#L210)). Hover does
not use that activation model. Its frontmatter branch requires the hovered
entry's pointer to equal the semantic owner's pointer and only accepts an owner
whose kind includes `TypeDefinition`
([frontmatter.rs:1037](../../dmls/src/providers/frontmatter.rs#L1037)). Its
remaining inline branch reparses only the reserved `$schema` entry
([frontmatter.rs:1070](../../dmls/src/providers/frontmatter.rs#L1070)). As a
result, nested definitions inside an ordinary property semantically declared
as `schema`, or inside a mapping-valued `type-definition`, receive completion
but no meta-schema hover.

The standalone/`$schema` projection has a second omission. The shared schema
model stores literal properties and pattern keys separately
([types.rs:35](../../lib/src/markdown/schemas/simplified/types.rs#L35)), and the
source projector explicitly resolves both
([source.rs:719](../../lib/src/markdown/schemas/simplified/source.rs#L719)).
`semantic_type_regions`, however, walks only `shape.properties`
([schema.rs:103](../../dmls/src/overlay/schema.rs#L103)). Valid definitions such
as `<string>: string(required)` therefore have source spans and parser support
but never become hover regions.

The specification activates DMLS from any effective semantic owner and requires
hover within a schema shape ([spec.md:462](spec.md#L462),
[spec.md:496](spec.md#L496)). This is a user-observable protocol behavior whose
appropriate verification is Level 1. Existing tests hover the owner of a
`type-definition` value and literal keys under `$schema`, but do not cover the
omitted paths; the strongest verification for them is therefore absent.

Unify hover with the same semantic-owner routing used by completion, then
project literal and pattern-key definitions through one recursive region walk.
Add in-memory LSP tests for a nested entry under an ordinary `schema` owner, a
nested entry under a mapping-valued `type-definition` owner, and each supported
pattern-key form in inline and standalone schemas.

### High: Flow-style standalone envelopes drop last-good activation during malformed edits

The authoritative standalone parser accepts YAML mappings independently of
block or flow presentation
([standalone.rs:121](../../lib/src/markdown/schemas/simplified/standalone.rs#L121)).
A valid flow document such as `{"$schema":{"title":"string"}}` therefore
seeds the standalone last-good cache. Once that buffer becomes temporarily
malformed, `standalone_envelope_claim` is the only activation guard before the
cache is consulted ([mod.rs:281](../../dmls/src/overlay/mod.rs#L281)). The guard
line-scans `key: value` entries and can recognize only block-style top-level
keys ([schema.rs:205](../../dmls/src/overlay/schema.rs#L205)); a flow mapping's
first token is not parsed as `$schema` or `kind`. The `claim?` return then drops
the overlay instead of retaining the seeded model
([mod.rs:313](../../dmls/src/overlay/mod.rs#L313)).

Acceptance criterion 9 requires content-based standalone activation to retain
last-good semantic data during malformed edits without restricting supported
YAML presentation ([spec.md:732](spec.md#L732)). Current Level-1 tests cover
block-style pure and scalar-reference envelopes only. This user-observable
completion/hover continuity has no verification for a flow-style document.

Replace the line-oriented claim with a bounded, tolerant envelope recognizer
that understands both block and flow mappings while still refusing ordinary
YAML and raw JSON Schema. Add in-memory LSP transitions for valid flow pure and
tagged envelopes, malformed intermediate text, retained completion/hover, and
current-buffer diagnostics.

### High: AC13 remains red and its scoped exception is still unratified

DECISION: DEFER FOR NOW

Acceptance criterion 13 requires `just test-l2` to pass, and the specification
states that its three-test exception is **proposed — not approved**
([spec.md:750](spec.md#L750), [spec.md:755](spec.md#L755)). A fresh Level-2 run
using the Cargo-built shim passed **66/69** CLI tests. The only failures are the
three tests named by the proposal:

- `level2_code_block_inverts_to_light_in_dark_terminal`
- `level2_default_code_block_inverts_background_and_foreground`
- `level2_code_block_clears_inherited_dim_before_theme_colors`

This confirms Review 6's additional eight failures are repaired, but it does
not make the declared gate green or approve an exception. Real-terminal
rendering is correctly assigned to Level 2; the available Level-2 evidence is
simply failing. Restage/fix the three tests, or obtain the specification's
documented ratification before treating AC13 as satisfied.

## Prior Review Closure

- **Level-2 error tests bypass the built product — closed.** The three command
  builders now route through `md_shim()`. All eight `level2_errors` tests passed
  in the fresh WezTerm run, and their captured commands show the temporary shim
  path rather than a bare host `md`.
- **Canonical Level-2 release gate — still open, narrowed back to its recorded
  scope.** The CLI tier is 66/69; only the three proposed-exception tests fail.
  The exception remains unratified.

## Requirement Verification Levels

| Requirement | Appropriate level | Strongest verification present | Assessment |
| --- | --- | --- | --- |
| AC1–6: grammar, lowering, validation, serialization, and semantic arrays | Level 1 | Parser, conversion, validation, serialization, property, and filesystem tests | Appropriate and passing in the focused feature suites. |
| AC7: shared passive semantic/source-aware authority | Level 1 | Parser-parity, source-projection, structural-path, and malformed-buffer tests | Appropriate and passing for covered literal-key paths; pattern-key hover projection is omitted under AC9. |
| AC8: base schema and referenced declaration preparation | Level 1 | Base-schema, multi-hop, cycle, dependency, origin, exact-depth, over-depth, and cache tests | Appropriate and passing in the focused feature suites. |
| AC9: DMLS completion, hover, diagnostics, activation, and last-good state | Level 1 | In-memory LSP and overlay tests | **Incomplete.** No Level-1 verification exists for nested hover under ordinary semantic owners, pattern-key hover, or malformed flow-envelope last-good continuity; the implementation omits those behaviors. |
| AC10: passive analysis performs no file/process/shell/network side effects | Level 1 | Sentinel integration tests | Appropriate and passing. |
| AC11–12: recursion bounds and compatibility | Level 1 | Depth-boundary, baseline replay, import-name, and downstream compatibility tests | Appropriate for the tested surface. The fresh downstream Claudine check did not complete within the non-interactive compile ceiling. |
| AC13: terminal/editor presentation and area release gate | Level 2 for rendering; Level 1 for protocol semantics | Fresh CLI Level 2: 66/69 | **Not satisfied.** Three real-terminal assertions fail, and their proposed exception is unratified. |

No requirement concerns OS keyboard encoding, modifier visibility, hotkeys,
paste, IME, or mouse delivery, so Level 3 is not applicable. DMLS completion
and hover are LSP protocol behavior and should be verified at Level 1; terminal
rendering is correctly exercised at Level 2.

## Verification Performed

- `biscuit-file` resolved the specification and canonical review paths. The
  requested `@prompts/./_reviews/.../review-6.md` path does not exist in this
  worktree; the colocated canonical
  `darkmatter/features/2026-07-13-meta-schema/review-6.md` was updated instead.
- `sniff` identified `darkmatter`, `darkmatter-cli`, and `dmls` as the affected
  package-area crates, with Claudine as the direct repository consumer of the
  public semantic types.
- GitNexus reports CRITICAL upstream impact for `parse_property_definition`
  (78 affected symbols) and `parse_schema_declaration` (92), and LOW impact for
  `meta_schema_hover` (9). These high-blast-radius parser symbols were inspected
  without modification.
- Focused Darkmatter feature tests passed: meta-schema library binaries
  **37/37**, additional meta-schema/source/base-schema selections **20/20**, and
  DMLS meta-schema/structural/last-good selections **18/18**.
- Fresh Darkmatter CLI Level 2 passed **66/69** through real WezTerm panes. All
  eight `level2_errors` tests passed against the Cargo-built shim; the three
  named code-block tests failed after four nextest attempts each.
- A fresh canonical Level-1 area run and the scoped downstream Claudine run
  exceeded the session's bounded non-interactive compile ceiling before
  producing results. Review 6's recorded full Level-1 build/test/lint evidence
  therefore was not replaced by an inferred result.
- Darkmatter parsed every requested lifecycle property with the exact value in
  frontmatter, and specification validation completed. Review validation is
  blocked by the existing `schemas/feature-review.yaml` definition: its tagged
  envelope contains unsupported top-level `description` and `$schema` keys.
- No Rust source was modified, no formatting command was run, and the
  pre-existing unrelated `CLAUDE.md` worktree change was preserved.

## Production Readiness

**Not ready.** Complete and test hover for all semantically activated schema
regions, retain flow-style standalone activation through malformed edits, and
either restore the three Level-2 tests or ratify their exact scoped exception.
