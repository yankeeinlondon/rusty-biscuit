---
area: biscuit-file
status: draft-spec
created: 2026-10-01
owner: Ken Snyder <ken@ken.net>
origin: ruling 6 (2026-10-01) on 2026-09-30-glob-reference
related:
    - 2026-09-30-glob-reference
    - 2026-09-30-reusable-path
packages:
    - biscuit-file
---

# Should a bare reference fall back to `base_dir()`?

## Question

A bare reference (`docs/spec.md`, or a bare glob such as `**/*spec*.md`)
searches two roots: the context's `cwd`, then the repository root. Since
`2026-09-30-reusable-path`, a context also knows its tree root, `base_dir()`,
which is a boundary when it comes from a repository, an explicit
`with_base_dir`, a containing vault, or the `~` / `{{VAR}}` anchor that opened
the document.

Should the second root be `base_dir()` when it is a boundary, instead of the
repository root? The change would apply to `FileReference` and
`GlobReference` together, so a bare single file and a bare glob keep the same
roots.

## What would change

Inside a repository the two are the same directory, since the tree root is
always the repository root there. Only non-repository trees change:

| Tree root origin | Bare fallback today | With this change |
|------------------|---------------------|------------------|
| Repository | repository root | repository root (unchanged) |
| Explicit (`with_base_dir`) | none (`cwd` only) | the explicit root |
| Vault | none (`cwd` only) | the containing vault root |
| Home or Environment | none (`cwd` only) | the home or `{{VAR}}` anchor |
| Fallback | none (`cwd` only) | none (`cwd` only; the tree root is `cwd`) |

For example, a note at `~/notes/projects/x.md`, opened as `'~/notes/projects/x.md'`
inside a vault rooted at `~/notes`, could then write `templates/daily.md` and
find `~/notes/templates/daily.md`, as a document in a repository finds a
repository-root file today.

## Context

The feature `2026-09-30-glob-reference` keeps the shipped behavior (bare is
`cwd`, then the repository root) for both types and deferred this question
here, because it changes single-file resolution as well as globs and is
independent of Darkmatter.

To settle before this becomes a scheduled fix:

- whether every boundary origin qualifies, or only some (an explicit root
  and a vault seem natural; a home anchor makes all of `$HOME` a fallback
  root);
- how `PortablePath`'s bare-input handling and `complete_partial_in_context`
  follow, so completion and resolution keep the same roots;
- which existing references, tests, and docs would resolve differently.
