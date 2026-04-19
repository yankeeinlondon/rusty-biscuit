# File Completion Supplement

## Problem Statement

When a user invokes a claudine subcommand that expects a markdown file
reference and triggers shell completion, they currently get generic
directory matches instead of useful markdown-file suggestions.

Concrete example: `claudine compose <tab>` currently returns directory
entries like:

- `@biscuit-terminal`
- `@fixes`
- `cli`
- `docs`
- etc.

None of these are markdown files, and the matches ignore the fact that
claudine's prompt/sequence arguments resolve via biscuit-file's
`FileReference` semantics (repo root + user root, `@`-magic paths,
implicit-relative paths). The completion layer should align with the
resolution layer.

## Goals

- Return only markdown files (`*.md`) as completion candidates at
  markdown-expecting argument positions.
- Start with a small, curated candidate set (prompts/sequences
  directories at both repo and user scope) and expand only once the
  user has typed enough characters to make a broader scan useful.
- Align completion resolution with biscuit-file's `FileReference`
  resolution, rather than inventing a second, divergent grammar.
- Work consistently across `bash`, `zsh`, and `fish` generated
  completion scripts.

## Non-Goals

- Completing non-markdown file arguments.
- Fuzzy matching beyond case-insensitive substring, or scoring-based
  ranking (see Open Questions for ordering).
- Completion inside editor/IDE surfaces (LSP, editor plugins, etc.).
- Retroactively fixing shell completion scripts that users have
  already installed (see Open Questions).

## Triggering Conditions

Dynamic completion fires for the following argument positions:

1. `claudine compose <file>` — positional argument
2. `claudine inline-compose <file>` — positional argument
3. `claudine sequence <file>` — positional argument
4. `--append-system-prompt <file>` / `--asp <file>` and
   `--replace-system-prompt <file>` / `--rsp <file>` flags, on:
   - every wrapped provider subcommand: `claude`, `codex`, `gemini`,
     `goose`, `kimi`, `opencode`, `qwen`
   - and on `compose`, `inline-compose`, `sequence`

At any other argument position, completion behavior is unchanged.

## Resolution Semantics

Resolution of a partial completion token defers to biscuit-file's
`FileReference`. The completion layer must not re-implement the
tokenization, prefix grammar, magic-path expansion, or vault/implicit
lookup.

Two token shapes are considered valid entry forms, matching
`FileReference`:

- **`@`-prefixed magic path** — e.g. `@prompts`, `@prompts/foo`,
  `@~/prompts/foo`. The `@` sigil triggers magic-path resolution
  against *both* the repo root and the user home root (`~/`)
  simultaneously, as elsewhere in claudine and darkmatter.
- **Implicit relative path** — e.g. `prompts`, `prompts/foo`. This
  matches `FileReference`'s "Implicit Relative" form and resolves
  against the repo root (and package root where applicable).

### Prerequisite in biscuit-file

`FileReference` as of this spec exposes parsing and whole-reference
resolution (`new`, `resolve`, `resolve_from`, `resolve_relative`) but
does not expose a "complete partial token" API — i.e. given a
partially typed reference, return the candidate expansions under each
magic/vault root. Adding that API is a **prerequisite task in
biscuit-file**. The completion layer is a consumer of that API, not a
re-implementation of its logic.

## Character Counting

The "typed length" that drives the candidate-set decision below is
measured in **meaningful query characters**, not raw token length.

- The leading `@` sigil is **excluded** from the count.
- The count **resets after each `/` path separator**.

Examples:

| Typed token       | Meaningful chars | Active segment |
|-------------------|------------------|----------------|
| `` (empty)        | 0                | (root)         |
| `@`               | 0                | (root)         |
| `@p`              | 1                | `p`            |
| `@pr`             | 2                | `pr`           |
| `@pro`            | 3                | `pro`          |
| `@prompts/`       | 0                | (after `/`)    |
| `@prompts/a`      | 1                | `a`            |
| `prompts/`        | 0                | (after `/`)    |
| `prompts/abc`     | 3                | `abc`          |

## Matching Semantics

Matching of the active segment (see "Character Counting") against a
candidate is **case-insensitive substring on the filename**. The
trailing `.md` extension is stripped for the purpose of matching, but
remains present on the returned candidate string the shell inserts.

- `@pr<tab>` matches any markdown file whose filename (minus `.md`)
  contains `pr` anywhere — `prompt.md`, `my-prompt.md`, and
  `suppress.md` all match.
- `@omp<tab>` matches `prompt.md` (mid-filename match).
- Directory components in the candidate path are not considered for
  matching; only the final filename segment is.

Matching is distinct from **ordering**. How matched candidates are
ranked or sorted within the returned list remains an open question
(see Open Questions).

## Candidate Sets by Typed-Length

### 0-2 meaningful characters — curated scope only

Only markdown files (`*.md`) under the following directories are
offered. No other filesystem locations are walked.

- Repo scope (see "Enclosing Repo" and "Package Resolution" below):
    - `<repo>/prompts/`
    - `<repo>/sequences/`
    - `<package-root>/prompts/`
    - `<package-root>/sequences/`
    - `<package-area-root>/prompts/`
    - `<package-area-root>/sequences/`
- User scope:
    - `~/prompts/`
    - `~/.claudine/prompts/`
    - `~/sequences/`
    - `~/.claudine/sequences/`

### 3+ meaningful characters — extend to enclosing repo

All curated-scope candidates above, **plus** all markdown files
discovered under the enclosing git repo, subject to the exclusion
rules in "Exclusion Handling".

### Path-separator resets

After a `/`, the meaningful-character count resets. Completion at
`@prompts/<tab>` lists files inside `<repo>/prompts/` *and*
`~/prompts/` (and `~/.claudine/prompts/`, `<package-root>/prompts/`,
`<package-area-root>/prompts/` where applicable) — delegating the
cross-root enumeration to `FileReference`.

Completion at `prompts/<tab>` (implicit-relative) lists files inside
`<repo>/prompts/` only.

### Enclosing Repo

"Repo" means the git repository that encloses the shell's current
working directory, determined by walking upward for a `.git`
directory. (This matches biscuit-file's own repo-root resolution.)

### No-Repo Fallback

When the cwd is not inside any git repository:

- The curated **user-scope** directories still apply.
- The repo-scope directories do not exist and are skipped.
- The "3+ meaningful chars extends to enclosing repo" behavior
  **does not activate**. No broad filesystem scan is performed as a
  substitute.

### Package Resolution

The `sniff` library is the authority for resolving both levels. The
completion layer must not re-implement package or area detection.

- **`<package-root>`** — the enclosing Cargo package directory.
  Resolved via
  `sniff::filesystem::repo::types::RepoInfo::package_for_dir`, which
  returns the `Package` whose `path` is the package root. The
  nearest-enclosing package is selected (the one with the longest
  matching prefix).
- **`<package-area-root>`** — the parent area containing one or more
  package crates. Resolved via
  `sniff::filesystem::repo::types::RepoInfo::package_area_for_dir`,
  which returns the area name; the area-root directory is
  `<repo>/<area>`.

Concrete example: when cwd is
`claudine/cli/src/commands/`, package-root is `claudine/cli/` and
package-area-root is `claudine/`.

Behavior when the two resolutions coincide:

- In a single-crate area (`biscuit-visualized`, `tabby`, `tui`),
  package-root and package-area-root point to the same directory;
  candidate paths resolving under both must be deduplicated by
  canonicalized path.
- When cwd is inside a workspace but outside any known package area,
  both resolutions return nothing; only `<repo>/...` and user-scope
  directories apply.
- When cwd is outside any workspace/repo, both resolve to nothing
  (and the No-Repo Fallback above applies).

If `RepoInfo::package_for_dir` / `RepoInfo::package_area_for_dir`
turn out to need a lighter-weight entry point that does not require
constructing a full `RepoInfo` for a completion call, that is a
**prerequisite task in sniff** (parallel to the biscuit-file
prerequisite).

## Exclusion Handling

For the 3+ character broad scan, reuse the existing `.gitignore`-aware
markdown walker rather than inventing a new exclusion list. The
authoritative implementation is:

- **`sniff/lib/src/filesystem/docs.rs::collect_markdown_files`**

It uses `ignore::WalkBuilder` configured with `.git_ignore(true)`,
`.git_global(true)`, and `.git_exclude(true)`, and filters to `*.md`
files. This gives us, for free:

- `.git/`, `target/`, `node_modules/`, and anything else excluded by
  repo `.gitignore`, global gitignore, or `.git/info/exclude`.
- Correct monorepo behavior (per-package `.gitignore` files respected).

The completion layer should depend on this function (or a thin
wrapper over the same `WalkBuilder` configuration). If a
completion-specific variant is required (e.g. returning relative
paths suitable for shell insertion), it should be a small adapter
around the same walker configuration, not a new exclusion policy.

## Architecture

### Dynamic callback subcommand

Completion logic lives in Rust, not in generated shell script. A
hidden `claudine __complete` subcommand is added. It:

- Accepts the context needed to drive completion (at minimum: the
  argument position being completed and the current partial token).
  Exact CLI shape is an implementation detail but must be stable
  enough for generated scripts to call reliably.
- Performs meaningful-character counting, curated-scope vs broad-scan
  selection, enclosing-repo detection, cwd-based resolution, and
  exclusion.
- Delegates reference parsing and candidate expansion to
  biscuit-file's `FileReference` completion API (see Prerequisite).
- Emits completion candidates on stdout in the format the shell
  expects (one per line, or shell-specific encoding).
- Is hidden from user-facing help output (clap `hide = true` or
  equivalent).

### Generated shell scripts

`claudine completions bash|zsh|fish` continues to generate the static
skeleton but, for the argument positions listed under "Triggering
Conditions", emits shell snippets that shell out to
`claudine __complete ...` and feed the result back to the shell's
completion machinery.

Static clap-generated completion is retained for all other positions.

## Acceptance Criteria

Each of the following scenarios must pass for each of `bash`, `zsh`,
and `fish` (except where noted as shell-agnostic Rust tests).

1. **Empty input, inside a repo.** `claudine compose <tab>` lists
   only markdown files under `<repo>/prompts/`, `<repo>/sequences/`,
   `<package-root>/prompts/`, `<package-root>/sequences/`,
   `<package-area-root>/prompts/`,
   `<package-area-root>/sequences/`, `~/prompts/`,
   `~/.claudine/prompts/`, `~/sequences/`, and
   `~/.claudine/sequences/`. No other directories are walked.

2. **Two meaningful chars with `@` sigil.** `claudine compose @pr<tab>`
   matches any curated-scope markdown file whose filename (with `.md`
   stripped) contains `pr` anywhere — e.g. `prompt.md`,
   `my-prompt.md`, and `suppress.md` all match. Ordering is not
   specified by this criterion (see Open Questions).

3. **Three meaningful chars with `@` sigil.** `claudine compose @pro<tab>`
   extends the search to include all `*.md` files under the enclosing
   git repo, subject to `.gitignore` exclusions per the sniff
   walker.

4. **Path-separator reset with `@` magic path.**
   `claudine compose @prompts/<tab>` lists files inside **both**
   `<repo>/prompts/` and `~/prompts/` (and `~/.claudine/prompts/`,
   `<package-root>/prompts/`, `<package-area-root>/prompts/` where
   applicable). The meaningful-character count is 0 in the segment
   after `/`, so no broad scan activates.

5. **Implicit-relative path.** `claudine compose prompts/<tab>` lists
   files inside `<repo>/prompts/` only (no cross-root expansion,
   matching `FileReference` implicit-relative semantics).

6. **`--asp` on a wrapped provider subcommand.**
   `claudine claude --asp <tab>` (and likewise `--asp`/`--rsp`/
   `--append-system-prompt`/`--replace-system-prompt` on every
   provider subcommand and on `compose`/`inline-compose`/`sequence`)
   produces the same candidate set as criterion (1).

7. **Outside any git repo, 3+ chars.** With cwd set to a directory
   that is not inside any git repo, `claudine compose @pro<tab>`
   returns only curated user-scope candidates filtered by `pro`. No
   broad filesystem scan is performed.

8. **Non-markdown files are never offered.** A `.txt`, `.rs`, or
   binary file in a curated directory must not appear in any of the
   above scenarios.

9. **Gitignored files are excluded from broad scan.** A markdown file
   inside `<repo>/target/` or `<repo>/node_modules/` (or any
   gitignored path) is not offered when the 3+ char broad scan is
   active.

10. **Mid-filename substring match.** `claudine compose @omp<tab>`
    matches `prompt.md` (and any other curated-scope markdown file
    whose filename contains `omp` anywhere with `.md` stripped),
    demonstrating that matching is substring, not prefix.

11. **Multi-crate area dedup.** When cwd is inside a multi-crate area
    (e.g. `claudine/cli/src/`), both `<package-root>/prompts/` and
    `<package-area-root>/prompts/` are walked. If a markdown file is
    reachable at both levels (unlikely but possible, e.g. via
    symlinks or a file directly under an area root that is also the
    package root in a single-crate area), duplicates are
    deduplicated by canonicalized resolved path so each file appears
    at most once in the candidate list.

### Validation approach

- Rust unit tests against the Rust-side completion function (inputs:
  partial token, cwd, HOME; outputs: candidate list). Shell-agnostic.
- Integration tests that invoke `claudine __complete` as a subprocess
  and assert on stdout.
- Manual shell-level smoke tests for bash, zsh, fish.

## Open Questions / Future Work

The following are explicitly **not decided** by this spec. They should
be raised during implementation but must not be silently resolved.

- **Ranking and ordering.** Within a candidate set, how should results
  be ordered? Alphabetical? Curated-scope first, then broad? Recently
  used? This spec does not commit to an ordering.
- **Caching.** The 3+ character broad scan walks the enclosing repo.
  For large repos this may be slow enough to be noticeable on every
  keypress. Whether to cache walker results (and with what
  invalidation strategy) is open.
- **Performance budget.** No explicit latency budget is set. A
  reasonable target is sub-100ms for the curated-scope case and
  sub-500ms for the broad-scan case on a medium monorepo, but this
  is not a decision.
- **Backwards compatibility of installed completion scripts.** Users
  who already installed an older `claudine completions ...` output
  will not get the new behavior until they reinstall. Whether to
  version the scripts, warn on stale scripts, or auto-reinstall is
  open.
- **`FileReference` completion API shape.** The prerequisite
  biscuit-file API for "complete partial token" is not yet designed.
  Its input/output contract, error behavior, and relationship to the
  existing `resolve*` methods are open questions for the biscuit-file
  feature that blocks this one.
- **Sniff package-resolution API shape for completion.** Both
  `RepoInfo::package_for_dir` and `RepoInfo::package_area_for_dir`
  currently require a constructed `RepoInfo`. Whether the completion
  layer should build one on every call, whether sniff should expose
  a lighter `cwd -> (package-root, package-area-root)` helper, and
  how caching interacts with invalidation are open questions for the
  sniff prerequisite.
- **Behavior when `HOME` is unset or user-scope directories do not
  exist.** Presumably "skip silently", but not explicitly decided.
- **Symlinked directories** inside curated scopes — follow or not?
  The sniff walker's current configuration governs the broad-scan
  case; the curated-scope case is not explicitly specified.
- **Observability.** Whether `claudine __complete` emits tracing/log
  output (and to where, given it runs under a shell completion
  pipeline) is not decided.
