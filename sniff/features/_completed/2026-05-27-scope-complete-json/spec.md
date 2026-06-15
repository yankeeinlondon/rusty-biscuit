---
status: ready for planning and implementation
reviewed: true
supersedes: original 2026-05-27 draft (changes #2, #4, #5 retired — see "What changed since the original draft")
---

# Specification: Scope-Complete JSON for `sniff repo`

The `sniff` CLI's `--json` output is governed by the principle documented at
[`sniff/docs/topics/json-output.md`](../../docs/topics/json-output.md):
**scope-complete JSON**. At any command node, `--json` returns exactly that
node's scope — no more, no less. A parent node's JSON is the aggregate of its
children's scopes, keyed by each child's subcommand name; a leaf returns only
its own data; default subcommands affect terminal output only.

This feature finishes bringing the `repo` command tree into compliance with that
principle. The remaining gap is concentrated in two places:

1. `sniff repo --json` does **not** aggregate — it emits a single leaf's data.
2. Three identity attributes that `repo name`'s verbose terminal form still
   prints have no leaf of their own and no JSON scope to draw from.

## What changed since the original draft

The original 2026-05-27 draft proposed five changes. Three have since been
resolved or reversed by shipped work and are **removed** from this spec:

- **`repo name --json` → leaf-only (was #2): DONE.** The `2026-06-12`
  `git-identity-request` feature routed `RepoAction::Name` through
  `name_outcome()` (`cli/src/commands/mod.rs:681`). `sniff repo name --json` now
  returns exactly `{ "name": "rusty-biscuit" }`.
- **Commit-family focused JSON (was #4): DONE.** `recent-commits`,
  `source-code-changes`, and `documentation-changes` are now served by
  `recent_commits::handle_recent_commits_command`
  (`cli/src/commands/mod.rs:818`) and emit focused shapes
  (`commits`, `period_label`, `repo_root`, plus `packages`/`filter`) — no longer
  the whole `RepoInfo` blob. The audit premise that named them non-compliant is
  stale.
- **Add `--with-network` global flag (was #5): REVERSED.** The
  `2026-06-11-with-network` feature (completed) removed `--with-network` as a
  misleading, unread contract. The principle doc's "Golden Exception"
  (`docs/topics/json-output.md:29-32`) was rewritten to gate supplemental data on
  **command-local opt-ins** (`--refresh-remotes`, `--latest-versions`) instead.
  This spec must not reintroduce the flag, and the aggregate design below uses
  the command-local model.

One regression surfaced *because of* the `git-identity-request` work and is now
folded into this spec — see change #3.

## Current state (verified against the working tree)

- `Commands::to_repo_action()` maps bare `repo` to `RepoAction::Name`
  (`cli/src/args/mod.rs:686`). Both `sniff repo` and `sniff repo name` therefore
  resolve to the same action and the same handler.
- `RepoAction::Name` (`cli/src/commands/mod.rs:681`) calls
  `detect_repo_identity` and, under `--json`, routes through `name_outcome()`.
  Result: **`sniff repo --json` and `sniff repo name --json` both emit
  `{ "name": "..." }`.** The parent is behaving like the leaf — it does not
  aggregate.
- `build_with_outcome`'s `None` arm still serializes the full `RepoInfo` blob via
  `fallback_repo_value` (`cli/src/output/repo_json.rs:92`), but that arm is no
  longer reached for bare `repo` (the `Name` early-return in `commands.rs`
  intercepts first). It remains the fall-through for actions without a focused
  builder.
- `detect_repo_identity` (`lib/src/filesystem/repo/identity.rs:72`) already
  computes everything the new leaves and the aggregate need —
  `name`, `version`, `language`, `is_monorepo`, `package_count` — from a single
  cheap scan. No new detection work is required.
- `render_repo_name` (`cli/src/output/filesystem/repo.rs:22`) at `-v` prints the
  name **plus** `version` and either `[<n> package monorepo]` or `[<language>]`.
  Since `repo name`'s JSON scope is now `{ name }`, that verbose form leaks
  four fields (`version`, `is_monorepo`, `package_count`, `language`) outside the
  leaf's JSON scope. The existing terminal-output topic only makes this a strict
  rule for non-verbose output and permits verbose output to reach related peer
  data as an exception. This spec intentionally chooses a stricter command-family
  rule for `repo name`: single-value leaves stay single-value in both default and
  verbose terminal modes, and richer identity output moves to the `repo` parent.

## Concrete changes

### 1. Make `sniff repo --json` aggregate (HIGH)

**Target:** `sniff repo --json` returns the aggregate of its participating
children, keyed by the subcommand name a user would type:

```json
{
  "name": "rusty-biscuit",
  "version": null,
  "language": null,
  "is-monorepo": true,
  "package-count": 65,
  "structure": { ... },
  "packages": [ ... ],
  "package-areas": [ ... ],
  "deps": { ... },
  "worktrees": [ ... ],
  "git-status": { ... },
  "staged-files": { "scope": "staged", "kind": "all-files", "paths": [] },
  "unstaged-files": { "scope": "unstaged", "kind": "all-files", "paths": [] },
  "untracked-files": { "scope": "untracked", "kind": "all-files", "paths": [] },
  "dirty-source-code": { "scope": "dirty", "kind": "source-code", "paths": [] },
  "staged-source-code": { "scope": "staged", "kind": "source-code", "paths": [] },
  "unstaged-source-code": { "scope": "unstaged", "kind": "source-code", "paths": [] },
  "dirty-files": { "scope": "dirty", "kind": "all-files", "paths": [] },
  "package": "",
  "package-area": "",
  "area": "root",
  "dirty-packages": { ... },
  "dirty-package-areas": { ... },
  "staged-packages": { ... },
  "staged-package-areas": { ... },
  "unstaged-packages": { ... },
  "unstaged-package-areas": { ... },
  "package-root": "",
  "package-area-root": "",
  "root": "/Users/ken/src/rusty-biscuit",
  "is-current-package-area-dirty": false,
  "package-area-has-source-code-changes": false,
  "has-merge-conflict": false,
  "worktree": null,
  "recent-commits": { ... },
  "source-code-changes": { ... },
  "documentation-changes": { ... }
}
```

**Keying rule.** Keys are kebab-case strings matching the subcommands users type
(`is-monorepo`, not `is_monorepo`) so the JSON keys round-trip back to drillable
subcommands. Each child contributes its own scope value:

- A single-key leaf (`repo name` → `{ "name": V }`) contributes its **unwrapped
  value** under that key (`"name": V`). This matches the principle-doc example
  (`docs/topics/json-output.md:43-50`).
- A multi-field child (`repo structure`, `repo deps`, a commit family)
  contributes its **whole scope object** under its subcommand key.

**Which children participate (decision — network policy).** Because `sniff` is a
host-detection tool and `--with-network` no longer exists, the bare aggregate
must be offline and deterministic. The aggregate includes **direct children that
have a meaningful no-argument, local, default-form JSON scope**. That means the
aggregate uses each child with its default flags only; it does not try every
formatting variant or every filterable/scoped variant.

- **Included identity leaves:** `name`, `version`, `language`, `is-monorepo`,
  `package-count`.
- **Included structure/dependency leaves:** `structure`, `packages`,
  `package-areas`, `deps`.
- **Included git and file leaves:** `git-status` (local form only),
  `staged-files`, `unstaged-files`, `untracked-files`, `dirty-source-code`,
  `staged-source-code`, `unstaged-source-code`, `dirty-files`.
- **Included package-context leaves:** `package`, `package-area`, `area`,
  `package-root`, `package-area-root`, `root`.
- **Included change-family leaves:** `dirty-packages`, `dirty-package-areas`,
  `staged-packages`, `staged-package-areas`, `unstaged-packages`,
  `unstaged-package-areas`.
- **Included boolean leaves:** `is-current-package-area-dirty`,
  `package-area-has-source-code-changes`, `has-merge-conflict`.
- **Included history/worktree leaves:** `recent-commits`,
  `source-code-changes`, `documentation-changes`, `worktree`, `worktrees`.
- **Excluded parameterized leaves:** `hash <SHA>` is excluded because it has no
  parent-level default value; the child scope only exists once the caller
  supplies a commit identifier.
- **Excluded network-primary leaves:** `remote` and `pr` are excluded because
  their entire purpose is a remote API call. Folding them in would force a
  network round-trip on every `sniff repo --json`.
- **Supplemental fields** gated by `--refresh-remotes` / `--latest-versions` are
  **never** present in the aggregate, because those opt-ins are command-local to
  the child subcommands and cannot be expressed at the `repo` parent. This is the
  Golden Exception applied consistently.

**Reader note — why this is broader than the first draft:** the scope-complete
rule says a parent aggregates its children, not just a hand-picked summary
subset. The original included-list omitted many direct local `repo` children
(`staged-files`, `package-root`, boolean checks, and related command families).
That would create a second, undocumented "summary aggregate" rule. This spec
instead keeps the existing rule and documents only the two acceptable exclusions:
mandatory-argument children and network-primary children.

**Aggregate error policy.** The aggregate must not silently omit participating
keys. A child-level "no value" state that already has a stable JSON shape remains
in the aggregate as that stable value (`null`, `""`, `[]`, `{ ... }`, or `false`,
matching the child contract). If required local detection fails in a way that
prevents a scope-complete aggregate, the parent command fails rather than
emitting a partial object. In `--json` mode, stdout must contain either the valid
aggregate JSON or nothing; diagnostics and error messages go to stderr.

Some existing script-friendly leaves currently treat "no results" as an early
exit before JSON is emitted. The aggregate path must not invoke those exits.
Instead, factor the JSON value construction into reusable helpers that can
represent empty results (`paths: []`, `names: []`, `worktree: null`, `name: ""`)
without exiting the parent process. Preserve each leaf command's existing exit
code behavior when invoked directly.

**Mechanism.** Bare `repo` and explicit `repo name` currently collapse to the
same `RepoAction::Name`. The implementation must distinguish them — e.g. check
`repo_subcommand.is_none()` at the dispatch site in `commands/mod.rs`, or add a
distinct action variant — and route the no-subcommand `--json` case to a new
aggregate builder that invokes each participating child's existing scope builder
and assembles the keyed object. Reuse the existing focused builders
(`structure_value`, `build_deps_value`, `package_family_value`, the
`recent_commits` handler, `git_status_value`, file-list JSON builders, locator
builders, boolean builders, etc.); do not hand-roll new serialization.

The aggregate builder should live in the CLI output layer, but the values it
aggregates must still come from library detection results or existing CLI
formatters. Do not move detection/business logic into the CLI while adding the
parent aggregate.

### 2. Add three new repo leaves (HIGH)

Add the leaves that give the aggregate somewhere to draw identity attributes from
and restore direct user access to values currently buried in `repo name -v`:

- `sniff repo is-monorepo` → text: `yes` / `no`; JSON: `{ "is-monorepo": true }`
- `sniff repo package-count` → text: the count as plain text (blank/`0` policy per
  existing leaf conventions); JSON: `{ "package-count": 65 }`
- `sniff repo version` → text: the version if present, blank otherwise; JSON:
  `{ "version": "0.1.0" }` or `{ "version": null }`

All three read directly from `detect_repo_identity`'s `RepoIdentity`
(`is_monorepo`, `package_count`, `version`) — no new detection. `sniff repo
language` already exists and is compliant; no change.

Text form follows existing single-value repo-leaf conventions (`sniff repo
package`, `sniff repo name`): plain text suitable for scripting, with the
standard `--no-error` / `--on-error` opt-ins where they make sense. Each leaf's
JSON is a single-key object whose key matches the subcommand name (kebab-case),
and exit code mirrors the text path (e.g. `version` exits `1` on `null`) so
scripts can branch on `$?` without parsing the body — matching `repo language`.

`is-monorepo` and `package-count` are always known when repo identity detection
succeeds, so they do not need `--no-error` / `--on-error`. `version` may be
absent; it should support the same no-result handling style as other
script-friendly optional leaves if that can be done without widening global CLI
semantics.

### 3. Fix the `repo name -v` terminal-subset leak (HIGH)

`render_repo_name` at `-v` prints `version`, `is_monorepo`/`package_count`, and
`language` — none of which are in `repo name`'s `{ name }` JSON scope. Although
the repo-wide terminal-output topic permits verbose peer data as an exception,
this command family is easier to reason about if identity leaves remain
single-value. Resolve by **relocating the rich one-liner from the `name` leaf to
the `repo` parent's default-dispatch terminal form**, where it is drawn from the
aggregate scope:

- **`sniff repo name` / `sniff repo name -v` (leaf):** show only the name.
  Verbose may add contextual styling but **no foreign data fields**. JSON scope
  `{ name }` and terminal output are then in lockstep.
- **`sniff repo` / `sniff repo -v` (parent, default dispatch to `name`):**
  - v0: the bare name (a subset of the aggregate — compliant, unchanged).
  - `-v`: the existing rich one-liner (`**name** vX [<n> package monorepo]` /
    `[<language>]`). Every field it shows (`name`, `version`, `package_count`,
    `is_monorepo`, `language`) is in the parent aggregate scope, so this is
    compliant and **preserves today's UX** rather than regressing it.

This requires the same bare-vs-explicit distinction introduced in change #1.
A richer *multi-line* aggregate terminal form (e.g. listing packages/areas) is
**out of scope** — the one-liner is the minimal compliant parent verbose form.

Terminal rendering should continue to use `biscuit-terminal` renderable
components such as `Prose`; this change is about scope and dispatch, not a move
back to raw ANSI strings.

## Out of scope

- Restructuring `RepoIdentity` or any detection logic. The new leaves and the
  aggregate consume existing fields as-is.
- Changing the JSON or terminal shape of any already-compliant command
  (`structure`, `packages`, `package-areas`, `deps`, `worktrees`, `git-status`,
  file-list leaves, package/context leaves, boolean leaves, the commit families,
  `files`, `docs`, `blast-radius`, `just`, `filesystem`).
- Refining the commit-family JSON shapes (`sha`/`subject`/`files-changed`
  proposal from the original draft). They are compliant; cosmetic shape changes
  are a separate concern.
- Reintroducing `--with-network` or any global network flag (deliberately
  removed; see "What changed since the original draft").
- Adding `remote`/`pr` to the aggregate, or any network-touching child or
  supplemental field. The aggregate is offline by contract.
- Adding `hash <SHA>` to the aggregate. It remains a parameterized leaf with no
  parent-level default.
- A multi-line verbose terminal form for `sniff repo`.
- Renaming any existing command or flag.

## Test considerations

- **Aggregate keys round-trip:** `sniff repo --json` returns an object whose
  top-level keys are exactly the participating children; a test walks each key
  and asserts `sniff repo <key>` is an invokable subcommand. Assert `remote` and
  `pr` keys are **absent**. Assert `hash` is absent because it requires a
  positional SHA.
- **Aggregate is offline:** `sniff repo --json` must not perform a network call
  (no `--refresh-remotes`/`--latest-versions`-only fields present); assert the
  `structure`/`git-status` sub-objects contain only their default local fields.
- **Aggregate is not partial:** if a participating child has no current value
  (for example `worktree` in a main worktree, `version` with no detected
  version, or `package` outside a package), the key is still present with the
  child contract's stable empty value. If local detection fails, assert stdout is
  not a partial JSON object and diagnostics are on stderr.
- **New leaves:** `sniff repo is-monorepo --json`, `sniff repo package-count
  --json`, `sniff repo version --json` each return a single-key object whose key
  matches the subcommand name; assert the key set, not just the value. Cover the
  `version: null` exit-code-`1` path.
- **`repo name` leaf-only holds:** `sniff repo name --json` still returns exactly
  `{ "name": "<repo-name>" }` (regression guard for the shipped behavior).
- **Terminal-subset:** assert `sniff repo name -v` prints only the name (no
  version/language/monorepo data), and that `sniff repo -v` still prints the rich
  one-liner.
- **Parse coverage:** add parser tests for the three new leaves and update the
  existing `to_repo_action_none_is_name` expectation if the implementation adds a
  distinct parent/default action.

## Documentation

The authoritative instruction is the repo's `CLAUDE.md` **Drift Maintenance**
rule (update READMEs on public-behavior change; `.claude/skills/` on
architecture/workflow change). The targets below are the concrete application of
that rule to this work and must be updated **in the same change** as the code:

- **`sniff/docs/cli/repo.md`** — the dedicated repo CLI doc and the most directly
  affected. Three edits:
  - `## Subcommands` — add `is-monorepo`, `package-count`, `version` (the new
    leaves from change #2).
  - `## JSON Output` — document the `repo --json` aggregate (change #1),
    including the keying rule and the offline/network-exclusion policy.
  - **Fix pre-existing drift:** line 3 claims `sniff repo` defaults to `sniff
    repo structure`. The code defaults to `RepoAction::Name`
    (`args/mod.rs:686`), and `docs/topics/json-output.md:52` already says `name`.
    Correct `repo.md` to `name`. This work rewrites exactly that default-dispatch
    behavior (changes #1/#3), so the correction belongs here, not a separate
    pass. (Code is authoritative per the `CLAUDE.md` drift convention.)
- `.claude/skills/sniff/SKILL.md` — document the aggregate, the three new leaves,
  and the offline-aggregate network policy.
- `sniff/cli/README.md` — output-modes section, if it enumerates repo
  subcommands.
- The repo CLI's `REPO_AFTER_HELP` (`args/mod.rs:1105`) — add the new leaves and a
  JSON-modes note.
- `docs/topics/json-output.md` already states the aggregate rule; verify its
  example matches the implemented key set after the work lands.
- `docs/topics/terminal-output.md` — update only if the team decides to make the
  stricter verbose-subset behavior a repo-wide standard. This spec applies that
  stricter rule only to `repo name`; it should not silently rewrite the broader
  terminal policy.
- `sniff/docs/cli/repo_structure.md` — touch only if the default-subcommand
  correction above changes any cross-reference pointing at it as the default.
