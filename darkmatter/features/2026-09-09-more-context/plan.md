---
total_phases: 12
created: "2026-09-11"
phase: 3
agent: "claude/opus"
yolo: "true"
source_files_during_phase_1:
  - darkmatter/cli/src/args/command.rs
  - darkmatter/cli/tests/compose_remote_caching.rs
  - darkmatter/lib/src/markdown/compose/cache/runtime.rs
  - darkmatter/lib/src/markdown/compose/context/authority.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/execution_tests.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/tests/schema.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/tests/persistent_cache_disabled.rs
  - darkmatter/lib/tests/reference_integration.rs
  - darkmatter/lib/tests/request_context_epoch.rs
docs_updated_during_phase_1:
  - darkmatter/docs/cli/compose.md
  - darkmatter/docs/structs/Markdown.md
  - darkmatter/docs/topics/caching.md
  - darkmatter/features/2026-09-09-more-context/spec.md
  - darkmatter/features/2026-09-09-more-context/plan.md
  - darkmatter/features/2026-09-09-more-context/implementation-log.md
  - darkmatter/fixes/2026-09-16-content-policy-no-cache/spec.md
docs_created_during_phase_1:
  - darkmatter/features/2026-09-09-more-context/decisions.md
skills_files_updated_during_phase_1:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/compose.md
  - .claude/skills/darkmatter/library-surfaces.md
source_files_during_phase_2:
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/aggregate_view.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/lib/tests/integration.rs
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/bin/render_git_status_fixture.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/filesystem/deps.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/filesystem/package_areas.rs
  - sniff/cli/src/output/filesystem/repo.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/snapshots/snapshots__cargo_monorepo_structure_text.snap
  - sniff/cli/tests/snapshots/snapshots__cargo_pnpm_monorepo_structure_text.snap
  - sniff/cli/tests/snapshots/snapshots__repo_aggregate_json.snap
  - darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs
  - darkmatter/lib/tests/empty_package_area.rs
  - darkmatter/cli/src/commands/compose.rs
  - claudine/lib/src/composition/launch_workspace.rs
  - claudine/lib/src/composition/lifecycle/control.rs
  - claudine/lib/src/events/environment.rs
  - claudine/cli/src/commands/wrap/env/mod.rs
  - claudine/cli/src/commands/wrap/env/package_context.rs
  - claudine/cli/src/commands/wrap/env/tests.rs
docs_updated_during_phase_2:
  - sniff/docs/cli/repo.md
  - sniff/docs/cli/repo_deps.md
  - sniff/docs/cli/repo_package-area.md
  - sniff/docs/cli/repo_package-area-root.md
  - sniff/docs/cli/repo_package-areas.md
  - sniff/docs/cli/repo_recent-commits.md
  - darkmatter/features/2026-09-09-more-context/plan.md
  - darkmatter/features/2026-09-09-more-context/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/sniff/remote-and-repository.md
source_files_during_phase_3:
  - sniff/lib/Cargo.toml
  - Cargo.lock
  - sniff/lib/src/network/mod.rs
  - sniff/lib/src/network/interface.rs
  - sniff/lib/src/network/address.rs
  - sniff/lib/src/network/gateway/mod.rs
  - sniff/lib/src/network/gateway/linux.rs
  - sniff/lib/src/network/gateway/windows.rs
  - sniff/lib/src/network/gateway/darwin.rs
  - sniff/lib/src/network/gateway/fixtures/linux_route_v4.txt
  - sniff/lib/src/network/gateway/fixtures/linux_route_v4_on_link.txt
  - sniff/lib/src/network/gateway/fixtures/linux_route_v4_no_default.txt
  - sniff/lib/src/network/gateway/fixtures/linux_ipv6_route.txt
  - sniff/lib/src/network/gateway/fixtures/linux_ipv6_route_on_link.txt
  - sniff/lib/src/network/gateway/fixtures/windows_route_print.txt
  - sniff/lib/src/network/gateway/fixtures/windows_route_print_on_link.txt
  - sniff/lib/src/network/gateway/fixtures/windows_route_print_no_default.txt
  - sniff/lib/src/network/icmp/mod.rs
  - sniff/lib/src/network/icmp/packet.rs
  - sniff/lib/src/network/icmp/unix.rs
  - sniff/lib/src/network/icmp/windows.rs
  - sniff/lib/tests/network_primitives.rs
docs_updated_during_phase_3:
  - sniff/docs/dependencies.md
  - docs/dependencies.md
  - darkmatter/features/2026-09-09-more-context/plan.md
  - darkmatter/features/2026-09-09-more-context/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/sniff/network.md
  - .claude/skills/os/macos.md
packages:
  - sniff
human_review: true
human_review_items:
  - "Build-host storage crisis blocks native Windows and WSL2 evidence (AC14, and Phase 12 generally). W: on build-win-native has 8192 bytes free, so cross-check fails with 'No space left on device' and the WSL guest (VHDX on W:) resets SSH. Read-only inventory: W:\\ci-verification\\rusty-biscuit\\target 95.1 GB (over the 80 GB sweep cap; that standing clone has no .cargo/config.toml target-dir pin), W:\\ci-verification\\rb-pr66 61.4 GB (2026-08-30, another session), W:\\WSL\\Ubuntu-26.04\\ext4.vhdx 130.8 GB. The daily sweep reported success at 04:00. Nothing was deleted because none of it belongs to this session (storage-strategy rule 2). Decide what to remove or compact."
  - "build-linux cross-check lock held since 2026-09-14T18:25Z by purpose=nightly-reward-spike, branch=feat-nightly-perf. It is probably stale, but the script never removes locks. Linux evidence for this phase came from Docker Desktop instead. Remove it if that run is dead."
  - "Confirm three Phase 3 design decisions that later phases build on. (a) Gateways are a separate API, sniff::network::detect_default_gateways(), not a NetworkRequest flag, because detect_network_with_request is GitNexus HIGH. (b) ICMP uses unprivileged datagram sockets with no ping-subprocess fallback, so Linux/WSL2 hosts whose net.ipv4.ping_group_range excludes the process group get IcmpError::NotPermitted, which becomes a compose error per R6. The wsl2-ubuntu CI leg must provide that sysctl for AC14. (c) macOS primary default = first UP default route that is not RTF_IFSCOPE (matches `route get default`); the spec's 'first default route in the dump' did not mention scoped routes."
  - "Carried from Phases 1-2, still unconfirmed: the retroactive HIGH edit to Darkmatter current_package_context; Q1 execution nonce (blocks Phase 5); Q2 per-expression memo scope (blocks Phase 6); the content-policy-no-cache --cache-root scope ruling (Phase 11/12); explicit review before Phase 10.3 edits CRITICAL capture_at_event."
message_to_agent: >-
  Phase 3 added Sniff network primitives; no Darkmatter/Claudine code changed. APIs for Phase 5/7/9:
  sniff::network::{ScopedIpAddr (strict FromStr/Display/serde string; `ipv6%zone`; is_within(&ipnet::IpNet) ignores the zone,
  is_loopback, is_link_local), host_addresses(&info.interfaces) -> sorted, deduped Vec<ScopedIpAddr> (IPv6 link-local zoned with the
  interface name on Unix, numeric index on Windows), cgnat_network(), contains_cgnat_address(..) (ctx.tailnet),
  detect_default_gateways() -> Result<DefaultGateways { v4: Option<Ipv4Addr>, v6: Option<ScopedIpAddr> }> (ctx.gateway / ctx.gateway_v6;
  a separate call, NOT part of detect_network_with_request, which is HIGH risk and was not edited; on Windows it costs one route print)}.
  sniff::network::icmp::{ping(&ScopedIpAddr, ProbeBudget) -> Result<PingReport, IcmpError>, ping_with(&mut dyn EchoProbe, ..) as the
  stub seam for AC8, ProbeBudget::from_millis(f64 ms, f64 attempts) (rejects NaN/inf/<=0/>60000 ms and fractional or out-of-range
  attempts 1..=100 without truncation; ping() default is from_millis(100.0, 1.0)), PingReport::verdict() AllReplied|NoneReplied|Unstable
  -> true|false|"unstable", IcmpError::{InvalidBudget, NotPermitted, UnknownScope, Send, Unsupported} = compose error; no reply is never an error}.
  Darkmatter still owns: allow-list/CIDR ICMP policy and denial null+warning (sends nothing), preflight effect records, and the
  ipv4()/ipv6() substring-vs-CIDR filter rule (use ipnet; is_within already returns false across families). Unix ICMP needs
  net.ipv4.ping_group_range on Linux/WSL2. Real ICMP tests are `real_`-prefixed in sniff/lib/tests/network_primitives.rs (just test-real).
  Native Windows and WSL2 runtime evidence is still missing because the Windows build host's W: volume is full (see human_review_items);
  Windows has compile evidence only (just check-windows). Baselines after Phase 3: sniff L1 2699 passed; darkmatter L1 7905 passed; both lints clean.
---

# Execution Plan: More Context

This plan implements the complete `spec.md` contract across Darkmatter, Sniff,
Claudine, and `claudine-gen`. Work is ordered so cross-platform discovery and
transport primitives land before Darkmatter consumes them, request-scoped
authority exists before lazy globals and nested composition use it, and the
three unresolved specification questions are closed before dependent behavior
is coded.

## Dependency and Parallelization Summary

| Work | Depends on | Parallelization |
|---|---|---|
| Phase 1: decisions and baseline | none | decision records and baseline inventories can run together |
| Phase 2: Sniff repository contracts | Phase 1 | parallel with Phases 3 and 4 |
| Phase 3: Sniff network primitives | Phase 1 | parallel with Phases 2 and 4; OS-specific parsers can be developed independently |
| Phase 4: descriptors and generation | Phase 1 | parallel with Phases 2 and 3 |
| Phase 5: capture authority and eager context | Phases 2-4 | document, network, and Git implementation tracks can run in parallel after shared capture types settle |
| Phase 6: lazy globals | Phases 1 and 5 | `current_env` tests can proceed alongside `current` provider work |
| Phase 7: non-ICMP functions | Phases 2-6 | repository, Git, network-enumeration, shell, and agentic-CLI tracks are parallelizable |
| Phase 8: nested composition | Phases 5-7 | implementation is serial around the shared compose pipeline |
| Phase 9: ICMP effects | Phases 3, 4, and 8 | transport tests and Darkmatter policy tests can run in parallel |
| Phase 10: Claudine integration | Phases 5-9 | evidence integration and lifecycle migration can be separate branches with coordinated shared files |
| Phase 11: documentation and cleanup | Phases 2-10 | package-specific docs can be updated in parallel |
| Phase 12: release validation | all prior phases | package gates can run concurrently; cross-OS CI runs in parallel |

Parallel branches must not independently edit the descriptor registries,
`ComposeOptions`, capture requirement types, or Claudine lifecycle lookup
assembly. Assign one owner to each shared surface and merge consumer branches
after that owner lands the contract.

## Phase 1 — Resolve Contracts and Establish the Baseline

- [x] **1.1 Resolve Q1 before identity implementation.** Ratify the recommended per-execution random nonce shared by `ctx.id` and `ctx.sid`, specify a typed entropy-failure error, retain the four length-prefixed/versioned inputs, and update R2/AC4/AC36 in `spec.md`; if the recommendation is rejected, explicitly weaken the uniqueness claim and freeze the alternate collision contract.
- [x] **1.2 Resolve Q2 before lazy-global implementation.** Ratify the recommended per-expression-evaluation, per-key memo scope, define a fresh evaluation scope for every lifecycle event, and update R30/AC27/AC28/AC36 so repeat reads within one expression are stable while later expressions can observe mutations.
- [x] **1.3 Resolve Q3 as a shipping prerequisite.** Activate and complete the cache-disable portion of `darkmatter/fixes/_unscheduled/content-policy-no-cache`, or amend this feature's scope with another ratified mechanism; prove with a warm-cache test that identity and probe output cannot be replayed before proceeding to Phase 12.
- [x] **1.4 Record architecture boundaries.** Write a short implementation decision record in the feature directory covering root-source ownership, fixed resolution/repository anchors, eager requirements versus lazy capabilities, the refresh-provider interface, descriptor-pair ownership, ICMP policy separation, and the shared recursion budget.
- [x] **1.5 Capture the source baseline.** Repeat the spec's scoped searches for `current.ctx.`, `current.env.`, and sentinel comparisons involving `package_area`, `current_package_area`, or `ctx.area`; classify each hit as implementation, test, shipped prompt, documentation, historical spec, or unrelated literal so later cleanup is observable.
- [x] **1.6 Capture the test baseline.** Run the existing package-local L1 suites for `sniff`, `darkmatter`, `claudine/gen`, and `claudine`, recording any pre-existing failures separately from this feature.
- [x] **1.7 Perform required impact analysis before edits.** For every existing symbol selected for modification, run GitNexus upstream impact analysis, record direct callers/processes/risk in the implementation log, and stop for explicit review before changing any symbol reported HIGH or CRITICAL.
- [x] **Checkpoint 1.** Q1-Q3 have ratified, testable contracts; the cache prerequisite is completed or a scope amendment is ratified; inventories and baseline test results are saved; and no implementation phase remains dependent on an unstated assumption.

## Phase 2 — Normalize Sniff Repository Semantics

- [x] **2.1 Remove the stored `"root"` package-area sentinel.** Change package detection, `Package.package_area`, `area_for_dir`, directory fallback, aggregate models, and fixtures under `sniff/lib/src/filesystem/repo/` to use `""` for the repository-root area while preserving a legitimate area actually named `root`.
- [x] **2.2 Update Sniff CLI projections.** Adjust `sniff repo area`, package-area listings, dependency/repository JSON output, help text, and `sniff/docs/cli/repo_deps.md` so semantic values stay empty and any human-facing root label is added only during rendering; keep the existing exit-status distinction for an empty result.
- [x] **2.3 Add a cheap current-worktree observation.** Expose the linked worktree name without requiring full worktree enumeration, returning `null`/absence for the main checkout and outside a repository, and make it available to invocation-owned evidence builders.
- [x] **2.4 Expose the canonical per-commit plain formatter.** Move or publish the per-commit rendering entry point from `recent_commits.rs`, parameterize deterministic date/time context as needed, and keep `CommitDescSet::describe(true)` plus `sniff repo recent-commits --plain` delegating to it byte-for-byte.
- [x] **2.5 Add repository regression fixtures.** Cover top-level packages, an area-only directory, repository root, outside-monorepo behavior, a real area named `root`, linked versus main worktrees, conventional/non-conventional commits, empty commits, and deterministic time-zone rendering.
- [x] **Checkpoint 2.** Sniff L1 tests and lint pass; AC22 and the Sniff-owned portions of AC2, AC21, AC24, and AC37 are proven; a scoped semantic grep finds no sentinel-specific branch left in Sniff.

## Phase 3 — Add Cross-Platform Sniff Network Primitives

- [x] **3.1 Load and follow the repository `os` skill before platform edits.** Identify the macOS, Linux, native Windows, and WSL2 evidence paths and define fixture formats that do not require the executing host to match the parsed OS.
- [x] **3.2 Add stable interface-address helpers.** Preserve routable IPv6 scope suffixes, deduplicate by address-and-scope, provide stable ordering, and expose reusable address-bit matching for CGNAT and Darkmatter filtering without DNS or ambient rediscovery.
- [x] **3.3 Parse primary IPv4 and IPv6 gateways.** Extend Linux `/proc` parsers, Windows route output/parsers, and macOS routing-socket parsing to select the specified UP lowest-metric/first default route, return no gateway for on-link routes, and retain `%scope` for link-local IPv6 gateways.
- [x] **3.4 Add the CGNAT/Tailscale predicate.** Return true when any captured interface address is within `100.64.0.0/10`, with deterministic fixture-driven coverage and no requirement for a live Tailscale installation.
- [x] **3.5 Implement the ICMP transport API.** Provide single-attempt and sequential multi-attempt probes with monotonic millisecond deadlines, bounded setup/cleanup, checked numeric limits, scoped IPv6 support, and distinct timeout versus cannot-send errors on macOS, Linux, native Windows, and WSL2.
- [x] **3.6 Harden subprocess/process behavior.** Close stdin, bound output and child lifetime, use Unix process groups and Windows process containment, and guarantee probe tests never activate a terminal or browser window.
- [x] **3.7 Add deterministic parser and transport tests.** Supply six route fixtures (two families on three OSes), on-link/no-route fixtures, scoped IPv6 fixtures, stubbed all-fast/all-slow/mixed ICMP outcomes, send-failure-after-success, invalid/overflow budgets, and bounded cleanup cases.
- [x] **Checkpoint 3.** Sniff L1 tests and lint pass on the macOS host; AC9, AC13, and deterministic portions of AC8/AC33 pass; cross-platform compilation evidence is queued for Phase 12.

## Phase 4 — Establish Descriptor and Generation Sources of Truth

- [x] **4.1 Design the variable/function pair representation.** Make one descriptor entry own the shared `recent_commits` semantics and project exactly one `ctx.recent_commits` variable plus one `recent_commits(count)` function, while retaining compatibility with the existing schema and expression catalogs.
- [x] **4.2 Extend the context descriptor catalog.** Add the Document group fields, `hostname`, Network fields, `recent_commits`, required-string scope fields, and reserved `current`/`current_env` roots with correct optionality and output types. _(Phase 4 note: the `required` flag on `current_package`/`current_package_area` moved to 5.5, because capture still projects `null` on both the ambient and supplied paths and the flag must change atomically with that projection.)_
- [x] **4.3 Extend expression descriptors.** Add `as_markdown`, repository, Git, network, shell, and agentic-CLI functions; represent `IpAddress`, nullable returns, and `boolean | "unstable" | null` without weakening argument validation.
- [x] **4.4 Add direct dependencies deliberately.** Enable Darkmatter's required `biscuit-hash` BLAKE3 feature, add direct `ipnet` and any ratified nonce/ICMP support dependency, and update root/per-area dependency records in Phase 11; do not introduce a Darkmatter-to-Claudine dependency.
- [x] **4.5 Extend `claudine-gen` with a roster-level emitter.** Read `claudine/docs/providers.yaml` directly, include every slug and `cli_aliases` regardless of `skip_research`, validate each `sniff_binding`, and emit a committed Darkmatter artifact containing accepted names and their `sniff::AiCli` mapping.
- [x] **4.6 Add generation drift protection.** Make `claudine-gen generate` reproduce the artifact and `claudine-gen check` plus nextest fail for stale additions, removals, aliases, renames, or mappings without routing through per-provider generation.
- [ ] **4.7 Preserve passive tooling.** Update schema parsing, descriptor corpus, DMLS completion/hover, and validation fixtures so all names and signatures are discoverable without evaluating functions, reading files, launching profiles, probing the network, or mutating documents.
- [ ] **Checkpoint 4.** Generator, descriptor-corpus, and passive DMLS tests pass; AC1, AC19, AC26, and descriptor portions of AC12/AC18/AC33 pass.

## Phase 5 — Extend Request-Scoped Capture and Eager Context

- [ ] **5.1 Thread immutable root-source identity into request creation.** Retain root bytes and `ComposeSource::{File, Url, Unknown}` metadata when the request is created, preserve the canonical local path and optional modification time, and never reopen/refetch the root solely for context capture.
- [ ] **5.2 Implement the Document context group.** Project `ctx.self`, `ctx.last_updated`, and `ctx.hash` according to source kind; compute the Markdown frontmatter/body xxHash through `Markdown::hash`; compute versioned, length-prefixed `ctx.id`/`ctx.sid` using the Q1 contract and fixed hostname/repository-name inputs.
- [ ] **5.3 Extend Host and Network eager capture.** Project `ctx.hostname`, `ctx.tailnet`, `ctx.gateway`, and `ctx.gateway_v6` from captured Sniff evidence, with schema-compatible empty/null values and `PartialRuntimeCapture` when requested evidence was not supplied.
- [ ] **5.4 Refine Git requirements below the public group.** Distinguish ordinary Git facts from eager recent-history demand, capture ten `CommitDesc` values only for `ctx.recent_commits`, render each through Sniff's canonical formatter, preserve `[]` outside/empty repositories, and report real Git failures.
- [ ] **5.5 Normalize scope projections.** Make `ctx.current_package` and `ctx.current_package_area` required strings with `""` for misses (including the `required` flag in `darkmatter.yaml`, deferred from 4.2), retain `ctx.area`'s empty-string contract, and delete Darkmatter sentinel workarounds in `capture/repo.rs` and `repository_scope.rs`.
- [ ] **5.6 Supply current-worktree evidence consistently.** Use Sniff's cheap observation in supplied/Claudine capture while leaving the working ambient path equivalent, so linked worktrees and main checkouts project the same values in both paths.
- [ ] **5.7 Preserve graph-wide identity.** Ensure transclusions and future `as_markdown` children reuse root Document values, fixed resolution anchors, diagnostics, and repository observation rather than creating nested snapshots.
- [ ] **5.8 Add hermetic capture tests.** Cover file/URL/in-memory roots, root mutation after load, missing mtime/hostname/evidence, outside-repository identity, frozen-clock collision behavior from Q1, all five scope positions through ambient and supplied evidence, and demand counters proving unrelated captures do no history/network/profile work.
- [ ] **Checkpoint 5.** Darkmatter context tests pass; AC3-AC5, AC9, AC13, AC21, AC23, AC24, AC30, AC31, and eager portions of AC36/AC37 are satisfied.

## Phase 6 — Add Lazy `current` and `current_env` Reserved Roots

- [ ] **6.1 Introduce invocation-owned refresh capabilities.** Define a provider that refreshes one requested mutable fact against retained launch roots/evidence authority, never recaptures CWD or repository topology, and returns `PartialRuntimeCapture` instead of ambient fallback when a capability is absent.
- [ ] **6.2 Separate planning concepts.** Track eager `ctx` requirements, deferred `current` capabilities, and function dependencies across frontmatter, body, `when=`, shell/preflight parsing, transclusions, and nested content; bare-root enumeration must use descriptor keys without materializing values.
- [ ] **6.3 Reserve `current` globally.** Mirror every `ctx` descriptor at `current.<key>`, preserve request-owned identity fields, refresh mutable facts through the provider, enforce the Q2 memo scope, prevent shadowing, and reject `current.ctx.*` as an unknown path.
- [ ] **6.4 Reserve `current_env` globally.** Resolve `current_env.<key>` by rereading the live process environment at reference time, use the same missing-value semantics as `env`, enforce the Q2 memo scope, prevent shadowing, and reject nested shapes such as `current_env.ctx.*`.
- [ ] **6.5 Integrate every Darkmatter expression surface.** Verify frontmatter passes, body interpolation, page-block `when=`, `$()` expression branches, and injected-global layering all recognize the two roots without enabling effects during passive validation or preflight.
- [ ] **6.6 Add mutation and laziness tests.** Use controlled in-process providers to change branches and environment values between evaluations; prove same-expression repeat stability, later-expression freshness, stable `ctx`/`env`, lazy `current.recent_commits`, no unreachable probes, and no fallback from missing supplied evidence.
- [ ] **Checkpoint 6.** Darkmatter L1 tests pass and the Darkmatter-owned portions of AC27, AC28, AC31, AC32, and AC36 are satisfied.

## Phase 7 — Implement Non-ICMP Expression Functions

The five tracks below can be developed in parallel after one owner lands the
shared dispatch/descriptor and evaluation-context changes.

- [ ] **7.1 Implement `package` and `package_area`.** Resolve `file | string` arguments through `FileReference` and the captured `FileResolutionContext`, reject malformed/remote references without fetching, then perform component-aware deepest lexical lookup over the captured package list with `""` for valid misses and independent area/package selection.
- [ ] **7.2 Implement `recent_commits(count)`.** Validate a finite integer within supported allocation/process limits, require `count >= 1`, use the captured repository root without rediscovery, perform uncached Git I/O at every reached call, and reuse the exact per-commit formatter used by eager capture.
- [ ] **7.3 Implement `ipv4` and `ipv6`.** Use captured stable interface data; exclude loopback/link-local by default; treat only syntactically valid same-family CIDRs as CIDRs; otherwise substring-match the default set; compare scoped IPv6 bits without discarding returned scope.
- [ ] **7.4 Implement shell classification probes.** Refactor the bounded alias launcher into dialect-aware, data-safe queries for bash, zsh, fish, and PowerShell; never interpolate or execute the probed name; recognize aliases to builtins/missing binaries, shell builtins/cmdlets, and user functions; return false for every launch/profile/timeout failure.
- [ ] **7.5 Implement `has_binary` and `can_execute`.** Bind `has_binary` to the existing `has_command` implementation, and implement the four-way OR with safe short-circuiting and no profile launch when the binary check succeeds.
- [ ] **7.6 Implement `has_agentic_cli`.** Resolve only generated enum names/aliases, map to Sniff's captured `InstalledAiClients` PATH index, return identical results for provider aliases, and make unknown names a compose error rather than false.
- [ ] **7.7 Add adversarial function tests.** Cover sibling-prefix/deepest/area-only/nonexistent paths, typed `FileReference` failures, mid-compose commits, zero/fractional/overflow counts, empty/unborn repositories, IP family mismatch and malformed-CIDR fallback, injection-shaped shell names, hanging/noisy profiles, detached-child cleanup, and all `can_execute` truth combinations.
- [ ] **Checkpoint 7.** Function and descriptor parity tests plus package-local L1 tests pass; AC10-AC12, AC17, AC18, AC20, AC25, AC34, AC35, and AC37 are satisfied.

## Phase 8 — Implement Shared Nested Composition with `as_markdown`

- [ ] **8.1 Extract a child-pipeline entry point.** Compose string content through every normal stage using the root request's `ComposeOptions`, captured `ctx`, lazy providers, resolution base, consent state, diagnostics sink, source provenance, and cache policy.
- [ ] **8.2 Share recursion and cycle state.** Increment one budget for every `as_markdown` call, share ancestry with `::file` transclusion, detect mixed function/transclusion recursion, and return the existing typed depth-limit error without hanging.
- [ ] **8.3 Preserve serialization/finalization boundaries.** Return composed Markdown including resulting frontmatter, keep root document identity in the child, and perform root-only link normalization exactly once at the outermost pipeline.
- [ ] **8.4 Extend static preflight discovery.** Recursively scan statically known nested Markdown including unreachable conditional branches, collect typed effects without executing functions/profiles/lazy globals, reject dynamic unapprovable command/effect shapes, and forbid frontmatter-local nested content from introducing remote reads.
- [ ] **8.5 Add nested-composition tests.** Cover root-relative `::file`, shared hostname/context, file/URL/in-memory identity, nested diagnostics provenance, frontmatter and body calls, root-only link normalization, mixed recursion, unreachable effects, and dynamic-effect rejection.
- [ ] **Checkpoint 8.** Darkmatter compose/preflight tests pass; AC15, AC16, and nested-composition portions of AC30/AC32 are satisfied.

## Phase 9 — Add ICMP Consent, Preflight, and Functions

- [ ] **9.1 Add a separate ICMP grant policy to `ComposeOptions`.** Parse exact IP and strict CIDR entries from the existing `--allow-host` input, normalize address spelling, retain explicit IPv6 scope constraints, and ensure these grants do not widen `FetchPolicy` HTTP host/wildcard permissions.
- [ ] **9.2 Model ICMP as typed effects.** Add planned target/timeout/attempt metadata to preflight, support already-approved exact-IP/CIDR capabilities for dynamic targets, emit one warning and no packets on denial, and prohibit evaluation from acquiring consent implicitly.
- [ ] **9.3 Implement `ping`.** Validate literal IPv4/IPv6/scoped IPv6 with no DNS, checked positive finite timeout defaulting to 100 ms, then map denied/timeout/send-failure outcomes to `null` plus warning/`false`/compose error exactly as specified.
- [ ] **9.4 Implement `ping_under`.** Validate positive integral attempts defaulting to three, enforce checked bounded total duration, run attempts sequentially, classify replies at-or-after the threshold as late, return true/false/`"unstable"`, and abort on any transport failure.
- [ ] **9.5 Verify nested and frontmatter policies.** Ensure statically known pings inside `as_markdown` appear in root preflight, runtime revalidates actual targets, and frontmatter/local-only restrictions cannot be bypassed through nesting.
- [ ] **9.6 Add deterministic consent/function tests.** Cover denied targets/CIDRs, malformed addresses, overflow and non-finite numbers, scoped IPv6, cross-family CIDRs, mixed outcomes, send failure after success, denied multi-attempt no-send, CIDR HTTP non-authorization, and exact descriptor return types.
- [ ] **Checkpoint 9.** Darkmatter L1 tests pass; AC6-AC8, AC33, and the ICMP portions of AC20/AC32 are satisfied without requiring live Internet connectivity.

## Phase 10 — Integrate Claudine and Complete the Clean-Break Migration

- [ ] **10.1 Extend Claudine evidence planning/building.** Supply Document, Network, and hostname evidence; request `CommitDesc` history only for eager recent-history demand; expose the cheap worktree observation; and retain launch-owned roots/provider capabilities for lazy refresh.
- [ ] **10.2 Update `claudine context` surfaces.** Confirm variable/function listings project from Darkmatter descriptors once each, and make `claudine context --values` use absent-document projections unless a document is explicitly supplied.
- [ ] **10.3 Replace `LifecycleCurrent`.** Remove Claudine's injected nested `current.ctx`/`current.env` object, route lifecycle lookup through Darkmatter's `current`/`current_env` roots, and preserve fresh event-scoped evaluation with fail-closed supplied evidence.
- [ ] **10.4 Extend late-binding validation.** Add `current_env` to `LATE_BINDING_ROOTS`, update known-root scanners and comments, and preserve rejection of lifecycle shell commands that reference either `current` or `current_env` during early-binding-only preflight.
- [ ] **10.5 Migrate all active code and tests.** Update the inventoried lifecycle, looping, compose, wrap, harness, and diagnostic sites to `current.<key>`/`current_env.<key>`; re-point arbitrary Darkmatter injected-global fixtures now that `current` is reserved; leave explicitly excluded historical specs unchanged.
- [ ] **10.6 Remove Sniff sentinel workarounds in consumers.** Replace each classified Claudine/Darkmatter sentinel branch with empty-string logic or delete dead branches, while preserving unrelated root labels and a real area named `root`.
- [ ] **10.7 Add Claudine L1 and L2 coverage.** Test linked/main worktrees, all five scope positions through supplied evidence, every lifecycle event, controlled parent-process environment mutation, branch mutation, prompt composition, missing capability diagnostics, and early-binding shell rejection.
- [ ] **Checkpoint 10.** Claudine and generator L1 tests pass; marked L2 tests prove AC2 and AC28; Claudine legs of AC1, AC21, AC26, AC27, and AC31 pass.

## Phase 11 — Update Public Documentation, Skills, and Dependency Records

- [ ] **11.1 Update Darkmatter documentation.** Revise `darkmatter/docs/topics/darkmatter-expressions.md`, relevant context-variable docs, the Darkmatter README, and `.claude/skills/darkmatter/compose.md` for eager `ctx`, lazy `current`, lazy `current_env`, source-kind identity, function pairs, probes/consent, and new functions.
- [ ] **11.2 Update Claudine documentation and skills.** Revise lifecycle/composition/context docs and the three Claudine skill files, replace old nesting, add both late-binding roots, remove every “implementation pending” marker, and keep the early-binding shell exception explicit.
- [ ] **11.3 Update Sniff documentation.** Document gateway/ICMP APIs, stable scoped addresses, worktree/commit formatter additions, and the empty package-area migration in its README and applicable CLI docs.
- [ ] **11.4 Update prompts and templates.** Remove sentinel workarounds in `system-prompt.md` and review prompts where classified, retain the implement-plan fallback unchanged, and verify every migrated shipped prompt including `prompts/format.md` composes.
- [ ] **11.5 Update dependency inventories.** Record all actual direct dependency and feature changes in root and Darkmatter/Sniff/Claudine area `docs/dependencies.md` files; do not document dependencies that were considered but not added.
- [ ] **11.6 Refresh Markdown hashes through Darkmatter.** For every modified Markdown file with a hash frontmatter property, run `md hash <file>` and write the resulting frontmatter/body hash using the repository's established workflow.
- [ ] **11.7 Run documentation drift checks.** Execute the scoped grep for `current.ctx.`/`current.env.` excluding the spec-approved historical locations, and the semantic sentinel grep; inspect every remaining hit rather than banning the word `root` globally.
- [ ] **Checkpoint 11.** AC29 passes, READMEs and skills describe only implemented behavior, dependency inventories match manifests, shipped prompts compose, and all required Markdown hashes are current.

## Phase 12 — Full Validation and Release Gate

- [ ] **12.1 Run Darkmatter gates.** From `darkmatter/`, run `just build`, `just test`, and `just lint`; run `just test-l2` only for real-terminal/transport cases and ensure all CLI subprocess tests use `CliProcessFixture` with fixture-owned environment and no window focus.
- [ ] **12.2 Run affected package gates.** Run package-local `just test` and `just lint` for Sniff, Claudine, and `claudine-gen`, plus their required build/generator drift recipes; do not substitute workspace-wide Cargo gates for these package-area commands.
- [ ] **12.3 Run real ICMP L2/CI evidence.** Execute bounded `127.0.0.1` and `::1` probes on macOS, Linux, native Windows, and WSL2 with an explicitly configured ICMP capability; failures must be actionable and no required leg may silently skip.
- [ ] **12.4 Run the four-OS L1 matrix.** Confirm all affected crates compile and their L1 tests pass on macOS, Linux, native Windows, and WSL2, including route-parser fixtures, shell-dialect fixtures, path behavior, and generated-artifact checks.
- [ ] **12.5 Exercise cache and freshness regressions.** Run cold and warm persistent-cache scenarios after the Phase 1 prerequisite, frozen-clock independent executions, branch/environment mutations across expression/event scopes, and demand counters for unreachable lazy work.
- [ ] **12.6 Audit every acceptance criterion.** Record pass evidence for AC1-AC37, with explicit links to the responsible test/command and no criterion inferred solely from another test.
- [ ] **12.7 Review scope before any commit.** Run GitNexus `detect_changes(scope: compare, base_ref: main)`, inspect `git diff` for unrelated/user-owned edits, verify only expected symbols and flows changed, and resolve every unexpected result; do not run `cargo fmt` and do not commit unless separately instructed.
- [ ] **Checkpoint 12.** AC1-AC37 are evidenced, all package and cross-platform gates are green, generator/docs/hash drift checks are clean, cache behavior matches the ratified Q3 outcome, and the feature is ready to move to `_completed` only after implementation approval.

## Acceptance-Criteria Traceability

| Acceptance criteria | Primary phase(s) |
|---|---|
| AC1, AC19, AC26 | 4, 10 |
| AC2 | 2, 5, 10 |
| AC3-AC5, AC30 | 5, 8 |
| AC6-AC8, AC14, AC33 | 3, 9, 12 |
| AC9, AC13 | 3, 5 |
| AC10-AC12, AC17, AC18, AC20, AC25, AC34, AC35, AC37 | 7 |
| AC15, AC16, AC32 | 8, 9 |
| AC21-AC23 | 2, 5, 10 |
| AC24 | 2, 5 |
| AC27, AC28, AC31, AC36 | 1, 5, 6, 10, 12 |
| AC29 | 11 |
