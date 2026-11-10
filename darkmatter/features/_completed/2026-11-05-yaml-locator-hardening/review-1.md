---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
    - "For later confirmation: alias-related test rewrites moved into Phase 1 to keep both packages passing; I recommend retaining this sequencing."
    - "For later confirmation: anchored originals retain their existing spans, including an anchored scalar's anchor prefix, as the spec permits; I recommend retaining this behavior."
    - "For later confirmation: the YAML-only unexpected-<< explanation is built in the existing diagnostic-message builder; I recommend retaining its placement and scope."
    - "For later confirmation: the missing-child fixture uses a valid present sibling because the validator otherwise reports only the sibling's type error; I recommend retaining this isolated test."
    - "For later confirmation: editor parity tests distinguish identical messages by start offset, assert the existing diagnostic code, and put Unicode before the alias on the same line; I recommend retaining these stronger checks."
    - "For later confirmation: the fallback test uses a nonnumeric string against a numeric contract because the planned number-to-string case converts successfully; I recommend retaining the corrected fixture."
    - "For later confirmation: supported YAML locations and the remaining unproven-location fallback have separate documentation sections; I recommend retaining this organization."
    - "For later confirmation: alias coverage uses a boundary set and one traversal of indexed mapping pointers, with no traversal when there are no aliases; I recommend retaining this internal approach."
    - "For later confirmation: unavailable YAML positions use an empty position map instead of an optional map; I recommend retaining this internal representation."
    - "For later confirmation: dependent tests were limited to relevant Claudine consumers, excluding Messenger because it does not consume positions; I recommend retaining that verification scope."
    - "For later confirmation: separate cross-OS runs were left to CI, and no gate-skip calculation was needed because package gates ran; neither choice leaves implementation work deferred."
human_in_the_loop: "2026-11-08: author (Ken Snyder) accepted all 11 human review items as recommended; no reversals."
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-08T18:14:23-08:00
spec: 2026-11-05-yaml-locator-hardening/spec.md
implemented: false
description: "A **feature** review of `2026-11-05-yaml-locator-hardening/spec.md`"
feature: 2026-11-05-yaml-locator-hardening/review-1.md
---

# Review 1: YAML Locator Hardening

The feature is production ready under the specification's contract. No findings require repair. The implementation preserves parsed YAML values and validation behavior while reporting alias descendants at the alias use and restoring positions in multi-line flow collections.

## Findings

None. This is the first implementation review; there are no previous findings to carry forward.

## Contract and verification

Reviewed the shared projection and mapping-order index, Darkmatter's data locator and diagnostic builder, whole-file YAML loading, frontmatter projection, editor diagnostics, changed tests, and documentation. The two requested parallel readers independently collected implementation decisions from the log and specification before the comparison.

| Requirement | Evidence reviewed | Appropriate verification |
| --- | --- | --- |
| Aliases to scalars, sequences, and mappings preserve neighboring positions | `simplified-schema/lib/tests/l1/alias_positions.rs`; `darkmatter/lib/tests/l1/load_data_diagnostics.rs` | L1 public library results |
| Repeated uses and failing or missing children retain distinct paths and token-only alias ranges | Shared-map tests assert starts and sliced range text; composition tests assert diagnostic paths and counts | L1 |
| Literal plain and quoted `<<`, explicit siblings, and sequences of aliases retain their data shape | Shared-map and composition fixtures; unexpected-key tests assert the exact hint and unchanged ordinary-key message | L1 |
| Multi-line flow collections handle nested mappings, comments, quoted delimiters, alias items, and following values | Composition matrix and shipped CLI flow case | L1 library and hermetic CLI |
| Projection still refuses unrelated mismatches and respects escaped segment boundaries | Mapping-key/order mismatch tests, existing mapping-count refusal tests, boundary tests for `/a`, `/ab`, and escaped slash keys | L1 |
| Byte columns, character columns, BOM, CRLF, and frontmatter offsets retain each surface's convention | Shared-map encoding tests; composition encoding tests; shared frontmatter fixtures and whole-file YAML validation | L1 |
| Editor diagnostics range only the alias token with correct encoded columns | Frontmatter parity suite and headless YAML editor harness, including Unicode before an alias on the same line | L1 protocol harness |
| CLI keeps the Markdown call site in `source` and YAML position in `data_location` | CLI alias case validates the existing compose output schema, compares the call site with the plain-data control, and checks that no later command runs | L1 hermetic CLI |
| Ordering, report limits, and malformed-YAML handling remain intact | Composition ordering/count assertions and existing cap tests; malformed YAML remains on the unchanged loader path | L1 plus code inspection |

No requirement here depends on terminal rendering or keyboard encoding. Real-terminal or OS-keyboard tests would not strengthen these parser, record, and editor-protocol checks.

The completeness check still requires authored mappings plus mappings covered by alias boundaries to equal the indexed mapping count. Authored non-alias mappings retain exact key/order checks. Alias recognition uses the shared token reader at authored scalar starts; quoted strings, comments, and block scalars do not become alias boundaries. Coverage scans the index once and tests escaped segment prefixes; it does not expand aliases again or search the whole document for each use. Downstream data ordering reads parsed container order independently of the newly available coordinates, and the existing record cap is applied afterward.

## Input robustness

This change adds no configuration fields or field validators. The YAML value remains data governed by the caller's contract: absence produces the existing required-member result, null remains parsed null, wrong scalar/container/element types produce the existing contract result, and empty values remain explicit values. The change only locates those results; it does not introduce a permissive default.

The document loader is unchanged. Reviewed its public tests for duplicate keys, malformed content, a second YAML document, non-string keys, inadmissible numbers, valid document markers, empty/scalar/sequence roots, BOM, and CRLF. Data-file tests also cover rejected tags and malformed input containing aliases. The whole-file validation and composition paths both retain the shared admitted-YAML loader. Successful alias controls demonstrate that the edited mismatch fixtures are load-bearing; the CLI control permits the later action, while the failing alias fixture prevents it.

The requested code-smell search found no new defaulting deserializer or silently filtered data field. Existing optional coordinates and failed precise-projection results represent unknown locations, as the spec requires. The existing fallible parse in the convenience position-map builder intentionally selects the preserved top-level fallback; the data locator uses only proven byte ranges.

## Verification performed during review

All completed runs used the package-area `just` recipes and nextest:

- `simplified-schema`: `just test alias_positions` passed 12 selected tests; its minimal-feature gate also passed all 1,454 tests.
- `darkmatter`: `just test yaml` passed 490 tests across library, CLI, and language-server targets.
- `darkmatter`: `just test load_data` passed 352 tests, including the new composition and CLI cases.
- `darkmatter`: `just test positions` passed 120 tests, including frontmatter/editor position parity.

These filtered runs overlap; their counts are not a count of unique tests. Filtered-out tests were not treated as executed evidence. The new integration files are declared by the consolidated L1 targets, and their names do not move them into an unrun tier. No selected test was reported slow. Linking emitted an unwind-table-size warning without failing compilation or tests.

An initial attempt to combine filters with `|` was interpreted by the recipe as shell syntax and failed before testing; the successful simple-filter runs above replaced it. The implementation log separately records full package test and lint passes, including Darkmatter's wasm check. This review did not repeat those full lint gates or perform separate cross-OS runs; cross-OS proof is outside readiness under the review instructions.

## Implementation decisions (informational)

None of these items meets the stopping test: each either follows an explicit spec decision or is a reversible internal/test/documentation choice I recommend. The author can confirm or reverse them after this review cycle; none blocks readiness.

- **Test sequencing:** three Darkmatter alias expectations were updated in Phase 1 rather than Phase 2 so the shared behavior change did not leave dependent tests failing between phases.
- **Anchored originals:** anchored scalar spans still begin at their anchor prefix. The spec explicitly preserves existing anchored-node spans; the nested `/base/port` case still identifies `x`.
- **Merge-key explanation:** the hint is assembled in the existing diagnostic builder, restricted to YAML unexpected-property failures with a decoded final key of `<<`. This implements the author's existing wording and scope without relocating message construction.
- **Missing-child fixture:** a valid present `port` isolates the missing required child because the current validator suppresses that missing-child report when the same object has a type failure. Separate existing tests verify missing-member ordering.
- **Editor measurement:** parity matching includes the expected byte start to distinguish identical messages. Independent expected starts and token text still anchor the test. The editor fixture uses the existing type-mismatch code and Unicode before the alias on its own line, which actually distinguishes encoded columns.
- **Fallback fixture:** a nonnumeric string against a numeric contract replaces the planned number-to-string mismatch, which would succeed through coercion. The anchored-key layout and pointer-only assertion remain intact.
- **Documentation organization:** supported alias/flow positions and literal merge keys have their own section; the existing unproven-position heading now describes only the remaining fallback.
- **Internal coverage strategy:** a set of escaped alias pointers and one traversal of the mapping index account for expanded mappings. Documents without aliases skip that traversal. This follows the spec's permitted escaped-boundary approach.
- **Unavailable positions:** an empty position map replaces an optional map in the data locator, keeping unknown coordinates explicit through the map's existing API.
- **Verification scope:** the implementer exercised relevant Claudine position/schema consumers and omitted Messenger, which does not consume positions. Separate cross-OS runs were left to CI. The gate-skip calculator was not consulted because the package gates were run.

No in-scope implementation work was deferred. Preserving anchored-original spans, leaving arbitrary unprovable layouts on the fallback, and excluding merge inheritance and malformed-input semantic changes follow explicit spec boundaries. The log records the intentional replacement of the earlier whole-file pointer-only exception without rewriting its historical specification. Comment drift describing universal alias refusal was corrected alongside the behavior. The active feature directory remains in place for the author's review process.
