---
$schema: feature-review.yaml
ready: false
findings:
    - title: Flow-collection scanners still treat quotes inside plain scalars as quoted syntax
      priority: medium
human_review: true
human_review_items:
    - |-
        Review the incomplete sweep of the recurring YAML quote-handling defect before another automated implementation cycle. The date editor is corrected, but two helpers also read raw YAML flow collections and still mistake an apostrophe or double quote inside an unquoted value for quoted syntax. This makes Claudine reject otherwise supported quoted values written by an agent. Confirm that the correction covers both raw YAML readers and preserves the helpers' separate use for decoded schema expressions, with public-result regression tests for both uses.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-09-29T13:19:49-07:00
spec: 2026-09-28-hash-writer-byte-fidelity/spec.md
implemented: true
implemented_by: claude/opus
log: darkmatter/fixes/2026-09-28-hash-writer-byte-fidelity/log.md
description: "A **fix** review of `2026-09-28-hash-writer-byte-fidelity/spec.md`"
fix: 2026-09-28-hash-writer-byte-fidelity/review-3.md
previous: 2026-09-28-hash-writer-byte-fidelity/review-2.md
next: 2026-09-28-hash-writer-byte-fidelity/review-4.md
---

# Review of Hash Writer Byte Fidelity

The fix is **not production ready**. The reported date-comment loss is corrected, and the original byte-fidelity requirements pass their focused tests. However, the same quote-handling defect remains in two related YAML flow-collection readers missed by the implementation sweep. It makes Claudine reject valid agent-written content. This is one medium-priority finding, with `recurrence: true` so the review loop pauses for human inspection before another cycle.

The remaining defect predates this iteration and does not reproduce comment deletion in the hash writer. It is reported because this iteration explicitly expanded its correction to the shared leaf-location scanners, and its claimed class sweep incorrectly excluded two readers of raw YAML. No production code changes are proposed as part of this review.

## Previous finding disposition

Review 2's Unblocked Findings contains one finding. Its Blocked Findings says “None”; no blocked finding became unblocked before implementation. Review 1 also declares no blocked findings.

| Earlier finding | Verification | Disposition |
| --- | --- | --- |
| Review 2, medium — Date replacement can delete a trailing comment after a quote inside a plain scalar | The new scalar-start rule retains comments after internal apostrophes and double quotes. Permanent library tests check inline and following-line layouts, LF/CRLF/lone CR, real quoted controls, repeated saves, and no-bump saves. CLI and Claudine tests assert persisted comment bytes. | Reported date-edit instances resolved; related raw-YAML readers remain affected as described below. |
| Review 1, high — Hash replacement can silently change unrelated alias values | Writer and property-restoration tests refuse changes to unmanaged alias values; CLI and Claudine tests verify no write. | Resolved. |
| Review 1, medium — Date writing mistakes indented comments for collection values | Parsed types distinguish collections from scalars; exact-byte tests retain indented comments and support a scalar alone on the following line. Unsupported block and multiline scalar layouts have accurate diagnostics and current documentation. | Resolved. |

The implementation also fixed quote handling in top-level and nested block-mapping keys. Those added tests pass. No prior finding was deferred in the implementation log.

## Unblocked Findings

### Medium — Flow-collection scanners still treat quotes inside plain scalars as quoted syntax

**Defect class:** a scanner opens quote mode at an apostrophe or double quote inside an already plain YAML scalar, hiding collection delimiters or separators and preventing an otherwise supported quoted leaf from being edited.

In the darkmatter package, [matching_delimiter](../../lib/src/markdown/schemas/simplified/source.rs:1404), which finds a collection's closing bracket or brace, and [split_top_level](../../lib/src/markdown/schemas/simplified/source.rs:1431), which separates its entries, still open quote mode at any quote character. These helpers serve both decoded schema expressions and **raw YAML**, through the flow-sequence and flow-mapping branches of `locate_inline`. The implementation log's assertion that these are only schema-expression scanners is incorrect.

The darkmatter public [locate_frontmatter_leaves](../../lib/src/markdown/hash/write.rs:543) API uses this reader to find exact replacement spans. Claudine's [encode_agent_values](../../../claudine/lib/src/composition/closure/persist.rs:174) uses those spans to store newly authored expression-looking text as literal data. A failure prevents [reconcile_inline_artifact](../../../claudine/lib/src/composition/closure.rs:100), Claudine's closure write-back entry point, from accepting the document.

Start with this valid fixture, matching the existing quoted-flow-leaf coverage:

```yaml
---
prompt: test
q: [ordinary, "{{x}}"]
---
Agent body
```

Claudine accepts it, encodes the quoted second item as literal data, and writes the hash and date. Change only `ordinary` to `don't`:

```yaml
q: [don't, "{{x}}"]
```

YAML still parses, and the requested second item is still genuinely quoted. Nevertheless, the leaf locator returns `UnsupportedShape`, and closure write-back returns `InlineAgentFrontmatterRejected` for `q[1]`, leaving the candidate file unchanged. The diagnostic describes unsupported plain flow items even though the target is quoted. Changing a harmless neighboring value should not remove support for that target.

There are two independently failing helpers. With `[don't, "v"]`, the apostrophe hides the closing bracket from `matching_delimiter`. With `[don't, can't, "v"]`, the two apostrophes balance accidentally, so the closing bracket is found; `split_top_level` then combines `don't, can't` into one entry. The public YAML source projection returns a two-item tree for a three-item sequence, and the safe leaf locator refuses the quoted third item. Fixing only bracket matching leaves this second instance behind.

The sweep used the existing writer/date and quoted-flow-leaf fixture shapes, changing the ordinary scalar or key to an internal-apostrophe or internal-double-quote spelling. All collection probes ran under LF and CRLF. The plain target itself was never requested for editing.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| darkmatter raw flow-sequence delimiter reader | `[don't, "v"]`; `[say "hi, 'v']`; the sequence nested in a flow mapping | Cannot find the collection end; public leaf lookup refuses | Locate the genuinely quoted target while retaining its plain sibling |
| darkmatter raw flow separator reader | `[don't, can't, "v"]` | Combines the first two entries; public lookup of the quoted third item refuses | Keep all three entries and locate the third |
| darkmatter raw flow-mapping reader | `{don't: "v"}`; `{say "hi: 'v'}`; `{a: don't, b: "v"}`; `{a: say "hi, b: 'v'}` | Public source projection returns no tree; quoted target lookup refuses | Recognize the plain key or sibling and locate the quoted value |
| darkmatter public YAML source projection, `locate_schema_value` | Every collection shape above | Returns no tree for unmatched internal quotes; paired apostrophes produce a tree with the wrong number of entries | Preserve the YAML collection's actual structure |
| darkmatter public leaf locator | Every collection shape above, requesting only the quoted string | Returns `Unlocated(UnsupportedShape)` | Return the exact quoted scalar span |
| Claudine encoding and closure write-back | Same collection shapes, with quoted target `"{{x}}"` or `'{{x}}'`, newly added to the baseline | Rejects the quoted target; candidate file remains byte-identical | Encode only the target, accept the body change, and stamp hash/date |
| All collection readers and Claudine write-back, controls | Ordinary plain siblings/keys; properly quoted `'don''t'` siblings/keys | Quoted targets locate successfully; closure accepts and encodes them | Clean |
| darkmatter text hash writer and property restoration | Every collection shape above, as an unmanaged property | Hash/date save succeeds with exact neighboring bytes; identical snapshot restoration is byte-stable | Clean |
| darkmatter-cli `md hash --save` / `--diff` | Every collection shape above | Save succeeds, retains collection/comment/body bytes, and immediate diff succeeds | Clean |
| darkmatter date comment scanner and top-level key scanner | Review 2's internal-quote date fixtures and new internal-quote key fixtures | Exact-byte tests pass for LF, CRLF, and lone CR | Clean |
| darkmatter nested block-mapping separator and scalar decoder | Internal-quote keys/values outside flow collections, plus genuine quoted controls | New public leaf-location tests and scanner controls pass | Clean for the corrected block layouts |
| Claudine repair scanner | Every flow fixture above | Returns the candidate byte-identically; YAML remains parseable | Clean: these valid collections need no repair |
| Map-based frontmatter serializers | Ownership and call-site inspection | Serialize already parsed values; do not discover editable raw collection spans | Outside this defect class |
| Decoded schema-expression scanners | Call-site inspection and existing source-projection tests | Quote syntax belongs to the decoded expression grammar | Separate grammar; retain its behavior when correcting raw YAML readers |

**Required correction:** give raw flow-collection scanning YAML's scalar-start quote rules in both helpers and their recursive collection paths. Preserve the separate expression grammar; changing shared helpers indiscriminately could regress schema suggestions. This does not require adding support for editing plain flow items, multiline flow collections, anchors, aliases, or tags.

Add a permanent table-driven test through the public leaf API with ordinary and genuinely quoted positive controls, internal apostrophe/double-quote keys and siblings, balanced internal quotes, and nested collections. Assert exact target spans and a reparsed splice that changes only the target. Add persistent Claudine tests through closure write-back for those same shapes, checking encoded target values, preserved sibling values/bytes, and successful persistence. Include expression-projection controls for any changed shared helper. Correct the implementation log's raw-YAML call-site classification.

## Blocked Findings

None. The technical correction is actionable without a design decision. Human inspection is requested because the user's recurrence rule stops another automated cycle, not because the finding itself requires unavailable evidence or approval to implement.

## Recurrence

This is the same defect class as review 2's **“Date replacement can delete a trailing comment after a quote inside a plain scalar.”** The earlier date edit is fixed; the scanner class remains in sibling readers.

That correction's sweep should have followed `locate_inline` into both `matching_delimiter` and `split_top_level`, covering raw flow sequences, raw flow mappings, nested collections, the public YAML source projection, public leaf lookup, and Claudine encoding/write-back. It checked the nested block-key separator but classified the two flow helpers as expression-only, leaving their raw-YAML callers out. Both helpers and every affected collection branch are included in this review's instance table. Review 1's alias-value and line-count defect classes do not recur.

## Input robustness matrix

YAML frontmatter is the affected format; no manifest or lockfile format is added. The hash writer's load-bearing fields are its configurable managed hash property and `last_updated`. The permanent 58-row writer matrix starts with a positive control, edits one field at a time, and asserts exact public output or refusal, reparsed unmanaged values, and body preservation. The additional flow-reader issue concerns locating a caller-selected string leaf inside an arbitrary property; its lexical cases and controls are enumerated above.

| Shape | Managed hash property | `last_updated`, date bump requested |
| --- | --- | --- |
| Positive control | Requested hash written | Requested date written |
| Absent | Inserted | Inserted |
| Explicit null | Whole managed node replaced | Null scalar spelling replaced |
| Empty YAML value | Whole managed node replaced | Date inserted with valid spacing |
| Wrong scalar type: number, float, boolean | Whole managed node replaced | Ordinary scalar replaced |
| Wrong type, one sequence element: string/number mix | Whole managed node replaced | Refused as collection |
| Wrong type, every sequence element: all numbers | Whole managed node replaced | Refused as collection |
| Empty sequence or mapping | Whole managed node replaced | Refused as collection |
| Duplicate semantic key, including quoted spelling | Refused | Refused |
| Invalid YAML or trailing garbage within frontmatter | Refused | Refused |
| Anchor, alias, tag | Allowed only when output parses and unmanaged values remain unchanged | Refused |
| Inline or indented comments | Complete managed node is owned by replacement | Comments retained |
| Internal quote in a plain date, followed by comment | Complete managed node replacement | Scalar replaced; comment retained |
| Single scalar on following line | Complete managed node replacement | Value-only replacement |
| Block scalar or scalar continued across lines | Complete managed node replacement | Documented layout refusal |

These APIs intentionally accept arbitrary managed hash values for replacement; the CLI's stored-hash validation is separate. Numbers and booleans are intentionally accepted date scalars. Absence, null spelling, and an empty value have different source edits even where the resulting date agrees. Collection elements are not filtered. Body content after the closing delimiter is Markdown, not trailing YAML. A decision requiring no write still returns `None` without parsing.

## Requirement verification and execution evidence

Every requirement concerns source bytes, parsed values, or file persistence. Level 1 is appropriate throughout; no requirement depends on terminal rendering, a terminal input encoder, or OS keyboard events.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Six original examples, including CRLF empty date | Level-1 exact-byte writer tests and typed refusal | Pass |
| Mixed terminators, hash-only replacement, inserted dates, lone CR, BOM, unterminated body | Level-1 exact-byte writer matrices | Pass |
| Anchored/aliased/tagged dates, no-bump and no-write behavior | Level-1 writer and persistent CLI/closure tests | Pass |
| Unmanaged values and body bytes; reused-anchor safety | Level-1 public writer/restoration tests and CLI/closure no-write tests | Pass |
| Date comments, spacing, quote style, following-line values | Level-1 exact-byte library tests and filesystem assertions through both callers | Pass |
| Repeated save stability | Level-1 writer and CLI save/diff checks | Pass |
| Expanded quote-class correction in leaf-location readers | Level-1 tests for block layouts and ordinary quoted flow targets | Gap: internal quotes in plain flow siblings/keys and balanced internal quotes are missing from permanent coverage |
| Current documentation and behavior comments | Inspection of CLI guide, Claudine composition topic, darkmatter skill, and changed symbol docs | Date-write contract matches implementation; implementation log misclassifies raw flow readers |

Executed on this host:

- `cd darkmatter && just test 'hash::write::'`: 63 permanent tests passed.
- `cd darkmatter && just test hash_kind_save_diff::`: 27 permanent tests passed.
- `cd darkmatter && just test schemas::simplified::source::`: four permanent tests and one temporary scanner probe passed.
- `cd claudine && just test-library 'composition::closure::'`: 46 permanent tests and one temporary public closure probe passed. An expanded closure probe also checked balanced internal quotes and nested flow collections.
- Temporary public writer/leaf/source-projection, CLI save/diff, closure, and scanner probes confirmed the instance table. Passing probes asserted the observed behavior, including the erroneous refusals; they are not evidence that the finding is resolved.
- `just check-tier-coverage darkmatter` and `just check-tier-coverage claudine`: zero stranded tests.

CLI tests are declared by `tests/l1/main.rs` under an explicit Cargo test target. Writer, scanner, and closure tests compile as library unit tests. The changed tests have no tier marker or feature gate excluding them from Level 1. Temporary probes were removed after execution; production code and permanent tests remain unchanged by this review.

Full area suites and lint were not rerun for this document-only review. The implementation log records two unrelated full-suite failures; those were not independently reverified here and are not counted as passing evidence. Claudine emitted its existing large unwind-table linker warning, but the focused tests passed. Cross-OS evidence remains a CI responsibility and does not determine readiness.
