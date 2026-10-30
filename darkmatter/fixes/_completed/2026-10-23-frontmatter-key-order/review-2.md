---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-29T14:37:13-07:00
spec: 2026-10-23-frontmatter-key-order/spec.md
implemented: false
description: "A **fix** review of `2026-10-23-frontmatter-key-order/spec.md`"
fix: 2026-10-23-frontmatter-key-order/review-2.md
previous: 2026-10-23-frontmatter-key-order/review-1.md
---

# Review 2: Frontmatter Key Order

The fix is production ready. The previous documentation defect is verified
fixed across every instance in the repair ledger. No new findings or decisions
requiring a human were identified. The informational decisions in review 1
remain unchanged; this review does not request that they be reaffirmed.

## Previous Finding Verification

Read review-1.md and the repair's Finding Ledger in log.md before inspecting
the repair. Review 1 contains the only earlier finding and no Human rulings
section. The table below checks every child row rather than relying on the
implementer's completion flags.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Crate example fails with supported rendering-only features | R1-F1.1–R1-F1.8, detailed below | verified fixed | The crate example's executable lines now sit inside a hidden `#[cfg(feature = "xx_hash")]` block. All nine feature configurations below pass independently rerun documentation tests. | Nothing |
| R1-F1.1: Rendering-only crate example | No default features, `json` | verified fixed | 2 documentation tests passed; the missing import is no longer compiled. | Nothing |
| R1-F1.2: Rendering with BLAKE3 | No default features, `json,blake3` | verified fixed | 7 documentation tests passed. | Nothing |
| R1-F1.3: Default-feature control | Default features plus `json`; all features; explicit `json,xx_hash` without defaults | verified fixed | 8, 18, and 8 documentation tests passed respectively. Source inspection confirms the original example still executes when xxHash is enabled. | Nothing |
| R1-F1.4: Other configurations without xxHash | No features; `blake3`; `argon2id`; `json,argon2id`, all without defaults | verified fixed | 1, 6, 6, and 7 documentation tests passed respectively. | Nothing |
| R1-F1.5: Canonical renderer example | `json.rs` example under rendering alone | verified fixed | Its two imports exist whenever the module is enabled; the `json`-only run executes the example successfully. No repair was needed at this sibling. | Nothing |
| R1-F1.6: Algorithm examples | Module and item examples in `xx.rs`, `blake.rs`, and `argon.rs` | verified fixed | Module declarations and re-exports use the matching algorithm feature guards; configurations above exercise each algorithm independently. No repair was needed at these siblings. | Nothing |
| R1-F1.7: Feature-specific documentation links | JSON hash helper links to byte-hash helpers | verified fixed | Each JSON hash helper is guarded by the same algorithm feature as its link target. No repair was needed. | Nothing |
| R1-F1.8: Included external examples | README or topic pages included into crate documentation | verified fixed | Search of `biscuit-hash/lib/src` found no `include_str!` or `#[doc = …]` inclusion; no additional compiled examples exist at this site. | Nothing |

## Unblocked Findings

None.

## Blocked Findings

None.

## Spec Review and Verification

Every behavioral requirement in this fix needs Level 1 verification: these are
in-memory values, serialized bytes, cache identities, or ordinary file edits
through the CLI. No requirement depends on terminal rendering, keyboard
encoding, or browser behavior, so Level 2 and Level 3 are unnecessary.

| Requirement | Implementation and strongest relevant verification | Assessment |
| --- | --- | --- |
| Historical frontmatter identity | Non-strict hashing still sorts top-level keys and joins `key:canonical-value` entries with literal backslash plus `n`. The fixed expected-hash test pins `26b30b2982c8c394`; the benchmark manifest test checks recorded identities. | Level 1; compatible historical format |
| Recursive order independence and strict sensitivity | Parsed-document matrix separately reorders top-level keys, nested mappings, and mappings inside arrays. It checks legacy/shared and option-based frontmatter results, keys-only behavior, and unchanged body components. Array reorder controls remain significant. | Level 1; required distinctions asserted |
| Shared JSON rendering and optional algorithms | Borrowed recursive serializer sorts object-key references, preserves array positions, and delegates scalar serialization to serde. Tests assert exact bytes, escaping, Unicode order, scalar roots, empty containers, unchanged input order, and agreement with byte-hash helpers. | Level 1; biscuit-hash suite passed |
| TOC and schema cache identity | TOC uses canonical whole-object hashing with its empty sentinel retained. Both structural and context-aware validator tests check cache reuse for object reorder and distinct entries for array reorder. | Level 1; correct normalization boundary |
| Author order and shallow merges | Darkmatter explicitly enables `preserve_order`. Existing-entry replacement retains position; incoming-only keys append. Tests enumerate keys, include every shallow strategy and defaults, and verify whole nested-object replacement. Parsed order metadata remains separate from mutation state. | Level 1; order and precedence remain separate |
| Recursive composition | Reviewed the merge helper, effective-state constructors/builder, preparation merge, runtime context merge, replacement-map consumers, directive overlays, and inherited child inputs. Ordered maps reach the merge; base-first insertion is independent of value preference. Tests enumerate nested keys and check null, array, and scalar precedence. | Level 1 in-process and composition integration |
| Repeated CLI persistence | `frontmatter_key_order` exercises get/set/rm twice through the normal binary, replacing existing and nested properties, appending and removing properties, and rereading saved text. Baseline tests check nested-only reorder causes no non-strict rewrite or timestamp bump; strict refresh requires explicit save. | Level 1 process; appropriate boundary |
| Feature combinations, gates, and records | Manifest guards and optional dependencies inspected. Documentation configurations independently rerun. Earlier implementation and review record feature-specific nextest/check runs, explicit feature-tree verification, lint in both areas, hash audit, output review, strict-baseline correction, and departures. | Required records present; documentation exception repaired |

The added integration modules are declared in their library/CLI
`tests/l1/main.rs` targets and have names selected by L1. Their temporary
fixtures and source-file-based requests avoid repository discovery from the
process directory. The shipped schema snapshot corpus checks deterministic
construction order, while process tests cross the actual edit/save boundary.

The structured-hash audit distinguishes semantic document hashes from raw
source and represented-state identities. Composition cache identity also
retains observable key order; normalizing a semantic frontmatter hash does not
justify dropping order from a cache whose expressions can iterate mappings.
The DMLS text/configuration identity retains byte-sensitive document text.

No input robustness matrix is required: this work changes serialization,
ordering, hashing, and merging, not configuration validators or document
loading rules. Frontmatter text grammar is excluded by the review instructions.

Review-run evidence on macOS:

- Nine biscuit-hash documentation configurations passed: rendering alone,
  rendering with each supported algorithm combination above, no features,
  individual optional algorithms, default plus JSON, and all features.
- `just test` in biscuit-hash: 90 passed, none skipped.
- `just test` in darkmatter: 13,768 passed, 10 skipped, 69.036 seconds.
  This includes the benchmark identity, key-order, merge, schema-cache, and
  CLI persistence tests listed above. The build emitted the existing macOS
  linker compact-unwind size warning; it did not prevent the run.
- Lint and feature-specific compile/nextest evidence from the preceding
  implementation and review was inspected rather than rerun for the
  documentation-only repair. Cross-OS evidence remains CI's responsibility.

## Recurrence

None. R1-F1 is fully repaired; no unchanged instance, repair regression, or
missed sibling remains in this review.
