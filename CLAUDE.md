# Rusty Biscuit Monorepo

## Language 

- always prefer **US English (en-US)** over other English variants such as UK English (en-GB) when creating symbol names or writing documentation

## Package Area Conventions

- you can run `sniff repo package-areas` for a list of all the _package areas_ in this monorepo
    - most of the package areas consist of a library and CLI pairing though there is some variance to that pattern
- you can run `sniff repo packages` to get the full list of packages in this monorepo

## Just Runner

- we use the `just` runner extensively throughout this monorepo. 
- you will find a justfile at the root of this monorepo and a justfile in each of the _package areas_
- shared recipes for just can be found in the @just/ directory

## Git Identity and Signing

- all commits must use the author `Ken Snyder <ken@ken.net>` and must be
  OpenPGP-signed
- repository-local Git configuration should set `user.name`, `user.email`,
  `user.signingkey`, and `commit.gpgsign`; verify these values before committing
- Claudine may replace `HOME` for a wrapped agent while the signing key remains
  in the user's normal GPG home; the agent launcher must capture that location
  before changing `HOME` and preserve it through `GNUPGHOME`
- if GPG reports `No secret key`, do not bypass signing with
  `--no-gpg-sign`; compare the active GPG home with the value reported by
  `gpgconf --list-dirs homedir` in the user's normal, pre-wrapper environment,
  then retry with that value assigned to `GNUPGHOME`
- verify every new commit with `git verify-commit HEAD` in the same signing
  environment before reporting success

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
