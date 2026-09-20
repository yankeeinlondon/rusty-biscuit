---
total_phases: 10
created: 2026-08-31
phase: 1
agent: codex/default
yolo: true
source_spec: reviews/2026-08-12-cli-or-lib/spec.md
---

# Claudine CLI ↔ Library Boundary Execution Plan

This plan turns the boundary review into independently landable migrations. The governing rule is: the `claudine` library owns typed domain state, provider behavior, parsing, validation, policy, and deterministic orchestration; `claudine-cli` owns clap, operator interaction, terminal channel selection, process-global mutation, provider process mechanics, and final emission. `claudine-contract` reuses explicit low-level plans without inheriting permissive CLI defaults.

## Success criteria

- `claudine-cli` contains no duplicated provider or domain decision tables; retained code is limited to presentation, protocol adapters, operator interaction, and process edges.
- `claudine-contract` uses shared provider and launch primitives while preserving its environment allowlist, isolated HOME/CWD, tool and MCP denial, provider allowlist, post-hoc event rejection, and fail-closed support matrix.
- Every provider-varying launch behavior is reachable through `ProviderInfo`; generated `provider/*/data.rs` files are changed only through `claudine-gen`.
- Every moved behavior has characterization and parity coverage for applicable values, diagnostics, warnings, argv, environment, CWD, artifacts, output routing, exit status, and persisted mutations.
- Old CLI implementations, copied test oracles, obsolete dispatch allowlist entries, and string-keyed provider tables are removed in the same child workstream that introduces their replacement.
- Claudine's canonical nextest, generator, lint, and applicable L2 gates pass on macOS, with Windows and Linux behavior verified in CI.

## Dependency and parallelization map

- Phase 1 precedes every code change and creates dated child specs with exact acceptance criteria and `depends-on` links.
- Phase 2 fixes defects and guard gaps. D3 must land before completion extraction; D6 before handoff extraction; D7 before or with hook-plan extraction; D10 before or with stream-policy extraction.
- Phase 3 establishes low-risk shared primitives used by later migrations.
- Phase 4 is the provider-behavior keystone and depends on Phases 2–3. Its launch-planning surface unlocks Phases 5–6 and contract de-forking in Phase 9.
- Phases 5 and 6 may run in parallel after Phase 4 when worktrees and file ownership are separated; both must converge before Phase 9.
- Phase 7 contains independent admin/resource/config lanes and may run in parallel with Phases 3–6 after Phase 2.
- Phase 8 contains independent infrastructure lanes; completion extraction depends on D3 and the shared discovery/argv primitives, while error taxonomy and perf extraction can proceed independently.
- Phase 9 starts after the shared provider, launch, lifecycle, and composition contracts are stable. Phase 10 starts only after all prior phases are merged.

## Phase 1: Establish the Program Baseline and Child Specifications

**Goal:** make the umbrella review executable as small, dependency-aware changes with observable baselines.

- [ ] Re-verify every cited finding (P1–P6, L1–L14, E1–E6, C1–C12, A1–A9, I1–I6, and D1–D15) against the current tree; record moved code, already-fixed defects, obsolete claims, and the current owning symbols before scheduling implementation.
- [ ] Create dated `features/` or `fixes/` child directories for each independently landable migration, with `sub-spec: true`, stable finding IDs, explicit acceptance criteria, intended library home, retained CLI boundary, deletion targets, and `depends-on` links; split the large P1, L1, L4, C1/C6, I1, and I2 workstreams further when one reviewable change cannot satisfy its own checkpoint.
- [ ] For every child spec, list the production symbols to be edited and require upstream GitNexus impact analysis before editing; record direct callers, affected execution flows, and risk, and pause that child workstream for review on HIGH or CRITICAL impact.
- [ ] Capture `detect_changes({scope: "compare", base_ref: "main"})`, `git status --short`, and the current dispatch-inventory/catalog-drift results so unrelated user changes and pre-existing drift remain distinguishable from this program.
- [ ] Run the package baseline from `claudine/` using the available canonical recipes (`just check`, `just build`, `just lint`, `just test`, and `just test-gen` where present); record pre-existing failures and host-capability skips without changing production code.
- [ ] Build a parity matrix for each launch provider and each affected execution mode: interactive/non-interactive, direct/resume, output format, model, sandbox/YOLO, system prompt, MCP, retry/resume/proxy, and contract mode; map every branch to an existing test or a characterization test required by its child spec.
- [ ] Document the fixed ownership constraints in every applicable child spec: typed `thiserror` diagnostics replace CLI `color-eyre` reports, warnings return as data, paths/env remain `PathBuf`/`OsString`, ambient environment/clock/filesystem reads are injected, and terminal writes or prompts stay at the CLI edge.
- [ ] Checkpoint: review the child-spec graph and baseline evidence; Phase 1 is complete only when every review finding has one owning child spec, an observable acceptance test, a deletion target, and an unambiguous dependency path.

## Phase 2: Fix Defects and Close Structural Guard Gaps

**Goal:** prevent later migrations from preserving known bad behavior or missing decentralized provider dispatch.

- [ ] Add characterization tests, then fix D1 by introducing one complete TOML basic-string encoder used by system-prompt delivery and contract launch planning; cover quotes, backslashes, control characters, newlines, the Codex 64 KiB limit, and Windows/Unix argument cases.
- [ ] Add regression tests, then fix D2, D4, D5, D11, D12, and D13: prevent duplicate repo-config saves, derive telemetry CWD/provider identity from resolved domain data, parenthesize native-exit classification correctly, centralize worktree-aware Git-root discovery, recognize `.json5` user config, and make uninstall provider selection plus config backup symmetric with install.
- [ ] Fix D3 before I2 by replacing both stale skill-peer directory lists with catalog-derived provider resource paths; assert Kilo, Pi, and Antigravity coverage and deterministic deduplication.
- [ ] Fix D6 before L6/C1 by making every proxy hop resolve from the currently adopted target context; add a multi-hop sequence regression proving hop 2 is relative to hop 1.
- [ ] Fix D7 before or within A1 by making dry-run and wet-run hook sync consume the same `registerable_events()` expectation; pin Codex behavior.
- [ ] Fix D10 before or within L13 by routing semantic-to-hook mapping through one library function and adding exhaustive semantic-variant coverage.
- [ ] Fold D9 and D15 into the install-plan workstream: characterize useful legacy fixtures, deliberately select one shipping policy, and forbid treating the test-only init fork as a production defect fix.
- [ ] Extend `cli/tests/dispatch_inventory.rs` to scan `contract/src` in addition to `lib/src` and `cli/src`; make current contract dispatch debt explicit through a scoped temporary allowlist that Phase 9 must delete.
- [ ] Replace known provider string tables (`agent_offset` directory matches, skill roots, telemetry provider names) with typed catalog projections or focused typed descriptors; add parity tests for unavoidable wire strings rather than a generic string heuristic.
- [ ] Checkpoint: run targeted nextest suites for every fixed defect, `cli/tests/dispatch_inventory.rs`, `just test`, `just test-gen`, and `just lint`; require all new regression tests to fail on the pre-fix behavior and pass on the corrected behavior.

## Phase 3: Extract Shared Low-Risk Domain Primitives

**Goal:** establish deterministic library seams needed by the provider, lifecycle, composition, and infrastructure migrations.

- [ ] Extract P2 and P3 into `claudine::system_prompt::delivery`, `claudine::provider::prompt_args`, and `claudine::opencode_config`: return typed delivery/selection plans, share the TOML encoder from Phase 2, preserve artifact path types, and keep status markup in the CLI.
- [ ] Extract P4–P6: put resume-forwarded flags and resume support in provider metadata/typed errors, move explicit native-output detection beside the provider output catalog, and replace title-casing helpers with `ProviderInfo.display_name`.
- [ ] Extract L8(a,c), L13, C2, C3, C6's `PreflightTask::expression_scan`, C7, L5, L12(a), E1, I5, and I6 into their specified library homes; migrate tests with each subject and delete the CLI copies.
- [ ] Make `ComposeContext` construction one public library entry point and route all nine current sites plus system-prompt preparation through it; prove requirements/evidence/snapshot/env override ordering is identical for compose, sequence, retry, and proxy entry.
- [ ] Add `SchemaStage::for_document`, `SessionCompatibilityKey::from_launch`, `ApprovalMode`, timeout precedence/conflict APIs, runaway configuration resolution, and canonical secret admission/redaction with table-driven typed tests.
- [ ] Extract E3, E5, E6, C9, C10, C11, and C12 where independent: centralize launch-workspace/package context, scoped temp directory and `.gitignore` maintenance, interrupt ownership, sequence source/admissibility, compose setters/contracts/dry-run facts, hook provider/session-signal rules, and diagnostic catalog projection.
- [ ] Keep the 15-second handle deadline/exit-124 protocol, process-global signal registration, terminal rendering, and shell/completion protocol adapters in `claudine-cli`; verify new library APIs return data or decisions rather than writing to a terminal.
- [ ] Checkpoint: run targeted library/CLI nextest suites, `just test`, and `just lint`; use `rg` and dispatch inventory to prove each extracted policy has one implementation and no copied oracle remains.

## Phase 4: Build the Provider Behavior and Launch-Planning Keystone

**Goal:** make the provider catalog the single authority for argv, environment, prompt, resume, output, sandbox, and launch behavior.

- [ ] Add `claudine::provider::wrapper::WrapperBehavior` as a focused fifth behavior facet on `ProviderInfo`; implement it on each provider's hand-written ZST in `provider/<slug>/behavior.rs` and update `claudine-gen` to emit only the field binding in generated `data.rs`.
- [ ] Move `PromptDelivery`, `PromptSource`, `YoloOutcome`, `SystemPromptApplication`, and other shared launch vocabulary into the library; delete the CLI-local output-format enum and use `claudine::composition::OutputFormat` everywhere.
- [ ] Implement `claudine::composition::launch::assemble_argv` returning `AssembledArgv { args, env_patch, artifacts, warnings }`; preserve `OsString`/`PathBuf`, return typed diagnostics, and keep Prose markup and emission in the CLI.
- [ ] Characterize and migrate every provider branch in the CLI-private wrapper profile across direct/resume, model, output format, sandbox, YOLO, entrypoint, prompt delivery, captured output, allowed environment keys, and stdout-noise behavior.
- [ ] Delete `WRAPPER_REGISTRY`, provider-profile dispatch exemptions, and behavior-bearing compatibility facades; make catalog completeness, generator drift, and dispatch inventory fail when a provider lacks wrapper behavior.
- [ ] Extract L4 launch rebuild/launch plan and C8 MCP session planning on top of the provider behavior seam; inject ambiguity resolution, preserve retry-proof recorded choices and `OPENCODE_CONFIG_CONTENT` merge order, and materialize shadow HOME whenever the shared policy requires it.
- [ ] Extract E2 shadow-HOME and E4 child-environment planning using typed provider metadata and explicit policy inputs; preserve Codex SQLite/WAL behavior, provider exclusions, credential re-admission rules, and contract-safe lower-level entry points.
- [ ] Add all-provider golden/parity tests for argv, env patches, CWD, artifacts, warnings, resume identity, MCP planning, and non-UTF-8 path/env inputs; include Windows and Unix fixtures without invoking real providers.
- [ ] Checkpoint: run provider, generator, launch-plan, environment, MCP, repo-home, and contract characterization suites; then run `just test`, `just test-gen`, and `just lint`. Do not begin engine migration until no provider behavior remains in a CLI dispatch table.

## Phase 5: Move Lifecycle, Handoff, Termination, and Stream Policy

**Goal:** provide reusable deterministic lifecycle drivers while leaving spawn, wait, signals, channels, and final emission at the CLI edge.

- [ ] Extract L3's active-document read model, including fresh re-read, overlay precedence, and pre-resume MCP tag capture; test authored, initialized, retried, resumed, and proxied entry reasons.
- [ ] Extract L2 transition dispatch into one exhaustive `apply_transition` that owns budget, hop/cycle, refusal, and provenance rules; replace all three CLI match tables and their `unreachable!` invariant.
- [ ] Extract L6 and C1 into one coordinator/handoff driver with an injected `DocumentRunner`; prove direct compose and sequence steps share target resolution, overlay merge, adoption, ledger commit, and multi-hop behavior.
- [ ] Extract L7, L8(b), L9, L10, L11, L12(b), and L14 into typed termination, timeout tick, subagent progress, Kimi wire protocol, stream plumbing, guarded stream, section, inline-closure, and final-response policies; keep Unix/Windows process loops, signal handlers, Job Objects, and channel wiring in the CLI.
- [ ] Preserve and test the termination invariants: runaway/content guards map to `Aborted`, timeout precedence is stable, tool calls discard accumulated closing text, stream guard trips are terminal, and weak status updates cannot override waiting-for-user state.
- [ ] Implement L1 as `claudine::harness::loop_engine::run_attempt_loop` over a narrow `AttemptRunner`; move prepare/execute/classify ordering, retry/resume/proxy surfacing, budgets, and terminal-event ordering while the CLI adapter performs provider launch and rendering.
- [ ] Migrate lifecycle tests to terminal-free library tests and retain focused CLI adapter tests for stdout/stderr routing, exit codes, spawn/wait behavior, interrupt ladders, and final rendering.
- [ ] Checkpoint: run library lifecycle/stream/termination suites and CLI adapter suites, then `just test`; run `just test-l2` for streaming, signal, terminal, prompt, and child-process paths, ensuring no terminal or browser window gains focus.

## Phase 6: Move Composition, Schema, Target, and Sequence Drivers

**Goal:** make compose and sequence behavior reusable without moving pickers, clap, or terminal effects into the library.

- [ ] Extract C4 schema-interactive policy into a library driver over `MissingValueSource`; move number coercion, candidate filtering, retry ordering, unsupported-shape handling, override precedence, and D14 error conversion while the CLI supplies biscuit-tui values.
- [ ] Extract C5 provider/model target selection into `TargetVerdict`; pass operator presence and catalog-refresh capability explicitly, retain inquire/OSC8 rendering in the CLI, and prove direct, sequence, dry-run, and retry launch paths resolve identically.
- [ ] Extract C6 sequence step driving, just-in-time `compose_step`, phase-1c aggregation, and review selection; replace string sentinels with `SequenceStepResult::interrupted()` and `ModelChoice::Default`.
- [ ] Finish C9/C10 integration so sequence YAML/Markdown references use the same magic-root resolution, shell sources use the library runner, inline validation preserves ordering, setter coercion is shared, loop options preserve env/flag precedence, and dry-run facts predict the live path.
- [ ] Add deterministic tests for serial and parallel groups, interrupts, output accumulation, preflight/execute equivalence, source snapshots, shell approval unions, schema recovery, target picker requests, and inline document mutation.
- [ ] Delete behavior-bearing CLI composition/sequence helpers after adapters call the new library drivers; retain clap parsing, TTY checks, picker implementations, `$EDITOR`, terminal frames, and channel selection only.
- [ ] Checkpoint: run all composition, schema, target, sequence, dry-run, and inline-closure nextest suites plus `just test`, `just lint`, and applicable `just test-l2` prompt/terminal cases.

## Phase 7: Complete Admin, Resource, Config, and Rendezvous Lanes

**Goal:** move remaining reusable administration policy into its natural owning crates. Tasks in this phase are parallelizable after Phase 2 when file ownership does not overlap.

- [ ] Implement A1's `ProviderHookPlan`, promote `AgentConfigurator::diff` to the trait, and return `HookSyncDiff` from registration; route hooks list, sync, providers, quick init, and wizard init through one expected-events rule.
- [ ] Implement A2's `InstallPlan::compute`, `quick`, and `apply`; choose and document one fresh-install policy, migrate useful fixtures, delete the test-only `commands/init/` policy fork, and prove logging/TTS/sound/provider choices persist.
- [ ] Move A3 dashboard folding, staleness/trust policy, and typed backward-compatible register projection into `rendezvous-core::dashboard`; preserve old/unknown JSON values, use the existing status reducer, and do not add `rendezvous-core` as a `claudine` dependency.
- [ ] Implement A4 MCP cascading removal, provider presence, and auth mode in the library; verify defaults cannot reference removed servers and provider-state key conventions are hidden behind typed APIs.
- [ ] Move A5 signal verification into `claudine::signals::verify` while retaining the deliberate `claudine-gen` bootstrap duplicate; test positive replay, overlap exclusions, version selection, and corpus loading.
- [ ] Complete A6 as separate reviewable slices: HookAction field parsing, Mapper `FromStr`/`Display`, voice reducer, validated config save, `biscuit-speaks` host voice enumeration, public merge policy, and `ProtectRuleToggles::set/toggle`; fix D2/D8 and delete copied tests/matches.
- [ ] Implement A7/A8: publish total messenger route conversion and typed descriptors, and make resource filters/report diagnostics consistent across skills, agents, and slash commands.
- [ ] Complete A9 small moves: uninstall/backup symmetry, config path resolution, per-action sound validation and suggestion ownership, canonical provider pairing, `LogTarget` constructors, and TTS install planning.
- [ ] Checkpoint: run config, hooks, init, dashboard/rendezvous, MCP, signal, messaging, resource, uninstall, sound, and TTS targeted suites; run `cd rendezvous && just test && just lint` for A3 and `just test`/`just lint` from `claudine/` for the converged phase.

## Phase 8: Extract CLI Infrastructure Domain Logic

**Goal:** leave only shell protocol, terminal adapters, and clap integration in CLI infrastructure.

- [ ] Implement I3's shared argv grammar first: move ownership classification, composition-tail partitioning, provider-token canonicalization, and one setter predicate into `claudine::composition::argv`; keep clap introspection and help hoisting in the CLI and add an explicit `--` boundary test.
- [ ] Implement I4's perf timing model in `claudine::perf`, migrate reconciliation tests, and decide through consumer evidence whether only the renderer adapter remains CLI-side; keep perf argv bootstrap with I3's CLI bridge.
- [ ] Implement I1's provider failure taxonomy through the standard diagnostic discovery seam: replace `AgentErrorCategory`, native keyword classification, duplicated presentation, and API-error parsing with typed library diagnostics and catalog-backed remediation; add the D5 precedence regression.
- [ ] Implement I2 only after D3 and shared argv/discovery/schema helpers are stable: move candidate walking, scope priority, fuzzy matching, glob semantics, Darkmatter schema completion, and frontmatter mode contracts into `claudine::composition`; add `ignore` and `globset` to the library and update both dependency documents.
- [ ] Migrate completion unit/performance tests to the library, preserve the 100 ms budget with a library-side benchmark/property test, and keep shell bootstrap, `CompleteEnv`, embedded scripts, ratatui chooser, and TTY gating in `claudine-cli`.
- [ ] Remove obsolete CLI syntax, discovery, schema, diagnostic, perf, and completion implementations; use `rg`, dispatch inventory, and dependency inspection to prove only adapter code remains.
- [ ] Checkpoint: run argv, perf, diagnostics, completion, shell-bootstrap, and interactive-choice suites; then run `just test`, `just lint`, and completion performance checks on macOS, with Windows path/shim cases reserved for CI.

## Phase 9: De-fork the Contract Adapter and Remove Transitional Debt

**Goal:** make `claudine-contract` a restrictive consumer of shared launch primitives, not a second provider runtime.

- [ ] Replace contract-local provider profiles, prompt parsing, TOML escaping, shadow-HOME planning, argv, and child-environment assembly with explicit low-level library plans from Phases 3–6; delete all decentralized `match Provider` launch tables.
- [ ] Keep contract policy explicit in its own types: enabled-provider allowlist, tool/MCP denial, isolated CWD and HOME, cleared environment plus allowlist, pre-turn restrictions, post-hoc tool/permission/input rejection, and fail-closed unsupported-provider behavior.
- [ ] Add adversarial contract tests proving shared planners cannot silently select permissive CLI defaults, inject MCP, inherit unexpected credentials, escape isolation, admit unreviewed providers, or accept tool-use events.
- [ ] Remove the temporary contract dispatch-inventory allowlist from Phase 2 and require the inventory, provider completeness, and catalog parity guards to pass across `lib/src`, `cli/src`, and `contract/src`.
- [ ] Audit all migrated workstreams for compatibility facades, copied test oracles, dead CLI modules, obsolete allowlist entries, string-keyed provider tables, and direct terminal writes from domain code; delete remaining behavior-bearing transitional code.
- [ ] Checkpoint: run contract unit/integration suites, provider completeness, dispatch inventory, generator drift, `just test`, and `just lint`; require security invariant tests to pass before accepting any line-count reduction as success.

## Phase 10: Converged Validation, Documentation, and Closure

**Goal:** prove the migrations compose correctly across crates and supported operating systems, then close the umbrella review with traceable evidence.

- [ ] Merge or rebase completed child workstreams in dependency order, resolve overlaps surgically, and run `git diff --check`; do not run `cargo fmt` unless explicitly authorized.
- [ ] Run the final Claudine gates from `claudine/`: `just check`, `just build`, `just lint`, `just test`, `just test-gen`, and `just test-l2`; run the rendezvous area gates for its changed crates and classify capability skips separately from failures.
- [ ] Run focused parity tests for all ten providers and all affected modes, including diagnostics, warnings, argv/env/CWD/artifacts, output routing, exit status, persisted mutations, retry/resume/proxy ordering, and contract security.
- [ ] Validate Windows and Linux in CI for path/argument handling, non-UTF-8 preservation where supported, `.git` file discovery, shadow HOME, temp artifacts, symlink/hardlink behavior, named-pipe/process-tree boundaries, environment isolation, and fake executable shims; do not claim portability until both jobs pass.
- [ ] Run generator report-only drift checks and confirm generated provider data was not edited by hand; verify `claudine-gen` still has no dependency on the library.
- [ ] Run `detect_changes({scope: "compare", base_ref: "main"})` before any commit; compare changed symbols and execution flows with the child specs, investigate unexpected blast radius, and preserve the report as closure evidence.
- [ ] Re-read all touched rustdoc and comments for behavioral drift, deleting HOW narration and correcting stale ownership claims; update public READMEs, topic docs, `claudine/docs/dependencies.md`, repo `docs/dependencies.md`, and Claudine skill snapshots when ownership, dependencies, behavior, or workflows changed.
- [ ] Update each child spec and the source review with completed acceptance evidence, tests, and remaining limitations; close a finding only when its duplicate is deleted and its checkpoint passes.
- [ ] Checkpoint: archive the umbrella review only after every child spec is complete, all guard and CI gates pass, contract security remains intact, and no unresolved finding or transitional behavior remains.
