---
area: darkmatter
status: implemented
created: 2026-11-14
owner: Ken Snyder <ken@ken.net>
canonical_docs:
    - darkmatter/docs/topics/lifecycle/index.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/schemas.md
related:
    - 2026-04-17-edit-command
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            spec-reviewed,
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
reviewed_on: 2026-11-13
review_iterations: 0
needs_rulings: false
implemented: true
implemented_by: claude/opus
---

# Edit Action

## Problem

Claudine's wrapper `--edit` flag opens a temporary Markdown file in the caller's
editor and uses its contents as the prompt. Darkmatter already exposes the
shared editor resolution and blocking launch implementation. Lifecycle actions
cannot currently use that functionality to open a named file.

A creation prompt should be able to compute a spec path, refuse to overwrite an
existing spec, and open the new file for the caller to write. Today, an authored
`edit: fqn` does not invoke an editor: `edit` is not registered as a lifecycle
action, and the bare operand would be literal text rather than a property lookup.

## Outcome

Add a shared Darkmatter lifecycle action, available to Claudine, that opens a
named file in the caller's editor and waits for the editor to finish. It reuses the editor behavior behind
`claudine claude --edit`, while editing the requested path in place.

The motivating workflow is:

```yaml
fqn: "{{ ctx.cwd }}/features/2026-11-14-example/spec.md"
initialize:
    - when: file_exists(fqn)
      then:
        - warn: "The spec already exists: {{ fqn }}"
        - skip
      else:
        - edit:
            file: "{{ fqn }}"
            create_new: true
        - info: "The spec is ready at {{ fqn }}"
        - skip
```

`skip` retains its existing meaning: skip the document during `initialize`,
without launching a provider. `edit` itself does not stop an event, skip a
document, compose the edited file, or launch an agent.

The example makes the path absolute using the captured launch directory so
the existence check and edit address the same file even when the prompt lives
in a separate prompt library. Its existence check is a convenience that gives
the friendly warning branch, not an atomic reservation: another process can
create the file after the check. `create_new: true` is the guarantee: if the
name is occupied by the time the action creates the file, the action fails
and the editor never opens.

## Authoring Contract

Support both forms:

```yaml
- edit: "{{ fqn }}"
- edit:
    file: "{{ fqn }}"
    create_new: true
    gui: false
    no_error: true
```

The ordinary one-argument list spelling, `edit: ["{{ fqn }}"]`, is equivalent
to the scalar short form. More than one positional argument is an error. A
bare `edit`, `edit: null`, and `edit: []` supply no argument and are errors.

The long form requires `file` and accepts three optional options, each a
literal YAML boolean:

- `create_new` requires the action to create the file. With `true`, the
  exclusive creation in [File and Editor Behavior](#file-and-editor-behavior)
  must succeed; if any entry (file, directory, symlink, or other) already
  occupies the name, the action fails before launching an editor and never
  opens or touches that entry. Missing parent directories are still created.
  The failure is an ordinary execution failure, so `no_error` may suppress it
  and the stack continues without an editor having opened. With `false`, the
  default, ordinary editing applies: an existing regular file is opened, a
  missing one created. Exclusive creation reserves the name only; it does not
  lock the file against later edits by other processes.
- `gui` constrains which kind of editor may be selected (see
  [Editor Selection](#editor-selection)). `false` requires a terminal editor,
  `true` requires a GUI editor, and omitting it accepts either under the
  ordinary resolution rules. The short forms are equivalent to omitting it.
- `no_error` is the existing action option, defaulting to `false`.

Reject unknown fields, missing
operands, empty or whitespace-only paths, and non-string resolved operands
with the normal located lifecycle diagnostics. Preserve whitespace in a
nonblank path. This action deliberately requires a string: do not silently
coerce a number or boolean into a filename, even though some ordinary
side-effect parameters permit scalar-to-string conversion. Passive tooling
and runtime execution must enforce the same rule.

`no_error` suppresses execution failures according to existing action semantics;
it does not suppress parsing, expression, or operand-type errors, or turn
cancellation into success. The ordinary sibling `no_error` on a compact
dictionary also applies; duplicate inner and sibling flags remain errors.
For `edit`, the execution failures are: no matching editor, an ineligible
host for the selected editor, filesystem refusal, an occupied name under
`create_new: true`, launch failure, a nonzero
editor exit, and a deleted or nonregular target after exit. Because a
nonzero exit (Vim's `:cq`, for example) stops the stack by default, quitting
the editor with an error is the caller's way to abort the rest of the stack;
`no_error: true` removes that gesture and suits optional edits only. A
non-boolean `gui` or `create_new` is an operand-type error and is never
suppressed.

Resolve the operand once using the existing lifecycle interpolation rules.
`edit: fqn` opens a file literally named `fqn`; `edit: "{{ fqn }}"` opens the
computed path. File contents are user data and are never evaluated as expressions
merely because the action opened them.

The action is recognized in the `md compose` host and all Claudine lifecycle
event stacks, including `initialize`, and inside supported conditional
branches. Recognition does not guarantee that the executing host has an
interactive terminal; execution eligibility is defined below. It runs synchronously
in normal list order, returns to the next action on success, and follows existing
dictionary ordering when authored alongside other verbs. It is an editor action,
so the prohibition on lifecycle `shell` during `initialize` does not prohibit it.
Sequence and group stacks use the same registered action and execution contract.
Classify `edit` as an `expression` action, the existing category for
non-shell side effects. In a compact dictionary it runs with the other such
verbs in ascending name order, after communication and `shell`, before flow
control. Prefer separate list items when a message must follow the edit.

This is a lifecycle-only operation. Do not introduce an effectful `edit()`
function into body expressions, interpolation, schema slots, or passive
validation merely to register the action.

## Editor Selection

Whether an editor runs in the terminal or in its own window decides which
hosts can run the action, how its output is routed, and what cancellation can
stop. That fact belongs to Sniff, which already catalogs editors, rather than
to a table inside Darkmatter.

### Sniff owns editor surface and blocking metadata

Sniff's `Editor` catalog does not currently record whether an editor is a
terminal or a GUI program, and the only blocking knowledge lives in
Darkmatter's `wait_args_for_editor`, keyed by a raw command token. Add both
facts to Sniff's `Editor` metadata:

- **surface:** `Terminal`, `Gui`, or `Both`. `Both` covers editors whose
  surface depends on the environment or flags (Emacs opens a window when a
  display exists and runs in the terminal with `-nw`).
- **wait flags:** the arguments that make a GUI editor block until the file
  is closed (`--wait` for VS Code, Zed, Sublime, TextMate, BBEdit, and the
  JetBrains IDEs; `--block` for Kate). A terminal editor blocks inherently and
  needs none. A GUI editor with no verified wait flag is **non-blocking**:
  launching it returns before the caller edits anything. Verify each GUI
  editor's flag when the table is built; do not carry Darkmatter's current
  table across unverified.

Sniff also identifies an editor from a configured command token. The token
is matched by file stem, so `/usr/local/bin/code`, `code`, and on Windows
`code.cmd` or `Code.exe` (case-insensitively) all identify VS Code; the
editor's alternate binary names match too. Today `EDITOR=/usr/local/bin/code`
receives no `--wait`, because the raw token is compared with `"code"`.
Darkmatter's editor module consumes this metadata and keeps no second table;
`wait_args_for_editor` either delegates to Sniff or is removed with its
callers moved to the Sniff lookup.

| Surface | Examples |
|---|---|
| Terminal | `vi`, `vim`, `nvim`, `nano`, `hx`, `micro`, `kak`, `amp` |
| GUI, blocking | `code`, `codium`, `subl`, `zed`, `mate`, `bbedit`, `kate`, JetBrains IDEs |
| GUI, non-blocking until verified otherwise | `geany`, `lapce` |
| Both | `emacs`, `xemacs` |
| Unknown | any command Sniff does not catalog, such as a wrapper script |

### Resolution

Resolution keeps the existing precedence: `$EDITOR`, then `$VISUAL`, then
the built-in probe list. A candidate is eligible only when it can block: a
terminal editor, a GUI editor with a wait flag, or an unknown command (which
is trusted to block, as today). This applies to every caller of the shared
resolver, including wrapper `--edit` and `md edit`:

- a configured variable naming a known non-blocking editor is skipped like a
  variable naming a missing program; if no later variable resolves, the error
  names it and says it cannot wait;
- the probe list never selects a non-blocking editor.

With `gui` set, resolution walks the same order and also skips any candidate
whose surface does not match. `Both` and unknown candidates never match an
explicit `gui`, because the action cannot prove which surface they will use.
Probes run even when a variable was set, because the author explicitly asked
for a surface the configured editors do not provide. If nothing matches, the
action fails with a typed no-matching-editor error that names the requested
surface and every candidate checked with the reason it was rejected.

```text
EDITOR=nvim  VISUAL=code
edit: { file: notes.md }              -> nvim (ordinary precedence)
edit: { file: notes.md, gui: true }   -> code --wait notes.md
edit: { file: notes.md, gui: false }  -> nvim notes.md

EDITOR=emacs  (no VISUAL)
edit: { file: notes.md, gui: false }  -> first terminal probe, e.g. nvim
```

Without `gui`, resolution returns the same editor it returns today, except
that a non-blocking editor is no longer selected.

### Host eligibility follows the selected surface

The selected editor's surface, not the `gui` option, decides what the host
must provide. With `gui` omitted, the surface of whatever resolves applies.

| | Terminal or unknown editor | GUI editor |
|---|---|---|
| Interactive terminal | required: the exclusive handoff below | not required |
| Display session | not required | required |
| stdin | the host terminal | null |
| stdout and stderr | the host's terminal (stderr under `md compose`) | discarded under data output; the host's stderr otherwise |
| `md compose` data output | refused | allowed |
| Parallel group member | refused | allowed |
| Cancellation | the whole editing session is owned | only the waiting client is owned |

A display session is detected through a new focused Sniff observation (Sniff
reads neither `DISPLAY` nor `WAYLAND_DISPLAY` today): on Linux and WSL2, `DISPLAY` or `WAYLAND_DISPLAY` is set;
macOS and native Windows assume an interactive desktop session. A `Both`
editor selected without `gui` is treated as a terminal editor for
eligibility, the conservative choice.

## File and Editor Behavior

1. Parse the resolved string with biscuit-file's
   [FileReference](../../../biscuit-file/lib/src/file_reference/mod.rs), the
   shared file-reference parser. Reuse the lifecycle mutation resolver with
   the captured request context and source document. Existing targets follow
   document resolution precedence; a missing bare relative path is created
   under the host's mutation root, the directory within which writes are
   permitted. Other local reference forms use their existing candidate rules.
   Accept the local forms already accepted by `ensure_file`, including
   relative, absolute, and prefixed references; reject URLs without a fetch.
   Every selected path must also pass mutation-root containment, so an absolute
   path is not permission to edit outside that root. Never recapture process CWD.
2. Refuse in cancellation cleanup or after cancellation is requested; do not
   start another blocking editor while trying to finish an interrupted run.
3. Resolve the editor before creating the target, as described in
   [Editor Selection](#editor-selection): the existing precedence, configured
   arguments, and fallback discovery, filtered by blocking ability and `gui`.
   Do not introduce a second editor configuration. Resolution reads only the
   environment and the program index; it creates nothing. Retain the resolved
   command for launch so discovery happens once per reached action, rather
   than resolving again after creating the file.
4. Check host eligibility for the selected surface. A terminal editor needs
   wrapper `--edit`'s existing minimum: stdin and stdout must both be
   terminals, and stdin must not be reserved for reading the prompt or
   document. The host must also grant this stack exclusive use of its
   interactive surface; a parallel group member has no such surface, even if
   the parent process has a terminal, and serial stacks may edit when the host
   grants it. A GUI editor needs a display session instead. These checks, and
   resolution, precede all directory and file creation.
5. Preserve an existing regular file's contents before opening it. For a missing
   file, create its parent directories and use exclusive creation for the empty
   file, never truncating or replacing an entry that appears concurrently.
   Under `create_new: true`, an existing target, or a failed exclusive create
   because another process created the name first, fails the action here.
   Otherwise, if another process creates a regular file first, recheck the
   target and policy, then open that file under the ordinary edit-existing
   contract; do not retry with an overwriting write. A directory or other
   nonregular target is an error. Honor shared symlink and root-escape rules before creation and launch.
   Release creation handles before launching, so Windows editors can replace
   the file during save. Do not call `ensure_file` directly for creation: its
   current existence-check followed by replacement does not reserve a missing
   name atomically.
6. Launch the editor on the resolved path as a distinct process argument, using
   the shared Darkmatter
   [launch_editor_on_path](../../lib/src/editor/mod.rs) machinery, which launches
   the selected editor and waits. Extract a reusable prepared launch operation
   as needed for one-time discovery and cancellation. Paths with
   spaces or shell metacharacters must reach the editor unchanged. Do not build a
   shell command to launch it. Pass an absolute native path so a name beginning
   with a dash cannot be mistaken for an editor option.
7. Wait for the editor to exit. Zero exit with the target still a regular file
   means success, including an unchanged or empty file. Nonzero exit, launch
   failure, or a deleted or nonregular target means action failure. Recheck
   containment after exit as well. Allow normal editor saves that replace the
   file; do not require the original file identity to survive. Do not read or
   require UTF-8 content merely to report success.
8. Preserve the caller's saved bytes exactly: no trimming, composition,
   frontmatter rewriting, or automatic hashing. Do not roll back saved edits or
   remove newly created files when an action fails or the run is canceled.

> **Reader's note — path resolution:** The draft's instruction to avoid the
> prompt's storage directory conflicted with its requirement to reuse
> `ensure_file` resolution. Existing files are looked up from the authoring
> document before a missing bare path falls back to the mutation root. This
> review preserves that shared contract; use one absolute target, as in the
> example, when an existence check and creation must agree across prompt libraries.

Editor availability, launch, exit-status, filesystem, and noninteractive errors
must identify the action, authored property location, and target through the
executing host's existing error routing, in both Darkmatter and Claudine.
Keep typed causes and file-reference failure classes rather than flattening
them into generic error text. A filesystem/reference refusal after successful
operand evaluation is a dispatch failure, as for existing mutation actions.
Subsequent actions do not run after an unsuppressed failure. Cancellation is
described in [Cancellation](#cancellation); a canceled edit never resumes the
normal stack, even with `no_error`.

## Cancellation

A run can be canceled while the editor is open: a sequence `timeout`, a
sibling's failure in a group, or the host's own cancellation. The action stops
only the process work it started and never touches an editor that was already
running.

```yaml
timeout: 30m
initialize:
    - edit: { file: "{{ fqn }}", create_new: true }
    - info: "ready"     # never runs if the timeout fires mid-edit
```

1. **Wait cancellably.** The shared launcher today waits with
   `Command::status()`, which cannot be interrupted. Extract a prepared launch
   operation that spawns the editor and waits in a loop that also observes the
   host's cancellation signal. Check the signal again before advancing to the
   next action.
2. **Own a process tree.** Start the editor as the root of a tree the action
   can stop as a unit: its own process group on Unix (`process_group(0)`), and
   a Job object on Windows, assigned immediately after spawn. A child started
   in the instant before assignment can escape the job; that window is
   accepted and documented in code. A terminal editor's group is made the
   terminal's foreground process group (`tcsetpgrp`, with the host ignoring
   `SIGTTOU` while it does so) so the editor can read input, and the host's
   group is restored afterwards. While a terminal editor is in the
   foreground, Ctrl-C reaches the editor, not the host.
3. **Terminate and reap.** On cancellation, send `SIGTERM` to the group, then
   `SIGKILL` after a fixed two-second grace period; on Windows, terminate the
   job. Wait for the root to exit, then restore the host's terminal state and
   foreground group before cancellation cleanup runs.
4. **Keep what was saved.** Never roll back saved bytes or remove a file the
   action created. Unsaved buffer contents in a terminal editor are lost, and
   the editor may leave a recovery file (for example Vim's `.swp`); the action
   does not clean those up. The run reports as canceled.
5. **GUI editors.** Only the waiting client (for example `code --wait`) is
   owned. Terminating it ends the wait, but the editor window may stay open
   with the file, and the user can still save after the run ends. The
   canonical documentation says so. `gui: false` is the way to get an editing
   session that cancellation fully stops.

Wrapper `--edit` and `md edit` keep their behavior on the same launcher; they
have no cancellation signal, so the loop waits until the editor exits.


Notify the host that a side effect was attempted after any creation or editor
launch, including a failed attempt that could have changed disk. This uses
Darkmatter's existing
[LifecycleHost::side_effect_attempted](../../lib/src/lifecycle/executor/host.rs)
hook, which lets hosts discard cached file reads. A later `file_exists` or
file-reading expression must observe the saved bytes or deletion, rather than
a value cached before the edit. The action does not add the edited file to
frontmatter or return its contents as a runtime value.

Editing does not implicitly recompute already evaluated frontmatter or predicates.
Existing document reread boundaries remain authoritative when the edited target
is also an active prompt document.

Mutation-root checks govern the path supplied to the editor; they do not
sandbox an arbitrary user-configured editor, its plugins, or other files the
caller chooses to open. Reuse the repository's existing containment model;
do not claim protection from every concurrent filesystem substitution.

## Terminal and Output Ownership

A terminal editor needs a terminal for input and screen updates. The host
lends it that terminal while the action blocks, then restores its own terminal state
before another action or cancellation cleanup runs. Reuse existing terminal
handoff utilities where available; never start multiple editors sharing one
interactive surface.

For `md compose`, stdout holds the composed document, and data-output modes
also require an empty stderr and one structured result. Inherit neither stream
blindly for the editor. For a terminal editor in Markdown mode with an
interactive terminal, keep stdin for input and route the editor's stdout and
stderr to the host's terminal on stderr; require that stream to be a terminal
too. Under any structured data output mode, refuse a terminal editor after
resolution and before creation, through the normal diagnostic result. This
remains true when stdout happens to be a terminal. A pipe or redirected
document output also fails the stdin/stdout terminal requirement.

A GUI editor draws in its own window, so it borrows no terminal: its stdin is
null, and the waiting client's stdout and stderr go to the host's stderr in
Markdown mode and are discarded under data output, which keeps stderr empty
and the one structured result valid. Claudine applies the same principle: an
output mode reserving streams for structured results cannot lend them to a
terminal editor, and never passes a GUI client's output through them.

This preserves machine-readable output without inventing a separate terminal
device or attempting to capture an interactive editor's screen as lifecycle
text. A `no_error`-suppressed refusal leaves normal output valid. Unknown-host
capability errors follow the same rule and never launch a fallback shell.

## Integration Boundaries

- Implement and register `edit` in Darkmatter's shared lifecycle machinery,
  with the editor execution service supplied by the executing host. Claudine
  exposes the shared action through its host profile; it must not own a separate
  parser or implementation. A host without interactive editor execution support
  reports an explicit capability error when execution reaches the action.
  Supply the service to both the `md compose` host and Claudine's document,
  sequence, and serial group adapters; parallel workers explicitly deny an
  interactive terminal rather than inferring permission from process-wide
  TTYs, and may still run a GUI editor.
- Reuse the shared editor module. Any extraction needed to resolve once and
  launch later must preserve existing wrapper `--edit` and `md edit` behavior,
  except the deliberate change that a non-blocking editor is never selected.
- Add editor surface, wait flags, and stem-based command identification to
  Sniff's `Editor` catalog, with its docs and the `sniff` skill's program
  page, plus the focused display-session observation. Darkmatter's
  editor module and Claudine's wrapper `--edit` read them from Sniff.
- Expose the operand, documentation, and availability through lifecycle
  registration, schemas, the host profile, and DMLS completion, hover, and
  diagnostics. Keep the existing four action categories; no new category or
  Claudine-owned operand parser is required. Define the lifecycle-only shape
  in Darkmatter's `schemas/partials/lifecycle.yaml` and add its references to
  Claudine's action, bundle, initialization, and loop-key schema surfaces.
  Do not add it to the ordinary expression/effect catalogs solely to gain
  schema generation: that would expose an effectful expression function.
  Preserve generated regions and their catalog-equality checks, distinguishing
  this explicit lifecycle-only verb from catalog-derived verbs in those checks.
  Regenerate the host lifecycle artifact if its bytes change, and regenerate
  shipped schema artifacts with root `just shipped-schemas` after schema or
  preparation changes. Runtime and passive-tooling tests must agree on the
  action's forms and strict string requirement.
- Passive analysis, schema validation, catalog generation, Claudine `--dry-run`,
  and Darkmatter `md compose --shell` never
  launch an editor or create files/directories. They still validate syntax and
  types. A branch that is not executed never resolves an editor or touches disk.
  Unknown event-time values remain deferred during passive checks; validation
  must not evaluate them just to discover a filename. Both conditional branches
  still receive structural and statically provable type checks. `md compose`
  has no `--dry-run` flag; do not add one for this action.
- The action needs no authored-shell approval because it uses the dedicated
  editor launcher; existing filesystem policy still applies.
- Document the two forms, `create_new`, the `gui` option and surface-dependent host
  requirements, file creation, blocking behavior,
  failures, and the distinction between `edit`, wrapper `--edit`, `stop`, and
  initialize-time `skip` in the Darkmatter and Claudine canonical lifecycle documentation.
  Update their schema authoring references and overview links where needed;
  current documentation must describe behavior directly and never link back to
  this dated specification. Update the darkmatter skill's lifecycle guidance
  when the service and registration contract land.

## Acceptance Criteria and Verification

1. The motivating initialize workflow opens the computed spec path, waits for
   saved content, prints its success message, and skips provider execution.
   An existing spec takes the warning branch and never launches an editor.
   Repeat with the prompt stored outside the launch directory; the absolute
   existence probe and edit still name the same target. When the target is
   created between the existence check and the edit (a test hook creates it
   after the check), `create_new: true` fails the action without launching the
   editor or changing the file's bytes; with `no_error: true` the stack
   continues. With `create_new` omitted, the same race opens the existing
   file without replacing it. A `create_new` target occupied by a directory or
   a symlink also fails without launch.
2. Short and long forms work with literal and interpolated paths. Malformed
   shapes and unknown fields produce located diagnostics across runtime and
   passive tooling. Bare `fqn` remains literal.
3. A portable fake editor records its exact arguments and writes known bytes.
   Tests prove relative-path resolution, paths containing spaces and
   metacharacters and leading dashes, preservation of existing content,
   missing-parent creation, concurrent creation without replacement, Unicode
   paths, and byte-for-byte preservation after editing (including trailing
   whitespace, CRLF, and arbitrary non-UTF-8 file content). Test allowed local
   prefixes, URL refusal without network activity, and containment for both
   existing and missing targets, including symlink escapes under the shared
   filesystem policy.
4. The existing editor precedence remains shared and covered by editor-module
   tests; Sniff tests cover each cataloged editor's surface and wait flags and
   stem identification (full paths, Windows extensions and case, alternate
   names). Resolution tests cover `gui: true`, `gui: false`, and omitted,
   including a mismatched `$EDITOR` falling through to `$VISUAL` and then to
   probes, `Both` and unknown commands never matching an explicit `gui`, the
   typed no-matching-editor error, and a non-blocking editor never being
   selected by wrapper `--edit`, `md edit`, or the action. Action tests prove
   dispatch through that implementation without duplicating its entire editor
   matrix.
5. Noninteractive invocation, missing editor, directory target, filesystem
   refusal, launch failure, nonzero exit, deleted target, and cancellation have
   defined failures. Verify continuation with `no_error` where applicable and
   no continuation after an unsuppressed failure. Terminal, output-mode, and
   missing-editor refusals leave no new target or parent directory. Check
   nonregular targets after editor exit and ordinary atomic-save replacement.
6. Dry-run, passive editor analysis, and untaken branches produce no editor
   launch or filesystem changes, including `md compose --shell`. Invalid
   operands in untaken branches still produce passive diagnostics. Test
   execution with a process recorder rather
   than relying only on mocked action dispatch.
7. Tests cover action ordering and execution in initialize, a later document
   event, and sequence/serial-group stacks without starting a real provider.
   Parallel members fail without a launch or creation when a terminal editor
   is selected, and run a fake GUI editor (null stdin, no terminal) when one
   is selected. A Linux host without a display refuses a GUI editor before
   creation. A terminal integration
   test proves the blocking handoff, input, and return to the next action.
   Verify that a post-edit read in the same stack sees changed or deleted data,
   including after a failed edit suppressed by `no_error`.
8. The implementation works on macOS, Linux, native Windows, and WSL2. Use
   portable fake-editor fixtures for process and path tests and the repository's
   existing terminal-test policy for interactive coverage.
   Use the repository Test Toolkit and nextest-backed package-area recipes:
   `just test` for ordinary tests and `just test-l2` for terminal integration.
   Terminal tests run without focusing terminal or browser windows and use
   fake editors rather than launching a real graphical application.
9. In `md compose` Markdown mode, the fake editor's screen output never appears
   in the composed document. Under structured output, a terminal editor
   produces one valid result with the edit refusal and no editor output on
   either stream (test both ordinary failure and `no_error` continuation),
   and a fake GUI editor runs with its client output discarded, leaving stderr
   empty and one valid result. Verify host terminal restoration on
   success, failure, and cancellation.
10. Cancellation tests use a controllable fake editor that starts a child
    process and ignores `SIGTERM` in one variant. A sequence timeout and a
    sibling cancellation each terminate and reap the whole tree (escalating to
    `SIGKILL` in the ignoring variant; job termination on Windows), restore
    the terminal and foreground group, keep bytes saved before cancellation
    and a file created by the action, skip the next action even with
    `no_error`, and report the run as canceled. A fake GUI client proves only
    the client is terminated. Tests never close a real user's editor.
11. The same malformed shapes and strict operand types are rejected by runtime
    parsing, `md schema validate`, and DMLS. Freshness checks cover the shipped
    schema artifacts and any changed host profile. Library composition and
    passive tooling never launch the action.

No performance spike is needed: this action waits on human input and reuses
editor discovery. Its risk is process, file, and terminal correctness rather
than throughput.

## Author Rulings

- **2026-11-13 — editor surface option is `gui`.** `visual` was rejected
  because `$VISUAL` historically names a full-screen terminal editor, the
  opposite of the intended meaning. Host eligibility follows the selected
  editor's surface, and Sniff owns editor surface and wait-flag metadata.
- **2026-11-13 — race-safe creation is `create_new: true`.** It closes the
  check-then-act race in the motivating workflow with the exclusive creation
  the action already performs, without a second verb. A best-effort
  conditional and a separate create-and-edit action were rejected.
- **2026-11-13 — cancellation stops only the action's own process tree.**
  Waiting through cancellation was rejected because an interrupted run could
  block indefinitely; admitting only fully ownable editors was rejected because
  it breaks common `--wait` GUI workflows. `gui: false` gives authors the full
  guarantee when they need it.

## Out of Scope

- Adding `--edit` flags to composition commands.
- Temporary-buffer editing with a returned runtime value.
- Seeding a new file with a template or composing the saved file automatically.
- Selecting a different editor per action or opening files asynchronously.
- Changes to `stop`, `skip`, document reread boundaries, or shell approval rules.
- Sandboxing the configured editor, locking a file for the editing session,
  or coordinating editors across independent CLI invocations.
