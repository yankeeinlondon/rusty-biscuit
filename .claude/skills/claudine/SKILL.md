---
name: claudine
description: Use when working in the claudine/ package area or with the Claudine library/CLI — normalizing agentic-CLI lifecycle events and hooks, wrapping providers (Claude Code, Codex, Gemini, Goose, Kimi, OpenCode, Qwen, Kilo, Pi, Antigravity), composing Markdown prompts (compose/inline-compose/sequence), managing the MCP catalog, linking skills/commands/agents across providers, or researching agentic CLI platform behavior.
---

## Overview

Claudine is a universal event handler, shared-resource linker, MCP catalog manager, and composition harness for agentic CLIs. It normalizes **16 lifecycle events** across **10 providers** (Claude Code, Codex CLI, Gemini CLI, Goose, Kimi Code, OpenCode, Qwen Code, Kilo, Pi, and Antigravity) into a single configuration model, fires **6 action types** — TTS, sound effects, logging, shell commands, reports, and blocking calls — when those events fire, synchronizes skills/commands/agents/scripts between providers, manages a provider-agnostic MCP catalog plus provider-specific import/sync/runtime behavior, and provides three Markdown composition commands (`compose`, `inline-compose`, `sequence`) that flow through the same wrapper-grade execution pipeline as the provider wrappers.

Lifecycle `say`/`speak`/`effect` and hook `speak`/`sound_effect` audio is
fire-and-forget after durable publication to Playa's private per-user queue.
Lifecycle audio is published in stack order: put `effect` before `say` to hear it first.

**TTS pre-warm** (`lifecycle/prewarm/`) prepares early `say`/`speak`
phrases ahead of their events. Each top-level command (compose,
inline-compose, sequence, provider wrapper) owns one deferred
`PrewarmWorker`. The worker starts on the first submission that speaks and
is cancelled on drop. Its gates read only the user's bridged settings
(`tts_prewarm`); each submission also carries the settings of the speech path
that plays it, and each candidate is prepared and keyed (`PrewarmKey`) with
that path's `TtsConfig`. Document and container lifecycles play with the user's settings.
Internal legacy task stages retain `sequence::task::task_playback_settings()`;
those model fields do not authorize retired task YAML. Its `PrewarmHandle` travels on `InvocationContext`
(`with_prewarm`; `DocumentEpoch::prewarm`). Every preparation site calls
`prewarm::collect_and_submit`: the pipeline's lifecycle phase, the staged
bootstrap, the loop bootstrap, and harness re-materialization. Sequence
preflight calls `submit_sequence_preflight`; internal legacy task stages are
reconsidered by `submit_task_stacks` wherever the task's context is
established (`run_step_task`, `open_member_run`, `open_leg_run` in
`cli/src/commands/wrap/sequence/task_run.rs`), against the step's effective
frontmatter and the task's captured `ctx`/`env`. A sequence step or prompt task
scopes the handle with `with_prewarm_injected_keys` so per-step values stay
late. A new preparation site submits through the same entry point. CLI tests
observe preparations through the `test-fixtures` seam
`CLAUDINE_TEST_PREWARM=record:<path>` or `block:<path>`, with
`PLAYA_DRY_RUN=0` and only never-firing phrases (`cli/tests/l1/tts_prewarm.rs`).
Only phrases *ready early* are submitted: literal text, or reads of `fm`/bare
properties, `ctx`, `env` with no function call and no `$(`/backtick
(`prewarm/classify.rs`); late globals, sequence overlay keys, `--set` keys and
task `with` keys wait for their event. A lifecycle `set` changes runtime state,
not the reread, so a phrase reading a `set` key misses the cache (never wrong
audio). User docs: [Lifecycle → pre-warm](topics/flow-control/lifecycle.md#speech-is-prepared-ahead-of-time-pre-warm).

**`tts` in `~/.claudine/config.json`** (`config/tts.rs::TtsValue`) is `true`,
`false`, or an object whose keys are all optional: `provider` (absent = default
provider, as `true`), `voice` (string or `{ male, female }`), `gender`
(`female` default), `prewarm` (default `true`). Explicit `null` is rejected
(`present` deserializer), unknown keys are rejected. `bridge_tts_settings` maps
it to `GlobalSettings { tts, tts_prewarm }`; read the switch only through
`TtsValue::prewarm_enabled`.

| Value | Hook `speak` | Lifecycle `say` config | Pre-warm |
| --- | --- | --- | --- |
| absent / `false` | off | auto-detect | on |
| `true` | on, auto-detect | auto-detect | on |
| object with `provider` | on, object's provider/voice | object's provider/voice | `prewarm` (default on) |
| object without `provider` | on, as `true` | as `true` | `prewarm` (default on) |

Full table and validation rules: [Configuring Actions → `tts`](topics/configuring-actions.md#text-to-speech-settings-tts).

**Lifecycle grammar.** Each lifecycle stack (the seven events, the `loop:`
block's loose verbs or `gate:` list, and container events) parses
into ordered `LifecycleNode`s — a leaf `LifecycleAction` or a
`LifecycleConditional` (`when` + `then`/`else` bodies, at most five levels) —
and every node carries its authored `LifecyclePath`
(`failure[0].else[1].shell`), which is what diagnostics name. A dictionary
bundle expands in a fixed canonical order at parse time. Every retired spelling
(the old stack/aggregator keys, the speech-ordering flag, communication
`message`/`sound`/`route` parameters, and the retired loop-value names) is
`CompositionError::RemovedGrammar` with a replacement hint. The directive
verbs live in one list (`signatures.rs::LIFECYCLE_CONTROL_VERBS`); `prep` is
the one directive that returns (`LifecycleControlAction::returns`), so it is
exempt from the last-item rule. A loop `gate:` is never `null` or `{}`. The parser,
executor, run accounting, catch protocol, and control admission live in
`darkmatter::lifecycle` (see the `darkmatter` skill's `lifecycle.md`); Claudine
re-exports them at their old paths and adapts them: `effects.rs`
(`ClaudineLifecycleEffects`: TTS and messaging configuration), `executor.rs`
(`StackExecutionContext` is the `LifecycleHost`), `ClaudineRunHost` for the run
guard, the `*Ext` traits for the old constructors, and `CompositionError`
twins of `LifecycleError`. Event names come from the host registry:
`claudine_registry()` (`composition/lifecycle/registry.rs`) renames
Darkmatter's `composed`/`failed` to `start`/`blocked` (the Darkmatter spelling
is `LifecycleRenamedEvent`) and registers `success`/`failure` and `loop`;
derive event keys from `authored_event_keys()`, never a hand list. The host
globals `seq`, `loop`, and `group` are declared once in the host catalog
`claudine/schemas/globals.yaml` (embedded by `composition/schema/globals.rs`,
`claudine_globals()`/`host_global()`; never a trigger or document schema), and
`bindings.rs` and the registry take their names from it; Darkmatter's own
globals (`fm`, `ctx`, `current`, `env`, `err`, `timing`) are declared in
`darkmatter/schemas/darkmatter.yaml`, whose resolved `fm` is the built-in
baseline. Trigger files are `kind: schema-trigger`; a top-level `match:` list
is AND and `group:` is OR. Every
recovery verdict, refusal route, `blocked`-vs-`failure` choice, and hop budget
is decided by `darkmatter::lifecycle::policy` (`SubjectKind` carries the
document/root-sequence/sequence/group differences); a driver calls it and maps
refusals through `CompositionError::from_lifecycle_refusal`. Pre-launch checks
run through `composition/preflight/checks.rs` (`claudine_preflight()`,
`sequence_preflight()`): a `BeforeRun` failure fires no event, an `InRun`
failure is a preflight failure. The one-implementation guards,
`lib/tests/l1/lifecycle_one_implementation_guard.rs` (library source) and
`cli/tests/l1/lifecycle_one_implementation_guard.rs` (CLI source), share the
scan in `lib/tests/common/lifecycle_ownership_scan.rs` and fail on a Claudine
definition of a Darkmatter-owned lifecycle item, a `MAX_PROXY_HOPS`
comparison, a refusal or parser-twin error built outside its one mapping, or an
unreviewed seam `impl` or registration; a new adapter adds an allowlist entry,
with its reason, to the guard that scans its file. `lib/tests/l1/lifecycle_lift_ready_guard.rs` still
reads the moved slice, so the caller injects the effect catalog
(`LifecycleParseOptions`). Loop values are the host-declared `loop` global
(`loop.count`, …), supplied to Darkmatter main compose through
`ComposeOptions::with_host_globals`; `fm.loop` is the loop configuration.
`retry.with` shares `proxy.with`'s evaluation (Darkmatter `executor.rs::resolve_with_overlay`)
and travels as `StackControl::Retry.overlay` into
`HarnessPromptState.pending_retry_overlay`; `prompt.rs::active_overlay` layers
it over the retained proxy overlay (retry wins a shared key). The retried
attempt's preflight read only looks at it and `materialize_harness_prompt`
consumes it, so it applies to exactly one transition; resume and proxy
adoption clear it.

**One loop for every primitive.** `looping/frame.rs::LoopFrame` is the
nearest-loop frame, allocated before `initialize` (which reads pass 1's
values): cap precedence override > authored > 100, `is_last` is `count ==
max` only, and `decide` reads the condition before classifying the cap
(`LoopDecision::{Complete, Continue, Exhausted}`); actions stage only on
`Continue`. `record_pass` runs before a pass's terminal event (that event,
`finalize`, the gate, and the condition read the pass's own typed result:
text, sequence slice array, group dictionary), and `retry_pass` restores the
previous result for a retried attempt. The document engine
(`execute_loop_with_lifecycle`) and the containers (`sequence/task/sequence.rs`,
`group.rs`, CLI `wrap/sequence/container.rs::RootRun`) share it; containers
run `looping/container.rs::finish_container_pass` after `Finished { failed:
false }` and `ContainerRunner::begin_next_pass`. A looping container's retry
restores `PassCheckpoint` (mutations and outputs at pass entry); a
non-looping one keeps its old rule. Loop-level fail-fast is gone (no
`CLAUDINE_FAIL_FAST` read): a failed pass ends the loop before its gate.
Gate/condition/action failures are `LifecycleLoopGateFailed`/
`LoopEvaluationFailed`/`LoopActionWriteRefused` with the pass. A `prompt:`
task whose document declares `loop:` runs through
`prepare_and_run_active_document` (`task_run.rs::run_looping_document`), and
`PromptTaskRequest.loop_values`/`StepComposeContext.loop_values` carry an
enclosing container's `loop.*` into non-looping documents.

**`break`, `prep`, dry run.** A `break` records `LoopBreak { reason, code }`
in its nearest loop's `LoopBreakSlot` (`LoopAmbient.breaks`, shared by every
page whose innermost loop it is; `LoopFrame::pending_break`) and ends its
event like `stop`; outside every loop it is `LifecycleBreakWithoutLoop`.
Containers stop starting children while the slot is pending, `defuse()` the
guard, run `finalize` once, publish partial results; the owner ends with
`ContainerPassEnd::Broken`/`LoopExecutionResult::broken`. Group legs get
`with_isolated_breaks()` and the fold forwards the first succeeded leg's
break in declaration order; a failure or retry `discard()`s it. The code is
the exit status only for the root loop (`RootRun::broken_exit_code`;
`prep.rs` when `step_scope` is `None`). `PromptRunOutcome.withheld` keeps an
unlaunched document out of `seq.outputs`. `prep` is a returning action
(Darkmatter `executor.rs::run_prep`, via `LifecycleHost::prep`) through the thread-installed
`lifecycle::prep::PrepRunner` (`install`, `inherit` for group workers, call
chain `enter_call`, one cycle domain with proxies); the CLI runner
(`commands/prep_target.rs`, installed by compose/inline-compose/sequence)
re-enters `run_composition_inner`/`run_sequence_target` in process with the
overlay as data setters. A failed call is `LifecyclePrepFailed` (a dispatch
failure `no_error` tolerates). Not built: `kind: group` targets and static
approval inventory of `prep` targets. `--dry-run` renders `shell:` and
`side_effect:` tasks (`PrimaryOutcome::withheld`), shows guards unevaluated,
previews one pass with a frame for `loop.*` (`dry_run_loop_note`), and lists
literal `prep` targets (`LifecycleConfig::literal_prep_targets`);
`--dry-run-shell` (clap `requires = "dry_run"`) runs the shells once.

**Loop configuration.** Every owner's `loop:` parses through
`looping/config.rs`: `resolve_loop_config` for a document,
`parse_loop_block(value, LoopOwner, &LoopLocation)` for a group or sequence
(controls plus the gate, via `lifecycle::parse_loop_gate`). Errors are located
`LoopInvalid { source_path, property, message }` with `property` rooted at the
block (`tasks[2].loop.max`); `loop.fail_fast` is `RemovedGrammar`;
`on_rate_limit` is document-only (`LoopRateLimitPolicyNotAccepted`). A
templated `max` stays in `LoopConfig::max_template` until
`resolve_templated_loop_max` resolves it once (the engine refuses an
unresolved one). `check_loop_config_keys` runs before the launch's schema
verdict so key typos get the loop parser's did-you-mean.

The `claudine` executable intercepts Playa scheduler/delegate modes and the
biscuit-speaks preparation-helper mode before normal CLI bootstrap, so queued
audio is serialized and survives the requesting Claudine process.

Outbound messages and desktop notifications are **not** fire-and-forget. Each
send is registered with `messaging::delivery::track`, and every ordinary CLI
exit goes through `cli/src/shutdown.rs::finish`, which drains them (10 s, or
less under `handle`'s deadline) and warns about any whose delivery is unknown
without changing the exit code. Never add a bare spawn in `lib/src/messaging/`
or a direct `process::exit` in `cli/src`: guard tests fail on both. See
[Messaging](topics/messaging.md#delivery-tracking).

Every home read in `lib/src` and `cli/src` is `biscuit_file::home_dir()`,
written qualified (`HOME` on POSIX, `USERPROFILE` on native Windows, relative →
`None`); never `dirs::home_dir` or `std::env::home_dir`. Request-scoped policy
code takes a captured home (`PolicyContext::with_home_dir`,
`SensitivePathChecker::home_dir()`) instead of re-reading. A canonical path that
is returned, stored (MCP state keys, overlay `Entry.source`), rendered, or put in
a URL comes from `biscuit_file::canonicalize_simplified`. `tests/l1/path_lookup_guard.rs`
in lib and cli fails on any other direct home or canonicalize call, and a new
home read also needs its `context_construction_guard.rs` allowance. Because
`CliProcessFixture` sets `HOME` and `USERPROFILE`, user-tier and
`~/.claudine` fixtures run on Windows too: do not gate them `cfg(unix)` for the
home.

Audio tests must remain silent: real playback uses explicit zero volume and
recognizable test speech with a pinned provider/voice. A fake agent does not
suppress lifecycle audio. Tests of literal shipped prompts should set child-only
`PLAYA_DRY_RUN=1` and a private spool, then assert no audio was published.
Publication-only library/CLI tests share `test_toolkit::LockedAudioSpool` to
remove pending work under `queue.lock` before releasing the scheduler lock,
including unwinding. See `claudine/docs/topics/testing.md` for the fixture
boundaries.

The **10 providers** above are the compiled `Provider` enum (`PROVIDER_COUNT = 10`); the research roster (`claudine/docs/providers.yaml`) is currently in sync with it (Kilo, Pi, and Antigravity graduated through the Phase H provider ladder, 2026-07-08). The roster is designed to run *ahead* of the enum during onboarding (a new entry is active-but-unwired until its variant lands) and carries `skip_research: true` for keep-identity-but-pause deprecations. Roo Code support was removed in 2026-07; major provider version changes enter the roster as new entries rather than mutating existing ones (see "Major-version changes are new providers" in the spec below). The provider-metadata automation workstream — research-driven metadata, codegen, signal catalog, the model ground-truth catalog, and the unified dispatch guard — is **complete** (RATIFIED & IMPLEMENTED, `claudine/features/_completed/2026-07-02-provider-metadata/spec.md`); consult it before extending provider metadata by hand. Each `lib/src/provider/<slug>/data.rs` is GENERATED by `claudine-gen` (in `claudine/gen`, using shared vocab from the leaf `claudine/catalog-types` crate) from roster + facts + research + overrides (regenerate with `claudine providers generate`; drift-checked in CI) — only `behavior.rs` is hand-written. A single site-level drift guard (`claudine-cli/tests/l1/dispatch_inventory.rs`) prevents new decentralized `match Provider` dispatch from regrowing across **both** `lib/src` and `cli/src`.

The package follows the monorepo `lib` + `cli` split: library crate `claudine`, CLI crate `claudine-cli` (binary `claudine`). A third sub-crate, `claudine-contract` (`claudine/contract`), implements `biscuit_contract::inference::InferenceAdapter` by running a provider as a single non-interactive, **tool-free, filesystem-isolated** session and returning its final assistant text — letting deterministic consumers (Reaper, Darkmatter) delegate to an agentic CLI via `Arc<dyn InferenceAdapter>` without depending on `claudine` directly. It depends on `claudine` (lib) but **not** `claudine-cli`. v1 enables Claude and Codex; other providers are reported `Unsupported`. See `claudine/contract/README.md` for the provider support matrix and security posture, and the `biscuit-contract` skill for the contract itself.

Alongside these, `claudine/rendezvous/` is a first-class package-area family of **three crates** backing `claudine dashboard` and the (unwired) lifecycle `defer` scheduler, in a `core → {daemon, client}` shape: `rendezvous-core` (leaf — protobuf/gRPC stubs, identity, signed envelopes, sync wire framing, and the typed `LocalEndpoint`), `rendezvous-daemon` (the long-running service: gRPC over the platform's local endpoint, the `redb → Loro → DuckDB` session-log pipeline, register store, QUIC sync engine), and `rendezvous-client` (the portable `connect(&LocalEndpoint)` plus a thin gRPC test client). Both leaf crates depend on `rendezvous-core`, never on each other. Its area justfile (`cd claudine/rendezvous`) now carries the full canonical recipe set, each iterating all three crates: `build`, `check`, `sanity`, `test`, `doctest`, `coverage`, `lint`/`lint-fix`, `install`, plus reasoned "not applicable" stubs for `test-l2`, `test-l3`, `test-browser`, `bench` and `fuzz`. `test-real` is not a stub — it is the only route to `rendezvous-daemon::peer_discovery`'s two `real_*` mDNS identities, which every other recipe's tier filter excludes.

Ordinary local L1 leaves `claudine-cli/daemon-tests`,
`claudine-cli/terminal-tests`, and both contract/CLI `real-tests` disabled.
Required-feature targets keep DuckDB, L2/L3 integration binaries, and live
provider targets out of that compile graph. CI enables daemon and terminal
features; explicit tier recipes enable the corresponding local targets.

Run `just test`, `just lint`, and `just check-tier-coverage claudine`
sequentially. The tier audit uses `nextest list` with default features and can
overwrite the fixture-enabled `claudine` binary just as lint-transport can.
Concurrent listing can remove diagnostic snapshot seams while CLI tests run.

**The L1 spawn contract.** An L1 test obtains a `claudine` command from
`CliProcessFixture` in `claudine/cli/tests/common/mod.rs` — `command()` for the
hermetic default, `command_builder()` when it needs a named escape. The default
pins `current_dir` to the fixture `cwd`, points the home variables at the
fixture home, sets `CLAUDINE_RENDEZVOUS_REPORT=false` and `NO_COLOR=1`, silences
lifecycle audio with a child-local `PLAYA_DRY_RUN=1` plus a private
`PLAYA_SPOOL_DIR` (assert `fixture.audio_spool()` is still absent after running a
shipped prompt), and composes `PATH` as the fixture `bin` plus a minimal system set (`/usr/bin:/bin`;
`%SystemRoot%\System32` on Windows) — enough for the `sh`/`cmd`/`git` claudine
itself spawns by bare name, and short of every prefix an agentic CLI installs
into. Three escapes exist, each requiring a call-site comment: `fake_only_path()`
(nothing but the fixture stubs), `host_path()` (the old `augmented_path`), and
`ambient_context(dir)` (launch CWD pinned to a repository the test built inside
its own workspace — the rusty-biscuit checkout can never be inherited).
The two `PLAYA_*` defaults are not cosmetic: a lifecycle audio effect makes
claudine re-exec *itself* as playa's detached spool worker, which outlives the
command that enqueued the job, so without them an L1 test that composes such a
prompt leaks two `claudine` processes per run and plays a sound on the host.
`detached_audio.rs`, whose subject is that worker, is the one file that opts
back in.
To assert the exact child argv or the composed prompt (providers read it on
stdin), install a recording stub provider with `common::launch_recorder`
(`install`/`launches`/`prompts`, Unix-only) rather than writing another one.
A test whose subject *is* the running child — a signal, a deadline, a streaming
read, `CREATE_NEW_PROCESS_GROUP`, an `expectrl` session — uses `command_std()`
(or `command_builder()…build_std()`), which is the same policy on a
`std::process::Command`; `apply_policy_to(&mut cmd)` applies it to a command the
fixture did not build, for the one shape where claudine is a grandchild of a
shell. `claudine/cli/tests/l1/spawn_site_guard.rs` enforces it: a raw
`assert_cmd::Command::cargo_bin("claudine")` or a `claudine_bin()` shell-out
outside the builder fails the suite unless its file carries a reasoned
`SPAWN_ALLOWLIST` entry, and an entry matching no live site fails too. **That
list is empty** — every L1 binary goes through the builder — so any raw spawn is
now simply a failure. The same guard requires a test file that builds an emulator session
(`TmuxHarness`, `WezTermHarness`, …) to be named `level2_`/`level3_`; a
`real_` file is exempt, since `just test` already excludes it, so a
real-provider test may drive its provider's TUI in tmux
(`real/real_pi_interactive_startup.rs`, `real/real_native_interactive_startup.rs`). Running
the binary from the ambient CWD is both a cost (the 35-member workspace walk,
20–77 s per test on WSL2) and a correctness hazard (the checkout's git state and
root `system-prompt.md`, the developer's `$HOME`, the host's real provider
binaries). See the `rust-testing` skill's `spawning-binaries.md`.

Structured stdout cleanup now carries `ProcessResult.completion_observation`:
separate bounded 256 KiB answer/raw prefixes, coherent atomic reader/settlement
operations, and separate output-worker queue/active-delivery observations.
`RunScope::freeze` closes publication and moves buffers before projection;
late readers cannot mutate the frozen copy or dispatch into another run.
Claude output and Codex agent messages publish original text through
`SemanticEventSink::on_response_text` before their Reasoning callbacks; sink
wrappers must forward this nonblocking observation without treating it as a
verdict. The `test-fixtures` build alone supports fixed completion-stall
scenario directories and the compiled fake completion provider. Stdout
settlement selects `claudine_completion_delayed` for native exit 0
without a verdict, `exit_failure` for native failure, and `interrupted` for
exit 130; reader timeouts remain subordinate warnings. Published answer/session
data survives a native failure. Typed completion-delay diagnostics and populated
lifecycle transport remain planned: the frozen execution observation is not
yet a populated session diagnostic or author-visible `err.detail`. Do not
publish a runnable retained-response recovery example until that transport
and its sequence policy tests land. Direct structured wrappers project semantic
failure/native exit 0 to caller exit 1 while retaining native evidence.
The diagnostic field contract still awaits the author's CPU ruling. See
[Timeouts](topics/timeouts.md#completion-observations-and-retained-prefixes).

The **local control plane** is platform-native and per stable OS user: a Unix-domain socket on macOS/Linux/WSL, a Windows named pipe on native Windows, qualified by the effective UID or process-token SID from `sniff::os::current_user_id()` — never a username. One portable `spawn_local_server` binds it to a transport-neutral daemon built exactly once. Read `claudine/docs/rendezvous/local-ipc.md` before changing endpoint, daemon-boot, or connector behavior; see [architecture.md](architecture.md) → Rendezvous Package-Area Family for the crate roles, the local-IPC rules, and the `SessionLogManager` module boundary.

**Where to look next:**

- For library internals (module structure, key types, provider adapters, dispatch, stream parsing) → [architecture.md](architecture.md)
- For full per-command documentation → [cli-reference.md](cli-reference.md)
- For when/why a feature changed → [timeline.md](timeline.md)
- For a specific subsystem → the deep topic docs under [Reference Documents](#reference-documents)

## Container preflight and approval

`composition/sequence/preflight/` owns the transitive walk (not the CLI).
`containers.rs` prepares container event shells with the same site policy as
primitive tasks; terminal event placeholders require templates. The graph's
`root_lifecycle` is installed on the root guard after the audit. Lifecycle
clones share settled authorization; a reread graph is settled only against the
original invocation's approval, never by prompting. Root retries pass the same
closed membership into graph approval; rebuilding the graph cannot reopen approval,
and the retry runs the first attempt's graph (`RootRun::graph`). Nested preparation
(`prepare_nested_sequence`/`prepare_nested_group`) rereads container events and
`variables` but keeps the frozen node's children and order. Preserve normalized exact
command keys, including multiple shell-value commands. Catalog cycle identity
includes the selected group name, so distinct entries in one catalog are legal;
diagnostics retain each entry as `catalog#name`. First position propagates to
all legs of a first group and only the first child of a first sequence.

## Container grammar boundary

Sequence tasks accept an exclusive `sequence:` list or file reference (source
suffix grammar applies). Inline list tasks alone accept container lifecycle
keys. Nested inline and referenced sequences execute recursively; group legs
may themselves be containers. Groups reject `execution:` (concurrency is implicit), require
unique nonempty leg names, reject reserved variables `legs`, `succeeded`,
`failed`, `cancelled`, and retain boolean `fail_fast` only on group objects.
Task `setup:`/`teardown:` and sequence-document `fail_fast:` are removed
grammar. Container `loop:` (group object, sequence-document root, inline
nested sequence task) is validated by `sequence/grammar.rs::container_loop`
and frozen on `PreflightSequence`/`PreflightGroup.loop_config`
(`PreflightLoop`, config plus location; the gate goes into the container's
`loop` stack so preflight approves its shells); the root's is parsed by the
CLI (`root_loop_frame`). Task `when:` (beside an executable only; a body
step's is `WhenMisplaced`) is evaluated at the task's turn by
`sequence::task::evaluate_task_guard` (read-only, task-setup scope): nested
children and legs in `TaskExecution::run_guarded` (a leg in its worker,
against the group-start member state), a root step in
`iterate.rs::run_one_step` before it composes (`task_run.rs::root_step_guard`,
then `run_admitted`). False is `TaskOutcome.skipped` (null result, nothing
published); a raising guard is `TaskWhenEvaluationFailed`. Sequence-root
`when:` and `loop` on any other task are permanent errors. JSON-family sources reject duplicate keys in
`sequence/data.rs` (`DuplicateScan`) before the `serde_json::Value` conversion. The CLI rejects `--fail-fast` before source loading, keeps explicit
provider `--` tails opaque, and no longer injects `CLAUDINE_FAIL_FAST`.
`cancelled` is a cleanup-only lifecycle event. Presence, including `[]` or
`{}`, opts running group legs into sibling-failure cancellation; null is
invalid. Ctrl-C interrupts every running primitive regardless of opt-in.
Cancellation stops admission and settles descendants before owner cleanup;
an opted-in container cancels all descendants. Entered-but-not-started scopes
still clean up with shells disabled; unadmitted legs record cancelled without
events. Run `cancelled` then `finalize` exactly once before lexical restoration,
never `failure` recovery. `stop` ends only the cancellation stack. Recovery in
the following finalize raises the typed cancellation-recovery error without
launching work; cleanup errors stay secondary and preserve cancellation/130.
Loop initialization checks cancellation before catch controls, and cancelled
iterations terminate before handoffs, loop gates, or mutations. Timeouts remain failures; cleanup timeouts remain cleanup errors. Root sequence
Ctrl-C exits 130 on every OS; native Windows provider status `0xC000013A` is
normalized to 130. See [Sequences → Cancellation](topics/flow-control/sequences.md#cancellation).
Sibling cancellation uses `composition.cancelled` with `err.detail.group`,
`leg`, and the original structured failure under `cause`. Group worker threads
explicitly inherit `lifecycle::cancellation::CancellationToken` scopes. Keep
terminal execution on `LifecycleRunGuard::run_event_stack`: its cleanup scope
allows terminal actions to finish after cancellation while preserving repeat
interrupt grace. Lifecycle shell exact/template dispatch shares task shells'
owned process trees, except on Unix while Claudine holds its terminal's
foreground (`task/shell.rs::holds_terminal_foreground`): an inherited-terminal
child in a group of its own is stopped by `SIGTTIN`/`SIGTTOU` forever (a
nested `claudine compose` from `shell: just commit` hung this way), so it runs
in Claudine's group and only the direct child is terminated on cancellation.
Regression: `cli/tests/l1/level1_lifecycle_shell_terminal_pty.rs`.
Container events use
`lifecycle::runtime::ContainerRunner` and `LifecycleRunGuard`; root initialize
precedes static preflight, nested preparation is just in time. Retry preserves
mutations and clears child results, with no finalize until the final attempt.
Native sequence arrays and group dictionaries publish only after finalize.
Atomic failed groups withhold all leg mutations; tolerant groups merge failed
leg mutations in declaration order. Body-only steps mask source lifecycle keys
through data overrides so canonical rereads do not fire container events again. Internal legacy task-stage tests
construct preflight models directly; they do not authorize old YAML syntax.
The canonical shipped trigger schema lives in `claudine/schemas/`, not
Darkmatter's docs tree. See [Sequences](topics/flow-control/sequences.md).
The sequence trigger is `schemas/sequence.yaml` (matches any non-null
`sequence`; payload `sequence`, `template`, and `fail_fast: null` so a root
`fail_fast` is reported by name), and it is the only declaration of
`sequence`: `claudine.yaml` no longer declares it, and a sequence document's
events and `loop` are typed by `claudine.yaml`, which they select. The
container types live in `schemas/sequence-types.yaml`, not `lifecycle.yaml`.
SimplifiedSchema inlines named types (no recursion), so they are unrolled to
three typed levels (`task`/`group`, `-2`, `-3`; below that any list). Each
level inlines the next twice (task `sequence:` and group `tasks:`), so a
container field costs 4x per level: `cancelled: cancelled-stack` on levels
1–2 only, the shared `loop` type on level 1 only (object below), other events
shape-only. Fully typed container events and loops measured +3 s per
document (release `md`) and doubling `sequence` in both triggers +0.34 s per
process (debug); every trigger payload is loaded whenever any trigger loads,
so this cost hits every Claudine launch (see
[Schemas](topics/schemas.md#sequence-containers)). A group is closed but a
union with a file reference, so `execution`/`max_paralel` are reported at
`/…/group` with the value in the message; a top-level `sequence: 42` is
coerced to the string arm (schema valid; the parser rejects it). The
`cancelled-*` family is unrolled to the parser's `MAX_CONDITIONAL_DEPTH` (5)
with no conditional at level 6; its conditionals use one
`<pattern::^(?:then|else)$>` key, since literal `then`+`else` inline each body
twice and grow the schema 2^n (measured 10x per validation at five levels).
DMLS neither completes pattern keys nor resolves values below them, which is
why `initialize-*` keeps literal keys and three typed levels.

## Per-task provider selection

A sequence has no single planned target. Preflight predicts each `prompt:`
task's provider from its own `agent` (`PreflightTask.provider`, computed in
`composition/sequence/preflight/providers.rs` without running or composing
anything) and keys every task by its graph `TaskIdentity`. The CLI
(`wrap/sequence/providers.rs`) turns predictions into per-task targets
(own / enclosing-document fallback / decided at launch / unresolved), fails an
unusable static hint with `SequenceTaskProviderSelection` before any task
starts, and gates only on unresolved tasks (review on a terminal, else
`AgentResolutionFailed`). `WrapperPromptRunner::document_target` prepares
each task under its own target; `launch_target` mirrors the rebuild's rule over
the document's effective hints, and `jit::compose_step_selecting` calls it
twice: on a frontmatter-only read (`composition::read_selection_hints`, no
shells) so frontmatter values see the choice, and inside the one full
preparation (`prepare_{direct,inline}_with_schema_selecting` →
`ResolvedIdentitySelector`, over Darkmatter's
`Markdown::compose_with_frontmatter_hook`) after frontmatter `$(…)` runs and
before the body renders, so the prompt's `env.AGENT`/`env.MODEL`, the child
environment, and the spawn agree with each command run once. The selected
overrides replace `CallerInputLayers.env_overrides`, so a retry/resume reread
keeps them. The launch rebuild
(`select_rebuilt_provider`) stays authoritative, and status lines read the
spawned attempts from the run's `LaunchLog`, never the plan. Review rows map
back by identity; read-only rows use `CellValue::StaticText` in an editable
`InputTable` column. User docs: [`claudine sequence` → Choosing providers](../../../claudine/docs/cli/sequence.md#choosing-providers).

## Lexical sequence bindings

`lifecycle/bindings/containers.rs` owns fixture-testable `SequenceBinding`,
`GroupBinding`, and named leg outcomes. `seq`, `group`, and `loop` shadow
complete roots. `seq` replaces generated frontmatter overlays; authored
`fm.seq`/`fm.group`/`fm.loop` remain ordinary data. `LayeredOverrides` carries
its `sequence_value` separately, and `PrepareOptions`/`CallerInputLayers`
retain `sequence_binding` across rereads and proxy adoption. Never flatten
that channel back into setters or live frontmatter. `RuntimeSnapshot.seq`
contains the active position; `sequence_value()` adds current committed
outputs. The loop's condition and lifecycle gate read that same cell.

`bindings/rename.rs` rejects parsed legacy root reads (including bracket
access and both condition branches), preserving authored/supplied properties
by presence, including null. Composition host globals retain a typed renamed
root fallback for reads introduced by transclusion; Darkmatter's explicit
`with_document_fallbacks` lets the reading child declare its own property.
`seq` string presentation uses the existing name-coercion keys for `seq`,
`seq.previous`, and `seq.next`; typed expression evaluation retains objects.
Sequence task templates defer `seq`/`group`/`loop`, including the first task.
Container runners use complete lexical roots for their own events and restore
the enclosing roots on exit. `RuntimeState::replace_sequence_scope` restores
position and outputs while keeping mutations; group workers clone the snapshot
so in-memory writes and proxy ancestry remain isolated across branches.

## Structured shell authorization

Document, lifecycle and sequence shells share Darkmatter's typed template
filler. An exact approval fixes resolved bytes; a template approval permits
only the authored shape. Never authorize newly filled bytes to bridge modes.
Sequence graph discovery alone grants no execution: `resolve_graph_shell_approvals`
records the audited `ShellPreApprovals` on `PreflightGraph`, including referenced
prompt lifecycle stacks through passive frontmatter-only preparation; task dispatch
checks membership before filling. Later tasks retain authored text, never
interim interpolated shell strings. Byte-eligible shell candidates retain their
`authored_command` even when template grammar fails: authored program holes
withdraw prefix approval, and `prompt_resolved_site` rejects unoffered grants.
Lifecycle resolution preserves that authored text across repeated resolution.
`task/templates.rs` installs approved
lifecycle stack sites and fills primary commands against current runtime
mutations. A prompt task's passive barrier and JIT composition share the
same document epoch (`PromptTaskRequest::document_epoch`), so context capture
and epoch accounting stay coherent. Referenced prompt lifecycle `shell` and whole-value `set` sites retain
`PrepareOptions::deferred_shell_roots` and lexical `lifecycle_group`; the passive
`validate_document_shell_late_values` probe returns their condition-blind lifecycle
for approval before start. `DocumentShellValues` keys freeze their late holes
for JIT preparation as well as document directives. Deferred holes use the step-entry snapshot
before start; ordinary holes read at command entry. `TaskShellRunner::run_structured` refuses by
default; `SystemTaskShell` shares the existing owned-tree process epilogue
with exact execution. Whole-chain policy and direct-target validation precede
the first spawn. Shared redirection targets identify original destinations:
`2>&1 > /dev/null` retains stderr on original stdout; never collapse it to
stderr's own capture channel. Native `native_task_` and `native_lifecycle_`
CLI regressions prove routing and downstream output on every OS. See [Pre-flight Checks](topics/pre-flight-checks.md).

Static presence decisions read Darkmatter's shared `TypeScope`.
`lib/tests/l1/type_scope_ownership_guard.rs` fails when a
`TypeScope::unguarded`, `DeclaredTypes::new`, `assume_at`, or presence lookup
appears outside its reasoned allowlist entry, or when an entry is stale. It also
fails when preflight refuses a `MissingValue` without asking `placeholder_guard`.
A new host site adds an entry with its reason. Runtime `is_null` checks are not
restricted. The guard/site conformance matrix is
`composition/lifecycle/executor/tests/scope_site_conformance.rs` plus its
sequence sibling in `sequence/preflight/tests.rs`. A guard fact is pre-flight
evidence only: it never crosses an event, loop pass, retry, resume, or proxy
handoff, and the live value is always checked again at the command. User docs:
[Lifecycle → What a guard proves](topics/flow-control/lifecycle.md#what-a-guard-proves-to-the-actions-it-selects)
and [Sequences → Task guards](topics/flow-control/sequences.md#task-guards-when).

Late roots are `seq`, `loop`, and `group`; sequence and container terminal
event shells participate in the same closed graph audit. Both group schedulers
validate all members before member start, retaining `LateValues`, prompt
`DocumentShellValues`, epoch/context, and lexical group scope through dispatch.
First-task commands with known values support exact or shape approval; terminal
event placeholders require templates. Membership is checked before filling and
before policy evaluation, so `--yolo` cannot authorize a command introduced by a
reread after execution begins. The shell runtime rejection chain omits rejected
values; earlier document schema validation can disclose them. Approval is
separate from result caching: current membership, validators, blacklists, and
direct targets still apply.

## Initialization shell boundary

Loop initialization uses the full bootstrap frontmatter separately from the
control-variable seed. Share its live state with catch handlers and retain
`set` writes in the invocation RuntimeState for subsequent preparations.
Mapping `set` destinations absent from the pre-write snapshot are known null
bindings; this permits copying an optional value before resetting it. Existing
values still win over those null defaults, and any other absent property reads
as `null`.

`initialize` runs before preflight and is shell-free. Reject shell actions and
`set` values written as a whole-value `$( … )` even in dead branches
(`parse.rs::runs_a_shell`), and reject bootstrap frontmatter `$(...)` expansion; approval
flags, whitelists, caches, and handlers cannot grant an exception. Non-shell
effects keep their existing restrictions. Early blocked/failure/finalize chains
cannot execute shells either, including catch evaluation-error routes, and
`no_error` cannot suppress the prohibition. The shared runtime permits lifecycle
shells only from `start`, after preflight, and proxy adoption resets that boundary.
Keep parser rejection and the runtime backstop together (`DisabledShellRunner`
refuses `run`, `run_values`, and `run_structured` alike, and its
`ensure_permitted` answers before a template site reads any placeholder). See the binding ruling
in `claudine/fixes/2026-09-15-initialize-after-proxy/spec.md` (R2).

## Library Module Map

The `claudine` library's modules and what each owns are in
[module-map.md](module-map.md).

## CLI Commands

The `claudine` binary provides interactive setup, hook inspection, event handling, shared-resource management, MCP management, log reporting, provider wrapping, and Markdown composition. Output flows through a structured logging system that separates pipeable data (stdout) from status messages (stderr), formatted with biscuit-terminal components. Provider names accept fuzzy matching (exact → prefix → contains), so `cl` resolves to `claude`. Full reference: [cli-reference.md](cli-reference.md).

Every command group and the cross-cutting behaviors (schema validation,
`match()` completion walks, retry/resume, shell pre-flight, dry run, protect)
are in [cli-commands.md](cli-commands.md); read it before changing a
command or one of those behaviors.

## MCP Support

Claudine stores normalized MCP data in `~/.claudine/mcp/catalog.json`, `~/.claudine/mcp/defaults.json`, and `~/.claudine/mcp/provider-state.json`, with optional repo defaults in `<repo>/.claudine/mcp.json` (repo defaults replace user defaults).

Provider rollout:

- **Import and sync:** Claude, Codex, Gemini, OpenCode
- **Runtime wrapper injection:** Codex, Gemini, OpenCode, Kilo
- **No MCP support yet:** Goose, Kimi, Qwen

Wrapper behavior: `--mcp` launches with effective defaults; `--use id-or-alias[,...]` adds explicit servers and enables MCP mode. Codex/Gemini inject into a per-launch provider overlay under `~/.claudine/overlays`, selected through `CODEX_HOME`/`GEMINI_CLI_HOME` — never by changing `HOME`, so nested tools keep the user's identity; Codex keeps SQLite state at its pre-overlay native `sqlite_home`/`CODEX_SQLITE_HOME`/`CODEX_HOME` destination so database and WAL/SHM files are never linked into the overlay. OpenCode and Kilo use `OPENCODE_CONFIG_CONTENT` and `KILO_CONFIG_CONTENT`. Claude, Goose, Kimi, and Qwen wrappers direct users to `claudine mcp export <provider> --apply` instead. Read [MCP Catalog](topics/mcp-catalog.md) and [MCP Mode](topics/mcp-mode.md) before changing MCP behavior.

## Reference Documents

### Skill docs (in this directory)

- [Architecture](architecture.md) — module structure, event matrix, key types, provider adapters, dispatch, stream parsing, composition, linking, exit codes
- [CLI Reference](cli-reference.md) — full per-command documentation, flags, output system, exit codes, file locations, env vars
- [Change Timeline](timeline.md) — condensed history of significant features and refactors
- [Supported Platforms](supported-platforms.md)
- [Unified Hook/Event Model](unified-hooks.md)
- [Supported Actions](hook-actions.md)
- [Linking Strategy](linking-strategy.md)
- [Non-Portable Assets](non-portable-assets.md) — summary; full reference in [topics/non-portable-assets.md](topics/non-portable-assets.md)
- [PolicyEngine](policy-engine.md) — summary; full reference in [topics/policy-engine.md](topics/policy-engine.md)
- [Validations and Handlers → Lifecycle Stacks](validations-and-handlers.md) — the retired validation/handler DSL and its lifecycle-stack replacement
- [OpenCode Event Sources](opencode-event-sources.md) — Dual-Source Contract, stderr promotion table, watchdog interaction
- [Research Contracts and Fleets](research-contracts.md) — the contract standard, the two gates, `just research`, the schema grammar's limits, and the traps found running fleets

### Claudine topic docs

`topics/` is a symlink to `claudine/docs/topics/`, so these are the
authoritative docs, not copies. Edit them there; never add a skill-local copy
of a topic doc.

- [Error Architecture](topics/error-architecture.md) — the `Diagnostic`/`BlockError` pair, the discovery seam, effective-diagnostic selection and the `Semantic`/`Transparent` role contract, the snapshot boundary, typed-wrapper rules, and the lossy-boundary audit
- [Composition](topics/composition.md) — `compose`, `inline-compose`, `sequence`, lifecycle stacks, provider selection
- [Schemas](topics/schemas.md) — the seven `claudine/schemas/` files (including the `sequence.yaml` trigger and the `sequence-types.yaml` container types), event/action-type permissions, what only the parser checks, disk discovery vs the embedded bundle, and DMLS's disk-only use for diagnostics, completion, and hover; the strict launch policy is in [Composition](topics/composition.md#claudines-trigger-schemas-in-the-launch-verdict)
- [Lifecycle](topics/flow-control/lifecycle.md) — the seven events, the list/dictionary stack grammar with `when`/`then`/`else`, action forms, `err`/`timing`/`current` globals, validation errors
- [Flow Control](topics/flow-control/flow-control.md) and [Flow Control Reference](topics/flow-control/flow-control-reference.md) — the `stop`/`skip`/`error`/`retry`/`resume`/`proxy`/`defer` directives, per-event runtime limits, retry/resume re-entry, `proxy` handoffs and the `with:` overlay
- [Timeouts](topics/timeouts.md) — the two timeout rules, four env vars, precedence, termination path, the post-exit stream-reader wait (`stream_reader_timeout`), the single output worker that keeps a blocked terminal from stalling the parser or the wrapper (`wrap/output_worker.rs`; run output is a queued frame, and once a run creates the worker every diagnostic — `crate::log`, signal notices, `--perf`, tracing, the panic hook, the library's `render::console` lines — is one too, through `crate::terminal_gate::write`; signal handlers never write the terminal), exit reasons, and the three runaway content guards
- [Signal Handling](topics/signal-handling.md) — user Ctrl+C vs wrapper-driven SIGTERM/SIGKILL, the unified wait loop, termination labels (`Aborted`/`Interrupted`/`TimedOut`), non-interactive ladder, Windows parity
- [System Prompt](topics/system-prompt.md) — launch-context discovery, `--append`/`--replace`, Darkmatter preparation, per-provider delivery
- [MCP Catalog](topics/mcp-catalog.md) and [MCP Mode](topics/mcp-mode.md)
- [Protect Service](topics/protect-service.md) — deny catalog, scan surfaces, rule groups, merge semantics, dispatch integration
- [Messaging](topics/messaging.md) — outbound routes (Discord/Slack/Signal/WhatsApp), the config-TUI route manager, webhook redaction invariants, the desktop-notification boundary
- [Traces and Logging](topics/traces-and-logging.md), [Log Reporting](topics/log-reporting.md)
- [CLI Pre-Parsing](topics/cli-pre-parsing.md) — argv normalization pipeline; rule-by-rule reference in [argv-normalization.md](topics/argv-normalization.md)
- [Shell Completions](topics/completions/shell-completions.md) — dynamic completion engine, per-mode pipelines, shared `@`/`&`/`^`/implicit file-reference resolution, and completion after the file through type-aware ownership (`composition::owner_of_last_argument`; nothing for the agent's words, never a prompt)

Research fleet entrypoints (`docs/research/<topic>/_fleet.md`) are sequence
coordinators. Shared formal task rosters add a prompt executable to each
canonical provider/runner row; parity tests keep their data and order aligned.
Per-row `_research.md` documents own freshness checks and success validation.
See [Research Contracts](research-contracts.md) before changing this workflow.

## Research on Agentic CLI Platforms

Cross-provider summaries, per-provider hooks research, ACP support, and the
schema-enforced topic fleets are indexed in
[research-index.md](research-index.md).
