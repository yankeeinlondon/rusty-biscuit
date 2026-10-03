---
$schema: feature-review.yaml
ready: false
findings:
    - title: Type-expression readers mistake description and file-reference text for argument syntax
      priority: medium
human_review: true
human_review_items:
    - |-
        Inspect why the expression-reader correction still inferred grammar from punctuation alone. Compare the source-span reader and completion cursor with the parser's separate rules for arguments, human descriptions, and imported filenames. Check every failing and clean case in this review's instance table, including successful results that omit a property or include description text in a filename span. The expected outcome is a correction and regression table covering all three contexts before another automated review cycle.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-09-29T15:45:33-07:00
spec: 2026-09-28-hash-writer-byte-fidelity/spec.md
implemented: true
next: 2026-09-28-hash-writer-byte-fidelity/review-6.md
implemented_by: claude/opus
log: darkmatter/fixes/2026-09-28-hash-writer-byte-fidelity/log.md
description: "A **fix** review of `2026-09-28-hash-writer-byte-fidelity/spec.md`"
fix: 2026-09-28-hash-writer-byte-fidelity/review-5.md
previous: 2026-09-28-hash-writer-byte-fidelity/review-4.md
---

# Review of Hash Writer Byte Fidelity

The fix is **not production ready**. The original hash-writer requirements and the concrete examples from review 4 pass their focused tests. One medium-priority defect class remains in the schema-reader changes added during the review cycle: readers infer argument syntax from parentheses even when the parser is reading a description or an imported filename. Valid definitions can lose source spans, and the language server can stop offering completions for the next type alternative.

This repeats review 4's expression-grammar finding, so `recurrence` is `true` and human inspection is required before another automated cycle. The technical correction is actionable; it is not blocked by that inspection. This review changes only review documents and specification metadata.

## Previous finding disposition

Review 4 lists three Unblocked Findings and no Blocked Findings. No blocked finding became unblocked before the last implementation. Its human-review request concerns the incomplete sweep and is separate from technical blockage. The implementation log reports all three findings implemented and none deferred; this review does not independently establish whether the requested human inspection happened.

| Earlier finding | Verification | Disposition |
| --- | --- | --- |
| Review 4 — Content colons still reopen quote mode inside plain YAML scalars | Public leaf/source tests, persisted Claudine tests, cursor tests, and DMLS tests cover content colons, both internal quote styles, nested collections, and quoted controls. Key readers now share `mapping_separator`. | Reported examples resolved. |
| Review 4 — Raw YAML flow readers still treat parentheses as collection structure | Public source and leaf tests, declaration-cursor tests, and Claudine persistence tests cover unbalanced parentheses and parentheses balanced across separate entries. | Resolved. Raw YAML and type-expression splitting are now separate. |
| Review 4 — Type-expression cursors lost quote handling inside flow alternatives | Permanent cursor and completion tests cover quoted argument punctuation and escapes. Public expression-projection tests cover the additional cases discovered by the implementation. | Reported examples resolved; description and filename contexts remain incorrect, as detailed below. |
| Reviews 2 and 3 — Quotes inside plain YAML scalars interpreted as quoting syntax | Writer, leaf, raw source, CLI, and closure tests pass for their fixtures and the content-colon extensions. | Resolved on the raw YAML surfaces tested. |
| Review 1 — Hash replacement can silently change unrelated alias values | Writer/restoration tests compare unmanaged parsed values; CLI and closure tests verify refusal without writing. | Resolved. |
| Review 1 — Date writing mistakes indented comments for collection values | Exact-byte writer, CLI, and closure tests accept the supported scalar layouts and preserve comments. | Resolved. |

## Unblocked Findings

### Medium — Type-expression readers mistake description and file-reference text for argument syntax

**Defect class:** a type-expression reader infers lexical context from delimiter depth instead of the parser's argument, description, and file-reference modes, so content punctuation hides real boundaries or creates false ones.

In the darkmatter package, [scan_expression](../../lib/src/markdown/schemas/simplified/source.rs:1641), the new shared scanner for source projection and flow-alternative cursors, opens a quoted string whenever parenthesis depth is nonzero. Its comment equates that depth with an argument list. The parser has other contexts that can contain parentheses: [read_inline_object_description](../../lib/src/markdown/schemas/simplified/grammar.rs:729) reads human prose without interpreting quotes, and [read_fileref](../../lib/src/markdown/schemas/simplified/grammar.rs:1043) reads imported filenames without interpreting their opening parentheses or brackets as argument or object syntax.

Copy the permanent expression-projection fixture that contains an apostrophe in an inline-object description, and put that description in parentheses:

```yaml
"{ a: string -> (it's fine), b: string(suggest(x, y)) }"
```

The public semantic parser accepts both properties. The public [parse_property_definition_with_source](../../lib/src/markdown/schemas/simplified/source.rs:127), which adds authored source ranges to that parsed definition, returns a projection error. The apostrophe is prose, but the scanner opens a string and never reaches the object's closing brace. The same result occurs with `(say "hi)` inside the description, through either YAML quote style and under LF or CRLF.

Balancing quotes across **different properties** hides the failure rather than fixing it:

```yaml
"{ a: string -> (it's fine), b: string -> (it's clear), c: string(suggest(x, y)) }"
```

The semantic result contains `a`, `b`, and `c`. Source projection returns success but omits **all spans for `b`**; the definition span for `a` also covers `b`. Checking only `is_ok()` would miss this. An unmatched `[` in description prose or an imported filename produces the same merged-property result, although brackets have no nesting meaning in those parser contexts.

The language-server effect is reproducible through DMLS's completion provider. Copy its permanent next-alternative completion fixture and change only the first alternative:

```yaml
---
$schema:
  title: [string -> (it's fine), s]
---

body
```

At the cursor just after `s`, the public darkmatter [locate_type_definition_cursor](../../lib/src/markdown/schemas/simplified/cursor.rs:105) returns `None`, and DMLS [completion](../../dmls/src/providers/frontmatter.rs:58) offers no `string` completion. Controls using `plain`, `(plain)`, or `it's fine` as the description all offer it. This does not depend on terminal rendering or keyboard encoding.

The same missing context occurs with `[Name@./a(b.yaml, s]` and `[Name@./a{b.yaml, s]`. The parser's imported reference is opaque until an arrow, comma, or closing brace; an opening parenthesis or brace in the filename does not open an expression frame. No file access or filename resolution is needed to reproduce the reader error.

The following table consolidates the sweep. Every expression was first accepted by the semantic parser. Source projections used authored YAML scalars, a nonzero document offset, single and double YAML quotes, and LF/CRLF. Cursor and DMLS probes used the same edited expression/reference controls. “Next alternative” means a final `str` or `s` after one completed alternative, which should have alternative index 1 and a type role.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| darkmatter shared expression scanner; delimiter matcher and recursive inline-object projection | Description `(it's fine)`, `(say "hi)`, `({x} it's fine)`; nested inline object containing the same description | Public property projection refuses a valid parsed definition | Preserve the parser's prose boundaries and return the complete source map |
| darkmatter expression entry splitter and inline-object projection | Apostrophes balanced across descriptions of `a` and `b`; description `plain [x`; imported reference `Name@./a[b.yaml` before `b` | Returns success but omits `b`; the previous property's definition span includes it. In the filename case, `b`'s constraints are attributed to `a` | Return separate, exact spans for every parsed property and its constraints |
| darkmatter delimiter matcher and constraint-group reader | Standalone and nested `Name@./a(b.yaml`; filenames containing an opening parenthesis followed by either quote style; filename followed by a description | Refuses projection because filename bytes are interpreted as an unclosed constraint group/string | Project the reference; do not scan its content for constraints |
| darkmatter top-level arrow reader and reference projection | `Name@./a{b.yaml -> description` | Returns success with import-reference text `./a{b.yaml -> description` | Reference span contains only `./a{b.yaml`; description is excluded |
| darkmatter top-level separator reader and import-name lookup | Ordinary, parenthesized, and brace-containing references | Finds the initial `@` correctly; failure occurs in the surrounding delimiter/arrow/group readers | Clean for the initial separator; retain this result while repairing the surrounding readers |
| darkmatter flow type-definition cursor | Parenthesized descriptions containing an unmatched prose quote; filenames containing opening `(`, `[`, or `{`, standalone and inside inline objects | Returns `None` or keeps the final token in alternative 0; a description containing unmatched `]` inside an object can instead report alternative 2 | Final token belongs to alternative 1 with the exact authored replacement range |
| darkmatter scalar type-definition cursor | `Name@./a(b.yaml`, without a flow sequence | Reports a constraint role and token `b.yaml`, rather than an import-reference role and the complete reference | Retain the import-reference context through filename punctuation |
| darkmatter scalar cursor after an inline description | `{ a: string -> plain, b: str` and the parenthesized-apostrophe variant | Returns `None`, even though the cursor has passed the description's terminating comma | Type context for property `b`. This sibling predates this iteration and also fails the quote-free control |
| DMLS type-definition completion provider | `[string -> (it's fine), s]`, double-quote prose counterpart, `[Name@./a(b.yaml, s]`, `[Name@./a{b.yaml, s]` | No `string` completion after the first alternative | Offer the same type completion and replacement range as the positive controls |
| darkmatter expression projection and cursor controls | Ordinary prose; apostrophe/double quote outside parentheses; `(plain)`; quote pairs balanced inside one description; complete parentheses inside one filename; closing-parenthesis filename; real `suggest('a)b', c)` argument | Complete maps and correct next-alternative context | Clean; retain genuine argument quoting and escape handling |
| darkmatter suggestion-only projection (`parse_yaml_schema_with_source` / `project_suggestion_spans`) | All expression fixtures above, including nested descriptions and imports | Projects successfully using semantic candidate spans and YAML decoding | Clean. This separate implementation does not prove the full structural source map is correct |
| darkmatter raw YAML delimiter/splitter, declaration cursor, public leaf locator, hash writer, and identical-snapshot property restoration | Failing expressions stored as genuinely YAML-quoted data, LF/CRLF | Complete collections/entries, exact leaf spans, correct declaration alternative, unchanged unmanaged property bytes, stable restoration | Clean; these consumers must retain YAML rules |
| Grammar authority: argument lexer, description reader, import-reference reader | Every semantic control and edited fixture above | Accepts the definitions with their intended properties/references | Clean for these reproductions; use each context's actual boundary rules |

The underlying helper sites are [matching_delimiter](../../lib/src/markdown/schemas/simplified/source.rs:1579), [split_top_level](../../lib/src/markdown/schemas/simplified/source.rs:1603), [find_top_level_arrow](../../lib/src/markdown/schemas/simplified/source.rs:1536), [find_top_level_byte](../../lib/src/markdown/schemas/simplified/source.rs:1556), and [scan_constraint_groups](../../lib/src/markdown/schemas/simplified/source.rs:1444). Their structural callers are `scan_expression_range` and `project_inline_shape_range` in the same darkmatter file. The cursor callers are [expression_flow_arm_starts](../../lib/src/markdown/schemas/simplified/cursor.rs:215) and [scan_scalar](../../lib/src/markdown/schemas/simplified/cursor.rs:275). The raw declaration splitter and quoted-prefix reader remain separate: genuine argument quotes and quoted declaration controls pass.

**Required correction:** make structural readers follow the parser's current lexical context. Description parentheses must not turn prose quotes into argument strings; description brackets must not become array nesting. Imported filenames must not create constraint/object/array frames, and their description arrow must end the filename span. Preserve the argument lexer and raw YAML readers' existing rules. The scalar cursor must also resume after an inline description's terminating comma and preserve its import-reference role while reading a filename.

Extend the existing public fixture tables with both quote styles, punctuation that opens and closes nesting, quotes balanced across different properties, nested objects, and imported filenames. Assert the entire expected property-path set and exact definition/reference/constraint spans, as well as cursor role, alternative index, token, and replacement range. Keep the DMLS provider positive controls beside the failing cases. Update the scanner and skill descriptions: “inside parentheses” is not equivalent to “inside an argument list.”

No performance or ergonomic issue independently blocks readiness. The correction should keep grammar authority shared without applying one context's rules to every kind of text.

## Blocked Findings

None. The technical correction can be implemented without an unresolved design choice or unavailable test environment. Human inspection is requested because the recurring class requires the automated loop to stop.

## Recurrence

This finding repeats review 4's **“Type-expression cursors lost quote handling inside flow alternatives.”** That finding classified the defect as a type-expression reader using incompatible quote/delimiter rules. Its implementation widened the correction to decoded-expression helpers and explicitly claimed that the new scanner matched the lexer for descriptions and file references.

The previous sweep checked apostrophes outside parentheses and punctuation inside genuine quoted arguments. It should also have checked parentheses and brackets in description prose, opening punctuation in imported filenames, the scalar cursor's import/description modes, and source maps that return success while omitting properties. Those sibling sites are now enumerated above, together with the separate suggestion projector and raw YAML readers that remain clean.

Reviews 2 and 3 concern the related but distinct raw-YAML scalar-boundary error; their concrete cases pass. Review 4's raw-YAML parentheses class is resolved. Review 1's unmanaged-alias and physical-line-count classes do not recur. Some additional expression-context instances predate this iteration; they are included because fixing the shared reader class without them would leave the same incomplete sweep in place.

## Input robustness matrix

The managed input format is YAML frontmatter. The writer's load-bearing inputs remain the configurable managed hash property and `last_updated`. The permanent 58-row matrix starts from a positive document and changes one field per row; the public writer output/refusal, unmanaged parsed values, and retained body bytes are asserted. It passed in this review.

| Shape | Managed hash property | `last_updated`, bump requested |
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
| Anchor, alias, tag | Replaced only if output parses and unmanaged values stay equal | Refused |
| Comments and supported following-line scalar | Whole managed node replaced | Value-only edit; surrounding bytes retained |
| Block or continued multiline date | Whole managed node replaced | Documented layout refusal |

These are deliberate replacement/refusal rules, not defaults that hide malformed input. A no-write decision still returns `None` before parsing. Markdown after the closing delimiter is body content.

The schema-reader finding concerns accepted string content and incorrect ranges/context, not a new manifest or configuration field, silently filtered collection elements, or absent/null conflation. The semantic parser accepts each reported expression before projection. The lexical instance table therefore includes positive controls, unbalanced punctuation, balanced punctuation within one value and across separate properties, both YAML quote styles, recursive objects, and the complete public result rather than just parser success.

## Requirement verification and execution evidence

Level 1 is appropriate for these requirements: source bytes, YAML values, source ranges, completion results, and persisted files. No requirement in this fix depends on emulator rendering, OS keyboard events, or terminal input encoding. Level 2 or 3 would not strengthen these checks.

| User-facing requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| All six original examples, including CRLF empty date | Level-1 exact-byte writer output and typed refusal tests | Pass |
| Per-line LF/CRLF/lone-CR preservation, mixed files, insertion and hash-only saves | Level-1 writer matrix and CLI persisted-byte tests | Pass |
| Leading BOM and body without a terminal newline | Level-1 exact-byte writer and CLI tests | Pass |
| Anchored/aliased/tagged dates; collections and unsupported layouts; no-bump/no-write paths | Level-1 writer and persistent-caller refusal tests | Pass |
| Unmanaged parsed values, reused-anchor safety, unchanged body bytes | Level-1 writer/restoration and CLI/closure tests | Pass |
| Date spelling, spacing, comments, following-line values, repeated saves | Level-1 exact-byte writer, CLI, and closure round trips | Pass |
| Quoted flow targets beside content colons and parentheses | Level-1 public leaf/source and Claudine persistence tests | Pass for the reported YAML cases |
| Expression source maps and completion context match the semantic grammar | Level-1 public projection/cursor and DMLS completion tests | Gap: description/reference cases fail or return incomplete/oversized spans |
| Current writer documentation and behavior comments | Hash guide, composition topic, darkmatter skill, and changed helper contracts inspected | Writer docs match; expression scanner/skill overstate agreement with the lexer |

Permanent checks executed successfully:

- `cd darkmatter && just test hash::write::`: 67 tests.
- `cd darkmatter && just test schemas::simplified::`: 325 tests.
- `cd darkmatter && just test hash_kind_save_diff::`: 28 CLI tests.
- `cd darkmatter && just test frontmatter::`: 288 tests, including DMLS completion and diagnostic tests.
- `cd darkmatter && just test schemas_source_projection::`: 6 public source-projection tests.
- `cd darkmatter && just test overlay::schema::`: 14 DMLS schema-overlay tests.
- `cd claudine && just test-library composition::closure::`: 50 closure/repair tests.
- `just check-tier-coverage darkmatter` and `just check-tier-coverage claudine`: zero stranded tests in either area.

Temporary Level-1 probes confirmed the failing public expression projections and cursor contexts, missing property spans, oversized filename spans, and DMLS completion failures. A separate passing probe verified that the same expression text remains data for raw YAML readers, the leaf locator, hash writing, and restoration. The discovery probes intentionally failed assertions of the expected public behavior; they are not passing regression coverage. Both temporarily edited source files were restored byte-for-byte to their pre-review contents, preserving the developer's existing changes.

The changed writer, cursor, source, DMLS, and closure tests are compiled unit-test modules without tier markers that exclude them from Level 1. `hash_kind_save_diff` and `schemas_source_projection` are declared in their respective `tests/l1/main.rs` files and explicit Cargo targets; `autotests = false` does not strand them. CLI tests use `CliProcessFixture`.

Full area suites and lint were not rerun for this document-only review. The implementation log records two unrelated full-suite failures; those claims were not independently reverified here and are not counted as passing evidence. Claudine emitted its existing unwind-table linker warning; its focused tests passed. Cross-OS evidence remains a CI responsibility and does not determine this readiness decision.
