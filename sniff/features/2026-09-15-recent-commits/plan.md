---
total_phases: 6
created: 2026-09-17
phase: 1
agent: codex/gpt-5.6-sol
yolo: true
---

# Recent Commits Redesign Implementation Plan

## Phase 1 — Contract and Risk Closure

### Necessary Rules

No additional product ruling is required before implementation. The feature spec is
marked `clarified: true` / `needs_rulings: false`, and Decisions 1–15 are the
authoritative contract wherever the current code or either future-state document
disagrees. Implementation choices that still need to be made must stay within those
decisions:

- Choose and document a concrete total commit-visit budget from the completed
  linking spike, with approximately 10× headroom over its measured normal-state
  ceiling. Budget exhaustion must yield `remote: null`, never a false negative.
- Preserve local-only behavior: recent-commit collection and linking must not fetch,
  probe a provider, or otherwise use the network.
- Treat the JSON/API changes as an intentional clean break. Do not retain deprecated
  `CommitDescSet`, `get_recent_commits_*`, post-hoc filters, or the old object
  envelope as compatibility shims.
- Keep business logic, selection, filtering, attribution, linking, JSON projection,
  and text authorship in `sniff`; the CLI may parse arguments, invoke the library,
  render library prose through `biscuit-terminal::Prose`, and write output.
- Preserve cross-platform path and time behavior on macOS, Linux, native Windows,
  and WSL2. JSON timestamps remain UTC; calendar selection and human date labels use
  the options' fixed offset, which defaults to the host's local offset.

### Work Summary

Replace the monolithic recent-commit implementation and its CLI-owned behavior with
a library-owned `RecentCommitsOptions -> RecentCommits::collect -> render/serialize`
pipeline. The redesign must add the complete payload (author, parsed message fields,
exclusive file categories, rename/copy metadata, line counts, package attribution,
and three-state remote/link information), implement all selectors and additive
filters during collection, consolidate URL and containment logic, and make the three
CLI commit-family commands and aggregate JSON projections thin consumers of the same
results.

The highest-risk change is the new canonical path classifier. GitNexus reports 53
transitively impacted symbols and direct consumers in Sniff, Darkmatter, and
Worktree, so it receives an isolated checkpoint and downstream regression tests.
Replacing `CommitDescSet` is medium risk (28 impacted symbols, chiefly Git and
aggregate projection); reusing the containment engine is low graph risk but has a
known pathological cost that the bounded-walk contract must prevent.

### Successful Completion

The feature is complete when library callers can collect correctly selected and
filtered commits from any supported platform, serialize the specified bare array,
and obtain prose, Markdown, or plain text without a terminal dependency; the CLI's
three commit-family commands expose that behavior with valid stdout-only JSON and
successful empty results; bare `sniff repo --json` embeds the identical last-10
arrays without network access or envelope rewriting; all legacy/duplicate paths are
removed; public docs describe the shipped contract; and Sniff plus affected
downstream and cross-platform validation gates are green.

### Risk Spikes

- The containment-cost spike is complete in `spike-linking-cost.md`. Its evidence is
  accepted: do not repeat the multi-minute unbounded cases. Implement a
  priority-ordered total visit budget and validate it with deterministic small-budget
  fixtures and work counters.
- The first implementation hour must verify that one `Prose` render preserves a
  representative multi-commit, multi-line report without paragraph inflation. Use
  `bt prose` and a focused `biscuit-terminal` test fixture before committing to the
  single-pass terminal integration.
- Verify clap's subcommand shadowing using parser tests for global `-v` before the
  subcommand versus `recent-commits -v` / `--verbose` after it. This is an accepted
  interface risk, not an invitation to redesign the flags.

### Work Group 1A — Contract Baseline

These tasks establish the observable contract and may run concurrently.

- [ ] **Contract matrix**
  - Convert Decisions 1–15 into a test matrix covering selectors, branch bases,
    filters, message parsing, merge commits, file changes, attribution, linking,
    serialization, rendering, empty results, sibling presets, and aggregate output.
  - For each case, identify the cheapest proving boundary (unit, library integration,
    CLI integration, or L2 terminal) and the exact broken behavior the assertion must
    catch.

- [ ] **Caller census**
  - Reconfirm text and graph callers of `CommitDescSet`, `get_recent_commits_*`,
    `commit_browser_url`, `populate_recent_commit_remotes_from_snapshot`, and the
    path-kind predicates before implementation begins.
  - Record the direct Darkmatter and Worktree classifier consumers as required
    downstream validation, and flag any newly discovered external public callers
    before deleting the legacy API.

- [ ] **Fixture inventory**
  - Catalog reusable Git fixtures and add only the missing fixture requirements:
    fixed author/timezone commits, skewed timestamps, merge and empty-merge commits,
    local/remote branch name collisions, rename/copy changes, nested package roots,
    multiple remotes, unpushed commits, and budget exhaustion.
  - Ensure fixtures set local Git identity and are hermetic from host Git config,
    credentials, network, CWD, and repository discovery.

### Work Group 1B — Interface Spikes

Complete this work group after the contract matrix; both tasks can run concurrently.

- [ ] **Prose rendering**
  - Run the representative multi-line report through `bt prose` and a focused test,
    checking line breaks, indentation, word wrapping, style degradation, and OSC8
    fallback.
  - Record the smallest library prose layout that survives one CLI-side
    `Prose::new(...).render(&terminal)` call; if paragraph inflation occurs, resolve
    it within `biscuit-terminal`'s supported prose vocabulary rather than restoring a
    CLI renderer.

- [ ] **Flag shadowing**
  - Use a focused clap fixture to prove that subcommand `-v` / `--verbose` selects
    verbose report output, `-c` / `--compact` selects compact output, and global
    verbosity before `repo recent-commits` remains the log-level counter; retain the
    cases as parser regressions when the production argument shape lands in Phase 4.
  - Pin conflicts between compact and verbose flags and confirm completion metadata
    suggests common `--operation` values without restricting free-form values.

**Validation checkpoint:** the contract matrix has an owner and proving boundary for
every decision; the Prose and clap risks have executable regression tests; the
linking budget choice and caller census are documented; no unresolved product ruling
remains.

## Phase 2 — Library Primitives

Build the independent data, classification, diff, and options foundations required
by collection. Establish shared public types first; then the three work groups below
may proceed concurrently because they own separate modules.

### Work Group 2A — Shared Types

- [ ] **Module scaffold**
  - Introduce the `filesystem/git/recent_commits/` module boundary and define the
    public `RecentCommits`, `RecentCommit`, `RecentCommitAuthor`,
    `RecentCommitFile`, `RecentCommitFileKind`, `RecentCommitFileTypes`,
    `RecentCommitsOptions`, `RecentCommitsVerbosity`, and `Selection` shapes.
  - Add two-level re-exports through `filesystem::git` and `filesystem`, matching the
    existing public export convention without exposing collection internals.

- [ ] **Payload contract**
  - Encode a bare-array JSON representation with UTC RFC 3339 datetimes, both author
    fields, `modified|added|deleted|moved`, optional `original_path` and line counts,
    parsed operation/scope/heading/description/bullets, file-type booleans, nullable
    `remote`, and `commit_url` only when a browser link is available.
  - Omit `packages` and `package_areas` together for non-monorepos; serialize both as
    arrays, including empty arrays, for monorepos; allow an empty file list for
    degenerate merges.

### Work Group 2B — Path Categories

- [ ] **Canonical classifier**
  - Add public `ChangeCategory` and `classify_path` to `filesystem/path_kind.rs`, with
    first-match precedence for CI/CD path rules, registry associations, then `other`.
  - Cover CI/CD path variants and the required edge cases: HTML/CSS/fonts as web
    assets, images including SVG, Angular `.component.html` as source code, and
    mutually exclusive classification.

- [ ] **Predicate wrappers**
  - Reimplement existing source/documentation predicates as wrappers over the enum
    and update their behavior docs and drifted tests for the intentional HTML/CSS
    changes.
  - Run focused Sniff, Darkmatter capture, and Worktree dirty-file tests at this
    checkpoint because GitNexus rates this shared change CRITICAL.

### Work Group 2C — Diff Metadata

- [ ] **Rewrite detection**
  - Enable gix rename/copy tracking for committed-tree diffs while preserving
    first-parent semantics, initial-commit behavior, deterministic path ordering,
    cache reuse, and fallible corruption handling.
  - Normalize both rename and copy rewrites to public `moved` records and retain the
    source path as `original_path`.

- [ ] **Line statistics**
  - Extend the committed diff result to carry optional added/removed line counts
    without duplicating blob loads or conflating binary/unavailable statistics with
    zero.
  - Add exact fixtures for add, modify, delete, rename, copy, binary, initial commit,
    first-parent merge, and no-change merge cases.

### Work Group 2D — Runtime Options

- [ ] **Options builder**
  - Implement the non-generic builder with one last-wins `Selection` defaulting to
    `Count(10)`, additive AND filters, repeatable OR values within operation filters,
    verbosity, `show_author`, file-family projection, and fixed-offset timezone.
  - Keep `parse_period` beside `Selection`; reject count zero and preserve the
    documented parse precedence for named days, ISO date, numeric count, duration,
    and hash.

- [ ] **Calendar bounds**
  - Convert `today`, `yesterday`, and a specific date into exact local-calendar
    bounds using the options' `FixedOffset`; ensure yesterday and explicit dates stop
    at the following local midnight.
  - Test positive and negative offsets and a boundary commit while asserting that
    serialized datetimes remain UTC.

**Validation checkpoint:** the new public types and options compile through both
re-export levels; JSON shape tests are exact; classification and diff fixtures pass;
shared predicate consumers remain green; `just check` and focused library tests pass
before collection is migrated.

## Phase 3 — Unified Collection and Linking

Implement one collection pipeline over the Phase 2 contracts. Complete collection
core first; attribution and linking may then run concurrently against the fixed
candidate representation, followed by one integration checkpoint.

### Work Group 3A — Collection Core

- [ ] **History selection**
  - Implement `RecentCommits::collect(&GitRepo, &RecentCommitsOptions)` for count,
    duration, named day, specific date, hash-through-tip, and branch-base selection.
  - Resolve `--branch` local-first then remote-tracking fallback without network;
    preserve merge commits and first-parent file diffs; surface invalid/unreachable
    hashes, unknown branches, corruption, and repository failures as typed errors.

- [ ] **Inline filtering**
  - Apply operation, conventional scope, author substring, package, package-area,
    and file-category filters as additive AND predicates, with repeated operations
    OR'd case-insensitively.
  - For count selection, continue walking until N matching commits are collected or
    history is exhausted; do not collect N first and filter afterward.

- [ ] **Message parsing**
  - Parse operation/scope from the subject line; split heading at the first period or
    newline; join prose after that sentence until bullets; always emit a description,
    including an empty one; retain bullet order.
  - Pin the specified abbreviation truncation, punctuation-free subject,
    multi-paragraph prose, bullet-only body, and non-conventional cases.

- [ ] **Author capture**
  - Capture author name and email from each commit and implement case-insensitive
    substring matching against either field; exclude co-author trailers from v1.
  - Verify non-UTF-8/lossy Git metadata follows the library's existing public string
    boundary policy and does not panic.

### Work Group 3B — Package Attribution

- [ ] **Structure catalog**
  - Request manifest-only structure-tier repository detection once per collection
    and build one `PackageOwnershipIndex` shared by attribution and filters.
  - Prove with work counters that collection does not start the full inventory,
    language, framework, or repository-wide document walks.

- [ ] **Ownership projection**
  - Attribute only files owned by a manifest-backed package; leave files directly
    under a package-area directory unattributed; keep deterministic package/area
    arrays.
  - Validate unknown package and area filters return typed errors listing valid
    names, and package filters reuse the index rather than rescanning the catalog.

### Work Group 3C — Commit Linking

- [ ] **Link authority**
  - Create `filesystem/git/commit_links.rs` as the single remote parser, owner/repo
    extractor, provider URL builder, and preferred-remote authority.
  - Make the existing single-commit URL helper a thin wrapper and migrate `repo hash`
    to it; delete the CLI parser and decoration-prefix pushed heuristic once their
    callers move.

- [ ] **Bounded containment**
  - Observe local remote-tracking refs without the fetch-coupled deep-request path;
    order tips by origin, alphabetically first non-upstream, then upstream; enforce
    the single total commit-visit budget selected in Phase 1.
  - Stop when every target is determined. Emit `true` for contained, `false` only
    after all relevant walks complete, and `null` for targets still undetermined at
    exhaustion; attach a URL only when the winning containing remote has a supported
    browser URL.

- [ ] **Counter coverage**
  - Reuse existing commit-visit and ref-walk chokepoint counters and ensure no worker
    or alternate path bypasses them; add no classifier counter because it performs no
    I/O-shaped work.
  - Test preferred-remote ordering, multiple containing remotes, self-hosted/no-URL,
    no remote refs, stale tips, skewed commit times, partially exhausted targets,
    and zero network/fetch work.

### Work Group 3D — Collection Integration

- [ ] **Collection assembly**
  - Enrich only after the selected/filtered candidate set is fixed, so diff/message
    work is not repeated and linking runs once for the final targets.
  - Expose `to_json()` as the bare array and remove period/root/package-catalog
    envelope data from the serializable result while retaining any context needed by
    text rendering privately.

- [ ] **Library regression**
  - Exercise all selector/filter combinations through real temporary repositories,
    including count-after-filter traversal, branch bases, local-time boundaries,
    empty success, merge commits, attribution, links, and exact JSON fields.
  - Compare work counters against compatible request shapes and assert no full-tier
    repository walk, fetch, provider request, or repeated containment pass.

**Validation checkpoint:** focused library unit/integration suites pass with the
`remote` feature; a last-10 default collection yields the exact bare-array schema;
all empty queries return `Ok` with an empty collection; bounded linking stays within
its visit budget and performs no network work.

## Phase 4 — Rendering and CLI Migration

Library rendering and CLI argument plumbing can be implemented concurrently after
Phase 3 fixes the options and payload types. Integrate them only after both work
groups pass their focused tests.

### Work Group 4A — Library Rendering

- [ ] **Single renderer**
  - Implement one library layout walk parameterized by normal/compact/verbose,
    `show_author`, and the all/source/documentation projection; keep the sibling
    projections' file-level pruning and headings in the library.
  - Render the heading separately from description and bullets, honor empty-file
    merge commits, and use the options timezone for relative day labels.

- [ ] **Format degradation**
  - Implement `to_prose`, `to_markdown`, and `to_plain` from the same layout:
    Prose tags and links; Markdown links/emphasis without color; bare text with no
    tags, formatting markers, or links.
  - Link the hash only when `commit_url` exists; create cross-platform `file://` URLs
    for file paths; apply the normative style vocabulary without raw escape codes.

- [ ] **Render fixtures**
  - Add exact compact/normal/verbose fixtures with and without authors, conventional
    metadata, descriptions, bullets, files, remote links, and file links.
  - Prove `to_json()` is invariant under verbosity/show-author and that plain output
    contains neither ANSI/OSC8 bytes nor Markdown markers.

### Work Group 4B — CLI Arguments

- [ ] **Shared arguments**
  - Replace the six-value `--action` enum with repeatable free-form `--operation` and
    add scope, author, branch, file-category, show-author, compact, and verbose flags
    to all three commit-family subcommands through one shared clap argument shape.
  - Remove `--no-error` and `--on-error` from all three sibling commands; retain
    dynamic package/area completions and add non-restricting operation suggestions.

- [ ] **Parser coverage**
  - Test every flag mapping, repeated operations, conflicting verbosity switches,
    the count-10 absent-period default, and subcommand/global verbosity position.
  - Update help/after-help and command discovery snapshots so removed and renamed
    flags cannot silently remain advertised.

### Work Group 4C — Thin CLI

- [ ] **Command adapter**
  - Replace CLI-side period bounds, filtering, linking, JSON manipulation, and
    styling with construction of `RecentCommitsOptions`, one library collection, and
    the selected library output.
  - Keep recent/source/documentation as real subcommands implemented as thin presets;
    emit valid empty output with exit 0 (`[]` for JSON and an optional brief note for
    human formats).

- [ ] **Terminal passthrough**
  - Render terminal output exactly once with
    `Prose::new(report.to_prose(&options)).render(&terminal)`; use `to_plain()` for
    `--plain` and library JSON for `--json`.
  - Send all main report content to stdout; keep JSON stdout parseable and free of
    logs, hints, performance text, or empty-result prose; route any permitted CLI
    metadata to stderr.

- [ ] **Duplicate removal**
  - Delete `cli/src/output/commit_blocks.rs` and its styled renderer/filter/URL
    helpers after all terminal and JSON callers use the library.
  - Remove obsolete CLI imports/dependencies only when no other command uses them,
    and update module declarations and tests in the same change.

**Validation checkpoint:** focused parser and CLI integration tests pass for all
three commands and output modes; stdout parses as a bare array in JSON mode,
including empty results; terminal output is library prose rendered once; L2 terminal
coverage confirms wrapping, styles, and hyperlink fallback without fixed sleeps.

## Phase 5 — Aggregate Alignment and Legacy Removal

Migrate aggregate evidence and projection first, then remove legacy APIs. Documentation
can proceed concurrently with legacy cleanup once focused and aggregate output shapes
are stable.

### Work Group 5A — Aggregate Pipeline

- [ ] **Shared collection**
  - Change `GitRepo::observe_aggregate_evidence` and `RepoAggregate` to carry the new
    last-10 `RecentCommits` result collected through the same options pipeline, with
    local-only linking and structure-tier attribution.
  - Preserve the aggregate's one-history-observation contract: recent, source, and
    documentation families are projections of one captured result, not separate
    walks or collections.

- [ ] **Array projection**
  - Embed each commit family as the same bare array emitted by its focused command;
    remove `aggregate_commit_family_value` and all period/filter/root/package envelope
    surgery.
  - Rewrite aggregate schema/snapshot tests for last-10 behavior, old quiet commits,
    author/file-types/link fields, empty arrays, and identical sibling pruning.

- [ ] **Offline counters**
  - Re-run aggregate counter assertions and compare compatible before/after evidence,
    attributing the new containment visits while preserving zero fetches, zero
    provider requests, one history collection, one structure observation, and no
    full inventory/doc walk.
  - Validate budget exhaustion does not invalidate the aggregate JSON and represents
    unknown containment as `null`.

### Work Group 5B — Legacy Cleanup

- [ ] **API removal**
  - Delete the five legacy `get_recent_commits_*` free functions, `CommitDescSet`,
    post-hoc `filter_by_*` methods, aggregate second-pass attribution, and old
    duration-window constant.
  - Complete the monolith split into options, collect, render, and links modules;
    update public re-exports, doctests, integration tests, and stale behavior comments.

- [ ] **URL consolidation**
  - Remove every superseded library/CLI remote URL parser and ensure `repo hash`,
    recent commits, aggregate output, and existing preferred-remote callers resolve
    through the single commit-links implementation.
  - Use text search after graph analysis to catch dynamic/property-style callers
    before declaring the deleted symbols unused.

- [ ] **Dependency audit**
  - Remove dependencies made unused by deleting CLI Markdown/styled rendering, and
    update `sniff/docs/dependencies.md` only if the manifest dependency set changes.
  - Confirm no new `chrono-tz` or network dependency was introduced.

### Work Group 5C — Documentation

- [ ] **Topic contract**
  - Update the topic and schema docs to the actual non-generic builder, fixed-offset
    timezone behavior, exact payload optionality, three-state remote semantics,
    visit budget, renderer vocabulary, and `min(0)` merge file list.
  - Replace the dangling Darkmatter prose link with
    `biscuit-terminal/docs/components/prose.md` and document the accepted plain-mode,
    HTML/CSS, and package-attribution behavior changes.

- [ ] **CLI contract**
  - Rewrite `repo_recent-commits.md` and sibling source/documentation command docs for
    count-10 defaults, new/removed flags, branch-base semantics, exit-0 empties, and
    bare-array JSON; point styling details to the library topic.
  - Update examples, generated/help-facing documentation, README references, and the
    Sniff skill references whose architecture or public workflow changed.

**Validation checkpoint:** focused and aggregate JSON arrays are structurally
identical for the same options; legacy symbols and duplicate URL/rendering logic have
no callers and are deleted; docs contain no old envelope, 3-day default, `--action`,
UTC-calendar, or no-result-exit-1 claims; `just sanity`, `just lint`, and `just
doctest` pass in `sniff/`.

## Phase 6 — System Validation and Review Handoff

### Work Group 6A — Local Gates

- [ ] **L1 suite**
  - Run `just test` in `sniff/` so the library uses the `remote` feature and the CLI
    executes through its isolated fixture; confirm newly added tests are selected,
    not skipped or cfg'd out.
  - Run dependency-derived downstream L1 tests for Darkmatter and Worktree because
    their direct classifier calls intentionally inherit the HTML/CSS semantic change.

- [ ] **Terminal suite**
  - Run `just test-l2` in `sniff/` and require the available backend evidence; verify
    complete final frames with `capture_until`, including wrapped multiline reports,
    styles, OSC8 support/fallback, plain output, and clean JSON stdout.
  - Do not add L2 cases for behavior already proved by L1; retain only real-terminal
    assertions whose failure depends on terminal rendering.

- [ ] **Full gate**
  - Run `just all` in `sniff/` after focused failures are resolved, then manually
    exercise representative recent/source/documentation and aggregate commands in
    terminal, plain, Markdown/library, and JSON forms.
  - Verify empty queries exit 0; invalid selections and unknown package/area/branch
    inputs remain typed failures; no command performs a fetch or network request.

### Work Group 6B — Platform Evidence

These tasks may run concurrently after the local L1 suite is green.

- [ ] **Windows compile**
  - Run `just check-windows` from `sniff/` for test-target compile evidence, paying
    particular attention to `file://` URLs, path separators, fixed-offset local time,
    and clap behavior.
  - Treat this as compile evidence only; do not substitute it for native Windows
    runtime results.

- [ ] **Host matrix**
  - Inspect available `BUILD_*` variables, then use `just cross-check sniff --os
    linux`, `--os windows`, and `--os wsl` where provisioned; native Windows proves
    Windows behavior and WSL2 independently proves the Linux/archive path.
  - Reuse qualifying evidence per `{package, environment, tier}` cell and leave final
    authoritative coverage to the normal CI matrix rather than adding speculative
    workflows or cells.

- [ ] **Platform fixtures**
  - Confirm calendar boundaries, paths/links, non-ASCII metadata, rename/copy diffs,
    and empty/budgeted JSON behave identically at the contract level on each runtime.
  - If a failure is unique to one OS, consult and update the matching `os` skill
    reference with any newly learned repo-specific fact in the same implementation
    change.

### Work Group 6C — Final Audit

- [ ] **Graph review**
  - Refresh GitNexus if stale and run `detect-changes --scope all`; re-run with an
    untruncated result and inspect all HIGH/CRITICAL effects, especially classifier
    consumers, before handoff.
  - Use compare-to-main impact review to confirm the touched packages and validation
    scope match the implemented change; resolve or explicitly report any unexpected
    caller.

- [ ] **Contract review**
  - Trace every specification decision to implementation and at least one meaningful
    assertion, and verify comments/docs changed alongside every behavior-changing
    symbol.
  - Confirm no temporary compatibility shim, spike harness, generated artifact,
    credential, network fixture, or CLI-side business logic remains.

- [ ] **Review handoff**
  - Summarize implemented contracts, intentional breaking changes, work-counter and
    containment-budget evidence, test/OS evidence, and any CI-only evidence still
    pending.
  - Leave the feature directory active and mark the implementation ready for review;
    do not move it to `_completed` and do not run `just complete`.

**Validation checkpoint:** all available local, downstream, terminal, and host-matrix
evidence is green; GitNexus change analysis is complete and untruncated; the decision
trace contains no gaps; the implementation is complete and ready for author review.
