---
$schema: feature-review.yaml
ready: false
findings:
  - title: Absolute glob directory matching still follows filesystem aliases
    priority: high
  - title: Inaccessible schema roots are silently treated as absent
    priority: high
  - title: The schema-variable documentation guard remains red
    priority: medium
human_review: true
human_review_items:
  - |-
      Review the completeness of the previous fixes before restarting the automated review loop. Directory-case matching and filesystem-error handling still have failing sibling cases, and the exact documentation failure from review #1 remains. Check that the next implementation addresses every row of the instance tables and commits public-result regression tests, rather than fixing only the first reproduction. No previously settled design decision needs reaffirmation.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-02T20:18:24-07:00"
spec: "2026-09-30-glob-reference/spec.md"
implemented: true
implemented_by: claude/opus
log: biscuit-file/features/2026-09-30-glob-reference/log.md
description: "A **feature** review of `2026-09-30-glob-reference/spec.md`"
feature: "2026-09-30-glob-reference/review-2.md"
previous: "2026-09-30-glob-reference/review-1.md"
next: "2026-09-30-glob-reference/review-3.md"
---

This feature is **not production ready**. The previous fixes improve ordinary glob traversal and user-facing hints, but absolute directory-case matching remains incorrect, schema discovery can silently omit required rules after a permission failure, and the previous documentation guard still fails. All three findings recur from review #1. Human review of the incomplete class sweeps is required before another automated cycle; the code fixes themselves are unblocked.

The review covers the specification, implementation records, every earlier review in this directory (review #1), shared glob parsing/preparation/listing/membership, recursive resolution, Darkmatter composition and schema readers, trigger discovery, DMLS projections, Claudine candidate walks, documentation, and test registration. Existing working-tree changes were preserved. Temporary Rust examples exercised public APIs with isolated fixtures; shipped Claudine completion exercised the candidate projection without opening a terminal window. The examples were removed afterward. No production implementation was edited.

## Previous review verification

Review #1 has a `Findings` section rather than separate `Unblocked Findings` and `Blocked Findings` sections. All four findings were explicitly unblocked; no blocked finding or subsequent design decision needed reconsideration.

| Previous finding | Verification | Status |
| --- | --- | --- |
| Absolute glob roots bypass case-sensitive directory matching | ASCII `docs`/`DOCS` controls now pass. Unicode case aliases and an ancestor that permits traversal but not enumeration still bypass the check. | Partially implemented; finding below |
| Glob traversal silently discards filesystem errors | Glob walk errors, recursive-resolution errors, directive walk errors, and trigger entry-read errors now propagate. Schema-root metadata failures still disappear before enumeration. | Partially implemented; finding below |
| Literal-glob hints never reach user-facing failures | Shared hint producers now feed composition, transclusion, expressions, schema references/file values, DMLS diagnostics, and Claudine errors. CLI regressions cover glob misses, plain misses, other failures, and successful controls. | Implemented |
| The schema-variable documentation guard fails on the new skill page | The same whole-word token remains in the same skill page; the declared Level 1 guard fails. | Not implemented |

The requested `implemented: true` update on review #1 records completion of its implementation attempt. It does not mean this review verified that every finding was resolved.

## Unblocked Findings

### High — Absolute glob directory matching still follows filesystem aliases

**Defect class:** Moving authored literal directories into a filesystem-resolved glob root permits differently spelled directory names whenever the compensating exact-name check misses an alias or cannot enumerate an ancestor.

In biscuit-file, [prepare](../../lib/src/file_reference/glob/roots.rs) removes the literal directory prefix of an absolute glob from its matcher. The new [respelled_by_case](../../lib/src/file_reference/glob/roots.rs) check compares lowercase strings to detect aliases. Lowercasing is not the filesystem's equivalence rule: on the reviewed macOS filesystem, `Σ` and `ς`, and `SS` and `ß`, each open the same directory but have different lowercase strings. Consequently, an absolute glob with the wrong directory spelling still matches. Relative globs retain the directory text in the matcher and reject it.

The same helper returns `false` when `read_dir` fails. This means “no mismatch detected,” so a directory that allows traversal but not enumeration makes even the ASCII `docs`/`DOCS` regression return. This is a successful incorrect listing, not an unreadable search directory: the child directory and file remain readable.

**Reproductions:**

1. Copy the existing directory-case fixture, replace stored `docs` with `Σ`, and replace authored `DOCS` with `ς`. Keep `Σ/a.md` as the positive control. Repeat with stored `SS` and authored `ß`. Test absolute patterns, captured `ROOT` interpolation, exclusions, and each context-rooted prefix.
2. Create `locked/anchor/docs/a.md`. Keep `docs` as the stored name and author absolute `locked/anchor/DOCS/*.md`. Changing only `locked` from mode `0755` to `0111` makes the previously rejected pattern list the file. Confirm `read_dir(locked)` fails while `read_dir(locked/anchor/docs)` succeeds; restore permissions during teardown. The Rust probe used the equivalent `schemas`/`SCHEMAS` child names.

Both reproductions were exercised through the public APIs and consumer results below. “Reject” means an empty listing/no first match and false membership, not rejection of the pattern's syntax.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| biscuit-file `list_files`, `take_first`, `matches`, `lists_file` | Absolute `ς/*.md` over stored `Σ/a.md`; also `ß` over `SS` | Lists/returns the file; both membership APIs return true | Reject the different authored directory spelling |
| Same four APIs | `{{ROOT}}/ς/*.md` and `{{ROOT}}/ß/*.md`, with a correctly spelled captured root | Same incorrect acceptance | Reject authored suffix mismatch |
| biscuit-file `roots`, `matches_without_context` | Absolute Unicode mismatches | Exposes the mismatched search root; detached membership returns true | No admitting root; false membership |
| Same four listing/membership APIs and detached membership | Absolute ASCII mismatch below a traversal-only ancestor | Lists/returns/admit the file | Reject the mismatch; inability to check spelling must not approve it |
| Same APIs | Absolute ASCII mismatch with readable ancestors; correctly spelled absolute patterns in both permission states | Mismatch rejected; correct spelling accepted | Clean controls |
| Same four APIs; Darkmatter `FileMatchGlobs` | Bare, `./`, `../`, `&`, `^`, `@`, `~/`, and `vault:` Unicode mismatches | Empty lists and false membership | Clean: directory text remains in the matcher |
| biscuit-file exclusion judgment | Positive `**/*.md` plus an absolute mismatched exclusion | Incorrectly removes the stored file for the Unicode mismatch | A differently spelled exclusion must not match |
| biscuit-file recursive `FileReference` | Absolute `%` payload using the filesystem alias | Resolves the literal file | Clean exception: single-file resolution keeps filesystem case semantics; its directory text is supplied as literal input |
| Darkmatter [find_files](../../../darkmatter/lib/src/markdown/compose/expression/functions/mod.rs), through composition | Absolute and environment-expanded Unicode mismatch; absolute ASCII mismatch below traversal-only ancestor | Frontmatter array contains the file | Empty array |
| Darkmatter [file-links discovery](../../../darkmatter/lib/src/markdown/compose/file_links/discovery.rs), through composition | Absolute Unicode mismatch and traversal-only-ancestor ASCII mismatch | Rendered tree contains `a.md` | No matching file |
| Darkmatter [FileMatchGlobs](../../../darkmatter/lib/src/markdown/schemas/file_match.rs), shared validation/completion judgment | Both mismatch fixtures | Admits the file | Reject it |
| Darkmatter [PathGlobs](../../../darkmatter/lib/src/markdown/schemas/triggers/grammar.rs) and the public trigger evaluator shared by `md` and DMLS | Absolute Unicode and traversal-only-ancestor ASCII mismatches | Trigger predicate returns true | Trigger must not apply |
| Same trigger evaluator | Bare, `./`, `../`, `&`, `^`, and home Unicode mismatches; `@`, vault, environment forms | Allowed forms reject; forbidden forms return definition errors | Clean controls |
| Claudine shipped `__complete`, through [match_glob_files](../../../claudine/cli/src/completion/schema_completion/candidates.rs) | Absolute Unicode mismatch | Offers `spec='ς/a.md'`; relative `ς/*.md` offers nothing | Absolute mismatch must also offer nothing |
| Same shipped completion | Absolute ASCII mismatch below traversal-only ancestor | Offers `spec='locked/anchor/DOCS/a.md'` at mode `0111`, but nothing at `0755` | Both must offer nothing |
| Claudine ENTER chooser | Same candidate judgment | Uses the same `match_glob_files` and `FileMatchGlobs::lists_file` as TAB; no independent case policy | Fix the shared producer; add both fixtures to its chooser parity coverage |

The chooser was checked through its shared public matcher and existing in-process parity test; no interactive chooser was opened. DMLS's trigger projection uses the same public evaluator; its existing CLI/server parity tests passed, but do not contain these directory fixtures.

**Required change:** Enforce the exact authored directory spelling without assuming lowercase equality describes filesystem aliases, and never interpret an inability to inspect spelling as successful verification. Preserve legitimate directory symlinks, supplied environment roots, and `%` single-file semantics. Cover positive and negative absolute patterns, environment-expanded patterns, every public view, and both consumer fixtures in permanent Level 1 tests. The documented Linux case-insensitive-directory exception also conflicts with the specification's every-OS promise; the fix must not depend solely on canonicalization changing the spelling. That Linux exception was inspected, not reproduced on another host in this review.

### High — Inaccessible schema roots are silently treated as absent

**Defect class:** A root detector converts filesystem metadata failures into absence, allowing a reader to return an incomplete schema set or choose a less local schema as if the preferred root did not exist.

Darkmatter's [SchemaRoots](../../../darkmatter/lib/src/markdown/schemas/roots.rs) determines which of the five folders feed trigger discovery and bare-name schema lookup. Its `is_searchable_directory` uses `Path::is_dir()` for environment/home roots and `symlink_metadata(...).is_ok_and(...)` for package/area/tree roots. Both erase `PermissionDenied`. The caller records `SchemaRootState::Absent`, so the improved enumeration error handling never sees that root.

This changes validation, not just a diagnostic: a required schema in a preferred root can be replaced by a permissive home schema with the same name. With no later schema, callers receive a generic no-match instead of the I/O failure.

**Reproduction:** Copy a schema-root fixture containing `locked/anchor/schemas/policy.yaml` with `hidden_rule: string(required)` and a valid trigger using that schema. Put a same-named fallback under a separate, readable fixture home with `fallback_rule: string`. Supply `locked/anchor` as each applicable context anchor in turn; for the environment root supply `SCHEMAS_DIR=.../locked/anchor/schemas`. Validate the contexts first. The readable control loads the trigger and resolves `hidden_rule`. Change only `locked` to mode `000`, directly confirm `PermissionDenied` for the schema folder, then repeat `SchemaRoots::for_document`, public `scan`, and public bare-name resolution. Restore permissions afterward.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Package schema-root detector | Inaccessible ancestor of `{package}/schemas` | Root is `Absent`; scan succeeds without its trigger; lookup selects home fallback | Preserve I/O error; no successful partial discovery or fallback |
| Package-area schema-root detector | Same edit for `{area}/schemas` | Same omission and fallback | Same error |
| Tree schema-root detector | Same edit for `{base_dir}/schemas` | Same omission and fallback | Same error |
| Environment schema-root detector | `SCHEMAS_DIR` folder behind inaccessible ancestor | Same omission and fallback | Same error |
| Home schema-root detector | Fixture home behind inaccessible ancestor | Root is `Absent`; scan succeeds empty; lookup reports no-match | I/O error naming the inaccessible path |
| All five detectors and their lookup/discovery consumers | Unedited readable fixture | Trigger loads; preferred `hidden_rule` schema resolves | Clean positive controls |
| Trigger root enumeration, all five kinds | Schema folder itself mode `000`, while its ancestors remain traversable | `scan` returns an I/O error | Clean: existing unreadable-root regression covers this different shape |
| Bare-name `$schema` lookup, all five kinds | Same directly unreadable schema folder | Typed resolution I/O error | Clean: the reader propagates errors when the detector includes the root |
| biscuit-file glob listing root preparation/walk | Same ancestor/folder edits, using absolute, captured environment, vault, home, and magic roots | Typed `GlobReferenceError::Io` | Clean: complete glob listings now preserve these failures |
| biscuit-file `take_first` and recursive `%` | Existing unreadable-root and descendant fixtures | Typed I/O failures when the unreadable entries could affect the first match | Clean; public regression tests pass |
| Darkmatter `find_files`, glob `::file-links`, directory-mode enumeration | Existing unreadable-directory fixtures | Typed expression/directive failure | Clean after the previous walk fix |
| Trigger per-entry reader | A failed `read_dir` entry or `file_type` result | Explicit error propagation in the reader | Clean by code inspection; no deterministic per-entry fault was injected |
| Claudine suggestion walker; biscuit-file lexical membership/optional roots | Errors while collecting optional suggestions or preparing a context-unavailable pattern | May omit suggestions/patterns under their stated contracts | Intentional exception; these do not claim a complete schema discovery |

Both metadata policies and all five root kinds were reproduced. The trigger registry, bare-name lookup, CLI schema-root display, and DMLS schema discovery share this root list; they must receive the correction from the detector rather than independently retrying omitted folders.

**Required change:** Make root classification fallible, or retain a typed error state that every complete discovery and lookup operation propagates. Distinguish genuinely missing/non-directory folders and intentionally excluded symlinks from metadata failures. Add an inaccessible-ancestor row to the five-root public-result matrix, with a later same-named schema proving that an error cannot change precedence. Preserve the existing directly unreadable-folder and missing-folder controls.

### Medium — The schema-variable documentation guard remains red

**Defect class:** Shipped documentation contains a token prohibited by its acceptance-test contract, leaving the normal test gate failing.

The exact issue from review #1 remains at [the Darkmatter testing skill](../../../.claude/skills/darkmatter/testing.md):77. Its guard explanation still spells the whole-word singular `SCHEMA_DIR`. The explanation is not a recommendation to configure that variable, but acceptance criterion 34 and the committed guard forbid the occurrence regardless of context.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Darkmatter skills tree | Whole-word singular-variable scan | One occurrence in `testing.md`; Level 1 guard fails naming that file | No occurrence; guard passes |
| Darkmatter docs tree | Same scan | No occurrences | Clean |
| Claudine docs and skills trees | Same scan | No occurrences | Clean |
| biscuit-file docs and skills trees | Same scan | No occurrences | Clean |

**Required change:** Rephrase the explanation without spelling the forbidden token and rerun the existing guard. No new test or design decision is needed.

## Blocked Findings

None. The recurrence review is an external process check, not permission needed to implement any finding.

## Recurrence

Every finding repeats a class from [review #1](review-1.md):

| Current finding | Earlier finding | Sibling sweep the earlier fix missed |
| --- | --- | --- |
| Absolute glob directory matching still follows filesystem aliases | “Absolute glob roots bypass case-sensitive directory matching” | Non-ASCII filesystem case aliases; exact-name checking when an ancestor cannot be enumerated; absolute negations and each consumer projection under these shapes |
| Inaccessible schema roots are silently treated as absent | “Glob traversal silently discards filesystem errors” | The schema-root metadata detectors that decide whether enumeration runs, in both symlink policies and all five root kinds; their precedence effect on bare-name lookup |
| The schema-variable documentation guard remains red | “The schema-variable documentation guard fails on the new skill page” | The originally identified `testing.md` occurrence itself; the complete docs/skills scan confirms no other current instance |

The current review carries the complete tested sibling lists and clean controls above. `recurrence: true` requires stopping the automated loop for human inspection of fix completeness, as requested. This does not reopen the specification's settled prefix, boundary, or ordering decisions.

## Input robustness audit

The changed format readers are YAML trigger `$path` and the typed JSON Schema `x-darkmatter-match` keyword. No new TOML, lockfile, JSON5, or manifest parser is introduced. Schema-root environment and home inputs arrive as captured typed strings/paths; their absent, empty, relative, and valid states are covered by the existing schema-root matrix. Their filesystem failures are the second finding, rather than a field-type coercion.

Temporary public-result probes started from the same valid fixtures as the existing tests and made one edit per row. The YAML control contributed `from_trigger` to the document's effective schema. The JSON control accepted an existing Markdown file and rejected an existing text file, proving the field affects the verdict.

| Shape | YAML trigger `$path` | Typed JSON Schema `x-darkmatter-match` |
| --- | --- | --- |
| Positive control | `docs/*.md` applies and contributes its payload | `["docs/*.md"]` accepts `.md`, rejects `.txt` |
| Absent | Empty match arm fails the vacuous-trigger check | Both files accepted: this optional keyword adds no constraint |
| Explicit null | Error naming null; empty YAML scalar is the same YAML null value | Validator-build error requiring an array of strings |
| Wrong type, whole field | Error naming number | Validator-build error |
| Wrong type, one element | Error; valid pattern not retained as a partial list | Validator-build error; no element silently discarded |
| Wrong type, every element | Error; no empty-list substitution | Validator-build error |
| Empty | Error requiring a pattern | Error requiring a positive pattern |
| Duplicate key | YAML duplicate-key load error | Not representable at this factory's typed `Value` boundary; unchanged source loader is outside the changed keyword reader |
| Trailing/invalid content | Additional YAML document and invalid trailing YAML each produce load errors | Not representable at the typed keyword boundary |
| Invalid pattern/forbidden prefix | Existing matrix rejects malformed globs and forbidden trigger forms | Shared glob constructor rejects invalid patterns |

No new field-shape defect was found. The committed YAML matrix lacks the trailing-document rows exercised here, and the complete raw-keyword shape matrix is review evidence rather than a committed matrix test; retain these probes as parameterized public-result regressions when expanding the tests.

## Requirement and test-level audit

This feature promises filesystem lists, schema selection, diagnostic text, and candidate contents/order. Level 1 public API and hermetic binary tests are appropriate. It adds no keyboard-encoder, hotkey, mouse, scrolling, or terminal-style requirement needing Level 2 or 3. The ENTER chooser's file source can be verified in process; opening its existing UI is not necessary to prove a shared glob decision.

Criterion numbers below identify the numbered acceptance list in the specification; each row also states the behavior.

| Requirement | Criteria | Present verification | Assessment |
| --- | --- | --- | --- |
| Prefix-aware completion and nested launch behavior | 1, 3, 7 | Level 1 completion/composition and entry-point parity | Present |
| One glob implementation; no ambient fallback | 2, 14 | Level 1 source guards and context tests | Present |
| Exclusions, nearest-root ownership, offered/admitted parity | 4–6, 12, 18 | Level 1 public glob/candidate/validation tests | Present; absolute mismatched exclusions remain broken |
| Definition errors and vault patterns | 8, 21 | Level 1 grammar, public matching, trigger matrix | Present |
| Merged lists, consumer filters, controlled filename widening | 9–11, 13 | Level 1 glob consumers and file-links tests | Present |
| Relative boundary, directory/file symlinks | 14, 25 | Level 1 filesystem and compose fixtures | Present |
| Root/depth/component order, first match, completion rendering | 15–17, 24 | Level 1 ordering, chooser parity, shipped completion | Present |
| Literal brackets and local-first recursion | 19, 23 | Level 1 public API tests | Present |
| Library/CLI/DMLS/completion parity | 20, 32 | Level 1 entry-point and schema-root parity | Existing fixtures pass; add the reproduced failure shapes |
| Context-keyed DMLS trigger cache | 22 | Level 1 repository-context tests | Present |
| Case-sensitive lexical membership | 26 | Level 1 case tests, including newly added ASCII directory fixtures | Incomplete; first finding |
| Five roots, precedence, user folders, omitted intermediate folders | 27–31 | Level 1 public root/lookup/discovery and CLI/server tests | Normal states pass; inaccessible-parent state missing and broken |
| Shipped example and variable documentation | 33–34 | Level 1 example/docs guards | Variable guard fails; third finding |

Test registration was checked: biscuit-file's `l1` target declares `glob_reference`, which declares `directory_case` and Unix-only `unreadable`; new CLI hint files and consumer tests are declared by their consolidated targets. `just check-tier-coverage` found no stranded tests in biscuit-file, Darkmatter, or Claudine. Unix permission tests need Level 1, not a real-terminal tier. Missing cross-OS execution evidence is not a readiness finding.

## Verification performed

- biscuit-file `just test`: **1,060 passed**, zero skipped; its recipe's separate six-test minimal-feature check also passed.
- Darkmatter `just test glob`: **82 passed** across library, CLI, DMLS, and editor-wrapper binaries.
- Darkmatter `just test schema_roots`: **17 passed, 1 failed**; the failure is the unchanged documentation guard.
- Darkmatter `just test unreadable`: **5 passed**.
- Darkmatter `just test entry_point_parity`: **23 passed**.
- Claudine `just test-cli glob`: **34 passed**, including shipped hint/completion tests and the chooser candidate-order test.
- `just check-tier-coverage biscuit-file`, `darkmatter`, and `claudine`: no stranded tests.
- Additional temporary public API/composition probes reproduced both high findings, swept clean prefixes and permission controls, and exercised the input matrix. Shipped Claudine completion reproduced both directory-case shapes.

No feature was moved to `_completed`, no formatting command was run, and no commit was made.
