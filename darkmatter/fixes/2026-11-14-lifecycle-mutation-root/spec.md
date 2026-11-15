---
kind: fix
created: 2026-11-14
status: finalized-spec
area: darkmatter
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-14-edit-action
depends-on:
    - 2026-11-14-absent-equals-null
next_spec: 2026-11-14-file-interpolate-option
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
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-14
review_iterations: 0
needs_rulings: false
---

# One mutation root for every lifecycle host

## Why this exists

A lifecycle host is a command that runs a document's event actions, such as
`initialize` or `finalize`. Every target of a file-writing action must lie
inside that host's **mutation root**: the directory within which those
actions may write. Today, the hosts choose different roots:

| Host or execution path | Root today | Source |
|---|---|---|
| Darkmatter's `md compose` | The composed document's directory | [CLI lifecycle host](../../cli/src/commands/compose/lifecycle.rs) |
| Claudine's `compose` and `inline-compose` preparation | The launch workspace's child working directory | [Composition preparation](../../../claudine/cli/src/commands/compose/prep.rs) |
| Claudine's wrapper composition pipeline | The provider child's working directory | [Composition pipeline](../../../claudine/cli/src/commands/wrap/composition/pipeline.rs) |
| Claudine's early `initialize` execution | The launch workspace's child working directory, passed by preparation | [Staged initialization](../../../claudine/cli/src/commands/wrap/composition/staged_boot.rs) |
| Claudine's root sequence events | The sequence source context's tree root | [Sequence entry point](../../../claudine/cli/src/commands/wrap/sequence/mod.rs) |
| Claudine's internal task stacks | The task's authoring directory | [Task execution](../../../claudine/cli/src/commands/wrap/sequence/task_run.rs) |
| Claudine's loop runs | The run's child working directory | [Loop execution](../../../claudine/cli/src/commands/wrap/harness_orch/loop_control.rs) |

So the same document may write a file under one host and be refused under
another. Under `md compose`, a prompt kept in a `prompts/` folder cannot
write elsewhere in the caller's repository:

```text
~/proj/                       ← caller's working directory (a Git repository)
~/proj/prompts/new-spec.md     ← composed document
~/proj/specs/auth.md           ← intended target
```

For this action in `new-spec.md`:

```yaml
initialize:
    - ensure_file: ["{{ ctx.cwd }}/specs/auth.md", "# Authentication"]
```

`claudine compose prompts/new-spec.md` creates `~/proj/specs/auth.md`;
`md compose prompts/new-spec.md` refuses it as outside the mutation root.

The root also doubles as the base for some relative targets. Darkmatter's
[effect engine](../../lib/src/effects/verbs.rs) joins raw relative paths onto
it. Darkmatter's lifecycle
[`mutation_target`](../../lib/src/lifecycle/executor.rs), which resolves
file-action and editor targets, first searches for an existing file from
the document's context, but creates a missing bare relative name under the
mutation root. Thus `edit: notes.md` can open `prompts/notes.md` when it
exists and create a different `notes.md` at the root when it does not.
Frontmatter writers currently bypass that resolver altogether.

The review of `2026-11-14-edit-action` surfaced this inconsistency. The
host-dependent boundary predates that feature.

## Author rulings (2026-11-14)

- **The mutation root is the same for every host** (`md compose`,
  `claudine compose`, `inline-compose`, `sequence`, and loop runs):
  - when the caller's working directory is inside a Git working tree, the
    root is that working tree's top level: a linked worktree's own root,
    and a monorepo's root rather than a package directory;
  - otherwise, the root is the caller's working directory.
- **The caller's repository decides, not the document's.** A shared prompt
  stored in another repository writes into the project the caller is working
  in.
- **Targets resolve by the unchanged file-reference contract; the mutation
  root is only a boundary.** An existing target is the contract's lookup
  result; a missing one is created at the contract's first candidate, which
  for a relative path is beside the document. The result must lie inside the
  mutation root. In the example above, `edit: notes.md` opens
  `~/proj/prompts/notes.md` if it exists, otherwise an existing
  `~/proj/notes.md`, and otherwise creates `~/proj/prompts/notes.md`.

## Required behavior

### Capture the boundary once

The caller's working directory means the directory captured when the
outer command starts. Keep the same boundary across initialization,
recomposition, sequence members and containers, parallel groups, loop
iterations, retries, resumed attempts, and `prep` or `proxy` documents.
Changing the active document changes the relative-path base; it does not
change the caller's write boundary. A provider child's working directory
must not become a new source of authority for writes.

Reuse biscuit-file's existing
[`LaunchMagicScope::local_root`](../../../biscuit-file/lib/src/file_reference/context.rs),
which selects the captured launch repository root or, outside a repository,
the captured launch directory. Hosts obtain that scope from the request's
[`FileResolutionContext::launch_magic_scope`](../../../biscuit-file/lib/src/file_reference/context.rs).
This scope is retained when a document context is rebuilt for a prompt in
another repository. Every lifecycle engine construction uses this selector;
there is no need for another root-selection function or repository scan.

> **Reader's note — source and launch repositories differ.** The original
> draft proposed using the document context's repository root. In both
> Darkmatter and Claudine, that value can describe the shared prompt's
> repository. The retained launch scope describes the caller's repository
> and already implements the author ruling, so it is the boundary authority.

Darkmatter's
[`MdRequest::launch_context`](../../cli/src/request.rs) owns the CLI's launch
capture. Claudine's
[`InvocationContext`](../../../claudine/lib/src/invocation_context.rs) owns
its launch capture and preserves the launch scope in derived source
contexts. Reuse those observations. A discovery failure must be reported
before file actions run; only a successful observation of no working tree
uses the launch-directory fallback. Do not turn a discovery failure into
permission to write under a guessed root.

### Resolve the target separately

Use one document-aware resolver in Darkmatter's lifecycle executor for
`edit` and every file-writing effect. Keep the following order:

1. Evaluate and bind the action's operands under existing lifecycle rules.
2. Parse the file operand through biscuit-file's
   [`FileReference`](../../../biscuit-file/lib/src/file_reference/mod.rs),
   using the active authoring document's captured context. Refuse a URL
   before resolution can fetch anything.
3. Select a local absolute target according to the table below.
4. Check that target against the mutation root before creating parent
   directories, loading a document for mutation, or invoking the editor.
   Keep the guarded write and editor rechecks as well.
5. Pass that same resolved target to the operation and, when applicable,
   the live-frontmatter update. Do not resolve the operand again from a
   different base.

**Target selection follows the file-reference contract (author ruling,
2026-11-14).** The file-reference resolution contract (`biscuit-file`,
`docs/topics/file-references.md`) does not change. Every lifecycle file
operation selects its target in two steps:

1. **Resolve the operand with the normal contract.** An existing file is
   the contract's lookup result (for an implicit relative path: the
   document's directory, then the repository root; for an explicit relative
   path: the document's directory only; for a prefixed reference: that
   prefix's roots). A missing file is created at the contract's first
   candidate (`candidate_plan`), never at the mutation root joined with the
   raw operand.
2. **Check the result is inside the mutation root.** A result outside it is
   refused before anything is opened, created, or written; never skip an
   outside result to look for another candidate.

Example: prompt `~/proj/prompts/new-spec.md`, run from `~/proj`, where
`~/proj/specs/auth.md` exists:

| Operation in `initialize` | Result |
|---|---|
| `edit: specs/auth.md` | opens the existing `~/proj/specs/auth.md` (second candidate) |
| `ensure_file: ["specs/auth.md", "# Auth"]` | same file; exists, so nothing is written |
| `append_line: ["specs/auth.md", "reviewed"]` | appends to `~/proj/specs/auth.md` |
| `ensure_file: ["specs/new.md", "# New"]` | creates `~/proj/prompts/specs/new.md` (first candidate) |
| `edit: "~/notes.md"` | refused: outside the mutation root |

Today's code creates a missing implicit relative target at the mutation
root (`darkmatter/lib/src/lifecycle/executor.rs`, `mutation_target`:
`mutation_root().join(raw)`); this fix replaces that with step 1. Two
operand kinds have no file to resolve: a recursive `%` reference with no
match is refused (a search directory is never used as a new filename), and
a URL is refused without fetching or writing.

An environment-expanded file
reference follows its effective anchoring: if its expanded value is absolute,
it is an absolute target. Keep captured environment, home, prefix, and typed
reference-error behavior; do not implement another reference parser.

For file-backed actions, the authoring document supplies the base. A
referenced sequence or group uses its own document; an inline group or task
uses the document that contains it. Input without a backing file, including
stdin and inline composition text, uses its captured authoring directory
(the launch directory for top-level input), not a temporary file's location.

> **Reader's note — the same file for reads and writes.** Because writes
> use the file-reference contract unchanged, an implicit relative target such
> as `notes.md` names the same file whether it is read or written, including
> the contract's fallback to the repository root. If the file beside the
> document is removed, a later `edit: notes.md` opens a repository-root
> `notes.md` when one exists, exactly as a read would. Write `./notes.md` to
> pin the document's directory for both.

A shared prompt outside the caller's root therefore cannot mutate a relative
target beside itself. An absolute target inside the caller's root
works. Prefixed references retain their existing meanings: for example,
`&` still names the document's repository, while `@` retains its launch-based
search roots. This fix does not reinterpret either prefix as a write grant.

### Cover every file-writing action

Route all seven frontmatter writers through the same target resolver:
`set_frontmatter`, `merge_frontmatter`, `delete_frontmatter`,
`increment_frontmatter`, `decrement_frontmatter`, `append_frontmatter`, and
`prepend_frontmatter`. Apply the same rule to both `ensure_file` overloads,
`ensure_dir`, `append_line`, `append_jsonl`, and `edit`.

Darkmatter's
[`EffectEngine`](../../lib/src/effects/mod.rs) remains the low-level boundary
enforcer. Lifecycle calls supply absolute resolved targets, so the engine
need not store a mutable document base shared by unrelated or parallel
stacks. Direct low-level callers retain the engine's existing explicit-root
and root-relative API; document-reference resolution belongs to the lifecycle
executor. Engine reads that prepare a mutation must check containment before
loading the target, as well as guarding the eventual write.

Darkmatter's
[`mirror_frontmatter_mutation` and `targets_document`](../../lib/src/lifecycle/executor.rs)
keep later actions' in-memory frontmatter consistent with successful writes
to the active document. Update them to use the already resolved target's
identity rather than joining the authored text to the mutation root. Compare
paths through biscuit-file's
[`PathIdentity`](../../../biscuit-file/lib/src/file_reference/portable/path_identity.rs),
with the existing canonical-path rules where filesystem identity is needed.
A mutation of another file must not update the active document's state.
Preserve the host's read-cache invalidation after an attempted effect or edit,
including an attempt that writes before failing.

Every execution path in the opening inventory is in scope, including
internal task stacks still reachable through library adapters. All events
use the rule, including early initialization and failure/finalization paths.
Do not add a separate Claudine resolver or a document-directory exception
for sequence initialization.

### Preserve the existing safety and failure contracts

This intentionally broadens `md compose` writes from the document directory
to the caller's working tree, while restricting sequence/task writes that
previously used an external source directory. Authors who relied on a missing
bare filename being created at the root must use an absolute caller-based
path instead. Do not retain a compatibility branch whose choice depends on
whether the file exists.

Preserve canonical containment for existing targets and missing tails, so a
symlink escape or a sibling path with the same text prefix is refused.
Reuse Darkmatter's [guarded file-write helpers](../../lib/src/effects/fs_write.rs)
and editor checks; editor-specific refusal of nonregular entries and
symlinks remains unchanged. Use biscuit-file's
[`canonicalize_simplified`](../../../biscuit-file/lib/src/lib.rs) when a
canonical path leaves a private comparison. Paths must work on macOS,
Linux, native Windows, and WSL2; string prefix checks are not containment.

Keep existing located diagnostics and failure classification. Path-resolution
and containment failures remain action dispatch failures, with the action's
existing `no_error` behavior; malformed operands, evaluation failures, and
cancellation retain their existing treatment. An outside target must not
create its parents or launch an editor. An outside-root error identifies the
target and effective root so the author can correct the path.

Shell approval, network allowlists, exclusive `create_new` behavior,
frontmatter hashing and literal-value storage, event ordering, and recovery
semantics are unchanged. The root limits lifecycle-managed file operations;
it does not sandbox arbitrary writes performed by an approved shell command
or the configured editor.

## Verification

Use deterministic library tests and isolated CLI fixtures. Editor assertions
use the existing fake editor service or fixture executable; they never open
a real user's editor or bring a terminal or browser window into focus.

- **Boundary selection:** inside and outside a repository; launch from a
  monorepo package directory; launch inside a linked worktree. The worktree
  case uses its own top level, never the main checkout. A captured discovery
  failure is reported rather than treated as an absent repository.
- **Host wiring:** exercise `md compose`, Claudine compose and inline
  composition, wrapper composition, root and nested sequence/group events,
  task adapters, staged initialization, and loop events through their
  existing fixture seams. All select the same caller boundary, including
  when the prompt lives in another repository. Use a shared set of path
  expectations rather than copying the resolution implementation into tests.
- **Relative targets follow the contract:** a prompt at `prompts/doc.md`
  with no `notes.md` anywhere creates and edits `prompts/notes.md`. With
  only root-level `notes.md` present, `edit: notes.md` and
  `append_line: ["notes.md", …]` act on the root-level file (the contract's
  second candidate), while `./notes.md` creates `prompts/notes.md`. Cover
  implicit, explicit, and nested missing-parent forms, stdin, and inline
  input.
- **Every writer:** table-driven coverage includes all seven frontmatter
  writers, both `ensure_file` overloads, directory creation, both append
  verbs, and `edit`. A successful frontmatter write to the active document
  is visible to the next action; writing another file does not change that
  state. Subsequent file reads observe successful file effects and edits.
- **Caller and source differ:** an external shared prompt can write an
  absolute target inside the caller's root, but cannot write beside itself
  through a relative target. Also cover launch outside a repository
  with a prompt inside one: the caller's launch directory still wins.
- **Reference semantics:** cover supported prefixed targets inside and
  outside the root, a missing nonrecursive target, effective absolute
  environment expansion, an unmatched recursive reference, and a URL that
  makes zero fetch attempts.
- **Boundary enforcement:** an in-root `../` target succeeds; an escaping
  `..`, sibling-prefix path, or symlink escape fails before any file or
  parent is created. Keep existing editor symlink and `create_new` tests.
  Use platform-appropriate path fixtures and the existing Windows link
  capabilities rather than treating Unix-only tests as Windows evidence.
- **Lifetime:** change source documents through `prep`/`proxy`, retry,
  resume, and loop progression; the boundary stays fixed and the authoring
  base follows the active document. Parallel members with different source
  directories do not share a mutable path base.
- **Passive surfaces:** schema validation, language-server requests, and
  composition without a lifecycle host dispatch no mutations or editor.

Run the affected Darkmatter and Claudine L1 suites with their package-area
`just test` recipes. Existing real-terminal tests, if affected, run only
through `just test-l2` and the repository's focus-preserving harness. This
fix needs no performance spike: it reuses captured launch facts and existing
target-resolution and write checks rather than adding a discovery pass.

## Documentation and dependency

Implementation follows `2026-11-14-absent-equals-null`. That dependency owns
null/default handling and the related editor-operand review changes; this
fix changes target selection and containment, not operand coercion.

Update these current behavior documents with the boundary/base distinction,
the subdirectory example, the shared-prompt case, and migration away from
root-relative creation:

- [Darkmatter lifecycle](../../docs/topics/lifecycle/index.md), especially
  `edit` target creation and the rules common to all file actions.
- [Darkmatter side effects](../../docs/topics/side-effects.md), including the
  low-level engine versus document-aware lifecycle distinction.
- [Claudine lifecycle](../../../claudine/docs/topics/flow-control/lifecycle.md),
  removing the sequence-root exception.
- [Claudine side effects](../../../claudine/docs/topics/state-management/side-effects.md),
  including containment through filesystem aliases rather than only lexical
  checks.

Update the Darkmatter and Claudine repo skills where they describe mutation
roots or lifecycle target resolution. Update affected caller-skill sources
under each area's `docs/use-skills/`, then run that area's
`just publish-skills`; never edit installed caller skills directly. Keep
README changes limited to any overview examples that become incorrect.
Current behavior docs state the rule directly and do not link back to this
specification.

## Open Questions

None. The existing author rulings determine the boundary and relative-target
semantics; the implementation requirements above make their consequences
explicit.

## Out of scope

- Making the lifecycle root configurable or granting writes outside it.
- Changing ordinary read access or file-reference search rules outside
  lifecycle mutation targets.
- Changing provider working directories, shell execution policy, or the
  editor's process and cancellation behavior.
- Redesigning the low-level effect engine's standalone relative-path API or
  providing a new filesystem sandbox.
