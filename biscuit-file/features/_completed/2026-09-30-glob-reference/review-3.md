---
$schema: feature-review.yaml
ready: false
findings:
  - title: Case-alias regression tests assume every macOS filesystem folds names
    priority: medium
  - title: Pattern readers lack the complete persistent input-shape matrix
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-02T21:48:42-07:00"
spec: "2026-09-30-glob-reference/spec.md"
implemented: true
implemented_by: claude/opus
log: biscuit-file/features/2026-09-30-glob-reference/log.md
description: "A **feature** review of `2026-09-30-glob-reference/spec.md`"
feature: "2026-09-30-glob-reference/review-3.md"
previous: "2026-09-30-glob-reference/review-2.md"
next: "2026-09-30-glob-reference/review-4.md"
---

This feature is **not production ready**. All three findings from review #2 are resolved on the reviewed fixtures. Two testing defects remain: fourteen regressions fail on a valid case-sensitive macOS filesystem, and the changed pattern readers do not have the complete persistent input matrix requested for this review. No new production behavior defect was reproduced. Both findings can be addressed without a design decision or human-only testing.

The review covers the specification, plan, implementation records, both earlier reviews, biscuit-file's public glob and recursive-resolution APIs, Darkmatter's glob consumers and schema readers, DMLS's schema projection, Claudine's candidate paths, documentation, and test registration. Existing working-tree changes were preserved. A temporary Rust example exercised public validators and effective-schema assembly; it was removed. A temporary case-sensitive filesystem was mounted without opening a window, used for existing nextest tests, detached, and removed. No production source was changed. The discovered filesystem-testing fact was recorded in the OS skill.

## Previous review verification

Review #2 lists three unblocked findings and no blocked findings. Its request for human inspection concerned the previous incomplete sweeps; it did not block implementation or reopen the specification's design. This review verifies the resulting fixes rather than carrying that external process check forward as a new design requirement.

| Previous finding | Verification | Status |
| --- | --- | --- |
| Absolute glob directory matching still follows filesystem aliases | Public API tests reject ASCII and Unicode directory aliases, including absolute exclusions and environment-expanded forms. Traversal-only ancestors, correctly spelled directories, directory symlinks, consumer results, trigger evaluation, shipped completion, and chooser candidate generation have regressions. | Implemented; the new tests' filesystem assumption is a separate finding below |
| Inaccessible schema roots are silently treated as absent | Both metadata policies now return errors. The five-root matrix covers readable roots, inaccessible ancestors, directly unreadable folders, and missing folders, with a same-named later schema proving that errors cannot change precedence. CLI and DMLS regressions pass. Claudine's additional prompt-discovery regressions also pass. | Implemented |
| The schema-variable documentation guard remains red | The guard passes after the skill explanation stopped spelling the forbidden singular token. | Implemented |

Review #1's literal-glob diagnostic finding remains resolved: the shared hint reaches the consumer error paths, and the glob-filtered CLI regression runs pass. The other review #1 findings are covered by the review #2 verification above.

## Unblocked Findings

### Medium — Case-alias regression tests assume every macOS filesystem folds names

**Defect class:** Regression fixtures infer a filesystem capability from the operating system and fail during setup when that capability is absent, before testing the public behavior they claim to verify.

Five helpers assert that a differently spelled directory must open whenever `cfg!(target_os = "macos")` is true. This is an assumption about the default development volume, not a macOS contract. On a case-sensitive volume, `DOCS` correctly does not open `docs`, and the tests panic before calling the glob API, composing a document, evaluating a trigger, or invoking completion. Case-sensitive macOS filesystems are a supported execution environment; this is an actual failing regression, not a request for additional cross-OS evidence.

**Reproduction:** Keep the existing fixtures and tests unchanged. Create a temporary case-sensitive HFSX image, attach it without browsing, and change only `TMPDIR` for each test command:

```sh
hdiutil create -size 40m -fs HFSX -volname glob-review3-case -type UDIF /tmp/glob-review3-case.dmg
mkdir -p /tmp/glob-review3-case-mount
hdiutil attach -nobrowse -mountpoint /tmp/glob-review3-case-mount /tmp/glob-review3-case.dmg
```

From the corresponding package area, run `TMPDIR=/tmp/glob-review3-case-mount just test directory_case`, Darkmatter's `just test absolute_`, and Claudine's `just test-cli chooser_rejects_absolute_directory_case_aliases` and `just test-cli completion_file_match_directory_names`, each with that same `TMPDIR`. Detach and remove the temporary image afterward. The default-volume runs are the positive controls. The five biscuit-file ASCII/directory-symlink controls also pass on the case-sensitive volume, proving this finding concerns test setup rather than incorrect glob results.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| biscuit-file [alias_opens](../../lib/tests/l1/glob_reference/directory_case.rs:130) | Same stored `Σ`/`SS` fixtures on the case-sensitive volume | Four tests fail at line 142: absolute membership, interpolation, exclusions, and context-rooted prefixes | Run the ordinary match/mismatch checks; require alias-specific coverage only when the fixture filesystem actually supplies the alias |
| Darkmatter [assert_consumers_judge_directory_spelling](../../../darkmatter/lib/tests/l1/glob_consumers.rs:392) | ASCII, Unicode, and traversal-only fixture variants on that volume | Three tests fail at line 401 before `find_files`, `::file-links`, or schema matching runs | Verify consumer results on either filesystem; do not require the mismatched path to exist |
| Darkmatter [assert_path_triggers_judge_directory_spelling](../../../darkmatter/lib/tests/l1/schema_roots.rs:644) | Same three variants on that volume | Three tests fail at line 651 before trigger evaluation or effective-schema assembly | Verify the correctly spelled trigger applies and the mismatch does not; capability-probe additional alias coverage |
| Claudine [chooser_rejects_absolute_directory_case_aliases](../../../claudine/cli/src/completion/schema_completion/parity_tests.rs:44) | Existing chooser fixture on that volume | One test fails at line 52 on `DOCS` versus `docs` | Exercise the candidate producer on either filesystem |
| Claudine [assert_completion_judges_directory_spelling](../../../claudine/cli/tests/l1/compose_schema_cli.rs:1793) | ASCII, Unicode, and traversal-only shipped-completion variants on that volume | Three tests fail at line 1799 before invoking `__complete` | Exercise the shipped completion results on either filesystem |
| All five helpers | Original fixtures on the default case-insensitive volume | All fourteen affected tests pass | Clean control; retain this alias coverage |
| biscuit-file ASCII absolute/interpolated/context-rooted checks, directory-symlink check, and temporary-directory spelling check | Same case-sensitive-volume edit | Five tests pass through the public listing and membership APIs | Clean siblings; no change needed |

A source sweep across biscuit-file, Darkmatter, and Claudine found these five assertion sites and no additional matching helper. Every site was run with the same filesystem edit, including the independently registered shipped-completion tests.

**Required change:** Remove the OS-based capability assertion at all five sites. Keep correct-name and mismatch result assertions running on case-sensitive filesystems; do not simply return early from all consumer coverage. Probe whether the alias opens only for a test that specifically needs filesystem aliasing, and retain evidence for that capability on the default case-insensitive fixture volume. Update the adjacent comments that currently claim aliases must open on macOS. Verify the same affected tests with both temporary-directory configurations. No permanent disk-image fixture or new CI environment is required.

### Medium — Pattern readers lack the complete persistent input-shape matrix

**Defect class:** Changed readers have expected malformed-input behavior that is exercised only by disposable review probes, leaving that behavior without the persistent public-result regression matrix required by the review contract.

Darkmatter's [match_keyword_factory](../../../darkmatter/lib/src/markdown/schemas/file_match.rs:165) reads the raw JSON Schema `x-darkmatter-match` array. It now rejects nulls, wrong types, mixed arrays, empty arrays, and invalid patterns. The committed tests exercise valid keyword conversion and matching, but no matrix asserts those malformed shapes through either public validator builder. The YAML [path_field_input_matrix](../../../darkmatter/lib/tests/l1/schema_roots.rs:484) does cover the field-shape rows, but its “invalid content” row is an invalid glob string, not invalid YAML or a valid YAML document followed by another document. Both earlier reviews used temporary probes for these remaining rows; those probes were removed and are not regression coverage.

**Reproduction and complete field sweep:** Copy the committed YAML trigger fixture and the existing raw JSON keyword fixture shape. For YAML, change one `$path` shape or append one trailing fragment, then inspect the public discovery/effective-schema result. Its control contributes `from_trigger` to the effective schema. For JSON, create existing `docs/a.md` and `docs/a.txt`, change only the keyword's shape, and use both `ValidatorCache::validator_for` (request-aware validation) and `ValidatorCache::structural_validator_for` (absolute values without a request). The control pattern `**/docs/*.md` accepts the Markdown file and rejects the text file in both modes. The temporary probe produced the following results; no permissive parsing defect was found.

| Shape | YAML trigger `$path` | JSON keyword, request-aware validator | JSON keyword, structural validator |
| --- | --- | --- | --- |
| Control | Contributes `from_trigger` | Markdown passes; text fails | Markdown passes; text fails |
| Absent | Empty match arm is a vacuous-trigger error | Both files pass; optional keyword adds no constraint | Same |
| Explicit null | Load error naming null; existing matrix also covers an empty YAML scalar | Validator-build error requiring an array of strings | Same |
| Wrong type, whole field | Load error naming number | Validator-build error | Same |
| Wrong type, one element | Load error; valid string is not retained as a partial list | Validator-build error; no element discarded | Same |
| Wrong type, every element | Load error | Validator-build error | Same |
| Empty collection | Error requiring at least one pattern | Error requiring a positive pattern | Same |
| Duplicate field key | YAML duplicate-key error | Not representable in the typed JSON value received by this reader | Same boundary |
| Valid document plus another document | YAML load error | Text syntax belongs to the upstream JSON loader, not this typed keyword reader | Same boundary |
| Invalid trailing document content | YAML load error | Same upstream text boundary | Same boundary |
| Invalid glob string | Pattern-definition error | Validator-build error naming invalid glob | Same |

| Reader/projection site | Shape tested | Observed result and permanent coverage | Expected result |
| --- | --- | --- | --- |
| YAML trigger discovery and effective-schema assembly | Every matrix row above | Behavior is correct. Persistent field-shape/control rows exist; the two trailing-source rows do not | Preserve existing behavior and add both source-syntax rows to the existing matrix |
| Raw JSON keyword through request-aware validation | Every representable field row above | Behavior is correct; no persistent malformed-field matrix | One registered Level 1 matrix with the positive/negative file control |
| Same keyword through structural validation | Same edits, with absolute existing values | Same correct behavior; no persistent malformed-field matrix | Exercise this public projection in the same matrix |
| Simplified `file(match(...))` grammar/conversion and caller-origin projection | Existing invalid-pattern, union, and entry-point tests; source inspection of the shared keyword factory | Pattern-definition and value-origin regressions exist; no separate array-field parser | Clean siblings; share the keyword matrix rather than duplicate its parser tests |
| Captured schema-root environment/home inputs | Existing `schemas_dir_and_home_input_matrix` | Absent, empty, relative, valid, duplicate-root, and snapshot-isolation cases pass; inputs are typed strings/paths | Clean sibling; null/array/document-syntax shapes are not values at this boundary |

**Required change:** Add the JSON field matrix through both public validator builders, and extend the existing YAML matrix with a second document and invalid trailing YAML. Document the intended outcomes beside those matrices, including the typed JSON boundary, rather than inventing duplicate-key semantics in the keyword factory. Register any new test module in the consolidated `l1` target; these tests need no terminal resource or feature beyond the existing area test configuration. The control must prove the field changes a public verdict or effective schema. No production parser change is currently indicated.

## Blocked Findings

None. Both findings are straightforward test changes.

## Recurrence

`recurrence: false`. Neither current finding repeats a defect class formally reported in review #1 or #2. Those findings concerned production directory matching, erased filesystem errors, missing diagnostic hints, and a forbidden documentation token. The first current finding concerns new test setup that assumes filesystem capabilities. The second concerns missing persistent malformed-input coverage; the earlier input audits observed correct behavior and did not report that coverage gap as a finding. No earlier production finding remains open on its reproduced shapes.

## Requirement and test-level audit

Every behavior added here concerns file sets, validation verdicts, schema selection, diagnostics, or completion candidate values. Level 1 public API and hermetic binary tests are appropriate. The feature does not add keyboard handling, modifier visibility, scrolling, terminal styling, mouse input, or a new interactive rendering requirement. Existing chooser candidate generation can therefore be tested in-process; no Level 2/3 mismatch was found.

The criterion numbers below refer to the specification's numbered acceptance list.

| User-facing requirement | Criteria | Verification inspected | Assessment |
| --- | --- | --- | --- |
| Prefix-aware completion from root/nested launch directories; exclusions; offered values validate | 1, 3–6, 12, 18 | Public candidate/validation tests and shipped `__complete`, Level 1 | Present |
| One glob implementation and prepared-context boundaries | 2, 14 | Source guards, public boundary tests, and composed-result tests, Level 1 | Present |
| Caller/document value origins and definition diagnostics | 7–8 | Grammar, caller-origin, composition, and entry-point tests, Level 1 | Present; malformed raw JSON field coverage is the second finding |
| Full merged sets, filename-view rules, and consumer-specific filtering | 9–11, 13 | Public glob and composed-result tests, Level 1 | Present |
| Root/depth/component order and first-match behavior | 15–17 | Order, unreadable-root, completion, and chooser candidate tests, Level 1 | Present |
| Literal brackets and recursive local-first selection | 19, 23 | Public single-file/glob tests, Level 1 | Present |
| Entry points agree; candidates resolve back to offered files | 20, 24 | Shared parity matrix and candidate resolve-back checks, Level 1 | Present |
| Trigger matching and context/snapshot cache separation | 21–22 | Public trigger/effective-schema and DMLS tests, Level 1 | Present; trailing YAML coverage is the second finding |
| Escaping file symlinks omitted with the required warnings | 25 | Public listing, composition, and completion tests, Level 1 | Present |
| Lexical missing-path matching and case-sensitive glob text | 26 | Public lexical/case tests and consumer/trigger/completion regressions, Level 1 | Implementation checks pass; fourteen regressions have the filesystem portability defect above |
| Five roots, shadowing, captured environment/home, and CLI/server agreement | 27–32 | Root matrix, CLI, DMLS, and inaccessible-root tests, Level 1 | Present |
| Shipped sibling schema example and supported variable spelling | 33–34 | Existing CLI example test inspected; documentation guard executed, Level 1 | Present |

The affected tests are compiled by declared consolidated targets and selected by Level 1 names. Claudine's fixture feature is enabled by its local recipe and CI metadata. `just check-tier-coverage` reports no stranded tests for biscuit-file, Darkmatter, or Claudine. The documentation guard spells repository reads in a recognized form. The findings concern actual test behavior and missing cases, not uncompiled test files.

## Verification performed

- biscuit-file `just test`: **1,065 passed**, zero skipped. The recipe's separate minimal-feature check also passed six tests.
- Darkmatter `just test schema_roots`: **26 passed**, including the field matrix and repaired documentation guard.
- Darkmatter `just test glob`: **84 passed**.
- Darkmatter `just test inaccessible`: **7 passed**, including all five root kinds, the CLI commands, and DMLS.
- Darkmatter `just test entry_point_parity`: **23 passed**.
- Claudine `just test-cli glob`: **34 passed**.
- Claudine `just test-cli completion_file_match_directory_names`: **3 passed** on the default volume.
- Claudine chooser case-alias regression: **1 passed** on the default volume.
- Claudine `just test inaccessible`: **2 passed**, covering prompt-discovery siblings.
- Case-sensitive-volume runs: biscuit-file directory-case tests **5 passed, 4 failed**; Darkmatter absolute-filtered tests **22 passed, 6 failed**; Claudine chooser **1 failed**; shipped-completion directory tests **3 failed**. All fourteen failures identify the same OS-based setup assumption.
- Tier checks for all three areas: **zero stranded tests**.
- Temporary public-result probes: both JSON validator projections and YAML discovery/effective-schema matrix produced the results tabulated above.

The implementation log separately reports two failing Darkmatter current-root documentation guards caused by an unrelated Claudine skill-page split. This review did not independently rerun those guards and does not treat their reported failure as a glob-feature defect. A whole-tree whitespace check also reports an existing extra blank line at the end of `darkmatter/lib/src/markdown/schemas/errors.rs`; it was not changed here. No full-workspace green result is claimed.

No formatting command, commit, publication, or lifecycle-directory move was performed.
