---
kind: plan
created: 2026-11-13
total_phases: 7
phase: 6
agent: claude/sonnet
yolo: 'true'
source_files_during_phase_1:
- sniff/lib/src/os/display.rs
- sniff/lib/src/os/mod.rs
- sniff/lib/src/programs/enums/editor_traits.rs
- sniff/lib/src/programs/enums/mod.rs
- sniff/lib/src/programs/mod.rs
- tools/test-toolkit/Cargo.toml
- tools/test-toolkit/src/lib.rs
- tools/test-toolkit/src/fake_editor.rs
- tools/test-toolkit/src/bin/test-toolkit-fake-editor.rs
- tools/test-toolkit/tests/fake_editor.rs
- darkmatter/cli/Cargo.toml
- darkmatter/cli/tests/bin/fake_editor/main.rs
- claudine/cli/Cargo.toml
- claudine/cli/tests/bin/fake_editor/main.rs
- Cargo.lock
docs_updated_during_phase_1:
- sniff/docs/sniff-library-architecture.md
- tools/test-toolkit/README.md
- docs/dependencies.md
- darkmatter/docs/dependencies.md
- claudine/docs/dependencies.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
- .claude/skills/sniff/SKILL.md
- .claude/skills/sniff/programs.md
- .claude/skills/darkmatter/testing.md
packages:
- sniff
- test-toolkit
- darkmatter-cli
- claudine-cli
- darkmatter
- claudine
- darkmatter-lsp
source_files_during_phase_2:
- darkmatter/lib/src/process_tree.rs
- darkmatter/lib/src/lib.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/launcher.rs
- darkmatter/lib/src/effects/mod.rs
- darkmatter/lib/src/effects/fs_write.rs
- darkmatter/lib/src/editor/mod.rs
- darkmatter/lib/src/editor/resolve.rs
- darkmatter/lib/src/editor/launch.rs
- darkmatter/lib/src/editor/surface.rs
- darkmatter/lib/src/editor/service.rs
- darkmatter/lib/tests/l1/context_construction_guard.rs
- darkmatter/lib/tests/l1/error_snapshots/editor.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/cli/tests/l1/editor_support.rs
- darkmatter/cli/tests/l1/editor_resolution.rs
- darkmatter/cli/tests/l1/editor_launcher.rs
- darkmatter/cli/tests/l1/editor_service.rs
- tools/test-toolkit/src/fake_editor.rs
- tools/test-toolkit/tests/fake_editor.rs
docs_updated_during_phase_2:
- darkmatter/docs/cli/edit.md
- tools/test-toolkit/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/darkmatter/testing.md
- .claude/skills/darkmatter/library-surfaces.md
- .claude/skills/os/SKILL.md
source_files_during_phase_3:
- darkmatter/lib/src/lifecycle/actions.rs
- darkmatter/lib/src/lifecycle/signatures.rs
- darkmatter/lib/src/lifecycle/mod.rs
- darkmatter/lib/src/lifecycle/action_shape.rs
- darkmatter/lib/src/lifecycle/parse.rs
- darkmatter/lib/src/lifecycle/validate.rs
- darkmatter/lib/src/lifecycle/operands.rs
- darkmatter/lib/src/lifecycle/source_map.rs
- darkmatter/lib/src/lifecycle/schema_projection.rs
- darkmatter/lib/src/lifecycle/executor.rs
- darkmatter/lib/tests/l1/lifecycle_edit_action.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/lifecycle_schema_parity.rs
- darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
- claudine/lib/src/composition/sequence/task/mod.rs
- claudine/lib/tests/l1/claudine_schemas.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/darkmatter/lifecycle.md
source_files_during_phase_4:
- claudine/lib/src/composition/lifecycle/executor.rs
- darkmatter/cli/src/commands/compose.rs
- darkmatter/cli/src/commands/compose/lifecycle.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/cli/tests/l1/compose_edit_action.rs
- darkmatter/cli/tests/level2/main.rs
- darkmatter/cli/tests/level2/level2_edit_action.rs
- darkmatter/dmls/tests/l1/no_side_effects.rs
- darkmatter/justfile
- darkmatter/lib/src/editor/service.rs
- darkmatter/lib/src/lifecycle/cancellation/tests.rs
- darkmatter/lib/src/lifecycle/executor.rs
- darkmatter/lib/src/lifecycle/executor/host.rs
- darkmatter/lib/src/lifecycle/executor/tests.rs
- darkmatter/lib/src/lifecycle/md_host.rs
- darkmatter/lib/src/lifecycle/md_host/tests.rs
- darkmatter/lib/tests/l1/lifecycle_event_binding.rs
- darkmatter/lib/tests/l1/load_data_runtime.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
- .claude/skills/darkmatter/lifecycle.md
- .claude/skills/darkmatter/testing.md
source_files_during_phase_5:
- claudine/lib/src/composition/lifecycle/editor.rs
- claudine/lib/src/composition/lifecycle/editor/tests.rs
- claudine/lib/src/composition/lifecycle/mod.rs
- claudine/lib/src/composition/lifecycle/executor.rs
- claudine/lib/src/composition/lifecycle/executor/tests/action_dispatch.rs
- claudine/lib/src/composition/error/mod.rs
- claudine/lib/src/composition/error/render/mod.rs
- claudine/lib/src/diagnostics/registry.rs
- claudine/lib/src/composition/sequence/task/group.rs
- claudine/lib/src/composition/sequence/task/tests.rs
- claudine/cli/src/commands/compose/prep.rs
- claudine/cli/src/commands/wrap/mod.rs
- claudine/cli/tests/l1/lifecycle_edit_action.rs
- claudine/cli/tests/l1/main.rs
- claudine/cli/tests/l1/error_guards.rs
- claudine/cli/tests/level2/level2_edit_action.rs
- claudine/cli/tests/level2/main.rs
- claudine/justfile
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
- .claude/skills/claudine/architecture.md
- .claude/skills/darkmatter/lifecycle.md
---

# Edit Action — Implementation Plan

Spec: `2026-11-14-edit-action` (`spec.md` beside this file). Every choice the
spec left open is settled under [Necessary Rules](#necessary-rules) in Phase 1;
later phases cite rules by number (`R7`). No phase contains a human gate.
Human judgment happens in the review after Phase 7.

## Summary and Definition of Done

### The work

Add a lifecycle action, `edit`, that opens a named file in the caller's editor,
blocks until the editor exits, and returns to the next action. It is
registered once in Darkmatter's shared lifecycle machinery and runs in the
`md compose` host and in every Claudine event stack (document, sequence,
serial group; parallel members may run only GUI editors).

Five pieces of work carry it:

1. **Sniff owns editor facts.** Sniff's `Editor` catalog gains a surface
   (`Terminal`, `Gui`, `Both`), verified wait flags, and stem-based command
   identification. A new focused observation reports whether a display session
   exists. Darkmatter's private `wait_args_for_editor` table is removed.
2. **Darkmatter's editor module becomes surface-aware and cancellable.** One
   resolver (used by the action, wrapper `--edit`, and `md edit`) never selects a
   non-blocking editor and honors a `gui` constraint. One launcher owns the
   editor's process tree (process group on Unix, Job object on Windows), waits
   in a loop that observes cancellation, hands a terminal editor the foreground,
   and restores the host terminal. A file/eligibility service creates the target
   safely (`create_new` is race-free) and launches through them.
3. **The grammar, validation, executor, and hosts.** A new
   `LifecycleActionKind::Edit` is parsed from the short and long forms with
   strict string/boolean operand types, executed by the shared executor
   through a host-supplied editor service, classified as an `expression` action,
   and routed through both hosts' existing error paths.
4. **Schemas and tooling.** `edit` is defined in
   `darkmatter/schemas/partials/lifecycle.yaml` as an explicit lifecycle-only
   verb (never an expression function), referenced from `md-lifecycle.yaml` and
   Claudine's schemas, with shipped artifacts regenerated and DMLS covering it.
5. **Documentation and skills** describe the behavior directly, with no link to
   this spec.

### Definition of done

The work is done when all of the following hold on the implementing host, with
the other OSes proven by the repository's CI and per-OS rules:

- Every acceptance criterion (AC1–AC11) of the spec has at least one named
  passing test, listed in the implementation log with its file and name.
- `just test` and `just lint` pass in `sniff/`, `darkmatter/`, `claudine/`, and
  `tools/test-toolkit/`; `just test-l2` passes in `darkmatter/` and `claudine/`
  with no terminal or browser window taking focus.
- Root `just shipped-schemas` leaves no diff after a second run; the host
  lifecycle artifact `claudine/schemas/globals.lifecycle.json` is regenerated if
  its bytes change.
- The motivating `initialize` workflow from the spec runs end to end through
  `md compose` (fake editor) and through `claudine compose`/`sequence`
  (fake editor, no real provider): it opens the computed path, waits, prints the
  success line, and skips the provider.
- `md schema validate`, runtime parsing, and DMLS accept and reject exactly the
  same shapes (the matrix in [Input robustness matrix](#input-robustness-matrix)).
- Wrapper `--edit` and `md edit` behave as before except that a non-blocking
  editor is never selected.
- Darkmatter and Claudine canonical lifecycle docs, the schema authoring
  references, the Sniff docs and skill page, and the `darkmatter` skill describe
  `edit`, `create_new`, `gui`, host requirements, cancellation, and the
  distinction from wrapper `--edit`, `stop`, and initialize-time `skip`.
- `implementation-log.md` in this directory lists every departure from the spec
  (the log is where departures go; the spec stays as decided).

### Non-goals (from the spec's Out of Scope)

No `--edit` on composition commands, no templates or returned buffer values, no
per-action editor choice, no async editing, no changes to `stop`/`skip`/reread
boundaries/shell approval, no sandboxing of the editor, no cross-invocation
locking. No `edit()` expression function, no `--dry-run` for `md compose`.

### Spike decision

No spike. The spec states the action waits on human input and reuses discovery,
so no throughput question exists. The Windows Job-object and Unix foreground
handoff behaviors are designed from the repository's existing launcher
(`darkmatter/lib/src/markdown/compose/shell_expansion/launcher.rs`) and POSIX job
control, and are proven by the L1/L2 tests in Phases 2 and 4, not by a
measurement.

---

## Phase 1: Foundations — settled rules, Sniff editor facts, test editor

Phase 1 settles every open choice, then builds the pieces nothing else can
start without: Sniff's editor catalog and display observation, and a portable
fake editor. Phase 1 changes no Darkmatter or Claudine behavior.

### Necessary Rules

Each rule states the decision, then its evidence.

**R1 — The action is a new `LifecycleActionKind::Edit(EditAction)`, not a side
effect or expression function.**
`EditAction { file: Expr, create_new: bool, gui: Option<bool> }` in
`darkmatter/lib/src/lifecycle/actions.rs`. Evidence: the spec forbids an effectful
`edit()` function and a catalog entry; `LifecycleActionKind` already holds
parser-owned verbs (`Shell`, `RuntimeSet`, `LifecycleControl`). `action_type()`
(`parse.rs:365`) returns `ActionType::Expression` for it; `action_verb()`
(`parse.rs:381`) returns `"edit"`. `action_writes()` (`actions.rs:~166`) returns
no write set for it (the spec: editing does not recompute frontmatter or
predicates). Every exhaustive `match` on `LifecycleActionKind` gets an arm; the
sites are: `actions.rs` (`action_writes`), `parse.rs` (`action_type`,
`action_verb`, `runs_a_shell` is non-exhaustive), `validate.rs:486-498`
(operand walk) and `validate.rs:580`, `executor.rs:~294` and `~683`,
`claudine/lib/src/composition/sequence/task/mod.rs:1724,1746` (task
side-effect classification: `edit` is **not** a task side effect, see R22), and
the preflight walkers that match `SideEffect`/`ExpressionFunction`
(`preflight/candidates.rs:136`, `preflight/shell_walk.rs:128`,
`shell_template.rs:244`, `claudine/lib/src/composition/prepare.rs:1565`,
`sequence/task/templates.rs:110`, `sequence/preflight/containers.rs:43`): each
treats `Edit` like `Communication` (no shell, no template, nothing to do).

**R2 — `edit` is a lifecycle-only verb listed in one constant.**
`LIFECYCLE_ONLY_VERBS: &[&str] = &["edit"]` in
`darkmatter/lib/src/lifecycle/signatures.rs`, with `pub fn is_lifecycle_only_verb`.
`is_known_lifecycle_verb` and `all_lifecycle_verbs` include it (so bare-verb
parsing, did-you-mean, and `claudine/lib/src/composition/looping/config.rs:470,515`
see it). Evidence: `signatures.rs:224` keeps the control verbs in one list the
parser, ranking, and source map share; the spec requires distinguishing the
explicit lifecycle-only verb from catalog-derived verbs in the generated-region
and equality checks, which read this constant.

**R3 — Operand grammar, enforced at parse time with the existing error
variants.**
- Short form `edit: <string>` and `edit: ["<string>"]` build `EditAction`
  (`create_new: false`, `gui: None`). `edit`, `edit: null`, `edit: []` and a list
  of two or more are arity errors: route through
  `check_exact_positional_arity("edit", &args, 1, …)` in a new `"edit"` branch of
  `validate_positional_arity_and_build` (`action_shape.rs:~100`); this gives
  `LifecycleWrongArity` and also makes the bare verb (`parse_bare_verb`,
  `parse.rs:892`) report arity rather than "unknown verb". `positional_operand_field`
  (`action_shape.rs:81`) maps `"edit"` to `Some("file")`.
- A literal YAML number or boolean operand (`edit: 5`, `edit: true`, long-form
  `file: 5`) is rejected at parse with `LifecycleInvalidArgs` ("`edit.file` must be
  a string, got number"), after `action_value_to_expr`, by checking for
  `Expr::NumberLiteral`/`Expr::BoolLiteral` (the `break_reason` precedent at
  `action_shape.rs:~640`). A literal string that is empty or whitespace-only is
  rejected with `LifecycleInvalidArgs` ("`edit.file` must not be blank"); a
  non-blank literal is kept byte-for-byte (no trim). An interpolated operand is
  typed at runtime (R14).
- Long form: `file` required (`LifecycleInvalidArgs`, "`edit` requires a `file`
  parameter"), `create_new` and `gui` optional, `no_error` the existing sibling
  option. Unknown keys go through `reject_extra_params`. `create_new` and `gui`
  must be raw YAML booleans: in `parse_long_form` (`parse.rs:~1105`) add the
  `break.code` precedent: `if verb == "edit" && matches!(key, "create_new" | "gui")
  && !value.is_boolean()` return `LifecycleInvalidArgs` ("`edit.gui` must be a
  boolean literal, got string"). That check runs before `action_value_to_expr`, so
  `"{{ true }}"`, `"true"`, and `null` are all rejected (`null` is not the absent
  default). `with:` on `edit` falls into the existing `LifecycleWithUnsupported`.
- Duplicate inner/sibling `no_error` stays an error through the existing bundle
  code (`parse_bundle`); nothing new.
- Dictionary ordering: `canonical_rank` (`parse.rs:1208`) already places an
  unlisted verb in the third bucket, ascending by name, after communication and
  `shell`, before flow control. `edit` needs no change there; a test pins it.

**R4 — Passive checks use the same parser; no `edit` function is registered.**
`validate.rs` walks `EditAction.file` as operand path `<path>.file` and visits
nothing for the booleans. Evidence: `validate.rs:486` walks `args` by
`arg[i]`; the spec requires located diagnostics and no expression-function
registration. Nothing is added to `EFFECT_DESCRIPTORS`,
`expression_function_descriptors`, `mutations.yaml`, or `functions.yaml`.
`template_operands` (`operands.rs:~205`) includes `edit` in its parser-owned
list so a malformed `{{ … }}` span in the operand is reported as a syntax
problem by schema validation. Its `Short`/`Array` arm is left unchanged for
`edit` (a one-element list is valid; arity is the parser's concern).

**R5 — Sniff stores editor facts in a separate table, not `ProgramInfo`.**
New `sniff/lib/src/programs/enums/editor_traits.rs`:
`EDITOR_TRAITS: [EditorTraits; Editor::COUNT]` indexed by `*editor as usize`
(the `EDITOR_INFO` convention), with `EditorTraits { surface: EditorSurface,
wait_args: &'static [&'static str], identify_also: &'static [&'static str] }`
and `pub enum EditorSurface { Terminal, Gui, Both }` (`Serialize`, `Deserialize`,
`Display`, `Copy`, snake_case, re-exported from `sniff::programs`). A unit test
asserts `EDITOR_TRAITS.len() == Editor::COUNT` (as `enums/mod.rs:25` does for
`EDITOR_INFO`). Evidence: `ProgramInfo` is shared by every category and is
serialized (`program_serialization.rs`); a separate table leaves every other
category and every serialized shape unchanged. New `Editor` methods:
`surface()`, `wait_args()`, `can_block()` (`surface != Gui || !wait_args.is_empty()`),
and the associated `Editor::identify(command_token: &str) -> Option<Editor>`.

**R6 — The surface/wait table.** Verified against each vendor's documented CLI:

| Editor | Surface | Wait args | `identify_also` |
|---|---|---|---|
| Vi, Vim, Neovim, Nano, Helix, Micro, Kakoune, Amp | Terminal | none | Helix: `helix` |
| Emacs, XEmacs | Both | none | |
| VSCode | Gui | `--wait` | `code-insiders` |
| VSCodium | Gui | `--wait` | |
| Sublime (`subl`) | Gui | `--wait` | |
| Zed | Gui | `--wait` | |
| TextMate (`mate`) | Gui | `--wait` | |
| BBEdit | Gui | `--wait` | |
| Kate | Gui | `--block` | |
| PhpStorm, IntelliJ IDEA, PyCharm, WebStorm, CLion, GoLand, Rider | Gui | `--wait` | |
| Lapce, Geany | Gui | none (non-blocking) | |

Lapce and Geany stay non-blocking because the spec lists them "non-blocking until
verified otherwise" and no verified blocking flag exists. A `Both` editor can
block, so `can_block()` is true for Emacs/XEmacs. The unit test lists every
variant explicitly (a `match` with no wildcard) so a new `Editor` variant cannot
compile without a surface.

**R7 — Identification by stem.** `Editor::identify` takes the first
whitespace-separated token's path, takes `Path::file_stem()`, strips one trailing
extension only when the *original* file name's extension is `exe`, `cmd`, `bat`,
or `com` (so `code.cmd` and `Code.exe` become `code`, but `code-insiders` and
`nvim.appimage` are not split), and compares against `binary_name()`,
`alternate_binary_names()`, and `identify_also` — case-insensitively on Windows
(`cfg!(windows)`), exactly elsewhere. Evidence: the spec's examples; today
`EDITOR=/usr/local/bin/code` gets no `--wait` because the raw token is compared
with `"code"`.

**R8 — Display session is an on-demand Sniff observation.**
`sniff::os::{DisplaySession, detect_display_session}` in
`sniff/lib/src/os/display.rs`; `DisplaySession::{Available, Unavailable}`.
`display_session_from(os: OsType, runtime: RuntimeEnvironment, var: impl Fn(&str)
-> Option<OsString>) -> DisplaySession` is the pure rule: macOS and Windows →
`Available`; Linux (including WSL1/WSL2, per the spec) → `Available` iff
`DISPLAY` or `WAYLAND_DISPLAY` is non-empty. `detect_display_session()` calls it
with `detect_os_type()`, `detect_runtime_environment()`, and the process
environment. It is absent from `OsInfo`, host inventory, and caches (the
`current_user_id()` precedent in the `sniff` skill). Evidence: the spec says
Sniff reads neither variable today and wants a focused observation.

**R9 — Darkmatter's resolver (`darkmatter/lib/src/editor/resolve.rs`).**
`pub enum SurfaceRequest { Any, Terminal, Gui }`; `pub struct ResolvedEditor
{ pub program: PathBuf /* find_program result */, pub args: Vec<String>
/* configured args, then wait args not already present */, pub surface:
Option<EditorSurface> /* None = unknown command */, pub display: String /* the
command as configured or the probe's binary name */ }`;
`pub fn resolve_editor(request: SurfaceRequest) -> Result<ResolvedEditor,
EditorError>`; the existing `resolve_editor_command() -> Result<String,
EditorError>` stays as a thin wrapper (`resolve_editor(Any)` then `display`), so
`darkmatter/cli/src/commands/frontmatter.rs:536` and
`claudine/cli/src/commands/wrap/prompt_source.rs:14,41` keep compiling unchanged.
Algorithm, in order:
1. For `EDITOR` then `VISUAL`: skip an unset or blank variable; take the first
   whitespace token; reject as `Missing` when `find_program` finds nothing; reject
   as `CannotWait` when `Editor::identify(token)` is `Some(e)` and `!e.can_block()`;
   with `request != Any`, reject as `SurfaceMismatch` unless the identified editor's
   surface equals the request exactly (`Both` and unknown never match an explicit
   request); otherwise accept (unknown commands are trusted to block).
2. With `request == Any` and a variable was set but none accepted: return the
   **first** rejected variable's error (`ConfiguredEditorMissing`, or the new
   `ConfiguredEditorCannotWait { variable, value }`); the probes do not run
   (existing behavior: a typo'd editor never silently becomes another).
3. Otherwise walk `DEFAULT_EDITOR_PRIORITY`, accepting the first installed editor
   with `can_block()` and (for `request != Any`) the exact surface. With
   `request != Any` the probes run even when variables were set.
4. Nothing accepted: `Any` → `NoEditorFound`; otherwise `NoMatchingEditor
   { requested, candidates: Vec<(String, RejectReason)> }` listing every variable
   and probe checked with `Missing | CannotWait | SurfaceMismatch(Option<EditorSurface>) |
   NotInstalled`.
`launch_editor_on_path` and `edit_text` call `resolve_editor(Any)`. The
`wait_args_for_editor` function and its tests are deleted; the equivalent tests
live in Sniff (R5–R7). `build_editor_command_details` is deleted with it. The
wait flag is added only when not already among the configured args
(`EDITOR="code --wait"` stays single). `EditorError`'s `BlockError::status_block`
match gets arms for every new variant (it has no wildcard).

**R10 — The launcher (`darkmatter/lib/src/editor/launch.rs`).**
`pub struct LaunchPlan { editor: ResolvedEditor, path: PathBuf }` built by
`LaunchPlan::new(editor, path)` (resolution happens once; creation never
re-resolves). `LaunchPlan::run(self, options: LaunchOptions) -> Result<(),
EditorError>`, where `LaunchOptions { io: LaunchIo, own_tree: bool, cancelled:
Option<&dyn Fn() -> bool> }` and `LaunchIo { stdin: Stdin::{Inherit, Null},
stdout: Out::{Inherit, ToHostStderr, Discard}, stderr: Out::{Inherit, Discard} }`.
`launch_editor_on_path(path)` becomes `resolve_editor(Any)` + `LaunchPlan::run` with
`own_tree: false`, all-inherit I/O, `cancelled: None`, and waits with a plain
`Child::wait()`: its observable behavior is exactly today's (no new process group,
no foreground handoff). With `own_tree: true` and `cancelled: Some(_)`:
- **Spawn.** `Command::new(&program)`; arguments are `args` then the path as one
  argument. The path is absolute and native (the service passes the simplified
  canonical path, R15), so a leading dash cannot occur.
- **Unix.** `CommandExt::process_group(0)`. For a terminal-surface editor (surface
  `Terminal`, `Both`, or unknown) whose stdin is a terminal, a `pre_exec` closure
  (run after std's `setpgid`) blocks `SIGTTOU` with `pthread_sigmask`, calls
  `tcsetpgrp(STDIN_FILENO, getpgrp())`, and restores the mask, so the editor is the
  foreground group before it reads input (no race with a parent-side handoff). The
  host first saved the stdin `termios` (`tcgetattr`) and its own foreground group;
  after the root exits or is reaped the host ignores `SIGTTOU` (saving and
  restoring the previous action), calls `tcsetpgrp(stdin, host_pgrp)`, then
  `tcsetattr(TCSADRAIN)` with the saved settings. Restoration runs on success,
  failure, and cancellation (a `Drop` guard owns it).
- **Windows.** The root is assigned to a Job object (kill-on-close) immediately
  after spawn. The documented window between spawn and assignment, in which a
  child could escape the job, is accepted and stated in a code comment. No terminal
  handoff exists on Windows (the console is shared); Ctrl-C handling stays the
  host's.
- **Wait loop.** `Child::try_wait()` polled every 20 ms, calling `cancelled()` on
  each pass. On `true`: terminate (below), then return `EditorError::Cancelled`.
- **Terminate (shared `ProcessTree`).** Move `ProcessTree` out of
  `markdown/compose/shell_expansion/launcher.rs` into `darkmatter/lib/src/process_tree.rs`
  (`pub(crate)`), keep `launcher.rs` using it, and add `terminate_gracefully(grace:
  Duration, root: &mut Child)`: Unix sends `SIGTERM` to the group (`kill(-pgid,
  SIGTERM)`), waits up to the grace period for the root with `try_wait`, then sends
  `SIGKILL` to the group unconditionally (a no-op `ESRCH` when empty, and it sweeps
  children that ignored `SIGTERM`), then `wait()`s the root; Windows calls
  `TerminateJobObject` and waits. The grace period is the constant
  `EDITOR_TERMINATE_GRACE = Duration::from_secs(2)`. The root is reaped only after
  the sweep when it is still running; the accepted pgid-reuse window (root reaped
  before the sweep) is documented in a comment.
- **I/O.** `Inherit` is `Stdio::inherit()`; `Null` is `Stdio::null()`;
  `ToHostStderr` is a `try_clone` of the host's stderr handle (`std::io::stderr()`
  `as_fd`/`as_handle` cloned to owned) used as the child's stdout; `Discard` is
  `Stdio::null()`.

**R11 — File preparation and the service (`darkmatter/lib/src/editor/service.rs`).**
```rust
pub trait EditorService: Sync {
    fn edit(&self, request: &EditRequest<'_>) -> EditOutcome;
}
pub struct EditRequest<'a> {
    pub target: &'a Path,          // resolved, may not exist yet
    pub mutation_root: &'a Path,
    pub create_new: bool,
    pub gui: Option<bool>,
    pub cancelled: &'a dyn Fn() -> bool,
}
pub struct EditOutcome { pub attempted: bool, pub result: Result<(), EditError> }
```
`attempted` is `true` once any directory/file creation or launch was tried, even
if it failed, and `false` for every refusal before that point (resolution,
eligibility). `ProcessEditorService { facts: EditorHostFacts, probe:
EnvironmentProbe }` is the production implementation, constructed per event stack
by hosts. The order inside `edit` is exactly the spec's: (a) refuse when
`cancelled()` is already true; (b) resolve the editor via `resolve_editor`
(`SurfaceRequest::from(gui)`); (c) check eligibility (R12); (d) prepare the file
(R13); (e) launch through `LaunchPlan` with `own_tree: true`; (f) after exit,
re-run `ensure_within` and require `fs::metadata(path).is_file()`; (g) return.
Success is a zero exit with a regular file at the target (empty and unchanged are
success); no content is read.

**R12 — Host eligibility facts.**
`pub struct EditorHostFacts { pub output: EditorOutput, pub stdin_reserved: bool }`
with `enum EditorOutput { Terminal, MarkdownDocument, Structured }`.
`pub struct EnvironmentProbe { pub stdin_tty: bool, pub stdout_tty: bool,
pub stderr_tty: bool, pub foreground: bool, pub display: DisplaySession }` with
`EnvironmentProbe::process()` filling it from `IsTerminal`, `tcgetpgrp(0) ==
getpgrp()` (always `true` on Windows), and `sniff::os::detect_display_session()`.
Terminal-editor rules, evaluated after resolution and before any creation:
- `Structured` → `EditError::StructuredOutputRefusesTerminalEditor`;
- a parallel member (R16) → `EditError::NoInteractiveSurface("parallel member")`;
- `stdin_reserved`, or `!stdin_tty`, or `!stdout_tty`, or `!foreground` →
  `EditError::NoInteractiveTerminal { reason }`; `MarkdownDocument` also requires
  `stderr_tty`;
- the process-wide surface guard (`static INTERACTIVE: AtomicBool`) is taken for
  the duration of the editor; contention → `EditError::SurfaceBusy`.
GUI-editor rules: `display == Unavailable` → `EditError::NoDisplaySession`; GUI
editors are allowed in parallel members and under `Structured`. A `Both` or
unknown editor selected with `gui` omitted is treated as terminal. I/O per the
spec table: terminal editor → stdin inherited, stdout/stderr inherited
(`Terminal`) or stdout→host stderr and stderr inherited (`MarkdownDocument`);
GUI editor → stdin null, stdout→host stderr and stderr inherited, or both
discarded under `Structured`.

**R13 — File preparation.** A new `fn create_new_guarded` in
`darkmatter/lib/src/effects/fs_write.rs` (beside `atomic_write_guarded`, reusing
`ensure_within`): `ensure_within(root, target)`; `create_dir_all(parent)`;
`OpenOptions::new().write(true).create_new(true).open(path)`; on success drop the
handle (Windows editors must be able to replace the file) and return `Created`; on
`ErrorKind::AlreadyExists` return `Existing` (the caller applies `create_new`).
Existing, not `create_new`: `fs::metadata(path)` (follows an in-root symlink to its
contained target, already verified by `ensure_within`) must be a regular file,
else `EditError::NotRegularFile`. `create_new: true` and `Existing` →
`EditError::Occupied` (never opened or touched; `O_EXCL`/`CREATE_NEW` also refuse a
symlink, even a dangling one, and a directory). A concurrent creator under
`create_new: false` → treated as `Existing` and opened, never overwritten. This is
**not** `EffectEngine::ensure_file` (check-then-replace).

**R14 — Operand evaluation and target resolution in the executor.** The executor
evaluates `EditAction.file` with `evaluate_operand` (`executor.rs`, existing),
requires `Value::String` (otherwise `ActionFailure::Evaluation` with
"`edit` needs a string path, got <type>", never suppressed by `no_error`), rejects
blank-after-evaluation with the same evaluation failure, then calls a new
`resolve_edit_target(raw) -> Result<PathBuf, ActionFailure>` beside
`resolve_effect_path` (`executor.rs:1219`): it builds `FileReference::new(raw)`,
returns a dispatch failure `EditError::UrlRefused` when
`class().kind == FileReferenceKind::Url` **before** `resolve_in_context` (no
fetch), then applies exactly the `resolve_effect_path` body (document resolution,
implicit-relative creation under `effect_engine.mutation_root()`, other local kinds
by first candidate). `resolve_effect_path` and `resolve_edit_target` share one
private helper so the two cannot drift. Process CWD is never read.

**R15 — Launch path.** After preparation the service calls
`biscuit_file::canonicalize_simplified(&cleaned)` (the file now exists), so the
editor receives an absolute native path with no Windows verbatim prefix.

**R16 — Parallel members deny terminal editors.** `darkmatter::editor::surface`
exposes `deny_terminal_in_this_thread() -> TerminalDenyScope` (a `thread_local`
`Cell<bool>` with an RAII restore) and `terminal_denied() -> bool`, read by
`ProcessEditorService`. `claudine/lib/src/composition/sequence/task/group.rs`
enters the scope in each `std::thread::scope` member closure (`~line 524`, next to
`cancellations[index].enter()` and `prep_scope.enter()`). Evidence: the spec says
parallel workers explicitly deny a terminal rather than infer from process-wide
TTYs; fresh scoped threads have the default (`false`), so denial must be explicit
and a nested serial stack inside a parallel member inherits it because it runs on
the same thread.

**R17 — Cancellation signal.** The executor passes
`cancelled = || cancellation::cause_with(|| host.interrupted()).is_some()`
(`darkmatter/lib/src/lifecycle/cancellation.rs:130`), **not** `requested_with`:
`requested_with` is false inside `CleanupScope`, which `run_guard.rs:411` enters
for every terminal event (`success`, `failure`, `blocked`, `cancelled`,
`finalize`), and `edit` must still work in those events on a healthy run. A cause
being set is exactly "cancellation requested", so this refuses an edit in
`cancelled`/`finalize` after a cancellation and in a normal `failure` event only
when a cause exists. A cancelled edit returns `ActionFailure::Evaluation` with
"the run was cancelled while an `edit` was open" (the `set` precedent at
`executor.rs:~1190`), which `no_error` never suppresses, so the next action never
runs; the run guard reports the run as cancelled because the cause exists. After a
successful editor exit the executor checks `cancelled()` once more and returns the
same failure.

**R18 — Hosts supply the service.** `LifecycleHost` gets
`fn editor(&self) -> Option<&dyn EditorService> { None }`
(`darkmatter/lib/src/lifecycle/executor/host.rs`). `None` yields
`EditError::Unsupported` ("this host cannot run `edit`") as a dispatch failure
when execution reaches the action. `MdStackHost` returns the service from a new
`editor: Option<&'a dyn EditorService>` field of `MdStackContext`
(`md_host.rs:~700`), which `darkmatter/cli/src/commands/compose/lifecycle.rs:579`
fills with a `ProcessEditorService` whose facts are: `output: Structured` when
`self.document.sink.is_some()` (data output), else `MarkdownDocument`;
`stdin_reserved` = the compose input is stdin (`<stdin>` source). Claudine's
`StackExecutionContext` returns a `ProcessEditorService` built per call (R20).

**R19 — Failure routing keeps the typed cause.** New
`LifecycleRunFailure::EditFailed { source_path, property, target: PathBuf, error:
Box<EditError> }`. The executor raises it through `host.failure(…)` for every
action-level failure (dispatch class, suppressible by `no_error`). Darkmatter's
`failure_message` (`md_host.rs:~485`) renders ``("edit", format!("`{property}`
could not edit {target}: {error}"))``; Claudine maps it to a new
`CompositionError::LifecycleEditFailed { source_path, property, target, error:
Arc<EditError> }` mirroring `LifecyclePrepFailed` at every site (grep
`LifecyclePrepFailed` in `claudine/lib/src` and mirror the variant, `Display`,
diagnostic code `composition.lifecycle_edit_failed`, source-location accessor, and
catalog entry). `EditError` keeps `FileReferenceError`/`EffectError`/`EditorError`
as `#[source]` variants rather than strings.

**R20 — Claudine output mode.** `claudine/lib/src/composition/lifecycle/editor.rs`
(new) holds `static STRUCTURED_OUTPUT: AtomicBool` with `set_structured_output(bool)`
(the `interrupt.rs` atomics precedent) and
`pub(super) fn service() -> ProcessEditorService` (`output: Structured` when set,
else `Terminal`; `stdin_reserved: false` because a piped prompt already fails the
stdin-tty check). The binaries set it where the output format is resolved: the
wrapper (`claudine/cli/src/commands/wrap/mod.rs`, the resolved `OutputFormat`
`Json` or `Stream`) and `claudine/cli/src/commands/compose/mod.rs` (the
`CompositionOutputFormat` `Json` or `Stream`); `sequence`/`inline-compose` have no
output-format option. Evidence: `grep OutputFormat claudine/cli/src/commands`
lists only the wrapper and `compose`.

**R21 — The test editor.** `tools/test-toolkit/src/fake_editor.rs` (new, `pub mod
fake_editor` in `lib.rs`) holds a portable native fake editor body, `pub fn
main() -> ExitCode`, driven by environment variables (all optional):
`FAKE_EDITOR_RECORD` (append one `argv_recorder` record of the arguments),
`FAKE_EDITOR_WRITE` (path of a file whose bytes it writes into the target, the
last argument), `FAKE_EDITOR_EXIT` (exit code), `FAKE_EDITOR_DELETE` (remove the
target), `FAKE_EDITOR_REPLACE` (write via temp file + rename, an atomic save),
`FAKE_EDITOR_STDIO` (path; records whether stdin/stdout/stderr are terminals and
what stdin yielded), `FAKE_EDITOR_OUT_TEXT`/`FAKE_EDITOR_ERR_TEXT` (printed to
stdout/stderr), `FAKE_EDITOR_READ_LINE` (read one stdin line and write it to the
target), `FAKE_EDITOR_SPAWN_CHILD` (spawn itself as a sleeping child, writing the
child's PID to `FAKE_EDITOR_PIDS`), `FAKE_EDITOR_IGNORE_TERM` (Unix: `SIGTERM` set
to ignore), and `FAKE_EDITOR_HOLD` (create the named ready-marker file, then
sleep until killed). Thin `[[bin]]` crates gated by `required-features =
["test-fixtures"]` mirror `darkmatter-argv-recorder` (`darkmatter/cli/Cargo.toml:26`):
`darkmatter-fake-editor` in `darkmatter/cli`, `claudine-fake-editor` in
`claudine/cli`. Wherever `darkmatter-argv-recorder` is named in `justfile`s, CI
manifests, or archive lists (`rg darkmatter-argv-recorder`), the new binaries are
named beside it. Because Cargo supplies a binary only to its own package's tests,
launcher, service, and `md compose` tests live in `darkmatter/cli/tests/l1/`
(they call `darkmatter` library APIs directly where no process is needed).

**R22 — `edit` is not a sequence-task side effect.** `claudine/lib/src/composition/sequence/task/mod.rs:1724`
classifies task `side_effect:` actions; its refusal message list (`:1746`) gets
``"`edit` opens an editor and is not a task side effect"`` and the `Edit` kind is
refused there. Evidence: a `side_effect:` task is unattended data work
(`dispatch_task_side_effect`, `executor.rs:~265`, rejects every non-side-effect
kind), and the spec lists only event stacks and `setup:`/`teardown:` stacks (which
run through the ordinary stack executor and therefore the registered action).

**R23 — Initial resolution of the "Reader's note" in the spec.** The target
resolves through the shared mutation resolver exactly as `ensure_file` does (R14);
the docs tell authors to use one absolute path (`{{ ctx.cwd }}/…`) when the
existence probe and the edit must agree, as the spec's example does. No behavior
departs from the spec.

**R24 — Schema definition.**
`edit-short: "string(not-empty; deferred; operands(edit-short)) -> open this file
in the editor and wait for it to close; a missing file is created"` accepting also
`tuple([string(not-empty; deferred)])`, and
`edit-long: "{ file: string(not-empty; deferred; required), create_new: boolean,
gui: boolean, no_error: boolean }"` in the hand-written part of
`darkmatter/schemas/partials/lifecycle.yaml`'s `types:` (outside every `GENERATED`
region), each with a doc string for hover. The types carry no `operands(...)`
marker because `edit` has no function registration to bind; `template_operands`
(R4) covers span syntax. References are hand-written
in `expression:` (before the `# BEGIN GENERATED expression` marker, beside `set`),
in `bundle:` and `initialize-bundle:` (beside `prep: prep-short`, outside the
generated markers), and in the loop-key surface. Sites enumerated from the files:
`darkmatter/schemas/partials/md-lifecycle.yaml` — literal key `edit` in
`md-expression` (the pattern key stays catalog-only), `edit: edit-short@./lifecycle.yaml`
in `md-bundle` and `md-initialize-bundle`, and `edit` added to the exclusion list of
the `md-unknown-action` pattern; `claudine/schemas/action.yaml` and
`claudine/schemas/lifecycle.yaml` — every place `prep` or `ensure_file` is hand-listed
(action.yaml ~88, ~523, ~678, ~848; lifecycle.yaml ~112, ~315). The generator and
the equality checks treat `LIFECYCLE_ONLY_VERBS` as expected non-catalog verbs.

**R25 — Matrix and tests: none of the existing public result shapes change.**
`edit` introduces no new loader. Document shapes (duplicate keys, trailing
content, YAML syntax) are decided by the existing frontmatter loader and are not
retested.

### Input robustness matrix

The `edit` action reads frontmatter fields, so the field matrix applies; the
loader is the existing frontmatter YAML/JSON loader (no new loader, so no
document-shape tests).

**Default outcome:** every invalid shape produces a located lifecycle error
(`LifecycleError` with source path and authored property), runs nothing, creates no
file or directory, and launches nothing. This holds identically in runtime
parsing (`parse_lifecycle_config_with`), `md schema validate`, and DMLS
diagnostics.

**Exceptions (behavior that is not an error):**

| Field | Shape | Outcome |
|---|---|---|
| `edit` (short/list) | one non-blank string, or list of exactly one | valid |
| `create_new` | absent | `false` |
| `gui` | absent | `None` (either surface) |
| `no_error` | absent | `false` (existing); sibling `no_error` on a bundle is valid; both inner and sibling is an error (existing) |
| `file` | interpolated whole-value or mixed template | valid at parse; typed string-only at runtime (non-string, blank → evaluation error, never suppressed) |

**Shared validators.** `file` is one validator (a non-blank string); `create_new`
and `gui` share one (a literal boolean). Walk the field shapes through each
validator once in-process through `parse_lifecycle_config_with` (and the same
cases through `md schema validate` and DMLS only as wiring cases, below):

| Shape | `file` | `create_new` / `gui` |
|---|---|---|
| absent | error (`requires a file`) | default (see above) |
| explicit null (`file:` / `file: null`) | error | error (not the default) |
| wrong type, whole field (`file: 5`, `file: true`, `gui: "yes"`, `gui: 1`, `gui: "{{ true }}"`) | error | error |
| wrong type, one element (`edit: ["a", 1]`) | error (arity) | n/a (scalar) |
| wrong type, every element (`edit: [1]`) | error (not a string) | n/a |
| empty (`""`, `"  "`, `[]`) | error (blank / arity) | n/a |
| object where a scalar is required (`edit: {{x: 1}}` nested, `file: {a: 1}`) | error | error |

Each of `create_new` and `gui` also needs a control case (the same fixture with a
valid boolean is accepted) and one invalid shape, proving it is wired to the
shared boolean validator. Add a control row per validator: the unedited motivating
fixture parses to `EditAction { create_new: true, … }`.

**Wiring cases (spawned CLI, once per loader):** `md compose` on one fixture with
a non-boolean `gui` and one with `edit: []` asserts the located error in rendered
output and that the fake editor recorded no launch and no file appeared. DMLS and
`md schema validate` get one invalid and one control case each in Phase 6.

**Smell grep before the matrix is done** (in new code only): `rg "serde\(default\)|unwrap_or_default\(\)|filter_map\(.*as_str|\.ok\(\)" `
over the new `edit` files must show nothing that parses a load-bearing field; no
`Option<T>` conflates absent with null (`gui: Option<bool>` is `None` only when the
key is absent, because the parser rejects `null` before building it).

### Tasks

#### Wave 1 — Independent foundations (parallel)

- [x] **Sniff editor traits** (Sniff skill)
    - add `editor_traits.rs`, `EditorSurface`, and the `Editor` methods per R5–R7;
      re-export from `sniff::programs`
    - L1 tests in `sniff/lib/src/programs/enums/` (unit) asserting, for every
      `Editor` variant, the surface and wait args in R6; `can_block` for
      Lapce/Geany false, Emacs true; `identify` for `/usr/local/bin/code`, `code`,
      `code.cmd`, `Code.exe` (case-insensitive only under `cfg(windows)`; gate that
      assertion), `code-insiders`, `helix`, `nvim`, `NVIM` (no match on Unix), a
      path with spaces in the directory, and an unknown wrapper script (`None`)
    - risk: a test of `program_serialization.rs` must stay green unchanged
- [x] **Sniff display session** (Sniff skill)
    - add `os/display.rs` per R8; unit-test `display_session_from` with injected
      environments for macOS, Windows, Linux with `DISPLAY`, with
      `WAYLAND_DISPLAY`, with both empty, with neither, and WSL2 with and without
      the variables; `detect_display_session()` is exercised only for not
      panicking
- [x] **Fake editor fixture** (`rust-testing` skill)
    - add `tools/test-toolkit/src/fake_editor.rs` and the two thin `[[bin]]`
      entries per R21; add a toolkit unit test per environment variable that does
      not need a terminal (record, write, exit, delete, replace, spawn child with a
      PID file, ignore-term where `cfg(unix)`)
    - prerequisite: read `tools/test-toolkit/README.md` and the `argv_recorder`
      module for the record format; reuse it rather than adding a second format

#### Wave 2 — Sniff documentation (after Wave 1)

- [x] **Sniff docs and skill page**
    - document surface, wait flags, `identify`, the display observation, and the
      removal of Darkmatter's table in `sniff/docs/` (a new section in the
      existing programs/editor documentation page found by `rg -l "Editor" sniff/docs`)
      and in `.claude/skills/sniff/programs.md`; the doc states the behavior, not
      this spec

### Validation checkpoint

- `cd sniff && just test && just lint`
- `cd tools/test-toolkit && just test && just lint`
- confirm `sniff/lib/tests/l1/program_serialization.rs` passes unchanged

---

## Phase 2: Darkmatter editor module

Phase 2 depends on Phase 1. It delivers the resolver, the cancellable launcher,
the file/eligibility service, and keeps wrapper `--edit` and `md edit` working.

### Tasks

#### Wave 3 — Resolver and process tree (parallel)

- [x] **Resolver**
    - implement R9 in `darkmatter/lib/src/editor/` (split `mod.rs` into `mod.rs`,
      `resolve.rs`, `launch.rs`, `service.rs`, `surface.rs`; keep every existing
      `pub` path re-exported from `mod.rs` so
      `darkmatter::editor::{resolve_editor_command, edit_text, launch_editor_on_path,
      EditorError, DEFAULT_EDITOR_PRIORITY}` is unchanged)
    - delete `wait_args_for_editor`, `build_editor_command_details`, and their
      tests; move the equivalent assertions to Sniff
    - extend `EditorError` and its `status_block` arms
    - tests (serial, scoped environment as the existing `ScopedEnv`; these tests
      exist today only under `cfg(unix)` because they write executable stubs:
      add a portable stub using the fake editor binary from Phase 1 via
      `darkmatter/cli/tests/l1/editor_resolution.rs` for the Windows legs): EDITOR
      precedence; blank variable skipped; missing EDITOR falls to VISUAL;
      non-blocking `geany`/`lapce` configured is skipped and the error names the
      cannot-wait cause; non-blocking never probed; `gui: true`/`false`/omitted
      including `EDITOR=nvim VISUAL=code`, `EDITOR=emacs gui:false` falling to a
      terminal probe, `Both` and unknown never matching an explicit `gui`, the
      typed `NoMatchingEditor` listing each candidate and reason, probes running
      although a variable is set under explicit `gui`, `EDITOR="code --wait"` not
      doubling the flag, `EDITOR=/abs/path/code` getting `--wait`
- [x] **Process tree extraction**
    - move `ProcessTree` to `darkmatter/lib/src/process_tree.rs` (R10), add
      `terminate_gracefully`, keep `launcher.rs` behavior byte-identical (its
      existing tests stay green)

#### Wave 4 — Launcher (after Wave 3)

- [x] **Launcher**
    - implement `LaunchPlan`/`LaunchOptions`/`LaunchIo` and the wait loop per R10;
      route `launch_editor_on_path` and `edit_text` through it unchanged in
      behavior (`own_tree: false`)
    - Unix: process group, `pre_exec` foreground handoff, `termios` save/restore,
      `SIGTTOU` ignore around the restore; Windows: Job object after spawn with the
      accepted-window comment
    - tests in `darkmatter/cli/tests/l1/editor_launcher.rs` using
      `darkmatter-fake-editor`: exact argument vector (spaces, metacharacters,
      Unicode, a leading-dash directory name still passed absolute); wait flag
      placement; nonzero exit, launch failure, missing binary; cancellation while
      `FAKE_EDITOR_HOLD` is open terminates and reaps the whole tree (child PID file
      proves the grandchild is dead), including the `FAKE_EDITOR_IGNORE_TERM`
      variant that must take the `SIGKILL` path (assert it elapsed at least the
      grace period, less than grace + 5 s) and, on Windows, job termination; a
      GUI-client case terminates only the client; stdout/stderr routing
      (`ToHostStderr`, `Discard`, `Inherit`) using the fake editor's output text
    - `own_tree: false` path: a test proving no new process group (child pgid equals
      the host's) so `md edit` behavior is preserved

#### Wave 5 — Service and file preparation (after Wave 4)

- [x] **File preparation**
    - add `create_new_guarded` (R13) and unit tests in `fs_write.rs`: creation under
      a missing parent, `Existing` for a file, a directory, a symlink (Unix) and a
      dangling symlink; two threads racing `create_new` produce exactly one
      `Created`; a symlink escaping the root is refused before creation
- [x] **Surface denial and probe**
    - implement `surface.rs` (`TerminalDenyScope`, R16) and `EnvironmentProbe` (R12)
- [x] **Editor service**
    - implement `ProcessEditorService` per R11–R13, R15 with the `EditorService`
      trait, `EditRequest`, `EditOutcome`, `EditError` (`Unsupported`,
      `StructuredOutputRefusesTerminalEditor`, `NoInteractiveSurface`,
      `NoInteractiveTerminal`, `NoDisplaySession`, `SurfaceBusy`, `Occupied`,
      `NotRegularFile`, `UrlRefused`, `Reference(FileReferenceError)`,
      `Containment(EffectError)`, `Io`, `Editor(EditorError)`) and exports under
      `darkmatter::editor`
    - tests in `darkmatter/cli/tests/l1/editor_service.rs` with injected
      `EnvironmentProbe`/`EditorHostFacts` (no real terminal): every eligibility
      cell of the spec's table (terminal/unknown vs GUI × interactive/no tty,
      display/none, structured, denied scope, `Both` without `gui`), refusals leave
      no file and no parent directory and `attempted == false`; `create_new`
      occupied by file/dir/symlink fails with the editor never launched and bytes
      unchanged; a test hook creating the file between the existence check and the
      call (a thread creating it when the fake editor's resolution has run) proves
      `create_new:false` opens the existing file without replacing it; missing
      parent creation; byte-for-byte preservation (trailing whitespace, CRLF,
      non-UTF-8); atomic-save replacement succeeds; delete after exit fails;
      directory target after exit fails; containment recheck after exit; Unicode
      paths; `attempted == true` after a failed launch

### Validation checkpoint

- `cd darkmatter && just test && just lint`
- verify `md edit` and wrapper `--edit` unit tests (existing) pass unchanged:
  `cd claudine && just test` filtered to `prompt_source`

---

## Phase 3: Grammar, parsing, and passive validation

Phase 3 depends on Phase 1 rules only (types), so Wave 6 may start as soon as
Phase 1's Sniff pieces exist; it is sequenced after Phase 2 only so the
`EditAction`/`EditError` names are not duplicated. It changes no execution.

### Tasks

#### Wave 6 — Typed action and parse (sequential)

- [x] **Action kind and verb registration**
    - add `EditAction`, `LifecycleActionKind::Edit`, `LIFECYCLE_ONLY_VERBS`,
      `is_lifecycle_only_verb`, and every exhaustive-match arm listed in R1/R2;
      `lifecycle/mod.rs` re-exports `EditAction`
- [x] **Parse short and long forms**
    - implement R3 in `action_shape.rs` (`validate_positional_arity_and_build`,
      `build_non_control_positional_action` before the expression-function
      fallback, `build_action_from_params` before the side-effect fallback,
      `positional_operand_field`) and `parse.rs` (`parse_long_form` boolean check)
    - the `initialize` shell prohibition does not apply to `edit` (it is not a
      shell: `runs_a_shell` stays false), and `refuse_undeclared_action_types`
      sees `ActionType::Expression`; confirm `EventRegistry::md()` and Claudine's
      registry declare `Expression` for `initialize` (read `registry.rs`)

#### Wave 7 — Passive surfaces and tests (parallel, after Wave 6)

- [x] **Operand walk and type scope**
    - add the `Edit` arm in `validate.rs` (operand path `<prefix>.file`, R4),
      `action_writes` (no writes), `walk_scoped`, and `template_operands`
    - confirm both branches of a conditional are structurally checked and that an
      event-time value in `file` is deferred, not evaluated
- [x] **Parse tests** (`darkmatter/lib/src/lifecycle/tests/` and
      `darkmatter/lib/tests/l1/`)
    - the Input robustness matrix, in process through `parse_lifecycle_config_with`:
      every cell above with located-error assertions (`source_path`, `property`
      such as `initialize[0].else[0].edit.gui`), plus control rows; `edit: fqn`
      remains the literal text `fqn`; `edit: ["{{ fqn }}"]` equals the scalar form;
      `edit: ["a","b"]` is an arity error; bare `edit` is an arity error not an
      unknown verb; `with:` on `edit` is `LifecycleWithUnsupported`; compact
      dictionary ordering puts `edit` after `info` and `shell`, before `stop`; the
      sibling and inner `no_error` rules; the verb is accepted in `initialize` and
      inside `then`/`else`; `all_lifecycle_verbs()` contains `edit`; did-you-mean
      suggests `edit` for `edti`
    - update the verb-inventory assertions in `lifecycle_generated_operands.rs`
      and the catalog-equality tests under
      `lib/src/markdown/compose/expression/catalog/tests.rs` to expect
      `LIFECYCLE_ONLY_VERBS` (R2) beside the catalog verbs
    - no `edit` appears in `expression_function_descriptors()` or
      `EFFECT_DESCRIPTORS` (a test asserts it so nobody adds the function)

### Validation checkpoint

- `cd darkmatter && just test && just lint`; Phase 3 also updates the
  verb-inventory assertions in `lifecycle_generated_operands.rs` and the
  catalog-equality tests to read `LIFECYCLE_ONLY_VERBS` (R2), so the suite is
  green at this checkpoint

---

## Phase 4: Execution and the `md compose` host

Phase 4 depends on Phases 2 and 3.

### Tasks

#### Wave 8 — Executor and host trait (sequential)

- [x] **Cancellation probe**
    - no new public function is needed (R17 uses `cause_with`); add a test in
      `darkmatter/lib/src/lifecycle/cancellation/tests.rs` pinning that
      `cause_with` is `Some` inside `CleanupScope` while `requested_with` is `false`,
      documenting why `edit` uses the former
- [x] **Host trait and failure variant**
    - add `LifecycleHost::editor` (R18) and `LifecycleRunFailure::EditFailed` (R19)
      in `executor/host.rs`; update the test `FakeHost` (`executor/tests.rs:94`),
      `MdStackHost`, and Claudine's host to compile (Claudine's arm lands in Phase 5)
- [x] **Executor dispatch**
    - in `execute_action_inner` (`executor.rs:~650`) add the `Edit` arm calling
      `dispatch_edit`: evaluate the operand (R14), refuse non-string/blank as an
      evaluation failure, resolve the target (R14), refuse in `Cancelled`/cause
      state (R17), call `host.editor()`, call `host.side_effect_attempted()` once when
      `EditOutcome::attempted`, map `EditError::Editor(EditorError::Cancelled)` and
      any cancelled state to the evaluation failure of R17, and map every other
      failure to `ActionFailure::Dispatch(host.failure(EditFailed{…}))`
    - `dispatch_task_side_effect_inner` keeps rejecting `Edit` via its wildcard
      (R22)

#### Wave 9 — `md compose` wiring and tests (parallel, after Wave 8)

- [x] **md host wiring**
    - add `editor` to `MdStackContext`, `MdStackHost::editor`, the
      `failure_message` arm, and the CLI fill at
      `darkmatter/cli/src/commands/compose/lifecycle.rs:579` per R18
- [x] **Executor and host L1 tests** (`darkmatter/lib/src/lifecycle/executor/tests.rs`
      and `darkmatter/cli/tests/l1/compose_edit_action.rs`, process recorder not
      mocked dispatch for the CLI ones)
    - the motivating `initialize` workflow end to end through `md compose` with the
      fake editor: opens the absolute computed path, writes known bytes, prints the
      success line (stderr in Markdown mode), skips the provider/composition; the
      existing-file branch warns and never launches; prompt stored outside the
      launch directory with an absolute `{{ ctx.cwd }}` path; the race hook
      (`create_new:true` fails, editor never launched, bytes unchanged,
      `no_error:true` continues; `create_new` omitted opens the existing file);
      directory and symlink `create_new` targets fail without launch
    - ordering in `initialize`, in a later document event (`composed`, `failed`,
      `finalize`), inside `then`/`else`, and in a compact dictionary
    - after-edit reads: a later `file_exists`/file-reading expression in the same
      stack sees changed and deleted data, including after a failed edit suppressed
      by `no_error` (`side_effect_attempted` invalidates the memo)
    - failures: no editor, `Unsupported` host (the `FakeHost` default), noninteractive
      stdio, directory target, filesystem refusal (outside the mutation root and a
      symlink escape), launch failure, nonzero exit, deleted target, each with the
      located message naming action, property, and target, no continuation after an
      unsuppressed failure, continuation with `no_error`; refusals leave no new
      target or parent directory
    - output ownership: Markdown mode — fake editor screen output never appears in
      the composed document; structured output — a terminal editor yields exactly one
      valid result with the refusal and no editor output on either stream (ordinary
      failure and `no_error` continuation), and a fake GUI editor runs with its
      output discarded, stderr empty, one valid result; a redirected document output
      fails the terminal requirement
    - dry-run/passive: `md compose --shell`, library composition, `md schema
      validate`, and DMLS analysis never launch or create (process recorder and
      directory snapshot); an untaken branch never resolves an editor or touches
      disk while its invalid operands still produce passive diagnostics
- [x] **Terminal integration test** (`darkmatter/cli/tests/level2/level2_edit_action.rs`,
      `just test-l2 edit_action`, tmux backend, no window focus per the repository
      L2 rules and `biscuit-test-harness` skill)
    - the blocking handoff: the fake editor with `FAKE_EDITOR_READ_LINE` reads a line
      typed into the pane, writes it, exits; the next action prints; assert the pane
      shows the editor received input and the host terminal is restored (the
      shell prompt accepts input afterward); a cancellation case that sends the
      interrupt to the host while `FAKE_EDITOR_HOLD` is open and asserts the tree is
      reaped and the terminal restored; Unix only, native Windows covered by the L1
      process tests

### Validation checkpoint

- `cd darkmatter && just test && just test-l2 edit_action && just lint`

---

## Phase 5: Claudine integration

Phase 5 depends on Phase 4. The shared action is exposed through Claudine's host
profile; Claudine owns no parser or implementation.

### Tasks

#### Wave 10 — Host and cancellation wiring (parallel)

- [x] **Host and error mapping**
    - add `editor.rs` (R20), `StackExecutionContext::editor` returning
      `Some(&service)`, and `CompositionError::LifecycleEditFailed` mirrored from
      `LifecyclePrepFailed` at every site (R19)
    - set `set_structured_output` in the wrapper and `compose` commands (R20)
- [x] **Parallel denial and fake editor**
    - enter `deny_terminal_in_this_thread()` in each `std::thread::scope` member
      closure in `group.rs` (R16)
    - add the `claudine-fake-editor` thin bin and its wiring beside
      `claudine-argv-recorder` (R21)
- [x] **Task refusal**
    - implement R22 in `sequence/task/mod.rs`

#### Wave 11 — Claudine tests (parallel, after Wave 10)

- [x] **Claudine L1** (`claudine/cli/tests/l1/` with `CliProcessFixture`, no real
      provider: a fake provider executable that records launches, as the existing
      L1 tests do)
    - the motivating workflow through `claudine compose` and through a `sequence`
      document: opens the file, prints success, `skip` means the provider is never
      launched; the existing-spec warn branch; document, sequence, and serial-group
      stacks each run `edit` in order (action ordering in `start`, `success`,
      `finalize`, task `setup:`/`teardown:`)
    - a `side_effect:` task containing `edit` is refused (R22)
    - parallel group: a terminal editor fails without launching or creating; a GUI
      editor (fake, null stdin, display forced available through the injected probe)
      runs; a Linux no-display host refuses a GUI editor before creation (injected
      probe — Claudine's service reads `EnvironmentProbe::process()`, so this one
      case uses a library-level test in `claudine/lib` with a probe override hook
      `service_with(facts, probe)` that the production `service()` calls)
    - structured output: `--output json` refuses a terminal editor and keeps one
      valid result; `no_error` continuation
    - cancellation: a sequence `timeout:` shorter than the held editor, and a sibling
      failure in an atomic parallel group, each terminate and reap the fake editor's
      tree (`FAKE_EDITOR_SPAWN_CHILD`, ignoring-`SIGTERM` variant escalates), keep
      bytes saved before cancellation and a file the action created, skip the next
      action even with `no_error`, and report the run as cancelled; a fake GUI client
      proves only the client is terminated
    - wrapper `--edit` and `md edit` regressions: a non-blocking configured editor is
      refused with the cannot-wait error; everything else unchanged
- [x] **Claudine L2** (`claudine/cli/tests/level2/level2_edit_action.rs`, tmux, no
      focus): a serial stack's terminal editor receives input and returns to the next
      action; terminal state restored after success and cancellation

### Validation checkpoint

- `cd claudine && just test && just test-l2 edit_action && just lint`

---

## Phase 6: Schemas, shipped artifacts, host profile, and DMLS

Phase 6 depends on Phases 3 and 5 (the parser decides the shapes the schemas
must mirror).

### Tasks

#### Wave 12 — Schema definition (sequential)

- [ ] **Define and reference the verb**
    - implement R24 in `darkmatter/schemas/partials/lifecycle.yaml`,
      `darkmatter/schemas/partials/md-lifecycle.yaml`, `claudine/schemas/action.yaml`,
      and `claudine/schemas/lifecycle.yaml`
    - teach `schema_projection.rs` and `lifecycle_schema_generator.rs` that `edit`
      is an expected explicit lifecycle-only verb (`LIFECYCLE_ONLY_VERBS`), not a
      catalog verb; the equality tests were updated in Phase 3
    - run `just regen-lifecycle-schema` (darkmatter) and confirm the generated
      regions did not change
- [ ] **Regenerate artifacts**
    - root `just shipped-schemas`, then regenerate
      `claudine/schemas/globals.lifecycle.json` with the host-artifact recipe in
      `darkmatter/docs/topics/lifecycle/host-artifact.md` (the host's accepted verbs
      and registry did not change, so expect no byte change; keep the result
      either way); run `just shipped-schemas` a second time and confirm no diff.
      Phase 3 changed no preparation code, so `shipped::PREPARATION_REVISION` is
      not bumped

#### Wave 13 — Parity tests (parallel, after Wave 12)

- [ ] **Schema validation parity**
    - the same malformed shapes and strict operand types are rejected by `md schema
      validate`, runtime parsing, and DMLS: one test file in
      `darkmatter/lib/tests/l1/` iterating the matrix cells through the library
      schema validator and the parser and asserting agreement, plus one CLI wiring
      case for `md schema validate`
    - freshness tests for the shipped artifacts and any changed host profile pass
- [ ] **DMLS**
    - `darkmatter/dmls/src/providers/lifecycle_host_tests.rs` and the completion,
      hover, and diagnostics tests: completion offers `edit` in `initialize` and in
      `composed`, hover on `edit` shows the schema description, a malformed `gui` and
      a blank `file` produce diagnostics at the authored location in both an `md`
      document and a Claudine document (host artifact loaded), an untaken-branch
      invalid operand still diagnoses, and no analysis path launches an editor or
      writes a file (a process recorder and directory snapshot)

### Validation checkpoint

- `cd darkmatter && just test && just lint`; `cd claudine && just test && just lint`
- root `just shipped-schemas` produces no diff

---

## Phase 7: Documentation, skills, and closing verification

Phase 7 depends on all earlier phases.

### Tasks

#### Wave 14 — Documentation (parallel)

- [ ] **Darkmatter lifecycle docs** — `darkmatter/docs/topics/lifecycle/index.md`
      (the action, both forms, `create_new`, `gui`, host requirements table, file
      creation, blocking, failures, cancellation behavior including GUI editors that
      stay open and the `gui: false` guarantee, and the distinction from wrapper
      `--edit`, `stop`, and initialize-time `skip`), `error-handling.md` for the
      failure classes, `side-effects.md` cross-link only, and an overview link
- [ ] **Claudine docs** — `claudine/docs/topics/flow-control/lifecycle.md`,
      `claudine/docs/topics/schemas.md` (authoring reference), parallel-group and
      structured-output behavior
- [ ] **Skills** — `.claude/skills/darkmatter/lifecycle.md` (the service/registration
      contract: `LifecycleHost::editor`, `EditorService`, `LIFECYCLE_ONLY_VERBS`),
      `.claude/skills/claudine/SKILL.md` (the lifecycle verb list),
      `.claude/skills/use-darkmatter` and `.claude/skills/use-claudine` (their
      lifecycle verb lists), and `.claude/skills/os/SKILL.md`: add one entry for
      the Windows editor facts (Job-object assignment window, shared-console
      Ctrl-C, `.cmd` shim launch through the `find_program` path); the Sniff skill
      page was updated in Phase 1
- [ ] Docs rules: explain what each section covers before its rules, one compact
      example per rule, a Mermaid diagram for the edit lifecycle (resolve → eligibility
      → create → launch → wait/cancel → recheck), no link to or mention of the dated
      feature directory, US English

#### Wave 15 — Closing verification (sequential)

- [ ] **Drift pass** over every changed symbol's `///`/`//!` docs and inline
      comments (comment-quality rules: no HOW-narration, WHY at surprising lines, the
      accepted Job-assignment window and pgid-reuse window documented where they
      occur)
- [ ] **Full package-area gates** — `just test` and `just lint` in `sniff/`,
      `darkmatter/`, `claudine/`, `tools/test-toolkit/`; `just test-l2` in
      `darkmatter/` and `claudine/`; root `just shipped-schemas` twice; run
      `just ci-local --plan` and review that the new test binaries are scheduled
- [ ] **Smell grep** from the robustness section over the new files
- [ ] **Implementation log** — write `implementation-log.md` in this directory:
      the AC1–AC11 to test mapping, every departure from the spec (if none, say
      so), the host(s) each test ran on, and the OSes proven only by CI
- [ ] Do not move this feature to `_completed`, run `just complete`, or commit; the
      terminal state is "implementation complete, ready for review"

### Validation checkpoint (final)

All items in [Definition of done](#definition-of-done) are checkable from the
implementation log and the command outputs above.