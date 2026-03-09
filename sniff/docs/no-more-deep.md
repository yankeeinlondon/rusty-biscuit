# No More `--deep`

## Overview

The `sniff` CLI should stop exposing `--deep` as a global flag.

`--deep` is too vague, it applies unevenly across subcommands, and in the current tree it causes two different classes of behavior:

- git-oriented enrichment that is described as remote-aware reporting
- dependency-oriented enrichment that queries package registries for newer versions

Those are different operations with different costs and different reporting surfaces. They should not share a single generic switch.

This document proposes replacing `--deep` with subcommand-scoped switches that name the expensive operation directly and require the CLI to actually report the additional data collected.

## Problem Statement

Today the CLI advertises `--deep` as “Enable deep git inspection (queries remotes for branch info)” in [args.rs](/Volumes/coding/personal/rusty-biscuit/sniff/cli/src/args.rs#L28), but the implementation does not line up cleanly with that promise.

Observed issues:

- The flag is global, so it appears available everywhere even when it has no meaningful reporting effect.
- `repo`, `filesystem`, `language`, and full JSON all share the same dependency-enrichment gate in [commands.rs](/Volumes/coding/personal/rusty-biscuit/sniff/cli/src/commands.rs#L265).
- `language --deep` currently does extra work without changing the report.
- Several `repo` modes (`--deps`, `--packages`, `--package`, `--package-area`, `--dirty-packages`, `--dirty-package-areas`) can currently pay enrichment cost without surfacing enriched data.
- `git` output claims some deep-only behavior, but `detect_git()` currently ignores its `deep` parameter in [git.rs](/Volumes/coding/personal/rusty-biscuit/sniff/lib/src/filesystem/git.rs#L856).
- The name `deep` gives no clue whether the expensive work is “fetch remote refs”, “query registries”, or something else.

## Goals

- Remove `--deep` from the CLI.
- Replace it with explicit, subcommand-scoped switches whose names describe the expensive operation.
- Only perform expensive work when the selected subcommand has a reporting surface for that extra data.
- Ensure every expensive switch changes text output, JSON output, or both.
- Make it obvious from help text why the switch may be slow.
- Align implementation, help text, tests, and docs.

## Non-Goals

- This document does not redesign `sniff git <remote>` remote inspection. That flow is already explicit because the user is directly asking for a remote report.
- This document does not require preserving `sniff --deep` behavior for the no-subcommand “full JSON” mode.
- This document does not require adding new enrichment to subcommands that do not have a clear output surface.

## Design Principles

### 1. Expensive operations must be named, not implied

Use verbs or concrete nouns that tell the user what will happen:

- `--refresh-remotes`
- `--latest-versions`

Avoid umbrella terms like `--deep`, `--full`, or `--enhanced`.

### 2. Expensive switches must have a reporting contract

If a switch triggers more work, the output must visibly include the newly discovered data.

Bad:

- run extra queries
- serialize nothing new
- hope the user infers that something happened

Good:

- add new JSON fields
- add new text sections or per-item annotations
- document exactly which fields appear only when the switch is used

### 3. Aggregate commands should reuse the same enrichment names

If `git` gains `--refresh-remotes`, `filesystem` should use the same flag for its embedded git section.

If `repo` gains `--latest-versions`, `filesystem` should use the same flag for its embedded repo section.

### 4. No silent “has no effect” flags

If a switch cannot affect the selected report shape, either:

- reject it with clap conflicts/requirements, or
- do not define it on that subcommand

We should prefer not defining the flag where it has no valid reporting surface.

## Proposed CLI

## Remove

- Remove global `--deep` from `Cli`.

## Add

### `sniff git --refresh-remotes`

Purpose:

- refresh local knowledge of remotes before rendering the local git report

What extra work it authorizes:

- fetch or otherwise refresh remote-tracking refs
- recompute local-vs-remote tracking status from refreshed data
- compute commit containment against refreshed remote refs if that feature is implemented

What extra reporting it must enable:

- remote default branch and branch inventory
- ahead/behind or “behind” reporting based on refreshed refs
- commit “synced to” or remote-containment reporting if implemented

Text output expectations:

- base output shows refreshed remote-tracking status when available
- remote branch counts appear only when refreshed data exists
- `-v` and `-vv` show progressively more remote branch detail and commit containment detail

JSON expectations:

- `remotes[].branches`
- `status.is_behind`
- `recent[].remotes` when containment is implemented

Suggested help text:

> Refresh remote-tracking data before reporting branch and sync status (may contact remotes)

### `sniff filesystem --refresh-remotes`

Purpose:

- same enrichment as `sniff git --refresh-remotes`, but for the embedded git section in the aggregate filesystem report

What extra reporting it must enable:

- the `Git Repository` subsection must visibly include the refreshed remote-aware fields

Notes:

- this should only affect the git subsection
- it should not imply dependency registry lookups

### `sniff repo --latest-versions`

Purpose:

- query package registries for current published versions of external dependencies

What extra work it authorizes:

- registry lookups for deduplicated dependency entries
- update classification (`is_updatable`, `has_major_update`)

What extra reporting it must enable:

- text: a clear update summary, not just a hidden marker
- JSON: latest-version fields and package-level update flags

Text output expectations:

- package rows continue to support concise markers
- add an explicit update summary legend or summary line
- at `-v`, show a short per-package summary such as `2 updates, 1 major`
- at `-vv`, optionally show a limited sample of `current -> latest` transitions for outdated dependencies

JSON expectations:

- `dependencies[*].latest_version`
- `dependencies[*].is_updatable`
- `dependencies[*].has_major_update`
- `packages[*].is_updatable`
- `packages[*].has_major_update`

Suggested help text:

> Query package registries for latest dependency versions and report available updates

### `sniff filesystem --latest-versions`

Purpose:

- same enrichment as `sniff repo --latest-versions`, but for the embedded repo/package subsection in the aggregate filesystem report

What extra reporting it must enable:

- the package list inside the filesystem report must visibly surface update availability

Notes:

- this should only affect the repo/package subsection
- it should not be accepted on commands that do not render repo/package data

## Commands That Should Not Get A Replacement Flag

### `sniff language`

Do not add a replacement for `--deep`.

Reason:

- the current `language` report only renders language breakdown data in [mod.rs](/Volumes/coding/personal/rusty-biscuit/sniff/cli/src/output/mod.rs#L356)
- dependency enrichment has no output surface there

### `sniff repo --deps`

Do not allow `--latest-versions` together with `--deps` or `--deps --ui`.

Reason:

- `--deps` reports internal workspace dependency relationships, not external package versions
- registry lookups do not make this report more complete

### `sniff repo --packages`, `--package`, `--package-area`, `--dirty-packages`, `--dirty-package-areas`

Do not allow `--latest-versions` on these focused output modes.

Reason:

- these modes intentionally collapse output to names/areas
- there is no place to report per-dependency or per-package update findings

### no-subcommand full JSON mode

Do not add a global replacement.

Reason:

- the design goal is to remove generic global enrichment
- users who want enriched filesystem or repo data should choose the relevant subcommand explicitly

## Reporting Contract

Every new expensive switch must satisfy the following contract:

1. The switch name describes the expensive action.
2. The expensive action is only executed when the switch is present.
3. The resulting report contains additional user-visible information.
4. Text and JSON behavior are both documented and tested.
5. If the switch cannot affect the selected output shape, clap rejects the combination.

## Library/API Design

The CLI should not continue routing these features through a single generic `deep` boolean.

## Recommended Direction

Replace broad `deep` plumbing with explicit enrichment options.

### Current state to unwind

- `Cli.deep`
- `SniffConfig.deep`
- `detect_filesystem(root, deep, commit_count)`
- `detect_git(path, deep, commit_count)`

### Proposed direction

Keep core detection cheap and local by default, and layer enrichment on top.

#### Base detection

- `detect_filesystem(root, commit_count)` or equivalent
- `detect_git(path, commit_count)` or equivalent

#### Explicit enrichment helpers

- `refresh_git_remote_report(...)`
- `enrich_dependency_versions(...)`

The names above are descriptive placeholders, not final API names.

The important part is the split:

- local detection remains local and cheap
- expensive enrichment becomes explicit and targeted

### Why this split is preferable

- It matches the user-facing CLI semantics.
- It prevents accidental enrichment from aggregate/global config.
- It makes testing easier because base detection and enrichment can be asserted separately.
- It removes the current mismatch where `detect_git()` accepts a `deep` parameter but does not honor it.

## Detailed Behavior By Subcommand

## `git`

### Without `--refresh-remotes`

- only report data available from local repository state
- do not fetch or refresh remotes
- do not promise remote completeness

### With `--refresh-remotes`

- refresh remote-tracking state
- recompute remote-aware fields
- report those fields in text and JSON

### `git --hash`

No new flag is required in phase 1.

Rationale:

- this output is commit-centric
- if remote containment is eventually reported for hash mode, it can be considered later as a separate explicit option

## `filesystem`

### Without enrichment flags

- render the current aggregate local report

### With `--refresh-remotes`

- only enrich the git subsection

### With `--latest-versions`

- only enrich the repo/package subsection

### With both

- both operations are allowed
- both subsections must visibly reflect the additional data

## `repo`

### Default repo view

Allow `--latest-versions`.

This mode has the best reporting surface for update availability because it already renders packages and package metadata.

### Specialized repo modes

Reject `--latest-versions` for modes whose output cannot surface update data.

## Output Design

## Text output for `--latest-versions`

The current `*` marker in [filesystem.rs](/Volumes/coding/personal/rusty-biscuit/sniff/cli/src/output/filesystem.rs#L758) is a useful compact signal, but by itself it is too implicit for an expensive switch.

Phase 1 text requirements:

- keep the marker
- add a summary line or legend with package counts
- ensure the user can tell what was checked

Recommended text additions:

- `Checked registry versions for 187 unique dependencies`
- `12 packages have updates available`
- `4 packages have major updates available`

Recommended verbose additions:

- `-v`: per-package counts
- `-vv`: sample dependency transitions such as `clap 4.5.50 -> 4.5.60`

## Text output for `--refresh-remotes`

The current remote section should only show refresh-derived details when the refresh flag was actually used.

Recommended base additions:

- `Remote origin: GitHub (refreshed, 15 branches)`
- `Behind: origin`

Recommended verbose additions:

- `-v`: branch list excluding default branch
- `-vv`: commit containment annotations where available

## JSON output

JSON should continue to use field presence as the signal that extra work was performed.

Rules:

- fields derived only from explicit enrichment should be omitted when enrichment was not requested
- field presence should be stable and documented
- JSON should never imply data freshness that did not occur

## Error Handling

## Remote refresh

If remote refresh fails:

- keep the local git report
- add a warning in text mode
- omit refresh-only JSON fields or include structured warning metadata if that pattern already exists

Do not fail the entire command unless the user explicitly chose a “must succeed” mode in the future.

## Registry lookups

Keep the current graceful degradation behavior:

- failed lookups do not fail the whole command
- missing latest-version data remains absent

But make the reporting honest:

- if text mode says updates were checked, it should also indicate partial failures when relevant

## CLI Compatibility and Migration

## Help and docs

Update:

- `sniff/cli/README.md`
- `sniff/lib/README.md`
- any help snapshots
- tests that mention `--deep`

## Compatibility strategy

Two acceptable rollout options:

### Option A: immediate removal

- remove `--deep`
- fail with clap “unexpected argument `--deep`”
- document the replacement switches in release notes

### Option B: one short deprecation window

- keep `--deep` hidden or deprecated for one release
- map it to a clap error with targeted guidance:
  - `sniff git --deep` -> `use --refresh-remotes`
  - `sniff repo --deep` -> `use --latest-versions`
  - `sniff filesystem --deep` -> `use --refresh-remotes`, `--latest-versions`, or both
  - `sniff language --deep` -> no replacement

Recommendation:

- use Option B only if downstream usage is expected to be significant
- otherwise prefer Option A to avoid carrying more ambiguity forward

## Implementation Plan

## Phase 0: Document the contract

- Add this design doc.
- Update team guidance: no new vague enrichment flags.

## Phase 1: Remove global CLI plumbing

- Remove `deep` from `Cli`.
- Remove global help text and completion coverage for `--deep`.
- Remove `deep_enabled` command routing in [commands.rs](/Volumes/coding/personal/rusty-biscuit/sniff/cli/src/commands.rs#L148).

Acceptance criteria:

- `sniff --help` no longer mentions `--deep`
- `sniff --deep ...` is rejected or redirected according to the chosen compatibility strategy

## Phase 2: Add explicit git refresh flags

- Add `--refresh-remotes` to `git`.
- Add `--refresh-remotes` to `filesystem`.
- Implement explicit remote refresh and remote-aware field population.
- Ensure text and JSON only surface refresh-derived fields when refresh was requested.

Acceptance criteria:

- `sniff git --refresh-remotes` changes reporting versus `sniff git`
- `sniff filesystem --refresh-remotes` changes the embedded git section versus baseline
- tests cover text and JSON field presence/absence

## Phase 3: Add explicit dependency-version flags

- Add `--latest-versions` to `repo`.
- Add `--latest-versions` to `filesystem`.
- Remove dependency enrichment from `language`.
- Reject invalid combinations for repo modes that cannot report update data.

Acceptance criteria:

- `sniff repo --latest-versions` visibly reports update findings in text and JSON
- `sniff filesystem --latest-versions` visibly reports update findings in text and JSON
- `sniff language --latest-versions` does not exist
- `sniff repo --deps --latest-versions` is rejected

## Phase 4: Tighten renderer/reporting behavior

- Upgrade text output so expensive flags produce explicit summaries, not only subtle markers.
- Make JSON field presence match enrichment requests exactly.
- Add partial-failure warnings where appropriate.

Acceptance criteria:

- text output clearly states what extra checks were performed
- JSON is deterministic about enrichment-only fields

## Phase 5: Clean up library terminology

- Remove or deprecate `SniffConfig.deep`.
- Replace generic boolean plumbing with explicit enrichment calls or option structs.
- Align rustdoc comments with actual behavior.

Acceptance criteria:

- no public API comments describe `deep` behavior that no longer exists
- the library surface no longer encourages callers to think in terms of “deep”

## Testing Plan

Add tests for:

- help output no longer mentioning `--deep`
- invalid use of removed/deprecated `--deep`
- `git` baseline vs `git --refresh-remotes`
- `filesystem` baseline vs `filesystem --refresh-remotes`
- `repo` baseline vs `repo --latest-versions`
- `filesystem` baseline vs `filesystem --latest-versions`
- invalid combinations like `repo --deps --latest-versions`
- JSON field absence when enrichment flags are not used
- JSON field presence when enrichment flags are used

## Open Questions

1. Should `--refresh-remotes` perform an actual fetch from remotes, or only recompute from locally cached tracking refs?

Recommendation:

- prefer actual refresh if the name remains `--refresh-remotes`
- if we choose local cached refs only, use a different name such as `--remote-refs`

2. Should text mode for `--latest-versions` show only package-level summaries in phase 1, or also sample dependency transitions?

Recommendation:

- package-level summary in phase 1
- sample transitions at `-vv`

3. Should no-subcommand full JSON gain a future explicit enrichment mode?

Recommendation:

- not in this change
- force users to pick the subcommand that matches the expensive work they want

## Recommendation Summary

- Remove global `--deep`.
- Add `git --refresh-remotes`.
- Add `filesystem --refresh-remotes`.
- Add `repo --latest-versions`.
- Add `filesystem --latest-versions`.
- Do not add a replacement on `language`.
- Reject enrichment flags on output modes that cannot report the enriched data.
- Make text output explicitly state what extra information was gathered.
- Make JSON field presence match the requested enrichment exactly.
