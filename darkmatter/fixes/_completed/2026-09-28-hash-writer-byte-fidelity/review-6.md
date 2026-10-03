---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
    - |-
        Retain review 5's request to inspect why the schema readers repeatedly applied the wrong rules to description prose and imported filenames. The correction now shares those boundaries with the semantic parser and tests complete source maps, cursor roles, and completion replacement ranges. Inspect that correction and the clean cases listed below; no new design decision or implementation approval is requested. This review does not establish that the earlier human inspection occurred.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-09-29T19:16:10-07:00
spec: 2026-09-28-hash-writer-byte-fidelity/spec.md
implemented: false
description: "A **fix** review of `2026-09-28-hash-writer-byte-fidelity/spec.md`"
fix: 2026-09-28-hash-writer-byte-fidelity/review-6.md
previous: 2026-09-28-hash-writer-byte-fidelity/review-5.md
---

# Review of Hash Writer Byte Fidelity

The fix is **production ready**. Review 5's actionable finding is implemented, the original byte-preservation requirements pass, and the sweep found no remaining instance of the earlier defect classes. The earlier request for human inspection remains an external review step; it does not block the technical readiness decision.

This review examined the specification, plan, implementation logs, all five earlier reviews, the current changes, the public writer and its callers, related YAML and type-expression readers, current documentation, and test placement. It leaves production code and permanent tests unchanged.

## Previous finding disposition

Review 5 has one Unblocked Finding and no Blocked Findings. No blocked technical finding became unblocked. Its implementation log reports the finding corrected without deferral.

| Earlier finding | Current implementation and verification | Disposition |
| --- | --- | --- |
| Review 5 — Type-expression readers mistake description and file-reference text for argument syntax | The darkmatter parser, structural source readers, and scalar cursor share description, reference, and pattern-key boundaries. Permanent tests assert the complete source-map entries, exact authored ranges, cursor role/path/token, and DMLS completion edits. | Resolved |
| Review 4 — Type-expression cursors lost quote handling inside flow alternatives | Expression flow alternatives use expression-aware argument quoting, while genuinely YAML-quoted alternatives are decoded separately. Tests include open/closed punctuation, escaped quotes, and incomplete enum arguments. | Resolved |
| Review 4 — Raw YAML flow readers still treat parentheses as collection structure | Raw YAML entry splitting and declaration cursors use YAML collection rules. Public leaf and persisted closure tests cover unbalanced parentheses and parentheses balanced across distinct entries. | Resolved |
| Reviews 2–4 — Quotes inside plain YAML scalars incorrectly start quoted syntax, including after content colons | Raw YAML readers distinguish content colons from mapping separators. Writer, leaf, source, cursor, diagnostic, and persisted caller tests cover both quote styles, keys, values, nested collections, and quoted controls. | Resolved |
| Review 1 — Hash replacement can silently change unrelated alias values | Hash saving and property restoration compare unmanaged parsed values before returning text. Scalar and nested reused-anchor tests cover bump/no-bump, unresolved aliases, and a surviving later declaration. Both persistence callers verify refusal without writing. | Resolved |
| Review 1 — Date writing mistakes indented comments for collection values | Parsed value types distinguish collections from scalars. Exact-byte writer and persistence tests retain comments and support a scalar alone on a following line. Unsupported block and continued scalar layouts have accurate documented refusals. | Resolved |

One correction to review 5's examples is justified: unquoted `[` and `{` inside a YAML flow scalar make the document invalid. DMLS tests explicitly assert those YAML errors and exercise the corresponding quoted filenames instead. The tolerant cursor separately covers the unfinished, unquoted spellings. This preserves the distinction between incomplete authoring input and a completed YAML document.

## Unblocked Findings

None. No additional performance or ergonomic change is necessary for readiness.

## Blocked Findings

None. The retained human inspection request concerns the history of the correction, not an unresolved technical finding.

## Class sweep

The recurring schema-reader class was **using punctuation to infer argument syntax while the parser was reading another kind of text**. The following sibling sites now agree with the relevant parser rules. The permanent fixture matrices use ordinary positive controls and change description, argument, key, or filename spelling; they assert public results rather than scanner success alone.

| Site | Shapes tested or inspected | Observed result | Expected result |
| --- | --- | --- | --- |
| darkmatter semantic description/reference/pattern-key readers | Parenthesized prose quotes, bracket prose, filenames with opening punctuation, opaque pattern keys | Intended properties and references parse | Clean: retain the grammar's existing boundaries |
| darkmatter expression delimiter, property splitter, arrow and separator readers | Quotes balanced across different descriptions, nested objects, bracket prose, opening punctuation in filenames | Complete source maps; no missing property; descriptions excluded from reference spans | Clean: exact spans for every parsed property |
| darkmatter constraint-group and call-argument readers | Real quoted arguments containing delimiters; `enum('a(b', c)`; imported filenames containing parentheses and quotes | Real groups project; filename/quoted-member punctuation creates no false call | Clean: project only actual constraints and arguments |
| darkmatter suggestion-only projection | The description/import controls and argument punctuation | Existing suggestion/source tests pass | Clean: preserve suggestion ranges independently of the structural source map |
| darkmatter scalar type cursor | Description ending before the next property, nested objects, punctuated filenames, pattern keys | Correct property path, type/import/constraint role, token, and replacement range | Clean: resume after prose; retain reference context inside filenames |
| darkmatter flow type cursor | Parenthesized prose quotes, imported filenames, genuine quoted argument punctuation, unfinished enum arguments | Correct alternative index and role; following type token is not swallowed | Clean: expression argument rules within each alternative |
| DMLS completion provider | Prose/import controls and edited fixtures; quoted YAML filenames with brackets/braces | Offers `string` with the exact one-character replacement range | Clean: same completion as positive controls |
| DMLS file-type detector | `file`, constrained/array file types, `file->description`; `file(` or `|` appearing in regex, prose, or imports | Only the leading file-type keyword selects file handling | Clean: content punctuation is not a union/type declaration |
| darkmatter expression serializer | Enum/literal arguments containing `->`, alongside ordinary quoted arguments | Serialized values reparse to the same semantic atoms | Clean: quote text that the argument lexer cannot read as one bare word |
| Raw YAML collection/separator readers and declaration cursor | Internal quotes, content colons, unbalanced/cross-entry parentheses, recursive collections, genuinely quoted controls | Correct collection structure and leaf/declaration ranges | Clean: retain YAML rules separately from expression rules |
| darkmatter hash writer/restoration and CLI | Same raw data shapes, reused anchors, comments, mixed line endings, repeated saves | Exact unmanaged bytes/values retained, or typed refusal without persistence | Clean: semantic and byte fidelity |
| Claudine repair, leaf encoding, and closure persistence | Quoted expression-looking targets beside plain siblings containing quotes, content colons, or parentheses | Encodes the intended target, preserves siblings, and persists a coherent hash/date | Clean: supported leaf editing stays supported |
| Style and Claudine frontmatter diagnostic key readers | Quoted keys and plain keys containing content colons/quotes | Focused location/diagnostic tests pass | Clean: shared mapping-separator rules |

Static constraint-catalog readers consume repository-defined descriptor text rather than authored expressions; they are outside this lexical-input class. Map-based serializers operate on parsed values and do not locate raw source spans.

An additional temporary public-API matrix copied the permanent inline-object fixture and changed one description or filename at a time. It covered ten description spellings and nine filename spellings, both YAML quote styles, nonzero offsets, Unicode, and LF/CRLF/lone-CR description fixtures. It checked semantic equality, the following property's exact definition span, import-reference boundaries, and the following cursor's path and role. All cases passed. The temporary test was removed after execution.

## Recurrence

No finding recurs in this review, so `recurrence` is `false`. Reviews 2–4's raw-YAML quote/boundary class and reviews 4–5's expression-context class now pass their sibling matrices. Review 1's alias-preservation and scalar-layout classes also remain resolved. The prior human inspection request is retained without introducing a new decision or requiring its outcome to establish production readiness.

## Input robustness matrix

YAML frontmatter is the affected format. The writer's load-bearing fields are the configurable managed hash property and `last_updated`. The permanent 58-row matrix starts from a positive fixture, changes one field per row, and checks exact public output/refusal, unmanaged parsed values, and body bytes. It passed in this review.

| Input shape | Managed hash property | `last_updated`, bump requested |
| --- | --- | --- |
| Positive control | Requested hash written | Requested date written |
| Absent | Inserted | Inserted |
| Explicit null | Complete managed node replaced | Null spelling replaced |
| Empty YAML value | Complete managed node replaced | Date inserted with valid spacing |
| Wrong scalar type: number, float, boolean | Complete managed node replaced | Ordinary scalar replaced |
| Wrong type, one sequence element | Complete managed node replaced | Collection refused |
| Wrong type, every sequence element | Complete managed node replaced | Collection refused |
| Empty sequence/mapping | Complete managed node replaced | Collection refused |
| Duplicate semantic key, including quoted spelling | Refused | Refused |
| Invalid/trailing YAML inside frontmatter | Refused | Refused |
| Anchor, alias, tag | Replaced only if output parses and unmanaged values remain equal | Refused |
| Comments and a supported following-line scalar | Complete managed node replaced | Value-only edit; surrounding bytes retained |
| Block or continued multiline scalar | Complete managed node replaced | Documented layout refusal |

These are explicit replacement/refusal rules. Missing, null, and empty source values follow different editing paths even when the resulting date agrees. Collection elements are not silently filtered. A no-write decision returns `None` before parsing; content after the closing delimiter is Markdown body text.

The schema changes correct lexical interpretation of already accepted string content; they add no configuration fields or formats. Their complete public-result matrices cover descriptions, imports, genuine arguments, quoted YAML, pattern keys, recursive objects, invalid completed YAML controls, and incomplete cursor input. Inspection of permissive parse patterns found no new conversion of malformed load-bearing input into a successful default: the date span's `.ok()` is followed by an equality check and refusal, and source projection failures remain errors.

## Requirement verification and execution evidence

Level 1 is appropriate for every requirement here: text transformation, parsing, filesystem persistence, source ranges, and completion results. None depends on terminal rendering, terminal input encoding, or OS keyboard events. Level 2 or 3 is not needed for these contracts.

| User-facing requirement | Verification present and executed | Assessment |
| --- | --- | --- |
| Six original cases, including CRLF empty dates and unsafe-date refusal | Level-1 exact-byte writer and typed-error tests | Pass |
| Per-line terminators for mixed files, insertion, hash-only replacement, and lone CR | Level-1 writer matrix and persisted CLI assertions | Pass |
| Leading BOM, existing frontmatter, and body without a final newline | Level-1 exact-byte writer and CLI fixtures | Pass |
| Comments, authored keys/spacing/quote style, following-line scalar | Level-1 writer, CLI, and Claudine persisted-byte assertions | Pass |
| Output validation, unchanged unmanaged values/body, anchor safety, no-write/no-bump | Level-1 writer/restoration and persistence refusal tests | Pass |
| Read/write/read stability and immediate hash agreement | Level-1 repeated writer saves, CLI save/diff, and closure tests | Pass |
| Source maps, cursor context, completion ranges, serializer round trips introduced during correction | Level-1 public projection, cursor, DMLS provider, and serializer tests | Pass |
| Current documentation and relevant comments | CLI hash guide, schema topics, Claudine composition topic, darkmatter skill, and changed-symbol inspection | Matches implemented behavior |

Executed successfully through nextest-backed recipes:

- Darkmatter `just test 'hash::write::'`: 67 tests.
- Darkmatter `just test schemas_source_projection::`: 7 tests.
- Darkmatter `just test 'simplified::'`: 330 tests.
- Darkmatter `just test 'frontmatter::'`: 289 tests, including DMLS completion and diagnostic readers.
- Darkmatter `just test hash_kind_save_diff::`: 28 tests.
- Darkmatter `just test substrate`: 19 tests.
- Darkmatter `just test 'style::parse::'`: 40 tests.
- Claudine `just test-library 'composition::closure::'`: 50 tests.
- Claudine `just test-library 'frontmatter_excerpt::'`: 36 tests.
- Temporary public boundary-sweep test: passed and removed.
- `just check-tier-coverage darkmatter`: no stranded tests.

The library/CLI integration modules are declared in their explicit `l1` Cargo targets. Writer, cursor, serializer, DMLS, and Claudine tests compile as library unit tests. The added tests have no resource-tier marker, ignored attribute, or feature gate excluding them from Level 1.

Full area suites and lint were not rerun for this document-only review. The implementation log's pre-existing root-union file-matching failure is outside the writer and lexical corrections; this review does not claim a clean full-suite run. Claudine emitted a large unwind-table linker warning, but its focused tests passed. Cross-OS evidence remains a CI responsibility and does not affect this readiness decision. No commit was created or lifecycle directory moved.
