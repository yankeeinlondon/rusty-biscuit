---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (low): Crate example fails with supported rendering-only features"
human_review: true
human_review_items:
    - "For later confirmation: the additional schema digest now ignores object-key order; retaining this avoids needless validator cache misses."
    - "For later confirmation: the initial baseline suite continued after failures to collect the complete failure list; no action is needed."
    - "For later confirmation: the JSON feature also enables optional serde to reuse the existing serializer and preserve its bytes."
    - "For later confirmation: the public helpers are named canonical_json, xx_hash_json, and blake3_hash_json, and rendering returns a String directly."
    - "For later confirmation: insertion-ordered JSON maps are enabled only for biscuit-hash tests, allowing them to prove rendering does not mutate input order."
    - "For later confirmation: biscuit-hash lint now checks all features, matching its test recipe."
    - "For later confirmation: ordered inputs also pass through frontmatter interpolation, shell-value APIs, and their Claudine callers to prevent order loss before merging."
    - "For later confirmation: an additional composition preparation merge was corrected so the receiving document fixes key position."
    - "For later confirmation: the whole-frontmatter expression keeps an authored ctx property at its original position."
    - "For later confirmation: reviewed changes from another implementation were retained; schema-validation reports remain alphabetical while JSON5 and generated schemas follow construction order."
    - "For later confirmation: schema removals preserve remaining key positions, union wrappers start with anyOf, and order-only golden changes were retained."
    - "For later confirmation: the shared-file warning in the implementation log is historical; the other session's changes were committed before the final phase."
    - "For later confirmation: CLI tests use the existing get, set, and rm commands; nested edits replace the parent object because dotted names are literal keys."
    - "For later confirmation: represented-state hashes retain text and key-order sensitivity, and the typed DMLS configuration hash stays unchanged."
    - "For later confirmation: hash documentation was updated during implementation rather than waiting for the final documentation phase."
    - "For later confirmation: five superseded renderer tests were removed, including the extra empty-object test; biscuit-hash supplies their replacement coverage."
    - "For later confirmation: the spec status was set to implemented alongside its implementation metadata."
    - "For later confirmation: no crate-wide ban on position-changing map removal was added; the broader audit would exceed the needed repair."
    - "For later confirmation: the previously deferred feature-specific doctest failure is now finding R1-F1 and needs the small repair described below."
    - "For later confirmation: strict baselines are refreshed explicitly through save; mode tracking and an automated mixed-mode test were omitted, and the mode limitation is documented."
    - "For later confirmation: cross-platform runs remain CI evidence, and unrelated dependent suites and float parsing remain outside this fix as specified."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-29T14:32:03-07:00
spec: 2026-10-23-frontmatter-key-order/spec.md
implemented: true
description: "A **fix** review of `2026-10-23-frontmatter-key-order/spec.md`"
fix: 2026-10-23-frontmatter-key-order/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/fixes/2026-10-23-frontmatter-key-order/log.md
implemented_by: claude/opus
next: 2026-10-23-frontmatter-key-order/review-2.md
---

# Review 1: Frontmatter Key Order

The frontmatter order and hashing behavior meets the specification in the
reviewed paths. One low-priority documentation defect remains, so this review
sets `ready: false`. No finding needs a human decision before repair.

## Findings

### R1-F1 (low): Crate example fails with supported rendering-only features

**Authority: defect.** The biscuit-hash crate's
[crate-level example](/Volumes/coding/wt/rusty-biscuit/feat-better-research/biscuit-hash/lib/src/lib.rs:22) imports
`xx_hash` unconditionally. Running the documentation tests with the supported
rendering-only configuration fails with `E0432: unresolved import
biscuit_hash::xx_hash`. The implementation log identifies this existing defect
but defers it because current recipes do not run that combination. It is a
realistic library verification command, and the repair is small.

Reproductions from the repository root:

```sh
cargo test -p biscuit-hash --doc --no-default-features --features json
cargo test -p biscuit-hash --doc --no-default-features --features json,blake3
```

Both fail only on that crate-level example; the canonical JSON example passes.
With default features and `json`, the documentation tests pass. The library
itself and its nextest tests compile and pass for all three required JSON
feature combinations, so this is a documentation feature-gating defect rather
than a broken renderer.

Gate the existing example's executable block with `#[cfg(feature =
"xx_hash")]`, preserving its default-feature demonstration. Rerun the two
commands above and the default-feature control. No new test framework, CI
matrix, or API change is needed.

## Verification and coverage

All requirements here need Level 1 verification: they concern values, serialized
bytes, cache identities, and ordinary CLI persistence. No terminal encoder,
rendering, keyboard, or browser behavior changes, so Level 2 and Level 3 evidence
is unnecessary.

Review-run evidence on macOS:

- `just test` in darkmatter: 13,768 passed, 10 skipped, 68.766 seconds.
- `just test` in biscuit-hash: 90 passed.
- Focused darkmatter runs: 216 order-related tests and four CLI/hash-order
  tests passed before the full suite.
- biscuit-hash library nextest with default features disabled: `json` 11
  passed; `json,xx_hash` 41 passed; `json,blake3` 24 passed.
- All three no-default JSON feature builds and the ordinary default build
  passed `cargo check`.
- Default-feature documentation tests with `json`: eight passed. The two
  no-xxHash documentation configurations fail as detailed in R1-F1.
- Lint results are taken from the implementation log, which records passing
  `just lint` in both areas; lint was not rerun during this review.

| Spec requirement | Strongest relevant evidence | Assessment |
| --- | --- | --- |
| Historical hash identities and delimiter | Fixed expected hash test and benchmark manifest identity test | Level 1; historical entry format retained |
| Reordering at the top level, two nested levels, and inside arrays | Parsed-document hash matrix across legacy and option-based kinds, strict keys-only checks, body controls | Level 1; correct order sensitivity |
| Shared rendering and optional algorithms | Exact-byte, scalar, Unicode, escaping, array-order, nonmutation, and byte-helper equivalence tests | Level 1; correct rendering and hashes |
| TOC and validator identities | Whole-frontmatter normalized hash tests and reordered-schema cache tests | Level 1; objects normalize and arrays remain ordered |
| Shallow and recursive merge order | Explicit key enumeration under both value preferences, defaults and conflict tests, null/array tests, transclusion and directive-set pipeline tests | Level 1; key position is independent of precedence |
| CLI read/edit/save/reread and saved baselines | Repeated get/set/rm process round trip; strict and non-strict save/diff tests | Level 1 process; passed |
| Explicit features and package gates | Manifest inspection, feature builds and nextest feature runs; implementation log's lint evidence | Functional feature combinations pass; documentation exception is R1-F1 |
| Documentation and audit | Hash audit, output-change explanations, strict-baseline correction, topic pages, README links, dependency docs and skills | Present; decisions assessed below |

The added integration modules are declared in the library and CLI `tests/l1/main.rs`
targets. Their names keep them in L1, and the review's test runs execute them.
The shipped-schema golden corpus covers generated output, while the CLI tests
exercise the normal commands against temporary files. Persistence is checked
through repeated reads and writes rather than only comparing in-memory values.

The order audit covered shallow frontmatter merges and defaults; recursive
document/external merges; the composition preparation merge; child inheritance;
directive object/property overrides; special replacement maps; whole-frontmatter
expression projection; ordered shell-state inputs; and schema conversion,
resolution, and scope removals. Remaining hash maps in the inspected composition
paths are lookup tables rather than sources of serialized key order.

The hash audit covered legacy and option-based frontmatter hashes, strict and
keys-only components, normalized and raw TOC identities, both schema digest
paths, DMLS configuration/text identity, composition cache inputs, and
represented-state provenance. The current composition cache deliberately
includes key-order identity because expression iteration can observe that
order; this appropriately differs from the non-strict document hash.

The input robustness matrix does not apply: this fix changes ordering and
rendering, not configuration field validation or document loading rules. YAML
frontmatter is the Markdown text grammar in the review prompt's exclusion.

## Implementation decisions (informational)

Two agents independently read the spec and implementation log before the code
review. The following combines their inventories. None requires stopping for
the author: each has a recommended option within the settled contract, is
explicitly authorized, or is cheap to reverse. The frontmatter summaries invite
later confirmation only; they do not block repair.

1. **Additional schema digest.** Canonicalizing the second digest is within the
   required structured-hash audit. Retain it and its unchanged file-format
   detection; reordered schemas should reuse validators.
2. **Complete failure inventory.** Continuing the initial suite after failures
   was a diagnostic execution choice. Retain the evidence; no product decision
   is involved.
3. **Optional serde dependency.** Reusing the serializer requires both serde
   and serde_json. Retain this bounded dependency choice rather than introduce
   a second JSON encoder that could change escaping or numeric bytes.
4. **Names and error contract.** The spec expressly lets the implementer name
   the helpers and choose infallible rendering. Retain `canonical_json`,
   `xx_hash_json`, and `blake3_hash_json`; the in-memory writer and valid JSON
   value representation justify the invariant checks.
5. **Test-only ordered maps.** Retain the dev-dependency feature that proves
   input order is untouched without imposing insertion order on production
   biscuit-hash consumers.
6. **All-features lint.** Retain it so the newly gated implementation receives
   lint coverage, as the acceptance criterion requires.
7. **Additional ordered boundaries.** Retain the changes to interpolation,
   shell-value APIs and Claudine callers. The spec authorizes the smallest
   ordered representation after tracing callers; keeping a randomized map at
   these boundaries would lose order before the merge could preserve it. The
   duplicate namespace helper was removed as part of this conversion.
8. **Additional preparation merge.** Retain receiving-document-first order in
   composition preparation. This is a sibling of the explicitly named merge
   sites, not a new precedence rule.
9. **Authored ctx position.** Retain its position in whole-frontmatter output;
   re-appending it would violate the author-order contract.
10. **Concurrent output updates.** Retain the reviewed JSON5 and conversion
    snapshots. Retain explicit sorting of the legacy validation report, whose
    byte-comparison test still compares the two routes. This report is not an
    authored frontmatter mapping.
11. **Schema removals and wrapper order.** Retain position-preserving removals
    and the construction order with `anyOf` before annotations. The log records
    order-only comparison of all changed golden outputs. No schema content
    change is justified merely by updating a snapshot.
12. **Shared-file warning.** Another session's named-type changes were present
    during implementation. The final phase records that they were committed;
    this is historical coordination information, not pending work or a ruling.
13. **Actual CLI route.** Retain process tests of `md get`, `md set`, and
    `md rm`, the shipped frontmatter commands. Replacing a parent object is the
    correct way to exercise nested edits through this existing interface; the
    spec does not require adding dotted-path edits or a new command group.
14. **Intentional hash sensitivity.** Retain raw represented-state identity
    and DMLS's typed configuration serialization. The spec explicitly permits
    intentional order sensitivity and requires document text to remain
    byte-sensitive.
15. **Documentation timing.** Updating the hash reference during the behavior
    change rather than waiting for the final phase is harmless and keeps docs
    aligned with the code.
16. **Moved test count.** Removing five rather than four tests is justified:
    the fifth also tested the deleted renderer, and replacement empty-container
    coverage is present in biscuit-hash.
17. **Spec status.** Using the existing `implemented` status alongside the
    implementation metadata is consistent with implementation ready for review.
18. **Broader removal-method lint omitted.** A crate-wide ban would require a
    broader audit. The targeted repairs and order tests suffice; do not add a
    new ban solely for this fix.
19. **Feature-specific doctest deferred.** The stated reason was that no current
    recipe runs it. Recommend reversing this inexpensive deferral: R1-F1
    describes the repair and needs no author choice.
20. **Strict baselines and mixed modes.** No automatic migration is permitted
    by the spec. Retain explicit save-based refresh. Tracking the mode in stored
    data is outside scope; the mixed-mode limitation was manually observed and
    documented, and does not require a new automated test for this fix.
21. **Verification and exclusions.** Cross-platform evidence belongs to CI and
    does not block this review. Unrelated dependent suites and float parsing
    are explicitly outside scope. Phase-local deferrals of renderer deletion,
    snapshot review, and documentation were completed in later phases; no
    corresponding implementation task remains open.

## Previous findings

This is the first implementation review. There are no carried findings or
new siblings of findings from an earlier review.
