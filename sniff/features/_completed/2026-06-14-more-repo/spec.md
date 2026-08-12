---
reviewed: true
status: ready for planning and implementation
---

# More Repo Feature

In this feature we will:

- Modify the structure of how the CLI presents software
- Add **Test Runner** infrastructure ([details](./test-runner-strategy.md))
- Add new CLI commands
- Sort out the messy `sniff repo --json` structure

This feature adds more subcommands to the `sniff repo` CLI and extends the Sniff library where needed.

- remember that the CLI should own reporting but the library should always own business logic and functionality
- use the 'cli', 'rust-devops', and 'sniff' agent skills

## Sniff Software

`sniff software` replaces the current top-level installed-programs surface. It is the logical parent for the existing program aggregate plus every program category:

- `sniff programs`
- `sniff editors`
- `sniff utilities`
- `sniff language-package-managers`
- `sniff os-package-managers`
- `sniff tts-clients`
- `sniff terminal-apps`
- `sniff audio-players`
- `sniff notification-helpers`
- `sniff agents`
- `sniff software test-runners` (new; see [Test Runners Design](./test-runner-strategy.md))

These SHOULD be subcommands of `sniff software`:

- `sniff software` (aggregate, equivalent to today's `sniff programs`)
- `sniff software editors`
- `sniff software utilities`
- `sniff software language-package-managers`
- `sniff software os-package-managers`
- etc.

This is a hard break: the old top-level paths (`sniff programs`, `sniff editors`, `sniff utilities`, `sniff language-package-managers`, `sniff os-package-managers`, `sniff tts-clients`, `sniff terminal-apps`, `sniff audio-players`, `sniff notification-helpers`, `sniff agents`) are removed entirely. There are no backward-compatible aliases, so the only invocation forms are `sniff software` and `sniff software <subcommand>`.

Reader note: the earlier draft listed only seven categories and accidentally omitted the two existing package-manager categories and the aggregate `programs` command. This spec intentionally rehomes the whole installed-programs surface so the CLI does not keep two competing nouns for the same feature.


## Adding Testing Runners

We need to add some library code to better track and report on the "test runner(s)" that a repo is using:

- the primary design document for this is [Test Runners Design](./test-runner-strategy.md)

### `sniff repo test-runner` CLI

Determines what test runner is declared by the current repo/package context. Host installation is reported by `sniff software test-runners`; this command reports repository usage.

- if this is a non-monorepo or a package in a monorepo then report that package's runner set
- if this is a monorepo:
    - if we're in a package-area then we should evaluate the test runners used across the contained packages
        - if they are all the same then just report the singular test runner
        - if there is variance across the packages then just report a unique list of test runners (csv,list,md-list)
    - if we're at the root of the monorepo then we should again evaluate across all packages
        - if all use the same test runner then just list it in the singular

The design decisions in `test-runner-strategy.md` are accepted for v1:

- `sniff software test-runners` searches project-local bins as well as global `PATH`, and reports `availability` (`installed`, `local`, `via_parent`, `not_found`) instead of a bare boolean.
- `sniff software test-runners` is **report-only**: unlike the other eight software categories it has no `install` / `install-plan` action. Test runners do not fit the host-install model — class B/C runners are subcommands of a parent tool (`cargo test`, `go test`, `dotnet test`) with nothing to install on their own, and class D runners are vendored per-project (`node_modules/.bin`, `vendor/bin`) rather than installed globally. The leaf reports availability only.
- package-manager global bin directories that are not on `PATH` are deferred to a follow-up.
- built-in ecosystem defaults are reported with `source: EcosystemDefault`.
- orchestrators such as `tox` and `nox` are reported with `kind: orchestrator`.
- `Package.test_runners` uses typed `TestRunnerUsage` values with evidence, not strings.
- v1 includes the full runner catalog from the strategy document.

## New Local Commands

### `sniff repo branches`

- lists out the local branches found in the git repo
- marks branches that are also represented by a locally known remote-tracking branch
- default behavior does not fetch from the network; if this command gets a refresh option, it must be explicit (for example `--refresh-remotes`) and follow the same non-interactive `GIT_TERMINAL_PROMPT=0` constraints as existing git remote refresh code
- text output is rendered through `biscuit-terminal` (`Prose` or a reusable renderable component), not hand-written ANSI
- stdout carries the branch list; stderr may carry a short legend such as `* indicates the branch is available on a remote git host`, but `--json` stdout must remain valid JSON and should not emit the legend
- JSON shape is an array of branch objects, not decorated text:

```ts
type BranchInfo = {
    name: string;
    current: boolean;
    sha: string | null;
    remote_represented: boolean;
    upstream: string | null;
    ahead: number | null;
    behind: number | null;
};
```

### `sniff repo package-manager`

Provides the name of the package manager of the CWD:

- if this is a non-monorepo or a package in a monorepo then this is always just a singular value
- if this is a monorepo:
    - if we're in a package-area then we should evaluate the package managers used across the contained packages
        - if they are all the same then just report the singular package manager
        - if there is variance across the packages then just report a unique list of package managers (csv,list,md-list)
    - if we're at the root of the monorepo then we should again evaluate across all packages
        - if all use the same package manager then just list it in the singular

The package-manager collapse logic should live in the library and be shared with `sniff repo test-runner`. The CLI only selects the output format and renders the value.

### `sniff repo dependencies`

This command reports external dependencies declared by repo packages.

Reader note: an earlier draft proposed renaming the existing `sniff repo deps` command to `sniff repo packages`. That conflicts with the already-established `sniff repo packages` contract, which lists package names and is used for shell automation. Keep `sniff repo packages` unchanged. Rename the existing internal workspace dependency graph command from `sniff repo deps` to `sniff repo package-dependencies`, and use `sniff repo dependencies` for external dependencies.

- we already return this kind of information but the structure of this information needs to be improved at the `sniff repo` level and with that re-structuring the reporting on "external-dependencies" will be easier
- there are the following sub-types of external dependencies:
    - `dev-dependencies`
    - `peer-dependencies`
    - `optional-dependencies`
    - `dependencies`
- each of these sub-types will be filterable with a CLI switch (e.g., `--dev-dependencies` shows only development dependencies, etc.)
- `sniff repo package-dependencies` keeps the current internal monorepo graph behavior, including the Mermaid `--ui` rendering path
- `sniff repo deps` is removed as part of the hard break; there is no alias

## New Remote Commands — deferred to a future feature

The remote commands (`sniff repo ci-cd` / `ci` with `id` / `last` / `list`, and `sniff repo issues`) are **deferred to a dedicated future feature**. That feature will fully specify them with 4-provider parity (GitHub, GitLab, Gitea, Bitbucket) and acceptance criteria, and will also cover back-porting the improved CI/CD reporting to `sniff repo remote`.

They are carved out here because `sniff repo issues` has no body or acceptance criteria yet, and `ci-cd id` / `last` require a new provider-trait method (a `get_workflow_run`-style single-run + "last" primitive) implemented across all four providers — the current `RemoteProvider` trait only exposes `list_workflow_runs(limit)`.

## Fix

### `sniff repo is-monorepo` — owned by `2026-06-16-monorepo-cli` (D5)

The focused `sniff repo is-monorepo` leaf is owned and already delivered by the `2026-06-16-monorepo-cli` feature (decision D5). Its redesign — the `false`/label STDOUT text, the predicate exit code, the `--no-error` switch, and the snake_case `{ is_monorepo, authority, orchestrators[] }` JSON — is therefore **out of scope here**. The `installed` / `binary` host-probe idea from an earlier draft of this spec is dropped and not carried forward.

### Fix `sniff repo version`

This needs to work across supported package ecosystems, but it is currently inconsistent.

- Seems to work in a Typescript project
- Doesn't work in Rust (including this monorepo)

Implement version detection in the library, not in the CLI. The command should inspect the repo root / package root manifest selected by existing repo detection and return the manifest version when the ecosystem has one:

- Cargo: `[package].version` in the root package manifest, or the workspace package that represents the root when the root is a package
- Node: `package.json.version`
- Python: `pyproject.toml [project].version`, then common tool-specific fallbacks only if already parsed by repo detection
- Go: `null` unless the repo has an explicit version source already modeled by Sniff
- JVM/.NET/PHP/Ruby/Elixir: use the ecosystem manifest version when the parser added for this feature can read it safely; otherwise return `null`

`sniff repo version --json` keeps the focused leaf shape `{ "version": string | null }`. A missing version **is** an error: the command exits with a nonzero status when no version is found (text mode prints nothing; `--json` still emits `{ "version": null }` on stdout). A `--no-error` flag removes the nonzero exit code while still returning `null` (JSON) / empty output (text) on stdout, for callers that treat absence as a normal outcome.

### Fix Base JSON payload for `sniff repo --json`

The data returned by bare `sniff repo --json` is embarrassingly poorly structured. This section is the redesign guide.

### Diagnosis (measured)

On this repo a single `sniff repo --json` produces **~2.48 MB** of JSON across **38 top-level keys**. The weight is not "lots of useful data" — it is duplication. The serialized byte cost concentrates in five keys:

| Key | ~bytes | Why it's that big |
|-----|-------:|-------------------|
| `recent-commits` | 771 KB | embeds **87 commits with full per-commit file lists** *and* a **complete 67-package catalog** |
| `structure` | 528 KB | embeds the **complete 67-package catalog** (every field of every `Package`) |
| `deps` | 119 KB | embeds the **same 67 packages** again (dependency-projected fields) |
| `documentation-changes` | 113 KB | 53 commits with full per-commit file lists |
| `source-code-changes` | 99 KB | 39 commits with full per-commit file lists |

The full `Package` catalog (`name`, `path`, `languages`, `dependencies`, `documentation`, `configuration`, `ecosystem`, `file_associations`, `package_managers`, `discovery_sources`, …) is serialized **three times** — in `structure.packages`, `deps.packages`, and `recent-commits.packages` — even though the top-level `packages` key already lists the package *names* (and is the correct, cheap representation for an aggregate). Likewise the full worktree set is serialized **twice in two different shapes** (top-level `worktrees` and `git-status.worktrees`).

The aggregate is assembled key-by-key in `sniff/cli/src/output/repo_json.rs` (the bare-`repo` builder around lines 640–840). Each helper is individually fine for its *focused* subcommand (`sniff repo deps --json` in the current implementation, renamed to `sniff repo package-dependencies --json` by this feature; `sniff repo git-status --json`; etc.). The problem is that the aggregate re-emits the heavy whole-scope objects that those focused commands return, instead of contributing a lean projection. The fix is mostly in the aggregate builder and a few `serde` shapes — not a rewrite of the library.

### Design principles

The bare `sniff repo` aggregate is meant to be a **complete information rollup of its informational children** — that is the right instinct, and we keep it. The redesign does **not** drop information; it removes *duplication and noise* so that the same completeness fits in a fraction of the bytes. Concretely:

1. **Complete, but each fact once.** Every informational child still contributes its facts, but a package, a worktree, a branch, or a commit's file list is serialized exactly once. Everything else references it by name/sha.
2. **No property without variance.** Drop any field whose value is fully determined by its key (e.g. `scope: "dirty"` under the `dirty-*` key) or is constant across the run (e.g. `base_branch: "main"` repeated 15×).
3. **Aggregate ≠ verbatim concatenation of focused commands.** A focused child command (`repo package-dependencies`, `repo git-status`, `repo recent-commits`) may return a rich/heavy shape; the aggregate contributes a **lean projection** of that same data, not a re-emission of the whole scope object.
4. **`snake_case` keys, consistently.** Today the aggregate mixes `kebab-case` top-level keys (`dirty-source-code`, `git-status`) with `snake_case` inner keys. Standardize on `snake_case`.
5. **Empty means empty array** — never an error string, never a `{kind,scope,paths:[]}` envelope.

> **Sequencing note (supersedes monorepo-cli D5's byte-identical aggregate guarantee).** `2026-06-16-monorepo-cli` (D5) deliberately kept the bare aggregate's `is-monorepo` member byte-identical (an unwrapped `"is-monorepo": bool`). This redesign lands **after** monorepo-cli and intentionally reshapes that aggregate: as part of the `snake_case` standardization in principle #4 above, the aggregate's `is-monorepo` member is renamed to snake_case `is_monorepo` (see the consolidated `SniffRepo` type below). This is a sequenced supersession, not a contradiction — D5's guarantee held until this feature lands.

### Child subcommand taxonomy — what the aggregate contains

`sniff repo` has ~40 children. "Contain everything from the children" is the goal *for the informational ones* — but several children are queries, network calls, or parameterized lookups that legitimately do **not** belong in a no-arg, no-network aggregate. Classify them explicitly:

**A. Repo-wide facts — include (the core of the aggregate).**
`name`, `version`, `is-monorepo`, `package-count`, `language`, `root`, `packages`, `package-areas`, `structure`, `package-dependencies`, `dependencies`, `package-manager`, `test-runner`, `worktrees`, `branches` (new), `git-status`, `recent-commits`, `source-code-changes`, `documentation-changes`, and the change-scope families. These describe the repository itself and are identical regardless of where in the tree you invoke them.

**B. Context / cwd-relative queries — include, but keep them distinct from repo-wide facts.**
`package`, `package-area`, `area`, `package-root`, `package-area-root`, `worktree` (current), `is-current-package-area-dirty`, `package-area-has-source-code-changes`. These answer **"where am I right now?"** — their values change with the working directory, unlike group A. They are cheap and worth including, but mixing them flat with repo-wide facts is what makes the current 38-key blob feel arbitrary. **Recommendation:** group them under a single `context` object so a consumer can tell "this is about my cwd" from "this is about the repo":

```ts
type SniffRepo = {
    // ... group A repo-wide facts ...
    context: {
        package: string;
        package_area: string;
        area: string;
        package_root: string;
        package_area_root: string;
        worktree: string | null;                       // current linked worktree, null in main
        is_current_package_area_dirty: boolean;
        package_area_has_source_code_changes: boolean;
    };
};
```

**C. Cheap repo-wide booleans — include flat.**
`has-merge-conflict` (and arguably promote a single top-level `is_dirty`). These are genuinely repo-wide, so they stay at the top level.

**D. Excluded — do not contribute to the aggregate (already the contract; keep it).**
- `remote`, `pr` — **network-primary**; the aggregate makes no network requests.
- `hash` — **parameterized** (needs a target); not meaningful with no argument.
- `default` — the bare `repo` command itself (no recursion).

This taxonomy should be encoded as the documented contract for the aggregate (and mirrored in the `sniff` skill's "`sniff repo --json` aggregate" paragraph), so future children are slotted deliberately into A/B/C/D rather than dumped flat.

### Fix 1 — Collapse the change-family envelopes

There are **13** top-level change keys today, each a wrapper object:

- `dirty-files`, `dirty-source-code`, `dirty-packages`, `dirty-package-areas`
- `staged-files`, `staged-source-code`, `staged-packages`, `staged-package-areas`
- `unstaged-files`, `unstaged-source-code`, `unstaged-packages`, `unstaged-package-areas`
- `untracked-files`

Every one carries a redundant `kind` + `scope` envelope, and the value array is inconsistently named — files use `paths`, package/area families use `names`:

```jsonc
// dirty-files
{ "kind": "all_files",     "scope": "dirty",     "paths": ["codebook.toml", ...] }
// dirty-package-areas
{ "kind": "package_areas", "scope": "dirty",     "names": [] }
// untracked-files
{ "kind": "all_files",     "scope": "untracked", "paths": ["sniff/features/..."] }
```

`kind` and `scope` are 100% derivable from the key — pure noise. Replace all 13 with the flat shape the spec proposes, one object per scope, plus a `documentation` projection symmetric with `source_code`:

```ts
type ScopeBucket = {
    files: string[];          // all changed paths in this scope
    source_code: string[];    // subset: source files
    documentation: string[];  // subset: markdown/doc files
    packages: string[];       // package names touched
    package_areas: string[];  // package-area names touched
};

type SniffRepo = {
    // ...
    dirty:     ScopeBucket;   // working tree (staged ∪ unstaged ∪ untracked)
    staged:    ScopeBucket;
    unstaged:  ScopeBucket;
    untracked: ScopeBucket;   // today only `untracked-files` exists; complete the set
};
```

> The original draft proposed `dirty_source_code` / `dirty_documentation` as nested `{files, package_areas, packages}` objects. Folding **all** projections of a scope under one `ScopeBucket` is the more consistent generalization — it gives `staged`/`unstaged`/`untracked` the same treatment for free and removes the `files` vs `paths` vs `names` naming drift. (If a flatter surface is preferred, the alternative is `dirty_files`, `dirty_source_code`, `dirty_packages`, `dirty_package_areas` as sibling arrays — but pick one scheme and apply it to every scope.)

Implementation: replace `file_list_value()` and `package_family_value()` (`repo_json.rs:423`, `:442`) — which build the `{kind,scope,...}` envelopes — with a single `scope_bucket_value(scope)` helper, and drop the 13 individual `map.insert(...)` calls in favor of 4 scope inserts.

### Fix 2 — Rebuild `git-status` (worst offender for overlap)

`git_status_value()` (`repo_json.rs:547`) just dumps the entire `GitInfo` struct via `serde_json::to_value`. Inside the aggregate that struct overlaps with — or fully duplicates — data already present at top level:

| `git-status` field | Problem | Action |
|--------------------|---------|--------|
| `repo_root` | duplicates top-level `root` | **remove** from aggregate |
| `base_repo_root` | absolute path to the *main* checkout; redundant with worktree data | **remove** from aggregate |
| `worktrees` (map of 15 full `WorktreeInfo`) | **full duplicate** of top-level `worktrees`, in a *different* shape (map-keyed-by-name with `ahead/behind/dirty/merged/has_conflicts/...` vs the top-level array's `{name,branch,path,current,detached}`) | **remove** from aggregate; reconcile into one worktrees representation (see Fix 4) |
| `branches` (39 `LocalBranchInfo`) | belongs alongside worktrees, not buried in git-status | **move** to top-level `repo.branches`, peer of `repo.worktrees` |
| `recent` (10 commits) | overlaps top-level `recent-commits` | **remove** from aggregate; keep the dedicated commit family |
| `org`, `repo`, `remotes` | remote identity; fine but should live in one place | keep once (e.g. a `remote` identity block), don't scatter |
| `status` (`{is_dirty, staged_count, unstaged_count, untracked_count, dirty:[], untracked:[]}`) | the `dirty`/`untracked` arrays are **always empty** here even when counts are non-zero (the real paths live in `file_changes` and the Fix-1 buckets) — dead, misleading fields | drop the empty arrays; keep only the counts/booleans, or derive them from the Fix-1 buckets |
| `file_changes` | each entry has **both** `action` and `status` with near-duplicate values (`"Modified"/"Modified"`, `"Created"/"Untracked"`) | collapse to a single field; this is the one genuinely useful per-file change list — keep it, but it now overlaps the Fix-1 buckets, so decide: buckets are name-rollups, `file_changes` is the per-file detail with line counts |
| `config` | fine as-is | no change |
| `current_branch` | fine as-is | no change |

Net: in the aggregate, `git-status` should shrink to roughly `{ current_branch, config, file_changes, status-counts }`. Everything else is either promoted to a top-level peer (`branches`) or already exists at top level (`root`, `worktrees`, `recent-commits`).

> Note: the earlier draft said "branches is empty." That was stale — on this run `git-status.branches` holds **39** branches. The real issue is *placement* (buried in git-status) and *duplication potential*, not emptiness.

### Fix 3 — Strip junk from the commit/period families

`recent-commits`, `source-code-changes`, and `documentation-changes` share the `commit_family_value()` shape and each carry three useless wrapper fields:

```jsonc
{
  "commits": [ /* per-commit objects */ ],
  "filter": "documentation",   // static — equals the key; remove
  "period_label": "last 3d",   // OK to keep, but see below
  "repo_root": "."             // wrong (relative ".") AND redundant with top-level root; remove
}
```

- **`filter`** — constant per key (`"source_code"`, `"documentation"`, absent/`"all"` for recent). Remove.
- **`repo_root: "."`** — a relative `"."` string, both wrong and redundant with top-level `root`. Remove.
- **`period_label`** — only meaningful field of the three; keep it (the aggregate hardcodes "last 3 days" via `default_commit_family_set`). Consider a structured `{ since, until }` instead of a prose label.
- **`recent-commits.packages`** — `recent-commits` additionally embeds the **entire 67-package catalog** (`first_elem_keys` = the full `Package` shape). This is the single biggest offender (≈600 KB of the 771 KB). **Remove it entirely** — the catalog has nothing to do with recent commits and the top-level `packages` already lists names. This is almost certainly an accidental field leak from the focused `recent-commits` command into the aggregate.

**Per-commit `package_areas` / `packages` summarization is inconsistent.** Counts of commits where these are populated:

| family | total commits | with `package_areas` | with `packages` |
|--------|--------------:|---------------------:|----------------:|
| `recent-commits` | 87 | 43 | 43 |
| `source-code-changes` | 39 | 39 | 39 |
| `documentation-changes` | 53 | **14** | **14** |

`documentation-changes` under-counts badly: the most recent commit moved `sniff/features/.../*.md` (clearly inside the `sniff` package area) yet reports `package_areas: []`, `packages: []`. The doc-file→package-area mapping path is not applying the same containment logic the source-code path uses. Fix the mapping so a markdown file under a package area is attributed to it.

Also reconsider **embedding full per-commit `files[]` arrays** in the aggregate at all. For the bare `repo --json` projection, a count + the rolled-up `package_areas`/`packages` is likely enough; full file lists belong to the focused `sniff repo recent-commits --json`. (Three families × dozens of commits × full file lists ≈ 980 KB of the 2.48 MB.)

### Fix 4 — Reconcile the two worktree shapes & drop no-variance fields

There are two worktree representations:

1. **Top-level `worktrees`** — `{ "worktrees": [ {name, branch, path, current, detached} ] }`. Note the redundant **double nesting**: the key is `worktrees` and its value is *also* `{worktrees: [...]}`. Flatten to `worktrees: WorktreeEntry[]`.
2. **`git-status.worktrees`** — a `HashMap<name, {ahead, behind, base_branch, branch, changed_files, dirty, has_conflicts, is_current, merged, sha, filepath}>`. Richer, map-shaped.

Pick **one**. Recommended: a single top-level `worktrees: WorktreeEntry[]` array carrying the union of useful fields, and remove `git-status.worktrees` from the aggregate (Fix 2).

- **`base_branch`** is `"main"` for every worktree (no variance in practice). Either drop it from the per-worktree object and surface a single top-level `default_branch` if it's ever needed, or keep it only where it actually differs. Do **not** repeat a constant 15×.

### Cleanup `sniff repo --json` data structure (consolidated)

```ts
type SniffRepo = {
    // A. repo-wide identity (already lean — keep)
    name: string;
    version: string | null;
    root: string;                 // single source of truth for repo root
    is_monorepo: boolean;
    package_count: number;
    language: string | null;

    packages: string[];           // names only (cheap) — the ONE package list in the aggregate
    package_areas: string[];      // names only
    package_manager: string | string[] | null;
    test_runner: string | string[] | null;

    branches: BranchInfo[];       // promoted out of git-status (Fix 2)
    worktrees: WorktreeEntry[];   // single flattened representation (Fix 4)

    // A. change scopes — flat ScopeBucket per scope (Fix 1)
    dirty: ScopeBucket;
    staged: ScopeBucket;
    unstaged: ScopeBucket;
    untracked: ScopeBucket;

    // A. git-status, slimmed (Fix 2)
    git_status: {
        current_branch: string | null;
        config: GitConfig;
        file_changes: FileChange[];   // single status field per entry, with line counts
        is_dirty: boolean;
        staged_count: number;
        unstaged_count: number;
        untracked_count: number;
    };

    // A. commit families — junk fields stripped, no embedded package catalog (Fix 3)
    recent_commits:        { period: { since: string; until: string }; commits: CommitSummary[] };
    source_code_changes:   { period: { since: string; until: string }; commits: CommitSummary[] };
    documentation_changes: { period: { since: string; until: string }; commits: CommitSummary[] };

    // C. cheap repo-wide booleans
    has_merge_conflict: boolean;

    // B. cwd-relative queries — grouped so they read as "about my location", not the repo
    context: {
        package: string;
        package_area: string;
        area: string;
        package_root: string;
        package_area_root: string;
        worktree: string | null;
        is_current_package_area_dirty: boolean;
        package_area_has_source_code_changes: boolean;
    };

    package_dependencies: PackageDependencySummary; // renamed focused `repo deps` projection
    dependencies: ExternalDependencySummary;        // external dependency projection

    // structure / dependency summaries stay available but must NOT re-embed the
    // full package catalog in the aggregate. `structure` keeps workspace tooling
    // + monorepo flags but references packages by name.
    // (D. `remote`, `pr`, `hash` are excluded — network / parameterized.)
};
```

`CommitSummary` = `{ sha, datetime, description, bullet_points, package_areas, packages, files?: FileChange[] }` — with per-commit `files[]` optional / omitted from the aggregate projection.

### Implementation pointers

All of the above lives in **`sniff/cli/src/output/repo_json.rs`** (the bare-`repo` aggregate builder) plus a couple of `serde` shapes:

- `file_list_value()` / `package_family_value()` → replace with `scope_bucket_value()` (Fix 1).
- `git_status_value()` → add an aggregate-specific projection that omits `repo_root`, `base_repo_root`, `worktrees`, `branches`, `recent` and collapses `file_changes` action/status (Fix 2). Keep the fat `GitInfo` dump only for the focused `sniff repo git-status --json`.
- `commit_family_value()` → drop `filter`/`repo_root`, drop the embedded `packages` catalog, fix doc-file package-area attribution (Fix 3).
- `worktrees_value()` (`repo_json.rs:406`) → flatten the double-`worktrees` nesting and drop constant `base_branch` (Fix 4).
- Promote branches: emit a top-level `branches` from `GitInfo.branches`.

Because the focused subcommands (`sniff repo package-dependencies`, `repo dependencies`, `repo structure`, `repo git-status`, `repo recent-commits`, …) keep their current rich shapes, these changes are scoped to the **aggregate builder** and are low-risk to the per-command contracts. Update the L1 aggregate integration tests and the `sniff` skill's "`sniff repo --json` aggregate" description accordingly.

## Backwards Compatibility & Acceptance Criteria

The remaining breaking changes in this feature — reparenting installed-program commands under `sniff software`, renaming `sniff repo deps` → `sniff repo package-dependencies`, adding `sniff repo dependencies`, the bare `sniff repo --json` redesign (kebab→snake_case keys, collapsed change-family envelopes, dropped embedded package catalog), and the `sniff repo version` fix — are a **coordinated hard break**. There are **no deprecation aliases, no legacy-shape flag, and no schema versioning**; the old surfaces are removed outright. `sniff` is depended on by ~16 in-repo packages, and claudine consumes its `repo --json`, so every call site is updated in the same change.

Acceptance criteria:

- **Call-site audit (in-repo invocations).** `git grep` across the monorepo for `sniff repo deps` invocations (and any scripted `repo --json` consumers) and update every call site to `sniff repo package-dependencies` / the new snake_case JSON shape **in the same change**. No call site is left invoking the renamed/removed surface.
- **Reparented command audit (in-repo invocations).** `git grep` across the monorepo for the 10 reparented commands (`sniff programs`, `sniff editors`, `sniff utilities`, `sniff language-package-managers`, `sniff os-package-managers`, `sniff tts-clients`, `sniff terminal-apps`, `sniff audio-players`, `sniff notification-helpers`, `sniff agents`) and update every invocation to the `sniff software` / `sniff software <x>` form **in the same change**. No call site is left invoking the removed top-level path.
- **claudine consumer migration.** claudine's `sniff repo --json` consumer(s) are updated to the new snake_case / restructured shape in the same change — no consumer is left reading the old kebab-case keys or the removed embedded package catalog.
- **`--json` redesign validation.** The `--json` redesign is validated against the "Diagnosis (measured)" fixtures above: assert the byte-size reduction, the presence of snake_case keys, the absence of the triplicated package catalog, and the flat `ScopeBucket` shape.

> The remote commands (`sniff repo issues`, `sniff repo ci-cd` / `ci`) are **not** part of this break — they are deferred to a future feature (see "New Remote Commands — deferred to a future feature").
