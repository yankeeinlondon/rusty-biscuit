---
$schema: feature-review.yaml
ready: false
findings:
    - title: Date replacement can delete a trailing comment after a quote inside a plain scalar
      priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-09-29T12:55:20-07:00
spec: 2026-09-28-hash-writer-byte-fidelity/spec.md
implemented: true
next: 2026-09-28-hash-writer-byte-fidelity/review-3.md
implemented_by: claude/opus
log: darkmatter/fixes/2026-09-28-hash-writer-byte-fidelity/log.md
description: "A **fix** review of `2026-09-28-hash-writer-byte-fidelity/spec.md`"
fix: 2026-09-28-hash-writer-byte-fidelity/review-2.md
previous: 2026-09-28-hash-writer-byte-fidelity/review-1.md
---

# Review of Hash Writer Byte Fidelity

The fix is **not production ready**. Both findings from review 1 are resolved, including the additional restoration and repair sites identified during implementation. One different defect remains: replacing an unquoted date value containing an unmatched quote can silently delete its trailing comment. No human decision is needed to fix it.

The review examined the specification, implementation plan and log, current changes, both production callers, the related text editors, current documentation, and test placement. Production code and existing tests were left unchanged after removing temporary review probes.

## Previous finding disposition

Review 1 has a single Findings section rather than separate Unblocked Findings and Blocked Findings sections. Its metadata declares no blocked findings, and the implementation log defers neither finding. There were no blocked findings to become unblocked.

| Previous finding | Implementation and verification | Result |
| --- | --- | --- |
| High — Hash replacement can silently change unrelated alias values | The darkmatter writer compares parsed values outside the managed properties before returning text. Tests cover anchors on scalar and nested hash values, date bumps on and off, and a later declaration that keeps the alias unchanged. CLI and Claudine tests prove refusal leaves the file unchanged. Property restoration now applies the same preservation check and tests both adding and dropping an anchor. | Resolved |
| Medium — Date writing mistakes indented comments for collection values | The darkmatter date writer uses the parsed value type and edits only the value's bytes. Exact-byte tests cover empty, plain, and quoted values with indented comments under LF, CRLF, and lone CR, plus values on the following line. Both production callers have persistent tests. Claudine's repair scanner now recognizes indented comments below plain values. | Resolved |

The previous review allowed either support for broader scalar layouts or explicit documentation of the narrower contract. Block scalars and values continued across lines remain refused, with an accurate diagnostic and current documentation explaining how to rewrite the date onto one line. This satisfies that requested correction; it is not an unresolved collection-type error.

## Unblocked Findings

### Medium — Date replacement can delete a trailing comment after a quote inside a plain scalar

**Defect class:** a YAML comment scanner treats quote characters inside an already plain scalar as opening quoted syntax, so a text edit includes and deletes a real trailing comment.

In the darkmatter package, [rewrite_date_scalar](../../lib/src/markdown/hash/write.rs:1044), which replaces the existing date value, uses [yaml_comment_start](../../lib/src/markdown/hash/write.rs:1086) to separate the value from its comment. That scanner enters quote mode whenever it sees an apostrophe or double quote. YAML permits those characters inside an unquoted scalar; they do not start quoted syntax there.

For example, `md hash --save doc.md` succeeds on this parseable document:

```yaml
---
hash: aaaa111111111111-bbbb222222222222
last_updated: yesterday's date   # keep this explanation
author: A
---
Changed body
```

When the date is bumped, the result contains:

```yaml
last_updated: 2026-09-29
```

It should contain:

```yaml
last_updated: 2026-09-29   # keep this explanation
```

The apostrophe prevents the scanner from recognizing `#` as the comment start. Parsing the proposed replacement span alone does not catch the mistake: the YAML parser correctly ignores the comment and returns the same old value. The writer then replaces the entire span, including the comment. The new parsed-value preservation check cannot protect comments because they are not part of the parsed values.

This violates the specification's requirement to retain trailing comments and authored whitespace. The existing date does not need to parse as a calendar date: the writer intentionally replaces ordinary scalar values, including strings, numbers, and booleans. Silently losing the explanation is therefore a byte-preservation defect, rather than an invalid-date refusal.

The sweep changed only the scalar spelling in the same fixture. The failing spellings were `yesterday's date` and `unknown "date`; controls were `ordinary`, `'yesterday''s date'`, and `"unknown date"`. Every shape was exercised under LF, CRLF, and lone CR. For the library writer, CLI, and leaf locator, both inline and following-line values were checked.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| darkmatter public text hash writer | Plain scalar with internal apostrophe or double quote, then a trailing comment; inline and following-line values | Returns successful output with `# keep` deleted in all three terminator forms | Replace only the scalar, preserving the authored spacing and comment |
| darkmatter-cli `md hash --save` | Same two failing spellings and both layouts | Exits successfully and persists the comment deletion | Persist a date replacement that retains the comment |
| Claudine closure write-back | Same two failing spellings, inline layout | Successfully writes the bumped date and deletes the comment in all three terminator forms | Retain the comment before the atomic write |
| darkmatter writer and both callers, controls | Ordinary plain value and properly single- or double-quoted values | Comment survives | Clean |
| darkmatter public leaf locator and its scalar-decoding path | All five spellings, both layouts; splice a quoted date into the returned span | Correctly excludes the trailing comment; the splice retains it | Clean; reuse the existing scalar-location rules where practical |
| darkmatter public property restoration | Same fixtures, restoring a different property from the snapshot | Date spelling and comment survive | Clean |
| Claudine public frontmatter repair | Same spellings in unchanged authored properties and newly added plain properties | Unchanged authored text survives; repaired agent text retains `# keep` as part of its intended value | Clean under the repair API's documented interpretation of new agent text |
| Hash-node replacement and map-based frontmatter writers | Source review of ownership boundaries | Complete managed hash replacement owns its comments; map serializers do not promise comment retention | Outside the date-comment preservation contract |

The leaf locator also serves Claudine's value encoding; its clean scalar spans are the shared boundary used by that path. Its existing committed tests exercise the encoding splice and whole-value preservation. The node scanners delimit whole properties but do not split inline comments. Schema-expression delimiter scanners operate on decoded expression syntax rather than YAML date values and are not sibling comment editors.

**Required correction:** recognize quoted syntax only when it begins the scalar, or use the existing scalar locator/decoder to identify the value's exact end. Preserve the current refusal of anchors, aliases, tags, collections, and unsupported multiline scalar layouts. Changing parsed-value validation alone will not fix this defect.

Extend the existing table-driven scalar coverage with internal apostrophe and double-quote cases, genuine quoted controls, and exact expected comment bytes. Cover inline and following-line layouts, the three supported terminators, and saves without a date bump. Add persistent CLI and Claudine tests asserting the written file bytes, so a successful exit cannot hide comment loss. Update relevant comments or documentation if the scalar-location implementation changes.

## Blocked Findings

None.

## Recurrence

The only earlier implementation review is review 1. Neither of its defect classes recurs: unrelated alias values are now checked semantically, and physical line count no longer determines whether the date is a collection. The new finding concerns the lexical boundary between a plain scalar and its trailing comment. Although both date findings involve comments, their causes and required corrections differ. `recurrence` is therefore `false`.

## Input robustness matrix

YAML frontmatter is the only input format affected. The load-bearing fields are the configurable managed hash property and `last_updated`. The public writer receives a save decision; it deliberately replaces an arbitrary managed hash node. The CLI's separate stored-hash shape validation is unchanged by this fix and is not equivalent to the writer's input contract.

The existing canonical hash/date fixture supplied the positive control. The committed matrix and focused tests were run; temporary public-writer probes filled the numeric, boolean, mixed-element, empty-collection, and trailing-invalid-content cells, changing one field at a time and checking the public write outcome and successful output's values/body.

| Input shape | Managed hash property | `last_updated`, date bump requested |
| --- | --- | --- |
| Positive control | Replaced with requested hash | Replaced with requested date |
| Absent | Inserted | Inserted |
| Explicit null | Replaced | Replaced |
| Empty YAML value | Replaced | Date inserted; comments retained in covered layouts |
| Number or boolean instead of string | Whole managed node replaced | Scalar replaced |
| Mixed string/number sequence | Whole managed node replaced | Refused as collection |
| All-number sequence | Whole managed node replaced | Refused as collection |
| Empty sequence or mapping | Whole managed node replaced | Refused as collection |
| Duplicate semantic key, including quoted spelling | Refused | Refused |
| Invalid syntax or trailing garbage inside frontmatter | Refused | Refused |
| Anchor, alias, or tag | Replacement allowed only if output parses and unmanaged values remain equal | Refused |
| Indented comments after the value | Whole managed node replacement | Retained |
| Scalar alone on following line | Whole managed node replacement | Replaced while retaining surrounding bytes |
| Block scalar or scalar continued across lines | Whole managed node replacement | Refused with documented layout diagnostic |
| Plain scalar containing unmatched quote, followed by comment | Whole managed node replacement | Date written, comment deleted: finding above |

Absence, explicit null, and an empty value lead to distinct source edits even when their resulting values agree. Collection elements are never silently filtered. Text after the closing frontmatter delimiter is Markdown body content and is preserved by the hash writer. The no-write decision still returns `None` without parsing. The permanent matrix does not yet include every numeric/boolean and mixed/empty-collection cell exercised during review; consolidate those cases into it when adding the finding's scalar cases.

## Requirement verification and execution evidence

Every requirement here concerns parsing, exact text bytes, or filesystem persistence. Level 1 is the appropriate verification level. No terminal rendering or keyboard behavior is promised, so Level 2 or Level 3 is unnecessary.

| Requirement | Strongest committed verification | Assessment |
| --- | --- | --- |
| Six original defects, including empty date under LF/CRLF, lone CR, and BOM insertion | Level-1 public writer tests with exact bytes or typed refusal | Pass |
| Anchored, aliased, and tagged dates; no-bump and no-write behavior | Level-1 writer tests and CLI refusal/no-write tests | Pass |
| Mixed terminators, hash-only replacement, inserted date, existing BOM, unterminated body | Level-1 writer preservation matrix and exact-byte tests | Pass |
| Unmanaged parsed-value and body preservation | Level-1 writer, CLI, and Claudine tests; reused-anchor controls and refusal tests | Pass for the parsed-value contract |
| Date comment and whitespace preservation | Level-1 exact-byte tests for ordinary/quoted dates and indented comments | Gap: internal quotes in a plain scalar can delete the trailing comment |
| Repeated save and restoration stability | Level-1 writer, CLI, and Claudine round trips | Pass for covered fixtures |
| Current docs and relevant code comments | Source review of CLI guide, Claudine composition topic, skill, and writer docs | Describe the intended contract and narrower supported layouts; comment-loss finding violates that contract |

Executed successfully on this host:

- `cd darkmatter && just test 'hash::write::'`: 60 tests.
- `cd darkmatter && just test hash_kind_save_diff::`: 26 tests.
- `cd claudine && just test-library 'composition::closure::'`: 45 tests, including repair tests.
- Temporary Level-1 public-writer, CLI, leaf-location, restoration, repair, and closure probes: confirmed the instance table and robustness outcomes. Their success means the diagnostic assertions matched the observed behavior, not that comment retention passed.
- `just check-tier-coverage darkmatter` and `just check-tier-coverage claudine`: no stranded tests.

The CLI module is declared in `tests/l1/main.rs` under an explicit Cargo test target. Writer and closure tests are compiled library unit tests. Changed tests have no tier-name marker or feature gate that removes them from L1. Temporary probes were removed after execution, preserving the implementation under review.

The full area suites and lint were not rerun for this review. The implementation log records full-suite failures in two unrelated schema-matching tests; those claims were not independently reverified here and are not used as proof of passing full suites. Claudine emitted a linker unwind-table warning, but the focused tests passed. Cross-OS evidence remains a CI responsibility and does not determine this readiness decision.
