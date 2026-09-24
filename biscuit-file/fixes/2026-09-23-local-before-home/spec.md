---
created: 2026-09-23
status: draft-spec
clarified: false
implemented: false
area: biscuit-file
packages:
    - biscuit-file
    - claudine
    - claudine-cli
    - darkmatter
human_review: true
human_review_items:
    - |-
        **Ruling 1 — how `@` roots are split into local versus user tiers when
        the launch directory or repository is `$HOME` itself.** Phases 2 and 3
        implemented the recommended option (infer by containment, with an
        explicit `MagicPathTier::User` override via
        `add_magic_path_with_tier`), and Phase 3 wired that override through
        Claudine's two `~/.claudine` registrations; the consumer tests now
        depend on it, so confirming or overturning the ruling is cheapest
        before Phase 4 hardens the CLI regression suite and documentation.

        A path-only rule cannot tell Claudine's user-level prompt directory
        (`~/.claudine/prompts`) apart from a local convention when the launch tree
        *is* `$HOME` — both would land in the local tier and the user prompt
        directory could outrank local files, recreating the bug at the home root.

        Options:

        - **Infer everything from paths.** No new API, but the overlap case has no
          correct answer: the home prompt directory outranks local files when the
          launch directory is `$HOME`. Cons: recreates this bug at the home root.
        - **Require every caller to declare a tier.** Unambiguous everywhere, but
          forces every client of `add_magic_path` to make a choice even when
          containment is obvious, churning all call sites for no behavioral gain.
        - **Infer by default, with an explicit tier override (implemented).**
          `add_magic_path` keeps its signature and inferred meaning for ordinary
          roots; Claudine marks its two `~/.claudine` registrations as user-tier.

        **Recommendation: keep the implemented infer-with-override design.** The
        caller knows whether a root is a user convention; a path comparison
        cannot. Please confirm, or pick a different option before Phase 4's
        CLI regression and documentation work lands.
    - |-
        **Ruling 2 — which tree is local for `@` references nested inside a prompt
        loaded from `~/.claudine` or another repository.** Phases 2 and 3
        implemented the recommended option: an immutable `LaunchMagicScope` on
        `FileResolutionContext`, preserved through `for_source`/`for_base`/
        trusted-external derivations, plus `with_launch_magic_scope` for
        requests that rebuild their context around an external source; Phase 3
        carries the snapshot through Claudine's `derive_source` paths and
        registers conventions against the launch local root. One CLI
        expectation was deliberately flipped to the new rule (nested `@` now
        resolves from the launch tree), so confirming the ruling is cheapest
        before Phase 4 hardens the CLI regression suite and documentation.

        Options:

        - **Keep source-derived contexts unchanged.** Minimal work, but nested `@`
          references keep violating the "local tree first" rule this fix exists to
          establish.
        - **Use the launch context for every nested reference.** Makes `@`
          consistent with the launch, but silently changes `./`, bare, `&`, and `^`
          references in externally loaded documents, breaking their documented
          source-relative meaning.
        - **Keep a launch `@` scope separate from source anchors (implemented).**
          `@` keeps the invocation's local-first search; `./`, bare, `&`, and `^`
          keep their source semantics.

        **Recommendation: keep the implemented separate-scope design.** It meets
        the user-facing rule without changing the meaning of the other reference
        kinds. Please confirm, or pick a different option before Phase 4's
        `derive_source`-dependent CLI tests and docs land.
message_to_agent: |-
    Phase 3 (consumer integration) is complete; `just test` / `just lint`
    pass in the claudine (7341), darkmatter (8497), and claudine-cli (2776,
    `test-fixtures`) areas and `cargo check --workspace --all-targets` is
    clean. What Phase 4 builds on:

    - Claudine registers conventions against the launch local root
      (repo root, else launch dir) everywhere, and the two `~/.claudine`
      rows are explicit `MagicPathTier::User`. `derive_source` and the
      compat `derive_request_context_for_source` seed
      `with_launch_magic_scope(launch.launch_magic_scope().clone())` last.
    - `CompositionError::from_detailed_no_match(detailed, context)` now
      takes the resolving context; `ResolutionDetail::magic_search_roots()`
      carries the ordered chain for direct `@` misses (empty otherwise).
    - The `@` miss report is live: payload once, ordered search-root
      directories, `(*)` only on configured roots, footnote
      "(*) searched in addition to the standard `@` roots, for this
      context"; bare/absolute misses keep "Cannot resolve … Tried:". For
      task 4.1's CLI assertions, render through `report_block_error` with
      a `ColorDepth::None` terminal (see `plain_terminal()` in
      `claudine/lib/src/composition/resolve/tests.rs`) and note that a
      100-column terminal word-wraps long root lines — assert on paths,
      not exact lines, or use a wide terminal.
    - Ruling 2 flipped one CLI expectation on purpose:
      `sequence_magic_reference_follows_launch_scope_and_relative_stays_source_anchored`
      (nested `@` follows the launch tree; `./` stays source-anchored).
      Task 4.1's nested-prompt cases should follow the same rule.
    - Darkmatter identity encodes package_root, the launch `@` scope, and
      tier-aware magic registrations; `LocalRoot` is cache code 8.
    - Docs/skills are untouched so far — task 4.2 owns them, including the
      `shell-completions.md` magic-order section and the unfinished
      "Local Wins" section of `compose-prompt-rules.md`.
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6-sol
reviewed_on: 2026-09-23
review_iterations: 0
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
three. The intended rule for an invocation's `@` references is:

> The local file tree resolves first. Its root is the repository root when
> the launch directory is inside a repository, and the launch directory
> itself when it is not; this holds whether or not that directory is below
> `$HOME`. Only when the local tree cannot resolve a reference does
> resolution fall back to home-based paths.

The rule needs two decisions before implementation: how to distinguish local
and user roots when the local root is `$HOME`, and how to keep the launch tree
for `@` references in a prompt loaded from another repository. Both are
recorded as rulings in [Rulings](#rulings); the [open questions](#open-questions) give the options and
recommendations they were chosen from. The requirements
below describe the intended behavior in the ordinary, non-overlapping case.

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

[`collect_roots`](../../lib/src/file_reference/resolve.rs) in biscuit-file orders
`ReferenceKind::Magic` roots as follows:

1. configured **prepend** roots (`PathPosition::Start`)
2. package root, then package-area root (intrinsic)
3. repository root (intrinsic)
4. home directory (intrinsic)
5. configured **append** roots (`PathPosition::End`)

[`completion_roots`](../../lib/src/file_reference/resolve.rs) in biscuit-file re-implements that order by hand ("Mirrors
the execution candidate builder"). Nothing enforces that the two agree.

Claudine registers its prompt conventions through
[`with_prompt_magic_roots`](../../../claudine/lib/src/composition/resolve.rs):

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

By default, biscuit-file's
[`FileResolutionContext`](../../lib/src/file_reference/context.rs) derives one
**local root** for its `@` chain:

- the repository root, when one is supplied or selected by the scope
  catalog;
- otherwise the **request directory**, recorded in `request_base_dir` and
  preserved by that context's `for_source` and `for_base` derivations. For a
  top-level Claudine command, this is the launch directory.

The local root is intrinsic to the `@` chain. With a repository it is the
repository root, as today. Without one it is the request directory, which
resolves Defect 4. It is distinct from `base_dir`, the directory of the
document authoring a reference. A context derived from a launch context for a
trusted external prompt should therefore still search the launch tree first.
Open question 2 recommends letting a caller explicitly preserve a launch
`@` scope when its source-specific repository anchor differs.

**Integration gap:** Claudine does not currently derive every source context
from its launch context. [`InvocationContext::derive_source`](../../../claudine/lib/src/invocation_context.rs)
and [`derive_request_context_for_source`](../../../claudine/lib/src/composition/resolve.rs)
build fresh contexts with the source's parent as `request_base_dir`. Merely
changing biscuit-file's `for_source` behavior will not make nested prompt
references launch-local. Implement the selected cross-repository policy in
[Open question 2](#open-questions) in these paths and in the corresponding
completion context; keep source-relative references tied to their authoring
base.

### R2: tiers follow from the local root

Every `@` chain root belongs to exactly one tier:

- **Local:** the package root, the package-area root, the local root, and
  every configured magic root that lexically lies inside the local root
  (after `normalize_components`), subject to the overlapping-root case in
  [Open question 1](#open-questions).
- **User:** the home directory and every other configured magic root.

The chain is ordered as follows:

1. local prepends
2. package root → package-area root → local root (intrinsic)
3. local appends
4. user prepends
5. home directory (intrinsic)
6. user appends

No user-tier candidate may precede any local-tier candidate in resolution,
`candidate_plan`, recursive `%@` search, or completion. Within a tier,
`PathPosition` keeps its current meaning relative to that tier's intrinsic
roots. Preserve registration order within each position. Deduplicate
normalized roots after ordering, keeping the first root and its provenance;
this also preserves first-match behavior when a configured root equals an
intrinsic one.

Tier is decided by containment in the **local root**, never by containment
in home. A local tree nested under `$HOME` (`~/config/sh`) is therefore
unambiguous: `~/config/sh/.claudine` is local, and `~/.claudine` is user. The
existing [`add_magic_path`](../../lib/src/file_reference/context.rs) method in
biscuit-file keeps its signature for ordinary roots. A configured root outside
the local root and home (for example `/opt/configs`) is in the fallback tier.
Relative configured roots need a defined base before containment is tested:
interpret them against the captured request directory, without reading the
process working directory later. The joined candidate and the tier must use
that same absolute spelling. Lexical containment is an ordering rule, not a
security boundary; do not resolve symlinks solely to classify a tier.
This intentionally changes a relative root supplied to ambient
`resolve_from(base)`: it follows `base` rather than the process working
directory. Document that change and test it. On Windows, compare normalized
components in one captured path spelling; do not assume an 8.3 short path and
its long spelling compare equal lexically.

**Reader's note:** containment alone cannot distinguish a local prompt root
from a user prompt root if the launch directory itself is `$HOME` or `$HOME`
is a repository. The proposed explicit override in Open question 1 addresses
this without reclassifying a user's home configuration as a local convention.

### R3: one ordering authority

In biscuit-file, `collect_roots` and `completion_roots` use one ordered root
builder for `@`; completion appends its typed path segment only **after**
selecting those roots. The other entry forms (`&` for repository root, `^` for
package then repository, and bare paths for authoring base then repository)
retain their own established order. Tests compare completion's roots for a
partial token with the matching resolver plan for a completed token, including
the same path segment and root deduplication. This checks the actual behavior,
not merely two internal functions that could agree on the wrong order.

### R4: the not-found error lists the search roots

For a direct `@` reference with no match, Claudine's human-readable
diagnostic names the reference payload once, replacing its current
"Cannot resolve" sentence, and lists the **search roots**
(directories) in priority order. It does not list joined candidate paths or
provenance labels (`magic:`, `repository:`, `home:`, …). A configured magic
root is marked `(*)`; the intrinsic roots (package, package area, local root,
home) are not. Other reference kinds keep their existing diagnostics, because
an absolute path or a source-relative path does not have this `@` search
chain. Shape:

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

Order follows R2: every local root precedes every home root. Keep Claudine's
existing structured diagnostic candidate list, with concrete paths, probe
results, and provenance, for tools that inspect it. Only the human-readable
list changes. Resolution still considers the same candidates and the first
regular file in the newly ordered chain wins.

Expose the ordered `@` roots (path and provenance) from the biscuit-file
context using the R3 builder. On a no-match, Claudine stores that root list
alongside its existing [`ResolutionDetail`](../../../claudine/lib/src/harness/error.rs)
probe record; the renderer reads the stored list. Do not subtract the
authored payload from a candidate: interpolation, normalized paths, and
duplicate candidates make that reconstruction unreliable. Change the
`@`-specific branch of [`provider.rs`](../../../claudine/lib/src/composition/error/render/provider.rs)
and any other renderer of this same error, while leaving labels for other
reference kinds and the machine-readable candidate record intact.

When there is no repository, give the intrinsic request-directory root a
distinct `RootProvenance::LocalRoot` value in biscuit-file. `Source` continues
to mean the authoring base for bare and explicit-relative references. Reusing
`Source` for the request directory would make Darkmatter's
`CandidatePlanOrder::AuthoringBaseFirst` move the wrong `@` candidate. Update
the exhaustive provenance mappings in Claudine and Darkmatter, including
Darkmatter's cache encoding, while preserving all existing variant codes.

### R5: Claudine's registration under the tiers

For a launch in a repository below `$HOME`, Claudine's
[`with_prompt_magic_roots`](../../../claudine/lib/src/composition/resolve.rs)
keeps its registrations. The tier follows from R2:

| Root | Tier | Position |
|---|---|---|
| `<pkg>/prompts`, `<area>/prompts`, `<repo>/prompts`, `<repo>/.claudine/prompts`, `<repo>/docs`, `<repo>/.<peer>/skills` | Local | Start |
| `<repo>/.claudine` | Local | End |
| `~/.claudine/prompts` | User | Start |
| `~/.claudine` | User | End |

Outside a repository, register the `<repo>/…` convention rows against the
launch directory instead (`<launch>/prompts`,
`<launch>/.claudine/prompts`, `<launch>/.claudine`, and so on). The convention
prompt directories of the local tree then resolve just as they do in a
repository. Claudine's [`prompt_magic_roots`](../../../claudine/lib/src/composition/resolve.rs)
and [`prompt_magic_fallback_roots`](../../../claudine/lib/src/composition/resolve.rs)
take that local root in place of `git_root`. Their callers include
[`InvocationContext`](../../../claudine/lib/src/invocation_context.rs),
composition compatibility paths, and
[`file_resolution_context`](../../../claudine/cli/src/completion/scopes.rs)
for shell completion. Apply the same root source to all three; changing the
helper alone would leave their source-derived contexts inconsistent.

With R1, these **matching locations** for `@prompts/x.md` win in this order;
the complete chain still includes other configured roots that may produce
nonmatching joined paths:

1. `<pkg>/prompts/x.md`, `<area>/prompts/x.md`, `<repo>/prompts/x.md`
2. `<repo>/.claudine/prompts/x.md`
3. `~/prompts/x.md`
4. `~/.claudine/prompts/x.md`

That resolves the second row of the Defect 2 table.

`@x.md` resolves through every local prompt directory and every intrinsic
local root before `~/.claudine/prompts/x.md`. That resolves the first row.

With the tiers in place, `~/.claudine/prompts` and `~/.claudine` use
`Start`/`End` to order them relative to the intrinsic home root. Their
intended user-tier classification must also hold in the `$HOME` overlap case
described in Open question 1.

### R6: docs follow the code

- Update the authoritative
  [file-reference topic](../../docs/topics/file-references.md), the
  [biscuit-file skill](../../../.claude/skills/biscuit-file/references/file-references.md),
  and biscuit-file README examples wherever they describe the `@` order or
  the meaning of `PathPosition`.
- Update the magic-order section of Claudine's `shell-completions.md` in both
  its documentation and skill copies to the R2 tiers.
- Complete Claudine's
  ["Local Wins" section](../../../claudine/docs/topics/completions/compose-prompt-rules.md)
  with the launch-local rule, including the chosen policy for prompts loaded
  from another repository. The section is currently unfinished.
- Review comments on the changed resolver, context, Claudine registration,
  and Darkmatter `with_magic_path` symbols so their stated order and anchor
  still match the code.

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
  `/h`, no repository. `/h/scratch/x.md` is a candidate and precedes the
  home-tier `/h/x.md` candidate (Defect 4).
- **Tier by containment in the local root:** with repository `/h/config/sh`
  and home `/h`, a configured `/h/config/sh/.claudine` is local, while
  `/h/.claudine` and `/opt/configs` are user.
- **Derived source keeps the launch local root:** a context derived with
  `for_trusted_external_source(/h/.claudine/prompts/c.md)` from launch
  `/h/scratch` (no repository) still uses `/h/scratch` as its local root.
- **Relative configured root:** a root registered as `prompts` resolves and
  classifies against the captured request directory even after deriving a
  source with a different authoring base. The ambient `resolve_from(base)`
  form uses `base` for this purpose too.
- **Recursive magic:** `%@x.md` walks local roots before user roots, and a
  duplicate configured/intrinsic root is searched once with first-seen
  provenance.
- **Execution/completion parity (R3):** for each supported entry form,
  completing a partial token and resolving the corresponding completed token
  enumerate the same ordered, normalized candidate paths.
- **Root list (R4):** the ordered root list for `@prompts/x.md` equals the
  chain from R2, with configured roots distinguishable from intrinsic ones.
  The no-repository local root has `LocalRoot` provenance; bare paths keep
  `Source` provenance.
- **Overlapping roots:** cover launch equal to home and repository equal to
  home using the selected design from Open question 1.

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
  - the structured diagnostic still contains the concrete candidates and
    probe results
- A miss on a bare or absolute reference keeps its existing diagnostic.
- Keep the existing five tests and the completion test green. Add a
  completion test showing that a same-named local and user prompt collapse to
  the local one when the partial is `@prompts/`.
- With the selected policy from Open question 2, a prompt loaded from
  `~/.claudine` or another repository resolves nested `@x.md` from the
  launch tree first; its `./x.md`, bare `x.md`, `&x.md`, and `^x.md` references
  retain their existing source-relative or source-repository meaning.

Update the unit tests in `claudine/lib/src/composition/resolve/tests.rs` for
the new registration shape (R5).

### darkmatter

Tests that register a configured magic root outside the repository with
`PathPosition::Start` and expect it to win over the repository root now see it
ordered after the local tier. Review each such test against the governing
rule and update its expectation. Keep the existing `Source`-first behavior
of `CandidatePlanOrder::AuthoringBaseFirst` for bare references.

## Affected call sites

The existing `add_magic_path` signature stays. The selected designs may add
an explicit tier override and a separate `@` scope, and `LocalRoot` adds a
public provenance variant. Review and update these callers, not just their
test expectations:

- `claudine/lib/src/composition/resolve.rs` (`with_prompt_magic_roots`)
- `claudine/lib/src/invocation_context.rs` and
  `claudine/cli/src/completion/scopes.rs` (launch versus source roots)
- `claudine/lib/src/composition/error/render/provider.rs` and
  `render/mod.rs` (human-readable error and provenance mappings)
- `claudine/lib/src/composition/sequence/expr.rs`
- `claudine/lib/src/composition/preflight/tests.rs`,
  `sequence/preflight/tests.rs`, `resolve/tests.rs`
- `darkmatter/lib/src/markdown/compose/util.rs` and
  `compose/context/options.rs` (`magic_paths: Vec<(PathBuf, PathPosition)>`
  is part of `ComposeOptions`)
- [`encode_file_resolution_context`](../../../darkmatter/lib/src/markdown/compose/context/options.rs)
  in Darkmatter already includes `request_base_dir`, repository root,
  package area, and configured magic paths in its graph identity, but omits
  `package_root`. Include `package_root` and any separately captured `@`
  scope or explicit tier override: both can change the winning file without
  changing the other fields. Preserve the existing provenance codes in the
  same file and assign a new one to `LocalRoot`. Verify the composed cache
  identity remains distinct when only the launch root or tier changes.
- `darkmatter/lib/src/markdown/compose/cache/hashing.rs`: check the separate
  compose cache key still includes the relevant root configuration.
- `darkmatter/lib/src/markdown/compose/transclusion/resolver.rs`,
  `reference/graph.rs`, `reference/validate.rs`, `compose/type_tests.rs`

## Rulings

Recorded 2026-09-23 (Phase 1, task 1.3). Both rulings adopt the recommended
option from the [open questions](#open-questions); because the recording
session was non-interactive, `human_review` remains `true` until the author
confirms them. If either ruling is overturned, revise the dependent plan tasks
(2.1, 2.2, and their acceptance checks) before implementation continues.

1. **Overlapping home and local trees — infer by default, with an explicit
   tier override.** `@` tiers follow from normalized lexical containment in
   the local root ([R2](#r2-tiers-follow-from-the-local-root)) for every
   ordinarily registered root; `add_magic_path` keeps its signature and that
   inferred meaning. A caller that knows a root is a user convention —
   Claudine's `~/.claudine/prompts` (Start) and `~/.claudine` (End)
   registrations — may explicitly mark it user-tier, and that override wins
   over inference in every layout, including launch directory equal to
   `$HOME` and `$HOME`-as-repository. A path outside the local root and home
   (for example `/opt/configs`) is a user-tier (fallback) root.
2. **Nested prompts from another tree — an immutable, request-scoped launch
   `@` snapshot, separate from source anchors.** The launch context captures
   the request directory and its selected repository, package, and
   package-area roots for `@`; that snapshot is preserved through
   `for_source`, `for_base`, and trusted-external derivations and is never
   rediscovered by source derivation or completion. A nested `@x.md` searches
   the launch tree first; `./x.md`, bare `x.md`, `&x.md`, and `^x.md` retain
   their source-specific meanings. Source derivation and completion must not
   rediscover the launch scope.

## Open questions

### 1. How are user roots identified when the local root is also `$HOME`?

If Claudine launches in `$HOME` without a repository, the local root is
`$HOME`; the same happens when `$HOME` is itself a repository. A containment
rule places both `<home>/prompts` and `<home>/.claudine/prompts` in the local
tier, despite the latter being registered as a user fallback. The draft's
original inferred-only rule cannot express their intended order.

- **Infer everything from paths.** Pros: no new API. Cons: the home prompt
  directory can outrank local files, recreating this bug at the home root;
  there is no reliable path-only way to distinguish the two registrations.
- **Require every caller to declare a tier.** Pros: unambiguous everywhere.
  Cons: changes an otherwise convenient API and forces all clients to make a
  choice even when containment is clear.
- **Infer by default, with an explicit tier override. Recommended.** Pros:
  existing calls retain their meaning in ordinary layouts; Claudine can mark
  its known home prompt roots as user roots even when the trees overlap. Cons:
  adds one builder method or tier argument and its cache/serialization
  representation. Recommend it because the caller knows whether a root is a
  user convention while a path comparison cannot know that intent. Keep the
  existing `add_magic_path` as the inferred form; use the override for
  Claudine's `~/.claudine` roots.

### 2. Which tree is local for nested references in a prompt loaded elsewhere?

Claudine's source contexts are rebuilt from the loaded prompt, so their
repository and `request_base_dir` can differ from the launch context. For
example, a prompt in `~/.claudine` or another repository can contain
`@x.md`. The top-level rule says to search the launch tree first, while
existing bare, `&`, and `^` references intentionally use source-specific
anchors. One context currently carries both meanings in one repository root.

- **Keep source-derived contexts unchanged.** Pros: minimal implementation
  and preserves all current nested behavior. Cons: nested `@` references can
  search the prompt's repository or home before the launch tree, violating
  the stated rule.
- **Use the launch context for every nested reference.** Pros: makes `@`
  consistent with the launch. Cons: changes `./`, bare, `&`, and `^` references
  in externally loaded documents and breaks their documented source meaning.
- **Keep a launch `@` scope separate from source anchors. Recommended.**
  Pros: `@` keeps the invocation's local-first search and completion order;
  `./`, bare, `&`, and `^` keep their source semantics. Cons: requires a
  separate immutable `@` root snapshot (including launch package and area
  roots) or equivalent resolver input, and Claudine must carry it through
  `InvocationContext::derive_source` and the compatibility path. Recommend
  it because it meets the user-facing rule without changing the meaning of
  the other reference kinds. The `@` snapshot must remain request-scoped;
  neither source derivation nor completion may rediscover it.

The two choices above are recorded as rulings (see
[Rulings](#rulings)), adopting each recommendation pending human confirmation.
The remaining decisions are settled: the human-readable `@` miss omits
provenance labels but keeps the structured probe record (R4), and nonmatching
joined paths such as `prompts/prompts/x.md` remain legitimate probes rather
than being skipped.

## Out of scope

- Vault, `^`, `&`, and implicit-relative ordering. None of them has a home
  tier.
- Sequence-document magic references (`sequence/expr.rs`) beyond what
  R1 and R2 change through the shared chain.
