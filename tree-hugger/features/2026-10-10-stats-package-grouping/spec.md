---
kind: feature
name: stats-package-grouping
date: 2026-10-10
status: implemented
area: tree-hugger
packages:
  - tree-hugger-cli
  - sniff
  - sniff-cli
  - claudine-cli
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
review_iterations: 2
completed: true
human_review: false
human_review_items: []
message_to_agent: |-
  All five implementation phases are complete, ready for review. Phase 5
  added the README Package Stats guide, clarified the topic flow, updated
  tree-hugger skill guidance and corrected two stale declaration-scan path
  comments. No runtime behavior changed in Phase 5. Final macOS evidence:
  1,938 L1 tests passed with zero skips; final-tree grouping check 30 passed;
  lint and tier coverage passed. Requirement-to-test mapping, cumulative
  inventories and validation exceptions are in the implementation log.
  Sniff evidence is retained from Phase 1, not rerun under the Phase 5 area
  limit. Local Linux/Windows/WSL2 execution remains unproven; existing CI
  includes the platform-specific tests. Earlier staged work is preserved.
  Do not move this directory: the author closes review and owns that move.
implemented: true
implemented_by: codex/gpt-6.1-sol

---

# Spec: Package Grouping in `hug --stats`

## Problem and scope

`hug symbols --stats` counts selected declarations by kind, visibility, and
language, with documentation coverage. A monorepo scan currently produces
one total, so a developer cannot see which packages contribute to it.

Add package groups to the stats reports for `functions`, `types`, `symbols`,
and `classes`, using sniff's manifest-backed package identity. Keep the
overall totals and include an explicit **Unassigned** group for selected
files without a usable package identity. Never infer package names from
directory names, and never combine a library and CLI merely because they
share a package area.

For example, functions in `tree-hugger/lib` and `tree-hugger/cli` belong to
`tree-hugger` and `tree-hugger-cli`, respectively. A source file beside both
packages belongs to Unassigned unless another discovered package owns it.

This feature changes reporting only. It adds no package selector, new source
extraction, package-area totals, or grouping for declaration listings,
`--count`, JSON Lines, or full-index JSON.

## Origin, scheduling, and prerequisites

The author moved this proposal from the pre-split draft of
`2026-10-03-symbol-improvements` on 2026-10-03. Scheduled for 2026-10-10,
after `2026-10-09-changed-files-filter`, which also brings sniff onto the
CLI's path. The author decided on 2026-10-05 that grouping should be built
before consumers exist; a missing consumer is not a reason to defer it.

`2026-10-03-symbol-filters-and-reports` supplies the implemented stats and
selection behavior, described in [Symbol Reports](../../docs/symbol-reports.md).
Sniff supplies package membership and identity, corresponding to the catalog
used by `sniff repo packages`. Scheduling after changed-file selection does
not make that selector a functional prerequisite: ordinary stats must work
without Git changes or `--changed`.

Two author decisions remain under Open Questions. The output contracts below
use the recommended automatic grouping and explicit name-source metadata;
they become final only when those choices are approved.

## Selection and counting contract

Apply existing file and declaration selectors first. Assign each selected
record to exactly one package group or Unassigned, and add that same record
to the overall totals. Grouping must not expand the scan, change default
scopes, or analyze files merely because sniff discovered their package.

Use the existing tree-hugger-cli
[`Stats`](../../cli/src/stats.rs) accumulator, which counts selected records
and measures documentation coverage, for both overall and per-group totals.
This keeps the meanings of the fields identical:

- Every selected record counts once in each of `by_kind`, `by_visibility`,
  and `by_language`. Each map's values sum to the corresponding `--count`.
- `classes --stats` counts selected containers, not their displayed members.
- Synthetic prelude exports count in those maps and in `synthetic_exports`,
  but contribute to neither the documentation denominator nor `unavailable`.
- `doc_coverage.total` counts nonsynthetic declarations whose documentation
  status is known; `documented` counts the documented subset. `unavailable`
  counts nonsynthetic declarations whose documentation status is unknown.
- `ratio` is `documented / total`, or JSON `null` and terminal `n/a` when
  `total` is zero. A group containing only unavailable documentation or only
  synthetic exports also has an undefined ratio. Never average group ratios
  to obtain the overall ratio; compute it from the overall counts.

For synthetic prelude exports, ownership follows the export record's identity
file (the prelude), rather than the resolved target declaration's location.
A cross-package re-export therefore contributes to the exporting package.
Use native analysis paths for lookup, not formatted locations or qualified
names. Grouping adds no extra prelude target resolution; any restriction on
combining `--prelude` with `--changed` remains owned by changed-file selection.

## Discovery root and package ownership

Capture one structure-only package catalog per stats request. In a Git
checkout, discover from the active worktree root, even when the invocation
starts inside a package. Keep the normal scan root and JSON `root_dir`
unchanged. In a project without Git, discover at the existing package scan
root; absence of Git alone is not an error. Do not open linked checkouts or
recursively discover independent repositories for explicitly selected files.
Files outside the captured discovery root go to Unassigned.

In sniff,
[`detect_repo_structure_or_root_package`](../../../sniff/lib/src/filesystem/repo/types.rs)
returns shallow workspace structure or a standalone root package. Prefer
this bounded API or its request-based equivalent with
`RepoRequest::structure()`, rather than full host detection or enriched
repository inventory. Standalone packages must work as well as monorepos;
a workspace-only root is not itself a package unless sniff includes it as
one. Membership rules, exclusions, and nested workspace discovery remain
sniff's responsibility. Do not implement another manifest walker in `hug`.

Assign a file to the deepest discovered package directory containing its
native path, using path components. A nested package takes precedence over
its parent, and `pkg-a` must not claim files in `pkg-a2`. A root package
claims only files not owned by a deeper package. Match boundaries before
checking the owner's name: an unnamed nested package goes to Unassigned,
not to its named parent.

Use normalized native paths consistent with tree-hugger's
[`file_identity`](../../lib/src/file/source_file.rs), which identifies analyzed
files, and sniff's existing ownership rules. Preserve symlink and Windows
path normalization through the repository's established helpers; do not
lowercase paths or use lossy strings as lookup keys. Portable `/`-separated
strings are for output only. Group by package directory identity, never by
name alone, so two packages named `utils` remain distinct. Deduplicate any
catalog aliases for the same package directory according to sniff's catalog
precedence rather than creating overlapping groups.

**Reader's note:** tree-hugger's existing listing sections are based on local
manifest boundaries and path headings. They are not the identity authority
for this report. Also, sniff's
[`attribute_paths`](../../../sniff/lib/src/filesystem/repo/aggregate.rs), used
to summarize changed-path attribution, can include ancestor packages as well
as nested owners; using it here would double-count records. Match the deepest
owner semantics of sniff's
[`RepoInfo::package_for_dir`](../../../sniff/lib/src/filesystem/repo/types.rs),
which locates a directory's owning package, while avoiding reconstruction of
its temporary ownership index for every record.

## Named groups, empty selections, and errors

Create groups for eligible selected files, before declaration filtering,
including files whose analysis fails. Thus a scanned package with zero
matching declarations has empty count maps, zero coverage counts, and a
`null` ratio. Do not emit every package in the repository catalog: a package
with no selected files is absent. Emit Unassigned only when at least one
selected file has no usable owner identity.

An empty selected file set has overall zero stats and `groups: []`. Preserve
the existing distinction between an ordinary `NoSourceFiles` error and the
successful empty selection supported by `--changed`; this feature introduces
no new empty-scan exception.

A successful discovery with no recognized packages places selected files in
Unassigned. A known package boundary with no manifest-backed name also uses
Unassigned. These are ordinary outcomes, not failures.

Do not convert a returned discovery failure into Unassigned. Trust,
permission, corrupt repository, and other returned errors fail with exit `1`
and an actionable stderr message before a successful stats envelope is
written. Sniff's existing best-effort manifest discovery may omit unsupported
or unreadable package identities; this report does not claim to validate
all manifests. Explicit name-source metadata must distinguish a missing name
from a fallback label, without inventing new global validation rules.

After discovery, per-file read and analysis failures retain existing partial
report behavior: count successful selected records, keep zero groups for
selected files that failed, report errors once in the top-level `errors`
array, and exit `1`. No atomic snapshot against concurrent file or manifest
edits is promised.

## Terminal and JSON reports

Under the recommended automatic policy, ordinary `--stats` prints the current
overall tables under an **All selected declarations** heading, followed by
one section per group. A named section includes both manifest name and
package path, for example `tree-hugger-cli (tree-hugger/cli)`. Unassigned is
labeled explicitly. Use the same category tables, coverage line, and
conditional synthetic-export count in each section, including zero groups.

Keep existing output conflicts: `--stats` accepts `--json`, default terminal
output, `--format plain`, or `--format table`; it rejects `--count`,
`--jsonl`, `--json-full`, `--sort`, `--tree`, and explicit `--signatures`.
Use biscuit-terminal `TerminalRenderable` components for headings and tables,
with literal handling of names and paths so manifest text cannot introduce
markup, links, or terminal controls. Preserve existing width and styling
behavior, and keep plain output free of escapes and hyperlinks.

For JSON, keep the current `stats` object as the overall result and add a
`groups` array before `errors`. Each entry has `package` and `stats`; every
group's `stats` has exactly the existing overall stats shape. A named
`package` is an object with `name` and `path`; Unassigned uses `package: null`,
so it cannot collide with a real package named `Unassigned`.

Example with one selected, undocumented Rust function:

```json
{
  "report_schema_version": 1,
  "root_dir": "/work/repo",
  "package_root_dir": "/work/repo",
  "stats": {
    "by_kind": {"Function": 1},
    "by_visibility": {"Public": 1},
    "by_language": {"Rust": 1},
    "doc_coverage": {"documented": 0, "total": 1, "unavailable": 0, "ratio": 0.0},
    "synthetic_exports": 0
  },
  "groups": [
    {
      "package": {"name": "utils", "path": "crates/utils"},
      "stats": {
        "by_kind": {"Function": 1},
        "by_visibility": {"Public": 1},
        "by_language": {"Rust": 1},
        "doc_coverage": {"documented": 0, "total": 1, "unavailable": 0, "ratio": 0.0},
        "synthetic_exports": 0
      }
    }
  ],
  "errors": []
}
```

`package.path` is relative to the discovery root, uses `/` separators, and
is `.` for a root package. This frame differs from `root_dir` when scanning
a subpackage; document it explicitly. Add `package_root_dir` to the envelope,
a portable absolute discovery-root path, before `stats`, so consumers can
resolve group paths. Emit it whenever grouping is active, including an empty
report or a report containing only Unassigned.

Order named groups by portable package path, then name as a tie-breaker;
Unassigned is last. Use this same order in terminal and JSON output,
independent of worker completion order. Keep category-key spellings, sorted
maps, integer counters, trailing newline, and `errors` always present and last.

This is an additive change to the report envelope: retain
`report_schema_version: 1` because existing fields keep their meaning.
Document this additive report-field policy with the implementation; the
record schema's major/minor version policy is a separate contract.
Neither the symbol-record schema nor extractor revision changes. Existing
JSON consumers reading overall `stats` retain those values; terminal output
changes intentionally and must be called out in release notes and snapshots.

## Implementation boundaries and cost

Add or reuse sniff in tree-hugger-cli with only the needed features; package
structure does not require sniff's network or remote features. The same
invocation may use `--changed`: reuse repository identity where the APIs
support it, and observe the package catalog once. Unchanged output modes
must not acquire a package catalog for this feature.

Normalize package boundaries once, resolve ownership once per selected file,
and update overall and per-group counters while consuming results. Hold
counters and the discovery catalog, not all selected symbol records. Do not
call `RepoInfo::package_for_dir` once per symbol: its current implementation
rebuilds and normalizes an ownership index on every call. A reusable ownership
lookup should follow sniff's existing component-based semantics.

Grouping and manifest identity stay outside persistent symbol entries and
cache keys. Renaming a manifest package must update group labels on the next
run while unchanged source files remain actual cache hits. Do not request
reference analysis, diagnostics, or richer metrics for grouping.

No performance spike is required. The avoidable costs are identifiable:
repeated discovery, repeated ownership-index construction, full inventory,
lockfiles, network, and extra source parsing. Verify bounded work through
focused tests or existing counters rather than adding timing thresholds or
benchmark hosts.

## Open Questions

### Should grouping be automatic or require an option?

The draft says to group `--stats` but does not say whether existing invocations
change. This matters for terminal snapshots and scripts reading report fields.

- **Automatically add groups while retaining overall totals (recommended).**
  Pros: directly fulfills the proposed `--stats` behavior, needs no new flag,
  and preserves the existing overall JSON fields. Cons: longer terminal
  reports and additional discovery work for every stats run. Recommend this
  because the repository has no established users and grouping remains a
  bounded reporting cost; unchanged modes still avoid discovery.
- **Require `--group-by package` with `--stats`.** Pros: existing stats output
  and cost stay unchanged; users explicitly request the detail. Cons: adds a
  flag and argument-validation rules, and makes the feature less discoverable.
  If chosen, keep ordinary stats byte-compatible and emit the new fields only
  for grouped stats; reject the option without `--stats` as an argument error.
- **Automatically group only when selected files span multiple packages.**
  Pros: compact single-package reports. Cons: output shape and headings vary
  with selection, and Unassigned makes the threshold ambiguous. This requires
  a separate threshold rule and complicates consumers without removing the
  discovery cost.

### How should sniff expose whether a package name came from a manifest?

In sniff, [`Package::name`](../../../sniff/lib/src/filesystem/repo/types.rs)
is the catalog's display identity. Its
[`resolve_package_name`](../../../sniff/lib/src/filesystem/repo/detection.rs)
reader falls back to the relative directory path when it finds no manifest
name. A nonempty name therefore does not prove manifest-backed identity.
Package membership provenance describes the boundary, not the name's source.
Comparing the name with the path cannot fix this: a real name can equal it.

- **Expose explicit name-source metadata through sniff (recommended).**
  Pros: preserves one manifest reader and distinguishes declared names from
  fallback labels without changing existing sniff output semantics. Cons:
  requires a small sniff API change and tests, so sniff must be added to the
  implementation's declared package scope before finalization. Recommend this
  because downstream callers can enforce honest identity without re-parsing
  manifests. Also expose a reusable ownership lookup if needed; its current
  internal index cannot be imported by the CLI.
- **Validate names by reading manifests again in tree-hugger-cli.** Pros:
  avoids changing sniff's public API. Cons: duplicates ecosystem rules and
  reads, risks disagreeing with sniff, and grows maintenance work whenever
  another manifest format is supported.
- **Accept sniff's fallback name as a package identity.** Pros: simplest
  integration and all discovered boundaries get a label. Cons: explicitly
  reverses the author's requirement against directory-derived names and can
  present guessed identity as declared identity. Choose only if the author
  intentionally changes that requirement, with matching docs and examples.

Both questions require author review before implementation. Review metadata
records that this document has been reviewed; it does not finalize these
choices or claim the feature is implemented.

## Acceptance and verification

Use the repository test toolkit and package-area `just test` recipes
(nextest). Cover report behavior with temporary local fixtures and CLI tests;
real terminal windows are unnecessary for package assignment. Any existing
L2 harness used must keep terminal and browser windows from gaining focus.

| Case | Required result |
| --- | --- |
| Cargo workspace and another supported ecosystem | Manifest names and membership come from sniff; grouping is language-independent |
| Standalone package, with and without Git | One named group; no Git prerequisite |
| Virtual workspace, manifest-less tree, unnamed nested package | No invented root/name; affected selected files use Unassigned |
| Root package plus nested package; sibling names sharing a prefix | Deepest owner once, without parent duplication or sibling collisions |
| Library and CLI in one area; duplicate manifest names | Separate groups distinguished by package paths |
| Invocation inside a package or linked worktree | Normal scan retained; active worktree catalog; correct path frame |
| Explicit file outside discovery root | Unassigned; no second repository or host inventory |
| File, name, scope, and other declaration filters | Group totals count exactly the records counted by ordinary stats |
| Package with selected files but zero matches or failed analysis | Zero group retained; undefined coverage ratio; failures reported once |
| No selected files, including a valid empty changed-file request | Existing success/error rules retained; successful empty stats have `groups: []` |
| Unknown documentation and synthetic cross-package prelude exports | Existing coverage semantics; prelude owns synthetic export |
| Successful absence versus returned discovery failure | Unassigned for absence; actionable nonzero failure for returned errors |
| Terminal, plain, table, and JSON stats | Same totals and deterministic group order; plain has no escapes |
| Names with spaces, non-ASCII, markup, or terminal-control characters | Literal safe display; JSON escaping; names never become lookup keys |
| Symlink aliases, native Windows path spellings, non-UTF-8 paths where supported | Native identity lookup, no lossy-key collision or duplicate counting |
| Warm cache after package-name edit | New label, unchanged totals, actual source cache hits |
| Non-stats modes and discovery work | No newly added catalog request outside grouped stats; no inventory or network enrichment |

Assert that summing each group's category counts, documented/total/unavailable
counts, and synthetic-export count equals the overall fields. Compute the
overall ratio from the summed denominator, including an unequal-group-size
case to catch accidental averaging. Verify CLI option conflicts and the
chosen name-source policy. Tests must work on macOS, Linux, native Windows,
and WSL2 within the existing CI schedule; add no new matrix cells.

## Documentation required with implementation

Update tree-hugger's CLI README, the symbol-report topic page, and the
applicable skill guidance with automatic or opt-in usage, overall and group
fields, discovery-root path framing, zero groups, Unassigned, synthetic-export
ownership, and failure behavior. Include before/after terminal output in
release notes. Update dependency documentation if sniff is newly added.
If the recommended sniff API extension is chosen, update its API docs and
skill guidance as well. Topic pages describe current behavior and never link
to or name this feature specification. Record implementation departures in
the implementation log; leave this specification as the dated design record.
