---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-11-21T21:11:05+00:00"
spec: 2026-11-20-commit-link-walks/spec.md
implemented: false
description: "A **fix** review of `2026-11-20-commit-link-walks/spec.md`"
fix: 2026-11-20-commit-link-walks/review-2.md
previous: 2026-11-20-commit-link-walks/review-1.md
---

# Review 2: Commit link walks

The fix is production ready. All three previous findings are verified fixed,
including the additional instances recorded during repair. No new findings or
author decisions remain.

Reviewed the current working tree, including the uncommitted repair, against
the specification, its three author rulings, review-1.md, and the repair log's
Finding Ledger. The previous review contains no Human rulings section.
Implementation completion flags were not used as proof of correctness.

## Previous Finding Verification

The child rows below cover every instance in the repair ledger, including
sites that were already correct. Source paths are relative to the repository
root. Test names identify the checks run in this review unless explicitly
described as recorded Windows evidence.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Required Windows Git startup optimization is missing | R1-F1.1–R1-F1.5 | verified fixed | Checked shared ownership, initialization order, child cleanup, Worktree consumers, and other binaries below. | Nothing |
| R1-F1.1: Shared helper ownership | Installation lookup and both hint helpers | verified fixed | `sniff/lib/src/programs/git_program.rs` owns the implementation; the former Worktree file is deleted. Compared the moved implementation with its original Git version; layout tests pass. | Nothing |
| R1-F1.2: Initialize before threads | Sniff main binary | verified fixed | Plain `sniff/cli/src/main.rs::main` calls the helper before runtime construction. Windows CLI tests count trace files, including a positive control with an unusable caller hint. Repair records their passing native Windows run; tests and wiring inspected here. | Nothing |
| R1-F1.3: Child environment | Internally supplied and caller-supplied hints | verified fixed | Hook precedes spawn in `run_command_with_timeout`; both `a_child_does_not_inherit_the_installation_hint_sniff_set` and `a_child_inherits_the_installation_hint_its_caller_set` pass. Production spawn search confirms the bounded runner is the launch boundary. | Nothing |
| R1-F1.4: Worktree consumers | Startup, Git command builder, refresh worker | verified fixed | `worktree/lib/src/git.rs` re-exports the shared functions; CLI main and refresh-worker cleanup retain their calls. Compared moved behavior with original; 18 Worktree Git tests pass. | Nothing |
| R1-F1.5: Other Sniff binaries | Declared fixture binaries | verified fixed | Render fixtures construct synthetic facts; foreground-handoff fixture launches a shell without repository discovery. No additional repository-opening startup needs initialization. | Nothing |
| R1-F2: Root-package status selection hides history and changes | R1-F2.1–R1-F2.8 | verified fixed | Public status-preparation tests and the original CLI reproduction now retain root-selected history and changes. | Nothing |
| R1-F2.1: Root prefix and history | Root member and nested-package control | verified fixed | Resolver tests return the empty root prefix and slash-terminated nested prefix. `a_root_package_selection_keeps_the_repository_history_and_changes` passes; original CLI reproduction shows one recent commit. | Nothing |
| R1-F2.2: Working-tree narrowing and counts | Staged, unstaged, untracked root changes | verified fixed | Public test asserts dirty state and counts `(1, 2, 1)`; CLI reproduction with both packages changed gives `(2, 2, 2)` and retains all four paths. | Nothing |
| R1-F2.3: Final links and rendering | Contained root-selected commit | verified fixed | Public test checks the displayed full commit ID's link; renderer test checks full-ID URL association and zero repository work. Compact CLI reproduction displays the commit. | Nothing |
| R1-F2.4: Nested-package control | `-p member` | verified fixed | Public test retains only the member change and its link; CLI reproduction retains member changes with counts `(1, 1, 1)`. | Nothing |
| R1-F2.5: Area and intersection controls | Area alone, member plus area, root plus named area | verified fixed | Original CLI reproduction confirms the first two; resolver and public status tests assert the root/named-area intersection error. Empty area remains excluded by the resolver. | Nothing |
| R1-F2.6: Later branch override | Root package plus another branch | verified fixed | Original reproduction restores branch history and clears changes/counts; combined-selector tests also pass. | Nothing |
| R1-F2.7: Catalog consumers | Package and area lists | verified fixed | Original reproduction returns `['root-app']` and `['']`; `commands/repo.rs` discards the resolved prefix and filters by identity. | Nothing |
| R1-F2.8: Other validation consumers | Structure, dependencies, changed-package actions | verified fixed | Searched every `resolve_package_and_area` caller. Generic `commands/mod.rs` uses the result for validation only; status preparation is the only production prefix consumer. | Nothing |
| R1-F3: Dirty package reports read arrays absent from ordinary requests | R1-F3.1–R1-F3.9, including the expanded R1-F3.6 | verified fixed | Ordinary collection, public path projection, CLI family tests, original reproductions, and sibling checks below agree. | Nothing |
| R1-F3.1: Dirty packages | Root and nested packages, staged/unstaged/untracked; conflict inclusion | verified fixed | Public `aggregate_changes` test proves patch arrays are empty while changed paths remain available. Original CLI reproduction names both packages correctly. Dirty scope includes every `file_changes` status, so conflicts cannot be filtered out. | Nothing |
| R1-F3.2: Dirty areas | Root and member selections | verified fixed | Original CLI reproduction returns `['']` for root and `['area']` for member; family test checks both together. | Nothing |
| R1-F3.3: Text renderers | Dirty packages and areas | verified fixed | Original reproduction and CLI test print `member` / `area` with exit 0. | Nothing |
| R1-F3.4: Staged projections | Package and area controls | verified fixed | Original reproduction names the correct packages/areas; public test checks staged-only attribution; CLI boundary fixture checks staged ownership. | Nothing |
| R1-F3.5: Unstaged projections | Package and area controls | verified fixed | Original reproduction gives correct names; implementation retains its explicit Modified/Both selection. Conflict exclusion is unchanged, not a regression introduced by repair. | Nothing |
| R1-F3.6: Current-area decisions | Dirty/source facts, sibling prefix, deep-mode duplicate count | verified fixed | Original reproduction returns both booleans true in the changed member. Area-prefix, source-count, and indeterminate-observation tests pass; both focused and aggregate callers use `area_changed_paths`. | Nothing |
| R1-F3.7: Clean controls | Dirty/staged/unstaged families after reset | verified fixed | Original reproduction returns empty names after reset; CLI family and public projection tests check clean results. | Nothing |
| R1-F3.8: Package directory boundary | Root exclusion, nested ownership, CLI projections | verified fixed | Shared `path_is_within` requires a separator boundary. Both attribution tests and `dirty_packages_respect_package_directory_boundaries` pass: `area/member2/notes.rs` belongs to root, not `area/member`. All three CLI collectors use shared attribution. | Nothing |
| R1-F3.9: Verbose filesystem listing | Dirty and untracked paths without patches | verified fixed | `verbose_filesystem_section_lists_ordinary_file_changes` passes. Renderer enumerates `file_changes`; optional patch arrays supply diff text only. | Nothing |

The repairs do not merely fix the first reported inputs: they cover the
ledger's root/member controls, downstream projections, directory boundaries,
and optional-detail consumers. No partial repairs or newly found siblings
remain.

## Unblocked Findings

None.

## Blocked Findings

None.

## Spec coverage and verification level

| Requirement | Evidence and level | Assessment |
|---|---|---|
| Shared visits across remote tips, preferred remote/default-branch ordering, duplicates, total budget, single-tip behavior | L1 repository fixtures with isolated linker counters; inspected completed-set publication and pruning | Passing |
| Failed-walk fallback preserves later matches and unknown results | L1 injected failure followed by actual ancestry walking; implementation discards sharing after failure | Passing |
| Shallow histories agree with independent walks; missing snapshots stay unknown | L1 shallow fixture and exact counters; missing/decoration-only snapshot tests | Passing |
| Final package/area/branch/worktree selection, detached HEAD, effective worktree configuration and ref reuse | L1 public preparation plus spawned CLI fixtures, including repaired root member | Passing |
| Link only displayed full IDs; renderer performs no Git reads | L1 public result and renderer markup/counter checks | Passing at the boundary the spec explicitly requires |
| Plain, empty, zero-history and focused JSON avoid linking; performance includes scoped preparation and linking | L1 library and CLI checks of stages, exact visit deltas, output shape and channels | Passing |
| Windows initialization, shared ownership, child-environment preservation | L1 source inspection and child-process tests here; native Windows process-trace tests inspected with repair's recorded execution | Required implementation and tests present |
| Linux terminal status within 500 ms of JSON without a commit-graph | Original implementation record: median 256 ms text versus 144 ms JSON, 16,682 linking visits, one ref walk and discovery | Recorded acceptance met; timing sample not repeated |
| Existing aggregate/recent-commit behavior | Shared linker remains the authority; deep containment retains independent walks; repair changes no commit-family wire shape | No defect found |

This change supplies links to existing terminal components and introduces no
keyboard, mouse, terminal-encoding, or new terminal-style promise. The spec
requests renderer-boundary URL checks, so L1 is appropriate; no new L2/L3
requirement is inferred. The input robustness matrix does not apply: no
file-format loader or field validator was added or changed. Moving the
existing installation lookup does not introduce a new configuration parser.

## Runs performed

```sh
cargo nextest run --color=never -p sniff --features remote -p sniff-detection-cli \
  -E 'test(commit_links::) | test(status_report::) | test(test_repo_git_status_) | test(captured_links_match_full_ids) | test(programs::git_program::) | test(installation_hint) | test(aggregate_changes) | test(dirty_package) | test(staged_package) | test(package_area_source) | test(current_package_area) | test(verbose_filesystem) | test(attribute_paths) | test(scope::tests)'
cargo nextest run --color=never -p worktree --lib -E 'test(git::)'
cargo build --color=never -p sniff-detection-cli --bin sniff
python3 -I target/commit-link-walks-review-1/reproduce.py
cargo clippy --color=never -p sniff -p sniff-detection-cli -p worktree --all-targets -- -D warnings
just check-tier-coverage sniff
just check-tier-coverage worktree
```

Results: **129 Sniff/CLI tests passed** (longest 0.242 s), **18 Worktree
tests passed** (longest 0.058 s); build and strict Clippy passed; both areas
have zero stranded tests. The reproduction script exercises disposable
repositories and confirms every original reporting case and clean control.
New integration modules are declared in their consolidated L1 targets; the
Windows module is platform-gated, not placed behind a non-running tier.

The repair's full canonical runs were not repeated. Their slow-test record
at `target/slow-tests/current.md` independently records 3,879 Sniff tests
and 1,480 Worktree tests, both with zero tests at or above five seconds.
No slow test appeared in this review's runs. Native Windows execution was
not repeated here; missing additional cross-OS proof is not a readiness
blocker under the review instructions.

## Implementation decisions and record correction

Root-package status selection keeps the whole repository because its path
prefix is empty. Package attribution still gives the root only unclaimed
paths. This follows the prior review's recommended repair and is now explicit
in the status guide; keeping it needs no new author decision. The previous
review's optional human-review item is therefore not carried forward.

The helper move preserves Worktree's public re-exports and existing behavior;
moving its Git command builder with the shared lookup avoids duplication.
Neither this choice nor the private test layout needs human review.

Corrected `log.md` in place and added a dated note: its claim that no OS rigs
were configured for this task contradicted its recorded Windows run. Direct
environment inspection also confirms Windows, WSL2, and macOS destinations
are configured now. Earlier phase-specific environment claims in
`implementation-log.md` were not disproved by today's environment and remain
unchanged. This record correction is not a finding or human-review item.
The same dated note corrects the ledger summaries to nine R1-F3 children
and 22 children overall, matching the enumerated IDs.

No production code was changed during this review. Existing unrelated edits
were preserved. The fix remains in its active directory for the author's
review and lifecycle decision.
