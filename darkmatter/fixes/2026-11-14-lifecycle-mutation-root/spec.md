---
kind: fix
created: 2026-11-14
status: finalized-spec
area: darkmatter
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-14-edit-action
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
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: false
review_iterations: 0
---

# One mutation root for every lifecycle host

## Why this exists

Every file a lifecycle action writes (`edit`, `ensure_file`, `ensure_dir`,
`append_line`, `append_jsonl`, and the frontmatter mutations such as
`set_frontmatter`) must lie inside the
host's **mutation root**. The hosts disagree about that root:

| Host | Root today | Where |
|---|---|---|
| `md compose` | the directory holding the composed document | `darkmatter/cli/src/commands/compose/lifecycle.rs` (`ComposeHost::new`) |
| `claudine compose` | the launch workspace | `claudine/cli/src/commands/compose/prep.rs` |
| `claudine sequence` root events | `source_context.base_dir()` | `claudine/cli/src/commands/wrap/sequence/mod.rs` |
| Claudine loop runs | the run's `child_cwd` | `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs` |

So the same document may write a file under one host and be refused under
another. Under `md compose`, a prompt kept in a `prompts/` folder cannot
write anywhere else in the repository it is run from:

```text
~/proj/                       ← caller's working directory (a git repo)
~/proj/prompts/new-spec.md    ← initialize: - edit: "{{ ctx.cwd }}/specs/auth.md"
```

`claudine compose prompts/new-spec.md` creates `~/proj/specs/auth.md`;
`md compose prompts/new-spec.md` refuses it as outside the mutation root.

The root also doubles as the base for relative names: the effect engine
joins a relative path onto it (`darkmatter/lib/src/effects/verbs.rs`), and
`edit` creates a missing relative name under it
(`lifecycle/executor.rs::resolve_edit_target`), while an **existing** file
is found relative to the document, like the document's other references.
Whenever the root is not the document's directory, `edit: notes.md` opens
`prompts/notes.md` if it exists but creates `notes.md` at the root if it
does not.

The review of `2026-11-14-edit-action` surfaced this; the behavior predates
that feature (`md compose` has used the document's directory since the
lifecycle moved into Darkmatter, 2026-10-21).

## Author rulings (2026-11-14)

- **The mutation root is the same for every host** (`md compose`,
  `claudine compose`, `inline-compose`, `sequence`, loop runs):
  - when the caller's working directory is inside a git repository, the
    root is that repository's root (the working tree's top level; a linked
    worktree's own root; a monorepo's root, never a package's);
  - otherwise, the caller's working directory.
- **The caller's repository decides, not the document's.** A shared prompt
  stored in another repository writes into the project the caller is working
  in.
- **A relative target resolves from the document's directory**, for reading
  and for creating alike, and must still lie inside the root. In the example
  above, `edit: notes.md` means `~/proj/prompts/notes.md` whether or not it
  exists yet. The root is only the boundary.

## Requirements

1. One function decides the root from the caller's working directory, used
   by every host; no host computes its own. Reuse the repository root the
   hosts already resolve (`FileResolutionContext::repository_root()`, derived
   from the caller's working directory; confirm it is CWD-based and not
   document-based) or `sniff::filesystem::git::repo_root`; do not add a third
   discovery.
2. The effect engine separates the **boundary** (the root) from the **base
   for relative paths** (the document's directory). Every file effect and
   `edit` resolve relative targets from the base and refuse anything outside
   the boundary, including through a symlink, exactly as today.
3. A document outside the root (a shared prompt in another repository) still
   resolves its relative targets from its own directory, so a relative target
   there is refused as outside the root; an absolute or `{{ ctx.cwd }}`-based
   target inside the root works.
4. Behavior that does not depend on the root is unchanged: shell approval,
   host allowlists, URL refusal, `create_new`, symlink refusal.

## Verification

- Both hosts, same fixtures: a prompt in a subdirectory of a git repo
  creates and edits a target elsewhere in that repo; a target outside the
  repo is refused before anything is created.
- Outside a repo: the root is the caller's working directory.
- A linked worktree: the root is the worktree's own top level.
- A monorepo package directory as the caller's working directory: the root is
  the monorepo root.
- `edit: notes.md` and `ensure_file: ["notes.md", …]` in `prompts/doc.md`
  create `prompts/notes.md`; with `prompts/notes.md` present, `edit` opens it.
- A shared prompt from outside the caller's repo: an absolute target inside
  the caller's repo works; a relative target is refused as outside the root.
- The existing escape tests (symlink, `..`) still refuse.

## Documentation

- `darkmatter/docs/topics/lifecycle/index.md` (the `edit` "mutation root"
  rules), `darkmatter/docs/topics/side-effects.md` (Safety Boundaries),
  Claudine's `docs/topics/flow-control/lifecycle.md` (`edit` and file
  effects): one shared statement of the rule with the example above.
- The darkmatter and claudine skills where they describe the mutation root.

## Out of scope

- Making the root configurable.
- Read access: references still resolve under the existing file-reference
  rules.
