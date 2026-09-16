---
total_phases: 9
created: 2026-07-16
phase: 1
agent: codex/default
yolo: true
source_review: review.md
---

# DRY Review Execution Plan

This plan converts the eight active recommendations in `review.md` into dependency-ordered implementation work. It preserves the intentional differences between persisted and runtime configuration, keeps generated provider files generated, and adds source-of-truth coverage before removing duplicated logic.

## Success criteria

- Every wrapper provider in `PROVIDERS_DISPLAY_ORDER` receives identical command classification, telemetry detail fields, and repo-root CWD treatment.
- Completion derives value-taking composition flags from clap and derives provider skill roots from generated capabilities; Kilo, Pi, and Antigravity are covered by regression tests.
- Setter and Markdown-extension syntax, Git-root discovery, messaging route defaults/validation, and MCP provider-entry construction each have one authoritative implementation.
- Protect and shadow-HOME policies cover every compiled provider without matching behavior from `agent_offset` strings or editing generated `provider/*/data.rs` files by hand.
- Common wrapper/composition/sequence integration tests use a cross-platform fake-provider fixture instead of rebuilding HOME, PATH, config, and capture state independently.
- Claudine's Level 1 tests, generator drift checks, lints, build, and applicable Level 2 tests pass on macOS, with Windows and Linux behavior covered by portable code and CI acceptance checks.

## Dependency and parallelization map

- Phase 1 is required before all production changes.
- Phase 2 is the shared test foundation for the CLI lane: Phases 3 through 6.
- The conflict-minimizing CLI order is Phase 3 → Phase 4 → Phase 5 → Phase 6 because these phases overlap `argv`, completion, telemetry, and scope discovery.
- Phase 7 and Phase 8 are independent library lanes. They may run in parallel with each other and with Phases 2 through 6 after Phase 1, using separate worktrees or non-overlapping file ownership.
- Phase 9 begins only after Phases 2 through 8 are merged.

## Phase 1: Establish Baselines and Lock Behavioral Contracts

**Goal:** make current behavior, known drift, and blast radius observable before changing shared abstractions.

- [ ] Run upstream GitNexus impact analysis for every production symbol named by the review (`wrapper_command`, `Commands`, `OwnedFlags::for_composition`, setter classifiers, repository-root helpers, `MessengerProviderConfig`, `MessagingRouteConfig`, `bridge_provider_config`, `RepoHomeManager::excluded_dirs`, protect path classification, MCP writers, and MCP injectors); record direct callers, affected processes, and risk in the implementation notes, and warn before proceeding with any HIGH or CRITICAL result.
- [ ] Capture a pre-change `detect_changes({scope: "compare", base_ref: "main"})` report and confirm that unrelated user changes will not be folded into this workstream.
- [ ] Run `just sanity`, `just test-library`, `just test-cli`, and `just test-gen` from `claudine/`; record any pre-existing failures separately so later checkpoints distinguish regressions from baseline state.
- [ ] Add or identify characterization tests for intentional behavior that refactors must preserve: persisted messaging WIP routes may be incomplete, runtime messaging routes reject missing credentials, Git discovery returns `None` before caller fallbacks, MCP export and injection keep their different document placement/name-selection behavior, and provider completion follows only supported repo skill paths.
- [ ] Add failing regression tests for the three confirmed drift classes before their fixes: Kilo/Pi/Antigravity telemetry classification, completion skipping values after `--step-timeout`/`--stall-timeout`/`--max-iterations`/`--on-rate-limit`, and generated provider skill/protect/shadow-HOME coverage.
- [ ] Checkpoint: review the characterization and failing-regression results against all eight review findings; do not begin extraction work until each finding has an observable test target or an explicitly documented non-testable invariant.

## Phase 2: Add the Cross-Platform Fake-Provider Integration Fixture

**Goal:** remove repeated test bootstrap before expanding wrapper and completion integration coverage. This addresses finding 8 and supports the remaining CLI phases.

- [ ] Run upstream impact analysis on `cli/tests/common::TestWorkspace`, `augmented_path`, `write_executable`, and `seed_minimal_config`; confirm the fixture can be added without changing production execution flows.
- [ ] Inventory the wrapper, composition-schema, and sequence tests that independently create a temporary HOME, fake bin directory, minimal config, PATH, and capture files; classify only this repeated setup as migration scope rather than treating all 49 `cargo_bin_cmd!("claudine")` users as equivalent.
- [ ] Add a deliberately narrow fixture under `cli/tests/common/` that owns the temporary home/workspace, fake bin directory, minimal Claudine config, capture paths, and a preconfigured `assert_cmd::Command`; expose provider-program content and per-test assertions as inputs instead of embedding provider behavior in the fixture.
- [ ] Build PATH with the existing `augmented_path`/`std::env::join_paths` path, isolate HOME consistently, and support Unix executable scripts and Windows `.cmd` shims without placing Unix-only assumptions in the common fixture core.
- [ ] Migrate one representative wrapper, composition-schema, and sequence test first; verify their assertions and environment isolation remain unchanged, then migrate the remaining mechanically identical setup blocks in `wrap_*.rs`, `compose_schema_cli.rs`, and `sequence_*.rs` while leaving specialized PTY, signal, and real-provider tests alone.
- [ ] Add fixture self-tests for isolated homes, executable resolution, capture-file ownership, inherited PATH entries, and cleanup; include a Windows-specific command-shim assertion behind `#[cfg(windows)]` and Unix permission behavior behind `#[cfg(unix)]`.
- [ ] Checkpoint: run the migrated integration test binaries with `just test-cli --test <name>` and then `just test-cli`; verify no test depends on the developer's real HOME, provider binaries, config, or credentials.

## Phase 3: Consolidate Composition Flag and Syntax Predicates

**Goal:** make clap and one small syntax module authoritative for completion token consumption, setters, and Markdown extensions. This addresses findings 3 and 4.

- [ ] Run upstream impact analysis on `OwnedFlags::for_composition`, completion token scanning, `looks_like_setter`, `parse_compose_setter`, `split_setter`, `is_setter_shaped`, and each Markdown-extension predicate; review the completion execution flows before editing shared parsing helpers.
- [ ] Add a narrow read-only query over the clap-derived `OwnedFlags` surface, such as `composition_flag_consumes_next(token)`, and use it from completion instead of `is_value_bearing_flag`; add caching only if measurement shows repeated clap introspection is material.
- [ ] Add a clap-driven drift test that enumerates every current value-taking composition flag and proves completion skips its following space-separated value; explicitly assert the four currently missing flags without introducing a second fixed inventory as the source of truth.
- [ ] Create a small CLI syntax module with one setter splitter/classifier implementing `^[A-Za-z_][A-Za-z0-9_-]*=` and one case-insensitive `has_markdown_extension` helper for `.md` and `.markdown`.
- [ ] Route argv normalization, compose setter parsing, completion token classification, frontmatter completion, setter-value completion, default-glob completion, and schema-interactive file details through the shared helpers; keep JSON/JSON5 value coercion in `commands::compose::setters`.
- [ ] Add table-driven unit tests for empty keys, invalid first characters, hyphens/underscores, additional `=` characters in values, non-UTF-8 paths where applicable, uppercase Markdown extensions, `.md`-like suffixes, and directories whose names end in `.md`.
- [ ] Remove the superseded fixed flag match and local syntax predicates, then use `rg` to prove no independent setter grammar or case-insensitive Markdown-extension implementation remains in the reviewed CLI paths.
- [ ] Checkpoint: run the argv, completion engine, completion scope/value, compose-setter, and schema-interactive unit tests, followed by `just test-cli`.

## Phase 4: Unify Wrapper Inventory, Dispatch, and Telemetry Classification

**Goal:** classify a `Commands` wrapper once and reuse that result across dispatch, argv handling, and telemetry. This completes finding 2.

- [ ] Run upstream impact analysis on `Commands`, `wrapper_command`, `WRAPPER_SUBCOMMANDS`, `command_name`, `provider_subcommand_name`, and `root_span`; stop for review if changing `Commands` classification has a HIGH or CRITICAL dispatch blast radius.
- [ ] Move the single exhaustive borrowed wrapper classification onto `Commands`, returning the `Provider` and shared `WrapperArgs` view, and adjust command ownership at dispatch so `main` and telemetry consume that classifier rather than maintaining separate provider matches.
- [ ] Derive the runtime wrapper-token inventory from the provider catalog while keeping clap's explicit enum variants; update pre-clap subcommand detection and help injection to consume the derived inventory without changing wrapper passthrough behavior.
- [ ] Refactor `command_name`, provider subcommand naming, effective-CWD selection, and detailed wrapper span fields to use the shared classification; verify Kilo, Pi, and Antigravity now receive `command=wrap`, their provider token, repo-root CWD treatment, and the same interactive/quiet/silent/repo/MCP fields as the older wrappers.
- [ ] Add one catalog coverage test comparing classified wrapper providers and tokens against `PROVIDERS_DISPLAY_ORDER`, and retain a separate assertion that every explicit clap wrapper variant is classified exactly once.
- [ ] Add telemetry tests that collect span fields for an older wrapper and each previously drifted provider, plus a non-wrapper control, so future catalog additions cannot silently fall back to the generic span.
- [ ] Confirm `completion::root_menu::wrapper_tokens()` and composition provider switches remain catalog-derived; do not replace the already-correct completion inventory.
- [ ] Checkpoint: run main/argv/telemetry unit tests, wrapper command-routing integration tests, `cli/tests/dispatch_inventory.rs`, and `just test-cli`.

## Phase 5: Introduce One Git-Root-Only Discovery Primitive

**Goal:** use one Sniff-backed `Option<PathBuf>` helper while preserving each caller's explicit fallback policy. This addresses finding 5.

- [ ] Run upstream impact analysis on every upward walk listed in the review plus `linking::resolve_repo_root`; record which callers start from a file, which start from a directory, and which require `None`, CWD/source-directory fallback, or repo-config lookup.
- [ ] Add a public library-level Git-root-only helper using `sniff::filesystem::git::GitRepo::discover`, with documented file-versus-directory start semantics and `Option<PathBuf>` failure behavior; keep broader repository/package discovery out of this primitive.
- [ ] Add tests for a nested directory, an existing file start, a normal checkout, a linked-worktree `.git` pointer, a non-repository path, and platform path separators; use real temporary Git repositories/worktrees only when Git is available and skip only the worktree-specific assertion when the host cannot create one.
- [ ] Replace the private walks in composition preparation, completion scope discovery, completion repo-config detection, harness shell-policy selection, and telemetry with the shared primitive; keep repo-config checks and source/CWD fallbacks at their current call sites.
- [ ] Refactor `linking::resolve_repo_root` into an explicit fallback wrapper over the Git-only helper while preserving its existing non-Git repository detection and final CWD fallback.
- [ ] Remove obsolete local helpers and correct any comments that still describe a `.git` directory-only walk; treat the existing code behavior as authoritative where comments drift.
- [ ] Checkpoint: run composition prepare, linking paths, completion scopes/engine, harness shell-options, and telemetry tests, then `just test-library` and `just test-cli`.

## Phase 6: Derive Provider Resource and Isolation Paths From the Catalog

**Goal:** eliminate the three literal provider-path policies while preserving their distinct completion, security, and shadow-HOME semantics. This addresses finding 6 and depends on Phase 5 because both touch completion scope discovery.

- [ ] Run upstream impact analysis on `ProviderCapabilities`, `provider_info`, completion skill scopes, `SensitivePathChecker`, and `RepoHomeManager::excluded_dirs`; inspect generated-catalog consumers and warn before any high-risk metadata schema change.
- [ ] Replace completion's `SKILL_PEER_DIRS` with a deterministic, deduplicated projection of supported repo skill paths and documented `also_reads_from` paths from `ProviderCapabilities`; preserve scope ordering and `follow_links = false` behavior.
- [ ] Add completion tests proving Kilo's `.kilo/skills`, Pi's `.pi/skills`, Antigravity's `.agents/skills`, and alternate read paths are included once, while unsupported or user-global skill paths are excluded.
- [ ] Split protect's static sensitive-home catalog into non-provider secrets plus provider-derived roots from generated `agent_offset`, `config_paths`, and resource user paths; normalize home-relative templates safely, ensure every compiled provider contributes its relevant roots, and remove `.roo` unless a documented non-provider compatibility requirement is found.
- [ ] Build a provider-by-provider matrix of existing shadow-HOME exclusions and compare it with exclusions derivable from `ProviderCapabilities`; if resource metadata cannot preserve an intentional exclusion such as Claude hooks, add a typed shadow-HOME policy field through catalog types, facts/overrides, `claudine-gen`, and generated output rather than matching `agent_offset` strings or hand-editing `provider/*/data.rs`.
- [ ] Change `RepoHomeManager` to retain typed provider identity and query the catalog-backed exclusion policy; add exact tests for all compiled providers, including Kilo, Pi, and Antigravity, in both normal and `repo_only` modes.
- [ ] Add inventory guards that fail whenever a compiled provider lacks protect coverage, completion-resource consideration, or a shadow-HOME policy decision; make intentional empty policies explicit.
- [ ] Regenerate provider data only through `claudine-gen` if the catalog schema changes, review generated diffs for all providers, and run the generator's report-only drift check to prove committed artifacts match facts/research/overrides.
- [ ] Checkpoint: run protect, linking-capability, provider catalog, repo-home, and completion-scope tests; then run `just test-library`, `just test-cli`, and `just test-gen`.

## Phase 7: Consolidate Messaging Route Metadata and Validation

**Goal:** share route kind, labels, credential defaults, and webhook rules while retaining separate persisted-WIP and resolved-runtime policies. This addresses finding 1 and is parallelizable after Phase 1.

- [ ] Run upstream impact analysis on `MessengerProviderConfig`, `MessagingRouteConfig`, their validators/default helpers, and `bridge_provider_config`; review config TUI, init, deserialization, and dispatch-loader callers before changing shared types.
- [ ] Expand characterization tests to cover all six route kinds, all eight default environment-variable values, persisted JSON/JSON5 round trips, user/repo runtime settings, inline credentials, blank environment-variable names, webhook URL validation, and intentionally incomplete bot routes during TUI editing.
- [ ] Introduce one internal route core keyed by a typed route kind and credential role, owning provider labels, default environment-variable names, credential-source normalization, and shared Discord/Slack webhook validation.
- [ ] Keep `MessengerProviderConfig` as the persisted/TUI-editable shape and make its validation policy explicitly WIP-tolerant; make `MessagingRouteConfig` a strict resolved runtime representation unless serialization compatibility proves a second public grammar is required.
- [ ] Refactor both configuration models to delegate common rules to the route core, and reduce `bridge_provider_config` to the intentional persisted-to-runtime shape differences rather than repeating route metadata and credential rules.
- [ ] Add parity tests showing both models use identical defaults and webhook rules, plus policy tests showing incomplete persisted routes remain editable while the equivalent runtime route fails with the expected typed error.
- [ ] Review rustdoc, config examples, the config TUI, and messaging docs for behavior descriptions affected by the type split; update only documentation whose public contract changed or drifted.
- [ ] Checkpoint: run messaging config, dispatch loader, config, config-TUI, and messaging send tests, followed by `just test-library` and `just test-cli`.

## Phase 8: Extract Shared MCP Provider-Entry Builders

**Goal:** serialize each Codex, Gemini, and OpenCode server entry once while leaving document mutation and placement provider-specific. This addresses finding 7 and is parallelizable after Phase 1.

- [ ] Run upstream impact analysis on `McpServer`, `write_codex_mcp`, `write_gemini_mcp`, `write_opencode_mcp`, and the three runtime injectors; verify import, export, sync, and wrapper-injection flows that consume the resulting documents.
- [ ] Add characterization tests for every shared server field and provider override type (`Value`, bool, integer, string, string array, and string map), including absent and wrong-type overrides, before moving accessor logic.
- [ ] Move typed provider-override accessors onto `McpServer` or one shared MCP helper module, preserving current fallback and malformed-value behavior and deterministic map ordering.
- [ ] Add one typed entry builder per provider schema for Codex TOML, Gemini JSON, and OpenCode JSON; make each builder return only the provider-native server entry and keep native-name/catalog-ID selection, managed-entry replacement, document loading, and output paths in export/injection callers.
- [ ] Refactor export and runtime injection to call the same builders, deleting their duplicated command/args/env/URL/header/tool-filter/override assembly and duplicate TOML array/map helpers.
- [ ] Add parity tests proving export and injection serialize identical server fields for the same `McpServer`, while existing provider-specific golden tests continue to prove the complete JSON/TOML documents, names, and placement are correct.
- [ ] Add negative tests for local versus remote transport, empty optional collections, provider overrides, and preservation of unrelated existing document keys.
- [ ] Checkpoint: run all MCP type/export/inject/import tests and wrapper MCP integration tests, followed by `just test-library` and `just test-cli`.

## Phase 9: Run Converged Validation and Close the Review

**Goal:** prove the independent lanes compose cleanly, generated sources are current, and every review finding has objective closure evidence.

- [ ] Rebase or merge the parallel lanes in dependency order, resolve overlaps surgically, and run `git diff --check`; do not run `cargo fmt` in write mode.
- [ ] Run `cargo fmt --check` as a read-only diagnostic and hand-correct only formatting introduced by this work, matching `main` and surrounding style.
- [ ] Run the final package gates from `claudine/`: `just sanity`, `just check`, `just build`, `just lint`, `just doctest`, `just test`, `just test-gen`, and `just test-l2`; classify any host-capability skips separately from failures.
- [ ] Run the generator report-only check from `claudine/gen` and confirm no direct edits exist under `lib/src/provider/*/data.rs` beyond generator output.
- [ ] Validate Windows and Linux in CI for the fake-provider `.cmd`/executable paths, path normalization, worktree discovery, shadow-HOME behavior, and integration-test environment isolation; do not declare cross-platform completion until those jobs pass.
- [ ] Run `detect_changes({scope: "compare", base_ref: "main"})`; verify the changed symbols and execution flows match findings 1 through 8, investigate unexpected blast radius, and retain the report as review evidence before any commit.
- [ ] Re-read all behavior-changing rustdoc and inline comments touched by the implementation, removing HOW narration or stale claims and updating READMEs, dependency docs, or the Claudine skill only when public behavior, dependencies, or architecture actually changed.
- [ ] Update `review.md` statuses and frontmatter counters with links to the closing tests for each finding; set all eight recommendations to addressed only when their phase checkpoint and cross-platform acceptance criteria pass.
- [ ] Archive the review directory through the repository's review lifecycle only after every checkbox above is satisfied; leave it active with precise remaining status if any validation gate is incomplete.

