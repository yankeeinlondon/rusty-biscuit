---
created: 2026-09-23
status: draft-spec
clarified: false
reviewed: false
implemented: false
area: biscuit-file
packages:
    - biscuit-file
    - claudine
    - claudine-cli
    - darkmatter
---

# `@` resolution exhausts local roots before the home directory

## Summary

`claudine compose @prompts/commit.md`, launched from `~/config/sh` (a plain
Git repository with no Cargo workspace and no `prompts/` directory), failed
with `Unresolvable file reference` even though `~/.claudine/prompts/commit.md`
exists and shell completion in the same directory offers
`~/.claudine/prompts/commit.md`. The failure's `Tried:` list also contained
candidates such as `…/prompts/prompts/commit.md` that looked like resolver bugs.

The investigation found four defects. Defect 1 (Claudine) is already fixed
(`259a482e4`). Defects 2 and 4 are in biscuit-file's `@` (magic) root chain,
and defect 3 is in Claudine's not-found diagnostic; this spec schedules those
three. The governing rule is:

> The local file tree resolves first. Its root is the repository root when
> the launch directory is inside a repository, and the launch directory
> itself when it is not; this holds whether or not that directory is below
> `$HOME`. Only when the local tree cannot resolve a reference does
> resolution fall back to home-based paths.

1. **(Claudine, fixed.)** The path-shaped `@prompts/<x>` form could not
   reach the `.claudine` prompt tiers.
2. **(biscuit-file.)** Magic roots have no notion of locality. A
   caller-registered home root can outrank the package, package-area, and
   repository roots, and a caller-registered repository root can be outranked
   by the home directory.
3. **(Claudine diagnostic.)** The not-found error lists every joined
   candidate path. That fills it with non-matches such as
   `<root>/prompts/prompts/<x>`, which read as resolver bugs even though they
   are just the rules applied.
4. **(biscuit-file.)** Outside a repository the launch directory is not an
   `@` root at all, so the local tree is never searched.

## Background: the reported failure

```text
CompositionError: Unresolvable file reference
Cannot resolve `@prompts/commit.md` from launch directory `/Users/ken/config/sh`.
Tried:
- magic: `/Users/ken/config/sh/prompts/prompts/commit.md`
- magic: `/Users/ken/config/sh/.claudine/prompts/prompts/commit.md`
- magic: `/Users/ken/config/sh/docs/prompts/commit.md`
- magic: `/Users/ken/config/sh/.claude/skills/prompts/commit.md`
  … (six more agent-skill peers)
- magic: `/Users/ken/.claudine/prompts/prompts/commit.md`
- repository: `/Users/ken/config/sh/prompts/commit.md`
- home: `/Users/ken/prompts/commit.md`
```

The launch directory is a repository nested **inside** `$HOME`. That layout
is common (dotfile and config repos), and before this fix no test exercised
it.

## Current behavior

### How the magic chain is built

`collect_roots` (`biscuit-file/lib/src/file_reference/resolve.rs`) orders
`ReferenceKind::Magic` roots as follows:

1. configured **prepend** roots (`PathPosition::Start`)
2. package root, then package-area root (intrinsic)
3. repository root (intrinsic)
4. home directory (intrinsic)
5. configured **append** roots (`PathPosition::End`)

`completion_roots` in the same file re-implements that order by hand ("Mirrors
the execution candidate builder"). Nothing enforces that the two agree.

Claudine registers its prompt conventions through `with_prompt_magic_roots`
(`claudine/lib/src/composition/resolve.rs`):

- prepended (`prompt_magic_roots`): `<pkg>/prompts`, `<area>/prompts`,
  `<repo>/prompts`, `<repo>/.claudine/prompts`, `<repo>/docs`,
  `<repo>/.{claude,codex,gemini,opencode,goose,qwen,kimi}/skills`, and
  `~/.claudine/prompts`
- appended (`prompt_magic_fallback_roots`, added by the Claudine fix below):
  `<repo>/.claudine`, `~/.claudine`

### Defect 1: the path-shaped form could not reach the `.claudine` tiers (Claudine, fixed)

The concise `@commit.md` form reaches every prompt directory. The
path-shaped `@prompts/commit.md` form is only served by a *bare* root whose
child is `prompts/`. Commit `e77d3f0a1` (2026-06-28) introduced the prompt
tiers but gave a bare root only to the package area. Neither `<repo>/.claudine`
nor `~/.claudine` was ever a root, so `@prompts/<x>` worked only where the
repository root happened to contain `prompts/`. That covers this monorepo and
nothing else.

**Fixed in `259a482e4` (2026-09-23):**

- `prompt_magic_fallback_roots` appends `<repo>/.claudine` and `~/.claudine`.
- `with_prompt_magic_roots` is the single registration point. Composition
  (`capture_file_resolution_context`, `derive_request_context_for_source`),
  `InvocationContext` (`build_file_resolution_context`), and shell completion
  (`claudine/cli/src/completion/scopes.rs`) all route through it. Before this
  there were four copy-pasted registration loops.
- The magic-order section of `shell-completions.md` (docs copy and skill copy)
  is corrected. It had already drifted from the code before this fix.
- Tests:
  - `claudine/cli/tests/l1/compose_prompt_tiers.rs` has five tests launched
    from `$HOME/config/sh`, covering a plain repository, no repository, and
    local-wins.
  - `completion_compose::compose_path_shaped_magic_offers_user_tier_from_plain_repo_under_home`
    covers completion from the same layout.
  - There are three unit tests in `composition/resolve/tests.rs`.
  - With the fallback registration disabled, the five path-shaped tests fail.

This spec keeps that fix and builds on it. Phase 2 replaces its ordering
mechanism.

### Defect 2: home candidates can outrank local candidates

`PathPosition` is the only ordering control, and it is relative to the
*whole* intrinsic block. A caller can therefore put a local root either
before every intrinsic root or after every intrinsic root, with home
included in both cases. It cannot put a root "after the local roots, before
home". Two concrete consequences under Claudine's registration today:

| Reference | Local candidate that should win | Home candidate that wins today | Why |
|---|---|---|---|
| `@x.md` | `<pkg>/x.md`, `<area>/x.md`, `<repo>/x.md` (intrinsic) | `~/.claudine/prompts/x.md` | `~/.claudine/prompts` is a prepend, so it precedes every intrinsic root |
| `@prompts/x.md` | `<repo>/.claudine/prompts/x.md` | `~/prompts/x.md` | `<repo>/.claudine` is an append, so it follows the home intrinsic root |

The first case is live today in every repository, this monorepo included.
It was reproduced on 2026-09-23 with a scratch repository at
`<tmp>/home/config/sh` (HOME=`<tmp>/home`) containing `x.md`:
`claudine compose --dry-run @x.md` composed `~/.claudine/prompts/x.md`.
The second exists only since the Defect 1 fix. The requirement is that
**local roots are exhausted before any home root, whatever the registration
position.**

### Defect 3: the not-found error lists joined paths instead of search roots

Resolution checks candidates in priority order, and the first match wins
(the most local). For `@prompts/commit.md`, `{root}/prompts/commit.md` is
tried before any configured prompt directory. A candidate such as
`{root}/prompts/prompts/commit.md` is simply a path that did not match, and
that is correct and harmless. The defect is only in the error: it prints
each joined candidate with a provenance label, so the reader has to reverse
the join to see where resolution looked, and the non-matching joins look
like mistakes. In the reported failure, 2 of the 13 `Tried:` lines are such
joins.

### Defect 4: outside a repository the local tree is never searched

The `Magic` arm of `collect_roots` consists of configured roots, package,
package area, repository, and home. It never includes the launch directory.
Inside a repository the repository root stands in for the local tree. Outside
one, nothing does, so the only candidates are home-based.

Reproduced on 2026-09-23. With HOME=`<tmp>/home`, a directory
`<tmp>/home/scratch` that is not a Git repository, and
`<tmp>/home/scratch/prompts/x.md` present, running
`claudine compose --dry-run @prompts/x.md` from `scratch` failed. The
`Tried:` list had exactly two entries:
`magic: <home>/.claudine/prompts/prompts/x.md` and
`home: <home>/prompts/x.md`. The file beside the user was never considered.

## Requirements

### R1: the local root

`FileResolutionContext` defines one **local root**:

- the repository root, when one is supplied or selected by the scope
  catalog;
- otherwise the **launch directory**, meaning the request's
  `request_base_dir`, which is preserved across `for_source` and `for_base`
  derivations.

The local root is intrinsic to the `@` chain. When there is a repository it
is the repository root, exactly as today. When there is not, it is the launch
directory, which resolves Defect 4. It is the launch directory rather than the
authoring source's directory: a trusted-external document such as
`~/.claudine/prompts/commit.md` still resolves its own `@` references against
the tree the user launched from, then home.

### R2: tiers follow from the local root

Every `@` chain root belongs to exactly one tier:

- **Local:** the package root, the package-area root, the local root, and
  every configured magic root that lexically lies inside the local root
  (after `normalize_components`).
- **User:** the home directory and every other configured magic root.

The chain is ordered as follows:

1. local prepends
2. package root → package-area root → local root (intrinsic)
3. local appends
4. user prepends
5. home directory (intrinsic)
6. user appends

No user-tier candidate may precede any local-tier candidate, in resolution,
in `candidate_plan`, or in completion. Within a tier, `PathPosition` keeps
its current meaning relative to that tier's intrinsic roots.

Tier is decided by containment in the **local root**, never by containment
in home. A local tree nested under `$HOME` (`~/config/sh`) is therefore
unambiguous: `~/config/sh/.claudine` is local, and `~/.claudine` is user. The
public API does not change: `add_magic_path(path, position)` keeps its
signature, and callers do not declare a scope. A configured root outside
both the local root and home (for example `/opt/configs`) is user-tier. That
is correct under the governing rule, because it is not part of the local
tree.

### R3: one ordering authority

`collect_roots` and `completion_roots` derive the magic chain from a single
function, so the ordering in R1 cannot drift between execution and
completion. A test asserts that both produce the same root sequence for each
entry form (`Magic`, `RepositoryRoot`, `RepositoryScoped`, `ImplicitRelative`)
from one synthetic context.

### R4: the not-found error lists the search roots

When no candidate matches, Claudine's diagnostic names the reference payload
once and lists the **search roots** (directories) in priority order. It
does not list joined candidate paths or provenance labels (`magic:`,
`repository:`, `home:`, …). A root the calling context added (a configured
magic root, `RootProvenance::Magic`) is marked `(*)`; the intrinsic roots
(package, package area, local root, home) are not. Shape:

```text
`prompts/foobar.md` was not found under any directory an `@` reference
searches:

- `{root}`
- `{root}/prompts` (*)
- `{home}`
- `{home}/.claudine` (*)
- `{home}/.claudine/prompts` (*)

(*) searched in addition to the standard `@` roots, for this context
```

Order follows R2: every local root precedes every home root. Anyone who
knows the rules can read which paths were tried. Resolution itself is
unchanged: all candidates are still considered, and the most local match
wins.

Implementation: the unresolved outcome already carries each candidate's
provenance. The root is the candidate path minus the authored payload.
Where that is awkward to recover, biscuit-file exposes the ordered root
list (path and provenance) from the same chain function as R3, so the
renderer never re-derives it. Replace both label renderers in Claudine:
`composition/error/render/mod.rs` (the `RootProvenance` → label mapping)
and `composition/error/render/provider.rs`. `RootProvenance` itself is
unchanged, because darkmatter uses it for ordering
(`compose/context/options.rs`).

### R5: Claudine's registration under the tiers

`with_prompt_magic_roots` keeps its current registrations. The tier follows
from R2:

| Root | Tier (derived) | Position |
|---|---|---|
| `<pkg>/prompts`, `<area>/prompts`, `<repo>/prompts`, `<repo>/.claudine/prompts`, `<repo>/docs`, `<repo>/.<peer>/skills` | Local | Start |
| `<repo>/.claudine` | Local | End |
| `~/.claudine/prompts` | User | Start |
| `~/.claudine` | User | End |

Outside a repository, the `<repo>/…` rows are registered against the launch
directory instead (`<launch>/prompts`, `<launch>/.claudine/prompts`,
`<launch>/.claudine`, and so on). The convention prompt directories of the
local tree then resolve just as they do in a repository.
`prompt_magic_roots` and `prompt_magic_fallback_roots` take the local root in
place of `git_root`.

With R1, `@prompts/x.md` resolves in this order:

1. `<pkg>/prompts/x.md`, `<area>/prompts/x.md`, `<repo>/prompts/x.md`
2. `<repo>/.claudine/prompts/x.md`
3. `~/.claudine/prompts/x.md`
4. `~/prompts/x.md`

That resolves the second row of the Defect 2 table.

`@x.md` resolves through every local prompt directory and every intrinsic
local root before `~/.claudine/prompts/x.md`. That resolves the first row.

With the tiers in place, `~/.claudine/prompts` and `~/.claudine` could keep
any position, because the tier alone puts them after the local tree.
Claudine keeps `Start`/`End` only to order them relative to home.

### R6: docs follow the code

- Update the magic-order section of `shell-completions.md` (both copies) to
  the R2 tiers.
- Update the `biscuit-file` skill and `biscuit-file/README.md` wherever they
  describe `add_magic_path`, `PathPosition`, or `@` order.
- Update `claudine/docs/topics/completions/compose-prompt-rules.md` "Local
  Wins" with the local-before-home rule. The section is currently
  unfinished.

## Tests

The layouts below mirror the reported case: HOME at `<tmp>/home`, and the
launch directory, where one exists, at `<tmp>/home/config/sh`.

### biscuit-file (pure, host-independent)

Build synthetic `FileResolutionContext`s through `from_snapshot`, so no
filesystem is needed. Use host-absolute roots from a `TempDir` or per-OS
literals (see `2026-08-30-path-spelling`) so the tests run on every OS.

- **Tier order:** with every anchor present and one local and one user root
  at each position, `candidate_plan("@x.md")` lists all local candidates
  before any user candidate, in exactly the R1 order.
- **Repository nested in home:** repo `/h/config/sh`, home `/h`. The
  repository root and its local roots all precede `/h/x.md`.
- **No repository, launch directory under home:** launch `/h/scratch`, home
  `/h`, no repository. `/h/scratch/x.md` is a candidate and precedes every
  `/h/…` candidate (Defect 4).
- **Tier by containment in the local root:** with repository `/h/config/sh`
  and home `/h`, a configured `/h/config/sh/.claudine` is local, while
  `/h/.claudine` and `/opt/configs` are user.
- **Derived source keeps the launch local root:** a context derived with
  `for_trusted_external_source(/h/.claudine/prompts/c.md)` from launch
  `/h/scratch` (no repository) still uses `/h/scratch` as its local root.
- **Execution/completion parity (R3):** for each entry form, the
  `collect_roots` sequence equals the `completion_roots` sequence.
- **Root list (R4):** the ordered root list for `@prompts/x.md` equals the
  chain from R2, with configured roots distinguishable from intrinsic ones.

### claudine / claudine-cli

Extend `claudine/cli/tests/l1/compose_prompt_tiers.rs` (same `stage` fixture
under `$HOME/config/sh`):

- `@x.md` with both `<repo>/x.md` and `~/.claudine/prompts/x.md` present
  resolves to the repository file. This fails today (Defect 2, row 1).
- `@prompts/x.md` with both `<repo>/.claudine/prompts/x.md` and
  `~/prompts/x.md` present resolves to the repository file. This fails today
  (Defect 2, row 2).
- From a non-repository `$HOME/scratch` containing `prompts/x.md`, both
  `@prompts/x.md` and `@x.md` resolve to that local file even when
  `~/.claudine/prompts/x.md` also exists. This fails today (Defect 4).
- A miss on `@prompts/missing.md` renders the R4 shape:
  - the payload `prompts/missing.md` appears once
  - the search roots are listed, with no joined candidate paths and no
    provenance labels
  - every local root precedes the first home root
  - configured roots, and only those, carry `(*)`
- Keep the existing five tests and the completion test green. Add a
  completion test showing that a same-named local and user prompt collapse to
  the local one when the partial is `@prompts/`.

Update the unit tests in `claudine/lib/src/composition/resolve/tests.rs` for
the new registration shape (R5).

### darkmatter

No signature changes. Tests that register a configured magic root outside
the repository with `PathPosition::Start` and expect it to win over the
repository root now see it ordered after the local tier. Each such test is
reviewed against the governing rule, and its expectation is updated (not the
rule).

## Affected call sites

The public API is unchanged, so these are review sites, not edit sites.
Current `add_magic_path` / `PathPosition` users outside biscuit-file:

- `claudine/lib/src/composition/resolve.rs` (`with_prompt_magic_roots`)
- `claudine/lib/src/composition/sequence/expr.rs`
- `claudine/lib/src/composition/preflight/tests.rs`,
  `sequence/preflight/tests.rs`, `resolve/tests.rs`
- `darkmatter/lib/src/markdown/compose/util.rs` and
  `compose/context/options.rs` (`magic_paths: Vec<(PathBuf, PathPosition)>`
  is part of `ComposeOptions`)
- `darkmatter/lib/src/markdown/compose/cache/hashing.rs`: magic paths feed
  the compose cache key. The tier is derived from the local root, so check
  that the key already covers the repository root or launch directory;
  otherwise the same options from two launch directories could share a
  stale entry
- `darkmatter/lib/src/markdown/compose/transclusion/resolver.rs`,
  `reference/graph.rs`, `reference/validate.rs`, `compose/type_tests.rs`

## Open questions

- **OQ1: declared vs inferred scope. Resolved 2026-09-23:** neither.
  The tier follows from containment in the local root (the repository root,
  or else the launch directory), per the governing rule. See R1 and R2.
- **OQ2: provenance labels. Resolved 2026-09-23:** no tier labels, and the
  existing labels are removed from `Tried:` (R4).
- **OQ3: skipping self-nested candidates. Resolved 2026-09-23:** no skip.
  Resolution considers every candidate and takes the most local match, so a
  non-matching `{root}/prompts/prompts/x.md` is harmless. The fix is in the
  diagnostic (R4).

## Out of scope

- Vault, `^`, `&`, and implicit-relative ordering. None of them has a home
  tier.
- Sequence-document magic references (`sequence/expr.rs`) beyond what
  R1 and R2 change through the shared chain.
