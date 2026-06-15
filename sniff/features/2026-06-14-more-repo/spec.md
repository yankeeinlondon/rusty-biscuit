# More Repo Feature

In this feature we will add more sub-commands to the `sniff repo` CLI as well as extend the sniff library where needed.

- remember that the CLI should own reporting but the library should always own business logic and functionality
- use the 'cli', 'rust-devops', and 'sniff' agent skills

## Sniff Software

The `sniff software` subcommand is the logical parent to:

- `sniff editors`
- `sniff utilities`
- `sniff tts-clients`
- `sniff terminal-apps`
- `sniff audio-players`
- `sniff notification-helpers`
- `sniff agents`

These SHOULD be subcommands of `sniff software`:

- `sniff software editors`
- `sniff software utilities`
- etc.

### Adding Testing Runners

In addition to restructuring we need to add another type of software: "Test Runners":

- `sniff software test-runners`
- is meant to primarily mean unit (and possible) integration test runner but not benchmarking or load testing, etc.
- includes:
    - rust:
        - cargo test
        - nextest
    - JS/TS
        - vitest
        - Jest
        - Mocha
        - AVA
        - Node Test Runner (built into node)
    - Python:
        - pytest
        - unittest
        - nose2
        - tox
        - nox
    - PHP
        - PHPUnit
        - Pest
        - Codecaption
        - Behat
        - atoum
    - ...
- need to be able to look at key files in repo to determine both what the repo uses as well as whether the host has this runner installed

## New Repo Local Commands

### `sniff repo branches`

- lists out the local branches found in the git repo

### `sniff repo package-manager`

provides the name of the package manager of the CWD:

- if this is a non-monorepo or a package in a monorepo then this is always just a singular value
- if this is a monorepo:
    - if we're in a package-area then we should evaluate the package managers used across the contained packages
        - if they are all the same then just report the singular package manager
        - if there is variance across the packages then just report a unique list of package managers (csv,list,md-list)
    - if we're at the root of the monorepo then we should again evaluate across all packages
        - if all use the same package manager then just list it in the singular
        - 


### `sniff repo test-runner`

Determines what "test-runner" is being used in repo/package.

- if this is a non-monorepo or a package in a monorepo then this is always just a singular value
- if this is a monorepo:
    - if we're in a package-area then we should evaluate the test runners used across the contained packages
        - if they are all the same then just report the singular package manager
        - if there is variance across the packages then just report a unique list of test runners (csv,list,md-list)
    - if we're at the root of the monorepo then we should again evaluate across all packages
        - if all use the same package manager then just list it in the singular


### `sniff repo dependencies`

> We need to rename the existing `sniff repo deps` to avoid semantic conflict; we will rename to `sniff repo packages` (as this is purely calling out monorepo package dependencies)

- we already return this kind of information but the structure of this information needs to be improved at the `sniff repo` level and with that re-structuring the reporting on "external-dependencies" will be easier
- there are the following sub-types of external dependencies:
    - `dev-dependencies`
    - `peer-dependencies`
    - `optional-dependencies`
    - `dependencies`
- each of these sub-types will be filterable with a CLI switch (e.g., `--dev-dependencies` shows only development dependencies, etc.)



## New Remote Commands

> Note: these commands need to work on all Cloud Git providers that we support (Github, Gitlab, Gitea, Bitbucket, etc.)

### `sniff repo ci-cd` (alias `sniff repo ci`)

- reports on the CI/CD pipeline status
- today when we run `sniff repo remote` we get information on CI/CD but it is not good enough
    - the improvements we make here should be back-ported to the reporting we do for 
- we will add the following subcommands:
    - `id <id>` - rather than list jobs, this will provide detail on a particular job
    - `last` - provides details on the last CI/CD action
    - `list <#>` - (this is the default command for `sniff repo remote`)
        - lists the last 5 (overridden by optional numeric parameter) CI/CD jobs, their id, status, and other key metadata

### `sniff repo issues`


### `sniff repo`

## Fix

### Fix `sniff repo is-monorepo` -> false | kind

- right now `sniff repo is-monorepo` seems to always return "yes"
- it should NEVER return "yes"!

The correct behavior is:

- `sniff repo is-monorepo` returns `false` (and an error exit code) when it is NOT a monorepo
    - we need to include a `--no-error` switch which will return `false` but NOT an error exit code
- when it is run inside a monorepo -- instead of reporting `true` -- it returns the monorepo standard that defines it (cargo, pnpm, etc.)

This approach solves the following problems:

1. you can do shell true/false branching with the exit code
2. the value reported on STDOUT can be considered a valid branching value too if you use "truthiness" as the test (which many languages will do by default)
3. if it is a monorepo you not only know that it IS but what technology standard the monorepo is based on

The `--json` output should return:

```ts
type Json = {
    is_monorepo: false;
} | {
    is_monorepo: true;
    kind: "cargo" | "pnpm" | ...
    /** 
     * whether the user has the required binary to operate with 
     * the monorepo tech 
     */
    installed: boolean;

    /**
     * the binary name used to manage this type of monorepo
     */
    binary: string;
}
```

### Fix `sniff repo version`

This needs to work on all programming languages but it's highly inconsistent. 

- Seems to work in a Typescript project
- Doesn't work in Rust (including this monorepo)

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

The aggregate is assembled key-by-key in `sniff/cli/src/output/repo_json.rs` (the bare-`repo` builder around lines 640–840). Each helper is individually fine for its *focused* subcommand (`sniff repo deps --json`, `sniff repo git-status --json`, etc.); the problem is that the **aggregate re-emits the heavy whole-scope objects** that those focused commands return, instead of contributing a lean projection. The fix is mostly in the aggregate builder and a few `serde` shapes — not a rewrite of the library.

### Design principles

The bare `sniff repo` aggregate is meant to be a **complete information rollup of its informational children** — that is the right instinct, and we keep it. The redesign does **not** drop information; it removes *duplication and noise* so that the same completeness fits in a fraction of the bytes. Concretely:

1. **Complete, but each fact once.** Every informational child still contributes its facts, but a package, a worktree, a branch, or a commit's file list is serialized exactly once. Everything else references it by name/sha.
2. **No property without variance.** Drop any field whose value is fully determined by its key (e.g. `scope: "dirty"` under the `dirty-*` key) or is constant across the run (e.g. `base_branch: "main"` repeated 15×).
3. **Aggregate ≠ verbatim concatenation of focused commands.** A focused child command (`repo deps`, `repo git-status`, `repo recent-commits`) may return a rich/heavy shape; the aggregate contributes a **lean projection** of that same data, not a re-emission of the whole scope object.
4. **`snake_case` keys, consistently.** Today the aggregate mixes `kebab-case` top-level keys (`dirty-source-code`, `git-status`) with `snake_case` inner keys. Standardize on `snake_case`.
5. **Empty means empty array** — never an error string, never a `{kind,scope,paths:[]}` envelope.

### Child subcommand taxonomy — what the aggregate contains

`sniff repo` has ~40 children. "Contain everything from the children" is the goal *for the informational ones* — but several children are queries, network calls, or parameterized lookups that legitimately do **not** belong in a no-arg, no-network aggregate. Classify them explicitly:

**A. Repo-wide facts — include (the core of the aggregate).**
`name`, `version`, `is-monorepo`, `package-count`, `language`, `root`, `packages`, `package-areas`, `structure`, `deps`, `worktrees`, `branches` (new), `git-status`, `recent-commits`, `source-code-changes`, `documentation-changes`, and the change-scope families. These describe the repository itself and are identical regardless of where in the tree you invoke them.

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

### Target top-level shape (consolidated)

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

    // structure / deps stay available but must NOT re-embed the full package catalog
    // in the aggregate — `deps` keeps the dependency projection; `structure` keeps
    // workspace tooling + monorepo flags but references packages by name.
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

Because the focused subcommands (`sniff repo deps`, `repo structure`, `repo git-status`, `repo recent-commits`, …) keep their current rich shapes, these changes are scoped to the **aggregate builder** and are low-risk to the per-command contracts. Update the L1 aggregate integration tests and the `sniff` skill's "`sniff repo --json` aggregate" description accordingly.
