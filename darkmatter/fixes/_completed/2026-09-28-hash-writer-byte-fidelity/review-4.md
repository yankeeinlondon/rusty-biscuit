---
$schema: feature-review.yaml
ready: false
findings:
    - title: Content colons still reopen quote mode inside plain YAML scalars
      priority: medium
    - title: Raw YAML flow readers still treat parentheses as collection structure
      priority: medium
    - title: Type-expression cursors lost quote handling inside flow alternatives
      priority: medium
human_review: true
human_review_items:
    - |-
        Before another automated implementation cycle, inspect why the plain-scalar quote defect escaped the previous sweeps. Check the correction against every reader and caller in this review's instance tables, including colons inside plain values, and verify that YAML readers and type-expression cursors each retain their own punctuation rules. The expected outcome is a complete regression suite for the affected readers, rather than another correction limited to the first failing example.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-09-29T13:54:20-07:00
spec: 2026-09-28-hash-writer-byte-fidelity/spec.md
implemented: true
implemented_by: claude/opus
log: darkmatter/fixes/2026-09-28-hash-writer-byte-fidelity/log.md
description: "A **fix** review of `2026-09-28-hash-writer-byte-fidelity/spec.md`"
fix: 2026-09-28-hash-writer-byte-fidelity/review-4.md
previous: 2026-09-28-hash-writer-byte-fidelity/review-3.md
next: 2026-09-28-hash-writer-byte-fidelity/review-5.md
---

# Review of Hash Writer Byte Fidelity

The fix is **not production ready**. The original writer requirements and the concrete examples reported in earlier reviews pass their focused tests. The expanded reader changes still reject supported quoted targets beside valid plain YAML values, and introduce a regression in the public type-expression cursor API. There are three medium-priority findings. The first repeats the plain-scalar quote defect from reviews 2 and 3, so `recurrence` is `true` and human inspection is requested before another automated implementation cycle.

The remaining failures concern the leaf-location and schema-reader work added during the review cycle. They do not reproduce date-comment deletion or unmanaged-value changes in the hash writer. This review changes only review documents and specification metadata.

## Previous finding disposition

Review 3 has one Unblocked Finding and no Blocked Findings. Its requested human inspection is separate from technical blockage: the finding was explicitly actionable. No blocked finding became unblocked before the last implementation, and the implementation log records no deferred finding. Review 1's metadata also declares no blocked findings; review 2's Blocked Findings section says “None.”

| Earlier finding | Verification | Disposition |
| --- | --- | --- |
| Review 3, medium — Flow-collection scanners still treat quotes inside plain scalars as quoted syntax | Permanent public leaf tests cover 15 shapes under LF/CRLF; public source-projection and persistent Claudine tests cover ordinary, quoted, internal-quote, balanced-quote, and nested controls. The raw readers now use separate helpers; decoded expression helpers retain their previous grammar. | Reported examples resolved. The scalar-start rule is incomplete for content colons; see the first finding. |
| Review 2, medium — Date replacement can delete a trailing comment after a quote inside a plain scalar | Permanent exact-byte writer and CLI tests pass. Additional public-writer probes put `a:'b` and `a:"b` before comments, inline and on the following line, under LF/CRLF/lone CR. | Resolved for the date editor. |
| Review 1, high — Hash replacement can silently change unrelated alias values | Writer and restoration tests compare unmanaged parsed values; permanent CLI and Claudine tests verify refusal without persistence. | Resolved. |
| Review 1, medium — Date writing mistakes indented comments for collection values | Writer, CLI, and closure tests retain indented comments and accept a scalar alone on a following line. Collections and unsupported multiline scalar layouts have distinct refusals. | Resolved. |

The implementation corrected review 3's call-site classification: the old delimiter helpers had raw-YAML callers as well as decoded-expression callers. It also updated schema-key projection, schema cursors, and DMLS diagnostic-span completion. Those additional changed readers are included below.

## Unblocked Findings

### Medium — Content colons still reopen quote mode inside plain YAML scalars

**Defect class:** a raw YAML scanner mistakes a colon inside an already plain scalar for a scalar boundary, then interprets the following apostrophe or double quote as opening quoted syntax.

In the darkmatter package, [scan_flow_yaml](../../lib/src/markdown/schemas/simplified/source.rs:861), the shared raw-flow scanner, sets `scalar_start` after every `:`. YAML permits a content colon immediately followed by a quote inside a plain scalar. Consequently, `a:'b` and `a:"b` are ordinary string values, but the scanner enters quoted mode midway through them. It hides a collection's closing delimiter or its commas.

Copy the existing quoted-flow-target fixture and change only its ordinary sibling:

```yaml
---
prompt: test
q: [a:'b, "{{x}}"]
after: z
---
Agent body
```

Darkmatter's YAML parser reads `q` as two strings: `a:'b` and `{{x}}`. Its public [locate_frontmatter_leaves](../../lib/src/markdown/hash/write.rs:543), which finds exact source spans for string replacement, returns `Unlocated(UnsupportedShape)` for `q[1]`. Claudine's public [reconcile_inline_artifact](../../../claudine/lib/src/composition/closure.rs:100), which accepts and persists an agent's edited document, returns `InlineAgentFrontmatterRejected` for that genuinely quoted target and leaves the candidate file unchanged. The ordinary-sibling and genuinely quoted-sibling controls both succeed.

The balanced spelling `[a:'b, c:'d, "{{x}}"]` independently proves that correcting delimiter matching alone is insufficient: the closing bracket is found, but two real entries are merged, and the quoted third target cannot be located.

All complete collection fixtures below were parsed through the production YAML parser first. Leaf lookup, writer/restoration, CLI save/diff, and persistent closure probes used LF and CRLF versions of the same fixtures. Cursor probes use the corresponding incomplete value before the cursor.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| darkmatter raw flow delimiter reader, `flow_collection_end` | `[a:'b, "v"]`, `[a:"b, 'v']`; mapping and nested variants | Returns `None` despite a closing delimiter | Find the actual closing bracket or brace |
| darkmatter raw flow entry reader, `split_flow_entries` | `[a:'b, c:'d, "v"]`; single internal-quote variants | Merges the first two entries, or the complete collection interior | Retain every YAML entry |
| darkmatter flow-sequence projection | Both quote spellings, balanced quotes, and `{list: [a:'b, "v"]}` | No tree, or a three-item sequence projected as two items | Preserve the parsed collection structure |
| darkmatter flow-mapping projection | `{a: a:'b, b: "v"}`, `{a: a:"b, b: 'v'}`, `{a:'b: "v"}`, `{a:"b: 'v'}` | No tree; quoted target lookup refuses | Locate the quoted value while preserving its plain sibling or key |
| darkmatter public leaf locator | All preceding shapes, requesting only the quoted target | `UnsupportedShape` | Return its exact quoted span; a splice changes only that leaf |
| Claudine encoding and closure persistence | The same shapes with newly added quoted `{{x}}` targets | `InlineAgentFrontmatterRejected`; file stays byte-identical | Encode the target, preserve siblings, and persist hash/date |
| darkmatter schema-declaration and type-definition cursors | `[a:'b, ./b`, `[a:"b, ./b`, `[a:'b, c:'d, ./b`, and a nested mapping variant | Reports the wrong alternative index; unbalanced variants also include the preceding alternative in the token | Identify the final alternative and its token |
| DMLS diagnostic-span completion | Sequence, mapping, and key variants with one unmatched internal quote | Leaves the range at the opening bracket/brace, rather than covering the collection | Complete the value range; balanced-quote control completes correctly |
| darkmatter raw mapping separators and schema-key projection | `a:'b: string(suggest(alpha, beta))` and the double-quote counterpart | `mapping_separator` and `mapping_value_offset` select the content colon; public `parse_yaml_schema_with_source` refuses projection | Find the actual key/value separator and project the suggestions |
| Every flow reader and closure caller, positive controls | Ordinary siblings, `don't` without a preceding colon, and genuinely quoted `'a:''b'` | Correct structure, spans, and successful persistence | Clean |
| darkmatter text hash writer and property restoration | All collection fixtures as an unmanaged property | Preserve the collection bytes; identical-snapshot restoration is byte-stable | Clean |
| darkmatter-cli `md hash --save`, `--diff`, repeated `--save` | All collection fixtures | Successful saves, retained collection/body bytes, clean diff, stable second save | Clean |
| darkmatter date editor and its scalar-parse guard | `a:'b`, `a:"b`, ordinary and genuinely quoted controls before a comment; inline/following-line layouts | Date writer preserves comments under LF/CRLF/CR | Clean: quotes inside these plain values remain content |
| Claudine repair reader | All complete collection fixtures | Returns the candidate byte-identically | Clean: valid collections require no repair |
| Decoded expression delimiter/projection readers | Quoted punctuation in schema-expression arguments | Public property projection succeeds | Separate grammar; retain this behavior |

The mapping-separator cases expose an additional consequence of the same incorrect content-colon boundary. Selecting the first colon in a plain key predates this iteration; the implementation log already notes that limitation. Its top-level sibling, darkmatter's [mapping_colon](../../lib/src/markdown/hash/write.rs:877), also selects the first content colon. That is not a newly introduced quote regression, but must not be mistaken for a correct separator rule when fixing nested keys. Claudine's repair separator checks the character following `:` and does not have that first-colon rule.

**Required correction:** recognize actual YAML scalar boundaries in the raw flow scanner, including content colons, and use the correct key/value separator when locating the affected mapping keys. Cover delimiter matching, entry splitting, recursive sequence/mapping projection, both cursor entry points, diagnostic ranges, public leaf spans, and persistent Claudine write-back. Preserve the decoded expression grammar and the existing refusal of edits to plain flow targets or unsafe node properties. No new editable target shape is required: the failing target is already quoted and supported.

Extend the permanent fixture tables with both internal quote spellings after a content colon, balanced spellings, plain keys, nested collections, and genuinely quoted controls. Assert the public result and exact persisted bytes, rather than only private scanner offsets.

### Medium — Raw YAML flow readers still treat parentheses as collection structure

**Defect class:** a raw YAML reader applies type-expression parenthesis nesting to plain YAML text, hiding real entry separators.

In the darkmatter package, [split_flow_entries](../../lib/src/markdown/schemas/simplified/source.rs:833), which separates raw flow collection entries, increments and decrements nesting at `(`/`)`. Parentheses are ordinary content in a plain YAML scalar. The same nesting rule remains in [flow_arm_starts](../../lib/src/markdown/schemas/simplified/cursor.rs:195) even when the caller is locating opaque schema file-reference alternatives rather than type expressions.

For the same quoted-target fixture, change `ordinary` to `a(b`:

```yaml
q: [a(b, "{{x}}"]
```

The production parser reads two strings. The source projection returns one scalar spanning both entries, and Claudine refuses `q[1]`. With `[a(b, c)d, "{{x}}"]`, the parentheses balance across two independent YAML entries; the source projection returns two items instead of three. This proves that balanced punctuation does not make the rule safe.

This defect predates the iteration: the new raw splitter deliberately inherited the expression splitter's parenthesis rule, and the implementation log records a related failure as outside the quote class. It is reported here because the newly introduced raw-YAML helper still breaks an existing supported quoted-target operation. The plain sibling is retained, never requested for editing.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| darkmatter raw sequence splitter and public source projection | `[a(b, "v"]`; `[a(b, c)d, "v"]` | Two entries become one; three become two | Parentheses stay content; retain every entry |
| darkmatter raw mapping projection | `{a: a(b, b: "v"}` | The second key/value pair is swallowed into the first value | Keep both entries |
| darkmatter recursive collection projection | `{list: [a(b, "v"]}` | Nested quoted target cannot be located | Preserve nested structure and locate the target |
| darkmatter public leaf lookup and Claudine closure write-back | All preceding shapes with quoted `{{x}}` target, LF/CRLF | Lookup refuses; closure rejects without writing | Locate/encode the supported quoted target and persist successfully |
| darkmatter schema-declaration cursor | `[./a(b.yaml, ./b`; `[./a(b.yaml, ./c)d.yaml, ./b` | Reports alternative 0 instead of 1, or 1 instead of 2; the first also has an oversized token | Parentheses in opaque references do not nest; identify the final alternative |
| darkmatter raw collection end and DMLS diagnostic-range reader | Sequence and mapping variants, LF/CRLF | Find the complete collection | Clean: these readers nest brackets/braces, not parentheses |
| darkmatter text writer, restoration, and CLI save/diff/repeated save | All quoted-target fixtures | Preserve bytes and parsed values; saves are stable | Clean |
| Claudine repair reader | All complete fixtures | Returns the candidate unchanged | Clean |
| Raw readers and persistent closure, quoted-sibling controls | `['a(b', "{{x}}"]`; cursor `['./a(b.yaml', ./b` | Correct span, structure, cursor alternative, and successful closure | Clean |
| Decoded type-expression reader | `enum(a, b)` and quoted punctuation inside expression arguments | Parenthesis nesting is required; public expression projection and scalar cursor controls succeed | Retain expression nesting in expression context |

**Required correction:** raw YAML entry splitting must nest only YAML collection delimiters. Opaque declaration references need the same rule. Preserve parenthesis nesting for the tolerant type-expression cursor, where constraints really do contain parentheses and commas. Add table-driven public leaf and closure tests for unbalanced and cross-entry balanced parentheses, mappings, nested sequences, and quoted controls; add declaration-cursor tests that assert both the alternative index and replacement token range.

### Medium — Type-expression cursors lost quote handling inside flow alternatives

**Defect class:** a tolerant type-expression reader uses raw YAML quote-start rules while tracking expression delimiters, so punctuation inside a quoted expression argument changes the cursor's structural context.

In the darkmatter package, [locate_type_definition_cursor](../../lib/src/markdown/schemas/simplified/cursor.rs:105) reports what a developer is typing, including incomplete expressions. Its flow-alternative splitter now delegates to `scan_flow_yaml`. That scanner ignores a quote after `(` because it is not the beginning of a YAML scalar. The splitter nevertheless counts expression parentheses, so it combines two incompatible grammar rules.

Starting from the permanent cursor control `[enum(a, b), str`, quote an argument containing a closing parenthesis:

```text
[enum('a)b', c), str
```

At the end, the public cursor reports alternative index 2, although `str` is the second alternative, index 1. With `'a(b'`, it reports index 0. More directly, while typing `[enum('a)b', c`, it treats the comma between enum arguments as an alternative separator and reports a type position instead of an enum-argument position. DMLS's [type-definition completion reader](../../dmls/src/providers/frontmatter.rs:324), which chooses suggestions from the cursor's role, therefore receives the wrong kind of context.

These are intentionally incomplete authoring inputs. The cursor module's contract and its permanent `a_comma_inside_a_constraint_does_not_open_a_union_arm` test explicitly support such inputs; requiring a finished YAML document or outer YAML quotes would narrow existing behavior. A temporary copy of the pre-change splitter from `HEAD` returned exactly one alternative separator for every completed-first-alternative reproduction below. The new splitter returns zero or two.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| darkmatter flow type-expression cursor, closed punctuation inside argument | `[enum('a)b', c), str`, `[enum('a]b', c), str`, double-quoted argument counterpart | Final token is associated with alternative index 2 | Associate it with index 1 |
| darkmatter flow type-expression cursor, open punctuation inside argument | `[enum('a(b', c), str`, `[enum('a[b', c), str` | Final token is associated with index 0 | Associate it with index 1 |
| darkmatter incomplete first-alternative cursor | `[enum('a)b', c`, closing-bracket and double-quote counterparts | Type context in a new alternative | Enum-argument context in the first alternative |
| darkmatter scalar expression cursor | Corresponding `enum('a)b', c`, `enum('a(b', c`, and bracket spellings without outer flow sequence | Correct enum-argument role and token | Clean: the expression lexer still handles quotes |
| darkmatter public property/source-map projection and decoded expression helpers | YAML-quoted expressions with suggested strings containing `(`, `)`, `[`, `]`, `{`, `}`, and `,` | Successful public projection | Clean: retain expression quote rules in `matching_delimiter`, `split_top_level`, constraint and inline-object projection |
| darkmatter cursor controls | `[enum(a, b), str`; `['enum("a)b", c)', str]` before the cursor | Correct second-alternative context | Clean |
| Raw YAML collection reader and decoded source projection | A genuinely YAML-quoted type expression as the first item, with each of the seven punctuation spellings above | Public property projection succeeds | Clean separation of YAML quoting from expression punctuation |

**Required correction:** keep expression-aware quote and delimiter handling for partially authored type alternatives, while keeping YAML's rules for opaque declaration references and raw collection readers. Extend the public cursor tests to assert alternative path, role, token, and authored replacement range for both quote styles and punctuation that opens/closes nesting. Include the incomplete enum-argument case, which detects the user-facing completion-context error; the existing projection control alone does not exercise this changed cursor path.

## Blocked Findings

None. All three technical corrections are actionable. Human inspection is required by the recurrence rule, not by an unavailable test environment or an unresolved design choice.

## Recurrence

The first finding repeats review 2's **“Date replacement can delete a trailing comment after a quote inside a plain scalar”** and review 3's **“Flow-collection scanners still treat quotes inside plain scalars as quoted syntax.”** The date editor is repaired, but the raw readers still open quote mode midway through a plain scalar when a content colon precedes the quote.

Review 3's correction should have swept scalar-boundary transitions in both flow collection branches, their recursive projections, public leaf lookup, Claudine encoding/persistence, the schema-key projection helpers, both cursor entry points, and DMLS diagnostic-range completion. Its fixtures covered ordinary internal quotes but omitted quotes immediately after content colons. Every affected reader and caller is enumerated in the first table; balanced quotes are included so delimiter matching cannot mask an incorrect entry split.

The second finding is a different punctuation class: parentheses are wrongly given expression meaning in raw YAML. The third is a newly introduced inverse grammar error: expression quotes are wrongly subjected to YAML scalar-start rules. Neither was a finding in reviews 1–3. Review 1's alias-value and physical-line-count classes do not recur.

## Input robustness matrix

The affected document format is YAML frontmatter; no new configuration, manifest, or lockfile format is introduced. The writer's load-bearing fields remain the configurable managed hash property and `last_updated`. Its permanent 58-row matrix starts with a positive fixture and changes one field per row, checking the public output or refusal, unmanaged parsed values, and retained body bytes.

| Shape | Managed hash property | `last_updated`, bump requested |
| --- | --- | --- |
| Positive control | Requested hash written | Requested date written |
| Absent | Inserted | Inserted |
| Explicit null | Complete managed node replaced | Null scalar spelling replaced |
| Empty YAML value | Complete managed node replaced | Date inserted with valid spacing |
| Wrong scalar type: number, float, boolean | Complete managed node replaced | Ordinary scalar replaced |
| Wrong type, one sequence element | Complete managed node replaced | Collection refused |
| Wrong type, every sequence element | Complete managed node replaced | Collection refused |
| Empty sequence or mapping | Complete managed node replaced | Collection refused |
| Duplicate semantic key, including quoted spelling | Refused | Refused |
| Invalid or trailing YAML content inside the frontmatter | Refused | Refused |
| Anchor, alias, tag | Allowed only if output parses and unmanaged values stay equal | Refused |
| Inline or indented comments | Complete managed node owns its comments | Retained |
| Quotes inside a plain scalar, including after content colons | Complete managed node replaced | Comment retained; additional public probes pass |
| Single scalar on following line | Complete managed node replaced | Value-only replacement |
| Block or continued multiline scalar | Complete managed node replaced | Documented layout refusal |

These type outcomes are intentional for a replacement API, rather than permissive parser defaults. The stored-hash reader validates stored hash shapes separately. Absence, spelled null, and an empty value produce distinct source edits. A no-write decision returns `None` before parsing. Markdown after the closing delimiter is body content, not trailing YAML.

The additional public leaf matrix used `q: [ordinary, "v"]` as its positive fixture, editing only `q` and requesting `q[1]`:

| Shape of `q` | Public leaf result |
| --- | --- |
| Positive control | Exact quoted span for `v` |
| Absent | `Missing` |
| Explicit null container | `Missing`: there is no indexed child; the parsed container remains null |
| Wrong whole-field scalar type, number | `Missing`: there is no indexed child |
| Wrong target element type | `NotAScalar` |
| Wrong type for every element | `NotAScalar`; elements are not filtered |
| Empty sequence | `Missing`: index 1 does not exist |
| Duplicate key | Document parse refusal |
| Trailing garbage after the collection | Document parse refusal |
| Valid plain sibling containing content-colon/quote or parentheses | Erroneous `UnsupportedShape`, as detailed above |

The raw source projection is a span reader, not a substitute for document validation. Its wrappers parse YAML first. The new findings concern valid lexical input and structural projection; there is no new defaulting/deserialization finding. The lexical tables include the relevant columns of siblings, keys, nested collections, quoted targets, and positive controls.

## Requirement verification and execution evidence

Level 1 is appropriate for source bytes, parsed values, cursor state, diagnostic ranges, and filesystem persistence. No requirement depends on terminal rendering or an OS keyboard encoder; Level 2/3 would not provide stronger evidence for these defects.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Six original examples, including CRLF empty date | Level-1 exact-byte writer output and typed refusal tests | Pass |
| Per-line LF/CRLF/CR preservation; mixed files; hash-only changes and inserted dates | Level-1 writer preservation matrix and CLI persisted-byte checks | Pass |
| Leading BOM and unterminated body | Level-1 exact-byte writer and CLI tests | Pass |
| Anchored/aliased/tagged dates; no-bump/no-write decisions | Level-1 writer tests and CLI/closure refusal without write | Pass |
| Unmanaged parsed values, reused-anchor safety, body bytes | Level-1 writer/restoration and persistent caller tests | Pass |
| Date quoting, spacing, comments, following-line values | Level-1 exact-byte writer, CLI and closure tests; additional content-colon controls | Pass |
| Repeated save and read/write/read stability | Level-1 writer, CLI save/diff, and closure round trips | Pass for covered fixtures |
| Quoted flow target beside a valid plain sibling/key | Level-1 public leaf and persistent closure tests for review 3 examples | Gap: content-colon/quote and parenthesis fixtures still fail |
| Cursor alternatives and argument roles retain expression grammar | Level-1 public cursor tests for ordinary constraints | Gap: quoted argument punctuation regresses alternative index and role |
| DMLS flow-value diagnostic ranges | Level-1 helper and diagnostic tests | Gap: content-colon/quote fixtures retain an incomplete range |
| Current docs and behavior comments | CLI guide, composition topic, darkmatter skill, changed reader contracts inspected | Writer docs match; the skill's quoted-target promise and cursor's grammar-preservation promise are violated by the findings |

Executed successfully on this host:

- `cd darkmatter && just test hash::write::`: 64 permanent tests.
- `cd darkmatter && just test schemas::simplified::`: 316 permanent tests.
- `cd darkmatter && just test hash_kind_save_diff::`: 27 permanent CLI tests.
- `cd darkmatter && just test diagnostics::frontmatter::`: 31 permanent DMLS tests.
- `cd claudine && just test-library composition::closure::`: 47 permanent closure/repair tests.
- Temporary Level-1 public writer, leaf, source projection, schema cursor, CLI save/diff, and persistent closure probes confirmed the instance tables. Additional diagnostic-helper probes confirmed the range failure. Passing diagnostic probes recorded observed failures; they are not evidence that the defective behavior is correct.
- `just check-tier-coverage darkmatter` and `just check-tier-coverage claudine`: zero stranded tests.

The permanent CLI module is declared in `tests/l1/main.rs` under an explicit Cargo target. Writer, schema, cursor, DMLS, and closure tests are compiled unit-test modules, without tier markers or feature gates excluding the changed tests from Level 1. CLI probes used `CliProcessFixture`. All temporary probes were removed, and all six source/test files used for probes were restored byte-for-byte to their pre-review state.

Full area suites and lint were not rerun for this document-only review. The implementation log records two unrelated full-suite failures; those claims were not independently reverified here and are not counted as passing evidence. Claudine emitted its existing large unwind-table linker warning; focused tests passed. Cross-OS proof remains a CI responsibility and does not determine this readiness decision.
