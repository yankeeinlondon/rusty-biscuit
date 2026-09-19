---
$schema: feature-review.yaml
ready: false
findings:
  - priority: high
    title: Hash-bounded walks can omit descendants in a skewed merge history
  - priority: high
    title: Valid breaking-change Conventional Commits are treated as non-conventional
  - priority: high
    title: Large duration scopes panic instead of returning InvalidPeriod
  - priority: high
    title: SourceHut and Azure SSH remotes receive invalid commit URLs
  - priority: high
    title: The Level 2 test does not prove the normative style-to-span mapping
human_review: false
reviewed_by: codex/gpt-5.6-sol
log: sniff/features/2026-09-15-recent-commits/implementation-log.md
implemented_by: claude/default
created: 2026-09-17T07:57:18-07:00
spec: 2026-09-15-recent-commits/spec.md
implemented: true
next: 2026-09-15-recent-commits/review-2.md
description: "A **feature** review of `2026-09-15-recent-commits/spec.md`"
feature: 2026-09-15-recent-commits/review-1.md
---

# Review 1 — Recent Commits

## Verdict

The feature is **not ready for production**. Most of the redesign is present
and well factored: the library owns collection and rendering, the CLI is a
thin adapter, aggregate and focused JSON share the same payload, rename-aware
diffs carry line counts, package attribution uses the structure tier, and
empty results preserve valid JSON/stdout behavior. The full Level 1 suite and
the focused tmux Level 2 test pass.

Five blockers remain. Four are directly reproducible data/CLI defects, and the
fifth is a verification-level mismatch for the normative terminal styling
contract. Each has a bounded technical fix and can be verified by an agent, so
human review is not required.

## Findings

### High — Hash-bounded walks can omit descendants in a skewed merge history

Hash selection resolves the requested commit, starts a commit-time-ordered
walk, and breaks as soon as that target is yielded
([`collect.rs:114`](../../lib/src/filesystem/git/recent_commits/collect.rs#L114),
[`collect.rs:126`](../../lib/src/filesystem/git/recent_commits/collect.rs#L126),
[`collect.rs:180`](../../lib/src/filesystem/git/recent_commits/collect.rs#L180)).
That is unsafe in a merge DAG because commit timestamps are not monotonic. A
future-dated boundary can enter the walk frontier through one merge parent and
be yielded before an older-dated descendant on the other parent.

A real temporary repository reproduced the loss. Its topology was:

```text
target base (2030)
├── main descendant (2021) ─┐
└── side descendant (2022) ─┴─ merge side (2023, HEAD)
```

Selecting the target hash returned `merge side`, `side descendant`, and
`target base`; it omitted `main descendant`, even though that commit is both
reachable from the tip and a descendant of the inclusive boundary. The current
hash tests use only linear history, while the separate clock-skew test covers
a time window rather than a hash boundary.

Build the hash range by graph membership rather than by stopping at the first
target yielded by commit-time order (or use an ordering/range construction
that guarantees every descendant of the boundary is emitted first). Add a
Level 1 library integration regression with the merge topology and timestamps
above, plus a shipped-CLI JSON case if the public integration test does not
exercise the same selection adapter.

### High — Valid breaking-change Conventional Commits are treated as non-conventional

The shared parser accepts only `type(scope): description`
([`types.rs:17`](../../lib/src/filesystem/git/types.rs#L17)); it does not accept
the standard optional `!` immediately before the colon. Consequently both
`feat!: breaking without scope` and `feat(api)!: breaking with scope` serialize
with `operation: null`, `scope: null`, and the entire prefix in `heading`.
`--operation feat` returns `[]` for a repository containing only those commits.

This violates Decision 3's promise to accept conventional-commit operations
and Decision 15's requirement to strip the conventional prefix before deriving
the heading. [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/)
explicitly defines the `!` form for breaking changes.

Extend `ConventionalCommit::parse` to accept `!` after either the type or the
optional scope while retaining the operation and scope. Add Level 1 parser,
public collection, and CLI filter regressions for both forms; the latter two
are needed because this parser feeds filtering, payloads, and rendering.

### High — Large duration scopes panic instead of returning InvalidPeriod

Duration parsing uses panicking `chrono::Duration` constructors and unchecked
month/year multiplication
([`options.rs:201`](../../lib/src/filesystem/git/recent_commits/options.rs#L201)).
Each of these valid-shape inputs terminates the shipped debug binary with exit
101 and a panic instead of returning the documented typed input error:

```text
9223372036854775807h
9223372036854775807d
9223372036854775807w
9223372036854775807mo
9223372036854775807y
```

Use checked multiplication and Chrono's checked duration constructors, then
map every out-of-range value to `SniffError::InvalidPeriod`. Add exhaustive
boundary tests at the largest accepted value and first rejected value for each
unit, plus a Level 1 CLI test proving rejection is non-panicking, writes no
stdout, and exits as an ordinary input error.

### High — SourceHut and Azure SSH remotes receive invalid commit URLs

The consolidated link builder parses a generic namespace/repository path and
combines it with a provider-wide static base
([`commit_links.rs:116`](../../lib/src/filesystem/git/commit_links.rs#L116),
[`commit_links.rs:137`](../../lib/src/filesystem/git/commit_links.rs#L137)).
That transformation is not valid for all providers Sniff recognizes:

- `git@git.sr.ht:~acme/project` becomes
  `https://sr.ht/~acme/project/commit/<sha>`; the repository lives under
  `https://git.sr.ht/...`.
- `git@ssh.dev.azure.com:v3/acme/widgets/project` becomes
  `https://dev.azure.com/v3/acme/widgets/project/commit/<sha>`; the canonical
  repository path is
  `https://dev.azure.com/acme/widgets/_git/project`.

The existing remote-observation code already records both provider-specific
normalizations
([`remote_observation.rs:166`](../../lib/src/filesystem/git/remote_observation.rs#L166)).
The new URL tests cover GitHub, GitLab, Bitbucket, and a generic self-hosted
server only, so both broken supported-provider forms pass the suite.

Make the consolidated link authority normalize each supported provider's
transport form before appending its commit route; factor the existing mapping
so remote observation and commit links cannot drift. Add table-driven Level 1
tests for every recognized public provider and transport form, and exercise at
least SourceHut and Azure SSH through `RecentCommits::collect` so containment
and URL selection are proven together.

### High — The Level 2 test does not prove the normative style-to-span mapping

The topic contract assigns bold to the hash/time/labels, blue to the operation,
blue+dim to the scope, italic to `at`, and a remote hyperlink to a linked hash
([`recent-commits.md:148`](../../docs/topics/repo/recent-commits.md#L148)).
This is real-terminal behavior and therefore requires Level 2 evidence.

The tmux test's raw assertions only check that *some* bold SGR and *some* blue
SGR occur anywhere in the captured frame
([`level2_recent_commits_rendering.rs:110`](../../cli/tests/level2_recent_commits_rendering.rs#L110)).
It never checks dim or italic, does not associate an SGR span with the intended
text, and its fixture has no containing remote-tracking ref, so it cannot test
the commit-hash hyperlink. The file-link assertion likewise accepts any OSC8
sequence in the whole frame. Those assertions would stay green if styles moved
to the wrong spans or the hash-link path stopped rendering.

Strengthen the Level 2 fixture with a local containing remote ref and inspect
the captured raw row around each known token. Prove the exact style transition
for hash, operation, scope, `at`, time, and section labels, and prove the hash
URL plus file URL independently (accepting the documented fallback where the
backend lacks OSC8). Level 1 tag tests remain useful but cannot replace this
real-terminal mapping check.

## Requirement Verification Levels

| User-observable requirement | Required level | Strongest evidence present | Assessment |
|---|---:|---|---|
| Bare-array JSON schema, author, moved files, line counts, UTC datetimes, attribution, and three-state containment | L1 | Library unit/integration tests plus shipped-CLI process tests | Correct level; broad coverage passes. Provider URL forms remain incomplete (finding 4). |
| Count/date/duration/hash/branch selection and operation/scope/author/package/file-type filters | L1 | Real temporary-repository integration tests plus CLI process tests | Correct level, but missing cases expose findings 1–3. |
| Empty-result exit status and stdout/stderr routing, including valid JSON stdout | L1 | CLI subprocess tests across all three commands | Correct level; passes. |
| Plain and Markdown degradation, projections, escaping, and library-authored report bytes | L1 | Library render tests and CLI subprocess tests | Correct level; passes for covered cases. |
| Word wrapping, markup degradation, file-link fallback, and absence of leaked tags | L2 | Real tmux pane capture | Correct level; focused test passes. |
| Exact SGR style-to-text mapping and linked-hash terminal rendering | L2 | Generic SGR/link-presence checks only | **Gap**; finding 5. |
| Keyboard, hotkey, paste, IME, or mouse behavior | L3 | No such feature requirement | Not applicable. |

## Validation Performed

- `cargo nextest run -p sniff --features remote --test recent_commits`: 30
  passed, 0 failed, 0 skipped.
- `just test` in `sniff/`: 2,735 passed, 0 failed, 24 tests excluded by the
  canonical tier filter.
- `just test-l2 --test level2_recent_commits_rendering` in `sniff/`: the tmux
  test passed with backend proof, 0 skipped.
- `just lint -D warnings` in `sniff/`: passed.
- The skill-required stricter
  `cargo clippy -p sniff --features remote -p sniff-cli --all-targets -- -D warnings`
  reached unchanged test targets and failed on three pre-existing
  `redundant_closure` warnings in `remote_observation.rs` and
  `focused_provider.rs`. Neither file differs from `main`; this is unrelated
  lint debt and does not contribute to this feature's readiness verdict.
- Reproduction repositories confirmed the hash-range omission, breaking-prefix
  exclusion, SourceHut/Azure URL output, and duration-input panics through the
  built `sniff` CLI.
- GitNexus index `sniff` was current at `96b63d7a0`. It reports the classifier
  change as CRITICAL (47 upstream symbols across Sniff, Darkmatter, and
  Worktree). The implementation record's downstream Darkmatter and Worktree
  L1 runs are green; this review's complete Sniff run is also green.

## Production Readiness

Not ready. Correct the four Level 1 data/input defects and close the exact-style
Level 2 gap before the next review iteration.
