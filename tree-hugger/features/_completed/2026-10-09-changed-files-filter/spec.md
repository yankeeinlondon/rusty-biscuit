---
kind: feature
name: changed-files-filter
date: 2026-10-09
status: draft-spec
area: tree-hugger
packages:
  - tree-hugger-cli
  - sniff
depends-on:
  - 2026-10-03-symbol-filters-and-reports
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
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-05
review_iterations: 4
completed: true
human_review: false
message_to_agent: |-
  Phase 5 documents the completed changed-file behavior and adds one public
  help regression test covering all four commands. No selection, cache, or
  Git API behavior changed. Existing staged lifecycle files were preserved;
  do not overwrite the index. Phase 5 gates passed; implementation is
  complete and ready for review. No subsequent
  implementation phase is needed. Do not move the feature directory.
implemented: true
implemented_by: "codex/gpt-6.1-sol"
---

# Spec: `--changed` File Selection for Symbol Reports

## Problem and scope

There is no direct way to ask "what declarations are in the files I have
changed?" A user must list changed files by hand and pass them to `hug`.

Add `--changed[=<ref>]` to the declaration commands: `functions`, `types`,
`symbols`, and `classes`. It selects whole files and reports their current
on-disk declarations. It does not select changed lines or read source from
the staging area or a historical commit.

```bash
hug functions --changed                  # uncommitted changed files in the normal scan
hug symbols --changed=main               # also include committed differences from local main
hug symbols --changed --file 'src/**'     # changed files intersected with a file selector
hug --json-full symbols --changed        # complete indexes of the selected changed files
```

`imports`, `lint`, `god-files`, and `completions` do not accept this flag in
this feature. Keep the option on declaration arguments rather than the
shared options used by every scanning command. Existing output-mode and
symbol-selector conflicts still apply; `--changed` is a file selector, so
it is allowed with `--json-full`.

## Origin and prerequisites

The author moved the changed-file proposal from the pre-split draft of
`2026-10-03-symbol-improvements` on 2026-10-03.

`2026-10-03-symbol-filters-and-reports` supplies the explicit file selectors
and shared record filtering with which this feature combines. Its current
behavior is documented in [Symbol Reports](../../docs/symbol-reports.md).

Scheduled 2026-10-09, next to `2026-10-10-stats-package-grouping`, which also
uses sniff. A focused sniff API extension and its tests are part of this
feature, rather than an unspecified prerequisite.

## Reference and changed-path contract

`--changed` defaults to `HEAD`. A supplied reference must use the equals
form, such as `--changed=main`; `--changed main` leaves `main` as a normal
positional token. Reject an empty value (`--changed=`) and repeated uses of
the flag as argument errors. References are resolved locally once and must
identify a commit, including a tag that can be resolved to a commit. A tree
or blob alone is not a valid base. No fetch or remote refresh is permitted.

For a repository with a commit at `HEAD`, the changed set is the union of:

1. Paths differing between the supplied base commit and the current `HEAD`
   commit, comparing those two tips directly.
2. Paths with staged changes relative to `HEAD`.
3. Paths with unstaged changes relative to the index (the staging area).
4. Individual nonignored untracked file paths.

This is a set of paths, not a concatenation of reports. A path appearing in
several categories is analyzed once. The committed comparison is empty for
the default `HEAD` reference. `--changed=main` compares to the local `main`
tip, not the branch's common ancestor with `main`; divergent changes on
`main` can therefore select files too.

**Reader's note:** the original proposal combined differences from the base
with staged and unstaged changes. Comparing base to `HEAD` and then adding
working-tree status preserves that union without a second comparison of
all current files to the base. Changes that cancel across stages still
select the file: if a staged edit is undone on disk, the file is selected,
but its restored on-disk contents are reported. Likewise, a committed
branch change undone locally is still selected.

An unborn repository (one with no first commit) uses an empty base only
when the user omits the reference. Include staged additions and nonignored
untracked files, then apply normal eligibility rules. Any explicit reference,
including `--changed=HEAD`, must resolve to a commit or fail. If an explicit
base resolves but `HEAD` is unborn, compare that base with the empty current
commit tree and union the unborn working-tree status. A detached `HEAD` is
valid and uses its current commit.

Renames are treated as delete/add pairs; similarity detection is unnecessary
for this path selector. Keep the destination if it exists and is eligible,
and skip an absent source. Determine absence from the current filesystem,
not solely from Git's deletion status: a staged deletion followed by
recreating the path must still select the current file. Content, executable
mode, and file-type changes count when Git reports them. Submodule entries
are directories, so they do not select source inside the submodule; there
is no recursive Git comparison across repository boundaries.

## Repository boundary and normal selectors

Discover the containing repository from the invocation's working directory.
Use the active checkout's root and index, including in a linked worktree;
never substitute the main checkout's working directory. Reject repository
absence and bare repositories, which have no source working tree.

Keep the existing package scan root, file-reference resolution, name
prefixes, and display-path conventions. Invoking `hug` inside a package
must not expand a default scan to every changed file in the repository.
Explicit files retain their existing ability to select a file outside the
package scan root, but only changed files belonging to the discovered
repository can survive this filter. A file in another repository does not
trigger a second repository discovery.

Selection is the intersection of the changed set with the ordinary file
selection. Positional file tokens and `--file` values remain alternatives
within the ordinary selection; `--changed` intersects their combined result.
Name and other declaration selectors apply afterward as usual.

- Retain normal language eligibility and `--language` behavior, including
  explicitly named extensionless files under a forced language.
- Retain `.gitignore` and default fixture exclusions, including the existing
  user-supplied path/glob overrides. `--exclude-files` wins over inclusion.
- Git-provided paths are candidates, not user-supplied explicit selectors.
  Feeding them into the explicit-file override would accidentally re-include
  fixtures or ignored files.
- Ignored untracked files are never added to the changed set, even if the
  user explicitly names one. A tracked ignored file may still be selected
  through the existing explicit override.
- Intersect and deduplicate before source reads, cache lookups, or parsing.
  Preserve deterministic file ordering and every report's existing sorting.

Keep repository-relative paths as native paths until mapping them to the
checkout root. Use tree-hugger's
[`file_identity`](../../lib/src/file/source_file.rs), which gives analyzed
files a common filesystem identity, for matching existing candidates to
scanner results. Normalize roots consistently, following the `os` skill's
Windows path guidance; do not compare raw slash spellings or lowercase
paths on case-sensitive filesystems. Skip absent paths before
canonicalization. Preserve non-UTF-8 Git paths on platforms that support
them; lossy display strings must not be used as lookup identities.

## Empty selections and errors

A valid changed-file request with no eligible files succeeds with exit `0`,
including when all changed paths are deleted, excluded, unsupported, or
outside the ordinary selection. This is an intentional exception to the
scanner's `NoSourceFiles` error. Handle that specific empty outcome in the
changed-file command path; do not suppress other scanner errors or alter
commands without `--changed`.

Use the existing empty report representations: zero records in terminal,
plain, table, outline, and JSON Lines output; `0` followed by a newline for
`--count`; zero totals and the existing undefined doc-coverage representation
for stats; the normal JSON envelope with empty `files` or `indexes` and
`errors: []`. Keep existing metadata and trailing-newline conventions.
No source analysis or per-file cache lookup is needed for an empty selection.

Validate arguments and Git state before emitting any report. Repository
absence, a bare repository, an unresolved or non-commit reference, corrupt
Git data, permission failures, and index merge conflicts are failures, not
empty successes. Reject conflicts anywhere in the discovered repository,
even outside the selected package, to avoid silently reporting an incomplete
view of unresolved work.

Malformed flag syntax is an argument error (exit `2`). Git discovery,
reference-resolution, comparison, and conflict failures exit `1`, with an
actionable stderr message naming the repository or reference and cause.
Do not print a successful empty JSON report for these failures. Per-file
read or analysis failures after selection retain the existing partial-report
and exit-status behavior. A path absent during selection is skipped; a file
removed after selection is a normal per-file read failure. The command does
not promise an atomic snapshot against concurrent edits or Git operations.

## Implementation boundaries and cost

Use sniff's focused Git APIs rather than full host or repository inventory.
In sniff,
[`GitRepo`](../../../sniff/lib/src/filesystem/git/types.rs) discovers a
trusted repository and provides local status inspection. However, its
`file_changes()` method computes line statistics and produces lossy display
paths. It is not a suitable path-only result for this selector.

Add a fallible focused API to sniff that returns deduplicated native
repository-relative paths for the union defined above and reports conflicts
and comparison failures explicitly. Its input must distinguish the omitted
reference from an explicit `HEAD`, so the unborn exception is enforceable.
The exact public method/type names can follow sniff's existing style.

Share status classification where practical without changing existing rich
status outputs, rename conventions, or bare-repository behavior. The new
API rejects bare repositories even where older APIs degrade to empty status.
Use the existing gix backend; do not parse shell status output, invoke Git
hooks, write the index, or introduce credential-dependent operations.

Capture local status once per request; reuse it for conflict detection and
staged/unstaged/untracked paths. Compare committed trees once when the
resolved base differs from `HEAD`. Request no line statistics, unified
patches, rename similarity, commit history, package inventory, or linked
worktree enumeration. Status may read working files to establish changes;
that necessary work is separate from avoidable line-diff enrichment.

Add sniff to tree-hugger-cli with only the features this focused API needs.
`--changed` and its reference do not enter the persistent symbol-cache key:
unchanged file contents reuse the same entries whether reached through a
changed-file scan or an ordinary scan. Test actual warm cache hits as well
as equal output. The ordinary scanner may still walk to enforce selectors;
this feature promises to narrow analysis, not eliminate all directory walks.

No performance spike is required: the known avoidable work is line-diff and
inventory enrichment. Use focused work assertions or existing counters to
prove it is skipped, without imposing a wall-clock threshold or adding a
benchmark matrix.

## Open Questions

### How should `--changed` combine with Rust prelude exports?

The existing `--prelude` report creates synthetic export records associated
with the prelude file but can locate their declarations in another file.
The prelude resolver also analyzes targets outside the ordinary scan.
Choosing which of these paths governs changed-file eligibility affects both
what users see and how much analysis the command performs.

- **Reject `--changed` with `--prelude` for the initial release (recommended).**
  Pros: the whole-file contract stays clear and the existing prelude resolver
  needs no new eligibility rule. Cons: users cannot request changed prelude
  reports until that behavior is designed. Recommend this because synthetic
  exports need a deliberate ownership rule; accepting the combination today
  risks reporting records from unchanged files or omitting changed targets.
- **Use the prelude export's file as its owner.** Pros: aligns eligibility
  with the synthetic record's identity; changed export statements are shown.
  Cons: changes only to a target declaration do not select an unchanged
  export, and resolving selected exports can analyze unchanged target files.
- **Select when either the export file or its target file changed.** Pros:
  captures both public export edits and target declaration edits. Cons:
  requires resolving exports before eligibility is known, can analyze many
  unchanged targets, and expands the whole-file rule into a relation rule.

The author should settle this before implementation. If the recommended
option is chosen, reject the combination as an argument error before Git
inspection and document it in help and the symbol-report topic page.

## Acceptance and verification

Use temporary local repositories with explicit branch names and fixture
identity; create fixture commits through library APIs so tests do not depend
on host signing settings or credentials. Follow the repository test toolkit
and run L1 tests with the package-area `just test` recipes (nextest).

| Case | Required result |
| --- | --- |
| Staged, unstaged, both, and nonignored untracked files | Current declarations reported once per eligible path |
| Staged edit undone on disk; committed edit undone locally | Path stays selected; current restored declarations reported |
| Ignored untracked file, including an explicit selector | Excluded |
| Default fixtures and explicitly targeted fixtures | Default exclusion retained; normal user override retained |
| Rename, deletion, copy, and staged deletion with recreated file | Existing eligible destinations/copies/recreated files selected; absent paths skipped |
| Supplied branch tip, including divergent branches | Direct tip comparison, not common-ancestor comparison |
| Omitted versus explicit reference on unborn `HEAD`; detached `HEAD` | Empty-base exception only for omitted reference; explicit valid base works as specified |
| Commit ID and commit tag; empty, invalid, and non-commit refs | Valid commit bases accepted; syntax and resolution errors distinguished |
| Missing/bare repository, Git failure, conflicts outside scan root | Nonzero failure before any success report |
| Package invocation, linked worktree, outside-package explicit file | Normal scan scope retained; active checkout used; changed explicit file in same repo eligible |
| Submodule or nested independent repository | No recursive changed-file discovery or accidental selection of its contents |
| File/name/exclusion/language selectors | Intersection and existing precedence retained; forced-language explicit files still work |
| Every report mode and an empty intersection | Existing output contract retained; empty reports succeed |
| Warm cache with and without `--changed` | Same file entry is a real cache hit; no selection-dependent cache key |
| Path spellings, spaces, non-ASCII names; non-UTF-8 where supported | Native identity matching and deterministic deduplication |
| Focused sniff request | One status observation; no optional rich diff, network, or inventory work |

Also test optional-value parsing, repeated-flag rejection, `--json-full`
compatibility, command availability, and the chosen prelude policy. Cover
syntax through argument-parser tests and report behavior through CLI tests;
real terminal windows are unnecessary for this file-selection feature. Any
existing L2 harness used must keep terminal/browser windows from gaining
focus. Tests must work on macOS, Linux, native Windows, and WSL2 and run in
the environments the existing CI plan schedules; do not add CI matrix cells.

## Documentation required with implementation

Update tree-hugger's CLI README, the symbol-report topic page, and the
relevant tree-hugger skill guidance with examples for default `HEAD`, local
tip comparison, selector intersection, empty success, and the chosen prelude
policy. Update sniff's Git API documentation and skill guidance for the new
path-only method, local-only cost, error behavior, and comparison semantics.
Record the new dependency in the repository dependency documentation and
applicable per-area dependency pages. Topic pages describe current behavior
and must not link to or name this feature specification.
