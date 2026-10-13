---
spec: /Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
plan: tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
implemented_by: codex/gpt-6.1-sol
started_phase: 1
source_files_during_phase_1:
  - claudine/cli/src/commands/wrap/env/tests.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/ownership.rs
  - sniff/lib/src/filesystem/repo/go.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/tests/l1/main.rs
  - sniff/lib/tests/l1/package_identity.rs
  - sniff/lib/tests/l1/lockfile_provenance.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/l1/cli.rs
docs_updated_during_phase_1:
  - sniff/docs/sniff-library-architecture.md
  - sniff/lib/README.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
docs_created_during_phase_1:
  - tree-hugger/features/2026-10-10-stats-package-grouping/implementation-log.md
skills_files_updated_during_phase_1:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/os/macos.md
source_files_during_phase_2:
  - tree-hugger/cli/Cargo.toml
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/grouping.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/declarations_grouping_tests.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/grouping_core.rs
docs_updated_during_phase_2:
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/dependencies.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/implementation-log.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_3:
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/src/grouping.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/declarations_grouping_tests.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/grouping_core.rs
  - tree-hugger/cli/tests/symbols/grouping_reports.rs
  - tree-hugger/cli/tests/symbols/golden.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.stats
  - tree-hugger/cli/tests/fixtures/golden/symbols-stats.json
docs_updated_during_phase_3:
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/implementation-log.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
docs_created_during_phase_3:
  - tree-hugger/docs/release-notes.md
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_4:
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/grouping_acceptance.rs
docs_updated_during_phase_4:
  - tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/implementation-log.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
packages:
  - sniff
  - sniff-cli
  - claudine-cli
  - tree-hugger-cli
source_files_during_phase_5:
  - tree-hugger/cli/src/declarations.rs
docs_updated_during_phase_5:
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/implementation-log.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
source_code:
  - claudine/cli/src/commands/wrap/env/tests.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/ownership.rs
  - sniff/lib/src/filesystem/repo/go.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/tests/l1/main.rs
  - sniff/lib/tests/l1/package_identity.rs
  - sniff/lib/tests/l1/lockfile_provenance.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/l1/cli.rs
  - tree-hugger/cli/Cargo.toml
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/grouping.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/declarations_grouping_tests.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/grouping_core.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/tests/symbols/grouping_reports.rs
  - tree-hugger/cli/tests/symbols/golden.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.stats
  - tree-hugger/cli/tests/fixtures/golden/symbols-stats.json
  - tree-hugger/cli/tests/symbols/grouping_acceptance.rs
documentation:
  - sniff/docs/sniff-library-architecture.md
  - sniff/lib/README.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/plan.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/spec.md
  - tree-hugger/features/2026-10-10-stats-package-grouping/implementation-log.md
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/os/macos.md
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/dependencies.md
  - tree-hugger/docs/release-notes.md
completed_phase: 5
implemented: true
---

# Implementation Log for 2026-10-10-stats-package-grouping (5 phases)

## Phase 1

### Test design before implementation

New library coverage belongs to the declared sniff L1 integration target,
registered in `tests/l1/main.rs`; the CLI case uses its existing L1 `cli` module. Hermetic filesystem discovery crosses the public API
boundary without needing a terminal. There is no existing failing stats input:
this phase adds API contracts, not grouping output.

| Requirement | Concrete test (module `package_identity`) |
| --- | --- |
| Manifest-backed identity versus fallback, each name shape | `cargo_name_matrix`, `npm_name_matrix`, `python_name_matrix`, `go_name_matrix`: discover a fixture after one edit per row; assert source, name, owner, and serialized catalog |
| Old serialized catalogs remain readable; new source survives persistence | `catalog_round_trip`: missing field defaults conservatively, read/write/read twice preserves identity |
| Deepest owner, prefix siblings, root, outside path, unnamed nested boundary | `ownership_boundaries`: public lookup and existing single-path API agree |
| Canonical and symlink identity | `ownership_symlink_alias` (Unix only) |
| Shipped manifest readers remain compatible | `shipped_manifest_corpus`: actual shipped Cargo manifests through ordinary structure discovery |
| Normal invocation with a shipped artifact | The same corpus calls `detect_repo_structure_or_root_package` with copied shipped manifests, exercising discovery, identity, lookup, and serialization |

Input matrix outcomes: absent, null, wrong scalar/container/element types,
empty/whitespace, duplicates and trailing garbage produce PathFallback;
positive written names produce Manifest. TOML null is unrepresentable and is
covered as invalid syntax. Go names are directive tokens (numeric tokens are
strings, not JSON numbers); array/object/null shapes are invalid module paths.
Cargo, npm, Python project and Go module names are read by the shared name
resolver. Standalone Composer identity is read by root synthesis (covered by
`composer_name_matrix`); requirements.txt remains fallback.

### Session constraints

Pre-existing staged/unstaged edits are present (Git, changed-file reporting,
docs and skills); preserve them without staging or committing. Never run
cargo fmt. Later-phase checkboxes remain untouched.

### Provisional rulings

Use all ten recommended Necessary Rules from the plan. Automatic grouping and
name-source policy still require author confirmation before Phase 2, as the
plan explicitly says; no author answers were supplied in this non-interactive
session. Phase 1 implements the provisional API only. Native-path ownership
reuses the existing index. `GitRepo::discover(...).repo_root()` already supplies
the active worktree root; no git2 production dependency is needed. sniff default
features are empty; platform dependencies are target-gated. No performance
benchmark or new CI cells are added.

### S1 structure-only spike (macOS)

Ran a temporary Rust example calling the public API once per temporary fixture,
then removed the example. Initial scratch example had quoting errors; corrected
it before collecting results. No spike source remains.

| Case | Packages `(name, relative, path relative to root)` | Inferred fallback |
| --- | --- | --- |
| Cargo workspace with root package and explicit member | `(child, pkg, pkg)` | none; root is not in this explicit member catalog |
| Virtual Cargo workspace | `(child, pkg, pkg)` | none |
| Standalone Cargo | `(standalone, "", "")` | none |
| Standalone package.json | `(standalone, "", "")` | none |
| Manifest-less directory | `None` | no catalog |
| Named root, unnamed nested Cargo member | `(pkg, pkg, pkg)` | nested name is relative-path fallback |

Absolute package paths are under the temporary root. Root `relative` is empty,
not `.`; Phase 2 must convert the empty relative output label to `.` without
changing sniff's existing path contract. The index honors exactly the discovered
catalog; it must not invent a root package excluded by workspace membership.

### API implementation and targeted verification

- Added `PackageNameSource::{Manifest, PathFallback}` (snake_case wire values).
  Only fallback is skipped in serialized output. This preserves legacy fallback
  payloads; named packages gain the intentional additive `name_source` field.
  Missing provenance in old catalogs is conservatively fallback. Updated
  exhaustive constructors; constructors using `..Default` remain conservative.
- Added public `filesystem::repo::PackageOwnershipLookup`, borrowing a `RepoInfo`
  and building the existing index once. Lookup returns `&Package`, rejects
  relative/outside-root paths, and handles missing descendants through their
  existing ancestor. The original single-directory API retains its behavior.
- Added bounded Go name tokenization because the original reader accepted an
  empty module, quotes as literal name characters, duplicates and garbage.
  Supports bare/raw/interpreted tokens, tabs/comments, byte/octal/Unicode escapes,
  known directives and balanced blocks. No dependency-version validation or
  new dependency is introduced.
- First targeted compile failed for absent APIs, proving test registration.
  After implementation, targeted tests exposed malformed-root errors and the
  existing single-directory API's missing-file alias behavior. Updated test
  framing to known workspace members and existing files when comparing the
  legacy API. Root errors remain errors; do not turn them into Unassigned.
- `just test package_identity`: initially 8 public-library tests passed while
  the CLI assertion used the wrong output contract (`repo packages`). Corrected
  to `repo structure --json`, then all 10 tests passed, including real shipped
  manifest end-to-end discovery and warm repeated process invocation after edit.
  Two additional ownership cases and expanded Go variants were added afterward.
- Initial `just check-tier-coverage sniff` could not list tests because an
  exhaustive CLI constructor was missing the new field. Corrected constructors;
  rerun required before handoff. No tier-marked tests or new targets were added.

### Input robustness matrix through public discovery

Each format's test starts with a real-tool shipped fixture and edits one name
cell per case. `F` = PathFallback, `M` = Manifest, `E` = returned discovery error.
The columns describe member identity in a valid workspace, not malformed root
workspace declarations. Full discovery retains its existing root-error policy.

| Shape | Cargo `[package].name` | npm `name` | Python `[project].name` | Go `module` token | standalone Composer `name` | JSON catalog `name_source` |
| --- | --- | --- | --- | --- | --- | --- |
| control | M | M | M | M | M | M |
| absent | F | F | F | F | F | F (legacy) |
| explicit null | F (invalid TOML) | F | F (invalid TOML) | no null type: bare `null` is M | F | E |
| wrong whole field | F (number) | F (number) | F (number) | F (object/list syntax); bare `123` is M | F | E |
| one wrong element | F (mixed name/container array) | F (mixed array) | F (mixed name/container array) | F (multiple tokens/list) | F | E |
| every wrong element | F | F | F | F | F | E |
| empty | F (string/array/table) | F (string/array/object) | F (string/array/table) | F (blank/quoted/list/object) | F | E |
| duplicate | F (TOML rejects) | last-wins, M or F according to final value | F (TOML rejects) | F | last-wins M | E |
| trailing/invalid | F | F | F | F (unknown directive, unclosed block/string, invalid escape) | F | E |
| representation variants | double/single quoted M | Unicode escape M | double/single quoted M | bare/raw/interpreted, tab, comment, hex/octal escapes M | native string M | `manifest` M; omitted fallback F |

Malformed Cargo/npm/Python root manifests return E, pinned separately by
`malformed_root_discovery_is_an_error`; this is a clarification of the plan's
blanket fallback wording, not a new global manifest-error rule. Group-presence
assertions are deferred to later phases because no hug grouping is built here.
`requirements.txt` is a boundary marker with F identity. Standalone Composer
has its own JSON name reader and follows the npm shape outcomes.

### Comment and documentation drift

Corrected `Package.name` docs that formerly claimed every label was native
manifest identity. Documented the new APIs, conservative legacy serialization,
root relative paths and bounded Go behavior in sniff's topic page and skill.
Tree-hugger's skill explicitly marks stats grouping planned. No dependency docs
change is needed: Phase 1 adds no crates or features.

### Additional boundary coverage and scope

- Added `ownership_alias_precedence`, `ownership_non_utf8_paths_remain_distinct`,
  and `manifest_name_priority_and_literal_values`. The non-UTF-8 test initially
  attempted to create invalid byte names: APFS rejected this with error 92.
  It now uses captured native catalog paths under a canonical temporary root,
  preserving the lookup assertion without unsupported on-disk filenames. Added
  this host fact to the OS skill alongside its pre-existing changes.
- `hug` already depends on sniff with `default-features = false` (changed-file
  selection), so Phase 2's dependency-add task should be treated as confirmation,
  not a second dependency edit. All-platform hardware dependencies remain
  target-gated. No timing benchmark is necessary or requested.
- Saved plan/log/spec frontmatter and ran the required root scope command:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`.
  It reported `packages: [sniff, sniff-cli, tree-hugger-cli, test-toolkit]`,
  `change_class: package`. The working tree contains other pre-existing changes,
  so this is the combined scope, not attribution to this phase. Gates cannot
  be skipped; run only the authorized sniff and tree-hugger area gates.
- Build hosts declared: BUILD_LINUX=build-linux, BUILD_WIN=build-win-native,
  BUILD_WSL=build-win. `just cross-check` creates a throwaway Git commit to ship
  the tree, so it was not invoked under the explicit no-commit constraint.
  Local evidence is macOS; other environments remain for the existing CI
  schedule. No new platform code or CI matrix cells were introduced.

### Composer root reader found during drift review

The shared resolver has four formats, but standalone root synthesis also
replaces the name using composer.json. Added `composer_name_matrix` before
changing that branch. Its shipped control failed with `PathFallback` versus
`Manifest`, then the branch was fixed to set provenance and reject blank names.
The test walks absent, null, wrong types and element shapes, empty strings,
duplicates and trailing garbage through the public catalog and ownership result.
Composer duplicates follow the existing JSON last-wins reader; malformed
standalone Composer identity falls back rather than returning an error. This
preserves its existing best-effort root behavior. It adds no new manifest read.

### Initial broad gate outcomes

- Tree-hugger `just lint` passed. Default-worker `just test` ran 1,695/1,908
  tests: 1,693 passed and two unchanged `cache_robustness` corpus tests timed out:
  `every_kind_evidence_identities_name_retained_declarations_in_every_language`
  and `retained_geometry_written_paths_and_candidate_order_are_checked_in_every_language`.
  The remaining 213 tests were not run because nextest stopped after failure.
- Sniff default-worker `just test` ran 1,024/3,169 selected tests: 1,012 passed,
  12 host probes timed out; 32 resource-tier tests excluded and 2,145 selected
  tests not run after failure. Timeouts: audio `detect_audio_devices_returns_vec`,
  `detect_audio_devices_on_macos`; hardware `test_detect_hardware_with_request_respects_flags`,
  `test_detect_hardware_full_includes_gpu_on_macos`; network
  `test_filtered_excludes_loopback`, `test_detect_network_returns_info`,
  `test_filtered_excludes_down_interfaces`, `test_filtered_ip_addresses_match_retained_interfaces`,
  `test_interface_has_name`, `test_ip_addresses_are_valid_ip_strings`,
  `test_ip_addresses_count_matches_per_interface_sum`,
  `test_ip_addresses_have_prefix_length_on_real_system`.
- These exact contention failures are already documented in the OS skill.
  Rerun full suites sequentially with four workers; keep all tests selected,
  without increasing timeouts or altering test tiers.

### Additional schema coverage

`catalog_round_trip` also walks persisted `name_source`: absent is the
explicit legacy PathFallback default; null, unknown or blank enum strings,
numeric/object/list values, mixed/all-invalid elements, duplicate fields and
trailing content are rejected. The Manifest and omitted-fallback controls
survive two disk read/write/read rounds and still resolve the same package.
There is no default on the manifest `name` field and no silent element filter.
Existing ManifestStore `.ok()` conversions are intentional best-effort member
identity reads, with separate required-root errors; Go byte/UTF-8 `.ok()`
conversions produce fallback on invalid tokens, as covered by the matrix.
No load-bearing parse error becomes a declared empty name.

### Additive serialization expectations

The first four-worker full sniff run passed 2,087 tests, including the formerly
timed-out probes, then failed three exact serialized-catalog expectations in
`lockfile_provenance`: `declining_requests_report_manifest_provenance_and_read_no_lockfile`,
`corroborating_requests_report_hand_written_provenance_for_every_lockfile_state`,
and `every_other_authority_reports_its_complete_repository_result`. The only
new field was `name_source: manifest`, which was deliberately added to the
hand-written expectations for the fixtures' declared Cargo/Node/Python/Go
names. Unknown ecosystem fallback fixtures retain their old payloads. The
run stopped with 1,081 selected tests unrun; these are phase-caused expectation
updates, not pre-existing failures. No golden data or lockfile rules changed.

### Final targeted result

`just test package_identity --test-threads 4` passed all 14 targeted tests.
Declared target `sniff::l1` runs these 13 tests without a feature or tier marker:
`cargo_name_matrix`, `python_name_matrix`, `npm_name_matrix`, `go_name_matrix`,
`composer_name_matrix`, `shipped_manifest_corpus`, `catalog_round_trip`,
`ownership_boundaries`, `ownership_symlink_alias`, `malformed_root_discovery_is_an_error`,
`ownership_alias_precedence`, `ownership_non_utf8_paths_remain_distinct`,
`manifest_name_priority_and_literal_values`. The declared `sniff-cli::l1`
target runs `cli::package_identity_shipped_manifest_through_cli`. The two
native-byte/symlink tests are Unix-specific; the other cases build native paths
and run on every supported OS. The registration/layout tests compile both
modules, and no custom feature selection is needed. Partial focused runs skip
other tests by filter; those skips are not evidence for the broader gates.

The next bounded full sniff run passed 2,795 tests before one remaining exact
CLI catalog expectation failed in
`cli::repo_structure_json_reports_cargo_subset_evidence_for_matching_lockfile`.
Added only the intended `name_source: manifest` field to its hand-written Cargo
package expectation. The full run stopped with 375 tests unrun; rerun after
this final expectation update. No behavior assertion was weakened.

### Non-interactive ruling checkpoint

The plan's author-answer checkpoint was clarified to record provisional
recommended answers and the required author confirmation before Phase 2.
No author answers were fabricated. `human_review: true` and readable options
in the spec request the two policy decisions; Phase 1 remains implementable
under its explicitly provisional assumptions. The other eight Necessary Rules
follow the proposed shapes and failure mapping. The root path-label difference,
already-present sniff dependency, and root-manifest error distinction are
recorded above for the next implementer. Phase 2–5 and feature-wide completion
checkboxes remain unchecked.

### Downstream constructor compatibility

A repository-wide text check found four exhaustive `Package` literals in
`claudine/cli/src/commands/wrap/env/tests.rs`. Added `name_source: Default::default()`
to those existing fixtures, preserving their conservative legacy meaning and
all assertions. Other downstream literals use `..Default` already. Added this
mechanical compile adaptation to source/package frontmatter; no Claudine
behavior, test names, targets, features or docs changed. Used the Claudine skill
for its package conventions. Runtime gates remain scoped to the requested
areas; no workspace-wide tests are run.

### Completed sniff gates

- `just test --test-threads 4`: **3,171 passed**, 32 resource-tier tests excluded,
  no failures or unrun selected tests (127.215 seconds). This run includes all
  14 new tests, the corrected exact catalogs, registration/layout guards, and
  the host probes that timed out under the initial competing default-worker run.
- `just lint`: passed with remote enabled.
- `cargo clippy --color=never -p sniff -p sniff-cli --all-targets -- -D warnings`:
  passed, including test targets with default features.
- `just check-tier-coverage sniff`: passed, **0 stranded tests**.
- No L2/L3/browser/real tests were added or renamed; the 32 excluded tests retain
  their existing resource tiers. No windows or browser panes were opened.

### Completed tree-hugger tests and final scope

- `just test --test-threads 4`: **1,908 passed**, 0 skipped, no unrun selected
  tests (379.908 seconds). Both formerly timed-out cache-robustness corpus tests
  passed. The four-worker limit is a local scheduling choice, not a test skip,
  retry setting, timeout change or CI policy change.
- Re-ran the required root scope command after plan/log/spec and downstream
  constructor metadata were saved. It reported
  `packages: [claudine-cli, sniff, sniff-cli, tree-hugger-cli, test-toolkit]`,
  `change_class: package`. All required area gates were run; none were skipped.
  The combined working-tree scope includes pre-existing changes.
- There are no remaining unchecked Phase 1 todos. Later phases remain pending.
  Author confirmation of automatic grouping and declared-name policy is
  intentionally a Phase 2 prerequisite, recorded in spec `human_review_items`.
- Local verification is macOS only; no cross-OS claim is made. L2/L3/browser/real
  gates and Claudine runtime gates were not run: no changed behavior needs their
  resources, and the Claudine edit only supplies defaults in existing fixtures.
  Windows/WSL2/Linux evidence remains on the existing CI schedule.

### Completion record

Final tree-hugger `just lint` passed after the full suite. All Phase 1 APIs,
spike notes, matrices, targeted tests, required area test/lint gates, docs,
skill guidance, plan checkboxes, and frontmatter are saved. The exact source
and documentation inventory is in both plan and log frontmatter. No unresolved
local test or lint failure remains. Initial concurrency timeouts and additive
catalog expectation failures were resolved as recorded above.

Implementation complete, ready for review. No cargo fmt, staging, Git commit,
feature-lifecycle move, new dependency, network enrichment, or CI cell was used.
Cross-platform evidence and author policy confirmation remain explicitly
identified for subsequent work; no background command is left pending.


## Phase 2

### Scope and test design before implementation

The explicit request to implement Phase 2 authorizes the recommended automatic
grouping and manifest-only identity policy. No interactive policy answer is
required. Phase 3 still owns the report fields and terminal sections: this phase
produces sorted group counters internally while retaining the existing writers.
Pre-existing staged changes remain untouched; no formatting, staging or commits.

| Changed behavior | Concrete test and observable assertion |
| --- | --- |
| Stats captures one structure catalog; other modes capture none | `grouping::tests::catalog_requests_only_stats_once`: injected discovery count and captured root/catalog |
| Git root versus standalone scan root | `grouping::tests::catalog_roots_follow_active_worktree`: public Git discovery with nested invocation and linked worktree; standalone root |
| Deepest owner, unnamed nested boundary, prefix sibling, duplicate names | `grouping::tests::ownership_ordering_and_zero_groups`: real temporary manifests, ordered group identities and serialized counters |
| No catalog, outside root, empty selected list | `grouping::tests::absence_outside_and_empty_selection`: Unassigned-only versus no groups and zero coverage |
| Native path identity and alias | `grouping::tests::aliases_share_one_group` (Unix): same group for native and alias paths |
| Tally selected records, failed files, synthetic exporter ownership | `declarations::grouping_tests::tally_preserves_groups_and_overall`: normal preparation and tally with library queries, group/overall serialization, one error; existing CLI reports/prelude tests cover normal invocation |
| Discovery failure before output; non-stats remain usable | `symbols::grouping_core::discovery_failure_is_stats_only_and_writes_no_envelope`: malformed Cargo root plus explicit source, exit 1, actionable stderr, empty stdout; other modes succeed |

All new tests are L1: hermetic filesystem/library and CLI boundaries require no
terminal or browser. Unit tests compile in the hug bin; CLI tests are registered
in the existing symbols target. No parser/schema/configuration reader is added
or changed, and no values are persisted: Phase 1's parser matrices and shipped
manifest corpus remain authoritative. No new fixtures or CI cells are needed.

### Core implementation and focused results

- `Catalog::capture` gates before Git or structure discovery. Stats resolves the
  active worktree root (scan root outside Git), captures one structure catalog,
  and builds a borrowing ownership lookup once in `Grouping::new`. Discovery
  errors return exit 1 before prelude/source work or output. Successful absence
  uses Unassigned. No new dependency: the existing sniff edge disables defaults;
  sniff has no tree-hugger edge and normal Cargo compilation confirms no cycle.
- Groups use native package directory keys, check the deepest owner's name
  provenance, retain zero/failed selected groups, sort by portable path then
  name, and append Unassigned. No source records enter the group state or cache.
- `tally` attributes once from the native analysis file and adds each selected
  record to overall and group Stats. Container-only classes, prelude export
  identity versus target source location, unknown docs, unequal-sized ratios,
  every summed counter, empty selection and single error reporting are covered.
  Existing Stats tests and output writers are unchanged. Phase 3 will consume
  `Tally.groups` and `Catalog.root`; temporary dead-code allowances document that
  phase boundary rather than introducing premature report changes.
- The first attempted test-file write used a root-relative path from the package
  working directory and failed; the accidental focused run selected zero tests
  and is not evidence. Corrected paths and registration before implementation.
  The registered CLI failure test then failed with exit 0 and a success envelope,
  proving the original behavior. After capture was wired, it exposed sniff's
  diagnostic without a manifest filename; the test now asserts the actual
  unclosed-array cause and repair guidance, alongside exit 1 and empty stdout.
- `just test grouping --test-threads 4`: 11 passed (10 new targeted tests plus an
  existing grouping-named tree report test), 1,907 excluded by the focused filter.
  Tasks 2.1–2.3 and the grouping-unit-test checkpoint are now checked off.

### Shipped inputs, documentation, and gate preparation

- Added `grouping::tests::shipped_manifests_form_declared_groups`, a passive
  corpus over both shipped tree-hugger package manifests, asserting declared
  names and sorted paths through catalog discovery and group results. Added
  `symbols::grouping_core::shipped_manifest_through_normal_stats_invocation`,
  copying the shipped CLI manifest into the isolated Scratch command builder
  and asserting normal stats counts, coverage and errors. Repository inputs
  use literal `include_str!` paths so CI can select their consumers.
- Final focused `just test grouping --test-threads 4`: **13 passed**, including
  **12 new tests** and one existing matching tree report test; 1,907 tests
  excluded only by the focused filter. All nine new bin unit tests and all
  three symbols integration tests compile without feature gates. The Unix
  symlink case alone is target-gated; the other eleven tests are portable L1.
  The units are five ownership/capture cases, one shipped corpus and three
  tally cases (nine bin tests total), plus three CLI cases.
- `cargo metadata --offline --no-deps --format-version 1` followed by normal
  dependency-graph traversal confirms sniff does not reach either tree-hugger
  package; the CLI sniff edge has defaults off and no requested features.
- Updated README, current symbol-report docs (including a file-to-group flow),
  dependency usage docs, and skill guidance. Docs explicitly mark package
  sections and JSON groups planned while describing built stats discovery and
  failure handling. No docs name the feature snapshot.
- Saved plan/log inventories and spec handoff metadata (`human_review: false`).
  The explicit Phase 2 implementation request resolves the prior policy
  checkpoint; no further human decision is needed before report implementation.
- Required root scope command,
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`,
  selected `[claudine-cli, sniff, sniff-cli, tree-hugger-cli, test-toolkit]`
  from this combined staged/unstaged tree. Phase 2 itself touches only
  tree-hugger-cli source. Gates cannot be skipped.
- `just lint` passed for both tree-hugger packages; it runs Clippy across all
  targets with warnings denied, without invoking cargo fmt.
- `just check-tier-coverage tree-hugger` passed: **0 stranded tests**.
- Available hosts: BUILD_LINUX=build-linux, BUILD_WIN=build-win-native,
  BUILD_WSL=build-win. `scripts/cross-check.sh` uses `git write-tree` and
  `git -c commit.gpgsign=false commit-tree` to transport a snapshot. Did not run
  it under the explicit no-commit instruction. Path ownership delegates to
  sniff's tested native lookup, and root normalization uses biscuit-file's
  cross-platform helper; no custom Windows comparison or platform API changed.
  Local evidence is macOS. Existing CI supplies other environments.

### Exact targeted tests and remaining verification

The twelve new tests are:

- `grouping::tests::catalog_requests_only_stats_once`
- `grouping::tests::catalog_roots_follow_active_worktree`
- `grouping::tests::ownership_ordering_and_zero_groups`
- `grouping::tests::absence_outside_and_empty_selection`
- `grouping::tests::aliases_share_one_group` (Unix)
- `grouping::tests::shipped_manifests_form_declared_groups`
- `declarations::grouping_tests::tally_preserves_groups_and_overall`
- `declarations::grouping_tests::classes_tally_counts_only_containers`
- `declarations::grouping_tests::synthetic_export_belongs_to_prelude_not_target_location`
- `symbols::grouping_core::discovery_failure_is_stats_only_and_writes_no_envelope`
- `symbols::grouping_core::absent_catalog_still_reports_selected_totals`
- `symbols::grouping_core::shipped_manifest_through_normal_stats_invocation`

The first nine run in the hug bin test target; the three `grouping_core` cases
run in the symbols target (`tests/symbols/main.rs` declares the module).
No tier marker or custom feature removes them from L1. Existing
`stats::tests::{coverage_counts_known_docs_and_reports_unknown_apart,ratio_is_null_when_no_record_has_known_docs}`
remain unchanged. CLI `reports::{count_and_stats_apply_every_filter,classes_count_and_stats_count_containers_only,stats_json_envelope_counts_doc_coverage_and_reports_failures_last}`
and the prelude/changed-file modules provide downstream invocation coverage.
The full area gate includes these tests, not just the new focused cases.

A second required root scope calculation after metadata/docs updates returned
`packages: [claudine-cli, sniff, sniff-cli, tree-hugger-cli, test-toolkit]`,
`change_class: package`, exit 0. The feature-wide non-stats catalog-cost
checkbox is also checked because both injected request-count and CLI
failure-isolation tests prove it. Other feature-wide acceptance and Phases 3–5
remain pending; only the Phase 2 full-test/lint checkbox awaits the full suite.

### Completion record

- Full macOS `just test --test-threads 4`: **1,920 passed**, **0 skipped**,
  no unrun selected tests, 11 slow notices, 397.257 seconds. All twelve new
  tests, the unchanged Stats tests, CLI filters, class/prelude counting,
  changed-file empty selections, caches, and report modes passed. No remaining
  pre-existing or phase-caused failure was found. The deliberate red CLI
  regression and diagnostic assertion adjustment are recorded above. An early
  root-level `just test grouping` invocation also matched no package; it was
  corrected to the package-area recipe and supplies no test evidence.
- `just lint`: passed for tree-hugger and tree-hugger-cli with all targets and
  warnings denied. `just check-tier-coverage tree-hugger`: passed, zero stranded
  tests. No required gate was skipped. `git diff --check`: passed.
- L2/L3/browser/real gates were not run: no added or changed behavior requires
  those resources, and no tests in those tiers were added or renamed.
  No terminal or browser window was opened. Linux/native Windows/WSL2 local
  runs were not performed because cross-check creates/stages a synthetic
  commit; other-OS evidence remains for existing CI, as described above.
  No monorepo-wide or unrelated package runtime gate was run.
- All Phase 2 todos are checked, alongside the independently proven
  feature-wide non-stats discovery constraint. Later-phase tasks remain
  unchecked. Plan/log source, docs, skill and cumulative package inventories
  are saved; the spec's review flag is false and its Phase 3 handoff is saved.
- The final required scope calculation after these completion records are
  saved reports `packages: [claudine-cli, sniff, sniff-cli, tree-hugger-cli,
  test-toolkit]`, `change_class: package`, so the already completed gates
  are mandatory. This combined scope includes the pre-existing staged work.

Implementation complete, ready for review. Group counters and discovery are
built; package output remains Phase 3 work. No cargo fmt, staging or commit of
workspace files, lifecycle move, new dependency, persistence change, inventory
enrichment, network request or CI cell was introduced. No process is left
pending.

## Phase 3

### Behavior-to-test mapping (before implementation)

- JSON envelope fields, ordering, root frame, named/null identities, identical stats shapes and summed counters: `grouping_reports::json_groups_preserve_overall_counts_and_order`.
- Root `.`, Unassigned-only, zero-match and empty changed selection: `grouping_reports::root_unassigned_zero_and_empty_reports`.
- Overall and per-group terminal sections in default/plain/table, zero coverage, literal Unicode/markup/control names: `grouping_reports::terminal_sections_are_ordered_and_literal`.
- Conditional synthetic counts and undefined coverage in every section: stats writer unit test `grouped_terminal_preserves_synthetic_and_unavailable_coverage`.
- Accepted modes and rejected stats combinations with exit 2/empty stdout: `grouping_reports::stats_option_conflicts_remain_argument_errors`, plus existing `output::tests` and `reports::exactly_one_output_mode_with_the_stats_exception`.
- Shipped manifests: existing passive `grouping::tests::shipped_manifests_form_declared_groups` and normal CLI `grouping_core::shipped_manifest_through_normal_stats_invocation`, extended to assert public group identity.

All new subprocess/filesystem tests use the existing isolated Scratch toolkit builder and are L1 in the declared symbols target. No real terminal resource is needed. No parser, config reader, schema input, template, or persistence changes are planned; the input robustness matrix and persisted round trips do not apply. Existing manifest readers and their Phase 1 matrix remain unchanged. JSON output schema additions will be tested through normal invocation.

The starting tree includes staged prior work, despite the prompt's clean-tree assumption. Preserve it; inventories below will list only Phase 3 edits. Required gates will run regardless of combined scope. Phase 3 completes only its own todos; Phase 4/5 remain pending.

### Report implementation and targeted evidence

- Added and ran the four CLI tests before changing implementation: the three output tests failed on missing `package_root_dir`/`groups`; option conflicts passed. Exact inputs are retained in `grouping_reports.rs`. A preceding invocation used the wrong working directory and ran zero matching tests; it supplies no evidence and was corrected.
- One `Stats::as_json` projection serves overall and group output; JSON retains version 1, appends portable discovery root before stats and ordered groups before final errors/newline. Root package paths use `.`, null package distinguishes Unassigned from a package with that name.
- Added grouped terminal writer: literal bold Prose headings without wrapping, followed by the unchanged table/coverage rendering. Existing `render::literal` neutralizes C0/DEL/C1 and escapes Prose syntax; plain strips rendered escapes. Removed Phase 3 placeholder comments/dead-code allowances.
- `just test grouping --test-threads 4`: 17 passed, 1,907 outside the filter. Existing corpus and CLI shipped-manifest tests passed; the latter now asserts named identity and overall/group equality. Tasks 3.1–3.3 checked immediately after this passing run.
- Added writer coverage for synthetic/unknown/zero counters and literal path markup/controls, plus CLI `subpackage_scan_keeps_separate_root_frames` proving scan-root and discovery-root independence. These supplement the pre-implementation mapping; targeted validation follows.

### Snapshot and documentation review

- Targeted `just test stats --test-threads 4` found the two expected golden differences and stopped early (9 passed, 2 failed, 10 selected tests not run). Updated only `symbols.stats` and `symbols-stats.json` using `HUG_UPDATE_GOLDEN=1 just test golden::stats --test-threads 4` (2 passed). The JSON golden helper now normalizes package_root_dir as well as root_dir. Reviewed: counts/coverage unchanged, JSON adds one Unassigned group equal to overall stats, terminal adds overall heading and repeats the same counters under Unassigned. The full gate runs without the update variable.
- README, symbol-report topic, and tree-hugger skill now describe emitted grouping rather than planned output. Created docs/release-notes.md with before/after terminal output and exact golden inventory. This necessarily overlaps the current-behavior portion of Phase 5; its checklist remains for that phase to audit. No dependency docs need changing.
- Local OS risk review: read the os skill; hosts are build-linux, build-win-native, build-win. No native ownership/comparison/platform logic changed. New output uses existing portable-path and literal rendering helpers. Cross-check is not run because scripts/cross-check.sh constructs an unsigned synthetic commit via write-tree/commit-tree, contrary to this session's no-stage/no-commit instruction. macOS is the local evidence; other environments remain for existing CI. No new CI cells.

### Required gates and tier registration

- Required root scope command after saved metadata: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`. Result: packages `[claudine-cli, sniff, sniff-cli, tree-hugger-cli, test-toolkit]`, change_class `package`, exit 0. Combined scope includes pre-existing staged work; Phase 3 source changes require both tree-hugger gates.
- `just lint` passed for tree-hugger and tree-hugger-cli, all targets with warnings denied. Recipe inspected: Clippy only, no cargo fmt. `just check-tier-coverage tree-hugger` passed, zero stranded tests. `git diff --check` passed.
- Five new CLI tests are declared by `tests/symbols/main.rs` in Cargo's symbols target; writer test is in the hug bin target. All names are L1 with no feature gates. Existing shipped manifest corpus covers both library/CLI manifests; the normal shipped CLI invocation asserts group identity and counters. The two goldens cover every shipped multi-language fixture and retain visible/error/count contracts.

### Completion record

- Full macOS `just test --test-threads 4`: **1,926 passed, 0 skipped**, 11 slow notices, 389.986 seconds, exit 0. The run had no update-golden environment flag. No remaining phase-caused or pre-existing failures were found. Deliberate initial red output tests and expected stale golden failures are recorded above; all selected tests now ran.
- `just lint`: passed for both area packages, exit 0. `just check-tier-coverage tree-hugger`: passed, **0 stranded tests**. `git diff --check`: passed. No required gate was skipped.
- Exact new tests (all passed in the full gate):
  - `symbols::grouping_reports::json_groups_preserve_overall_counts_and_order` — version/field order/newline, package identity/order, shared stats shape, sum of all categories/coverage/synthetic counters, unequal-size overall ratio.
  - `symbols::grouping_reports::root_unassigned_zero_and_empty_reports` — named root `.`, actual package named Unassigned versus null identity, absent catalog, zero matches and empty changed selection with discovery root always present.
  - `symbols::grouping_reports::terminal_sections_are_ordered_and_literal` — default/plain/table sections in JSON group order, zero coverage in every section, spaces/Unicode/markup/link syntax/OSC/C0 names displayed literally, original name preserved in JSON.
  - `symbols::grouping_reports::stats_option_conflicts_remain_argument_errors` — every declaration command accepts JSON/default/plain/table stats, rejects count/JSONL/full/sort/tree/all explicit signature modes with exit 2, stderr and empty stdout.
  - `symbols::grouping_reports::subpackage_scan_keeps_separate_root_frames` — normal subprocess invocation inside a package preserves scan root and worktree discovery root, relative group path, and matching totals.
  - `hug::stats::tests::grouped_terminal_preserves_synthetic_and_unavailable_coverage` — literal path/name rendering, conditional synthetic counts in overall/group sections, unavailable and empty coverage ratios, JSON original identity and stats parity.
- Extended `grouping_core::shipped_manifest_through_normal_stats_invocation` asserts the real shipped CLI manifest's identity and overall/group equality. Existing passive manifest corpus, multi-language JSON/terminal goldens, `output::tests`, stats filters/class counts, partial-error reports and forced-style/narrow-width tests passed in the full gate.
- No L2/L3/browser/real test was added or changed; those resource gates were not needed or run. No terminal/browser window gained focus. Linux/native Windows/WSL2 local gates were not run for the cross-check transport reason above; this is macOS behavioral evidence only. No monorepo-wide gate ran.
- All **Phase 3** tasks and its validation checkpoint are checked. Phases 4 and 5 and feature-wide acceptance checkboxes remain open for their own work. Both plan/log Phase 3 source/docs/skill inventories are saved, cumulative packages preserved, spec `human_review: false`, `human_review_items: []`, and Phase 4 handoff saved. No human decision is required before the next phase.
- Final root scope calculation (same required command, after completion records): packages `[claudine-cli, sniff, sniff-cli, tree-hugger-cli, test-toolkit]`, change_class `package`; gates were required and completed.

Implementation complete, ready for review. No cargo fmt, staging, workspace commit, lifecycle move, dependency change, new parser, cache/persistence change, or CI matrix cell. Prior staged work is preserved. Every started command has completed.

## Phase 4

### Requirement-to-test mapping before implementation

Phase 4 adds acceptance coverage for existing public behavior. No parser,
configuration reader, persistence format, or extraction behavior is changed;
the Phase 1 input robustness matrix and shipped manifest corpus still apply.
There is no reported regression input requiring an implementation fix.

| Requirement | Concrete public CLI tests planned in symbols::grouping_acceptance |
| --- | --- |
| Cargo root/nested/unnamed/prefix siblings, duplicate names, library and CLI, virtual root | `cargo_membership_uses_deepest_declared_identity`, `shipped_area_manifests_remain_separate_packages` |
| Other ecosystem, standalone with and without Git, absent manifests | `npm_membership_and_standalone_without_git` plus existing grouping_reports root/absence tests |
| Linked checkout and subpackage scan/discovery frames | `linked_worktree_uses_its_own_catalog` plus existing `subpackage_scan_keeps_separate_root_frames` |
| Outside root cannot trigger second discovery | `outside_file_stays_unassigned_even_with_broken_external_manifest` |
| Aliases, native Windows spellings, non-UTF-8 names on supporting filesystems | `aliases_count_once_in_the_target_package`, `windows_spellings_share_package_identity`, `non_utf8_package_paths_do_not_merge` (platform cfg only) |
| File/name/scope/visibility/test/doc filters, class containers, all counter sums | `selectors_preserve_group_sums_and_count`, `classes_count_containers_in_each_package` |
| Failed and zero-match files, unknown docs, empty selections | `failed_and_zero_match_packages_keep_zero_groups` plus existing empty changed tests |
| Cross-package synthetic export belongs to exporter | `cross_package_prelude_counts_only_the_exporting_package` |
| Default/plain/table totals and order agree with JSON | `text_modes_match_json_sections_and_counts` plus existing literal-heading/conflict tests |
| Warm-cache manifest rename, no source reparse or entry changes | `manifest_renames_refresh_groups_on_actual_cache_hits` |
| Failure versus absence, no non-stats catalog, bounded cost | Existing grouping_core failure-isolation CLI test and grouping::tests::catalog_requests_only_stats_once; inspect production call sites for structure-only discovery |

Tests use the existing isolated Scratch process builder and temporary local
filesystems. They belong to L1: no terminal, browser, device, or network is
needed. Register the module in the existing symbols Cargo target and run the
tier-coverage check. All fixture paths are constructed from native temp roots.
Prior staged edits are preserved. Only Phase 4 todos will be completed here;
Phase 5 remains open. The prompt limits runtime gates to tree-hugger, overriding
the plan checkpoint's sniff gate. No L2 files are being changed.

### Acceptance coverage and targeted findings

- Registered grouping_acceptance.rs in the existing symbols target. Added
  14 CLI tests (11 portable, one Unix alias test, one native-Windows spelling
  test, one Linux non-UTF-8 case; 12 run on macOS). All use public reports and
  isolated processes; the shared assertion checks complete category maps,
  documented/total/unavailable and synthetic sums, and each ratio.
- Root-plus-nested fixture initially omitted `.` from workspace members.
  Sniff's catalog owns workspace membership; corrected the fixture to include
  the root explicitly rather than changing grouping or adding a manifest walker.
  The virtual-root variant removes both root identity and its membership.
- The initial outside-file test used a positional absolute glob, which did not
  select the external file. Corrected to the normal explicit `--file` path; the
  external broken manifest is never discovered, and the file is Unassigned.
  Alias tests now run each spelling separately as well as together so a dropped
  spelling cannot hide behind a successful control. Forced Rust uses the
  established positional explicit-file path. Windows verbatim cwd uses a
  native current_dir instead, since positional `?` is glob syntax.
- Text row assertions initially used JSON enum casing; terminal labels are
  lowercase for kinds/visibility, so corrected test expectations. No production
  defect or behavior change was found. The original fixtures remain in tests.
- Membership, root/path, counting and warm-cache tests passed before checking
  Tasks 4.1, 4.2, 4.3 and 4.5; output cross-mode assertions await the rerun.
  Warm rename runs read each new name twice and return to the original name:
  every run hits unchanged source with zero parses/captures/diagnostics; cache
  entry bytes and path remain identical. Existing non-stats injected counter
  and CLI malformed-catalog tests prove no catalog request in other modes.
- Production cost inspection: Catalog::capture calls only
  detect_repo_structure_or_root_package; no inventory, lockfile, network or
  remote enrichment is requested. Dependency still disables default features.
  Existing passive shipped-manifest corpus remains; the new normal CLI test
  reads both shipped area manifests through include_str!, ensuring visible
  CI dependencies and separate library/CLI groups. No new parser/schema input
  or persistence code means no additional input matrix is needed.
- OS skill and Windows-path guidance reviewed. APFS rejects invalid UTF-8
  filename bytes; native non-UTF-8 filesystem test targets Linux (including
  WSL2), while existing sniff public lookup coverage uses captured native
  catalog paths on all Unix systems. Windows-specific test names are ordinary
  L1 names behind cfg(windows), never tier markers or skips.
- No BUILD_* host variables are exported in this shell. Cross-check transport
  also runs git add/write-tree/commit-tree (scripts/cross-check.sh:257–269);
  it is not invoked under the explicit no-stage/no-commit instruction. Other
  OS execution remains for existing CI; no new cells. Local evidence is macOS.
- A root-level `just test grouping` selected no package and supplied no test
  evidence. One fixture-edit command also used a package-relative working
  directory with a repository-relative path; it changed nothing and reran the
  same failing fixture. Corrected the path before the next build.

- Final targeted `just test grouping --test-threads 4`: **30 passed**,
  1,908 tests outside the filter. Task 4.4 checked immediately after this
  pass. The feature-wide summed-counters and warm-cache checkboxes are now
  checked; Phase 5 and the feature-wide local-Linux evidence requirement stay
  open. Tree-hugger skill now points to the acceptance suite and platform cfg.

### Gate preparation

Plan/log Phase 4 inventories list only this phase's two test source files,
three updated lifecycle documents and one updated skill; no new docs.
Cumulative package scope is retained (tree-hugger-cli is already listed).
Spec human_review remains false with no items; Phase 5 handoff is saved.

- Final Windows source review found that a verbatim positional path's `?`
  enters the existing glob grammar, so the Windows test uses a verbatim
  working directory plus an ordinary native --file argument. Forward-slash,
  backslash and simplified absolute inputs are still tested separately and
  together. This tests supported identity spellings without changing the
  existing selector grammar. The cfg-only edit does not alter macOS tests.
- Required root scope command returned packages [claudine-cli, sniff, sniff-cli,
  tree-hugger-cli, test-toolkit], change_class package, exit 0. This includes
  pre-existing staged work; both tree-hugger gates are mandatory.

- `just lint` passed for both area packages, warnings denied. Repeated after
  the final cfg(windows) test edit: passed again. Recipe is Clippy only; no
  formatting command ran. `just check-tier-coverage tree-hugger` passed with
  zero stranded tests. All new tests are reached by symbols/main.rs and
  selected at L1 with no feature gates. `git diff --check` passed.
- Portability review checkpoint checked: no hard-coded Unix roots, ordinary
  platform cfg on boundary-specific tests, no new CI cells. Plan wording now
  says covered by existing CI, with an explicit pending-execution note, rather
  than implying Windows/WSL2 passes were produced by this macOS session.

### Completion record

- Full macOS `just test --test-threads 4`: **1,938 passed, 0 skipped**,
  11 slow notices, 395.764 seconds, exit 0. Every selected test ran, including
  all 12 new macOS acceptance tests, existing output safety/conflicts, empty
  changed selections, golden corpus, counters and shipped manifest corpus.
- `just lint`: passed for tree-hugger and tree-hugger-cli with all targets and
  warnings denied (including final rerun after cfg-only review).
  `just check-tier-coverage tree-hugger`: passed, **0 stranded tests**.
  `git diff --check`: passed. No mandatory tree-hugger gate was skipped.
- Exact new tests and requirement mappings are listed in the Phase 4 table
  above. Fourteen tests were added: twelve selected and passed on macOS;
  windows_spellings_share_package_identity and non_utf8_package_paths_do_not_merge
  are target-specific and were not compiled/selected on macOS. They remain in
  the ordinary L1 target for native Windows and Linux/WSL2 respectively.
  This session does not claim execution evidence for those two tests.
- No remaining pre-existing or phase-caused local failure. Initial fixture
  expectation failures, empty root command selection and failed path edit are
  recorded above and corrected. No production implementation change was needed.
- Sniff runtime gates were not run: the phase prompt explicitly limits tests
  and lint to tree-hugger, and no sniff file changed this phase. This overrides
  the plan checkpoint's broader scope. L2/L3/browser/real gates were not run:
  no files in those tiers changed, and no changed behavior needs those
  resources. No terminal or browser window gained focus. Other OS local runs
  were not performed for the recorded transport constraint; existing CI
  scheduling and cells are unchanged.
- All Phase 4 tasks and checkpoints are checked. Feature-wide local-Linux
  execution and Phase 5 close-out remain open for their respective evidence
  and audit. Plan/log source/docs/skill inventories and cumulative packages
  are saved. Spec human_review is false, human_review_items is empty, and
  message_to_agent carries the Phase 5 handoff.

Implementation complete, ready for review. Source files changed this phase:
tree-hugger/cli/tests/symbols/grouping_acceptance.rs (created) and
tree-hugger/cli/tests/symbols/main.rs (module registration). No production
source, manifest/parser, dependency, cache/persistence format or CI change.
Tree-hugger skill and lifecycle documents were updated. No cargo fmt, staging,
workspace commit, lifecycle move or subagent was used. All started commands
have completed. Prior staged work is preserved.

Final scope calculation after completion records: packages ['claudine-cli', 'sniff', 'sniff-cli', 'tree-hugger-cli', 'test-toolkit'], change_class package, exit 0. Required tree-hugger gates were run and passed.

## Phase 5

### Test design and scope before edits

This phase audits and completes documentation for behavior already implemented
in Phases 1–4. No production, parser, configuration, persistence, fixture, or
test behavior change is planned; no regression input was reported. Existing
public CLI tests are the verification for each documented rule. Documentation
examples are explanatory, not new executable templates or configuration.
No additional input robustness matrix or round-trip test is needed; Phase 1
already covers supported manifest formats and repeated catalog serialization,
and Phase 4 covers repeated warm-cache reads after manifest edits.

| Documented requirement | Existing concrete verification |
| --- | --- |
| Automatic overall/package sections, matching terminal/plain/table/JSON totals and deterministic order | `symbols::grouping_acceptance::text_modes_match_json_sections_and_counts`; `symbols::grouping_reports::json_groups_preserve_overall_counts_and_order` |
| Deepest manifest-backed owner, unnamed nested owner, root/virtual packages, duplicate names and prefix siblings | `symbols::grouping_acceptance::cargo_membership_uses_deepest_declared_identity`; `symbols::grouping_reports::root_unassigned_zero_and_empty_reports` |
| Cargo/npm membership, standalone projects without Git, shipped library/CLI identities | `symbols::grouping_acceptance::npm_membership_and_standalone_without_git`; `symbols::grouping_acceptance::shipped_area_manifests_remain_separate_packages` |
| Separate scan/discovery frames, active linked worktree, outside-root files | `symbols::grouping_reports::subpackage_scan_keeps_separate_root_frames`; `symbols::grouping_acceptance::linked_worktree_uses_its_own_catalog`; `symbols::grouping_acceptance::outside_file_stays_unassigned_even_with_broken_external_manifest` |
| Filters, classes counting containers, sums and unequal-size coverage ratios | `symbols::grouping_acceptance::selectors_preserve_group_sums_and_count`; `symbols::grouping_acceptance::classes_count_containers_in_each_package`; `symbols::grouping_reports::json_groups_preserve_overall_counts_and_order` |
| Zero-match/failed files, partial errors, unknown documentation, empty selections | `symbols::grouping_acceptance::failed_and_zero_match_packages_keep_zero_groups`; `symbols::grouping_reports::root_unassigned_zero_and_empty_reports`; `changed::empty::deleted_only_changes_render_the_empty_report_in_every_mode`, `excluded_changes_render_the_empty_report_in_every_mode`, `unsupported_changes_render_the_empty_report_in_every_mode`, and `out_of_selection_changes_render_the_empty_report_in_every_mode` |
| Synthetic exports belong to the exporting prelude and stay outside coverage | `symbols::grouping_acceptance::cross_package_prelude_counts_only_the_exporting_package` |
| Discovery errors versus absence, non-stats isolation, bounded discovery cost | `symbols::grouping_core::discovery_failure_is_stats_only_and_writes_no_envelope`; `symbols::grouping_core::absent_catalog_still_reports_selected_totals`; `grouping::tests::catalog_requests_only_stats_once` |
| Literal Unicode/markup/control headings and existing option conflicts | `symbols::grouping_reports::terminal_sections_are_ordered_and_literal`; `symbols::grouping_reports::stats_option_conflicts_remain_argument_errors` |
| Alias/native path identity and warm-cache renames | `symbols::grouping_acceptance::aliases_count_once_in_the_target_package`; `windows_spellings_share_package_identity` (Windows); `non_utf8_package_paths_do_not_merge` (Linux); `manifest_renames_refresh_groups_on_actual_cache_hits` |
| Additive envelope/version 1 and intentional terminal changes over the shipped language corpus | `symbols::golden::stats_symbols_over_multi_match_the_golden`; `symbols::golden::stats_json_symbols_over_multi_match_the_golden` |
| sniff declared-name API, reusable lookup, input matrix, persistence and passive manifest corpus | Phase 1 `package_identity::{cargo_name_matrix, npm_name_matrix, python_name_matrix, go_name_matrix, composer_name_matrix, catalog_round_trip, shipped_manifest_corpus}`; Phase 1 passing sniff gates |

The CLI tests use the declared `symbols` target, registered modules, and
ordinary L1 names. Their hermetic subprocess/filesystem boundaries need no
terminal or browser resource. Shipped-manifest tests use `include_str!` reads
visible to CI and the normal CLI invocation path. No tests are added or renamed
in this phase. The user's Phase 5 scope overrides the plan's request to rerun
sniff gates: only tree-hugger gates will run, if required by the scope calculator.

### Initial audit

Earlier phases already supplied the topic page, release notes, dependency docs,
sniff API docs and skills. The README currently delegates most package semantics
to the topic page; add a compact stats guide so its readers can use the report
directly. Clarify the topic's flow diagram: selected files establish even empty
groups, while selected declarations feed the counters. No code changes needed.
The initial grouping, stats, sniff identity and ownership comment audit found
no drift; the subsequent declaration-scan field audit found the two stale
path-frame comments corrected below.

The working tree contains pre-existing staged work, contrary to the prompt's
clean-tree assumption. Preserve it and inventory Phase 5 edits separately.
The mandated scope command will consequently include earlier work. The OS skill
was reviewed: no BUILD_* host variables are set here, and cross-check builds a
synthetic commit using `git add`/`commit-tree`. It will not be invoked under the
explicit no-stage/no-commit instruction. No new OS behavior is introduced;
Linux, Windows and WSL2 execution remains on the existing CI schedule. Record
this as pending OS evidence, never as a passing local run.

### Documentation tasks completed

- Task 5.1: added the README Package Stats guide covering all report rules,
  path frames, error/absence distinction, coverage and additive versioning.
  Clarified the topic flow so files initialize groups and selected declarations
  increment counters; linked its before/after release example. Skill guidance
  now points to these current records. Checked the task immediately after edits.
- Task 5.2: audited sniff API topic/README and sniff skill against the public
  name-source and ownership APIs. They already describe manifest/fallback
  names, conservative legacy catalogs, alias precedence, native paths,
  root/outside semantics and reusable lookup correctly. Dependency docs
  already record default-features = false, no network/remote/metrics, and
  structure-only discovery without inventory/lockfiles. No sniff/dependency
  documentation edit or new dependency is needed. Checked the task after audit.
- Task 5.3: audited the existing before/after release notes, spike evidence,
  Phase 2 authorized rulings, Phase 1 input/serialization departures and
  subsequent implementation entries. Release notes preserve overall JSON
  stats and correctly call out intentional terminal sections. Relevant source
  docs/comments have no drift. This phase adds no specification departure.
  Checked the task and drift checkpoint after review.
- Exact targeted tests added this phase: none; behavior is unchanged. The
  mapping above identifies existing targeted coverage. Final tree-hugger L1
  rerun will include those tests and passive shipped artifacts.

### Comment drift correction and initial gates

- A closer declaration-scan review found two stale field comments:
  scan_root claimed to anchor every JSON path, and git_root claimed to anchor
  stats grouping. Code correctly uses Catalog.root for package paths. Updated
  only those comments in cli/src/declarations.rs to distinguish declaration
  paths/listing sections from stats package paths. No executable line changed;
  this is the only source file edited in Phase 5. Source inventory updated
  immediately. The correction resolves existing drift, not a behavior change.
- Initial `just lint` passed (both packages, all targets, warnings denied).
  Rerun after the comment correction for the final source tree.
  `just check-tier-coverage tree-hugger` passed with zero stranded tests.
- The required root scope command returned packages
  [claudine-cli, sniff, sniff-cli, tree-hugger-cli, test-toolkit] and
  change_class package. Both tree-hugger gates are mandatory. The full L1
  gate is in progress; four workers preserve all selected tests.

### Completion record

- Full local macOS `just test --test-threads 4`: **1,938 passed, 0 skipped**,
  11 slow notices, 398.794 seconds, exit 0. All selected tests completed.
  This includes the existing public behavior tests mapped above, both stats
  goldens over the shipped language corpus, normal-invocation shipped manifest
  tests, changed-file empty cases and all applicable acceptance cases.
- Final-tree `just test grouping --test-threads 4`: **30 passed**, 1.599 seconds,
  exit 0; 1,908 outside the targeted filter were already proved by the full
  gate. This focused rerun rebuilds after the comment-only source edit made
  while the full gate was running; no runtime behavior changed.
- `just lint`: passed for tree-hugger and tree-hugger-cli, all targets, warnings
  denied; the final rerun after source-comment edits also passed. The actual
  recipe runs Clippy only; no cargo fmt command was run.
- `just check-tier-coverage tree-hugger`: passed, **0 stranded tests**.
  `git diff --check` and YAML frontmatter/inventory validation passed.
  No tests were added, renamed, stranded, ignored, or feature-gated in Phase 5.
- Requirement-to-test mapping is in the table at the start of this phase.
  Exact targeted tests added: **none**, because this phase changes docs and
  two comments only. Existing tests assert public CLI results, errors, summed
  dependent counters, cache bytes/work counters, native identity variants and
  shipped manifest/golden corpora; Phase 1 retains its parser shape matrix and
  repeated catalog round trips.
- No mandatory tree-hugger gate was skipped. Sniff gates were not rerun under
  the user's explicit area limit; Phase 1 records 3,171 passing sniff tests and
  passing lint. L2/L3/browser/real gates were not run: no changed behavior
  requires their resources, and no such tests changed. No terminal or browser
  window was opened or focused. Linux, Windows and WSL2 were not run locally;
  the two platform-specific acceptance tests remain selected on their targets
  by existing CI. There are **no remaining local failures**, pre-existing or
  phase-caused, and no aborted or pending command.
- The plan's original local-Linux execution request remains unproven. Its
  close-out checkbox now records the completed coverage audit and explicitly
  preserves that validation exception instead of implying an OS pass. The
  final both-area gate checkpoint similarly states actual Phase 5 tree-hugger
  evidence and retained Phase 1 sniff evidence. These are validation-scope
  departures from the plan, not reporting behavior departures from the spec.
- All Phase 5 tasks and plan todos are checked. Cumulative source_code and
  documentation inventories are the deduplicated union of the five phase
  inventories (25 source files and 12 documentation/skill files). Phase 5's
  only source edit is tree-hugger/cli/src/declarations.rs, two stale path-frame
  comments; no executable line changed. Phase 5 docs/skill inventories and
  cumulative package scope are saved.
- Plan and log now set completed_phase: 5 and implemented: true. Spec sets
  implemented: true, implemented_by: codex/gpt-6.1-sol, human_review: false
  with no items, and a review handoff. No next implementation phase remains.
  Implementation complete, ready for review; the author owns the lifecycle
  move after review closes. No staging, commit, formatting, lifecycle move,
  subagent or cross-check synthetic commit was performed. Prior staged work
  is preserved. All started commands have completed and their results were read.

Final scope calculation after all completion flags and inventories were saved:
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
returned packages [claudine-cli, sniff, sniff-cli, tree-hugger-cli, test-toolkit],
change_class package, exit 0. Both required tree-hugger gates were run and passed.
The scope includes preserved pre-existing staged changes.
