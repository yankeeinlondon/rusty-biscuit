---
status: draft — awaiting decision on the failure mode
created: 2026-08-02
area: darkmatter
packages:
  - darkmatter
---

# A missing runtime value should not render as nothing

## Summary

Darkmatter documents can embed facts about the machine and repository they are
composed on — today's date, the current Git branch, the repository root, the
operating system. These are written as `{{ ctx.something }}`.

Gathering those facts is expensive, so Darkmatter gathers only the ones a
document actually asks for. That is the right design. The problem is what
happens when the gathering step and the document disagree: if a document asks
for a fact that was never gathered, **the value renders as an empty string and
nothing reports a problem**. The document composes successfully, exits zero, and
is quietly missing content.

This document proposes making that disagreement visible.

It is written after a real instance of the failure, described below, which
passed a 7,500-test suite unnoticed.

## Background, for a reader new to this area

Three pieces of vocabulary are needed. Nothing else in this document assumes
prior knowledge.

**A runtime value** is a fact about the world at the moment of composing, rather
than something written in the document. Authors reach them through the `ctx`
namespace: `{{ ctx.today }}`, `{{ ctx.branch }}`, `{{ ctx.repo_root }}`.

**A capture group** is a bundle of related runtime values that are gathered
together because they come from the same source. `ctx.branch` and
`ctx.worktree` are both in the *Git* group. `ctx.repo_root` and `ctx.packages`
are in the *Repo* group. `ctx.today` and `ctx.year` are in the *DateTime* group.
There are ten groups in total, listed in
[`capture/groups.rs`](../../lib/src/markdown/compose/context/capture/groups.rs).

**Capturing** is the act of gathering a group. Cost varies enormously:

| Group | What it does | Cost in this monorepo, on Windows |
|---|---|---|
| DateTime | reads the clock | microseconds |
| Repo | scans the repository for every package | ~530ms |
| Documents | walks the documentation tree | ~585ms |
| FileChanges | runs the equivalent of `git status` | ~679ms |

Capturing all ten groups costs roughly two seconds here. That is why Darkmatter
captures selectively: it scans the document for `ctx.` references first, and
gathers only the groups those references need. A document mentioning no runtime
values costs nothing.

## The defect

Darkmatter has **two separate mechanisms** for answering `ctx.something`, and
they behave differently when the answer was never gathered.

### Path one — the snapshot (used when composing a document)

When a document is composed, the captured values are copied once into a fixed
map, in
[`effective_state.rs`](../../lib/src/markdown/compose/context/effective_state.rs)
(`EffectiveStateBuilder::build`). Every later stage reads from that copy. Nothing
re-reads the original or gathers anything further.

So if the Git group was never captured, `ctx.branch` is simply not a key in the
map. Looking up an absent key yields an empty value, and interpolation renders an
empty value as an empty string. **No warning, no error, no diagnostic.**

### Path two — the lazy resolver (used when evaluating a standalone condition)

A separate type, `CtxLookup` in
[`expression/ctx.rs`](../../lib/src/markdown/compose/expression/ctx.rs), works
the opposite way. When asked for `ctx.branch`, it checks whether the Git group
has been captured, captures it on the spot if not, and returns the value. It can
never be missing a group, because it fetches on demand.

This path is only reachable from `evaluate_condition_against`, a public function
for evaluating a single condition string. It is not used when composing.

### Why having both is the problem

The two paths look interchangeable and are not. Path two is self-correcting;
path one fails silently. Any change to *when* capture happens is safe under path
two and potentially destructive under path one, and nothing in the code marks
which path a given change affects.

## What went wrong on 2026-08-02

While removing an unrelated performance problem, `ComposeOptions::new()` was
changed to stop capturing all ten groups up front. The reasoning was that any
group a document needs would be captured on demand later — which is true of path
two, and false of path one, the one composition actually uses.

The observable result:

```text
composing "repo_root={{ ctx.repo_root }}|os={{ ctx.os }}|today={{ ctx.today }}"

before:  repo_root=C:\Users\ken\rusty-biscuit|os=Windows|today=2026-08-02
after:   repo_root=|os=|today=2026-08-02
```

Two of three values silently disappeared. The date survived because DateTime was
still captured.

**The entire test suite — 7,510 tests — passed.** No test composed a `ctx.*`
value through that constructor, so nothing observed the loss. It was found by
manually comparing output against a full capture, not by any automated check.

The immediate fix (already applied, see *Current state* below) closes this
instance. It does not close the class: the next change to capture timing has the
same silent failure available to it.

## What this proposes

**A `ctx.*` reference whose group was never captured must be reported, not
rendered as nothing.**

Concretely:

1. The value map carries enough information to distinguish *"this group was
   captured and the value is genuinely empty"* from *"this group was never
   captured"*. Today both look identical — an absent key.

2. Composing a document that reads an uncaptured group produces a diagnostic
   naming the key and the group, in the same style as existing compose
   warnings.

3. Whether that diagnostic is a warning or a hard error is the open decision
   below.

4. The two resolution paths are reconciled, so that a reader does not have to
   know which one they are looking at to know whether missing data is possible.

### The open decision: warning or error

**Error** is the stronger position. Rendering a document with silently missing
content is not a lesser outcome than failing to render it — it is worse, because
it looks like success. This mirrors the argument already accepted for
transclusion failures in
[`2026-07-31-error-handling-transclusions`](../2026-07-31-error-handling-transclusions/spec.md),
which inverted the same silent-degradation default.

**Warning** is the safer position for a value that may legitimately be empty on
some machines — `ctx.branch` in a directory that is not a Git repository, for
instance. Under an error policy, a document referencing `ctx.branch` becomes
uncomposable outside a repository.

A likely resolution is to separate the two questions: *the group was never
captured* is a defect and should be an error; *the group was captured and the
value is legitimately absent* is normal and should render empty. That
distinction is exactly what point 1 above makes expressible, and it is currently
impossible to state.

## Alternatives considered

**Always capture everything.** Removes the failure by removing the selectivity.
Costs about two seconds per compose in this monorepo, which is what the
2026-08-02 performance work removed. It also over-invalidates the compose cache:
with the FileChanges group always captured, editing any file anywhere changes
the cache key for every document, including documents that read no runtime
values at all. Rejected.

**Make the snapshot lazy, like path two.** Attractive because it removes the
divergence entirely rather than reporting it. The obstacle is that the snapshot
is deliberately fixed: composition reads `ctx` many times across several stages,
and a fixed snapshot guarantees every read within one compose sees the same
values. A lazily-filled map would let two reads of `ctx.today` straddle midnight
and disagree. Worth revisiting, but it needs its own consistency design and is
larger than this fix.

**Rely on convention.** Document that callers should construct their context via
`capture_for_document`, which reads the document and captures what it names.
This is what the CLI already does and it is why the CLI was never affected. It is
insufficient on its own: it is exactly the convention that was violated on
2026-08-02, and a rule whose violation is silent will be violated again.

## A related question, and a correction

An earlier version of this concern held that the compose **cache key** could go
stale — that a document could resolve `ctx.branch` correctly while the cache key
omitted it, so switching branches would serve stale output.

**On investigation this does not occur, and the reasoning behind it was wrong.**
It assumed composition resolved `ctx.*` through the lazy path. It does not; it
uses the snapshot. Because the cache key is computed from that same snapshot
(`context_hash` in
[`cache/hashing.rs`](../../lib/src/markdown/compose/cache/hashing.rs)), the key
and the rendered output are derived from one source and cannot disagree.

The real behavior is the reverse of a staleness bug: the key covers every group
the document *mentions*, including groups mentioned in a branch that never
executes. That over-invalidates rather than under-invalidates — a performance
consideration, not a correctness one, and out of scope here.

This correction is recorded because the incorrect version was used to argue for
a cache redesign that the evidence does not support.

## Current state

Applied on 2026-08-02, closing the specific instance but not the class:

- `ComposeOptions::new()` captures only DateTime, and the compose pipeline
  upgrades that context to exactly the groups the document names, before any
  stage reads `ctx`
  ([`pipeline/mod.rs`](../../lib/src/markdown/compose/pipeline/mod.rs)).
- `EffectiveStateBuilder::build()` keeps a full capture as its default, because
  a bare builder has no document to narrow against and narrowing there is what
  blanked the values.
- `lib/tests/ambient_ctx_capture.rs` asserts on rendered output that ambient
  options resolve discovery-backed values identically to a full capture. It was
  confirmed to fail without the pipeline upgrade.

That test covers one constructor on one path. It does not prevent a future
change from reintroducing the silent-empty behavior elsewhere, which is what
this document exists to address.

## Verification

The fix is complete when all of the following hold:

1. A document referencing a group that was never captured produces a diagnostic
   naming the key and its group — asserted directly, not inferred from output.
2. A document referencing a group that *was* captured, whose value is genuinely
   absent, renders empty and produces no diagnostic.
3. The two cases above are distinguishable in a test without reading source.
4. Removing the pipeline upgrade causes a failure that names the missing group,
   rather than an output-comparison mismatch.
5. A document referencing no runtime values still captures only DateTime, and
   the full darkmatter L1 suite shows no test above the 5s slow threshold.

## Out of scope

- Making the snapshot lazy (see *Alternatives*).
- Narrowing the cache key from mentioned groups to consumed groups.
- Reducing the cost of any individual capture group; the expense is inherent to
  walking a repository this size on this filesystem.
