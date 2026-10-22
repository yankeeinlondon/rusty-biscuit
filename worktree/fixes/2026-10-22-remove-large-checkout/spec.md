---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: "2026-10-22"
review_iterations: 2
completed: true
implemented: true
related:
    - 2026-10-05-graceful-file-contention
    - 2026-10-18-remove-observation-under-load
implemented_by: "claude/opus"
---

# `wt remove` cannot finish a built checkout in one run

## Outcome

`wt remove` of a checkout that has been built (a `target/` directory of
hundreds of thousands of entries) finishes in one run, and its usage check
still looks for processes using the checkout.

Success means:

- One `wt remove` of a 500,000-entry checkout, with nothing else wrong,
  removes the checkout and its registration, then performs the approved
  branch actions. A branch approved for deletion is deleted. No retry.
- A removal can no longer stop halfway because of the journal's size. When
  a free-space reading shows that the journal cannot fit, `wt remove`
  refuses **before the move**, leaving the checkout, registration, and branch
  intact. This check does not reserve space or prevent later disk-full errors.
- On such a checkout, the usage check reuses the captured inventory instead
  of spending its budget walking the tree again. It spends the remaining
  budget on the process checks supported by the host. Coverage still records
  permissions, unsupported mechanisms, and unfinished inspection; complete
  discovery is not guaranteed.

## What happened

On 2026-10-22 three removals of merged worktrees stopped halfway, each with
the same error:

```text
Removal stopped: a step failed (the deletion journal reached its 67108864-byte limit).
```

| Removal | Entries | Deleted before the stop | `wt` build |
| --- | --- | --- | --- |
| `feat/modern-config`, attempt 1 | about 354,000 | 136,217 | report version 4 (stale) |
| `feat/modern-config`, attempt 2 | about 218,000 | 329,174 (attempts 1–2) | report version 4 (stale) |
| `feat/osc-9-4-protocol`, attempt 1 | about 441,000 | 135,818 | report version 5 (current `main`) |

`feat/modern-config` took three runs to remove. `feat/osc-9-4-protocol` was
left half-removed, with 305,230 entries in its hidden directory. Both
checkouts were mostly `target/` (about 92 GB in `feat/modern-config`).

Every one of these runs also reported, before the move:

```text
Discovery could not run; ...; the checkout's files were only partly read;
the process list, open handles and working directories were not checked;
the check ran out of time.
```

The stop message then printed the same sentence a second time for the moved
directory.

The retries of `feat/modern-config` also asked for consent for 472, and then
7,191, "uncommitted files". These were all tracked files that earlier attempts
had deleted (` D` in `git status`). That build predated
`2026-10-18-remove-observation-under-load`, which already excludes an
operation's own confirmed deletions from resume consent. This was not a new
defect. Verification below adds a large-checkout regression for it.

## Causes

### The journal limit is a fixed constant, sized from a wrong estimate

`DEFAULT_JOURNAL_LIMIT` in the `worktree` library's
[journal](../../lib/src/remove/recovery/journal.rs) is 64 MiB per attempt.
It counts only batch lines (intents and results), not the header or the
inventory line. `begin_batch` refuses an intent unless the worst-case results
line still fits, which ends deletion with `LimitReached`.

The 2026-10-05 implementation estimated about 150 bytes of batch lines per
entry, so 64 MiB would cover about 440,000 entries. It deferred "sizing the
limit from the inventory" because no test checkout came close. Measured on
`feat/osc-9-4-protocol`:

| Quantity | Measured |
| --- | --- |
| Batch-line bytes per entry | 494 |
| Serialized path per entry (`{"display":"…"}`) | 221 bytes on average |
| Entries that fit in 64 MiB | about 136,000 |

Each entry's path is written twice, once in the intent and once in the
results, and paths under `target/` are long. For example:

```text
target/ci-local/debug/.ci-build-wrappers/ci-build-archive-fixture/18c405a3957c66fa/target/aarch64-apple-darwin/debug/incremental/archive_portability-0izlkn6cqxb9d/s-hnbzb0q487-11n4dm7-9iujb8myp09rv1chj62wso2uo/0vvmqex7xawj85zvfspfz6rox.o
```

So the real capacity is about a third of the estimate, and an ordinary
built checkout exceeds it. A retry archives the journal and starts a new one
with a fresh 64 MiB, so every removal finishes eventually. Each attempt costs
the user a run, a long error message, and a half-removed checkout in between.

The deletion engine plans each inventory entry at most once per attempt.
The writer validates membership and duplicates within a batch, but does not
itself enforce uniqueness across batches. Given the engine's invariant,
the largest journal an attempt can write is computable before it starts.
Keep that invariant explicit in the shared batch planner and its tests.

### The usage check spends its whole budget walking the tree

The decisive usage check ([usage.rs](../../lib/src/remove/usage.rs)) calls
Sniff's `query_path_usage` with Sniff's default two-second budget. Sniff
walks the target tree first, to build the identity index it matches open
handles against, and only then enumerates processes. On `feat/modern-config`
the walk read 206,342 entries in 2.0 s and exhausted the budget:

| Mechanism | Status |
| --- | --- |
| `tree_identity` | partial (budget expired) |
| `process_enumeration` | not attempted |
| `open_handles` | not attempted |
| `working_directories` | not attempted |

Policy treats "no usage observed" as clear whatever the coverage, so the
removal went ahead. On any large checkout the check that refuses to delete
files a running process holds is effectively never run. Those are the
checkouts most likely to have a build or language server still running in
them.

`wt` has already walked the same tree. Step 2 of
[remove_checkout](../../lib/src/remove/lifecycle.rs) captures the
`DeletionInventory`, with each entry's identity, before step 3 runs the
usage check.

## Decisions

### 1. Size the journal limit from the attempt's inventory

Replace the fixed `DEFAULT_JOURNAL_LIMIT` with a limit computed when an
attempt's journal is started (the `worktree` library's
[RecoveryStore::begin and reopen](../../lib/src/remove/recovery/store.rs),
which create the initial journal and its retry replacement). The limit is
the batch-line bytes the attempt would need if it deleted every entry of its
inventory:

- Share the batch-planning logic with the `worktree` library's
  [deletion engine](../../lib/src/remove/observation/deletion.rs): ordinary
  content first, `.git` entries last; within each group, deepest entries
  first, then directory order, then chunks of at most `MAX_BATCH_ENTRIES`.
  Keep the existing entry order within each directory. Grouping by parent
  alone would incorrectly merge the root's content and `.git` batches.
- For each batch, add the intent line's bytes and the worst-case results
  line's bytes. Use the same serializer and the same worst-case state that
  `begin_batch` reserves today (`ExternallyAbsent`). Do not use a
  per-entry estimate.
- Serialize each batch with its actual consecutive number starting at one,
  including the newline. Use checked arithmetic for lengths and totals;
  overflow refuses before the move or before resumed deletion.
- Count serialized bytes without retaining a second full journal in memory.
  A counting writer or one temporary batch buffer is sufficient. This pass
  uses inventory data only: no filesystem walk or content reads.

A retry sizes its limit from that attempt's fresh inventory of surviving
entries, as it already captures one.

The bound is the sum of worst-case reservations, not the exact byte count
of an all-`Deleted` journal: `ExternallyAbsent` serializes longer. The
checkout root itself is not a journaled inventory entry. A shorter result
line or an early stop may leave unused capacity.

Keep `LimitReached` and the reserve check in `begin_batch`. With a computed
bound they can only fire because of a defect in `wt` (for example, a batch
grouping that differs from the computation). In that case the stop message
says that `wt` reached an internal limit and that a retry continues, rather
than presenting a byte count as the user's problem.

Exposing the limit as a constant or option is not needed. Tests that
exercise `LimitReached` pass an explicit small limit to `JournalWriter::start`,
as they do today.

### 2. Refuse before the move when the journal cannot fit

The journal lives under the repository's common Git directory, possibly on a
different volume from the checkout. Before publishing a new attempt, compare
available space on the recovery store's filesystem with the additional bytes
that attempt can require:

- First attempt: journal header, inventory line, computed batch bound, and
  the initial recovery record.
- Retry: the new journal's header, fresh inventory line, and batch bound,
  plus the previous journal's full length if a new archive copy is needed.
  Existing archives already consume disk space and are not counted twice.
  Check before archiving or creating the replacement; preserve the current
  journal and all earlier evidence on refusal.

Use the same serialization for counting and writing. This is a check of the
known journal and initial record writes, not a promise that future report or
filesystem metadata writes will fit. Do not credit space deletion may free:
the checkout may be on another filesystem, and hard links may retain blocks.

**Reader's note:** Sniff's existing storage inventory enumerates devices and,
on macOS, probes device kinds with `diskutil`. That is unnecessary for one
free-space reading and its filtered mount list is not a reliable way to select
the filesystem containing an arbitrary path. Add a focused path-based
available-space query to Sniff instead, using native filesystem APIs and no
subprocess or full host discovery. Resolve the actual recovery-store path,
including ancestor aliases; do not select a volume by string prefix. Report
space available to the calling user, rather than total unallocated space.

Insufficient space or an unavailable space reading is an environmental block
(exit 4), represented through the existing storage-refusal path. Name the
recovery-store path, the space needed, and the space available; for a failed
reading, state why availability could not be established. On a first attempt,
no operation directory is published and the checkout, registration, and branch
remain intact. Opening the private store and taking its lock may already have
created bookkeeping; do not claim the entire store is unchanged. On retry,
the checkout remains at its existing moved path and no further deletion occurs.

A free-space reading is not a reservation: another writer, quota, or allocation
failure can still prevent a later write. Preserve today's durable intent and
results ordering, poison the writer on a write failure, and stop deletion.
Only durably recorded results are confirmed; a torn results line retains the
existing uncertain-entry recovery behavior. No preallocation is required.

For scale, `feat/osc-9-4-protocol`'s inventory line was 167 MB and its full
batch bound would be about 218 MB. These measurements already establish why
64 MiB is insufficient; no performance spike is needed.

### 3. Let any caller supply the tree index Sniff would walk

A caller that has already walked a tree should not pay for the walk again
inside Sniff's budget. `wt` is the first such caller. Others are likely: any
library that inventories, hashes, copies, or deletes a tree before asking who
is using it.

So this is a general Sniff capability, not a `wt`-specific hook:

- The `sniff` library exposes a public form of its existing
  [TreeIndex](../../../sniff/lib/src/filesystem/query/tree.rs), which maps
  file identities to all their in-scope paths. Any caller can build it: the
  target root's identity plus the relative path, kind, and identity of each
  entry beneath it. It is built from values the caller already has, through a
  small constructor or builder, with no dependency on any caller's types.
- `PathUsageOptions` gains an option that takes this index. When it is set,
  `query_path_usage` does not walk the tree. Root verification and final
  consistency checks remain within the budget, and the time saved is
  available to process enumeration and inspection. Build and validate the
  reusable index before starting the query; do not replace the walk with
  an equally large per-query conversion or clone.
- `wt` converts the `DeletionInventory` it captured in step 2 into that
  index and passes it to the step 3 check, and does the same with the fresh
  surviving inventory on a quarantined retry. The Windows Search recheck
  reuses that attempt's index and still verifies the root on each query.
  The budget and removal policy do not change.

Requirements on the Sniff side:

- The index must describe the same root Sniff verifies. Sniff still resolves
  and verifies the root itself, and it refuses an index whose root identity
  differs, as an invalid-option `PathUsageError`. Constructors reject
  absolute paths, parent traversal, duplicate relative paths, and descendants
  below a supplied link. Preserve native names without lossy conversion,
  every hard-link alias, and link identities without following targets.
  A supplied descendant index is accepted only for a recursive directory
  query; reject combinations with `target_only` or a file root.
- Matching handles against a supplied index must behave exactly as against a
  walked one. Sniff's own walk produces the same type, so there is one index
  representation and one matcher. Preserve the regular-file/directory probe
  entries used by the Windows file-side holder scan; never probe supplied
  links as their targets. Supplying an index does not eliminate that scan,
  which can itself exhaust the budget on a large checkout. Windows working
  directory discovery remains unsupported.
- The report records where the index came from (`walked` or `supplied`).
  A supplied index is reported as the caller's coverage, so a partial or stale
  index cannot pass as a complete Sniff walk. A caller may mark its index
  partial, and Sniff reports that coverage as partial. Provenance and
  completeness are separate: a caller-declared complete capture may have
  complete tree coverage, but Sniff must not claim to have verified its
  descendants. Keep process-mechanism coverage and outcome library-owned.
- The index describes the tree when the caller captured it, so entries
  created later are not in it. The rustdoc and Sniff's docs state this. A
  caller like `wt`, which watches the tree from before the capture, closes
  that gap for deletion through its existing event drain and inventory
  confirmation; it does not make the usage query atomic. Keep those checks
  after index construction and after journal sizing. Other callers must
  decide how to handle concurrent changes.

After-stop diagnostic queries keep walking the current leftovers: the
pre-deletion index must not be reused as if it described the remaining tree.
Extend the `worktree` library's [UsageSource](../../lib/src/remove/usage.rs),
the injected source of process reports, so tests can verify that preflight
receives an index while diagnostics do not. Keep registered observer-handle
exclusions and the Windows Search recheck policy intact.

The Sniff change follows that package area's own conventions and docs, and
lands with its own tests in this fix.

### Serialized report compatibility

The supplied-index provenance changes Sniff's serialized usage-report shape.
Increment its schema version and include an explicit unknown provenance when
reading a report written before provenance existed; never infer `walked` from
an absent field. Preserve the existing explicit-null convention for optional
fields. Update Sniff's CLI JSON contract, schema fixtures, and round-trip tests.

The `worktree` library embeds these reports in durable recovery reports.
Increment its report version and retain support for previously accepted report
versions, including version 4 and 5 pending removals. Old journals retain their
current format and version; the computed limit is attempt-local and is not a
new journal field. Test loading older reports and journals and then resuming
with the newly sized journal. Older binaries need not read newly written
reports; document that direction of compatibility.

### 4. Say a repeated discovery result once

When the after-stop discovery summary for the moved directory says the same
thing as the before-move summary, the stop message prints it once and says it
applies to both paths. Different summaries are printed separately, as today.
Compare the rendered coverage notes, not whole reports (timestamps and paths
differ). Merge only the current attempt's preflight and moved-path diagnostic
notes, and keep their before/after timing explicit: identical limitations do
not mean the two queries were one observation. Do not merge earlier-attempt
notes, failed-query messages, or holder evidence. Preserve all discovery runs
in the saved report.

This is a rendering change in the `worktree-cli` package's
[recovery messages](../../cli/src/commands/remove/recovery.rs), using the
existing `biscuit-terminal` components.

## Non-goals

- A more compact journal format (for example, results as a state array
  without repeating paths). It would halve the journal but needs journal
  version 3. With Decision 1 the size no longer stops a removal, so this is
  not needed to fix the defect.
- Reducing `report.json`'s size. It was 107 MB for `feat/modern-config`.
  Reports already truncate deleted-entry listings (`deleted_omitted`).
- Guaranteeing complete process discovery, or adding support for mechanisms
  an OS currently cannot inspect.
- Changing Sniff's default budget, or the removal policy that a check with
  partial coverage still continues.
- Speeding up deletion.

## Verification

Use nextest through each package area's `just test`, following the Test
Toolkit and the repository's `rust-testing` skill. The automated checks below
are L1 and must stay within its five-second per-test limit. No test opens or
focuses a terminal or browser window. Use synthetic inventories and counting
sinks for volume; small real trees prove lifecycle wiring.

- **Computed bound (worktree):** cover short and long paths, JSON escaping,
  native non-UTF-8 names, hard links, empty inventories, many small directories,
  more than `MAX_BATCH_ENTRIES` siblings, `.git` separation, and batch-number
  digit transitions. Exercise native filename encodings on their applicable
  platforms; do not require APFS to create invalid UTF-8 names. All planned
  batches fit the bound for every result state. Replaying all batches with
  worst-case `ExternallyAbsent` results fits exactly; a one-byte-smaller limit
  rejects the final reservation. An all-`Deleted` journal need not fill the
  bound exactly. Checked-size overflow refuses without writes.
- **Former size ceiling (worktree):** generate a synthetic inventory whose
  serialized batch bound exceeds 64 MiB. Stream batches through a counting
  sink rather than retaining the entire log or creating hundreds of thousands
  of files. Assert there is no `LimitReached` and that emitted bytes stay
  within the bound. Keep serialization work bounded enough for L1.
- **Removal and resume wiring (worktree):** use small real checkouts with
  injected observers and usage reports to prove the initial and resumed
  writers receive their inventory-derived limits. Stop after a confirmed
  tracked-file deletion, then resume without `discard_files`; the operation's
  own confirmed deletions require no renewed consent. Leave `.git` until last
  and perform only the approved branch actions.
- **Free-space refusal (worktree):** inject insufficient and unavailable
  readings. Before the first move, assert checkout, registration, and branch
  are intact and no pending operation was published. On retry, assert no
  further deletion, archive, or replacement was written and prior recovery
  evidence still reads. Cover the previous journal's archive-copy cost and
  the already-identical archive case. A sufficient reading followed by an
  injected write failure still follows existing crash-recovery rules.
- **Focused space query (sniff):** test path-to-filesystem resolution and
  errors through the platform seams, including ancestor aliases and a store
  on a different filesystem from the checkout. Prove it invokes neither
  storage-device inventory nor subprocesses.
- **Supplied tree index (sniff):** use the existing fake usage backend and
  performance counters to prove zero tree walks and zero descendant identity
  reads for index construction inside a query. Root verification remains.
  Walked and supplied indexes of the same tree find the same holders and
  hard-link aliases. Cover invalid paths, wrong root identity, partial
  coverage, invalid target scope, links, and Windows probe-entry behavior.
  Unsupported mechanisms remain unsupported; budget exhaustion during process
  inspection remains visible.
- **Usage handoff (worktree):** verify first-attempt preflight, resumed
  preflight, and the Windows Search recheck receive the appropriate index.
  Diagnostics use a fresh query of leftovers. With deterministic budget and
  backend seams, process enumeration is reached without spending time on a
  tree walk; do not assert full coverage from the live host's process load.
- **Compatibility (sniff and worktree):** round-trip new provenance and load
  old reports with unknown provenance; resume accepted old pending removals
  without losing earlier reports or confirmed deletion results.
- **Stop message (worktree-cli):** identical current-attempt coverage notes
  render once with both time/path contexts; different notes render separately.
  Earlier attempts, failed queries, and holder evidence are not collapsed.

The 500,000-entry outcome is the real-checkout acceptance case when an
appropriate disposable built checkout is available, not a mandatory huge
filesystem fixture in every L1 run. The serializer regression above proves
the former size ceiling without a timing benchmark or additional hosts.

Cross-platform verification covers `worktree`, `worktree-cli`, and `sniff` on
macOS, Linux, native Windows, and WSL2 under the repository's existing CI and
evidence rules.

## Documentation

- `worktree/docs/cli/remove.md`: the journal no longer limits a removal; the
  free-space refusal and its message; the usage check's coverage on large
  checkouts; one discovery summary when both match.
- `sniff/docs/topics/filesystem-query.md`: the tree index type, how a caller builds and supplies one,
  its root requirement, its point-in-time limit, and how the report marks
  it, including old-report compatibility and unchanged Windows limitations.
  Lead with the use case: a caller that has already walked the tree.
- Sniff's storage API docs: the focused available-space query, its error
  behavior, and why a reading is not a reservation.
- `worktree/README.md`: briefly mention support for built checkouts and link
  to the removal topic; keep detailed rules in `docs/`.
- `.claude/skills/worktree/remove.md` and `lifecycle.md`: the computed
  journal limit (replacing "the size limit counts batch lines only" as the
  whole story) and the inventory passed to the usage check.
- `.claude/skills/sniff/`: when a library should supply its own index, so
  future callers that walk a tree first find the option.

## Open Questions

None remain from this review. The decisions above preserve the existing
advisory usage policy and crash-recovery rules; implementation should not
turn either into a new guarantee of complete discovery or reserved disk space.

## Decided Alternatives

**Sniff reserving part of its budget for processes.** Sniff would enumerate
processes first, walk the tree with the time left, and match handles inside
the target by path when the index is incomplete. Not chosen: identity
matching (hard links, firmlinks, junctions) would become partial exactly when
the tree is large. A caller-supplied index keeps full matching for any
caller that has walked the tree. Callers that have not are unchanged.
