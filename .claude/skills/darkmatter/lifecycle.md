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
| The `md` host | `md_host.rs` (+ CLI `darkmatter/cli/src/commands/compose/lifecycle.rs`) | Core-only profile (`registry()` is `EventRegistry::md()`, `UnknownVerbPolicy::IgnoreUnprovided(UNPROVIDED_VERBS.into())`: another host's verb or an unknown one is an inert `Ignored` node, warned once per reached occurrence; every verb `md` runs parses strictly, and per-event action types are enforced); `MdStackHost`, `MdShellRunner` (permission → exact approval per segment → blacklists), `MdDiagnostics` (its `DiagnosticCatalog::event_name` gives `md` spellings in default texts), and `drive`, the request/reply loop over `Run`. Only documents authoring a core event key take the lifecycle path; an `initialize` error is reported as the preparation failure so `failed` runs; a `composed` error or a failed write ends failed without `failed`. `stamp_lifecycle_shells` types placeholders by the root document's effective schema (as body `::shell` does), so a value the schema rejects is refused before the command runs. Nothing runs unless `drive` is called. The CLI's `TerminalEffects` styles a `stdout` action only when stdout is a terminal; on a pipe the text joins the document as authored. The locked-shell message names the host's unlock event through `DiagnosticCatalog::event_name(Start)` (`executor.rs::error_or_action`), so `md` says `composed` while `ShellRunError::BeforePreflight`'s `Display` keeps Claudine's traced `start`. `MdDiagnostics` classifies nothing, so under `md` every `err` facet but `msg` is `null` |
| Host lifecycle artifact | `host_profile.rs` (+ `markdown/schemas/catalog.rs::host_lifecycle_profiles`) | A host's `EventRegistry` and `LifecycleParseOptions` as passive JSON, `<catalog stem>.lifecycle.json` beside its host global catalog, versioned by the catalog's BLAKE3 hash: `lifecycle_artifact(&options, effect_names, catalog_text)` writes it (Claudine: `composition::lifecycle::lifecycle_artifact`, `just regen-lifecycle-artifact`, freshness test in `claudine/lib/tests/l1/host_catalog.rs`); `HostLifecycleProfile::from_artifact` rebuilds both through the registration calls (strict reader; any problem is a reason, never a default). DMLS's only route to a host's events, renames, per-event globals, and verbs. Runtime data is owned, never leaked: `GlobalRequirement` names and `declare_host_global` take `Cow<'static, str>`, `register_global_catalog` takes a `CatalogHandle` (`&'static` or `Arc`), `IgnoreUnprovided` holds a `VerbList` (static slice or `Arc<[String]>`); compiled hosts keep one-line registrations. `host_lifecycle_profiles` is a lazy iterator (roots in order, sorted names, shadowing claimed before a candidate is tried) over a latest-per-path cache built from the hashed catalog bytes (`from_host_text`); `catalog::discovery_probe` (`cfg(test)`/`work-counters`) observes reads and builds and injects read errors on the current thread |

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
- Never add a hand-maintained list of lifecycle event keys or late roots;
  derive it from the host's `EventRegistry`.

## Planned

Claudine's drivers still call the direct-call `LifecycleRunGuard` rather than
`Run`'s request/reply protocol (both decide through `policy.rs`).
`check_emission` is not called: `md` registers no host events, and Claudine's
drivers fire only registered ones.
