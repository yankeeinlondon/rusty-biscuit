# Sniff Request Architecture

Use this reference when changing detection plans, shared filesystem work,
subprocesses, host capabilities, or request cost.

## Contents

- [Request planning](#request-planning)
- [Filesystem observation](#filesystem-observation)
- [Change classification and committed diffs](#change-classification-and-committed-diffs)
- [Package topology](#package-topology)
- [Lockfile observation pipeline](#lockfile-observation-pipeline)
- [Programs and subprocesses](#programs-and-subprocesses)
- [Network defaults](#network-defaults)

## Request planning

Top-level OS, hardware, network, and filesystem domains run in scoped workers
through `detect_with_plan`. Requests choose what evidence is needed before work
starts. A detector must not widen another domain simply because a Git handle or
repository root is available.

`DetectionPlan::default()` enables all domains at safe defaults. In particular,
it uses full OS identity with NTP disabled, so Tier-1 detection performs no
implicit network probe.

## Filesystem observation

`FilesystemObservation` is the request-scoped owner of Git discovery. Its
clones share one serialized repository handle and retain all three acquisition
outcomes: repository present, explicit absence, or the original typed failure.
`repository_identity()` exposes the worktree root, worktree-specific Git
directory, shared common directory, and bare-repository flag without exposing
the underlying `GitRepo`.

Two additive seeded entry points consume this owner:

- `detect_filesystem_with_observation` executes one filesystem request with an
  observation acquired for the same root.
- `detect_with_plan_and_filesystem_observation` executes a detection plan and
  returns `ObservedSniffResult`, preserving the same observation for later Git
  or file-change queries.

The ordinary `detect_with_plan` and `detect_filesystem_with_request` APIs stay
ambient compatibility paths and acquire an observation when Git is requested.
Seeded execution performs no upward Git discovery; Git identity, shared-walk
root selection, and later `detect_git` / `detect_file_changes` projections all
reuse the retained handle. A retained absence or failure is projected without
retrying discovery.

`FilesystemObservation::for_root` rebases a present observation to another
directory in the same worktree without Git discovery. It uses canonical,
platform-aware containment and rejects an intervening nested `.git` boundary.
Bare repositories, absent observations, and failed observations are exact-root
only. Linked worktrees remain distinct because their worktree-specific Git
directories differ even when they share a common Git directory.

Full repository detection builds one request-scoped
`FilesystemSystemView`. It retains selected evidence rather than `DirEntry`
objects or file bodies. `RepoEvidence::from_view` is the bridge into repository
detection.

Walk scope comes from consumers:

- Formatting-only requests probe `.editorconfig` directly and start no walker.
- Structure-only repository requests start no shared descendant walker.
- Inventory-only requests walk the resolved package root.
- Full repository and repository-wide docs requests walk the repository root.

Inventory accepts at most `MAX_FILES`. An inventory-only walk stops globally at
the cap. A combined walk continues for active manifest/docs consumers while the
inventory projection reports `truncated` and `limit`. A truncated subset is not
stable across runs; complete output is sorted and deterministic.

## Change classification and committed diffs

`filesystem::path_kind::classify_path` is the only path classifier. It maps a
path to exactly one `ChangeCategory` with first-match precedence: CI/CD path
rules, then registry exact file name, basename pattern, the `.html`/`.htm`
web-asset override, extension, then `other`. `is_source_code_path` and
`is_documentation_path` are wrappers, and darkmatter and worktree call them
directly. Any change is CRITICAL in GitNexus, so run their downstream tests.
HTML and CSS are web assets, not source code. Angular `.component.html` is
source code.

Committed-tree diffs have two paths:

- The public `get_commit_files*` family keeps rename tracking **disabled**, so a
  rename is a delete/add pair.
- The crate-private `discovery::committed_file_changes_with_cache` enables
  Git-default rename and copy tracking (`FromSetOfModifiedFiles`) and adds line
  counts. Binary files and non-blob entries get `None`, never `0/0`. A
  rewrite reuses its similarity stats, so computing line counts does not
  diff the pair a second time.
- gix's rewrite tracker drops a copy's **source** modification from the change
  list. `restore_copy_source_modifications` re-adds it from the parent tree.
  Keep that step if you touch copy detection.

## Recent commits and commit links

`RecentCommits::collect(&GitRepo, &RecentCommitsOptions)`
(`git/recent_commits/collect.rs`) is the one collection pipeline:

- Filters apply **during** the walk. Message filters (operation, scope,
  author) run first. Path filters (package, area, file type) use the cheap
  untracked file listing. A count selection stops at N *matches*.
- Only survivors are enriched: the rename-aware diff with line counts,
  structure-tier attribution, and one linking pass. Filtering and
  attribution share one per-path resolver in `PackageCatalog`: the owner
  comes from `PackageOwnershipIndex`, and the area is the deepest area
  directory strictly containing the path (`PackageAreaIndex`, also behind
  `RepoInfo::package_area_for_dir`), whether or not a package owns it; only
  a path no area directory contains falls back to its owner's area. Package
  and area boundaries are independent: a shared file under `worktree/fixes/`
  has area `worktree` and no package, and a file in `darkmatter/dmls/` (both
  the `dmls` package and a nested area) has package `dmls` and area
  `darkmatter/dmls`. The root area `""` is never a directory fallback. The work counters prove this: `git.file_diffs` covers only
  survivors, with zero inventory, doc, or enrichment counters.
- `--branch` resolves in order: local branch, `refs/remotes/<name>`, then
  `<remote>/<name>` over *configured* remotes. Nothing is fetched.

`git/commit_links.rs` is the only remote URL and containment authority.
`repository_link` and `commit_url` build browser URLs. Do not add another URL
parser; `remote::parse_remote_url` is feature-gated and unusable here. Linking
rules:

- Linking walks remote-tracking tips in `preferred_remote_order`. Within a
  remote, it walks the `refs/remotes/<r>/HEAD` default branch first.
- All walks share `COMMIT_VISIT_BUDGET` (5,000,000) through
  `remote_refresh::walk_ancestry`, the same walker the deep tier uses.
- `remote` is `false` only when every walk completed. An exhausted budget or an
  unreadable ancestor gives `null`.
- `commit_browser_url` and CLI `repo git-status` link a commit only when a
  remote-tracking ref contains it.
- Older single-purpose parsers remain outside linking and were deliberately
  not consolidated: `GitInfo.org`/`repo` (`types.rs::parse_org_repo`),
  provider host classification (`extract_remote_host`), and the identity
  basename (`repo/identity.rs`). Do not build commit or repository links from
  them.

The bare `sniff repo --json` aggregate collects once with default options
through the crate-private `RecentCommits::collect_observed`. It runs after
the Git and repo-detection threads join, so it can reuse the request's detected
`RepoInfo` as the package catalog (zero extra manifest parses) and the branch
`RefSnapshot` for linking (still one `git.ref_walks`). The three commit families
are `projected` views of that one collection and equal the focused commands'
default `--json` arrays. Expect the aggregate's `git.commit_visits` to grow on a
branch with unpushed commits: proving `remote: false` walks every
remote-tracking tip (about 153k visits on this monorepo on 2026-09-17, against
13k before linking). `aggregate_view::tests::commit_families` pins the deltas.
The legacy `CommitDescSet` / `get_recent_commits_*` API is gone.

Text reports (`git/recent_commits/render.rs`) come from one layout walk folded
three ways: `to_prose`, `to_markdown`, and `to_plain`. The library owns every
report byte, including sibling headings and `file://` links.

- Dynamic text is backslash-escaped for Prose and Markdown. Prose has no
  backtick escape (it renders the backslash), so only Markdown escapes
  backticks.
- Link targets percent-encode `( ) space < >`.
- `RecentCommits::projected` is the single file-pruning authority for text
  and JSON. `to_json()` ignores every display option.

## Package topology

`RepoInfo.packages` is the canonical catalog. `MonorepoLayer.packages` stores
repo-relative references to that catalog rather than duplicate package data.

Each layer has one membership authority and zero or more task orchestrators.
An orchestrator alone is not a monorepo. Package `standard` and `provenance`
identify the membership authority and discovery method.

Structure requests collect membership and minimum identity only and read no
lockfile, so provenance stays manifest-derived. Use a focused request for
selected manifest facts and full mode for inventory-backed enrichment and
lockfile corroboration; `with_lockfile_provenance` opts any tier in or out.

## Lockfile observation pipeline

Every `MonorepoLayer` carries a required `lockfile: LockfileObservation`
(`filesystem/repo/lockfile/`), which replaced `lockfile_match` and must not
return. The manifest stays the membership authority: an observation never adds
or removes packages or changes ownership. Its only side effect is provenance,
and only an exact `match` upgrades the layer and the seeds it owns
(`upgrade_owned_seed_provenance`). Cargo reports `members_present` /
`members_missing` with `subset_only`, never `match`, so it never upgrades.
The detector's raw Cargo seeds hold an included *and* an excluded seed for a
directory matched by both `members` and `[workspace].exclude` (they merge only
later), so `lockfile::cargo::compare` skips every path with an excluded seed;
checking `is_excluded` per seed reports a false `members_missing`.

`lockfile::sources::source` is the single authority-to-lockfile table:

| `Source` | Standards | Behavior |
|---|---|---|
| `Candidates` | pnpm, npm, Yarn, Bun, uv, Cargo | First present file in precedence order is selected (`npm-shrinkwrap.json` > `package-lock.json`, `bun.lock` > `bun.lockb`); never retry a lower-priority file after the selected one fails |
| `Configured` | Rush | `lockfile/rush.rs` picks the manager's lockfile from `rush.json`; only the single pnpm workspace layout compares (importer base `common/temp`) |
| `Fallback` | Go, Gradle, Bazel | Metadata only, never opened: `unverifiable` + `no_membership_data`; Bazel without `MODULE.bazel` is `not_applicable` |
| `NotApplicable` | Maven, .NET, Pants, Buck2, orchestrators, `Unknown` | `no_lockfile_source`, or `unknown_standard` for `Unknown` |

Adding a standard means adding its row; the `sources` test pins all 18.
`observe_layer_lockfile` applies the spec's precedence in order: no source,
metadata failure, absence, declined request (`not_requested`), metadata-only
fallback, then parse and classify. Only the last step reads content.

`ManifestStore` owns every lockfile touch for the request:

- `lockfile_presence` probes each normalized path once and caches
  `Present`/`Absent`/`Failed(kind)`. A missing file or parent is absence; any
  other metadata error is `unreadable` + `metadata_failed`, never absence.
- `lockfile(path, format)` gates on presence, then reads and parses at most
  once, caching typed `LockfileFailure`s (absent, metadata, read, parse) as
  well as successes. A directory in place of the file is a read failure.
- `Cargo.lock` goes through `cargo_lock`, one shared parse serving both
  dependency-version enrichment (`CargoLockVersions`, byte-for-byte legacy
  results) and corroboration.
- The metadata-failure test seam is `lockfile::test_seam::fail_metadata`;
  Unix mode bits are not a portable way to inject one.

Membership comparison (`lockfile/membership.rs`) is lexical: both sides become
`/`-separated paths relative to the layer root, the root is excluded from both
sets, `..` resolves against the format's base, and no lockfile path is ever
opened or canonicalized. Do not intersect lockfile paths with manifest members
first; that erases `extra`. A layer is `incomplete_manifest_discovery` (R10)
when it has no matching detector outcome, when the outcome's `incomplete` flag
is set, or when a member's own manifest is missing or fails to parse (Cargo:
its identity cannot be resolved). The flag comes from `glob.rs`'s
`MembershipExpansion`: a dropped unsupported or unparseable pattern, a walk
failure under a glob's walk root (the shared walk records its failures in
`manifest_walk_errors`), or, for Cargo and Rush only, a declared literal
member that does not exist. The Node tools and uv expand literals as globs, so
a missing one matches nothing. Detectors also set it when a declared member
entry is not a string, or when a present member field is not a list at all
(`DeclaredPatterns::has_invalid`, `DeclaredPatterns::invalid()`: Cargo
`members` and `exclude`, uv `members`, pnpm `packages`, `package.json`
`workspaces` in both forms, Rush `projects` and a project without a string
`projectFolder`); never filter such an entry out silently, and keep the layer
even when every entry is invalid or the field has the wrong type. An absent
field, a null pnpm `packages`/`package.json` `workspaces`, and the object form
without a `packages` key still declare nothing. The npm lockfile parser is
stricter: its root-record `workspaces: null`, and `resolved: null` on a
`link: true` record, are `unreadable` + `parse_failed`, because only an
omitted field means "no locked declarations" or "no link target". An explicit uv `members = []` still forms a
root-only layer (empty sets can `match`); only an absent `members` array forms
none. A uv layer lists its root among `packages`, so
`membership_resolves_non_degenerately` treats one uv package as degenerate and
the root-only workspace is not a monorepo. Incompleteness blocks only the comparison
step; every earlier status wins. Nested dispatch treats a malformed
`package.json` as no npm/Yarn/Bun root (`ManifestStore::npm_unless_malformed`),
so a malformed member reaches its layer's corroboration as incomplete; a
malformed workspace root manifest, or an unreadable one anywhere, still fails
detection.

Standalone Poetry, PDM, and Composer lockfiles are repository-level
`RepoInfo.standalone_lockfiles` entries (`lockfile/standalone.rs`), never
layers: one probe per tool at the repository root and each unique package
root, no walk, no read, and no entry for an absent file. A single-package root
reaches the probe only through root-package synthesis
(`synthesize_root_package_repo_with_store`), which accepts a lone
`composer.json` as well as the `PackageEcosystem` markers; filesystem
detection synthesizes only when the repo request carries `details`.

`bun.lock`, `rush.json`, and Rush configuration are JSON with comments. Parse
them only through `filesystem::repo::jsonc::from_str` (strict `jsonc-parser`
options: comments and trailing commas, nothing looser), never `serde_json`.

## Programs and subprocesses

Program categories share an executable index. Prefer eager builders for bulk
lookup. `_only` builders exclude platform fallback layers, which changes
Windows App Paths behavior.

All subprocesses go through `process::run_with_timeout` or
`process::run_command_with_timeout`. These helpers own deadlines, concurrent
pipe draining, process-tree termination, and reaping on macOS, Linux, and
Windows. Preserve caller cwd and environment when using the builder form.

`process` is public only for `configure_detached_child`, which other packages
(`playa`, `biscuit-speaks`, `worktree-cli`) use to set up an intentionally
detached, unbounded child. It sets a new process group on Unix and, on Windows,
clears `HANDLE_FLAG_INHERIT` on this process's stdio before setting the
detached creation flags. The caller still sets null stdio. Sniff's own spawns
never use it.

Batch service enrichment with bounded chunks. A failed or timed-out chunk
degrades only its own services; it must not discard healthy chunks.

## Network defaults

WAN IP is opt-in through the network request. It reuses one blocking client and
queries fallback endpoints sequentially, stopping after success. Strictly parse
the body as `IpAddr`; do not include a response body in errors or counters.

Remote Git refresh is explicit. Ordinary branch, aggregate repository, and
conflict-prediction paths use locally known refs and perform no fetch.
