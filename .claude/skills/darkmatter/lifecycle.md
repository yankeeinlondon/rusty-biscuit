# Lifecycle engine (`darkmatter::lifecycle`)

Darkmatter owns the lifecycle grammar and engine; a host (today Claudine)
supplies its names, services, and the work only it can do. The module's `//!`
docs (`darkmatter/lib/src/lifecycle/mod.rs`) are the map; this page is the
working summary. User docs: `darkmatter/docs/topics/lifecycle/index.md`
(events, grammar, the `md` host, host registration) and `error-handling.md`
(failure routing, `err`, recovery modes, the `finalize` rules); Claudine's
host view is `claudine/docs/topics/flow-control/lifecycle.md`. Change those
pages with the behavior.

## What lives where

| Piece | File | Notes |
|---|---|---|
| Event registry | `registry.rs` | `EventRegistry`: Darkmatter's five `CoreEvent`s, host renames (`composed`/`failed` only), host events by `Position` × `Role`, one `OutcomePair` with downgrades, per-event `BindingView` (`generic_view`: `err`, `timing`) and `ActionTypes`, host late globals; fixed once cloned (`RunStarted`). Maps events to `LifecycleSignal` slots (`signal`, `id_of`, `name_of`); `resolve_key`, `authored_event_keys` (the deferred lifecycle keys), `host_globals`, `check_emission` (`EmissionError`). `EventRegistry::md()` is core-only; Claudine's registration is `claudine::composition::claudine_registry()` |
| Parser, action shapes, signatures, validation, source paths | `parse.rs`, `action_shape.rs`, `actions.rs`, `signatures.rs`, `validate.rs`, `source_map.rs` | `LifecycleParseOptions` injects the host's `registry` (which keys are stacks; a renamed Darkmatter spelling is `LifecycleRenamedEvent`, a host-only key under `md` is data), the effect catalog, and `UnknownVerbPolicy` (`Reject`, Claudine's; `Ignore` keeps an unknown verb as an opaque `LifecycleActionKind::Ignored { verb, path }` whose operand is never read) |
| Authoring errors | `error.rs` | `LifecycleError` (27 variants). Claudine maps each to its same-named `CompositionError` twin with identical `Display` |
| Typed shell templates, presence decision | `shell_template.rs` | `placeholder_guard`, `lifecycle_declared_types` (typed against the registry's `host_globals`), `walk_scoped` (in `actions.rs`) are reviewed sites in Claudine's `type_scope_ownership_guard.rs`, which also scans this directory |
| Preflight | `preflight.rs`, `preflight/{shell_walk,candidates}.rs` | `PreflightPlan` (host checks registered once, in run order) → `PreflightRun::{check, skip, ready}`; a `CheckStage::InRun` failure is `HostFailure { stage: Preflight }`, a `BeforeRun` one fires nothing. The approval-free lifecycle shell walk: `LifecycleShellSites::stamp` fixes bytes/typed templates through the host's `ShellAuditBindings`; `collect_lifecycle_shell_audit` (condition-blind), `settle_lifecycle_shell_modes`. Errors are `PreflightError` (Claudine twins in `composition/error/lifecycle_preflight.rs`). Claudine keeps the prompting approval and registers its checks in `composition/preflight/checks.rs` |
| `err`/`timing` envelope | `envelope.rs`, `context.rs` | `DiagnosticSnapshot` data and `LifecycleErrorInfo`; classification is the host's `DiagnosticCatalog` |
| Services | `effects.rs`, `services.rs` | `LifecycleEffects` (text, message, speech, effect, notification; no TTS/messaging config), `LifecycleServices`, `MissingServiceError` |
| Executor | `executor.rs`, `executor/{host,shell,outcome}.rs` | `StackExecutor` + `StackInputs`; host policy through `LifecycleHost` (including its `registry`); shells through `ShellRunner` (`DisabledShellRunner` before `start`) |
| Run accounting | `run_guard.rs` | `RunAccounting` (once-only events, one terminal slot, shell lock/relock), `LifecycleRunGuard` over `LifecycleRunHost` |
| Policy kernel | `policy.rs` | Every lifecycle decision, pure: `recovery_verdict(SubjectKind, signal, control)` (`Admit`/`Ignore`/`Refuse(Refusal)`), `refusal_route` (`FinalizeIfOwed`, `FinalizeIfOwedReadingRefusal`, `FailureThenFinalize`), `failure_event(FailureStage, launched)` (`blocked` or `failure`), `proxy_budget_exhausted(documents_in_chain)` (the chain counts its origin), `refused_recovery(EvaluationMode, control)` (the recovery verb a cleanup refuses). `SubjectKind` is `Document`, `RootSequence`, `Sequence`, or `Group`. `Run`, `ContainerRunner`, and Claudine's drivers call it; Claudine maps a `Refusal` to its error with `CompositionError::from_lifecycle_refusal`, and Claudine's one-implementation guards (`claudine/{lib,cli}/tests/l1/lifecycle_one_implementation_guard.rs`) fail on a Claudine copy of any of these |
| Routing and admission | `runtime.rs`, `control.rs` | `decide_lifecycle_transition`, `LifecycleCatchProtocol`, `EvaluationMode` (normal recovery, fatal-evaluation, refusal, cancellation cleanup), `ContainerRunner`, `decide_control` |
| Request/reply driver | `run.rs`, `run/protocol.rs` | `Run::{start, pending, reply, cancel, shell_permission}`; tickets bind run/document/attempt/iteration/generation; a stale, duplicate, foreign, wrong-kind, or post-cancel reply is a `ReplyError` and changes nothing |
| The `md` host | `md_host.rs` (+ CLI `darkmatter/cli/src/commands/compose/lifecycle.rs`) | Core-only profile (`registry()` is `EventRegistry::md()`, `UnknownVerbPolicy::IgnoreUnprovided(UNPROVIDED_VERBS.into())`: another host's verb or an unknown one is an inert `Ignored` node, warned once per reached occurrence; every verb `md` runs parses strictly, and per-event action types are enforced); `MdStackHost`, `MdShellRunner` (permission → exact approval per segment → built-in blacklist; exclusions were judged at preflight, and `MdStackHost::shell_exclusions` serves the template executor), `MdDiagnostics` (its `DiagnosticCatalog::event_name` gives `md` spellings in default texts), and `drive`, the request/reply loop over `Run`. Only documents authoring a core event key take the lifecycle path; on a root a registered host identifies, only the stacks of `md_host::execution_projection` (the events `md` shares with the host by name) parse and run, the host's event keys are deferred (`foreign_deferred_keys`), and its globals are unavailable through `MdStackHost`'s bindings (`ComposeOptions::with_selected_host`); an `initialize` error is reported as the preparation failure so `failed` runs; a `composed` error or a failed write ends failed without `failed`. `stamp_lifecycle_shells` types placeholders by the root document's effective schema (as body `::shell` does), so a value the schema rejects is refused before the command runs. Nothing runs unless `drive` is called. The CLI's `TerminalEffects` styles a `stdout` action only when stdout is a terminal; on a pipe the text joins the document as authored. The locked-shell message names the host's unlock event through `DiagnosticCatalog::event_name(Start)` (`executor.rs::error_or_action`), so `md` says `composed` while `ShellRunError::BeforePreflight`'s `Display` keeps Claudine's traced `start`. `MdDiagnostics` classifies nothing, so under `md` every `err` facet but `msg` is `null` |
| Host lifecycle artifact | `host_profile.rs` (read by `config::registration::load_lifecycle`) | A host's `EventRegistry` and `LifecycleParseOptions` as passive JSON (format 2), the `lifecycle` member of its registration bundle, versioned by the namespace catalog hash (`config::registration::namespaces::namespace_catalog_hash` over the namespace files and every bundled file they read; `global_catalog: true` requires `host_globals` ⊆ the catalog): `lifecycle_artifact(&options, effect_names, catalog_hash)` writes it (Claudine: `composition::lifecycle::lifecycle_artifact`, staged by the root `just shipped-schemas` into `claudine/lib/generated/globals.lifecycle.json` after `check_lifecycle_artifact` loads it, freshness test in `claudine/lib/tests/l1/host_catalog.rs`); `HostLifecycleProfile::from_artifact_at(catalog, hash, path, artifact)` rebuilds both through the registration calls (strict reader; any problem is a reason, never a default). DMLS's only route to a host's events, renames, per-event globals, and verbs. Runtime data is owned, never leaked: `GlobalRequirement` names and `declare_host_global` take `Cow<'static, str>`, `register_global_catalog` takes a `CatalogHandle` (`&'static` or `Arc`), `IgnoreUnprovided` holds a `VerbList` (static slice or `Arc<[String]>`); compiled hosts keep one-line registrations. Schema roots are never searched for a host catalog or artifact; a registered host's profile is its only source |

## Rules

- Darkmatter never depends on Claudine: `darkmatter/lib/tests/l1/dependency_direction_guard.rs`
  walks the lib, CLI, and DMLS manifests (following path dependencies) and
  scans their sources.
- One implementation: `LifecycleRunGuard` and `Run` share `RunAccounting` and
  the routing functions, and every driver asks `policy.rs` which recovery an
  event admits and how a refusal is routed. Do not add a second state machine
  or decide those again at a call site.
- Production behavior is authoritative over the spec's section 8. The
  recorded traces (`claudine/cli/tests/fixtures/lifecycle_traces/*.trace`)
  replay through `Run` in `darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs`;
  never edit a `.trace` to make a change pass.
- `lifecycle_boundary/{engine,profiles,publisher}.rs` are a test-only
  prototype run driven by the production registry; its registration tests
  live in `protocol_tests.rs`.
- A parser-owned verb that is neither flow control nor a catalog entry (today
  only `edit`, `LifecycleActionKind::Edit`) is listed once in
  `signatures.rs::LIFECYCLE_ONLY_VERBS`; `is_known_lifecycle_verb`,
  `all_lifecycle_verbs`, `template_operands`, the source map's long form, and
  the schema projection's collision check (`hand_owner`) read it. Never
  register such a verb as an expression function or side effect
  (`lib/tests/l1/lifecycle_edit_action.rs` asserts it). A new
  `LifecycleActionKind` needs an arm in `action_type`/`action_verb`
  (`parse.rs`), `walk_action` (`validate.rs`), `execute_action_inner`, and
  Claudine's `describe_action` (`sequence/task/mod.rs`); the wildcard walkers
  (preflight, shell templates, `action_writes`) treat it as inert.
- `edit` runs in `executor.rs::dispatch_edit` through the host's
  `LifecycleHost::editor()` (`None`, the default, fails the action as
  `EditError::Unsupported`; `md compose` fills `MdStackContext::editor` with a
  `ProcessEditorService` per event: `Structured` under a data output, else
  `MarkdownDocument`, `stdin_reserved` when the document came from stdin;
  Claudine's host lends one built at each edit, see the `claudine` skill's
  architecture page, and maps `EditFailed` to `LifecycleEditFailed`).
  A non-string/blank `file` and any cancellation are unsuppressible
  evaluation failures; everything else is a dispatch failure classified as
  `LifecycleRunFailure::EditFailed` (`no_error` applies). The cancellation
  probe is `cancellation::cause_with`, never `requested_with`, which is false
  inside the `CleanupScope` every terminal event runs in. The target resolves
  through `mutation_target`, the one helper `resolve_effect_path` also uses
  (the seven `*_frontmatter` verbs included, resolved once and mirrored into
  live state by `PathIdentity` of the resolved target), with URLs refused
  first: an existing file by the reference contract, a missing one at its
  first candidate (beside the document for a bare relative name; an
  unmatched `%` is refused), then `ensure_within` against the engine's
  mutation root before anything is opened or created. The root is only the
  boundary, never a base. Every host (`md compose` and every Claudine
  route: compose, wrapper, staged boot, loop harness, sequence container,
  task, step guard) builds its `EffectEngine` with the request context's
  `launch_scope().local_root()` (the working tree containing the
  launch directory, else that directory), so a prompt in a sibling library
  can edit a `{{ ctx.cwd }}` path but not a file beside itself. The resolver
  derives the document context from `inputs.file_resolution_context`, so a
  host running a stack for another document (Claudine's referenced sequences
  and groups) must pass that document's own context, keeping the launch
  scope, or the reference fails as "does not contain the resolution source".
  Failure messages are folded to one escape-free line (`concise_message`)
  and never shortened, so `err.msg`, `DiagnosticSnapshot::message`, and
  every report (`report_diagnostics::lifecycle_failure_diagnostics`,
  `md compose`'s status line, Claudine's run and step errors) carry the same
  complete text; delivery adapters (speech, messaging, notifications) fit it
  to their channel. `edit` keeps its reason-first wording because it reads
  better. Terminal-editor success is provable only in L2
  (`cli/tests/level2/level2_edit_action.rs`); L1 process tests use the fake
  editor installed as `code` (GUI, `--wait`) with `DISPLAY` set.
- The editor seam is `darkmatter::editor::EditorService` (`edit(&EditRequest)
  -> EditOutcome`; `Sync`). A host registers one by overriding
  `LifecycleHost::editor()`; the stock implementation is
  `ProcessEditorService::new(EditorHostFacts { output, stdin_reserved })`,
  which checks in order: cancelled → resolve (`resolve_editor`, never a
  non-blocking editor; `gui` constrains the surface) → eligibility (a
  terminal editor needs an interactive terminal no one else holds and is
  refused on a thread under `editor::deny_terminal_in_this_thread()`, which
  Claudine's parallel members enter) → containment → `create_new_guarded`
  → launch → recheck. Errors are `EditError` (cancellation is
  `EditError::Editor(EditorError::Cancelled)`, test with `is_cancelled()`).
  `EditOutcome::attempted` turns true at the first create or launch; the
  executor then calls `host.side_effect_attempted()` once so cached reads
  are dropped. The service owns the editor's process tree with Sniff's
  `sniff::process::ProcessTree` (own process group on Unix, kill-on-close Job
  Object assigned before the editor runs on Windows) and polls
  `EditRequest::cancelled` every 20 ms; cancellation terminates the tree
  (Unix `SIGTERM`, `EDITOR_TERMINATE_GRACE`, then `SIGKILL`) and reaps it
  before returning. On Unix a terminal editor with inherited stdin also runs
  under `sniff::process::ForegroundHandoff`, restored explicitly after the
  editor is reaped (details in [library-surfaces.md](library-surfaces.md)). When Windows cannot release a finished editor's tree,
  `EditOutcome::warning` carries the text and the executor passes it to
  `LifecycleEffects::warn` once; the edit still succeeds. Contract:
  [process trees](../../../sniff/docs/topics/process-trees.md); Windows
  specifics are in the `os` skill.
- `edit` is a lifecycle-only verb, never a catalog entry: schema projection
  (`hand_owner`) fails generation when a catalog function or effect is named
  `edit`. Its schema types (`edit-short`/`edit-long` in
  `darkmatter/schemas/partials/lifecycle.yaml`, flags
  `boolean(default(false))` / `gui: boolean`) are backed by the parser's own
  rules: `markdown/schemas/bound_operands.rs::report_refused_targets` applies
  `action_shape.rs::{edit_target_problem, edit_flag_problem}` (which the
  parser also calls, over `edit_file_value` and `literal_flag`), so
  `md schema validate`, DMLS, and `md compose` agree: `edit: 5` opens `"5"`,
  `gui: "yes"` is `true`, a null flag is unset, and `edit: null`, `gui: 1`, or
  a template flag is refused. A whole-value target template is never refused
  for its type at parse time (`"{{ 5 }}"` passes); `executor.rs::dispatch_edit`
  passes the evaluated value through the same `edit_file_value`, so a number
  or boolean result names its text and only null, a collection, or blank text
  fails at run time, before any launch.
- Authored flag origin: hosts that parse a schema-completed value pass the
  pre-completion value as `LifecycleParseOptions::authored_values` (`md`
  parses the authored document, so `None`). Only a non-null authored
  `no_error` is a placement; two placements fail even when equal; a filled
  `false` never conflicts, meets `then`, creates a verb, or gives `set` a flag;
  `set` refuses any authored `no_error`, `null` included
  (`lifecycle/tests/authored_flags.rs`). Those placement and branch-shape
  rules are `parse.rs`'s `PlacementRule` helpers, shared with schema
  validation through `placement_rule` (see `schema.md`); add a new
  placement rule there, never as a second check in one of them.
- `initialize` runs before schema validation; a schema refusal is a
  composition failure (`failed`, `finalize`) and `initialize`'s effects stay.
  CLI cells that must prove "no effect on refusal" put the marker in
  `composed` (`cli/tests/l1/compose_loader_wiring.rs`). Passive surfaces (DMLS,
  `md schema validate`, hover, completion) never run a stack, so they never
  create a target or launch an editor (`dmls/tests/l1/no_side_effects.rs`,
  `edit_action_editor.rs`).
- Never add a hand-maintained list of lifecycle event keys or late roots;
  derive it from the host's `EventRegistry`.
- Lifecycle operands evaluate lazy functions in
  `executor.rs::call_with_operand_values`: its `LazyKind::Recover` arm sends
  `try` to `call_recovery`, which evaluates the fallback only when the first
  operand fails with `is_recoverable_read_failure()` (a template operand's
  `Compose` failure propagates). Claudine's lifecycle `message`, `set`, and
  `when:` reach it because Claudine runs its stacks through the same
`StackExecutor`; never add a Claudine copy
  (`claudine/lib/src/composition/lifecycle/executor/tests/recovery.rs`).
  Lifecycle shell failures are outside the boundary. See
  [Recovering from read failures](../../../darkmatter/docs/topics/darkmatter-expressions.md#recovering-from-read-failures).

## Planned

Claudine's drivers still call the direct-call `LifecycleRunGuard` rather than
`Run`'s request/reply protocol (both decide through `policy.rs`).
`check_emission` is not called: `md` registers no host events, and Claudine's
drivers fire only registered ones.
