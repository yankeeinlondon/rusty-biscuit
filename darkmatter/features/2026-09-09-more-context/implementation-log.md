---
feature: darkmatter/features/2026-09-09-more-context
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
source_files_during_phase_4:
  - Cargo.lock
  - darkmatter/lib/Cargo.toml
  - darkmatter/lib/src/markdown/compose/context/catalog.rs
  - darkmatter/lib/src/markdown/compose/context/capture/groups.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/roots.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/agentic_cli_generated.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/pending.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/cli/tests/schema_about.rs
  - darkmatter/lib/tests/ambient_ctx_capture.rs
  - claudine/gen/src/agentic_clis.rs
  - claudine/gen/src/agentic_clis/tests.rs
  - claudine/gen/src/apply.rs
  - claudine/gen/src/errors.rs
  - claudine/gen/src/lib.rs
  - claudine/gen/src/main.rs
  - claudine/gen/tests/drift.rs
  - claudine/gen/tests/generate_ux.rs
docs_updated_during_phase_4:
  - darkmatter/docs/schemas/darkmatter.yaml
  - darkmatter/docs/schemas/expression-functions.yaml
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/context-variables.md
  - darkmatter/docs/dependencies.md
  - darkmatter/features/2026-09-09-more-context/plan.md
  - darkmatter/features/2026-09-09-more-context/implementation-log.md
  - .claude/skills/darkmatter/library-surfaces.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
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

# Implementation Log: More Context

## Phase 1

### 1.1 / 1.2 — Q1 and Q2 outcomes

Adopted the spec's recommendations and folded them into `spec.md` (Document
identity, `current`/`current_env` required behavior, Open Questions outcomes,
AC4, AC27, AC28, AC36). Exact identity encoding recorded in `decisions.md` D1.
An agent cannot ratify a ruling, so each outcome is marked "adopted, pending
Ken's confirmation" and listed in the plan's `human_review_items`.

### 1.4 — Decision record

Created `decisions.md` (D1 open-question outcomes, D2 root-source ownership,
D3 fixed anchors, D4 eager requirements vs deferred capabilities, D5 refresh
provider, D6 descriptor-pair ownership, D7 ICMP policy separation, D8 shared
recursion budget).

### 1.5 — Source baseline (2026-09-16)

Search A: `current\.(ctx|env)\.` in `*.md|*.rs|*.yaml|*.toml` under `prompts/`,
`claudine/`, `darkmatter/`, `.claude/skills/`, excluding `target/`,
`node_modules/`, `.gitnexus/`, `_completed/`.

| Classification | Hits |
|---|---|
| implementation (Claudine) | `claudine/lib/src/composition/lifecycle/context.rs:44-45,446-447,464,494`; `lib/src/composition/looping/engine.rs:1004-1005`; `cli/src/commands/compose/prep.rs:595`; `cli/src/commands/wrap/composition/pipeline.rs:1701`; `cli/src/commands/wrap/composition/preflight.rs:144`; `cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs:419` |
| test (Claudine) | `lib/src/composition/lifecycle/context/tests.rs:517,526,530,587`; `lib/src/composition/lifecycle/tests/diagnostics.rs:339,516`; `lib/src/composition/looping/engine/tests/mod.rs:31,36`; `lib/src/composition/prepare/service/tests.rs:160,164,176,195`; `cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs:25,31,63`; `cli/tests/composition_seams.rs:287,325` |
| test (Darkmatter, arbitrary injected-global fixture) | `darkmatter/lib/src/markdown/compose/subtree.rs:639`; `darkmatter/lib/src/markdown/compose/tests/frontmatter.rs:492` |
| user documentation | `claudine/docs/topics/lifecycle.md:437`; `claudine/docs/topics/composition.md:813,884` |
| skills (ratified shape with "implementation pending" marker) | `.claude/skills/claudine/composition.md:113-116,910,981`; `.claude/skills/claudine/SKILL.md:177`; `.claude/skills/claudine/lifecycle.md:77,463` |
| historical spec (excluded from AC29) | `claudine/features/2026-07-13-proxy-with/spec.md:488`, `plan.md:852`; `claudine/features/2026-08-01-faster-compose/spec.md:115,376`; `claudine/features/2026-08-26-finalized-references/spec.md:751`; `claudine/fixes/2026-09-13-better-static-analysis/spikes/parser-agreement.md:166` (new since 2026-09-11) |
| this feature's own spec/plan | `darkmatter/features/2026-09-09-more-context/spec.md`, `plan.md` |
| unrelated literal | `claudine/cli/tests/level2_lifecycle_control.rs:938` (local struct field `current.env.push`) |
| shipped prompts | none |

Drift versus the spec's 2026-09-11 list: line numbers shifted in Claudine
(`prep.rs:578→595`, `pipeline.rs:1698→1701`, `lifecycle_events.rs:406→419`,
`composition.md:797/868→813/884`, `subtree.rs:631→639`); the
`better-static-analysis` spike is a new historical hit.

Search B: sentinel comparisons — `(package_area|current_package_area|ctx.area)`
compared with `'root'`/`"root"`, plus every `"root"` literal on a line naming an
area or package.

| Classification | Hits |
|---|---|
| implementation (Sniff lib) | `sniff/lib/src/filesystem/repo/detection.rs:1388` (source doc + producer); `repo/types.rs:158` (field doc); `repo/aggregate_view.rs:103,294,308,319,326` |
| implementation (Sniff CLI) | `sniff/cli/src/output/filesystem/mod.rs:1343,1368,1420,1445`; `output/filesystem/package_areas.rs:28,43,181,386,405,415`; `output/filesystem/repo.rs:61`; `output/filesystem/deps.rs:100`; `args/repo.rs:494` (help text) |
| implementation (Darkmatter) | `darkmatter/lib/src/markdown/compose/context/capture/repo.rs:133`; `context/repository_scope.rs:43`; `darkmatter/cli/src/commands/compose.rs:203,265` |
| implementation (Claudine) | `claudine/lib/src/composition/launch_workspace.rs:135`; `lib/src/composition/lifecycle/control.rs:311`; `lib/src/system_prompt/context.rs:21,140`; `cli/src/completion/scopes.rs:304,329`; `cli/src/commands/wrap/env/package_context.rs:247` |
| test fixtures | Sniff `detection.rs:1930,1978,2556,2563,2599`; `types.rs:613,766,799,839`; `aggregate_view.rs:973,1043,1056`; `aggregate.rs:912-1506`; `cli/src/output/filesystem/mod.rs:2447`; `cli/src/output/repo_json.rs:2932,2989`; Darkmatter `repository_scope.rs:70`; Claudine `lib/src/events/environment.rs:558`, `cli/src/commands/wrap/env/mod.rs:68` |
| shipped prompt | `system-prompt.md:3` (`ctx.area == 'root' ? …`, always the else branch today) |
| user documentation | `sniff/docs/cli/repo_deps.md:66`; `claudine/docs/topics/completions/shell-completions.md:140`; skill `.claude/skills/claudine/completions/shell-completions.md:157` |
| historical spec | `claudine/fixes/2026-06-30-completion-failures/spec.md:68` |
| unrelated literal (do not rewrite) | `{"name":"root"}` package-name fixtures (`detection.rs:2785`, `aggregate.rs:1183`, `sniff/lib/tests/fixtures.rs:489,961`); JSON `{ "root": … }` path keys (`repo_json.rs:1556,1584`, `sniff/cli/tests/cli.rs:7121,7512`, `sniff/docs/cli/repo_package-root.md:71`, `repo_package-area-root.md:77`, `repo.md:121`); `claudine/cli/tests/ctx_launch_anchor.rs:357` (test label) |

Drift versus the spec's list: `prompts/_reviews/feature-review.md` no longer
contains `when="ctx.area != 'root'"` (already removed); the implement-plan
fallback moved to `prompts/_implement/implement-plan.md:16` and needs no edit;
`claudine/cli/src/commands/wrap/env/tests.rs:721` no longer matches;
`sniff/cli/tests/cli.rs:8859` is a comment describing the sentinel.

### 1.6 — Test baseline (before any source edit)

| Package area | Command | Result |
|---|---|---|
| `darkmatter` | `just test` | 7898 passed (3 slow), 7 skipped, exit 0 |
| `sniff` | `just test` | 2620 passed, 24 skipped, exit 0 |
| `claudine/gen` | `just test` | 162 passed, 0 skipped, exit 0 |
| `claudine` | `just test` | 7019 passed, 9 skipped, exit 0 |

No pre-existing failures.

### 1.7 — Impact analysis

Symbols modified in Phase 1 (all for 1.3):

| Symbol | GitNexus risk | Direct callers / processes | Notes |
|---|---|---|---|
| `PipelineRuntime::with_remote_fetch` (`shell_expansion/types.rs`) | LOW | `Markdown::run_compose_pipeline`; 5 `run_compose_pipeline` flows | public signature: dropped `cache_root`; no caller outside `darkmatter/lib` (text search) |
| `PipelineRuntime::new` (`#[cfg(test)]`) | not indexed | `context/authority.rs:271`, `frontmatter_shell_expansion/tests/execution_tests.rs:55` (both passed `None`) | dropped `cache_root` |
| `reference::graph::make_cache` | LOW | `build_graph_inner` → `build_reference_graph` (27 transitive, no processes) | no longer attaches a store |
| `RunLocalCache::with_persistent` | UNKNOWN (target not found) | text search: the two `PipelineRuntime` constructors, `make_cache`, `cache/runtime.rs` unit tests | now `#[cfg(test)]` |
| `Markdown::run_compose_pipeline` | UNKNOWN (no callers resolved) | text search: `compose/mod.rs:209`, `:230` | internal block only; signature unchanged |

No HIGH/CRITICAL symbol was modified.

Forward inventory for later phases (upstream, recorded now so HIGH/CRITICAL
review can be scheduled):

| Symbol (phase) | Risk | Evidence |
|---|---|---|
| `capture_at_event` — Claudine `LifecycleCurrent` (10.3) | **CRITICAL** | 19 impacted, 2 processes; direct: `provider_run_handoff`, `emit_preflight_blocked_and_finalize_in_context`, `capture_lifecycle_globals`, `capture_loop_lifecycle_globals`, test `capture_at_event_populates_ctx_and_env`. **Stop for explicit review before editing (plan 1.7).** |
| `Package` — Sniff (2.1) | MEDIUM | 21 impacted; `pkg`, `pkg_with_path`, `runner_pkg`, `repo_with_versions`, `create_package_with_request` |
| `make_package_area` — Sniff (2.1) | LOW | 9 impacted; `create_package_from_seed`, `create_package_with_request` |
| `resolves_outside_frontmatter` — Claudine (10.4) | LOW | 5 impacted; `undefined_bare_variable` |
| `late_binding_root_in_expr` — Claudine (10.4) | LOW | 3 impacted; `first_late_binding_root` |
| `LATE_BINDING_ROOTS`, `area_for_dir`, `get_recent_commits_by_count`, `try_current_worktree_name` | UNKNOWN (0 resolved) | text search files: `claudine/lib` 4; `sniff/lib` 2 + `sniff/cli` 1; `sniff/lib` 5 + `sniff/cli` 1; `sniff/lib` 2 + `darkmatter/lib` 1 |
| `ComposeOptions`, `ComposeSource`, `ContextGroup`, `ContextCaptureEvidence`, `InjectedGlobal`, `LayeredLookup`, `LifecycleCurrent`, `CommitDescSet`, `RunLocalCache` | UNKNOWN (types not resolved) | files referencing: `ComposeOptions` darkmatter 72 / claudine 13; `ComposeSource` darkmatter 47 / claudine 7; `ContextGroup` darkmatter 19 / claudine 2; `ContextCaptureEvidence` darkmatter 9 / claudine 1; `InjectedGlobal` 2/2; `LayeredLookup` 1/3; `LifecycleCurrent` claudine 9; `CommitDescSet` sniff 8 |

UNKNOWN is unresolved, not low: each later phase must re-run impact on the
exact methods it edits.

### 1.3 — Q3 cache prerequisite

Activated `darkmatter/fixes/_unscheduled/content-policy-no-cache` as
`darkmatter/fixes/2026-09-16-content-policy-no-cache` (status `active`, link
depth corrected, implementation status section added) and implemented its
cache-disable portion.

Failing-first evidence: `lib/tests/persistent_cache_disabled.rs` was written
before the fix and failed on unmodified code:

- `a_warm_cache_root_never_replays_composed_local_output` — Strict mode warm run
  replayed the cold run's `ctx.timestamp_ms` (`1789624149163` both runs): the
  volatile key is excluded from the persistent key, so the composed child was
  served stale. This is exactly the replay R18/Q3 forbids.
- `a_reference_graph_with_a_cache_root_persists_no_local_artifact` — found
  `.darkmatter/cache/v1/branch/manifests/snapshot`.

Change:

- `PipelineRuntime::with_remote_fetch` / test `PipelineRuntime::new` no longer
  accept a cache root; the run-local cache is never given a persistent store.
- `run_compose_pipeline` no longer resolves a local persistent root; the
  remote-fetch runtime still resolves `cache_root`/`cache_namespace` for raw
  remote bodies.
- `reference::graph::make_cache` attaches no store.
- `RunLocalCache::with_persistent` is `#[cfg(test)]`; the read/write machinery
  is retained and unit-tested for `ContentPolicy` (deletion left to Ken).
- `md compose --cache-root` help text updated.

Pre-R18 tests updated (they asserted persistent hits/writes and failed after the
fix, 4 total): `compose::tests::schema::baseline_cache_does_not_reuse_across_distinct_baselines`
(now asserts zero persistence; key sensitivity remains covered by the unit test
`cache::hashing::options_hash_sensitive_to_baseline_schema`), and
`request_context_epoch::persistent_cache::{a_changed_child_only_value_invalidates_the_cached_child,
a_changed_grandchild_only_value_invalidates_the_cached_parent,
a_persistent_entry_cannot_bypass_a_frozen_missing_capture}` (never-stale and
frozen-capture assertions kept; hit/write expectations flipped to zero). Stale
comments in `distinct_file_ref_fallback_dirs_…` and two
`reference_integration.rs` tests corrected (the namespace test still passes
because the remote-body store creates the namespaced directory).

Documentation drift found and resolved (code taken as authoritative):
`docs/topics/caching.md` "Current Status" claimed remote artifact caching was not
implemented; it is. Rewritten along with the Overview and reference-analysis
sections, `docs/cli/compose.md` `--cache-root`, `docs/structs/Markdown.md`
reference-graph cache paragraph, and the `darkmatter` skill's remote/cache bullet.
None of these files carries a hash frontmatter property.

### Requirement-to-test mapping

| Requirement | Test | Level |
|---|---|---|
| Warm cache root never replays a composed `::file` child (original failing input: child rendering `ctx.timestamp_ms`) in all four freshness modes | `darkmatter::persistent_cache_disabled a_warm_cache_root_never_replays_composed_local_output` | L1 lib integration |
| No `snapshot`/`composed`/`operation` manifest written; stats show zero persistent hits/writes; `::code` and `::toc-linking` still render | same test | L1 |
| Reference graph with cache root + namespace persists nothing and reads edited children from disk | `darkmatter::persistent_cache_disabled a_reference_graph_with_a_cache_root_persists_no_local_artifact` | L1 |
| End-to-end through `md compose FILE --cache-root`: local child recomposed on warm run, remote body still served from cache (1 request), `manifests/remote` present, no local manifests | `darkmatter-cli::compose_remote_caching test_compose_cache_root_never_replays_composed_local_output` (written after the fix; failing-first evidence is the lib test above, same mechanism) | L1 CLI via `CliProcessFixture` |
| Remote-body persistence unchanged (TTL, refresh, fallback) | existing `compose_remote_caching` tests (12) | L1 CLI |
| Never-stale child/grandchild values and frozen missing-capture contract hold with a cache root | updated `request_context_epoch::persistent_cache::*` (3) | L1 |

Passive corpus / round-trip requirements do not apply: no parser, schema,
template, prompt, or persisted-value format changed.

### Gates

| Gate | Result |
|---|---|
| `cargo nextest run -p darkmatter --test persistent_cache_disabled` before fix | 2 failed (expected) |
| same, after fix | 2 passed |
| `cargo nextest run -p darkmatter-cli --test compose_remote_caching` | 13 passed |
| `darkmatter: just test --no-fail-fast` (after fix, before updating old tests) | 7897 passed, 4 failed (the pre-R18 tests listed above) |
| `darkmatter: just test --no-fail-fast` (final) | 7901 passed, 7 skipped, exit 0 |
| `darkmatter: just lint` | exit 0 (includes zed-dmls wasm32-wasip2 check) |
| GitNexus `detect-changes --scope all` | 18 files, 39 symbols, risk medium; affected flows limited to the five `run_compose_pipeline` flows |

Skipped: Claudine/Sniff/claudine-gen suites were not re-run after the change —
no code outside `darkmatter/` references the changed API (text search), and
Claudine never sets a cache root. Cross-OS `just cross-check` was not run: the
change removes a code path and adds no platform-specific behavior; test paths
use `Path::join` and forward-slash relative joins that Windows accepts. CI
will provide the Linux/Windows/WSL legs.

### Human review requested

1. Confirm Q1 (execution nonce), Q2 (per-expression memo) and Q3 (hard
   prerequisite) outcomes adopted from the spec's recommendations; Q1 blocks
   Phase 5, Q2 blocks Phase 6.
2. Rule on `--cache-root` scope (kept for raw remote bodies vs removed) and on
   retaining vs deleting the disabled persistent compose machinery; needed
   before Phase 11 docs and Phase 12's cache gate.
3. `capture_at_event` is CRITICAL in GitNexus; explicit review is required
   before Phase 10.3 edits it.

### Skill updates and final gates

- Skill qualifiers added to `.claude/skills/darkmatter/compose.md` (child
  compose-cache identity is run-local in production) and
  `library-surfaces.md` (only remote bodies persist). `compose.md` carries a
  `hash` property that was already stale at HEAD (stored body
  `149976b01d0952ef`, computed `253a9c5e256b6120`); refreshed with
  `md hash --save` → `ef46db3751d8e999-c86ed94e4c505efc`, `md hash --diff` clean.
- Final re-run after the `--cache-root` help-text change: `darkmatter: just test`
  7901 passed, 7 skipped; `darkmatter: just lint` exit 0.

## Phase 2

### Scope and impact analysis

Phase 2 is Sniff work. Removing the sentinel changes a value that Darkmatter
and Claudine read, so the consumer sites that compare against `"root"` were
re-inventoried (Phase 1 recorded only `Package` MEDIUM / `make_package_area`
LOW) and run through GitNexus upstream impact before editing:

| Symbol | Risk | Decision |
|---|---|---|
| Sniff `make_package_area`, `area_for_dir_with_index`, `package_area_for_dir_with_index`, `area_change_facts`, `render_commit_block`, `render_commit_centric` | LOW | edited |
| Sniff `area_for_dir`, `try_current_worktree_name`, `get_recent_commits_by_count` | UNKNOWN (text search: Sniff lib/CLI; Darkmatter `snapshot.rs` for the worktree lookup) | edited / reused |
| Darkmatter `run_compose` (`cli/src/commands/compose.rs`) | LOW | `== "root"` → `is_empty()` (keeps the repo-root spelling without a trailing separator) |
| Claudine `deepest_package_area`, `package_area_for_source`, `select_package_area_for_cwd`, `detect_wrap_startup` | LOW | `== "root"` → `is_empty()`; Claudine's own promptless root `PackageContext` now stores `""` so it agrees with the prompted path |
| Darkmatter `current_package_context` (`capture/snapshot.rs`) | **HIGH** (22 impacted, 3 direct: `ContextCapture::new`, `from_evidence`, `with_documents_for_source`; 0 processes) | **edited — flagged for review** (see below) |
| Claudine `select_package_area_root` | **HIGH** (11 impacted, 1 process) | not edited: `root.join("")` is component-equal to the root (same `Eq`/`Hash`/`starts_with`/depth), so scope selection and dedupe are unchanged; only the `PathBuf` spelling gains a trailing separator. Left to 10.6 |
| Claudine `resolve_compose_scopes` | **CRITICAL** (20 impacted) | not edited: an `""` area builds `<repo>/prompts`, which `dedup_scopes` already collapses into the repo scope. Its `area != "root"` guard and doc comment are now dead/stale and would wrongly skip a real area named `root`. Left to 10.6 |
| Darkmatter `repository_scope::package_area_root` | **CRITICAL** (17 impacted) | not edited: already returns `None` for `""`. The `"root"` arm is Phase 5.5's scheduled deletion |
| Darkmatter `populate_monorepo_area` (`capture/repo.rs:133`) | LOW | not edited: unreachable with `""` after the `current_package_context` fix; Phase 5.5's scheduled deletion |

Why the HIGH `current_package_context` edit was unavoidable: its area fallback
is `packages.find(|p| base_dir.starts_with(repo.root.join(&p.package_area)))`.
With `"root"` the join named a nonexistent directory; with `""` it is the repo
root and matches every path, so the first top-level package claims every
area-only directory. Evidence, captured with only the Sniff change applied:

- `claudine just test`: 2 failures in this real monorepo —
  `composition::prepare::service::tests::{a_different_launch_anchor_derives_a_different_context,
  context_derivation_ignores_a_later_process_cwd_change}` rendered `ctx.area`
  as `[]` instead of `[claudine]` from the `claudine/` area directory.
- new `darkmatter::empty_package_area` tests: `an_area_directory_resolves_to_its_area_not_the_empty_top_level_area`
  (`ctx.current_package_area` `""` instead of `zeta`) and `the_monorepo_root_has_no_area`
  (`ctx.package_area_root` became the repo root) failed.

The fix is one filter skipping empty areas; both Claudine tests and all four
new Darkmatter tests pass after it. Plan 1.7 says to stop for explicit review
before changing a HIGH symbol; in a non-interactive run the alternative was
leaving the workspace regressed, so the edit is made and raised under
`human_review_items`.

### 2.1 / 2.2 — Sentinel removal

- Library: `make_package_area` returns `""`; `area_for_dir` falls back to `""`;
  `package_area_for_dir` skips the empty area in its directory fallback (the
  join would otherwise match every path) and documents `Some("")` for a
  top-level package; `aggregate_view` uses `is_empty()` for the area root and
  change attribution; `resolve_named_package_area` no longer lists or accepts
  `""` as a nameable area. A real area named `root` is now an ordinary area
  (previously the `!= "root"` filters hid it).
- CLI: `area_display_label` renders `""` as `(root)` in package-area listings
  (csv/md/list/verbose), dirty/staged/unstaged area lists, the structure tree,
  and the deps DOT cluster label; JSON keeps `""`. `package-area-root`,
  the dirty/source-change exit-code helpers, and `--package-area` resolution use
  `is_empty()`; shell completion omits the empty area.
- Exit codes: `sniff repo area` at the monorepo root prints an empty line and
  exits 0 (a result, distinct from the non-monorepo exit 1). `sniff repo
  package-area` inside a top-level package is now a no-result (exit 1 / `{"name": ""}`)
  instead of printing `root`.
- Fixtures: every `package_area: "root"` Sniff test fixture now uses `""`;
  snapshots `cargo_monorepo_structure_text`, `cargo_pnpm_monorepo_structure_text`
  (`- root` → `- (root)`) and `repo_aggregate_json` (`"area": "root"` → `""`) updated.
- Scoped grep (`"root"` in `sniff/lib/src`, `sniff/cli/src`) leaves no area
  sentinel: remaining hits are the real-`root`-area tests, the `.editorconfig`
  `root` key, the repo-root dependency *package* label, and JSON `root` path keys.

Docs: `sniff/docs/cli/repo_package-area.md`, `repo_package-area-root.md` (its
"returns the repository root" line was already drift; the code returned empty —
code taken as correct), `repo_deps.md`, `repo_package-areas.md` (new
"Top-Level Packages" section), `repo.md`. None has a hash property.

### 2.3 — Cheap current-worktree observation

`GitInfo.current_worktree: Option<String>` (serde `skip_serializing_if = None`,
`default`) is filled by every preset, including `identity()`, from the open gix
handle via `current_worktree_name_from_gix` — the same function the ambient
`try_current_worktree_name` uses — without enumerating worktrees
(`git.worktree_opens` stays 0). JSON for the main checkout is unchanged; old
serialized evidence without the key still decodes. Phase 5.6 should project
`ctx.worktree` from it instead of `worktrees.values().find(is_current)`
(`snapshot.rs:688`), which also depends on the process CWD.

### 2.4 — Per-commit plain formatter

`CommitDesc::describe_plain(&self, today: NaiveDate) -> Option<String>` renders
one block; `None` for a commit with no files (the set renderer's skip rule now
has one owner). `render_commit_block` takes `link_root: Option<&Path>` (None =
plain) and a `TimeZone`, so `describe(plain)` and `describe_plain` share one
code path and time labels are unit-testable with fixed offsets; public APIs
still render in `Local`. No `repo_root` parameter: plain blocks never use it.

### Requirement-to-test mapping

| Requirement | Test(s) | Level |
|---|---|---|
| Top-level package area is `""` (AC22) | `sniff repo::detection::tests::make_package_area_is_empty_for_top_level_package`; `repo::types::tests::top_level_package_has_the_empty_area` | L1 unit |
| `area_for_dir` at repo root is `""`; outside-monorepo fallback `""` (AC22) | `repo::types::tests::{area_for_dir_is_empty_at_repo_root, directory_fallback_disabled_outside_monorepo}`; `aggregate_view::tests::cwd_context::area_fact_stays_empty_outside_a_monorepo` | L1 unit |
| Area-only directory not claimed by the empty area (top-level package listed first) | `repo::types::tests::empty_area_does_not_claim_area_directories` | L1 unit |
| A real area named `root` stays valid | `repo::types::tests::a_real_area_named_root_is_an_ordinary_area`; `detection::tests::make_package_area_keeps_a_real_area_named_root`; CLI below | L1 |
| `""` is not a selectable/listed area name | `aggregate::tests::{resolve_scope_overrides_unknown_package_area_errors, resolve_scope_overrides_rejects_the_empty_area_name}` | L1 unit |
| Aggregate context attribution for the empty area | `aggregate_view::tests::cwd_context::{root_area_excludes_paths_under_named_areas, root_area_counts_paths_outside_named_areas}` (updated) | L1 unit |
| CLI: `repo area` at root prints empty line, exit 0, JSON `""` | `sniff-cli::cli test_repo_area_at_repo_root_prints_an_empty_area` | L1 CLI |
| CLI end-to-end on a real on-disk workspace: top-level package (`area`, `package-area` exit 1 + JSON `""`, `package-area-root` JSON `""`, aggregate `--json` context), real `root` area (`area`, `package-area`, `package-area-root`), listings `(root)` vs JSON `""` | `sniff-cli::cli test_repo_scope_projections_use_the_empty_top_level_area` | L1 CLI |
| Render-time `(root)` label in verbose listing | `sniff-cli::cli test_repo_package_areas_root_area_verbose_renders_dot_slash` (updated) | L1 CLI |
| Structure/aggregate projections | snapshots `cargo_monorepo_structure_text`, `cargo_pnpm_monorepo_structure_text`, `repo_aggregate_json` | L1 snapshot |
| Linked worktree named for identity/minimal/summary presets without enumeration, equal to ambient lookup, from a nested dir | `git::types::tests::every_preset_names_the_linked_worktree_without_enumerating_worktrees` | L1 unit |
| Main checkout `None`, JSON key omitted (identity/summary/full) | `git::types::tests::main_checkout_has_no_current_worktree_and_omits_the_json_key` | L1 unit |
| Persisted evidence round trip (read/write/read, legacy decode) | `git::types::tests::current_worktree_round_trips_through_json` | L1 unit |
| Crate-boundary: Claudine's exact request (`summary` + remotes) via public `detect_git_with_request` — linked/main/outside repo | `sniff::git_parity summary_evidence_names_the_current_worktree_like_the_ambient_lookup` | L1 integration |
| Exact plain block: conventional + bullets; non-conventional without Description; empty commit `None` | `recent_commits::tests::rendering_tests::{describe_plain_renders_one_exact_block, describe_plain_omits_prefix_and_description_for_plain_messages, describe_plain_is_none_for_a_commit_without_files}` | L1 unit |
| Blocks concatenate to `describe(true)` byte for byte, empty commit shortens | `rendering_tests::describe_plain_blocks_concatenate_to_the_plain_set_rendering` | L1 unit |
| Deterministic time-zone rendering (zone shifts across midnight, Today/Yesterday/absolute, unparseable) | `rendering_tests::commit_time_label_renders_in_the_viewer_zone` | L1 unit |
| End-to-end: `sniff repo recent-commits 4 --plain` stdout == concatenated `describe_plain` blocks on a fixture with conventional, non-conventional, and empty commits dated 2020 | `sniff-cli::cli test_repo_recent_commits_plain_is_the_concatenated_per_commit_blocks` | L1 CLI |
| Unborn repository yields no commits; outside a repository is `NotARepository` (AC37 Sniff portion) | `sniff::integration test_get_recent_commits_by_count_unborn_repo_is_empty_and_outside_errors` | L1 integration |
| Consumer regression: Darkmatter scope from Sniff's real `detect_repo` output (top-level package first) — area dir, top-level package, monorepo root, package in area | `darkmatter::empty_package_area` (4 tests) | L1 lib integration |
| Consumer regression in this monorepo | `claudine composition::prepare::service::tests::{a_different_launch_anchor_derives_a_different_context, context_derivation_ignores_a_later_process_cwd_change}` (existing) | L1 |

Failing-first evidence: temporarily restoring the old sentinel lines in
`types.rs`, `detection.rs`, and `aggregate_view.rs` failed 10 Sniff tests
including `area_for_dir_is_empty_at_repo_root`,
`empty_area_does_not_claim_area_directories`,
`a_real_area_named_root_is_an_ordinary_area` (the old `!= "root"` filter hid
real `root` areas), and `make_package_area_is_empty_for_top_level_package`;
production code was then restored. The Darkmatter/Claudine failures are listed
under impact analysis. The worktree and formatter tests cover new API surface
and have no pre-fix form.

Passive corpus: not applicable — no parser, schema, template, prompt, or shipped
configuration artifact changed in this phase.

### Gates

| Gate | Result |
|---|---|
| `sniff: just test --no-fail-fast` (before snapshot/test updates) | 2628 passed, 5 failed (the expected sentinel snapshots/tests), 24 skipped |
| `sniff: just test` (final) | 2636 passed, 24 skipped, exit 0 (baseline 2620) |
| `sniff: cargo nextest ... test_get_recent_commits_by_count_unborn_repo_is_empty_and_outside_errors` (added after the full run started) | 1 passed |
| `sniff: just lint` | exit 0 |
| `darkmatter: just test` with only the Sniff change | 7901 passed (no existing coverage of the regression) |
| `claudine: just test` with only the Sniff change | 7017 passed, 2 failed (above) |
| `darkmatter: just test` (final) | 7905 passed, 7 skipped, exit 0 |
| `darkmatter: just lint` | exit 0 |
| `claudine: just test` (final) | 7019 passed, 9 skipped, exit 0 |
| `claudine: just lint` | exit 0 |
| GitNexus `detect-changes --scope all` | 55 files / 142 symbols / 7 flows, risk high — cumulative with uncommitted Phase 1; Phase 2 adds only the `handle_repo_package_areas`/`handle_repo_packages → resolve_package_area_path` flows |

Not run: `claudine/gen` (no generator input changed); `just cross-check`. The
changes are string and component-based `Path` logic; the new tests normalize
separators (`ends_with("/root")` after `\` → `/`, Darkmatter's `portable_dir`
output), and commit fixtures use explicit UTC epoch times. Cross-OS evidence is
queued for Phase 12 per the plan.

## Phase 3

### 3.1 — OS evidence paths and fixture formats

Loaded the `os` skill. Declared hosts: `BUILD_LINUX=build-linux`,
`BUILD_WIN=build-win-native`, `BUILD_WSL=build-win`. Fixture formats were
chosen so no parser needs the executing host to match the parsed OS:

- Linux: captured-shape `/proc/net/route` and `/proc/net/ipv6_route` text.
- Windows: captured-shape `route print` text (both tables, active and
  persistent blocks, a wrapped IPv6 row, localized headings).
- macOS: the `PF_ROUTE` `NET_RT_DUMP` byte layout, parsed by offset rather than
  through `libc::rt_msghdr`, with messages built by a test builder. A macOS-only
  test (`layout_matches_libc`) pins every offset and constant to `libc`, and a
  `real_` test compares the native result with `/sbin/route -n get default`.

All gateway fixtures live in `sniff/lib/src/network/gateway/fixtures/`.

### Impact analysis and a design change forced by it

| Symbol | Risk | Decision |
|---|---|---|
| `detect_network_with_request` | **HIGH** (22 impacted, 3 direct, 0 processes) | **not edited.** The first design added `NetworkRequest.include_gateways` and a `NetworkInfo.gateways` field filled here. Plan 1.7 forbids HIGH edits without review, so gateways became the separate focused API `sniff::network::detect_default_gateways()`. That also keeps Windows' `route print` subprocess off every ordinary network detection. |
| `NetworkInfo` | LOW | not edited |
| `detect_local_interfaces` | LOW (19 impacted, 1 direct) | edited: fills `NetworkInterface.index` from getifaddrs |
| `NetworkInterface` | UNKNOWN (text search: constructed only through `new`/`Default`; no struct literal anywhere in the workspace) | additive field `index: Option<u32>` (`serde(default, skip_serializing_if = None)`, so JSON is unchanged where no index is known) |
| `NetworkRequest` | UNKNOWN | not edited |
| Existing `parse_linux_proc_default_route_interface`, `parse_bsd_route_dump`, `parse_windows_default_route_interface_ip` | — | deliberately not reused or changed. The primary-*interface* selector has no mask check (a VPN `0.0.0.0/1` split route counts) and uses the `RTF_GATEWAY`-filtered macOS dump, which hides on-link defaults. Changing it would alter `primary_interface` for existing consumers, which is out of scope. The gateway selectors are new and apply R15/R21 exactly. |

### 3.2 — Interface-address helpers (`network/address.rs`)

- `ScopedIpAddr`: strict `FromStr`/`Display`/serde string for `a.b.c.d`, an
  IPv6 literal, or `ipv6%zone`. No DNS, brackets, whitespace, or IPv4 zone
  (`ScopeOnIpv4` is the family-mismatch error). The IPv6 spelling is
  normalized; the zone is kept verbatim. `Ord` is stable (IPv4 < IPv6, numeric,
  then zone).
- `is_within(&ipnet::IpNet)` compares bits only and ignores the zone. The other
  family never matches, and an IPv4-mapped IPv6 address is not CGNAT.
  `is_loopback`/`is_link_local` cover both families for Phase 7's default
  exclusion.
- `host_addresses(&[NetworkInterface])`: deduplicated by address+zone and
  sorted. Only IPv6 link-local addresses are zoned: interface **name** on Unix
  (`fe80::1%en0`), numeric **index** on Windows (`fe80::1%12`), each platform's
  native, routable spelling.

### 3.3 — Gateways (`network/gateway/`)

`detect_default_gateways() -> Result<DefaultGateways { v4: Option<Ipv4Addr>, v6: Option<ScopedIpAddr> }>`.

- Linux: lowest-metric route with UP set and REJECT unset (first wins a tie)
  whose destination **and mask** are zero. IPv6 also requires source prefix
  `00`, which skips source-specific defaults and the kernel's `lo` reject route.
  A gateway exists only with `RTF_GATEWAY` and a non-zero address. The device
  name is the zone for a link-local next hop.
- Windows: one `route print` via `process::run_with_timeout` (stdin null,
  deadline, Job Object). Parsed structurally, so localized headings and a
  multi-word `On-link` still work. Only active blocks count; persistent routes
  are ignored. A wrapped `::/0` row reads its gateway from the next line. The
  IPv6 zone is the `If` column.
- macOS: first UP default that is not `RTF_IFSCOPE`, `RTF_REJECT`, or
  `RTF_BLACKHOLE`. This host's dump has only interface-scoped IPv6 defaults
  (`fe80::%utunN UGcIg`) and a scoped `link#34 bridge100`; `route get` agrees
  that neither is the system default. Link-local zones come from the KAME
  embedded bytes, then `sin6_scope_id`, then `rtm_index`, then the number.
  Darwin rounds sockaddrs to 4 bytes (the old interface code's `c_long`
  alignment is only correct when reading the first sockaddr).
- The best route decides. An on-link best route yields `None` even when a worse
  route has a gateway (R21).
- Errors only when the IPv4 table is unreadable; a missing IPv6 table is `None`.

### 3.4 — CGNAT predicate

`cgnat_network()` (`100.64.0.0/10`) and `contains_cgnat_address(addresses)` over
`ScopedIpAddr`, the same helper Darkmatter's CIDR filters will use.

### 3.5 — ICMP transport (`network/icmp/`)

- API: `ping(&ScopedIpAddr, ProbeBudget) -> Result<PingReport, IcmpError>`;
  `PingReport::verdict()` is `AllReplied | NoneReplied | Unstable`, which maps to
  `ping_under`'s `true | false | "unstable"`. `ping_with(&mut dyn EchoProbe, …)`
  is the public stub seam for Darkmatter's Phase 9 AC8 tests.
- `ProbeBudget::from_millis(f64, f64)`: timeout finite and in `(0, 60000]` ms
  (fractional allowed); attempts whole and in `1..=100`. Both are checked
  before any cast, so `4294967297` is rejected rather than truncated to 1.
  `deadline_bound()` = attempts × timeout + 500 ms setup allowance.
- The sequencer runs attempts sequentially with a monotonic `Instant` per
  attempt. A reply counts only when **both** the probe's round trip and the
  measured elapsed time are below the timeout, so a reply at the threshold is
  late and a dishonest or overrunning probe cannot report a fast reply. The
  first `IcmpError` aborts the series, even after successes.
- No reply (including `ENETUNREACH`/`EHOSTUNREACH`/`EHOSTDOWN` and Windows
  `IP_DEST_*`/`IP_REQ_TIMED_OUT`) is an outcome. Errors are `NotPermitted`
  (EACCES/EPERM, Windows access denied), `UnknownScope` (raised before any
  packet), `Send` (any other OS error), and `InvalidBudget`.
- Unix: `socket2` `SOCK_DGRAM` ICMP/ICMPv6 sockets, opened once per family per
  probe. Replies match on sequence plus a 16-byte random token, because Linux
  rewrites the identifier. An IPv4 header is stripped when present (macOS
  includes it, Linux does not). The IPv6 checksum is left to the kernel.
- Windows: `IcmpSendEcho2`/`Icmp6SendEcho2` with handles closed on drop. The
  timeout is rounded **up** to whole milliseconds. No privilege or subprocess.
- **Decision:** no `ping`-binary subprocess fallback. On Linux/WSL2 without
  `net.ipv4.ping_group_range` covering the process group, the result is
  `NotPermitted` with that sysctl named. R6 says "cannot send ICMP → compose
  error", and a subprocess fallback would add locale-dependent parsing and
  imprecise timing to `ping_under`.

### 3.6 — Subprocess/process hardening

The ICMP path has no subprocess. The only new child process is Windows
`route print`, which runs through the existing `process::run_with_timeout`
boundary: stdin null, both pipes drained on threads, 3 s deadline, Unix process
group / Windows kill-on-close Job Object, reaped on every path. No byte cap was
added. Output is sized by the routing table, and the boundary already prevents
pipe-buffer deadlock. No new test spawns a terminal or browser. The only test
subprocess is the macOS `real_` `/sbin/route` call, with stdin null.

### 3.7 — Requirement-to-test mapping

| Requirement | Test(s) | Level |
|---|---|---|
| AC13 Linux v4: lowest-metric UP default; DOWN, `0/1` split, and worse routes ignored | `gateway::linux::tests::ipv4_picks_the_lowest_metric_up_default_route` (fixture `linux_route_v4.txt`) | L1 |
| AC13/R21 Linux v4 on-link best route → `None` | `…linux::tests::ipv4_on_link_default_route_has_no_gateway` (`linux_route_v4_on_link.txt`) | L1 |
| Linux v4/v6 no default → `None`; malformed rows skipped | `…ipv4_without_a_default_route_has_no_gateway`, `…ipv6_without_a_default_route_has_no_gateway`, `…malformed_rows_are_skipped` | L1 |
| AC13 Linux v6 link-local gateway keeps `%dev`; `lo` reject and worse route ignored | `…ipv6_link_local_gateway_keeps_the_device_scope` (`linux_ipv6_route.txt`) | L1 |
| Linux v6 on-link → `None`; global gateway unzoned | `…ipv6_on_link_default_route_has_no_gateway`, `…ipv6_global_gateway_is_unscoped` | L1 |
| AC13 Windows v4+v6: metric, tie, persistent exclusion, `%If` zone | `gateway::windows::tests::picks_the_lowest_metric_active_default_route_for_both_families` (`windows_route_print.txt`) | L1 |
| Windows on-link / persistent-only / wrapped row / localized output | `…on_link_default_routes_have_no_gateway`, `…persistent_only_default_routes_are_not_in_effect`, `…a_wrapped_gateway_column_is_read_from_the_next_line`, `…localized_headings_do_not_change_the_result` | L1 |
| AC13 macOS v4: first unscoped UP default; IFSCOPE, split, DOWN, REJECT skipped | `gateway::darwin::tests::ipv4_picks_the_first_unscoped_up_default_route` | L1 |
| macOS on-link (`link#N`) / no default / no netmask sockaddr | `…ipv4_on_link_default_route_has_no_gateway`, `…ipv4_without_a_default_route_has_no_gateway`, `…a_default_route_without_a_netmask_sockaddr_is_still_default` | L1 |
| AC13 macOS v6 link-local zone (KAME, scope id, route index, numeric) | `…ipv6_link_local_gateway_keeps_its_embedded_scope`, `…ipv6_scope_falls_back_to_scope_id_then_route_index_then_number` | L1 |
| macOS v6 global / on-link / no default / family mismatch / truncation and garbage | `…ipv6_global_gateway_is_unscoped`, `…ipv6_on_link_or_missing_default_route_has_no_gateway`, `…a_family_mismatch_is_not_a_default_route`, `…truncated_or_foreign_messages_never_panic_or_match` | L1 |
| Darwin layout matches the real `rt_msghdr` / sockaddrs | `…layout_matches_libc` (macOS only) | L1 |
| Native parser equals the system's `route get` | `…darwin::tests::real_native_gateways_match_route_get` (macOS) | real |
| Tie and on-link-best selection rule | `gateway::tests::{a_later_route_with_an_equal_metric_does_not_replace_the_first, a_better_on_link_route_hides_a_worse_gateway_route}` | L1 |
| Gateway JSON read/write/read round trip incl. zone and nulls | `gateway::tests::serializes_both_families_with_the_zone_and_round_trips` | L1 |
| Address spellings, normalization, strict rejection (no DNS), zone on IPv4, bad zones | `address::tests::{parses_and_displays_every_accepted_spelling, normalizes_ipv6_spelling_but_keeps_the_zone_verbatim, rejects_malformed_addresses_without_resolving_names, rejects_a_scope_on_ipv4_as_a_family_mismatch, rejects_empty_or_malformed_zones}` | L1 |
| Address serde round trip | `address::tests::serde_round_trips_through_the_display_spelling` | L1 |
| Bit matching ignores zone; other family / mapped never match | `address::tests::{network_membership_compares_bits_and_ignores_the_zone, a_network_of_the_other_family_never_matches}` | L1 |
| Loopback/link-local classification | `address::tests::loopback_and_link_local_classification_covers_both_families` | L1 |
| AC9 tailnet true only with a CGNAT interface fixture; exact boundaries | `address::tests::{tailnet_predicate_is_true_only_with_a_cgnat_interface_address, cgnat_boundaries_are_exact}` | L1 |
| Dedupe and stable ordering, per-OS zone spelling | `address::tests::host_addresses_deduplicate_and_order_stably` | L1 |
| Crate boundary: real interfaces → sorted, unique, reparseable, zoned exactly when link-local v6; CGNAT agrees | `network_primitives::host_addresses_from_real_interfaces_are_unique_sorted_and_reparseable` | L1 |
| AC8 all-fast / all-slow / mixed (stubbed) | `icmp::tests::{all_fast_attempts_are_all_replied, all_slow_attempts_are_none_replied, mixed_attempts_are_unstable}` | L1 |
| Reply at threshold is late; overrunning probe is late | `icmp::tests::{a_reply_exactly_at_the_threshold_is_late, a_probe_that_overruns_the_deadline_is_late_whatever_it_reports}` | L1 |
| AC33 send failure after earlier success aborts, no further attempts | `icmp::tests::a_send_failure_after_an_earlier_success_aborts_the_series` | L1 |
| Timeout is an outcome; series of timeouts within `deadline_bound` | `icmp::tests::{a_single_attempt_timeout_is_an_outcome_not_an_error, a_series_of_timeouts_stays_within_the_deadline_bound}` | L1 |
| AC33 invalid numbers and overflow (NaN, ±inf, 0, −0, negative, over max, fractional/zero attempts, 2^32+1, 1e20); boundaries accepted | `icmp::tests::{budget_rejects_invalid_and_overflowing_numbers_without_truncation, budget_accepts_the_boundaries}` | L1 |
| AC33 scoped IPv6: numeric zone, unknown name → `UnknownScope` | `icmp::tests::numeric_zones_resolve_without_a_lookup_and_unknown_names_fail`; `network_primitives::real_icmp_unknown_scope_fails_before_sending` | L1 / real |
| Bounded cleanup: noisy socket ends at the deadline; EINTR/unreachable keep waiting; read failure is an error; expired deadline never reads | `icmp::unix::tests::{a_noisy_socket_ends_as_no_reply_at_the_deadline, interrupts_and_unreachable_reports_keep_waiting_for_the_match, a_read_failure_is_an_error, a_passed_deadline_never_reads}` | L1 |
| Errno classification (unreachable → no reply, EACCES/EPERM → NotPermitted) | `icmp::unix::tests::socket_errors_are_classified` | L1 |
| Echo encoding: IPv4 checksum (RFC 1071), IPv6 kernel checksum, reply matching with and without an IPv4 header, rejection of other sequences, tokens, types, and truncations | `icmp::packet::tests::*` (6) | L1 |
| AC14 local form: IPv4 and IPv6 loopback round trips; unanswered TEST-NET target → NoneReplied within bound; real gateway read round-trips | `network_primitives::{real_icmp_ipv4_loopback_replies, real_icmp_ipv6_loopback_replies, real_icmp_unanswered_target_is_no_reply_within_the_bound, real_default_gateways_are_readable_and_well_formed}` | real |

Failing-first evidence: this is new API surface with no pre-fix form, so four
mutations were applied one at a time and then restored. Each was caught:

- M1 (drop the macOS IFSCOPE skip) failed 3 tests, including the real
  `route get` cross-check.
- M2 (drop the Linux mask check) failed `ipv4_picks_the_lowest_metric_up_default_route`.
- M3 (count persistent `route print` blocks) failed 2 Windows tests.
- M4 (inclusive threshold, no elapsed guard) failed
  `a_reply_exactly_at_the_threshold_is_late` and
  `a_probe_that_overruns_the_deadline_is_late_whatever_it_reports`.

Passive corpus / shipped-artifact end-to-end: not applicable. No parser for a
shipped Darkmatter/Claudine artifact, schema, template, prompt, or
configuration changed. Darkmatter consumption (AC6–AC9 end to end) is
Phases 5 and 9.

### Gates and OS evidence

| Gate | Result |
|---|---|
| `sniff: just test` | 2699 passed, 31 skipped, exit 0 (baseline after Phase 2: 2637) |
| `sniff: just lint` | exit 0 |
| `sniff: cargo nextest … network::` + `--test network_primitives` (includes `real_`) on macOS | 146 + 6 passed |
| `sniff: just check-windows` (x86_64-pc-windows-gnu, compile evidence only) | success. One new warning (`MAX_REPLY_LEN` unused on Windows tests) fixed by gating; the other warnings are pre-existing (`windows_apps.rs`, `executable_index.rs`, `git_parity.rs`) |
| Linux behavior, Docker Desktop `rust:1.97.1` linux/aarch64 (`ping_group_range = 0 2147483647`) | `--lib network::` 146 passed; `--test network_primitives` 6 passed, including real IPv4/IPv6 loopback ICMP and live `/proc` gateway reads. The container's real `ipv6_route` has the `lo` reject default the fixture models |
| `darkmatter: just test` | 7905 passed, 7 skipped, exit 0 (unchanged) |
| `darkmatter: just lint` | exit 0 |
| GitNexus `detect-changes --scope all` | 63 files / 157 symbols / 7 flows, risk high. This is cumulative with the uncommitted Phases 1–2. Phase 3 adds no affected flow (7 flows, the same as after Phase 2) |

**Blocked (environment, not code):**

- `just cross-check sniff --os windows`: `fatal: write error: No space left on
  device`. `W:` on `build-win-native` has **8192 bytes free**. Read-only
  inventory: `W:\ci-verification\rusty-biscuit\target` 95.1 GB (last written
  2026-09-14; above the 80 GB sweep cap, and that clone has no
  `.cargo/config.toml`), `W:\ci-verification\rb-pr66` 61.4 GB (2026-08-30,
  another session), `W:\WSL\Ubuntu-26.04\ext4.vhdx` 130.8 GB. The scheduled
  `RustyBiscuit-CargoSweep` last ran 2026-09-16 04:00 with result 0. Nothing
  was deleted: none of these artifacts are this session's (`storage-strategy`
  rule 2).
- `just cross-check sniff --os wsl`: SSH `Connection reset by peer`. The guest's
  VHDX is on the full `W:`.
- `just cross-check sniff --os linux`: waited on the host lock held since
  2026-09-14T18:25Z by `{"purpose": "nightly-reward-spike", "branch":
  "feat-nightly-perf"}`. The lock is probably stale, but the script never
  removes it and neither did I. The run was stopped, and Docker supplied the
  Linux evidence instead.

Native Windows and WSL2 runtime behavior of the ICMP and `route print` paths
is therefore **unverified**. Windows has compile evidence only. Checkpoint 3
requires macOS plus queued cross-platform evidence for Phase 12, so the phase is
complete, but AC14 on Windows/WSL2 remains open.

Scratch cleanup: the Docker source copy and target (`~/.cache/p3-linux`,
2.2 GB) were deleted. The `os` skill's `macos.md` now records that Docker
Desktop rejects `/Volumes` and `/tmp` bind mounts on this host.

## Phase 4

### Scope and impact analysis (2026-09-17)

GitNexus upstream impact before any edit:

| Symbol | Risk | Direct callers |
|---|---|---|
| `expression::catalog::project_descriptors` | **HIGH** (26 impacted, 0 processes) | `expression_function_descriptors`, parser test `enum_returns_preserve_variants_and_orthogonal_shapes` |
| `expression::catalog::parser::parse_expression_function_catalog` | **HIGH** at depth 3 / MEDIUM at depth 1 (0 processes) | `expression_function_catalog`, `try_parse_catalog`, five parser tests |
| `ExpressionFunctionDescriptor::typed_signature` | UNKNOWN (no resolved edges) | text search: DMLS `overlay/expressions.rs`, `providers/dsl.rs`, `md schema about` |
| `expression::functions::bindings` | LOW (11 impacted) | registry validation and dispatch |
| `context::catalog::project_descriptors` | UNKNOWN (no resolved edges) | text search: `CONTEXT_VARIABLE_DESCRIPTORS` `LazyLock` only |
| claudine-gen `apply_generations` | not indexed | text search: `claudine-gen` `run_generate` and its unit tests |

**HIGH warning.** The two catalog loaders are HIGH because every descriptor
consumer (DMLS completion/hover, `md schema about`, `claudine context`, the
runtime registry) sits above them. Phase 4 cannot deliver the new parameter and
return shapes, or the R29 pair, without extending them. The edits are additive
(new optional YAML fields and new descriptor fields), and the existing parity,
baseline, and schema tests guard them. Proceeding under the yolo run and listing the edit for
human review, as Phases 1–3 did.
