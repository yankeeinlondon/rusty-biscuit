# Rusty Biscuit Monorepo

## Language 

- always prefer **US English (en-US)** over other English variants such as UK English (en-GB) when creating symbol names or writing documentation

## Package Area Conventions

- you can run `sniff repo package-areas` for a list of all the _package areas_ in this monorepo
    - most of the package areas consist of a library and CLI pairing though there is some variance to that pattern
- you can run `sniff repo packages` to get the full list of packages in this monorepo

## Operating Systems

- every package must compile and work on macOS, Linux, native Windows, and WSL2
- the `os` agent skill (`.claude/skills/os/`) records the non-obvious, repo-specific facts: which hosts can produce evidence for each OS and how to reach them, how the WSL2 CI leg runs nextest archives and how to reproduce its failures, and the Windows and macOS traps that have already cost time
    - load it before claiming an OS cannot be tested from the current host, before touching `#[cfg(windows)]` or path-comparison code, and whenever a test is red on exactly one CI environment
    - when you learn a new OS-specific fact the hard way, add it to that skill in the same change

## CI Structure

- **Area groups, package identifies.** CI fans out one top-level entry per
  selected package *area*, but every stored name — artifact, JUnit manifest
  record, baseline entry, receipt cell — stays keyed on
  `{package, environment, tier}`. Area is derived from the manifest directory,
  never re-keyed onto anything, and never folded into a parent for nested areas
  like `claudine/rendezvous`. Do not introduce an area-keyed store.
- `scripts/ci/affected_scope.py` emits **one** canonical resolved plan with a
  `{package, environment, gate}` cell per unit of work. Every consumer reads
  that document; nothing recalculates scope.
- **Compile coverage is per target kind.** L1 covers `lib`, `bin`, and `test`; a
  `check` cell exists only where `example` or `bench` targets are declared. An
  unchanged reverse dependency is reported by name and scheduled nowhere.
- **Each area owns its outcome.** An area's rollup applies its own baseline,
  governed gaps, and missing-cell rule and nobody else's. A governed, unexpired
  capability gap is a distinct `ACCEPTED GAP` state decided by the planner — it
  is never inferred from a GitHub conclusion, never baselined, and a real
  failure outranks it.
- **The merge gate is still `ci-verdict`,** transitionally, and removing it is
  not separable from moving the required context. Load the `rust-devops` skill
  before changing CI scope, evidence reuse, or the gate.

## Test Execution Constraints

- A user's instruction not to run or rerun a test environment applies to both
  direct commands and jobs triggered indirectly by a push, dispatch, or retry.
  Do not narrow it to manual reruns unless the user explicitly does so.
- **Record the restriction, do not remember it.** The constraint store named by
  `BISCUIT_CI_CONSTRAINTS_DIR` holds `{environment, gate?, reason, owner,
  expiry, repository?, branch?}` records that `just ci-local --plan` and the
  pre-push hook refuse on. CI deliberately never reads them, so a constraint can
  only stop a push and can never make CI silently skip required coverage.
- Before pushing, review `just ci-local --plan` — the resolved cells with their
  execution, origin, state, and evidence — against every active constraint.
  Verifying one exclusion does not establish that the others are satisfied.
  That preview reads the working tree; the hook itself reviews the outgoing
  revision's COMMITTED plan (its own planner, manifests, and policy, in a
  temporary worktree when the checkout is dirty), so a committed change an
  unstaged edit masks is still reviewed and blocked.
- Local evidence combines **per cell across environments**: every
  `refs/notes/ci-local/<environment>` ref reachable from the outgoing head is
  read, so this host's receipt and a prior WSL `cross-check` receipt suppress
  their own cells together. `lint` and `check` are always CI-origin. Scope
  receipts on `refs/notes/ci-local/scope` are authoritative on an exact
  `{base, head, tree}`; CI runs the planner only on a miss.
- If current CI cannot honor a constraint or accept the prior evidence, explain
  that specific gap before triggering the workflow. Do not silently rerun tests,
  invent a receipt, or describe an unimplemented reuse mechanism as available.
- Classify a failed job by its failing step. Passing tests followed by failed
  artifact upload are not failed tests and do not alone justify rerunning them.

## Just Runner

- we use the `just` runner extensively throughout this monorepo. 
- you will find a justfile at the root of this monorepo and a justfile in each of the _package areas_
- shared recipes for just can be found in the @just/ directory

## Git Identity and Signing

- all commits must use the author `Ken Snyder <ken@ken.net>` and must be
  OpenPGP-signed
- the current host is expected to have the correct signing keys available; a
  signing failure is an environment or configuration problem and must not be
  bypassed with `--no-gpg-sign`
- commit messages must not include agent attribution, co-authorship, or
  co-signing trailers such as `Co-authored-by`, `Generated-by`, or similar
  agent-identifying metadata
- repository-local Git configuration should set `user.name`, `user.email`,
  `user.signingkey`, and `commit.gpgsign`; verify these values before committing
- verify every new commit with `git verify-commit HEAD` before reporting success

## Code Comment Quality

1. **HOW-narration** — prose that restates the implementation step-by-step.
2. **Tautological examples** — assertions guaranteed by the function signature.
3. **`## Arguments` / `## Returns` blocks that duplicate field docs** — use only when adding a constraint not expressed by the type.
4. **Format-string, color, or glyph narration** — quoting literal `format!` strings, ANSI colors, or emoji codepoints in prose. They drift.
5. **Redundant accessor docs** — one-line `///` on `fn foo() -> bool { self.foo }`. Prefer `#[allow(missing_docs)]` at impl level.
6. **Section-marker `//` comments** — `// Protocol` immediately above `extra.insert("protocol", …)`.
7. **Heavy-setup doc examples** — >20-line fixtures for one assertion. Link to a real test or omit.
8. **Stale comments past their code** — comments that no longer match what the code does. Worse than no comment.

Positive criteria — comments worth their length:

- **A. Contract or invariant** not derivable from types (e.g. `atomic_write`'s `last-rename-wins` semantics).
- **B. WHY a counter-intuitive choice was made** — at the surprising line, not in the surrounding docblock.
- **C. Semantics of complex return shapes** (e.g. `extract_frontmatter_text`'s `base_line`).
- **D. Hidden coupling or external surprise** (e.g. "serialize-compatible with X", "persisted to disk").
- **E. Link to authoritative design** at module (`//!`) / module-defining-type (`///`) level only. Per-function linking is a treadmill.

**Authoring discipline.** Any edit that changes a symbol's behavior must include a pass over its `///`/`//!` docs and inline `//` comments. Fix or delete drifted ones in the same change. Reviewers should flag behavior-changing PRs that do not touch the relevant comments.

**Scope discipline.** Comment-only cleanup commits must contain no behavior changes. If `git diff` of the commit shows non-comment line changes (rendering, format strings, constants, glyphs), split the behavior change into a separate commit before requesting review.

- When in doubt, ask: *would deleting this comment lose information a future reader needs?* If no, delete.
- when drift between comments and code is detected, always assume the code is correct and the comment is wrong (unless instructed otherwise); take appropriate actions and communicate that this drift was detected and how it was resolved

## Drift Maintenance

Update alongside code changes:

- READMEs when public behavior changes
- `docs/dependencies.md` (and per-area `docs/dependencies.md`) when crates are added/removed
- `.claude/skills/` when architecture or workflows change
- This file when workspace layout, commands, or repo-wide conventions change

## Rules

- **Rule 1** — Think Before Coding.
    No silent assumptions. State what you're assuming. Surface trade-offs. Ask before guessing. Push back when a simpler approach exists.
- **Rule 2** — Simplicity First.
    Minimum code that solves the problem. No speculative features. No abstractions for single-use code. If a senior engineer would call it overcomplicated — simplify.
- **Rule 3** — Surgical Changes.
    Touch only what you must. Don't "improve" adjacent code, comments, or formatting. Don't refactor what isn't broken. Match existing style.
- **Rule 4** — Goal-Driven Execution.
    Define success criteria. Loop until verified. Don't tell Claude what steps to follow, tell it what success looks like and let it iterate.

## Features and Fixes

- each package area will have a `features` and `fixes` directory which contains specs
    - **features** tend to be larger in scope and can introduce new features
    - **fixes** are primarily focused on fixing existing functionality
- we have two lifecycle directories `_unscheduled` and `_completed` which can be found as subdirectories of features/fixes
    - features/fixes which have been identified but not scheduled (aka, lower urgency) will be found in `_unscheduled` with non-dated filename
    - features/fixes as direct subdirectories are "active" features/fixes and should always follow the format `YYYY-MM-DD-{name}`
        - the files in a feature/fix can vary but almost always will be the `spec.md` file
    - when a feature/fix is completed it is moved to `_completed`

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **rusty-biscuit** (158905 symbols, 335372 relationships, 815 execution flows).

> Index stale? Run `node .gitnexus/run.cjs analyze --index-only` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? Bootstrap with `npx`, `bunx`, or `pnpm dlx` — e.g. `bunx gitnexus@latest analyze` (npm 11 npx crash; #1939).

## Always Do

- **MUST run impact before editing.** Use `impact({target: "symbolName", direction: "upstream"})` or `node .gitnexus/run.cjs impact "symbolName" --direction upstream --repo .`; report callers, processes, and risk. Never substitute grep for graph analysis.
- **MUST analyze graph changes before committing.** Use `detect_changes({scope: "all"})` (MCP) or `node .gitnexus/run.cjs detect-changes --scope all --repo .` (CLI fallback). `partial: true` or `truncated: true` is not a clean check — a zero means unseen, not unaffected; re-run it. For regression review: `detect_changes({scope: "compare", base_ref: "main"})` or `node .gitnexus/run.cjs detect-changes --scope compare --base-ref "main" --repo .`.
- MUST warn on HIGH/CRITICAL `risk` pre-edit; never use `riskSharedAxes` to waive a HIGH/CRITICAL `risk` warning. Compare File/symbol: MCP File omits axes; Graph-RAG expands File.
- **MUST treat `risk: UNKNOWN` as unresolved, not as low.** An empty caller set is not evidence the symbol is unused — it can also mean the callers are not resolvable by the index (plain-object property access, dynamic dispatch, cross-language calls). `impact` pairs `UNKNOWN` with a `riskNote` saying so. Confirm with a text search before treating the symbol as safe to change or delete; do not proceed on the strength of a zero.
- **MUST use `query({search_query: "concept"})` for concepts/flows, `context({name: "symbolName"})` for a named symbol, or `impact` for blast radius, on read-only callers, dependencies, imports, or execution flow.** Graph first; text search only for empty/`UNKNOWN`/literals.
- For security review, `explain({target: "fileOrSymbol"})` lists taint findings (source→sink flows; needs `analyze --pdg`).

## Never Do

- NEVER edit a function, class, or method before MCP/CLI impact analysis.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis, and never read `UNKNOWN` as an all-clear — it means the walk could not answer, which is the one verdict that requires confirming by other means.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit before MCP/CLI graph change analysis.

## Resources

| Resource | Use for |
| --- | --- |
| `gitnexus://repo/rusty-biscuit/context` | Codebase overview, check index freshness |
| `gitnexus://repo/rusty-biscuit/clusters` | All functional areas |
| `gitnexus://repo/rusty-biscuit/processes` | All execution flows |
| `gitnexus://repo/rusty-biscuit/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
| --- | --- |
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
