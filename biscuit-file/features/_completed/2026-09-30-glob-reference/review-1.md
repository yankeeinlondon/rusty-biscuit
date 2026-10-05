---
$schema: feature-review.yaml
ready: false
findings:
  - title: Absolute glob roots bypass case-sensitive directory matching
    priority: high
  - title: Glob traversal silently discards filesystem errors
    priority: high
  - title: Literal-glob hints never reach user-facing failures
    priority: medium
  - title: The schema-variable documentation guard fails on the new skill page
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-02T17:56:06-07:00"
spec: "2026-09-30-glob-reference/spec.md"
implemented: true
description: "A **feature** review of `2026-09-30-glob-reference/spec.md`"
feature: "2026-09-30-glob-reference/review-1.md"
next: "2026-09-30-glob-reference/review-2.md"
---

This feature is **not production ready**. The shared glob implementation and consumer migration are present, but two reproduced correctness defects and two incomplete requirements remain. None needs a new design decision or human-only testing.

The review covers the specification and implementation log, biscuit-file's glob parser, root preparation, listing and membership APIs, recursive file resolution, Darkmatter's expression and directive consumers, schema matching and roots, trigger discovery, DMLS schema caching, Claudine completion, and the declared test targets. Existing working-tree changes were preserved. Temporary Rust examples exercised public APIs against isolated filesystem fixtures; CLI probes exercised the built binaries. Those examples were removed after collecting evidence. No implementation was changed.

## Findings

### High — Absolute glob roots bypass case-sensitive directory matching

**Defect class:** Moving authored literal directory components into an absolute search root removes them from the case-sensitive matcher, allowing filesystem case rules to determine membership.

In biscuit-file, [prepare](../../lib/src/file_reference/glob/roots.rs) prepares the directories and matcher used by `GlobReference`. Its absolute branch turns the complete literal directory prefix into a root and compiles only the remaining glob. Canonicalizing that root equates `DOCS` with `docs` on a case-insensitive filesystem. The relative branch retains the directory names in the matcher and correctly rejects the same mismatch. This violates the specification's case-sensitive-on-every-OS contract and makes schema activation and completion depend on the host filesystem.

**Reproduction:** Create only `{repo}/docs/a.md` in a canonical temporary repository context. Change the pattern's directory spelling to uppercase. On the reviewed macOS filesystem, `{repo}/DOCS/*.md` lists the file and matches the actual lowercase path. The correct-case pattern is the positive control. Supply `ROOT={repo}` through the captured context to test the environment-expanded absolute form; do not modify the process environment.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| biscuit-file `list_files`, `take_first`, `matches`, `lists_file` | Absolute `{repo}/DOCS/*.md` | List/first contain `DOCS/a.md`; both membership APIs return true for `docs/a.md` | Empty list, no first match, false membership |
| Same four APIs | `{{ROOT}}/DOCS/*.md`, with an absolute `ROOT` | Same incorrect acceptance | Reject the mismatched authored `DOCS` component |
| Same four APIs; Darkmatter `FileMatchGlobs` | Bare, `./`, `&`, `^`, `@`, `~/`, and `vault:` forms with `DOCS/*.md` | Empty lists and false membership | Reject; clean sibling controls |
| Same four APIs | `docs/*.MD` against `a.md` | Empty list and false membership | Reject; filename-case control is clean |
| Darkmatter [find_files_fn](../../../darkmatter/lib/src/markdown/compose/expression/functions/mod.rs), through composition | Absolute and environment-expanded mismatched patterns | Frontmatter contains the lowercase-directory file, spelled with `DOCS` | Empty array |
| Darkmatter [glob directive discovery](../../../darkmatter/lib/src/markdown/compose/file_links/discovery.rs), through composition | Absolute mismatched pattern | `::file-links` includes `a.md` | No matching files |
| Darkmatter [FileMatchGlobs](../../../darkmatter/lib/src/markdown/schemas/file_match.rs), the shared completion/validation matcher | Absolute mismatched pattern against the existing lowercase path | Admitted | Rejected |
| Darkmatter [PathGlobs](../../../darkmatter/lib/src/markdown/schemas/triggers/grammar.rs), through `scan` and the public trigger trace shared by `md` and DMLS | `$path: '{repo}/DOCS/*.md'` | Trigger reports `matched: true` for `docs/a.md` | Trigger does not apply |
| Same trigger reader | Bare, `./`, `&`, `^`, and home mismatches; `@`, vault, and environment forms | Allowed forms do not match; forbidden forms produce definition errors | Clean |
| Claudine shipped `__complete`, using [the shared candidate walk](../../../claudine/cli/src/completion/schema_completion/candidates.rs) | `spec: file(match({repo}/DOCS/*.md))` | Offers `spec='DOCS/a.md'`; bare `DOCS/*.md` offers nothing; correct-case `docs/*.md` offers the file | Mismatched forms both offer nothing |

The TAB and chooser paths both call the same `match_glob_files` function and `FileMatchGlobs::lists_file`; they have no separate case policy. The chooser's existing native-order test passes, but does not include this fixture. Likewise, `matches_without_context` delegates absolute patterns to the same preparation and membership code.

**Required change:** Preserve case-sensitive checks for authored literal directory components when preparing absolute patterns, including patterns made absolute by interpolation. Keep canonical identity for containment and directory aliases. Add the fixture to the public API tests and consumer parity tests. The existing [matching_is_case_sensitive](../../lib/tests/l1/glob_reference/literal.rs) test checks only the filename extension, so it cannot detect this defect.

### High — Glob traversal silently discards filesystem errors

**Defect class:** A filesystem traversal converts failed directory reads into omitted entries, presenting incomplete searches as successful results.

In biscuit-file, [PreparedSet::walk](../../lib/src/file_reference/glob/list.rs) uses `filter_map(Result::ok)` on `WalkDir`. This drops errors even when the search directory itself exists but cannot be enumerated. Canonicalizing a directory does not prove that its entries can be read. Consequently, the public API's documented error contract and the specification's diagnostic policy are not fulfilled.

**Reproduction:** Create `docs/a.md` and `docs/locked/secret.md`; remove all permissions from `docs/locked`. First verify that `read_dir(docs/locked)` returns `PermissionDenied`. Run the same glob through each listing consumer, then restore permissions before fixture teardown. The reviewed host is not privileged and the permission denial was observed directly.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| biscuit-file `list_files` | `docs/**/*.md` with unreadable child directory | Successful partial list containing only the readable files | Typed I/O error identifying the unreadable directory |
| biscuit-file `list_files` and `take_first` | `docs/locked/*.md` | Successful empty list and `Ok(None)` | Typed I/O error, rather than no match |
| biscuit-file `list_files` | `&docs/locked/*.md`, `^docs/locked/*.md`, and absolute `{repo}/docs/locked/*.md` | Successful empty lists | Same I/O error for every listing prefix |
| biscuit-file recursive `FileReference` resolution | `%secret.md` | `Ok(None)` | Preserve the traversal failure through the recursive resolver |
| Darkmatter `find_files()`, through composition | Broad and direct locked-directory patterns | Partial array or `[]`, with no warnings | Expression failure carrying the typed glob I/O cause |
| Darkmatter `::file-links`, through composition | Broad and direct locked-directory patterns | Partial tree without a failure, or a generic no-matching-files warning | Directive failure identifying the unreadable directory |
| biscuit-file root preparation | Make the parent `docs` unreadable, then use `docs/locked/*.md` | Typed I/O error during containment/root preparation | Clean: this earlier error path already preserves the cause |
| biscuit-file `list_files` | Actually missing `missing/*.md` | Successful empty list | Clean: a missing search directory remains an ordinary empty result |

Claudine's suggestion walker also flattens traversal errors, and trigger discovery flattens individual `read_dir` entry errors. Those sibling implementations were inspected: completion is a filtered suggestion operation, while schema discovery's initial `read_dir` failure is already an error. They do not excuse returning a successful complete glob listing after a failed walk. Lexical `matches` is not a listing operation and need not read the directory's children.

**Required change:** Handle `WalkDir` errors explicitly and return `GlobReferenceError::Io` with the failing path. Preserve the intentional empty result for a missing literal search directory. Add public-result regression tests for an unreadable walk root and an unreadable descendant, then verify the expression, directive, and recursive-resolution projections. Existing permission tests exercise failures during canonicalization, not failed enumeration after canonicalization succeeds.

### Medium — Literal-glob hints never reach user-facing failures

**Defect class:** A useful no-match diagnostic is exposed as an optional accessor but discarded or ignored by every production projection that reports the failure.

In biscuit-file, [DetailedResolution::glob_hint](../../lib/src/file_reference/mod.rs) supplies the required explanation that wildcard characters are literal in a single-file reference and suggests `::file-links`. However, a repository search finds no production call to that accessor. The convenience resolver discards the detailed outcome, and even Claudine's reader that keeps the detailed result never uses the hint. The implementation log records the accessor choice, but that alone does not fulfill the specification's promise that the error suggests a glob-accepting form.

**Reproduction:** Keep `docs/a.md` as the positive control and replace the single-file reference with the literal string `docs/*.md`. Pass that string as one argument, without shell expansion. Repeat through the following shared readers and their normal outputs.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| biscuit-file detailed resolver | Literal-glob miss | `glob_hint()` returns the intended explanation | Clean producer |
| biscuit-file convenience resolver | Same miss | `Ok(None)`; diagnostic explanation is discarded | Consumers reporting this miss must retain or derive the hint |
| Darkmatter [file argument reader](../../../darkmatter/cli/src/io/mod.rs), shared by the `md` file-taking routes | Shipped `md compose 'docs/*.md'` | File-argument error with `failure: no-match`, without a glob suggestion | Include the literal-character explanation and a glob-accepting form |
| Darkmatter [transclusion file reader](../../../darkmatter/lib/src/markdown/compose/transclusion/resolver.rs) | `::file docs/*.md` and `::code docs/*.md` | File-not-found errors, without the hint | Same hint |
| Darkmatter [TOC-linking reader](../../../darkmatter/lib/src/markdown/compose/toc_linking/mod.rs) | `::toc-linking docs/*.md` | Unresolved-reference warning and fallback text, without the hint | Same hint in the warning |
| Darkmatter [expression file reader](../../../darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs), shared by read-side document functions | `markdown_title('docs/*.md')` | Invalid-file-path expression error, without the hint | Same hint in the expression failure |
| Darkmatter schema file-value validation and [schema-reference reader](../../../darkmatter/lib/src/markdown/schemas/resolve.rs) | Eager `file` value and `$schema: docs/*.md` | Validation/schema-preparation failures, without the hint | Same hint in the relevant file-reference diagnostic |
| Claudine [composition source reader](../../../claudine/lib/src/composition/resolve.rs) | Shipped `claudine compose --dry-run 'docs/*.md'` | Detailed no-match error listing attempted paths, without the hint | Same hint |

Other single-file projections, including schema imports/examples, DMLS diagnostics, and proxy source handling, cannot currently display this accessor's message either: there are no production calls anywhere in the repository. They should receive the fix through the shared diagnostic producers rather than acquire separate wildcard heuristics. The tests currently prove the accessor, not the messages shown to users.

**Required change:** Carry the hint into shared no-match diagnostics and their warning/error renderers. Add public API and CLI assertions that a literal-glob miss displays the guidance, while an ordinary missing filename and non-no-match failures do not acquire it. Keep single-file resolution literal.

### Medium — The schema-variable documentation guard fails on the new skill page

**Defect class:** Newly authored documentation violates a source-scanning acceptance test, leaving the feature's normal test gate red.

The Darkmatter [testing skill page](../../../.claude/skills/darkmatter/testing.md) includes the singular token `SCHEMA_DIR` while explaining the guard. The explanation correctly describes it as forbidden; it is not an instruction to configure that variable. Nevertheless, the specification requires no such token in docs or skills, and [no_doc_or_skill_names_the_singular_schema_variable](../../../darkmatter/lib/tests/l1/schema_roots.rs) rejects every whole-word occurrence, including this explanatory one.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Darkmatter skills tree | Whole-word singular token | One occurrence in `testing.md`; test reports this file | No occurrence, passing guard |
| Darkmatter docs tree | Same whole-word scan | No occurrences | Clean |
| Claudine docs and skills trees | Same scan | No occurrences | Clean |
| biscuit-file docs and skills trees | Same scan | No occurrences | Clean |

**Required change:** Rephrase the skill's explanation without spelling the prohibited token, then rerun the guard. No new behavior or test is needed. The failure was reproduced in both the initial focused run and the complete focused run with fail-fast disabled.

## Input robustness audit

The changed file-format field is a trigger schema's YAML `$path`; the raw JSON Schema match keyword also reads a pattern array. Environment schema roots read captured strings, so nulls, mixed arrays, duplicate keys, and trailing documents are not representable at that typed boundary. No new TOML or lockfile reader is involved.

For YAML, the public `scan` result and trigger trace were exercised with one replacement per cell of a valid trigger. A control trigger applied to `docs/a.md` and contributed its payload. The outcomes below include every required shape; the committed [path_field_input_matrix](../../../darkmatter/lib/tests/l1/schema_roots.rs) covers the field-shape cases, and review probes additionally checked trailing documents and invalid YAML.

| Shape | YAML `$path` result |
| --- | --- |
| Control: `^docs/**` | Trigger applies |
| Absent, leaving an empty match arm | Vacuous-trigger load error |
| Explicit null / empty YAML scalar | Load error naming null, distinct from absence |
| Wrong whole-field type | Load error naming the wrong type |
| One wrong array element | Load error; the valid element is not retained as a permissive partial pattern list |
| Every element wrong | Load error; not an empty list |
| Empty sequence | Load error requiring a positive pattern |
| Duplicate `$path` key | YAML duplicate-key load error |
| Additional YAML document / invalid trailing YAML | YAML load error |
| Invalid glob / forbidden prefix | Definition error naming the pattern |

The raw JSON Schema keyword's [factory](../../../darkmatter/lib/src/markdown/schemas/file_match.rs) was also exercised through the public context-bound validator, with both `docs/a.md` and `docs/a.txt` existing. The control array `["docs/*.md"]` accepts only the Markdown file; removing the keyword accepts both, proving the field changes the result.

| Shape | JSON Schema `x-darkmatter-match` result |
| --- | --- |
| Control array | Validator accepts the matching existing file and rejects the other existing file |
| Absent | Both files pass; this optional keyword adds no constraint |
| Explicit null | Validator-build error requiring an array of strings |
| Wrong whole-field type | Same validator-build error |
| One wrong array element | Validator-build error; no element is silently discarded |
| Every element wrong | Validator-build error; not an empty collection |
| Empty array | Validator-build error requiring a positive pattern |
| Duplicate key / trailing content | Not representable in the typed JSON value this factory receives; source-document syntax is handled by the existing upstream loader, not this changed keyword reader |

The YAML envelope preserves an explicit null payload as a YAML value rather than conflating it with absence. No permissive default or element-filtering defect was found in the changed pattern readers. The committed YAML matrix does not include the review's trailing-document cases; add them when expanding the regression fixtures.

## Requirement and test-level audit

All requirements here concern filesystem results, schema selection, diagnostics, or candidate lists. Level 1 public API and binary-invocation tests are the appropriate verification level. This feature changes no terminal keyboard encoding, hotkeys, mouse handling, scrolling, or styling requirement; Level 2/3 tests are not required merely because a candidate list can appear in an existing chooser.

The criterion numbers below refer to the numbered acceptance list in the specification. Each row states the behavior rather than relying on the number alone.

| User-facing requirement | Criteria | Strongest applicable tests inspected | Assessment |
| --- | --- | --- | --- |
| Prefix-aware completion from root and nested launch directories | 1, 3 | Claudine completion API tests and shipped `__complete` tests, Level 1 | Present |
| One glob implementation and no ambient resolution fallback | 2, 14 | Glob implementation guard and context-construction guards, Level 1 | Present |
| Exclusions, nearest-root ownership, and offered values admitted by validation | 4–6, 12, 18 | Public glob API and Claudine candidate/validation tests, Level 1 | Present |
| Caller values use launch context; document values use document context | 7 | Composition and entry-point parity tests, Level 1 | Present |
| Bad patterns produce definition errors; vault patterns are usable | 8 | Grammar/consumer and completion tests, Level 1 | Present |
| Existing outputs, merged roots, controlled filename widening, and consumer filters | 9–11, 13 | Glob consumer, file-links, frontmatter-expression, and candidate tests, Level 1 | Present |
| Relative boundaries, external roots, and directory-symlink handling | 14 | Public glob boundary and composition tests, Level 1 | Present |
| Native root/depth/component order; first match; completion order | 15–17 | Public order tests and chooser candidate parity, Level 1 | Present |
| Literal brackets and local-first recursive search | 19, 23 | Public `FileReference`/`GlobReference` tests, Level 1 | Present |
| Library, CLI, DMLS, completion, and chooser agree | 20 | Shared entry-point matrix and chooser candidate runner, Level 1 | Present for the matrix's fixtures; missing new defect fixtures |
| Trigger prefixes, definition errors, and shared matching verdicts | 21 | Trigger scan/trace and `md`/DMLS parity, Level 1 | Present; absolute directory-case fixture missing |
| Schema cache distinguishes context and snapshot | 22 | DMLS package-root and snapshot-environment cache tests, Level 1 | Present |
| Inserted candidate spellings resolve to the offered files | 24 | Claudine portable-candidate tests with resolve-back assertions, Level 1 | Present |
| Escaping file symlinks are omitted with the required warning policy | 25 | Public glob and composed-result tests; completion test, Level 1 | Present |
| Missing typed paths match lexically and all glob text is case-sensitive | 26 | Public lexical/extension-case tests, Level 1 | Directory-case coverage gap and defect in first finding |
| Five schema roots, shadowing, captured environment, home roots, and no ancestor walk | 27–32 | Root-reader, CLI, and DMLS parity tests, Level 1 | Present |
| Shipped example uses its explicit sibling schema | 33 | CLI shipped-example validation test, Level 1 | Present |
| Docs and skills use only the supported schema-variable spelling | 34 | Documentation guard, Level 1 | Fails as described above |
| Literal-glob misses explain how to request a set of files | Expected Behavior: “Hint on a literal miss” | Accessor unit test, Level 1 | User-facing message assertions missing |
| Failed listing is distinguishable from an empty or complete listing | Expected Behavior: “Diagnostics, never silence” and public API contract | Root-preparation permission tests, Level 1 | Enumeration failure coverage missing |

Consolidated test files are declared by their `tests/l1/main.rs` modules and Cargo test targets. The shared parity fixture is declared as a source input by its consumers. `just check-tier-coverage biscuit-file darkmatter claudine` reported **zero stranded tests**. No cross-OS evidence gap was used to decide readiness.

## Verification performed

- `biscuit-file`: `just test glob` — 26 selected nextest tests passed; the recipe's six minimal-feature path-text checks also passed.
- `darkmatter`: focused `just test --no-fail-fast` across glob consumers, schema roots, file matching, implementation guards, and entry-point parity — 59 passed, one failed: the documented singular-variable guard.
- `claudine`: focused `just test --no-fail-fast` across schema completion and entry-point parity — 48 passed, including the chooser candidate-order runner.
- Additional focused Darkmatter checks covered schema-trigger CLI behavior, DMLS context cache keys, frontmatter file-list expressions, and context-construction guards — 22 passed.
- Public API probes reproduced the directory-case and unreadable-directory defects, checked clean prefix/missing-directory controls, walked the YAML `$path` robustness matrix, and observed the missing literal-glob hints. Shipped Claudine completion and both CLI prompt-argument failures reproduced the relevant projections.

## Nonblocking performance observation

Claudine's candidate walk calls `FileMatchGlobs::lists_file` for each file. That delegates to `GlobReference::prepare_available`, which interpolates patterns, prepares roots, and recompiles matchers on every call. The ordinary listing prepares these once per operation. A context-bound prepared matcher could let filtered walks reuse that work. This review did not measure a completion latency regression, so this is a profiling and API-ergonomics suggestion rather than a readiness finding.

The findings are implementation issues with agent-executable fixes and checks. Human review is not a blocker for this iteration.
