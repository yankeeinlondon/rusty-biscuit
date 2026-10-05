---
$schema: feature-review.yaml
ready: true
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-02T22:16:45-07:00"
spec: "2026-09-30-glob-reference/spec.md"
implemented: false
description: "A **feature** review of `2026-09-30-glob-reference/spec.md`"
feature: "2026-09-30-glob-reference/review-4.md"
previous: "2026-09-30-glob-reference/review-3.md"
---

This feature is **production ready** within this review's scope. Both unblocked findings from review #3 are implemented and verified. The earlier production defects remain resolved on their regression fixtures. No new correctness, test-placement, or appropriate-test-level gap was found, and no human design decision is required.

The review covers the specification, implementation plan and recorded departures, all three earlier reviews, the shared glob implementation, Darkmatter's consumers and schema readers, DMLS's schema cache and public projection, Claudine's two candidate producers, current documentation, and test registration. Existing working-tree changes were preserved. Only this review and the requested review/spec metadata were edited.

## Previous review verification

Review #3 has two unblocked findings and no blocked findings. There was therefore no blocked item to check for a later change in authorization. Earlier reviews' recurrence inspection did not block their implementation or reopen the feature's settled design.

### Filesystem capability assumptions — resolved across all five sites

**Class checked:** Regression fixtures must detect whether their filesystem supplies a case alias rather than infer that capability from the operating system.

The same existing ASCII, Unicode, and traversal-only fixtures were run on the default temporary filesystem and with only `TMPDIR` changed to a temporary case-sensitive HFSX volume. Correct-name and mismatched-name assertions still run in both environments. Only the assertion about exposing no root for an existing alias depends on whether that alias actually opens.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| biscuit-file [directory-case public API tests](../../lib/tests/l1/glob_reference/directory_case.rs) | Absolute, interpolated, exclusions, context-rooted forms, Unicode aliases, directory symlink, and temporary-directory spelling | All nine tests pass on both filesystems | Exact spellings match; mismatches fail; alias-specific root assertion is conditional on the actual capability |
| Darkmatter [glob consumer tests](../../../darkmatter/lib/tests/l1/glob_consumers.rs) | ASCII, Unicode, and traversal-only fixtures through `find_files()`, `::file-links`, and schema membership | All three affected tests pass on both filesystems | Consumers reject mismatches and accept stored names without requiring an alias to exist |
| Darkmatter [trigger tests](../../../darkmatter/lib/tests/l1/schema_roots.rs) | Same three fixture variants through trigger judgment and effective-schema assembly | All three affected tests pass on both filesystems | Correctly spelled trigger applies; mismatched trigger does not |
| Claudine [chooser candidate tests](../../../claudine/cli/src/completion/schema_completion/parity_tests.rs) | ASCII and both Unicode aliases, absolute and bare patterns | Chooser test passes on both filesystems | Stored spelling offers the file; mismatched spelling offers nothing |
| Claudine [shipped completion tests](../../../claudine/cli/tests/l1/compose_schema_cli.rs) | ASCII, Unicode, and traversal-only fixture variants through `__complete` | All three affected tests pass on both filesystems | Candidate results remain correct in either filesystem environment |

The five failures-at-setup sites identified in review #3 now use filesystem probes. The probes verify that an alias, when present, reaches the fixture's stored directory. No OS-based alias assertion remains at those sites. The four previously clean biscuit-file controls remain clean; no sibling was fixed by skipping its ordinary result checks. All fourteen previously failing regressions pass on the case-sensitive volume.

### Persistent input matrices — resolved in both reader families

**Class checked:** Every changed pattern reader must retain malformed-input regressions through its public result, including each public validator projection and source-syntax errors at a text-reader boundary.

Darkmatter's [JSON keyword matrix](../../../darkmatter/lib/tests/l1/match_keyword_input_matrix.rs) walks one base schema with one field edit per row through both public validator builders. Its control accepts an existing Markdown file and rejects an existing text file. The absent-keyword row accepts both, proving the keyword changes the verdict. Darkmatter's [YAML trigger matrix](../../../darkmatter/lib/tests/l1/schema_roots.rs) retains its effective-schema control and adds both trailing-source rows. Both matrices are registered in the consolidated Level 1 target and passed.

| Input shape | YAML trigger `$path` | JSON `x-darkmatter-match`, request-aware validation | JSON `x-darkmatter-match`, structural validation |
| --- | --- | --- | --- |
| Control | Contributes `from_trigger` to the effective schema | Markdown passes; text fails | Markdown passes; text fails |
| Absent | Empty condition arm fails as vacuous | Both pass; keyword is optional | Same |
| Explicit null | Load error naming null; empty YAML scalar also tested | Validator-build error requiring an array of strings | Same |
| Wrong type, whole field | Load error naming number | Validator-build error | Same |
| Wrong type, one element | Load error; valid string is not retained alone | Validator-build error; no discarded element | Same |
| Wrong type, every element | Load error | Validator-build error | Same |
| Empty collection | Error requiring a pattern | Error requiring a positive pattern | Same |
| Duplicate field key | Duplicate-key load error | Not representable in the typed JSON value this keyword reader receives | Same boundary |
| Valid document plus another document | Multiple-document load error | Text syntax belongs to the unchanged upstream JSON loader | Same boundary |
| Invalid trailing content | YAML load error at the appended line | Same upstream text boundary | Same boundary |
| Invalid glob | Pattern-definition error naming the pattern | Validator-build error naming the invalid glob | Same |

| Sibling reader or projection | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| YAML discovery/effective schema | Entire `$path` matrix, including both appended-source edits | Public errors or the positive effective-schema contribution above | Matches the documented outcomes |
| Request-aware keyword validator | Entire representable field matrix | Public build errors or file verdicts above | Matches the documented outcomes |
| Structural keyword validator | Same field edits with absolute file values | Same build errors and control verdicts | Matches request-aware behavior at this boundary |
| Simplified `file(match(...))` conversion and caller/document value origins | Existing invalid-definition, union, origin, and entry-point fixtures | Definition errors and origin-sensitive verdicts remain covered | Shared keyword factory; no separate array reader to duplicate |
| Captured environment/home schema-root inputs | Existing unset, empty, whitespace, relative, valid, duplicate-root, and snapshot-isolation rows | Existing input matrix passes | Typed strings/paths; null, arrays, and document syntax are outside this boundary |

The changed readers introduce no TOML, JSON5, manifest, or lockfile parser. The typed JSON boundary is explicitly documented beside its matrix; duplicate-key and trailing-text semantics are not invented inside a reader that receives an already parsed value.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

`recurrence: false`. No current finding repeats an earlier defect class. Review #1's directory case matching, failed traversal, missing literal-glob hints, and documentation-token findings, plus review #2's incomplete alias checks and erased root metadata errors, were compared with the current implementation. Their regression tests and consumer projections pass. Review #3's fixture-capability and persistent-matrix findings are resolved as shown above.

## Remaining contract sweep

| Decision or input family | Sites checked | Result |
| --- | --- | --- |
| Prefix parsing and rejected patterns | biscuit-file glob parser, shared root preparation, recursive single-file adapter; Darkmatter schema and trigger wrappers | One reference grammar; remote and recursive glob prefixes rejected; environment values escaped as literals; recursive payload brackets remain literal |
| Ownership, ordering, and exclusion | biscuit-file listing, membership, optional roots, detached membership; Darkmatter listing consumers; both Claudine candidate producers | Nearest-root judgment, merged roots, depth/component ordering, and exclusions remain covered by public-result and parity fixtures |
| Case-sensitive authored directories | Absolute, interpolated, relative and context-rooted forms; listing/membership; expressions, directives, triggers, completion and chooser | Exact-name checks retained, including Unicode and traversal-only regressions; both filesystem configurations pass |
| Failed filesystem reads | Glob root/descendant traversal and recursive resolution; Darkmatter expression/directive paths; five schema-root detector kinds; CLI and DMLS | Required failures propagate instead of becoming empty listings or selecting a later schema; missing-root controls pass |
| Literal-file failure hints | Single-file error representation, Darkmatter file/code/TOC/expression/schema projections, Claudine composition source | Shipped CLI regressions retain hints; literal brackets still resolve as file names |
| Boundaries and file symlinks | biscuit-file listing/membership; Darkmatter warning projections; Claudine candidate filtering | Escaping file links are skipped by bound listings with the required warning projections and omitted silently from suggestions |
| Schema discovery and caching | Shared five-root detector, trigger discovery, bare-name lookup, CLI display, DMLS overlay | Root order, shadowing, snapshot inputs, CLI/server agreement, and package/environment cache separation pass |
| Documentation and test registration | Current topic pages, affected skill references, declared targets, feature metadata, source guard and tier checks | Current behavior documented; new matrix module compiled and selected; no stranded tests |

The implementation records deliberate departures: membership/optional roots ignore context-unavailable patterns while complete listing APIs report errors; file identity preserves a symlink's own name while canonicalizing its parent; multiple positive patterns merge roots in first-appearance order. These choices are reflected in the public contracts and existing tests rather than treated as undocumented changes. The earlier nonblocking performance observation does not establish a new readiness defect; this review makes no benchmark claim.

## Requirement and test-level audit

The numbers below refer to the specification's numbered acceptance criteria. Every requirement concerns file sets, validation, schema choice, diagnostic content, or candidate values. Level 1 public API and hermetic binary tests are appropriate. This feature adds no terminal rendering, keyboard handling, scrolling, paste, mouse, or modifier-key requirement; no Level 2 or Level 3 verification mismatch was found.

| User-facing requirement | Criteria | Strongest relevant verification inspected | Assessment |
| --- | --- | --- | --- |
| Incident reproduction, prefix-aware candidates, exclusions, offered values validate | 1, 3–6, 12, 18 | Level 1 shipped completion, candidate and validation fixtures | Present and passing |
| One glob implementation and prepared-context boundaries | 2, 14 | Level 1 source guards and public boundary/composition fixtures | Present and passing |
| Value origins and malformed definition diagnostics | 7–8 | Level 1 origin, grammar, matrix and entry-point fixtures | Present and passing |
| Merged lists, filename view, consumer filters, changed outputs | 9–11, 13 | Level 1 public listing/composition fixtures; output log inspected | Present |
| Root/depth/component order and first-match behavior | 15–17 | Level 1 order, unreadable-root, completion and chooser fixtures | Present and passing |
| Literal names and local-first recursive lookup | 19, 23 | Level 1 single-file and glob public API fixtures | Present and passing |
| Entry-point agreement and candidates resolve back | 20, 24 | Level 1 shared parity and candidate rendering fixtures | Present and passing locally |
| Trigger matching and cache separation | 21–22 | Level 1 trigger, DMLS parity and cache fixtures | Present and passing |
| Escaping links omitted and explained | 25 | Level 1 listing, composition warning and candidate fixtures | Present and passing |
| Missing-path membership and case-sensitive text | 26 | Level 1 public API and all five consumer/test-site families above | Present and passing on both local filesystem configurations |
| Five roots, shadowing, captured inputs, CLI/server parity | 27–32 | Level 1 schema-root input and parity fixtures | Present and passing |
| Shipped sibling-schema example and variable spelling | 33–34 | Existing CLI example regression inspected; Level 1 documentation guard executed | Present; guard passes |

The new matrix module is declared by Darkmatter's consolidated `l1` target. All changed regression names are selected by Level 1. Claudine's fixture feature is enabled in its recipe and CI test metadata. Tier audits report zero stranded tests for biscuit-file, Darkmatter, and Claudine. Cross-OS CI evidence and external release-process checks are not readiness blockers under this review's instructions.

## Verification performed

All commands below completed successfully. Filtered-out tests are selection exclusions, not claims that those tests ran. Runs overlap; counts must not be added as distinct coverage.

| Area and command | Result |
| --- | --- |
| biscuit-file `just test` | 1,065 nextest tests passed; separate recipe-owned minimal-feature check passed six tests |
| Darkmatter `just test input_matrix` | 5 passed, including both JSON validator projections and extended YAML matrix |
| Darkmatter `just test glob` | 84 passed |
| Darkmatter `just test schema_roots` | 26 passed, including the documentation guard |
| Darkmatter `just test inaccessible` | 7 passed across library, CLI, and DMLS |
| Darkmatter `just test entry_point_parity` | 23 passed |
| Darkmatter `just test literal_glob_hint` | 6 shipped CLI tests passed |
| Darkmatter `just test schema_roots_parity` | 4 DMLS tests passed |
| Darkmatter `just test schema_cache_keys_on` | 2 DMLS cache tests passed |
| Claudine `just test-cli glob` | 34 passed |
| Claudine `just test-cli completion_file_match_directory_names` | 3 shipped completion tests passed |
| Claudine `just test-cli chooser_rejects_absolute_directory_case_aliases` | 1 chooser candidate test passed |
| Claudine `just test-cli literal_glob_hint` | 1 shipped CLI test passed |
| Claudine `just test-cli entry_point_parity` | 3 passed |
| Root `just check-tier-coverage` for each of biscuit-file, Darkmatter, and Claudine | All passed; zero stranded tests |

With `TMPDIR` pointing to the temporary case-sensitive HFSX volume: biscuit-file's `directory_case` selection passed all nine tests; Darkmatter's `absolute_` selection passed 28 tests; Claudine's chooser selection passed one test and its shipped directory-name completion selection passed three. The disk image was attached with `-nobrowse`, detached, and removed; no terminal or browser window was opened or focused.

No full-workspace green result is claimed. Unrelated failures reported by earlier implementation logs were not used as feature defects or claimed fixed. No formatting command, production source edit, commit, publication, or lifecycle-directory move was performed. The specification is marked completed as requested; its directory remains for the author's review-cycle closure.
