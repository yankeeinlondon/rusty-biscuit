---
status: draft — awaiting naming and migration decisions
created: 2026-07-31
area: darkmatter
packages:
  - darkmatter
  - darkmatter-cli
amends:
  - ../../../biscuit-file/features/2026-07-31-portable-strings/spec.md
---

# Transclusion failures should be errors by default

## Summary

A transclusion that fails today does not fail the compose. The engine records a
warning, substitutes something for the directive, and returns `Ok`. The caller
gets an exit code that says the document composed, a document that is missing
content, and a warning on stderr that is easy to lose in a pipe.

This inverts the default: **a transclusion failure is an error**. Suppressing it
becomes an explicit, per-invocation choice via a CLI switch or an environment
variable, and choosing it yields exactly the behavior shipped on 2026-07-31 —
a visible failure notice in the directive's place plus the existing warning.

Nothing about *how* a tolerated failure renders changes. Only who has to ask for
it, and what happens when nobody does.

## Today's behavior

### The tolerance model has three unrelated switches

Transclusion failure handling is currently decided by three mechanisms that were
added at different times, are read at different points, and do not compose.

**1. `fail_fast` — a `ComposeOptions` boolean, default `false`.**

Checked once, at
[`phases.rs:387`](../../lib/src/markdown/compose/pipeline/phases.rs#L387). When
`true`, any resolution error returns from the transclusion phase unchanged. It
is a library option with **no `md compose` flag**; the only CLI surface is
`md validate --fail-fast`
([`args/target.rs:32`](../../cli/src/args/target.rs#L32)), which is a different
command with a different meaning ("stop on first error" while validating).

**2. `ignore_invalid` — a three-tier lookup, default `false`.**

Resolved by `resolve_ignore_invalid`
([`engine.rs:1689`](../../lib/src/markdown/compose/transclusion/engine.rs#L1689)),
in precedence order:

1. `ComposeOptions::ignore_invalid_references: Option<bool>`;
2. the document's own `ignore_invalid` frontmatter key;
3. the `IGNORE_INVALID` environment variable, read from the *snapshot*
   environment (`options.context().env()`), not `std::env`.

It gates only **target-resolution** failures — a `::file` whose path does not
resolve — at
[`engine.rs:622`](../../lib/src/markdown/compose/transclusion/engine.rs#L622)
(block directives) and
[`engine.rs:803`](../../lib/src/markdown/compose/transclusion/engine.rs#L803)
(frontmatter `prologue`/`epilogue` references). When on, the directive is
replaced with an **empty string** and a warning is recorded. When off, the error
propagates.

Like `fail_fast`, it has **no `md compose` flag**.
`--allow-missing-transclusions` looks like one but is not: it feeds
`ComposeAllowFlags` ([`commands/mod.rs:134`](../../cli/src/commands/mod.rs#L134)),
which filters the *reference-validation report*, and never reaches
`ComposeOptions::ignore_invalid_references`.

**3. The structural allowlist — hardcoded, not configurable.**

At [`phases.rs:377`](../../lib/src/markdown/compose/pipeline/phases.rs#L377),
three `TransclusionError` variants always propagate regardless of the other two
switches:

- `CycleDetected`
- `MaxDepthExceeded`
- `RemoteFetchFailed`

Everything else — including any error raised by the child document's own compose
pipeline — is tolerated by default.

### What each failure point does

| Failure | Decided at | Default outcome | Directive becomes |
|---|---|---|---|
| Target does not resolve | `engine.rs:622`, `engine.rs:803` | **error** | — |
| Target does not resolve, `ignore_invalid` on | same | warning | empty string |
| `::file-links` matched nothing | `engine.rs:1313` | warning | empty string |
| `::file-links` matched nothing, `fail_fast` on | same | *not an error* | `_No matching files_` |
| Cycle / max depth / remote fetch | `phases.rs:377` | **error** | — |
| Child document's compose fails | `phases.rs:376` | warning | failure notice |
| Any of the above, `fail_fast` on | `phases.rs:387` | **error** | — |

Two things stand out. `ignore_invalid` and `fail_fast` gate overlapping
failures at different layers with no defined interaction. And `::file-links`
uses `fail_fast` inverted from everywhere else: strict mode inserts a
placeholder and keeps going, because an empty match is not actually a failure —
it is reusing the flag to mean "annotate rather than delete".

### What the 2026-07-31 change did and did not fix

The portable-strings review surfaced that a child compose failure left the
authored `::file child.md` line **literally in the composed output**, because
the apply loop's `continue` skipped recording a replacement and therefore never
overwrote the span. Directive syntax reached the reader, and for HTML and
browser targets it rendered as a paragraph of engine internals.

That is now fixed: `PreparedTransclusion::failure_anchor`
([`engine.rs:308`](../../lib/src/markdown/compose/transclusion/engine.rs#L308))
captures the span and a per-kind notice before the value is consumed, and the
apply loop substitutes it. `fit_notice_to_span`
([`phases.rs:505`](../../lib/src/markdown/compose/pipeline/phases.rs#L505))
reproduces the directive's indentation and trailing newline so a notice inside a
list item does not unnest the container.

It deliberately did **not** touch the default. A composed document that is
missing a section still exits `0`.

### Why the default is wrong

- **The exit code lies.** A build step, a CI job, or a script that pipes
  `md compose` into a publisher has no way to distinguish a complete document
  from one with a hole in it without parsing stderr. Exit status is the one
  channel every one of those callers already reads.
- **Silence scales badly.** One missing section in an interactive run is
  obvious. Fifty documents composed in a loop, each with one tolerated failure,
  produce fifty warnings nobody reads and fifty published pages with gaps.
- **The tolerant path is the unusual case.** Authoring a `::file` directive is
  a statement that the content belongs there. A missing target is nearly always
  a typo, a moved file, or a broken generation step — not an intent to publish
  without it.
- **The switches are unreachable from the CLI.** Neither `fail_fast` nor
  `ignore_invalid_references` has an `md compose` flag, so the only way a CLI
  user changes this behavior at all is a frontmatter key or an environment
  variable most of them do not know exists. A default that cannot be overridden
  from the command line is not a default, it is a hardcoded policy.
- **Tolerance is currently invisible in the artifact's provenance.** The
  document does not record that it was composed in a degraded state, so a file
  written to disk carries no evidence of the gap beyond the notice text itself.

## Proposed behavior

### 1. A transclusion failure is an error

Remove the tolerate-by-default branch. Every failure reaching
`phases.rs:376` — resolution, child compose, code render, remote — returns
`Err` from the transclusion phase and out of `compose`. The structural allowlist
becomes redundant and is deleted: everything is structural now.

`::file-links` matching nothing stays **not a failure**. It is a legitimate
empty result and keeps its current strict-mode placeholder behavior; it must not
be swept into the new error path.

### 2. One switch suppresses it, on two surfaces

Suppression is a single concept with a single resolved value, reachable from
both a flag and the environment. When on, behavior is byte-for-byte what ships
today after the 2026-07-31 change: warning recorded, directive replaced with its
notice, compose returns `Ok`.

Proposed names, following the repo's existing conventions:

| Surface | Name | Precedent |
|---|---|---|
| CLI | `md compose --allow-failed-transclusions` | the `--allow-missing-*` family in [`args/command.rs:152-166`](../../cli/src/args/command.rs#L152) |
| Environment | `DARKMATTER_ALLOW_FAILED_TRANSCLUSIONS` | `DARKMATTER_REMOTE_CONCURRENCY` ([`args/command.rs:213`](../../cli/src/args/command.rs#L213)) |
| Frontmatter | `allow_failed_transclusions` | the existing `ignore_invalid` key |
| Library | `ComposeOptions::with_allow_failed_transclusions(Option<bool>)` | `with_ignore_invalid_references` |

Precedence, highest first — matching `resolve_ignore_invalid`'s existing shape
so there is one rule to learn:

1. the library option, when `Some`;
2. the document's frontmatter key;
3. the environment variable, read from the snapshot environment;
4. otherwise `false` — the new strict default.

The environment variable is read from `options.context().env()` rather than
`std::env` for the same reason the existing one is: compose must be reproducible
from a captured context, and a snapshot that replays differently because the
ambient environment changed is not a snapshot.

### 3. Fold the existing switches in

`ignore_invalid` and `fail_fast` currently express two thirds of this idea each.
Leaving all three would give three ways to say one thing.

- **`ignore_invalid`** becomes an alias for the new switch, covering *all*
  transclusion failures rather than only target resolution. Its frontmatter key
  and `IGNORE_INVALID` environment variable keep working, at lower precedence
  than the new names, and warn once per compose that they are superseded.
- **`fail_fast`** keeps its `md validate` meaning untouched. Its `ComposeOptions`
  field becomes redundant for transclusion once the default is strict; whether
  it retains meaning for other phases needs the audit in the open questions
  below before it is removed.

### 4. Report the degradation as data, not only as text

`ComposeReport::transclusions_skipped` already counts tolerated failures. Callers
that suppress the error should be able to act on the count without scraping
warning strings, and `md compose` should print a one-line summary to stderr when
it is non-zero, naming the count. This is what makes the suppressed mode safe to
use in automation: the operator opted out of the error but not out of knowing.

## Migration

This is a breaking change to the default behavior of `md compose` and
`Markdown::compose`. A document that composed with a warning yesterday fails
today.

- **The failure message must name the escape hatch.** The error text should
  state the flag and the environment variable, so the first person to hit it in
  CI does not have to read source to recover.
- **One release of overlap.** Ship the new switch and the report summary before
  flipping the default, so a caller can adopt the flag while the old default is
  still in force.
- **Repository sweep.** Every `md compose` invocation in `justfile`s, CI
  workflows, and `scripts/` needs auditing for documents that currently rely on
  a tolerated failure. Any that do should be fixed, not flagged — the flag is
  for callers outside this repository.

## Testing

- Each failure category returns `Err` by default: unresolved target, child
  compose failure, code-render failure, remote failure, cycle, max depth.
- Each of the same categories returns `Ok` with a notice and
  `transclusions_skipped` incremented when the switch is on, through **each**
  of the four surfaces, proving the precedence order rather than only the
  library option.
- `::file-links` with no matches stays `Ok` in both modes.
- The error message names both the flag and the environment variable.
- A suppressed run prints the stderr summary and a strict run does not.
- The deprecated `ignore_invalid` frontmatter key and `IGNORE_INVALID`
  environment variable still suppress, warn once, and lose to the new names.
- `darkmatter/lib/tests/declined_path_transclusion.rs` — written for the
  portable-strings fix — becomes a default-error test, keeping its
  suppressed-mode assertions under the new flag.

## Documentation impact

- `darkmatter/README.md` and the `md compose` help text gain the flag.
- `.claude/skills/darkmatter/SKILL.md` — the transclusion failure model is
  currently undocumented there and is exactly the kind of thing an agent will
  guess wrong.
- The `ignore_invalid` frontmatter key's documentation gains its deprecation.

## Open questions

1. **Naming.** `--allow-failed-transclusions` sits next to
   `--allow-missing-transclusions`, which does something else entirely
   (validation reporting). Is that adjacency clarifying or a trap? A
   `--strict-transclusions=false` form, or renaming the validation flag, are
   both alternatives.
2. **Environment variable prefix.** The existing `IGNORE_INVALID` is unprefixed
   and therefore collision-prone; `DARKMATTER_REMOTE_CONCURRENCY` is prefixed.
   This spec proposes the prefixed form for anything new, but the repo should
   settle the convention once rather than per-feature.
3. **Granularity.** Is one switch right, or do resolution failures and child
   compose failures deserve separate control? One switch is simpler and matches
   how the failures actually reach the operator; the counter-argument is that a
   missing optional include is a different risk from a child that fails to
   compose.
4. **`fail_fast`'s remaining scope.** Does `ComposeOptions::fail_fast` still mean
   anything outside transclusion once the default is strict? Needs an audit of
   its read sites before removal.
5. **Notice text as a contract.** Once failures are opt-in-tolerated, the notice
   becomes something callers may match on. Should its format be specified, or
   explicitly declared unstable?
