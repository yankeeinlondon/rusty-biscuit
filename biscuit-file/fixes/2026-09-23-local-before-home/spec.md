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
nonsense candidates such as `…/prompts/prompts/commit.md`.

The investigation found three defects. One is in Claudine and is already
fixed in the working tree. The other two are in biscuit-file's `@` (magic)
root ordering and candidate generation, and they are what this spec
schedules:

1. **(Claudine, fixed.)** The path-shaped `@prompts/<x>` form could not
   reach the `.claudine` prompt tiers.
2. **(biscuit-file.)** Magic roots have no notion of locality. A
   caller-registered home root can outrank the package, package-area, and
   repository roots, and a caller-registered repository root can be outranked
   by the home directory. The rule we want is absolute: **every local
   candidate is tried before any home-directory candidate.**
3. **(biscuit-file.)** A prompt directory is joined with a reference that
   already names that directory. That produces `<root>/prompts/prompts/<x>`
   probes, which fill the `Tried:` diagnostic with paths no one authored.

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

**Fix already in the working tree (uncommitted as of 2026-09-23):**

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

### Defect 3: self-nested prompt-directory probes

A prompt directory `R = P/<name>` is joined with the whole reference. When
the reference begins with `<name>/`, the candidate is `P/<name>/<name>/…`
(`prompts/prompts/commit.md`, `docs/docs/x.md`). Whenever `P` is itself a
root in the same chain, the author's evident meaning, `P/<name>/…`, is
already produced by `P`. The self-nested candidate is then a probe of a path
nobody wrote. It costs a `stat` and, worse, shows up in user-facing
diagnostics as if the resolver meant to look there. In the reported failure,
2 of the 13 `Tried:` lines are self-nested.

## Requirements

### R1: locality tiers

Every magic-chain root belongs to exactly one tier:

- **Local:** the package root, package-area root, repository root, and every
  configured magic root registered as local.
- **User:** the home directory and every configured magic root registered as
  user-scoped.

The magic chain is ordered as follows:

1. local prepends
2. package root → package-area root → repository root (intrinsic)
3. local appends
4. user prepends
5. home directory (intrinsic)
6. user appends

No user-tier candidate may precede any local-tier candidate, in resolution,
in `candidate_plan`, or in completion. Within a tier, `PathPosition` keeps
its current meaning relative to that tier's intrinsic roots.

### R2: explicit scope on registration

Callers declare the tier when registering a magic root. Recommended shape:

```rust
pub enum MagicScope { Local, User }

impl FileResolutionContext {
    pub fn add_magic_path(self, path: impl Into<PathBuf>, scope: MagicScope, position: PathPosition) -> Self;
}
```

(`FileReference::add_magic_path` changes the same way.) Every call site in
the workspace is updated in the same change: claudine lib, darkmatter
compose/reference/transclusion, and tests. The current list is in "Affected
call sites" below.

Scope is declared rather than inferred from path containment, because the
motivating layout, a repository nested inside `$HOME`, makes lexical
containment ambiguous: `~/config/sh/.claudine` lies inside both the repository
and the home directory. See OQ1.

### R3: one ordering authority

`collect_roots` and `completion_roots` derive the magic chain from a single
function, so the ordering in R1 cannot drift between execution and
completion. A test asserts that both produce the same root sequence for each
entry form (`Magic`, `RepositoryRoot`, `RepositoryScoped`, `ImplicitRelative`)
from one synthetic context.

### R4: provenance distinguishes the tiers

Diagnostics label each candidate with its tier, so a `Tried:` list shows the
user where the local search ended and the home fallback began. Recommended:
add `RootProvenance::UserMagic` alongside `Magic`, which becomes
"local magic". Claudine's renderer (`composition/error/render/lifecycle.rs`
and the `Tried:` enumeration) prints it distinctly. See OQ2 for naming.

### R5: no self-nested probes

When building candidates for a relative magic payload `S = s₁/s₂/…`, skip a
root `R` when:

- `R`'s final component equals `s₁`, **and**
- `R`'s parent is itself a root earlier or later in the same chain (intrinsic
  or configured).

That parent already produces `parent/s₁/…`, which is the path the author
meant. The skip applies equally to resolution, `candidate_plan`, and
completion, so a skipped candidate never appears in `Tried:`. Roots whose
parent is not in the chain (for example `<repo>/.claude/skills`, whose parent
`<repo>/.claude` is not a root) are unaffected. See OQ3.

### R6: Claudine registers by tier

`with_prompt_magic_roots` registers:

| Root | Scope | Position |
|---|---|---|
| `<pkg>/prompts`, `<area>/prompts`, `<repo>/prompts`, `<repo>/.claudine/prompts`, `<repo>/docs`, `<repo>/.<peer>/skills` | Local | Start |
| `<repo>/.claudine` | Local | End |
| `~/.claudine/prompts` | User | Start |
| `~/.claudine` | User | End |

With R1, `@prompts/x.md` resolves in this order:

1. `<pkg>/prompts/x.md`, `<area>/prompts/x.md`, `<repo>/prompts/x.md`
2. `<repo>/.claudine/prompts/x.md`
3. `~/.claudine/prompts/x.md`
4. `~/prompts/x.md`

That resolves the second row of the Defect 2 table.

`@x.md` resolves through every local prompt directory and every intrinsic
local root before `~/.claudine/prompts/x.md`. That resolves the first row.

`prompt_magic_fallback_roots` either folds into `with_prompt_magic_roots` or
keeps its name, whichever gives the smaller diff. The public
`prompt_magic_roots` return type changes to carry scope and position, or is
replaced by a single function that returns `(PathBuf, MagicScope,
PathPosition)` triples; completion and the unit tests use the same list.

### R7: docs follow the code

- Update the magic-order section of `shell-completions.md` (both copies) to
  the R1 tiers.
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
- **No repository:** only user-tier roots plus any explicitly local roots.
  Home is still after local roots.
- **Execution/completion parity (R3):** for each entry form, the
  `collect_roots` sequence equals the `completion_roots` sequence.
- **Self-nesting (R5):**
  - With `R = /r/prompts` and `/r` in the chain, `@prompts/x.md` yields no
    `/r/prompts/prompts/x.md`.
  - With `R = /r/.claude/skills` and `/r/.claude` not in the chain,
    `@skills/x.md` keeps `/r/.claude/skills/skills/x.md`.
  - `@x.md` is unaffected.
- **Provenance (R4):** user-scoped configured roots report `UserMagic`, and
  local ones report `Magic`.

### claudine / claudine-cli

Extend `claudine/cli/tests/l1/compose_prompt_tiers.rs` (same `stage` fixture
under `$HOME/config/sh`):

- `@x.md` with both `<repo>/x.md` and `~/.claudine/prompts/x.md` present
  resolves to the repository file. This fails today (Defect 2, row 1).
- `@prompts/x.md` with both `<repo>/.claudine/prompts/x.md` and
  `~/prompts/x.md` present resolves to the repository file. This fails today
  (Defect 2, row 2).
- A miss on `@prompts/missing.md` renders a `Tried:` list that:
  - contains no `prompts/prompts` segment
  - lists every local candidate before the first user or home candidate
  - labels the tiers distinctly (R4)
- Keep the existing five tests and the completion test green. Add a
  completion test showing that a same-named local and user prompt collapse to
  the local one when the partial is `@prompts/`.

Update the unit tests in `claudine/lib/src/composition/resolve/tests.rs` for
the new registration shape (R6).

### darkmatter

Existing `add_magic_path` users (see below) compile against the new
signature and keep their behavior. Each existing test root is local unless
the test is specifically about a home or external root.

## Affected call sites

Current `add_magic_path` / `PathPosition` users outside biscuit-file:

- `claudine/lib/src/composition/resolve.rs` (`with_prompt_magic_roots`)
- `claudine/lib/src/composition/sequence/expr.rs`
- `claudine/lib/src/composition/preflight/tests.rs`,
  `sequence/preflight/tests.rs`, `resolve/tests.rs`
- `darkmatter/lib/src/markdown/compose/util.rs` and
  `compose/context/options.rs` (`magic_paths: Vec<(PathBuf, PathPosition)>`
  is part of `ComposeOptions`)
- `darkmatter/lib/src/markdown/compose/cache/hashing.rs`: magic paths feed
  the compose cache key, so scope must be hashed too, or a local/user swap
  would hit a stale cache entry
- `darkmatter/lib/src/markdown/compose/transclusion/resolver.rs`,
  `reference/graph.rs`, `reference/validate.rs`, `compose/type_tests.rs`

## Open questions

- **OQ1: declared vs inferred scope.** The recommendation is declared (R2).
  The alternative infers scope lexically: a root inside the repository
  root is local, anything else is user. That avoids an API change, but it
  classifies `/opt/configs`-style external roots as user and depends on
  spelling normalization for containment.
- **OQ2: provenance naming.** `Magic` + `UserMagic`, or `Magic { scope }`.
  The field form is cleaner, but it changes a public enum variant's shape
  and every exhaustive match on it.
- **OQ3: R5 is a resolution change, not only a diagnostic one.** After R5,
  a file literally at `<repo>/prompts/prompts/x.md` is no longer reached by
  `@prompts/x.md`; it needs `@prompts/prompts/x.md`. The alternative is to
  keep probing and only hide such candidates from `Tried:`. That keeps
  resolution unchanged but makes the diagnostic under-report what was probed.
  The recommendation is the resolution change, because the self-nested path
  is never what the reference's text says.

## Out of scope

- Vault, `^`, `&`, and implicit-relative ordering. None of them has a home
  tier.
- Sequence-document magic references (`sequence/expr.rs`) beyond the
  signature update.
