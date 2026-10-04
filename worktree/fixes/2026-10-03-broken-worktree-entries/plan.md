---
total_phases: 6
created: 2026-10-03
phase: 3
agent: claude/sonnet
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/worktree/remove.md
source_files_during_phase_2:
  - worktree/lib/src/availability.rs
  - worktree/lib/src/remove/admin_entry.rs
  - worktree/lib/src/remove/mod.rs
  - worktree/lib/src/lib.rs
  - worktree/lib/src/worktree.rs
  - worktree/lib/src/listing.rs
  - worktree/cli/src/commands/list_table.rs
  - worktree/cli/tests/list_table.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/worktree/list.md
source_files_during_phase_3:
  - worktree/lib/src/fast_forward.rs
  - worktree/cli/src/commands/list_table.rs
  - worktree/cli/src/commands/list.rs
  - worktree/cli/tests/list_table.rs
  - worktree/cli/tests/list_output.rs
  - worktree/cli/tests/snapshots/list_table__closing_notes.snap
  - worktree/cli/tests/snapshots/list_table__unavailable_and_unknown_rows.snap
  - worktree/cli/tests/snapshots/list_table__unavailable_notes.snap
  - worktree/cli/tests/snapshots/list_table__unavailable_note_quoting.snap
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/worktree/list.md
packages:
  - worktree
  - worktree-cli
---

# Plan: Worktrees Git can no longer read

Spec: `2026-10-03-broken-worktree-entries` (`spec.md`).
Skills to load before work: `worktree` (topics `list`, `remove`, `cli-contracts`,
`testing`), `rust-testing`, `os` (Windows short names, verbatim prefixes,
symlink/reparse points, macOS `/private` aliases), `biscuit-terminal` (notes and
legends render through its components), `sniff` only if repository path helpers
live there.

## Summary and definition of done

A linked worktree Git marks `prunable` is currently shown as `○` (clean) because
`dirty_status` maps every `git status` failure to `Clean`, and `wt remove` fails
with a bare Git error. The work has four parts:

1. **Parse and classify (lib).** `WorktreeEntry` gains `prunable: Option<String>`;
   a separate filesystem classifier labels each prunable entry Missing, Unlinked,
   or Other unavailable, using only positive evidence of absence.
2. **Honest listing (lib + cli).** `DirtyStatus::Unknown`; no `git status` for
   prunable entries; `✕` / `?` markers, conditional legends, one dim note per
   unavailable entry; `--ff` refuses a prunable default-branch holder.
3. **Safe removal preparation (lib + cli).** Before inventory, `wt remove`
   prepares the target: healthy → unchanged; Missing → index check, then
   `git worktree remove` of only that record; Unlinked → targeted
   `git worktree repair`, verified by exact postconditions, then the ordinary
   flow; anything else → exit 3 refusal. Bare Git errors gain target, path and
   operation.
4. **Docs and skill.** `docs/cli/list.md`, new `docs/cli/remove.md`, `README.md`,
   `.claude/skills/worktree/{remove,list,cli-contracts}.md`, and the changed
   symbol comments (`dirty_status`, `WorktreeError`, `exit_code`).

Done when every acceptance bullet of the spec holds:

- [ ] a fixture matching `lhg-before` renders `✕` with an accurate note, never `○`; no live checkout is touched
- [ ] no row claims checked-clean after a status failure; merge-comparison `clean` is unchanged
- [ ] unlinked removal verifies repair by postconditions and then follows ordinary protections; failed verification refuses (exit 3) with no deletion and reports possible metadata changes
- [ ] an unknown inventory never permits deleting an existing directory, with any force flags
- [ ] missing removal targets only that entry, follows the staged-index ruling and existing branch rules, and refuses if the target reappears
- [ ] `just test`, `just test-l2`, `just lint` pass in `worktree/` (nextest, focus-free helpers); evidence states the environments actually exercised
- [ ] docs listed in the spec updated, describing current behavior without naming this fix
- [ ] the agent stops at "implementation complete, ready for review": no `just complete`, no move to `_completed`, no commit unless asked, no `cargo fmt`

## Wave schedule (overview)

Tasks in one wave are independent and may run on concurrent subagents. Each wave
ends at a validation checkpoint before the next starts.

| Wave | Phase / tasks | Depends on |
| ---- | ------------- | ---------- |
| 1 | P1 spike (Git facts) and baseline | none |
| 2 | P2 parse + classify · P2 `DirtyStatus::Unknown` + `dirty_status` | Wave 1 |
| 3 | P3 listing render · P3 `--ff` holder refusal · P4 repair engine (lib) · P4 missing-record engine (lib) | Wave 2 |
| 4 | P5 `wt remove` wiring · P5 error context · P5 exit-code contract | Wave 3 |
| 5 | P6 docs and skill (parallel by file) · P6 tests that need the full flow | Wave 4 |
| 6 | P6 final validation | Wave 5 |

## Phase 1: Rulings, spike, baseline

### Necessary Rules

These are decisions the spec leaves open or ambiguous. Each needs an answer from
the author before the task it gates; the proposed default lets work proceed
(`yolo: true`) and must be recorded in the implementation log.

- [x] **R1 — staged-index policy for a Missing directory (gates P4 missing engine).**
  The spec itself blocks missing removal until this is decided and recommends:
  inspect the surviving administrative index, require ordinary discard consent
  when it differs from recorded HEAD, refuse if inspection fails. *Proposed
  default: adopt the recommendation.* Until the author confirms, the spec status
  stays `draft-spec`; the implementation refuses whenever staged state cannot be
  proved disposable.
- [x] **R2 — "index differs from HEAD" definition.** Proposed: compare the tree
  written from the administrative index (`git read-tree`/`diff-index --cached`
  against that index via `GIT_INDEX_FILE`, no checkout needed) with the recorded
  HEAD tree; any difference, an unreadable index, or a missing index with a
  present admin dir counts as "cannot prove disposable". An absent admin
  directory *and* absent directory is a bare record: disposable.
- [x] **R3 — unlinked-state evidence.** "`.git` confirmed absent" means
  `symlink_metadata(<path>/.git)` returned `NotFound` and the path itself is a
  real directory (`symlink_metadata`, not following links). Any other error
  (`PermissionDenied`, I/O) → Other unavailable. A target that is a symlink or
  Windows reparse point is Other unavailable even if the link target is a
  directory.
- [x] **R4 — administrative-entry association.** Proposed: enumerate
  `<common-git-dir>/worktrees/*/gitdir`, read each back-reference, compare to
  `<recorded-path>/.git` with the repo's path-equality helpers; require exactly
  one match. Zero, several, or any unreadable entry → refuse. The common git
  dir comes from `git rev-parse --git-common-dir` run in the base, never from
  assuming base `.git` is a directory. Windows short names, verbatim prefixes and
  macOS `/private` aliases are covered by the helpers.
- [x] **R5 — where the "other unavailable" classification applies to `wt remove`
  when the porcelain entry is not prunable but status fails.** Proposed: removal
  keeps today's flow for non-prunable entries, but `collect_inventory` failures
  are wrapped with target/path/operation context (spec "Error context") and still
  exit via the existing mapping; no repair is attempted for non-prunable entries.
- [x] **R6 — name suggestions in notes.** "Unambiguous command argument accepted
  by existing name resolution": proposed order is branch name, then directory
  basename, each only if `find_worktree` would resolve it to exactly this entry
  (reuse the resolver's candidate logic, not a copy); otherwise the note lists
  the conflicting names and suggests no command.
- [x] **R7 — wider measurement.** The spec rules out performance work. A listing
  now skips `git status` for prunable entries and adds bounded local metadata
  checks; no spike and no benchmark. Recorded here so the author can widen it if
  wanted.
- [x] **R8 — Windows reparse-point test.** Creating a symlink/junction on Windows
  may need privilege. Proposed: Unix symlink tests run on macOS/Linux; the
  Windows reparse-point branch is covered by an injected metadata-inspector
  seam (`#[cfg(windows)]` unit test plus the injected-error cases) rather than a
  privileged fixture. The `os` skill records which hosts can produce Windows
  evidence.

- [x] **R9 — repair is not targeted (found by Spike A; needs author input before
  Phase 4).** `git -C <base> worktree repair <path>` also recreates `.git` for every
  other unlinked worktree, not only `<path>`. *Proposed default:* accept Git's
  behavior, verify only the target, and say that repair "may have changed Git
  metadata for this or other worktrees". The options are in the spec's
  `human_review_items`. Phases 2 and 3 are not affected.

### Spike (runs once, before Phase 2)

One host (macOS, the current one), one scratch repository, no network:

- [x] **Spike A — real Git facts the plan depends on.** With a disposable repo
  create one Missing and one Unlinked worktree and record in the implementation
  log:
  - the exact `worktree list --porcelain` output including `prunable` with and
    without a reason, and whether the line can be bare `prunable`;
  - the layout under `<common-dir>/worktrees/<id>/` (`gitdir`, `HEAD`, `index`)
    for each state, and whether `index` survives deleting the checkout;
  - whether `git -C <base> worktree repair <path>` restores the three
    postconditions (back-reference, `.git` content, non-prunable listing) and
    its exit code (observation only, never asserted);
  - whether `git worktree remove` of a Missing entry with a *locked* record
    refuses without `--force` (confirms "do not add a second `--force`");
  - whether a `GIT_INDEX_FILE`-based `diff-index --cached <HEAD>` works against
    the surviving admin index (informs R2).
  The spike does not widen to other OSes or Git versions; the spec's Git 2.55.0
  observations already answer the rest, so no further spike is scheduled.

### Baseline

- [x] Run `just test` and `just lint` from `worktree/` and record failures that
  predate this work (the git status shows a heavily staged tree; do not attribute
  those to this fix).
- [x] `sniff repo`-style discovery is not needed; use the existing path helpers
  in `lib/src/util.rs` / `lib/src/git.rs`. Locate them and list the exact helper
  names in the implementation log (comparing existing paths, Windows short names,
  verbatim prefixes).
- [x] Inventory every `match`/default over `DirtyStatus` and every use of
  `WorktreeEntry` construction (struct literals will break when `prunable` is
  added): `lib/src/worktree.rs`, `lib/src/listing.rs`, `cli/src/commands/list_table.rs`,
  `cli/src/commands/git_graph.rs`, `cli/src/commands/list.rs`, status refresh
  after `--ff`, tests, and `cli/tests/list_table.rs`.

**Checkpoint 1:** spike notes in the implementation log; R1–R8 answered or
defaults recorded; baseline results recorded.

## Phase 2: Parse, classify, and make status honest

Wave 2 — the two task groups touch different parts of `lib/src/worktree.rs`
and can run concurrently (coordinate on the file; merge `WorktreeEntry` first).

### Task group A — parsing and classification

- [x] **Prunable field.** Add `prunable: Option<String>` to `WorktreeEntry`
  (`None` = no marker, `Some("")` = marker without reason, else Git's reason).
  - Update `parse_worktree_list` (`lib/src/worktree.rs:129`) so the marker is
    captured and reset for every entry, including the last one flushed after the
    loop; remove the "skip prunable lines" comment and fix drifted docs.
  - Fix every struct literal flagged in the baseline inventory.
  - Do not interpret reason text.
- [x] **Availability classifier.** New module (for example `lib/src/availability.rs`
  or inside `worktree.rs` if small) exposing `Availability { Healthy, Missing,
  Unlinked, Other(OtherReason) }` and `classify(entry) -> Availability`.
  - Use `symlink_metadata` on the path and on `<path>/.git`; never `Path::exists()`;
    never canonicalize a missing path; keep Git's recorded spelling.
  - Missing = path `NotFound`; Unlinked = real directory and `.git` `NotFound`;
    everything else prunable (file, dangling link, symlink/reparse point,
    permission/I/O error, `.git` present but Git reports damage) = `Other` with
    a typed reason carrying the observed condition.
  - Inspection goes through a small injectable trait/closure so permission and
    reparse-point cases are testable on every host (R8).
- [x] **Input robustness matrix (below)** implemented for the porcelain
  `prunable` line and the admin `gitdir` back-reference reader.

### Task group B — `DirtyStatus::Unknown`

- [x] **Unknown variant.** Add `DirtyStatus::Unknown` (`lib/src/worktree.rs:~35`).
  `dirty_status` returns `Unknown` on spawn failure and on nonzero exit; update
  its doc comment (it currently promises a `Clean` fallback).
- [x] **No status for prunable entries.** The listing path that calls
  `dirty_status` (`worktree.rs:~491`) skips prunable entries and records
  `Unknown` without running Git; availability is carried separately on the
  status record (add `availability` next to `dirty`, computed once per entry per
  run).
- [x] **Audit every match/default over dirtiness** found in the baseline,
  including the graph annotations, the status refresh after `--ff`, and anything
  using `unwrap_or(Clean)`. Unknown must never count as checked-clean nor as
  known source changes.

### Input Robustness Matrix

Load-bearing inputs: (a) the `prunable` porcelain line; (b) the admin
`<common-dir>/worktrees/<id>/gitdir` back-reference file. Outcomes are asserted
through the public result (entry classification, association, refusal), one
test per input walking a real-Git fixture with one edit per cell, plus a control
row proving the unedited fixture gives the positive result.

| Shape | `prunable` porcelain line | `gitdir` back-reference | Required outcome |
| ----- | ------------------------- | ----------------------- | ---------------- |
| absent | no marker → `None` | file missing → entry not associable | `None` / refuse (never "no match = safe") |
| bare marker | `prunable` → `Some("")` | n/a | marker without reason |
| empty content | `prunable ` trailing space only | empty file | `Some("")` / unreadable → refuse |
| wrong shape | marker embedded in a path or branch value | non-path content, multiple lines | not a marker / refuse |
| duplicate | two markers in one entry | two entries back-reference the same target | last-wins forbidden: refuse ambiguity |
| trailing/invalid | CRLF, trailing blank record, last entry flushed | trailing garbage after a valid path | parsed per format / refuse |
| non-UTF-8 | non-UTF-8 bytes in a reason | non-UTF-8 path bytes | preserved lossily for display only; never used for a safety decision / compared as raw paths |

Code smells to grep before declaring done: `unwrap_or_default()`, `.ok()` and
`filter_map` on the `gitdir` read; `#[serde(default)]` is not involved.

**Checkpoint 2:** `just test` in `worktree/` green; `cargo clippy` via `just lint`
clean; porcelain, classifier (absence, missing `.git`, present broken `.git`,
injected inspection errors, symlink), and `dirty_status` failure tests pass.

## Phase 3: `wt list` and `--ff`

Wave 3 — both tasks are independent.

- [x] **Markers and legend.** In `cli/src/commands/list_table.rs`
  (`dirty_dot` at ~778, row builder ~658, legend ~483): render `✕` for
  unavailable rows, `?` for `Unknown` on available rows. Add
  `✕ git can't read this worktree` and `? couldn't check` to the Worktree legend
  only when they occur; keep the Branch legend; size the table from the
  *rendered* legends.
- [x] **Unavailable notes.** One dim note per unavailable entry in table row
  order, appended after existing PR-status/closing notes without reordering them.
  - Text per spec for Missing, Unlinked (with the repair command and base path),
    and Other (observed condition plus Git's reason when present). Do not claim
    `.git` is absent unless it was classified Unlinked.
  - Render via `biscuit-terminal` components; escape names, paths, and reasons as
    text so they cannot become markup.
  - Command-name selection per R6; shell-quote suggested commands for the
    caller's shell (reuse an existing quoting helper if present); never execute
    displayed text.
- [x] **Comparison cells unchanged.** Ref comparisons keep their meaning;
  nothing derived from dirtiness feeds them. Confirm in the audit.
- [x] **`--ff` holder refusal.** In `lib/src/fast_forward.rs` (`holder_of`, ~121)
  a prunable holder of the default branch counts as a holder: refuse without
  repair or ref move; healthy-holder behavior unchanged. Listing itself performs
  no repair and no record removal; cache writes and worker behavior are untouched.
- [x] **Snapshots and tests.** Rendering snapshots live in
  `cli/tests/list_table.rs`, not in shared CLI unit modules (they compile under
  both the library and binary targets). Cover: `✕`/`?` markers, conditional
  legends, dim notes, stable row order, markup escaping, ordinary comparisons.
  Record Git calls to prove no status or repair runs for prunable entries; inject
  status spawn/nonzero failures through the runner `dirty_status` actually uses
  (`git_command_in`) because the existing recorder's failure injection does not
  reach it. Add the `--ff` refusal test (no repair, no ref move).
- [x] **Snapshot hygiene.** Review the touched `.snap` files by eye; do not
  blind-accept. Include a fixture shaped like `lhg-before` (Unlinked, detached,
  recorded HEAD) and a Missing one.

**Checkpoint 3:** `just test` green; L1 snapshot suite reviewed; `just lint`.

## Phase 4: Library engines for removal (runs in Wave 3 alongside Phase 3)

Wave 3, concurrent with Phase 3 (disjoint files under `lib/src/remove/`).

### Repair engine (Unlinked)

- [ ] **Associate before repair.** Implement `admin_entry_for(base, path)` per R4:
  exactly one `<common-dir>/worktrees/*/gitdir` back-reference equal to
  `<path>/.git`; ambiguous or unreadable → typed refusal. Record the admin
  directory identity before repair.
- [ ] **Targeted repair.** Run `git -C <base> worktree repair <path>` through the
  existing Git helpers from the base checkout (so Windows does not see the target
  held open), noninteractive, no network. Capture stdout/stderr and the spawn
  error, if any.
- [ ] **Verify by postconditions, ignoring exit status.** All must hold:
  - target `.git` resolves to the identified admin directory and its common
    directory is this repository's;
  - the admin `gitdir` back-reference resolves to the target's `.git`;
  - `git rev-parse --show-toplevel` run in the target is the target itself
    (a discovered parent repository is failure);
  - a fresh worktree listing identifies the same target with the same branch or
    detached HEAD and no `prunable`.
  Return `Verified` or a typed `RepairFailed { diagnostics, spawn_error }`; a
  spawn failure is named, and "repaired" is never claimed without verification.
- [ ] **Injectable runner.** Repair and verification go through a seam so tests
  can inject: nonzero exit with valid postconditions (success), zero exit
  without them (refuse), no change, a wrong admin entry in the same repository,
  a foreign repository, parent-repository discovery, and identity change.

### Missing-directory engine

- [ ] **Missing state.** A `CheckoutState { Healthy, Missing, Repaired }`-style
  value carried in `Facts`; an empty `Inventory` for Missing means "no directory
  to inspect", never "checked clean" (distinct type/flag, not an empty vec).
- [ ] **Branch and tip.** Resolve from this repository's refs, or the recorded
  HEAD for a detached entry (no checkout reads).
- [ ] **Surviving index inspection** per R1/R2: metadata-only check using the
  admin index; differing paths are reported as staged paths needing ordinary
  discard consent; inspection failure → refusal.
- [ ] **Record removal.** Immediately before execution re-resolve the listing and
  confirm the target is the same entry and still absent (`symlink_metadata`
  `NotFound`); reappearance of a directory or link → exit 3 asking for a rerun.
  Use only `git -C <base> worktree remove <path>`; no `prune`, no admin-directory
  deletion, no second `--force` (Git's lock protections stand). On Windows skip
  the rename-based lock probe (`check_not_in_use`) only for confirmed absence;
  keep it for existing directories.
- [ ] **Order of effects.** Record removal → copy-record cleanup → approved
  local/remote branch steps. If Git removal fails, no branch deletion and no
  completion claim. Report "directory was already gone, record removed".

**Checkpoint 4:** lib unit tests for repair/association/index inspection pass
using disposable repos; failed/incorrect repair cases refuse and leave files,
records, and branches intact even with every force flag.

## Phase 5: Wire `wt remove`

Wave 4 — three tasks in separate files; the wiring task lands first, the other
two may proceed concurrently.

- [ ] **Prepare before inventory.** In `cli/src/commands/remove/mod.rs`, after
  `find_worktree` and the main-checkout / shell-wrapper guards and the move to
  base, refresh the entry, classify, and branch (healthy → unchanged;
  Missing → missing flow; Unlinked → repair then ordinary `Facts::local`;
  Other → exit 3 with the observed condition). `Facts::local` (line 73) must not
  call `collect_inventory` for Missing.
- [ ] **Repair precedes consent.** Report `restored the link for <name> so its
  files could be checked` only after verification. Inventory must succeed before
  any deletion even with all force flags. When the user declines or a later
  check refuses, leave repaired links in place and say so (no rollback).
- [ ] **Handoff second run.** In `run_handoff` (~714) never auto-repair; re-verify
  the repaired checkout's identity and apply content/index/rules/baseline/branch/
  remote checks. A link changed or broken between runs invalidates approval and
  refuses with exit 3. The fingerprint (`fingerprint`, ~620) must incorporate
  checkout state so a Missing target that reappears cannot reuse approval.
- [ ] **Refusal messages.** Failure text for the confirmed missing-`.git` case
  follows the spec's wording (target, "No working files, branches, or worktree
  records were removed. The repair attempt may have changed Git metadata.",
  inspect and retry steps, no `git worktree prune` advice). Include captured
  repair diagnostics and name any spawn failure. Render via Prose markup and
  escape dynamic text.
- [ ] **Error context.** Add target name, path, and operation to otherwise bare
  Git failures in the remove flow (including the `collect_inventory` failure that
  produced `fatal: not a git repository`), preserving the underlying
  `WorktreeError` category and exit code; keep partial-success reporting when a
  later branch operation fails; never downgrade an environment error to exit 1.
- [ ] **Exit-code contract.** Update the `WorktreeError::RefusedToLoseWork` and
  `BlockedByEnvironment` doc comments in `lib/src/error.rs` and the `exit_code`
  mapping comment in `cli/src/exit.rs`: exit 3 means "nothing removed", not
  "nothing changed"; cancellation stays exit 0; environment/in-use stays exit 4.
  Never print "nothing was changed" after a repair attempt.
- [ ] **Handoff/L2 tests.** Normal approval checks after repair; breaking or
  redirecting the link between runs refuses without another repair. Use the
  existing windowless terminal helpers (`cli/tests/level2_remove.rs`,
  `level2_powershell_remove.rs`) so no window gains focus.

**Checkpoint 5:** `just test` and `just test-l2` pass from `worktree/`; the
spec's L1 groups (missing removal, unlinked removal, failed/incorrect repair,
repair exit status, error context) are all present.

## Phase 6: Docs, skill, and final validation

Wave 5 — documentation tasks are independent by file; run them concurrently.
Write each page for a developer with no experience of this repository, lead with
what the reader can do, give a compact example per rule, and use a Mermaid
diagram for the removal flow. No page names this fix or links to a spec.

- [ ] **`worktree/docs/cli/list.md`:** markers, conditional legends, notes,
  working-file status versus branch merge comparisons, and the `--ff` holder
  refusal.
- [ ] **`worktree/docs/cli/remove.md` (new):** removal safety and handoff rules,
  recovery and refusal examples, missing versus unlinked versus other, the repair
  verification postconditions, the exit-code meaning, and the force-flag limits.
  Include the flowchart from the spec in current-behavior terms.
- [ ] **`worktree/README.md`:** missing/unlinked removal, repair before consent,
  metadata changes on refusal, force limits.
- [ ] **`.claude/skills/worktree/remove.md`:** preparation, exact repair
  verification, missing-directory handling, and the exit-status trap (never trust
  repair's exit code). **`list.md`:** unavailable/unknown states.
  **`cli-contracts.md`:** refusal side effects. Add the new OS facts learned
  (symlink/reparse points, short names) to `.claude/skills/os/` in the same
  change if any were learned the hard way.
- [ ] **Symbol comments:** pass over `dirty_status`, `parse_worktree_list`,
  `WorktreeEntry`, `DirtyStatus`, error docs, and any new public items; fix
  drifted comments and report any drift found and how it was resolved. No
  HOW-narration.
- [ ] **`docs/dependencies.md`:** update only if a crate was added (none expected).

Wave 6 — final validation:

- [ ] `just test`, `just test-l2`, `just lint` from `worktree/`; affected `sniff`
  tests only if `sniff` code changed.
- [ ] Confirm no test touches `/private/tmp/lhg-before` or any live checkout,
  and that none opens or focuses a terminal/browser window.
- [ ] Record in the implementation log which environments were actually
  exercised (the host macOS run; Windows/Linux/WSL2 evidence only if obtained
  via the `os` skill's reachable hosts, otherwise state that CI covers them).
- [ ] Final grep: `unwrap_or(DirtyStatus::Clean)`, `Path::exists()` on classified
  paths, `canonicalize` on missing paths, `--force` appended to record removal,
  `git worktree prune` in remove code or user-facing text.
- [ ] Set the spec `status` to `implemented` only if the spec's own process says
  an agent does so; otherwise leave it and report "implementation complete,
  ready for review". Do not run `just complete`, move the directory, or commit.
