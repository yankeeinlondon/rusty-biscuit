# Lifecycle engine (`darkmatter::lifecycle`)

Darkmatter owns the lifecycle grammar and engine; a host (today Claudine)
supplies its names, services, and the work only it can do. The module's `//!`
docs (`darkmatter/lib/src/lifecycle/mod.rs`) are the map; this page is the
working summary.

## What lives where

| Piece | File | Notes |
|---|---|---|
| Parser, action shapes, signatures, validation, source paths | `parse.rs`, `action_shape.rs`, `actions.rs`, `signatures.rs`, `validate.rs`, `source_map.rs` | `LifecycleParseOptions` injects the effect catalog and `UnknownVerbPolicy` (`Reject`, Claudine's; `Ignore` keeps an unknown verb as an opaque `LifecycleActionKind::Ignored { verb, path }` whose operand is never read) |
| Authoring errors | `error.rs` | `LifecycleError` (26 variants). Claudine maps each to its same-named `CompositionError` twin with identical `Display` |
| Typed shell templates, presence decision | `shell_template.rs` | `placeholder_guard`, `lifecycle_declared_types`, `walk_scoped` (in `actions.rs`) are reviewed sites in Claudine's `type_scope_ownership_guard.rs`, which also scans this directory |
| `err`/`timing` envelope | `envelope.rs`, `context.rs` | `DiagnosticSnapshot` data and `LifecycleErrorInfo`; classification is the host's `DiagnosticCatalog` |
| Services | `effects.rs`, `services.rs` | `LifecycleEffects` (text, message, speech, effect, notification; no TTS/messaging config), `LifecycleServices`, `MissingServiceError` |
| Executor | `executor.rs`, `executor/{host,shell,outcome}.rs` | `StackExecutor` + `StackInputs`; host policy through `LifecycleHost`; shells through `ShellRunner` (`DisabledShellRunner` before `start`) |
| Run accounting | `run_guard.rs` | `RunAccounting` (once-only events, one terminal slot, shell lock/relock), `LifecycleRunGuard` over `LifecycleRunHost` |
| Routing and admission | `runtime.rs`, `control.rs` | `decide_lifecycle_transition`, `LifecycleCatchProtocol`, `EvaluationMode` (normal recovery, fatal-evaluation, refusal, cancellation cleanup), `ContainerRunner`, `decide_control` |
| Request/reply driver | `run.rs`, `run/protocol.rs` | `Run::{start, pending, reply, cancel, shell_permission}`; tickets bind run/document/attempt/iteration/generation; a stale, duplicate, foreign, wrong-kind, or post-cancel reply is a `ReplyError` and changes nothing |

## Rules

- Darkmatter never depends on Claudine: `darkmatter/lib/tests/l1/dependency_direction_guard.rs`
  walks the lib, CLI, and DMLS manifests (following path dependencies) and
  scans their sources.
- One implementation: `LifecycleRunGuard` and `Run` share `RunAccounting` and
  the routing functions. Do not add a second state machine.
- Production behavior is authoritative over the spec's section 8. The
  recorded traces (`claudine/cli/tests/fixtures/lifecycle_traces/*.trace`)
  replay through `Run` in `darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs`;
  never edit a `.trace` to make a change pass.
- `lifecycle_boundary/{engine,profiles,publisher}.rs` are a test-only
  prototype of the host registry, kept until the registry exists.

## Planned

The host registry (host event names, outcome pairs, positions, roles),
the `md compose` host, and Claudine's drivers running on `Run` are not built
yet. Events are the fixed `LifecycleSignal` set.
