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
reviewed: false
review_iterations: 0
implemented: false
related:
    - 2026-10-05-graceful-file-contention
    - 2026-10-18-remove-observation-under-load
---

# `wt remove` cannot finish a built checkout in one run

## Outcome

`wt remove` of a checkout that has been built (a `target/` directory of
hundreds of thousands of entries) finishes in one run, and its usage check
still looks for processes using the checkout.

Success means:

- One `wt remove` of a 500,000-entry checkout, with nothing else wrong,
  removes the checkout, its registration, and its branch. No retry.
- A removal can no longer stop halfway because of the journal's size. When
  the journal cannot fit on disk, `wt remove` refuses **before the move**,
  so nothing has changed.
- On such a checkout, the usage check enumerates processes and inspects
  their open handles and working directories before anything is moved.

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

The limit protects nothing that the inventory does not already bound. A
batch may plan only inventory entries, and each entry is deleted once, so
the largest journal an attempt can write is computable before it starts.

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
attempt's journal is started (`RecoveryStore::begin`, and the retry path that
reopens a journal). The limit is the batch-line bytes the attempt would need
if it deleted every entry of its inventory:

- Group the inventory's entries the way the deletion engine batches them:
  one directory per batch, at most `MAX_BATCH_ENTRIES` per batch.
- For each batch, add the intent line's bytes and the worst-case results
  line's bytes. Use the same serializer and the same worst-case state that
  `begin_batch` reserves today (`ExternallyAbsent`). Do not use a
  per-entry estimate.
- Batch numbers grow with each batch, so compute their digits from the
  attempt's batch count, not from a constant.

A retry sizes its limit from that attempt's fresh inventory of surviving
entries, as it already captures one.

Keep `LimitReached` and the reserve check in `begin_batch`. With a computed
bound they can only fire because of a defect in `wt` (for example, a batch
grouping that differs from the computation). In that case the stop message
says that `wt` reached an internal limit and that a retry continues, rather
than presenting a byte count as the user's problem.

Exposing the limit as a constant or option is not needed. Tests that
exercise `LimitReached` pass an explicit small limit to `JournalWriter::start`,
as they do today.

### 2. Refuse before the move when the journal cannot fit

The journal lives in the repository's `.git` directory, possibly on a
different volume from the checkout. Before `RecoveryStore::begin` writes
anything, compare the free space of the volume holding the recovery store
with the journal's expected total: header, inventory line, and the computed
batch bound. Get free space through Sniff (its storage discovery in
`sniff/lib/src/hardware/storage.rs` already reads it), never from a `df`
process.

If it does not fit, refuse with a `Refusal`: nothing is moved, and the exit
class is the one other pre-move refusals already use. The planner picks the
specific class from the existing `Refusal` mapping. Name the volume, the space needed, and the
space free. A short free-space read is not a guarantee: another writer can
still fill the disk. A later write error keeps today's behavior: the journal
is poisoned and deletion stops with every confirmed result kept.

For scale, `feat/osc-9-4-protocol`'s inventory line was 167 MB and its full
batch bound would be about 218 MB.

### 3. Let any caller supply the tree index Sniff would walk

A caller that has already walked a tree should not pay for the walk again
inside Sniff's budget. `wt` is the first such caller. Others are likely: any
library that inventories, hashes, copies, or deletes a tree before asking who
is using it.

So this is a general Sniff capability, not a `wt`-specific hook:

- Sniff gains a public tree index type that any caller can build: the
  target root's identity plus the relative path, kind, and identity of each
  entry beneath it. It is built from values the caller already has, through a
  small constructor or builder, with no dependency on any caller's types.
- `PathUsageOptions` gains an option that takes this index. When it is set,
  `query_path_usage` does not walk the tree, and the whole budget goes to
  process enumeration and inspection.
- `wt` converts the `DeletionInventory` it captured in step 2 into that
  index and passes it to the step 3 check. This makes the check's coverage on
  a built checkout the same as on a small one, with no change to the budget
  or to the removal's usage policy.

Requirements on the Sniff side:

- The index must describe the same root Sniff verifies. Sniff still resolves
  and verifies the root itself, and it refuses an index whose root identity
  differs, as a `PathUsageError`.
- Matching handles against a supplied index must behave exactly as against a
  walked one. Sniff's own walk produces the same type, so there is one index
  representation and one matcher.
- The report records where the index came from (`walked` or `supplied`).
  A supplied index is reported as the caller's coverage, so a partial or stale
  index cannot pass as a complete Sniff walk. A caller may mark its index
  partial, and Sniff reports that coverage as partial.
- The index describes the tree when the caller captured it, so entries
  created later are not in it. The rustdoc and Sniff's docs state this. A
  caller like `wt`, which watches the tree from before the capture, closes
  that gap itself; other callers must decide whether they need to.

The Sniff change follows that package area's own conventions and docs, and
lands with its own tests in this fix.

### 4. Say a repeated discovery result once

When the after-stop discovery summary for the moved directory says the same
thing as the before-move summary, the stop message prints it once and says it
applies to both paths. Different summaries are printed separately, as today.
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
- Changing Sniff's default budget, or the removal policy that a check with
  partial coverage still continues.
- Speeding up deletion.

## Verification

Use nextest through each package area's `just test`. Everything below is L1.
No test opens or focuses a terminal or browser window.

- **Computed bound is exact (worktree, L1):** for inventories with short,
  long, and non-UTF-8 paths (Unix bytes and Windows UTF-16, through
  `NativeString`), many small directories, and one directory with more than
  `MAX_BATCH_ENTRIES` entries, deleting every entry fits the computed limit.
  Deleting with a limit one byte smaller fails with `LimitReached`.
- **Large-checkout removal (worktree, L1):** a generated checkout whose batch
  lines exceed 64 MiB is removed in one `remove_checkout`, with the injected
  observer and usage source. Generate long paths rather than 500,000 files
  so the test stays fast. A file count is not what triggered the defect;
  bytes per entry were.
- **Large-checkout resume (worktree, L1):** stop such a removal partway
  (injected fault), then resume without `discard_files`. No consent is asked
  for the tracked files the first attempt deleted. This is the regression for
  the stale-binary symptom.
- **Free-space refusal (worktree, L1):** with an injected free-space reading
  smaller than the expected journal, removal refuses before the move. The
  checkout, its registration, and the store are unchanged.
- **Supplied tree index (sniff, L1):** with a supplied index, the query does
  not walk the tree, reports the index as `supplied`, and still finds a holder
  of a file inside the target. An index for another root identity is refused.
  An index marked partial is reported as partial coverage. A walked index and
  a supplied index of the same tree give the same matches.
- **Usage coverage (worktree, L1):** the step 3 query receives the step 2
  inventory. A report from a large generated checkout has process
  enumeration attempted.
- **Stop message (worktree-cli, L1):** identical before and after discovery
  summaries render once; different ones render twice.

Cross-platform verification covers `worktree`, `worktree-cli`, and `sniff` on
macOS, Linux, native Windows, and WSL2 under the repository's existing CI and
evidence rules.

## Documentation

- `worktree/docs/cli/remove.md`: the journal no longer limits a removal; the
  free-space refusal and its message; the usage check's coverage on large
  checkouts; one discovery summary when both match.
- `sniff/docs/`: the tree index type, how a caller builds and supplies one,
  its root requirement, its point-in-time limit, and how the report marks
  it. Lead with the use case: a caller that has already walked the tree.
- `.claude/skills/worktree/remove.md` and `lifecycle.md`: the computed
  journal limit (replacing "the size limit counts batch lines only" as the
  whole story) and the inventory passed to the usage check.
- `.claude/skills/sniff/`: when a library should supply its own index, so
  future callers that walk a tree first find the option.

## Decided Alternatives

**Sniff reserving part of its budget for processes.** Sniff would enumerate
processes first, walk the tree with the time left, and match handles inside
the target by path when the index is incomplete. Not chosen: identity
matching (hard links, firmlinks, junctions) would become partial exactly when
the tree is large. A caller-supplied index keeps full matching for any
caller that has walked the tree. Callers that have not are unchanged.
