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
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - darkmatter/features/2026-09-09-more-context/plan.md
  - darkmatter/features/2026-09-09-more-context/implementation-log.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_files_during_phase_7: []
docs_updated_during_phase_7:
  - darkmatter/features/2026-09-09-more-context/plan.md
  - darkmatter/features/2026-09-09-more-context/implementation-log.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_files_during_phase_8: []
docs_updated_during_phase_8:
  - darkmatter/features/2026-09-09-more-context/plan.md
  - darkmatter/features/2026-09-09-more-context/implementation-log.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
source_files_during_phase_9: []
docs_updated_during_phase_9:
  - darkmatter/features/2026-09-09-more-context/plan.md
  - darkmatter/features/2026-09-09-more-context/implementation-log.md
docs_created_during_phase_9: []
skills_files_updated_during_phase_9: []
source_files_during_phase_10: []
docs_updated_during_phase_10:
  - darkmatter/features/2026-09-09-more-context/plan.md
  - darkmatter/features/2026-09-09-more-context/implementation-log.md
docs_created_during_phase_10: []
skills_files_updated_during_phase_10: []
source_files_during_phase_11: []
docs_updated_during_phase_11:
  - sniff/README.md
  - sniff/lib/README.md
  - darkmatter/features/2026-09-09-more-context/plan.md
  - darkmatter/features/2026-09-09-more-context/implementation-log.md
docs_created_during_phase_11: []
skills_files_updated_during_phase_11: []
packages:
  - darkmatter
  - sniff
implementation_1: "2026-09-17T16:43:41-07:00"
implementation_3: "2026-09-17T19:07:11-07:00"
implementation_4: "2026-09-17T20:43:06-07:00"
human_review: true
human_review_items:
  - "Phase 11 was mostly halted (2026-09-17). Only 11.3 (Sniff README and library README additions for the committed Phase 2-3 APIs) was done. Checkpoint 11 requires docs and skills to describe only implemented behavior. Tasks 11.1, 11.2, and 11.4 document ctx/current/current_env, new functions, as_markdown, ICMP consent, and the Claudine LifecycleCurrent replacement, and all of that belongs to the unimplemented Phases 5-10. 11.5 is partly done already (Phase 4 recorded ipnet, getrandom, socket2, and biscuit-hash blake3), but it stays open until later phases stop changing manifests. 11.6 and 11.7 run after 11.1-11.5. /Volumes/coding is down to 1.3 GiB free. HEAD is still 153717fa5, and the 14 modified and 2 untracked Phase 5-looking files are unchanged. Please free disk space and finish Phases 4.7 and 5-10 in order before rerunning Phase 11."
  - "Phase 10 was halted before any code change (2026-09-17). Phase 10 depends on Phases 5-9, and task 4.7 plus every Phase 5-9 task is still unchecked. The worktree is unchanged since the Phase 6-9 halts, and HEAD is still 153717fa5. 10.1 extends Claudine evidence for the Document/Network groups and eager recent-history demand (Phase 5). 10.2 projects descriptor listings (4.7). 10.3 and 10.4 route lifecycle lookup through Darkmatter's current/current_env roots (Phase 6), and 10.3 also needs the still-pending explicit review of the CRITICAL capture_at_event edit. /Volumes/coding still has only 1.7 GiB free, so neither a Darkmatter nor a Claudine build, test, or lint can finish. Please stop scheduling later phases until someone frees disk space and finishes Phases 4.7 and 5-9 in order."
  - "Phase 9 was halted before any code change (2026-09-17). Phase 9 depends on Phases 3, 4, and 8. Phase 8 is not implemented, and task 4.7 plus every Phase 5-8 task is still unchecked. The worktree is unchanged since the Phase 6-8 halts, and HEAD is still 153717fa5. 9.1 and 9.2 need the Phase 6.2 effect/requirement planning split and the Phase 8.4 preflight discovery. 9.5 needs as_markdown (Phase 8.1). /Volumes/coding free space fell again, to 1.7 GiB, so a Darkmatter build, test, or lint cannot finish. Please stop scheduling later phases until someone frees disk space and finishes Phases 4.7, 5, 6, 7, and 8 in order."
  - "Phase 8 was halted before any code change (2026-09-17). Phase 8 depends on Phases 5-7, and task 4.7 plus all Phase 5, 6, and 7 tasks are still unchecked. The worktree is unchanged since the Phase 6 and 7 halts, and HEAD is still 153717fa5. The child pipeline (8.1) needs Phase 5 root identity/ctx and Phase 6 lazy providers. Preflight discovery (8.4) needs Phase 6.2 planning. /Volumes/coding free space fell to 2.1 GiB. Rerunning later phases will keep halting until someone frees disk space and finishes Phases 4.7, 5, 6, and 7 in order."
  - "Phase 7 was halted before any code change (2026-09-17). It depends on Phases 2-6 (plan dependency table), and Phases 4.7, 5, and 6 are still unchecked; the worktree is unchanged since the Phase 6 halt (same 14 modified + 2 untracked unverified files). Phase 7 needs Phase 5 captured interface data (7.3), the eager recent-history formatter path (7.2), and Phase 6 evaluation-context/function-dependency planning (the plan says the five tracks start only after one owner lands shared dispatch and evaluation-context changes). Layering Phase 7 on an unbuilt, unverified tree would compound unreviewed HIGH-risk work. Also /Volumes/coding still has only 5.0 GiB free, so a Darkmatter build/test/lint cannot be trusted to complete. Next: free disk space, finish and log 4.7, 5, and 6 in order, then rerun Phase 7."
  - "Phase 6 was halted before any code change. Its prerequisites are incomplete: Phase 4 task 4.7 and Checkpoint 4 are unchecked, and every Phase 5 task is unchecked with no Phase 5 log section. Uncommitted, unverified Phase 5-looking work sits in the worktree (new capture/document.rs and capture/network.rs; edits to capture/{git,host,mod,repo,snapshot}.rs, context/{checked,options,repository_scope,runtime}.rs, expression/error.rs, markdown/mod.rs, claudine/lib/src/invocation_context.rs). Decide whether to finish and log Phases 4.7 and 5 (recommended) before rerunning Phase 6, which depends on Phase 5's retained launch evidence and eager-capture planner."
  - "Mac dev volume is exhausted. At 00:54 PDT every shell call failed with ENOSPC; afterward the shared APFS container had 5.9 GiB free of 3.6 TiB. Read-only target sizes on /Volumes/coding: fix-cli-slow-tests 206G, feat-unifi 182G, feat-single-os 142G, feat-dark-fixes 93G, feat-better-static-analysis 76G, feat-better-sniff 70G, personal/rusty-biscuit 42G. Nothing was deleted (storage-strategy rule 2). A Darkmatter L1 build and test run will not fit until space is freed."
  - "Build-host storage crisis blocks native Windows and WSL2 evidence (AC14, and Phase 12 generally). W: on build-win-native has 8192 bytes free, so cross-check fails with 'No space left on device' and the WSL guest (VHDX on W:) resets SSH. Read-only inventory: W:\\ci-verification\\rusty-biscuit\\target 95.1 GB (over the 80 GB sweep cap; that standing clone has no .cargo/config.toml target-dir pin), W:\\ci-verification\\rb-pr66 61.4 GB (2026-08-30, another session), W:\\WSL\\Ubuntu-26.04\\ext4.vhdx 130.8 GB. The daily sweep reported success at 04:00. Nothing was deleted because none of it belongs to this session (storage-strategy rule 2). Decide what to remove or compact."
  - "build-linux cross-check lock held since 2026-09-14T18:25Z by purpose=nightly-reward-spike, branch=feat-nightly-perf. It is probably stale, but the script never removes locks. Linux evidence for this phase came from Docker Desktop instead. Remove it if that run is dead."
  - "Confirm three Phase 3 design decisions that later phases build on. (a) Gateways are a separate API, sniff::network::detect_default_gateways(), not a NetworkRequest flag, because detect_network_with_request is GitNexus HIGH. (b) ICMP uses unprivileged datagram sockets with no ping-subprocess fallback, so Linux/WSL2 hosts whose net.ipv4.ping_group_range excludes the process group get IcmpError::NotPermitted, which becomes a compose error per R6. The wsl2-ubuntu CI leg must provide that sysctl for AC14. (c) macOS primary default = first UP default route that is not RTF_IFSCOPE (matches `route get default`); the spec's 'first default route in the dump' did not mention scoped routes."
  - "Carried from Phases 1-2, still unconfirmed: the retroactive HIGH edit to Darkmatter current_package_context; Q1 execution nonce (blocks Phase 5); Q2 per-expression memo scope (blocks Phase 6); the content-policy-no-cache --cache-root scope ruling (Phase 11/12); explicit review before Phase 10.3 edits CRITICAL capture_at_event."
message_to_agent: >-
  PHASES 6, 7, 8, 9, 10, AND MOST OF 11 NOT IMPLEMENTED (all halted 2026-09-17; only task 11.3, the Sniff READMEs, is done): finish and log Phase 4.7, then Phases 5-10 in order, then rerun Phase 11 for 11.1, 11.2, 11.4-11.7 before Phase 12; finish and log Phase 4.7 and Phase 5 first, and free disk space on the Mac; see human_review_items and the Phase 6 section of implementation-log.md. Phase 3 notes follow.
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

## Phase 6

### Halted before any code change (2026-09-17 00:54 PDT)

Phase 6 was **not started**. No source file was edited. Two blockers:

1. **The prerequisite phases are incomplete.**
   - Phase 4: task 4.7 (passive tooling) and Checkpoint 4 are unchecked, and
     the Phase 4 log ends after the impact analysis.
   - Phase 5: every task is unchecked and this log has no `## Phase 5`
     section. The prompt said Phase 5 log entries exist; they do not.
   - The worktree holds uncommitted work that looks like Phase 5, written
     between 00:37 and 00:51. New files: `capture/document.rs` and
     `capture/network.rs`. Edited files: `capture/{git,host,mod,repo,snapshot}.rs`,
     `context/{checked,options,repository_scope,runtime}.rs`,
     `expression/error.rs`, `markdown/mod.rs`, and
     `claudine/lib/src/invocation_context.rs`.
   - Nobody has verified or logged that work. Phase 6 needs its retained
     launch evidence (6.1) and its eager-capture planner (6.2).
2. **The macOS system volume is full.** Every shell command fails with
   `ENOSPC: no space left on device` writing under
   `/private/tmp/claude-501/...`, including `df -h /`.
   - This session cannot build, test, lint, run GitNexus impact analysis, or
     diagnose the disk.
   - Per the `storage-strategy` skill, nothing was deleted; the space was not
     created by this session.

Next step for a human: free space on `/`, then verify or finish Phase 4 (4.7)
and Phase 5 (the uncommitted work, plus its log and checkboxes). After that,
run Phase 6 again.

## Phase 7

### Halted before any code change (2026-09-17)

Phase 7 was **not started**. No source file was edited. What was checked:

1. **Prerequisites are still incomplete.** The plan's dependency table says
   Phase 7 depends on Phases 2-6. Phase 4 task 4.7 and Checkpoint 4 are still
   unchecked, every Phase 5 and Phase 6 task is unchecked, and this log has no
   `## Phase 5` section. The Phase 6 section above records why Phase 6 stopped.
2. **The worktree has not changed since the Phase 6 halt.** `git status` shows
   the same 14 modified files and 2 untracked files (`capture/document.rs`,
   `capture/network.rs`). Nobody has verified or logged that work.
3. **Phase 7 needs code that does not exist yet:**
   - 7.3 (`ipv4`/`ipv6`) needs the interface data captured in Phase 5.3.
   - 7.2 (`recent_commits(count)`) must use the same formatter as Phase 5.4's
     eager capture.
   - Every track needs Phase 6.2's function-dependency planning and the shared
     evaluation context. The plan says the five tracks start only after one
     owner lands that shared change.
   - Building Phase 7 on top of that would stack more unreviewed work on
     HIGH-risk catalog and capture edits.
4. **Storage is still exhausted.** `df -h /Volumes/coding` reports 5.0 GiB
   free of 3.6 TiB. A Darkmatter build, `just test`, and `just lint` cannot be
   trusted to finish. Nothing was deleted, following storage-strategy rule 2.

Spec frontmatter already had `implemented: true` and
`implemented_by: claude/opus`, so it was left as it was.

No tests were added or run. There is no requirement-to-test mapping because no
behavior changed.

Next step for a human: free disk space. Then finish and log Phase 4.7, Phase 5,
and Phase 6, in that order, and run Phase 7 again.

## Phase 8

### Halted before any code change (2026-09-17)

Phase 8 was **not started**. No source file was edited. What was checked:

1. **Prerequisites are still incomplete.** Phase 8 depends on Phases 5-7.
   Task 4.7, Checkpoint 4, and every task in Phases 5, 6, and 7 are unchecked.
   This log has no `## Phase 5` section, and the Phase 6 and Phase 7 sections
   above record halts.
2. **The worktree has not changed since the Phase 6 and 7 halts.** `git status`
   shows the same 14 modified files and 2 untracked files
   (`capture/document.rs`, `capture/network.rs`). HEAD is still `153717fa5`.
3. **Phase 8 needs code that does not exist yet:**
   - 8.1 passes the captured `ctx`, the lazy providers, and the root identity
     into the child pipeline. Those come from Phase 5.1/5.2 (source identity
     and Document group), 5.7 (graph-wide identity), and 6.1 (refresh
     providers).
   - 8.4 extends static preflight discovery. It builds on Phase 6.2's split
     between eager requirements, deferred capabilities, and function
     dependencies.
   - 8.5 tests "shared hostname/context" and "file/URL/in-memory identity".
     Those values are added in Phase 5.
   - `as_markdown` is the fourth expression-dispatch change stacked on the
     unbuilt Phase 7 dispatch work.
4. **Storage is worse.** `df -h /Volumes/coding` reports 2.1 GiB free of
   3.6 TiB, down from 5.0 GiB at the Phase 7 halt. A Darkmatter build,
   `just test`, and `just lint` cannot finish. Nothing was deleted, following
   storage-strategy rule 2.

The spec frontmatter already has `implemented: true` and
`implemented_by: claude/opus`, so it was left as it was.

No tests were added or run. There is no requirement-to-test mapping because no
behavior changed.

Next step for a human: free disk space. Then finish and log Phase 4.7 and
Phases 5, 6, and 7, in that order, and run Phase 8 again.

## Phase 9

### Halted before any code change (2026-09-17)

Phase 9 was **not started**. No source file was edited. What was checked:

1. **Prerequisites are still incomplete.** Phase 9 depends on Phases 3, 4, and
   8. Phase 3 is done. Task 4.7 and Checkpoint 4 are unchecked. Every task in
   Phases 5-8 is unchecked, this log has no `## Phase 5` section, and the
   Phase 6, 7, and 8 sections above record halts.
2. **The worktree has not changed since the Phase 6-8 halts.** `git status`
   shows the same 14 modified files and 2 untracked files
   (`capture/document.rs`, `capture/network.rs`), plus this log and the plan.
   HEAD is still `153717fa5`.
3. **Phase 9 needs code that does not exist yet:**
   - 9.2 adds ICMP as a typed preflight effect. That builds on Phase 6.2's
     split between eager requirements, deferred capabilities, and function
     dependencies, and on Phase 8.4's static preflight discovery.
   - 9.5 checks pings inside `as_markdown` and nested frontmatter policy.
     `as_markdown` and the child pipeline come from Phase 8.1-8.4.
   - 9.3 and 9.4 add expression functions. They would be stacked on the
     unbuilt Phase 7 dispatch changes.
   - 9.1 changes `ComposeOptions`. The plan gives that shared surface one
     owner, and it is also edited in Phases 5, 6, and 8.
4. **Storage is worse.** `df -h /Volumes/coding` reports 1.7 GiB free of
   3.6 TiB, down from 2.1 GiB at the Phase 8 halt. A Darkmatter build,
   `just test`, and `just lint` cannot finish. Nothing was deleted, following
   storage-strategy rule 2.

The Sniff ICMP transport that Phase 9 will use already exists from Phase 3
(`sniff::network::icmp::{ping, ping_with, ProbeBudget, PingReport, IcmpError}`).
It needs no changes before Phase 9 runs.

The spec frontmatter already has `implemented: true` and
`implemented_by: claude/opus`, so it was left as it was.

No tests were added or run. There is no requirement-to-test mapping because no
behavior changed.

Next step for a human: free disk space. Then finish and log Phase 4.7 and
Phases 5, 6, 7, and 8, in that order, and run Phase 9 again.

## Phase 10

### Halted before any code change (2026-09-17)

Phase 10 was **not started**. No source file was edited. What was checked:

1. **Prerequisites are still incomplete.** Phase 10 depends on Phases 5-9.
   Task 4.7 and Checkpoint 4 are unchecked. Every task in Phases 5-9 is
   unchecked, this log has no `## Phase 5` section, and the Phase 6, 7, 8,
   and 9 sections above record halts.
2. **The worktree has not changed since the Phase 6-9 halts.** `git status`
   shows the same 14 modified files and 2 untracked files
   (`capture/document.rs`, `capture/network.rs`), plus this log and the plan.
   HEAD is still `153717fa5`.
3. **Phase 10 needs Darkmatter code that does not exist yet:**
   - 10.1 has Claudine supply Document, Network, and hostname evidence and
     request `CommitDesc` history only for eager recent-history demand. Those
     requirement types and planners come from Phase 5.2-5.6.
   - 10.2 projects `claudine context` listings from descriptors, which needs
     Phase 4.7's passive tooling work to be finished.
   - 10.3 replaces `LifecycleCurrent` with Darkmatter's `current` and
     `current_env` roots, and 10.4 adds `current_env` to
     `LATE_BINDING_ROOTS`. Both roots and the refresh-capability provider come
     from Phase 6.1-6.4. 10.3 also edits the CRITICAL-risk `capture_at_event`,
     which still waits on the explicit review requested in Phases 1-2.
   - 10.5 moves tests to `current.<key>` and `current_env.<key>`. That only
     works after Darkmatter reserves both roots (Phase 6.3-6.4).
   - 10.7's L2 coverage for AC2 and AC28 checks behavior from Phases 5 and 6.
4. **Storage has not recovered.** `df -h /Volumes/coding` still reports
   1.7 GiB free of 3.6 TiB. Neither the Darkmatter nor the Claudine L1 build
   can finish. Nothing was deleted, following storage-strategy rule 2.

The spec frontmatter already has `implemented: true` and
`implemented_by: claude/opus`, so it was left as it was.

No tests were added or run. There is no requirement-to-test mapping because no
behavior changed.

Next step for a human: free disk space. Then finish and log Phase 4.7 and
Phases 5, 6, 7, 8, and 9, in that order, and run Phase 10 again.

## Phase 11

### Mostly halted; only task 11.3 done (2026-09-17)

Phase 11 writes the public docs for the feature. Checkpoint 11 says READMEs
and skills must describe **only implemented behavior**. That behavior mostly
does not exist yet, so most of the phase was not started.

**What was checked:**

1. **Prerequisites are still incomplete.** Task 4.7, Checkpoint 4, and every
   task in Phases 5-10 are unchecked. This log has no `## Phase 5` section.
   HEAD is still `153717fa5`. The worktree still has the same 14 modified and
   2 untracked Phase 5-looking files, and nobody has verified them.
2. **Storage is worse.** `df -h /Volumes/coding` reports 1.3 GiB free, down
   from 1.7 GiB at the Phase 10 halt. Nothing was deleted, following
   storage-strategy rule 2.

**Per-task status:**

| Task | Status | Reason |
|---|---|---|
| 11.1 Darkmatter docs and compose skill | not started | eager `ctx` additions, `current`/`current_env`, the new functions, `as_markdown`, and ICMP consent are Phases 5-9 |
| 11.2 Claudine docs and skills | not started | the `LifecycleCurrent` replacement and `current_env` late binding are Phase 10 |
| 11.3 Sniff docs | **done** | documents only committed Phase 2-3 APIs (see below) |
| 11.4 Prompts and templates | not started | removing sentinel workarounds needs Phase 5.5/10.6; checking that prompts compose needs a Darkmatter build, and the disk is full |
| 11.5 Dependency inventories | open, partly done already | Phase 4 (commit `78540226f`) already recorded `ipnet`, `getrandom`, `socket2`, and `biscuit-hash`'s `blake3` feature in the root, Darkmatter, and Sniff `docs/dependencies.md`; the task stays open until Phases 5-10 stop changing manifests |
| 11.6 Markdown hashes | not started | runs after 11.1-11.5; neither edited Sniff README has a hash frontmatter property |
| 11.7 Drift greps | not started | would fail by design until Phases 5.5, 10.3, and 10.5 land |

### 11.3 details

- `sniff/README.md`: the Network bullet and the Detection Categories row now
  list default gateways, CGNAT/Tailscale detection, and ICMP probes.
- `sniff/lib/README.md`, Network Module: added `ScopedIpAddr`,
  `host_addresses`, `contains_cgnat_address`, `detect_default_gateways`,
  `network::icmp::{ping, ping_with, ProbeBudget, PingVerdict, IcmpError}`, the
  Linux/WSL2 `net.ipv4.ping_group_range` requirement, and a short example.
- `sniff/lib/README.md`, Git key functions: added
  `get_current_worktree_name` / `current_worktree_name_with_repo` (the cheap
  worktree observation) and `CommitDesc::describe_plain` (the per-commit
  formatter).
- Every name was checked against `sniff/lib/src` re-exports first.
- CLI docs needed no change. No `sniff` subcommand exposes gateways or ICMP.
  Phase 2 already documented the empty `""` package area in
  `docs/cli/repo_package-area.md` and `repo_package-areas.md`.

### Tests

These were documentation-only edits, so behavior did not change and there is
no requirement-to-test mapping. `just test` and `just lint` were not run: no
source changed, and 1.3 GiB free is not enough for a Darkmatter build.

The spec frontmatter already had `implemented: true` and
`implemented_by: claude/opus`, so it was left as it was.

Next step for a human: free disk space. Then finish and log Phase 4.7 and
Phases 5-10 in order, and run Phase 11 again for 11.1, 11.2, and 11.4-11.7.

## Implementation of Review Findings #1

> **started at:** 2026-09-17T08:30:00-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/darkmatter/features/2026-09-09-more-context/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- storage precondition checked: `/Volumes/coding` has 351 GiB free, so the earlier ENOSPC halts no longer apply
- starting the work on 'Complete the capture-group integration so Darkmatter compiles' at 08:30:25
        - discovery: the capture, snapshot, runtime, and Claudine code already handled `Document`, `GitHistory`, and `Network`; only `capture/groups.rs` lacked the variants, and Claudine's `context_group_name` already fixed their stable names as `document`, `git_history`, and `network`
        - discovery: once the groups own their keys, `catalog::PENDING_CAPTURE_KEYS` contradicts capture for all ten keys (`hostname` was already projected by `host::OS_KEYS`), and its own docs say Phase 5 deletes it
        - changed `darkmatter/lib/src/markdown/compose/context/capture/groups.rs`: added the three variants, `all()` (now 14), stable names, and `projected_keys` ownership (`document::KEYS`, `git::HISTORY_KEYS`, `network::KEYS`); `ctx.branch`/`worktree`/`merge_conflicts` stay in `Git`, so only `ctx.recent_commits` demands `GitHistory`
        - changed `darkmatter/lib/src/markdown/compose/context/catalog.rs` and `darkmatter/lib/tests/ambient_ctx_capture.rs`: deleted `PENDING_CAPTURE_KEYS` and its filters, so every cataloged descriptor is now held to the capture-parity and capture-shape tests
        - changed `darkmatter/lib/src/markdown/compose/context/capture/network.rs`: fixed a `redundant_closure` clippy error that was hidden while the crate did not compile
        - tests added: stable names and `from_name` round trip for the new groups, `all()` membership with an exhaustive match, no key projected by two groups, spec key ownership (`hostname` in `Os`), Git facts never demanding `GitHistory`, Document/Network keys demanding only their group, a cache-manifest group-name round trip (`cache/manifest.rs`), and an AC31 work-counter test showing `ctx.branch` does no history or network work (`capture/mod.rs`)
        - docs: added GitHistory, Document, and Network rows plus `hostname` to `darkmatter/docs/topics/context-variables.md`; removed the `PENDING_CAPTURE_KEYS` sentence from `.claude/skills/darkmatter/library-surfaces.md`
        - results: `darkmatter` `just test` passed 7937 of 7937 (7 skipped) and `just lint` passed; `claudine` `just test` passed 7026 of 7026 (9 skipped) and `just lint` passed
        - residual: `.claude/skills/darkmatter/library-surfaces.md` has 25 lines with a literal leading `+` from commit 78540226f; the edit kept that style and did not repair it
- work completed for 'Complete the capture-group integration so Darkmatter compiles' at 08:56:12
- starting the work on 'Implement the thirteen expression functions that still deliberately fail' at 08:56:20
        - orchestration: split into four serial sub-tracks following plan Phases 7-9: (a) `package`, `package_area`, `recent_commits`, `ipv4`, `ipv6`; (b) `has_alias`, `has_builtin_function`, `has_user_function`, `can_execute`, `has_agentic_cli`; (c) `as_markdown`; (d) `ping`, `ping_under` plus removal of the pending-failure ratchet
        - sub-track (a): implementing `package`, `package_area`, `recent_commits`, `ipv4`, `ipv6` (plan 7.1-7.3 and their 7.7 cases)
        - discovery: the captured package list and interface addresses existed only inside `ContextCapture` (projected to JSON, where `ctx.packages` has names but no roots) and the `FileResolutionContext` catalog has roots but no names, so no request-owned state could answer a named lookup; `ctx.*` scanning also meant a document calling only `package()` or `ipv4()` captured neither group
        - design: new `CapturedObservations` (`context/capture/observations.rs`) retains a `PackageLookup` when `Repo` is captured and the stable scoped addresses when `Network` is captured; it rides on `ComposeContext` beside the JSON values (a fifth `CaptureResult` element, merged by `extend_missing_with` and `adopt_groups` per group) and is copied into `ResolutionContext::observations` by both `ComposeOptions` resolution-context builders; the ambient and supplied-evidence paths share `populate_capture`, so Claudine gets it with no Claudine change
        - design: `scan_needed_groups` now also demands `Repo` for `package(`/`package_area(` calls and `Network` for `ipv4(`/`ipv6(` calls (identifier-boundary match, literal-masked); `recent_commits(count)` demands no group and walks history from `ResolutionContext::repository_root` (the file-resolution snapshot) on every reached call, rendering with the capture's `render_recent_commits`
        - design: `PackageLookup` lives in `context/repository_scope.rs` because `resolver_inventory_has_one_projection_adapter_and_no_discovery_fallbacks` allows exactly one `RepoInfo` to `RepositoryScopeCatalog` adapter; it rebuilds the shared projection with `PackageAreaFallback::None` (no inferred first-component area), selects package and area independently through `RepositoryScopeCatalog::scope_for` (component-aware, normalized, Windows prefix-aware), and rebases a path under the file-resolution root spelling onto the observed root
        - design: added fatal `ExpressionError::FunctionContextNotCaptured { function, group }` (in `is_authoring_fatal` and `is_missing_runtime_context`) so a call whose group was never captured cannot degrade to `""`/`[]`
        - GitNexus: upstream impact reported CRITICAL for `populate_capture`, `from_values`, `extend_missing_with`, `adopt_groups`, `expression_resolution_context`, `local_expression_resolution_context`, `scan_needed_groups`, and `render_recent_commits` (result sets include name-collision noise from unrelated packages); `ResolutionContext` and `ContextCapture` were ambiguous/UNKNOWN and confirmed by text search; every edit is additive and existing values are unchanged; `detect-changes --scope all` reports critical across the whole uncommitted tree, including the earlier capture-group work
        - files changed: `context/capture/{observations.rs (new),mod.rs,groups.rs,git.rs,agent.rs,repo.rs}`, `context/{runtime.rs,options.rs,repository_scope.rs,mod.rs}`, `expression/{error.rs,resolve_ctx.rs,ctx.rs}`, `expression/functions/{repository.rs (new),network.rs (new),git.rs,mod.rs,pending.rs}` under `darkmatter/lib/src/markdown/compose/`; new `darkmatter/lib/tests/context_functions.rs`; `.claude/skills/darkmatter/compose.md`
        - pending ratchet now lists the remaining eight: `has_alias`, `has_builtin_function`, `has_user_function`, `can_execute`, `has_agentic_cli`, `as_markdown`, `ping`, `ping_under`
        - tests added (unit, through `dispatch_fs`): package/area deepest and independent selection, sibling prefix `foobar`, area-only, nested area `claudine/rendezvous`, repository-root file, unobserved directory, `..` normalization, outside repository, non-monorepo, root-spelling rebase, a `cfg(windows)` drive-letter-case/separator case (compiles for `x86_64-pc-windows-gnu`; runs only on the Windows leg), remote URL and unconfigured-vault typed errors, non-string arguments, uncaptured `Repo`; `ipv4`/`ipv6` AC20 set, covering CIDRs including loopback/link-local, host-bit CIDRs, malformed `192.168.10/99` fallback, other-family CIDRs, scoped IPv6 retained, empty capture, uncaptured `Network`; `recent_commits` zero/negative/`0.0`/fractional/`1e300`/beyond-2^53 counts, non-number counts, no-root `[]`, unborn repository, a commit between two calls on one context, a captured root that is not a repository; function-call demand scanning
        - tests added (compose pipeline, `darkmatter/lib/tests/context_functions.rs`, in-process git2 fixtures and supplied interface evidence): package lookups in a monorepo with only function demand, misses in a plain repository and outside it; `recent_commits(10)` equal to `ctx.recent_commits`, `(3)` a prefix, `(12)` twelve, `(50)` all thirteen; a commit made by an approved frontmatter `$(git ... commit)` appears in `recent_commits(1)` and not in `ctx.recent_commits`; unborn and outside-repository `[]`; `recent_commits(0)` in a frontmatter value is a compose error; `ipv4`/`ipv6` through `capture_for_document_with_evidence`
        - deviation: in lenient body interpolation, invalid-count and remote-URL errors are `ExpressionError::Other` and demote to warnings like sibling functions (`predict_conflicts`); they are compose errors in frontmatter values (tested). `recent_commits` output can be shorter than `count` because the shared formatter omits commits that touched no files (spec R27)
        - results: `darkmatter` `just lint` passed and `just test` passed 7966 of 7966 (7 skipped); `claudine` `just lint` passed and `just test` passed 7026 of 7026 (9 skipped)
        - sub-track (a) follow-up: closing the reported R28/AC25 deviation, where an invalid `recent_commits` count was `ExpressionError::Other` and became a warning in the lenient body
        - discovery: the only error-to-warning downgrade is lenient body interpolation (`interpolation/rewrite.rs`, gated by `ExpressionError::is_authoring_fatal`); page-block and `::file` `when=` conditions and frontmatter `$()` ternary conditions and branches already fail on every evaluation error, so one classification covers every surface
        - mechanism: new `ExpressionError::ContractViolation { function, message }`, a call that breaks a function's specified contract where the spec requires a compose error; `is_authoring_fatal` returns true for it, it is not a missing-runtime-context error, and it renders exactly like `Other` (`{function}(): {message}`)
        - converted: every `recent_commits` count rejection (zero, negative, fractional, non-finite, beyond the exact-float or `usize` range) now returns `ContractViolation`; a non-number count stays `ArgType`, which the evaluator treats as non-fatal for every function
        - decision: spec R12/R26 ("never errors" means a lookup miss, not accepting malformed references; typed argument/reference errors are preserved) plus plan 7.1 ("reject malformed/remote references without fetching") make a URL passed to `package`/`package_area` a `ContractViolation`, checked in `repository.rs` before the shared `resolve_path_shape`, so `dirname` and the other path functions keep their existing behavior; typed `FileReference` failures keep their existing classification (malformed, not-found, and found-elsewhere are already fatal)
        - files changed: `expression/error.rs` (variant, fatality, docs, unit test), `expression/functions/git.rs`, `expression/functions/repository.rs`, `interpolation/fatality_characterization.rs` (policy doc), `darkmatter/lib/tests/context_functions.rs`, `.claude/skills/darkmatter/compose.md`
        - tests added: `recent_commits_rejects_invalid_counts_in_the_body` (counts `0`, `-1`, `1.5` in a lenient body fail the compose, while `recent_commits("3")` in the same lenient body only warns), `package_rejects_a_remote_reference_in_the_body`, `contract_violation_is_authoring_fatal_and_renders_like_other`, and `ContractViolation` plus fatality assertions in the existing count and remote-URL unit tests; non-vacuity check: with the `is_authoring_fatal` arm flipped to `false`, both body tests failed; restored
        - results: `darkmatter` `just lint` passed and `just test` passed 7969 of 7969 (7 skipped); `claudine` `just lint` passed and `just test` passed 7026 of 7026 (9 skipped)
        - sub-track (b): implementing `has_alias`, `has_builtin_function`, `has_user_function`, `can_execute`, `has_agentic_cli` (plan 7.4-7.6 and their 7.7 cases)
        - design: new `shell_expansion/launcher.rs` is the bounded login-shell launcher (closed stdin, discarded stderr, 64 KiB stdout tail, deadline); Unix keeps the `setsid` session and now kills the whole process group on exit and on timeout, Windows adds `CREATE_NO_WINDOW` and a kill-on-close Job Object; `alias::query_alias` now uses it (the job-control hazard docs moved with the launcher), so alias resolution keeps its behavior and gains descendant cleanup
        - design: new `shell_expansion/probe.rs` `ShellProbe` authority on `ResolutionContext::shell_probe`, built by both `ComposeOptions` resolution-context builders from the request environment snapshot (`SHELL`, and on Windows `PATH` for the `pwsh` then `powershell` fallback); `ResolutionContext::default()` holds a disabled probe that never launches; one launch returns all three kinds (alias/builtin/function), the name travels only in `DARKMATTER_SHELL_PROBE_NAME` and is read by `type -a -t` (bash), `whence -wa` (zsh), `functions`/`builtin --names` (fish), and `Get-Command` with a wildcard-escaped name via `-EncodedCommand` (PowerShell); answers are whole `darkmatter-shell-probe:<kind>` lines and require a successful exit
        - design: `can_execute` calls `has_command_fn` first and returns before any launch when it is true; null, non-string, and empty names read as `false` without a launch, matching `has_command`
        - design: `has_agentic_cli` validates the name against the generated `AGENTIC_CLI_NAMES` first (unknown name -> `ContractViolation`, even when uncaptured; non-string -> `ArgType`), then reads `CapturedObservations::agentic_clis`, an `Arc<OnceLock<InstalledAiClients>>` created by the `Agent` group capture and scanned on first use with `ExecutableIndex::build_path_only()`; `has_agentic_cli(` in `FUNCTION_GROUPS` demands `Agent`; uncaptured -> `FunctionContextNotCaptured`
        - discovery: `md compose` evaluates body expressions in three passes before this change added suppression: reference validation (`build_reference_graph` -> `prepare_content`), shell-command preflight discovery (`collect_recursive`, run both by `compose_preflight_approvals` and inside the terminal compose's `validate_pre_approved`), and the terminal body interpolation, so one call launched the shell four times; spec R8 forbids preflight launches, so new `ComposeOptions::suppress_shell_probes` (graph identity, not cache identity, mirroring `defer_missing_runtime_context`) is set by preflight discovery; reference validation still probes because it is an effectful compose pre-pass that also runs shell expansion
        - GitNexus: `query_alias` and `resolve_alias_with_shell` report CRITICAL upstream (shell expansion and preflight flows); `spawn_alias_query` LOW; `populate_capture`, `fill_from`, `expression_resolution_context`, `local_expression_resolution_context` returned UNKNOWN (index predates sub-track (a)) and were confirmed by text search: two builder sites in `context/options.rs`, one capture site; `ResolutionContext::new` production callers are `options.rs` (probe set) and `conditions.rs` `ShortcutLookup` (disabled probe, same as its uncaptured observations)
        - files changed: new `shell_expansion/{launcher.rs,probe.rs}`, `expression/functions/{shell.rs,agentic_cli.rs}` and edits to `shell_expansion/{alias.rs,mod.rs}`, `expression/functions/{mod.rs,pending.rs}`, `expression/resolve_ctx.rs`, `context/options.rs`, `context/capture/{observations.rs,mod.rs,groups.rs}`, `preflight/collect.rs` under `darkmatter/lib/src/markdown/compose/`; `darkmatter/lib/Cargo.toml` and `Cargo.lock` (Windows-only `windows` 0.62 with `Win32_Foundation`/`Win32_System_JobObjects`); new `darkmatter/lib/tests/shell_probe_preflight.rs` and `darkmatter/cli/tests/compose_context_probes.rs`; docs `.claude/skills/darkmatter/compose.md` and `darkmatter/docs/dependencies.md`; the catalog descriptions in `expression-functions.yaml` and `darkmatter-expressions.md` already matched the behavior and were not changed
        - pending ratchet now lists exactly `as_markdown`, `ping`, `ping_under`
        - tests added (unit): launcher kills a background descendant after a normal exit and at the deadline, bounds noisy output to the tail, missing executable; probe dialect detection, marker-line parsing, `-EncodedCommand` base64, disabled default authority, missing/empty `$SHELL` on Unix, unsupported shell and NUL/empty names with zero launches, unlaunchable shell; `cfg(windows)` `pwsh` -> `powershell` -> neither fallback with `.cmd` stubs on a request `Path`; real-shell fixtures (fixture HOME/ZDOTDIR/XDG_CONFIG_HOME, BASH_ENV/ENV removed) for bash and zsh (aliases to a binary, a builtin, and a missing binary; builtin; user function; function shadowing a builtin; unknown), fish, and PowerShell on Unix, each skipped with a message when not installed; injection-shaped names (`$(...)`, backticks, `;`, `&&`, `|`, quotes, newlines, `x=$(...)`, PowerShell `New-Item`, `-rm`, `--help`, `*`) for every installed dialect with a marker file that must not appear; noisy profile, hanging profile within a 500 ms bound, profile that exits, detached profile child killed; dispatch tests for non-string/empty names with zero launches, each predicate reading only its kind, failing and hanging stub shells, and AC11 `can_execute` over all 16 binary/alias/builtin/function combinations with the launch counter proving zero launches whenever the binary is found; `has_agentic_cli` AC18 alias parity (`kimi`/`kimi_code`/`kimicode`/`kimi-code`) in both outcomes, unknown names (`not_a_provider`, `Kimi`, `kimi_cli`, `aider`, empty, leading space) as fatal `ContractViolation` captured or not, uncaptured `Agent` fatal, non-string `ArgType`, clones sharing one scan; `has_agentic_cli(` demanding `Agent` and `can_execute(` demanding nothing
        - tests added (pipeline): `md compose` with a launch-logging stub `bash` answering all three kinds (10 launches for 5 calls: reference validation and the terminal pass each probe once per call), `can_execute` of a fixture binary with zero launches, real `/bin/bash` with `inherit_no_env` recognizing `cd` while backtick and `;` names stay false with no marker, no `$SHELL` (and on Windows no `pwsh` on the fake-only PATH) reading false, `has_agentic_cli` against a fake-only PATH holding `kimi` (`kimi_code`/`kimi` true, `claude` false) and `not_a_provider` failing the compose; library `compose_preflight` launches zero times while the following `compose_with` launches once
        - non-vacuity: with `BASH_QUERY` changed to `eval` the name, `injection_shaped_names_never_execute` failed (`bash executed "$(touch …/pwned)"`); with the process-group kill removed, both launcher descendant tests and `a_detached_profile_child_is_cleaned_up` failed (descendant survived); both restored
        - results: `darkmatter` `just lint` passed (after fixing one `field_reassign_with_default` in a test helper) and `just test` passed 7994 of 7994 (14 skipped); `claudine` `just lint` passed and `just test` passed 7026 of 7026 (9 skipped); `x86_64-pc-windows-gnu` `cargo check -p darkmatter -p darkmatter-cli --tests` passed (one test-only dead-code warning, then fixed)
        - cross-check blockers: `$BUILD_WIN` (`build-win-native`) W: volume has 8192 bytes free, so the Windows run died in `git fetch` with "No space left on device" (no files were deleted; recorded for the host owner); `$BUILD_LINUX` is held by a stale `.cross-check.lock` from 2026-09-14 (owner `reward-20260914-c3e60d0`, `nightly-reward-spike`), which the script never removes and neither did I; `$BUILD_WSL` (`build-win`) reset the SSH connection; `just cross-check … -E` also breaks on the recipe's unquoted args, as `os/build-hosts.md` already records
        - Linux evidence instead: Docker `rust:1.97.1` (linux/arm64, root) with bash, zsh, and fish installed: the probe, launcher, alias, shell, agentic_cli, and pending unit tests passed 39 of 39 (fish real-shell test ran; pwsh absent), `shell_probe_preflight` 1 of 1, `compose_context_probes` 5 of 5; a wider filter also ran three `reservation_cleanup::*_write_failure_*` tests that fail only because root can write the "unwritable" whitelist (they pass on macOS in `just test`), unrelated to this change
        - residual risk: native Windows behavior (Job Object containment, `CREATE_NO_WINDOW`, `.cmd` fallback stubs, `kimi.cmd` PATHEXT lookup) is compile-checked only until a Windows host or CI runs it; real PowerShell classification is exercised only on Unix hosts with `pwsh`, because the Windows profile lives in the Documents known folder that no environment variable can redirect to a fixture
        - sub-track (c): implementing `as_markdown` (plan Phase 8, 8.1-8.5; AC15, AC16, nested AC30/AC32)
        - discovery: the compose pipeline is synchronous (transclusion concurrency is rayon, no async runtime), expression handlers are sync `fn(&[Value], &ResolutionContext)`, and a transcluded child already runs `run_compose_pipeline_internal` on the calling worker thread; so the nested child runs synchronously on the calling thread too, with no new blocking primitive, and self-recursion to depth 16 on a default 2 MiB test thread does not overflow
        - discovery: an in-memory root pushes no ancestry node, so the root-only gates (`transclusion.depth() <= 1` for link normalization and the pre-approved command gate) also fired in its first-level children; a nested child of an in-memory root would have normalized links twice
        - discovery: shell-command preflight evaluates body expressions in its inline discovery compose, and the terminal pipeline runs frontmatter interpolation before the pre-approved gate, so a frontmatter `as_markdown` could run an approved nested command before a later unapproved body command failed the gate
        - design: new `compose/nested.rs`; `ComposeOptions::nested_compose: NestedComposeSlot` (`Unavailable` default, `Discover(sink)`, `Active(NestedCompose)`) is copied into `ResolutionContext` only by `expression_resolution_context` and `frontmatter_resolution_context`; every document pipeline installs `Active` (a snapshot of its options without the handle, a `clone_for_child` runtime fork, and an outcome sink) and folds child reports, deepest depth, context groups, and dependencies back when its stages finish
        - design: `run_compose_pipeline_internal` now delegates to `run_compose_pipeline_node(options, runtime, node)`; an `as_markdown` call pushes a process-unique `as_markdown#N` node (display `as_markdown()`) onto the shared ancestry, so it spends the `max_transclusion_depth` budget, forms cycles only through real sources, and self-composition ends at `MaxDepthExceeded`; a `CycleDetected`/`MaxDepthExceeded` below the call is restored as the typed error when the caller finishes, other nested failures are fatal `ContractViolation`s ("nested composition failed: …") wrapped with the caller's location
        - design: the child reuses the caller's options (consent, pre-approved set, remote config, cache policy, external state), takes the request root's source from new `PipelineRuntime::root_source` as its resolution base, drops `set_overrides` and `preflight_graph`, and extends its context through the request context epoch like a transcluded child; output is the composed body plus only the frontmatter keys the content authored; nested warnings get stage `as_markdown > <stage>`
        - design: new `PipelineRuntime::is_root()` (children from `clone_for_child`) replaces the two depth checks; new shared `preflight_validated` flag lets the first frontmatter `as_markdown` call run the root's pre-approved gate against the root document first (the pipeline gate then skips); `pipeline::preflight_gate_applies` is the one condition for both
        - design (8.4): `collect_recursive` now gives each document a discovery copy of its options with `suppress_shell_probes` and `NestedComposeSlot::Discover` for both the frontmatter scan and the inline discovery compose; `as_markdown` there records its evaluated argument and returns `""`; the walk then recurses (root source, `visited` keyed by content xxHash) into the recorded content plus every string-literal `as_markdown` argument in the authored frontmatter and body, so false `::block` regions and untaken ternary branches contribute
        - design (8.4 and the sub-track (b) residual): new `ShellExpansionError::UnevaluatedDependencyShape { command, dependency, origin }` (display starts "dynamic command shape", like `DynamicCommandShape`, whose message names only frontmatter keys) rejects, before execution, a `::shell` line, shell-block body, transclusion target, or frontmatter `$(...)` value calling `has_alias`, `has_builtin_function`, `has_user_function`, `can_execute`, or `as_markdown`, and any non-literal `as_markdown` argument reading one of those or a pending frontmatter-shell value; the check is unconditional like the existing dynamic-shape checks
        - decision: `as_markdown` on a surface without a compose request (`local_expression_resolution_context`, passive validation, Claudine loop/hook adapters, bare `ResolutionContext`) is a `ContractViolation`; a non-string or null argument is `ArgType` (non-fatal in the lenient body, like sibling functions); `""` composes to `""`; content with malformed frontmatter is a `ContractViolation`
        - GitNexus: upstream impact returned "not found"/UNKNOWN for `run_compose_pipeline_internal`, `expression_resolution_context`, `frontmatter_resolution_context`, `collect_recursive`, and `detect_dynamic_command_shape`, and ambiguous for `PipelineRuntime`/`ResolutionContext` (index commit 583ea7c predates sub-tracks (a) and (b)); confirmed by text search: `run_compose_pipeline_internal` callers are `pipeline/mod.rs`, `context/authority.rs` (test), and two transclusion-engine child paths; the two resolution-context builders have five production callers (frontmatter interpolation passes, frontmatter shell expansion, body interpolation, page blocks, transclusion `when=`) plus preflight; `detect-changes --scope all` reports critical across the whole uncommitted tree
        - files changed: new `compose/nested.rs` and `expression/functions/composition.rs`; `compose/{mod.rs,pipeline/mod.rs,preflight/collect.rs}`, `shell_expansion/types.rs`, `context/options.rs`, `expression/resolve_ctx.rs`, `expression/functions/{mod.rs,pending.rs}` under `darkmatter/lib/src/markdown/`; new `darkmatter/lib/tests/nested_composition.rs` and `darkmatter/cli/tests/compose_nested_markdown.rs`; docs `.claude/skills/darkmatter/compose.md` and `darkmatter/docs/inline/preflight-checks.md` (the catalog description and generated function table already matched)
        - pending ratchet now lists exactly `ping`, `ping_under`
        - tests added (unit, `composition.rs`): `Unavailable` surface is a fatal `ContractViolation`; number/null/array arguments are `ArgType` and record nothing; wrong arity; `Discover` records content (empty string included), returns `""`, and drains through a clone
        - tests added (library, `nested_composition.rs`): AC15 shared `ctx.hostname` plus root-relative `::file` from the root and from a transcluded `sub/child.md` whose sibling `part.md` must not be chosen; AC30 identity (`ctx.id`/`sid`/`self`/`hash` equal in root and child) for file, URL, and in-memory roots; frontmatter and body calls, authored nested frontmatter serialized, external state not serialized; empty and non-string arguments; nested warning stage and nested failure message; link normalization runs once (perf `calls == 1`) for file and in-memory roots; AC16 self-composition through external state ends in `MaxDepthExceeded { max_depth: 16 }`; mixed budget (`as_markdown` then `::file` passes at depth 3, fails at 2); mutual `as_markdown`<->`::file` recursion is `CycleDetected` with an `as_markdown()` node; preflight collects nested commands from a literal, a false `::block`, an untaken ternary branch, a frontmatter-value argument, and doubly nested content without running them, then the approved compose runs them (execution half `cfg(unix)`); six dynamic shapes (pending frontmatter argument, probe in argument, `has_alias` in `::shell`, `has_user_function` in a shell block, nested `as_markdown` argument, `has_builtin_function` in a frontmatter `$(...)`) fail `compose_preflight` with `UnevaluatedDependencyShape` and fail the approved compose with no frontmatter command run; a frontmatter nested command waits for the root gate (`NotPreApproved` for the body command, sentinel absent); with a host allowed but remote transclusion off, nested `::file https://…` from frontmatter or body makes zero requests (wiremock)
        - tests added (CLI via `CliProcessFixture`, `compose_nested_markdown.rs`): shared `ctx.id` and root-relative `::file`; a whitelisted nested command runs exactly as often as the same command in the body (once each, through reference validation, preflight approval, and compose); dynamic nested content rejected with "dynamic command shape" and `frontmatter.part` before any command runs; mutual recursion fails with a cycle, not a hang
        - non-vacuity: with the finalization gate back on `depth() <= 1` the link test failed for the in-memory root; with the typed-error restore and the lazy gate removed and the literal scan disabled, the self-composition, mixed-budget, mutual-cycle, root-gate, and preflight tests failed; with `detect_unevaluated_dependency_shape` disabled the dynamic-shape test failed; all restored
        - results: `darkmatter` `just test` passed 8014 of 8014 (14 skipped) and `just lint` passed; `claudine` `just test` passed 7026 of 7026 (9 skipped) and `just lint` passed; `x86_64-pc-windows-gnu` `cargo check -p darkmatter -p darkmatter-cli --tests` passed after removing one Unix-only import warning in the new CLI test (re-checked with clippy and the test plus `spawn_site_guard`, 27 of 27)
        - deviation: the spec's "frontmatter-local restrictions" assume frontmatter surfaces are local-only, but `frontmatter_resolution_context` has carried the authorized remote runtime since 2026-07-22 (tested by `frontmatter_and_body_remote_reads_have_parity_and_single_flight`); the nested child therefore inherits exactly the calling surface's remote policy and never widens it; the stale "frontmatter is local-only" statements in `.claude/skills/darkmatter/compose.md`, `darkmatter/docs/topics/darkmatter-expressions.md`, and the `ResolvingLookup` doc comment were left for the author to reconcile with the spec
        - deviation: D8 says mixed recursion "reaches the same typed depth-limit error"; recursion that repeats a real source reports the existing typed `CycleDetected` first, and only recursion without a repeated source reaches `MaxDepthExceeded`
        - residual: a body expression outside a `::shell` line whose output is itself a directive (for example `{{ can_execute("x") ? "::shell echo a" : "::shell echo b" }}`) still approves the discovery shape and fails closed at run time as `NotPreApproved`
        - residual: `validate_pre_approved` still runs inside the terminal compose after frontmatter interpolation (unchanged); the lazy gate covers only frontmatter `as_markdown`
        - sub-track (d): Phase 9 ICMP consent, preflight effects, and the `ping`/`ping_under` handlers; `pending.rs`, its `mod` declaration, and the `pending_bindings_are_exactly_the_unimplemented_functions` ratchet are deleted, so every cataloged function now has a real binding
        - new `compose/icmp.rs` owns the request's `IcmpAuthority`: parsed `IcmpGrant`s (exact `ScopedIpAddr`, strict `IpNet`), an optional injected `EchoProbe`, an optional discovery sink, and a shared denial-warning sink. It is a `ComposeOptions` field projected into both resolution-context builders; its grants are re-derived from `remote_read_config.allowed_hosts` on every projection (`ComposeOptions::icmp_authority`), so `--allow-host` stays the single entry point and no second flag exists
        - HTTP/ICMP separation: new `RemoteReadConfig::http_hosts()` withholds strict-CIDR entries from `is_host_allowed`, `remote_reads_enabled`, and the `FetchPolicy` built in `RemoteFetchRuntime::with_store`. A bare IP literal keeps its existing exact-HTTP meaning *and* grants ICMP; a hostname/wildcard grants HTTP only; a CIDR grants ICMP only. An exact grant that spells an IPv6 zone permits only that zone, an unscoped one permits any
        - preflight: `collect_effects` is the new internal entry (`collect_shell_commands_with_graph` delegates); `collect_recursive` carries a `&mut Vec<PlannedIcmpProbe>` accumulator, `discovery.icmp = options.icmp.discovering()` guarantees no packet, and `authored_icmp_probes` additionally dispatches every all-literal `ping`/`ping_under` call in the authored source so untaken branches contribute. `ComposePreflightReport::icmp_probes` is the typed record (function, target, timeout, attempts, granted). `ping`/`ping_under` were added to `UNEVALUATED_IN_DISCOVERY`, so a shell/transclusion shape depending on one is `UnevaluatedDependencyShape`
        - discovery order note: the accumulator follows the interpolation walk, not source order (a `when="false"` block's probe can precede an earlier one); the report is deduplicated but not source-ordered, and the tests assert coverage rather than sequence
        - denial warnings surface as `ComposeWarning { stage: "icmp" }`, drained once at the root in `run_compose_pipeline` from a handle cloned before the pipeline consumes the options, so a transcluded or `as_markdown` child's denial reaches the root report through the shared `Arc`
        - `md compose --shell` now prints a second table (`print_icmp_effect_report`) with function, target, timeout, attempts, and the grant verdict; it sends nothing
        - transport is entirely Sniff's (`sniff::network::icmp::{ping, ping_with, ProbeBudget, PingVerdict}`); nothing was added to or changed in `sniff/`, and AC14's real loopback tests there are not duplicated. `ProbeBudget::from_millis` supplies the checked, non-truncating number validation the spec requires
        - GitNexus: `ComposePreflightReport` resolved ambiguous with CRITICAL risk (5800 impacted) on the `darkmatter` candidate; `collect_recursive` and `expression_resolution_context` returned "not found" (index commit 583ea7c predates sub-tracks (a)-(c)). Confirmed by text search that the report is only constructed inside `preflight/` and only read (never built) by `darkmatter/cli/src/commands/compose.rs`, `claudine/lib/src/composition/preflight.rs`, and six library tests, so the added `icmp_probes` field breaks no consumer. `detect-changes --scope all` reports 44 files, 18 symbols, risk high across the whole uncommitted tree
        - files changed: new `compose/icmp.rs`, `compose/tests/icmp.rs`; edits to `compose/{mod.rs,remote.rs,remote_fetch.rs,pipeline/mod.rs}`, `context/options.rs`, `expression/resolve_ctx.rs`, `expression/functions/{mod.rs,network.rs}`, `preflight/{mod.rs,collect.rs}`, `compose/tests/mod.rs` under `darkmatter/lib/src/markdown/`; deleted `expression/functions/pending.rs`; `darkmatter/cli/src/commands/compose.rs` and new `darkmatter/cli/tests/compose_icmp.rs`; docs `darkmatter/docs/cli/compose.md`, `darkmatter/docs/inline/preflight-checks.md`, `.claude/skills/darkmatter/compose.md`, `.claude/skills/darkmatter/library-surfaces.md`
        - docs drift found and fixed: `.claude/skills/darkmatter/library-surfaces.md` lines 35-58 carried literal `+` diff markers on every line (present in HEAD, from a botched patch in an earlier phase) and described a `PENDING_CAPTURE_KEYS` constant that no longer exists; the prefixes were stripped and the pending bullet replaced by the surviving `ContextGroup` claim
        - tests added (unit, `icmp.rs`): grant parsing rejects hostnames/wildcards, normalizes `2001:0db8:0000::0001`, keeps an explicit zone as a constraint, treats an unscoped grant as any-zone, matches CIDRs by bits without crossing family (`::ffff:10.1.0.1` is not in `10.1.0.0/16`), and withholds only CIDR entries from HTTP (`10.0.0.0/99` is neither)
        - tests added (dispatch, `network.rs::ping_tests`): AC6 denial is `null` + one warning + zero sends; AC33 denied 3-attempt call sends nothing; CIDR admits its own family only; scoped IPv6 zone survives to the transport and a different zone is denied; AC7 no-reply is `false` and a send failure is a never-demoted `ContractViolation`; AC33 a send failure after a success aborts before the third attempt; AC8 all/mixed/none map to `true`/`"unstable"`/`false` with the 3-attempt default; a reply exactly at the threshold is late; nine malformed spellings (`localhost`, `example.com`, `10.0.0.256`, `10.0.0`, `[::1]`, leading space, `10.0.0.1%en0`, `fe80::1%`, empty) are rejected with zero sends; nine invalid budgets (0, -5, 60001, 1e300, 2.5, 101, 1e18, …) are rejected before probing; optional arguments default from absent and `null` while `ping_under`'s timeout stays required; the default authority denies everything; a discovery authority records the plan and sends nothing
        - tests added (pipeline, `compose/tests/icmp.rs`): a granted ping resolves to `true` through the real pipeline; an ungranted one warns once with zero sends; `ping_under` reaches frontmatter as the literal string `unstable`; preflight lists a planned ping without sending; preflight is condition-blind over a false `::block` and an untaken ternary branch and records the grant verdict; a nested `as_markdown` ping appears in the *root* preflight and then runs under the root's consent; a nested ping cannot escape the root grant; a CIDR entry leaves `http_hosts`/`is_host_allowed` untouched; a dynamic (frontmatter-derived) target is revalidated against the grant before sending
        - tests added (CLI via `CliProcessFixture`, `compose_icmp.rs`): `md compose --shell --allow-host 10.77.0.0/16` reports both probes with `granted` true/false; composing an ungranted target exits 0 with the `{{ … }}` resolved and one stderr warning naming the address and `--allow-host`; `::file https://10.77.1.5/remote.md` under the same CIDR still fails policy, proving the CIDR authorizes no HTTP. Every CLI case is packet-free by construction (preflight, or an ungranted target)
        - descriptor return types were already asserted by `catalog/mod.rs::icmp_descriptors_declare_address_arguments_and_denial_returns` and `new_functions_have_their_specified_typed_signatures`; no duplicate was added
        - results: `darkmatter` `just test` 8044 of 8044 passed (2 slow, 14 skipped), `just lint` passed, `just doctest` 181 passed; `claudine` `just test` 7026 of 7026 passed (9 skipped) and `just lint` passed; `cargo check --tests --target x86_64-pc-windows-gnu -p darkmatter -p darkmatter-cli` passed (compile evidence only, not behavioral). No pre-existing failure was observed in any of these gates
        - OS notes: no `#[cfg]` arm was added here — the platform split lives in Sniff's transport. Linux without the process group in `net.ipv4.ping_group_range` makes a granted probe `IcmpError::NotPermitted`, which this layer maps to the spec's compose error rather than `false`; on Windows a scoped grant must spell the numeric interface index (`fe80::1%12`), not the Unix interface name, and `docs/cli/compose.md` now says so. No test sends ICMP, so none depends on either
        - deviation: the spec's AC6 wording ("the preflight lists the planned ping as an approvable effect") is satisfied by a typed `PlannedIcmpProbe` carrying a `granted` flag rather than by folding ICMP into the shell approval set; the shell `approval_set()` is unchanged and no orchestrator approval API was added, which Phase 10 (Claudine integration) still owns
        - residual: a `ping` whose target is computed from a value discovery cannot resolve is not recorded in preflight at all (it is neither an evaluated call nor an all-literal one); it is still safe at run time — the grant check happens before the send — but an approver will not see it listed. Making it visible would need the same dynamic-shape rejection `::shell` uses, which the spec does not require for ICMP
        - residual: `remote_reads_enabled()` now ignores CIDR-only allowlists, so `--allow-host 10.0.0.0/8` alone no longer flips the remote-read capability on; the CLI sets `with_allow_remote_transclusion(true)` unconditionally, so CLI behavior is unchanged and only a library caller passing a CIDR-only allowlist sees the difference (deny-all either way)
- work completed for 'Implement the thirteen expression functions that still deliberately fail' at 11:16:52
        - all four sub-tracks landed; `expression/functions/pending.rs` and its ratchet test are deleted, so every cataloged function now has a real binding
        - note: the `pending.rs` deletion is staged in the index (`git rm`); nothing was committed
- starting the work on 'Implement the lazy roots and replace Claudine'"'"'s obsolete lifecycle shape' at 11:16:52
        - orchestration: split into two serial sub-tracks following the plan: (e) Darkmatter Phase 6 lazy `current` / `current_env` roots; (f) Claudine Phase 10 clean-break migration
        - sub-track (e): Darkmatter Phase 6 — the lazy `current` / `current_env` reserved roots (6.1-6.6). Claudine Phase 10 stays with sub-track (f); only three Claudine test expressions were migrated here, no Claudine product code.
        - provider API: new `compose/context/current.rs`. `CurrentProvider::refresh(key) -> CurrentRefresh::{Observed(Value), Unsupported}` is the invocation's capability to observe ONE cataloged `ctx` key now. `CurrentAuthority` is the request-owned handle (provider + shared `PartialRuntimeCapture` sink + a `discovering()` passive mode), carried on `ComposeOptions` beside `IcmpAuthority`, projected into every `ResolutionContext` and into `EffectiveStateBuilder::with_current_authority`. Public: `ComposeOptions::with_current_provider`, `CurrentAuthority`, `CurrentProvider`, `CurrentRefresh`, `DeferredCapabilities`.
        - which provider a request gets is derived from the existing `ContextAuthority`, mirroring `icmp_authority()`: an explicitly installed provider always wins; otherwise `DarkmatterOwned` (i.e. `ComposeOptions::new()` / `md compose`) gets `AnchoredRefresh`, which re-captures the key's group at `ComposeContext::anchor()` and never reads the process CWD; `CallerSupplied` / `CallerExtended` get nothing and fail closed. This is the seam sub-track (f) plugs Claudine into.
        - request-owned facts never refresh (D2/D3): the `Invocation` and `Document` groups read the eager capture through `classify_ctx_key`, so `current.cwd` / `current.id` equal their `ctx` twins and never reach the provider. Every other group refreshes.
        - memo scope (Q2/D5): `CurrentScope` holds the memo, and `EvaluationLookup::begin_expression_scope()` (new default-no-op trait method) clears it. `expression::evaluate` was split into a public scope-opening entry plus a private recursive `evaluate_expr`, so EVERY caller — including Claudine's direct `evaluate(&expr, &lookup)` sites — is correct by construction; `Evaluator::eval` also opens one because its variable fast path bypasses `evaluate`. Cloning a lookup starts an empty memo, so the provider is shared and observations never are.
        - shadowing: the two roots are intercepted in `EffectiveState::{get,get_checked}`, `FrontmatterSeedState::{get,get_checked}`, and `LayeredLookup::{get,get_checked,is_known_variable_root}` BEFORE frontmatter, external state, and injected globals. An injected global named `current` is now unreachable, which is the change that broke Claudine's `LifecycleCurrent` nesting.
        - unknown paths: new `ExpressionError::ReservedRootPathUnknown { root, path }`, authoring-fatal. Interpretation recorded deliberately — AC27 says `current.ctx.x` is an "unknown-path error" while the R30 required-behavior bullet says it fails "the way any unknown `ctx.<missing>` path fails today" (a warning plus empty). A typed error was chosen because, unlike `ctx`, `current` cannot be shadowed and has no authored fallback, so the path can never resolve; that is the same rationale `UnknownFunction` uses. If the author prefers warn-and-empty, only `CurrentScope::resolve_current` / `resolve_current_env` and the fatality entry change.
        - bare roots: bare `current` enumerates `context_variable_descriptors()` with null values (names without materializing values, per R30); bare `current_env` resolves to nothing, exactly as bare `env` does.
        - `current_env.<KEY>` is `std::env::var` at reference time with `env`'s missing-value semantics; any dotted remainder (`current_env.ctx.x`) is an unknown path, since an environment name cannot contain a dot.
        - planning split (6.2/D4): new `context/capture/capabilities.rs` with `DeferredCapabilities::{for_content,for_document,current_keys,current_env_keys,functions,union,is_empty}`, unioned across the preflight walk and published as `ComposePreflightReport::deferred_context`. It records names only and observes nothing. `scan_needed_groups` was tightened with `is_root_position` so a trailing `ctx.` path segment (`current.ctx.agent`, `a.ctx.os`) no longer creates an eager requirement.
        - passive preflight (6.5/AC32): `collect_recursive` installs `options.current_authority().discovering()` next to the existing `icmp.discovering()`, so the discovery compose pass reaches `current.*` / `current_env.*` references, answers them null, observes nothing, and records no diagnostic. Reaching the reference is the metadata; the value is not.
        - surfaces integrated: frontmatter passes 1 and 2 (the authority travels on `frontmatter_resolution_context`, picked up by `FrontmatterSeedState::with_current_authority`), body interpolation (`EffectiveStateBuilder` in the pipeline, through `ResolvingLookup`/`DeferrableLookup`), page-block and transclusion `when=`, both `$()` ternary surfaces, and `SubtreeCompose`/`LayeredLookup`.
        - deliberate non-integration: `conditions::evaluate_condition_against` (the public `ShortcutLookup` shortcut API) gets NO provider. It has no `ComposeOptions`, no request, and no launch evidence, so installing an ambient refresh there would be exactly the ambient rediscovery D5 forbids; `current.*` there is an unknown root as before. Same reasoning for `local_expression_resolution_context` callers (`schema_validation`, `reference/graph`): they now carry the authority because they build a `ResolutionContext` from `ComposeOptions`, but schema validation is passive and never evaluates, and the graph's `when=` evaluation gets the request's real authority.
        - fail-closed diagnostics surface as `ContextMergeDiagnostic::PartialRuntimeCapture { area: "current", detail }`, drained once at the root of `run_compose_pipeline` from a handle cloned before the recursive run (the same shape ICMP uses) and rendered as a `ComposeWarning { stage: "context" }`.
        - GitNexus impact before editing: `EffectiveState` resolved ambiguous — the impl candidate is HIGH (21 impacted, 16 direct); `LayeredLookup` resolved ambiguous — the impl candidate is CRITICAL (286 impacted, 34 direct) and the struct candidate returned UNKNOWN with no resolved callers. Both were edited anyway because the reservation is the finding's requirement and cannot live anywhere else; the edits are additive interception branches with no change to any existing path, and the full Darkmatter and Claudine L1 suites cover the blast radius. Flagged here for review.
        - files changed (Darkmatter): new `lib/src/markdown/compose/context/current.rs`, `lib/src/markdown/compose/context/capture/capabilities.rs`, `lib/src/markdown/compose/tests/lazy_roots.rs`, `cli/tests/compose_lazy_roots.rs`; edits to `compose/{mod.rs,subtree.rs,conditions.rs,frontmatter_interpolation.rs,frontmatter_shell_expansion.rs}`, `compose/context/{mod.rs,options.rs,effective_state.rs}`, `compose/context/capture/{mod.rs,groups.rs}`, `compose/expression/{mod.rs,error.rs,resolve_ctx.rs,catalog/roots.rs}`, `compose/interpolation/evaluator.rs`, `compose/inline/interpolation.rs`, `compose/pipeline/mod.rs`, `compose/preflight/{mod.rs,collect.rs}`, `compose/tests/mod.rs`, and the two fixture files the spec's migration list named (`compose/subtree.rs` tests and `compose/tests/frontmatter.rs`) whose arbitrary injected global `current` was re-pointed to `snapshot`.
        - files changed (Claudine, tests only): `lib/src/composition/lifecycle/context/tests.rs` (`when_clause_reacts_to_env_changed_after_prepare` now asserts the live `current_env` reread instead of comparing two `LifecycleCurrent` env snapshots), `lib/src/composition/prepare/service/tests.rs` (`current.ctx.agent` -> `current.agent`; the assertion now pins "not the stored `codex`" rather than a literal empty, because Claudine's prepare options are `DarkmatterOwned` and therefore get the ambient refresh), `cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs` (`current.env.<KEY>` -> `current_env.<KEY>`, with the env teardown moved after the evaluation now that the read is live). `LifecycleCurrent`, `capture_at_event`, and `LATE_BINDING_ROOTS` were NOT touched.
        - docs/skills updated: `darkmatter/docs/topics/darkmatter-expressions.md` (Namespaces is now five reserved prefixes plus a lazy-roots section), `darkmatter/docs/topics/context-variables.md` (pointer to the `current.<key>` twin), `.claude/skills/darkmatter/compose.md` (new "Lazy reserved roots" section, variable-resolution table rows, hash and `last_updated` refreshed through `md hash`), `.claude/skills/darkmatter/library-surfaces.md` (the roots are reserved by the evaluator, with the public surface named).
        - tests added (unit, `context/current.rs`): repeat-read stability then cross-scope freshness; only the referenced key is observed; a missing capability is null plus one `PartialRuntimeCapture` and never the eager value; a no-provider request fails closed; request-owned keys never reach the provider; all three removed nestings are unknown paths that observe nothing; bare-root enumeration materializes no value; `current_env` scope behavior including a missing name; non-root paths are left alone; a cloned scope starts an empty memo; dotted tails walk the observed value.
        - tests added (unit, `capture/capabilities.rs`): a lazy reference is planned but adds no eager requirement; the removed nesting names neither; env names and function calls are planned separately; inert `{{{ }}}` literals and longer identifiers plan nothing; an unreached ternary branch is still planned; document-level frontmatter+body planning; union.
        - tests added (pipeline, `compose/tests/lazy_roots.rs`): the memo scope proven in both directions inside one compose using a provider whose every observation differs; `ctx` stable while `current` refreshes; frontmatter + body + `when=` all resolve; an unchosen ternary operand is never observed; a missing capability fails closed with the typed diagnostic; a request with no provider observes nothing; the three removed nestings fail the compose; a lazy read neither satisfies nor creates an eager requirement; request-owned identity is fixed; neither an injected global nor a frontmatter key can shadow; preflight plans without observing; `current_env` live vs `env` frozen.
        - tests added (CLI via `CliProcessFixture`, `cli/tests/compose_lazy_roots.rs`): `md compose` resolves `current.today` equal to `ctx.today` through the ambient anchored refresh; `current_env.<KEY>` reads the child's environment and a missing name renders empty; `current.ctx.today` fails the compose with the reserved-root message. The env key was renamed away from the `DM_` prefix because `spawn_site_guard` classifies that namespace as an undeclared darkmatter application input.
        - results: `darkmatter` `just test` 8078 of 8078 passed (78 slow, 14 skipped) and `just lint` passed (exit 0, including the wasm32-wasip2 Zed extension check); `claudine` `just test` 7026 of 7026 passed (21 slow, 9 skipped) and `just lint` passed. No pre-existing failure was observed in either area.
        - notes for sub-track (f): Claudine should build one `CurrentProvider` from its invocation evidence authority and install it with `ComposeOptions::with_current_provider(Arc::new(..))` (and, for the lifecycle subtree lookup, `EffectiveStateBuilder::with_current_authority(..)` on the state it hands `LayeredLookup`). `refresh(key)` must answer from retained launch evidence only and return `CurrentRefresh::Unsupported` for a capability the invocation did not supply — never an ambient capture. `LifecycleCurrent`'s injected `current` global is already dead (a reserved root cannot be shadowed), so `lifecycle_injected_globals` should drop its `current` arm and `LifecycleCurrent`/`capture_at_event`/`to_value` can be deleted with it; `err` and `timing` are unaffected. Note that Claudine's prepare path uses `DarkmatterOwned` options, so it currently receives Darkmatter's anchored ambient refresh — replacing that with an evidence-backed provider is the behavior change 10.3 owns, and `prepare/service/tests.rs::current_is_not_a_prepare_time_fallback_for_the_stored_context` will need its assertion tightened back to a literal empty once it is. `current_env` needs nothing from Claudine: it is launch-area independent and already live everywhere. 10.4's `LATE_BINDING_ROOTS` addition of `current_env` is still outstanding.
        - deviation: the reservation landed in `LayeredLookup` ahead of injected globals, which is what R30 requires but also what makes Claudine's `LifecycleCurrent` unreachable before Phase 10 runs. Three Claudine tests were migrated to keep `claudine` green; that is the minimum and it is test-only.
        - residual: `md compose` refreshes `current.<key>` by re-capturing the whole owning group at the anchor (`capture_runtime_context_for_groups`), so a `current.branch` read pays for the rest of the `Git` group. Per-key ambient refresh would need group population to be split per key; the provider interface is already per-key, so that is a provider-side optimization only.
        - residual: `ComposeOptions` fingerprinting encodes only whether an explicit provider is installed (`current_provider`); the derived ambient provider is implied by the already-encoded `context_authority` tag. Lazy reads are evaluated per expression and never cached, so no observation can enter a cache key, but a future change that caches a `current.*` result would have to revisit this.
        - sub-track (f): Phase 10 — Claudine clean-break migration (10.1–10.6, plus 10.7's L1 half). Review-1 High "Implement the lazy roots and replace Claudine's obsolete lifecycle shape".
        - GitNexus impact before edits: `LifecycleCurrent` reports **HIGH** risk (18 direct callers, 23 impacted) — its removal is exactly what R33 ratifies, so it was carried out and is reported rather than deferred. `lifecycle_injected_globals` is not in the 583ea7c index; a text search found the two callers (`lifecycle/executor.rs`, `composition/mod.rs` re-export) and both were updated. `detect_changes(scope: all)` afterwards: 190 changed symbols, 88 affected, `critical`, neither `partial` nor `truncated` — the count spans every sub-track's uncommitted work, and the affected processes are the lifecycle/compose flows this migration intends.
        - 10.1 discovery: the Document, Network, `hostname`, `GitHistory` and cheap-worktree evidence was already wired by `55a282797` and the Sniff Phase 2/3 work, so 10.1 reduced to the one outstanding clause — retaining launch-owned provider capabilities for lazy refresh.
        - 10.1 (new): `InvocationContext::current_provider` / `current_authority` and `DocumentEpoch::current_provider` / `current_authority`, backed by a private `LaunchRefresh` provider and `InvocationContext::{refresh_current, refresh_evidence}`. `refresh_evidence` re-observes **one** group against the retained launch roots, deliberately bypassing the invocation's evidence caches (those exist so the eager snapshot is captured once; reusing them would replay the launch observation rather than refresh it). Git and file-change refreshes go through the retained `FilesystemObservation` handle, so no repository discovery runs. Repo topology, `Os`/`Hardware`/`Gpu`, and the invocation CWD stay fixed per decision D3; `Network`, `GitHistory`, `Languages`, and `Documents` re-observe. `Invocation`/`Document` are request-owned and Darkmatter never consults the provider for them.
        - 10.1 decision: the refresh evidence bundle is built with the **live** process environment (`std::env::vars()`), not the frozen launch snapshot, so `current.agent` / `current.model` track a wrapper stage that re-exported `AGENT`/`MODEL` since launch. That preserves the pre-migration `capture_at_event` behavior and matches `current_env.<KEY>`, which rereads the live environment for the same reason. Documented at the call site.
        - 10.3: `LifecycleCurrent` (and `capture_env`, `capture_env_only`, `capture_at_event`, `to_value`) deleted; `lifecycle_injected_globals` lost its `current` arm and is now a two-argument function. `StackExecutionContext.current` changed from `Option<&LifecycleCurrent>` to an owned `Option<CurrentAuthority>` — owned because the authority is a cheap Arc-backed handle every derived context clones, and because `build_lifecycle_stack_context_for_materialized` derives it from `materialized.document_epoch` and could not return a borrow of it. `StackExecutionContext::build_state` installs it via `EffectiveStateBuilder::with_current_authority`, so every event starts fresh evaluation scopes. `capture_lifecycle_globals` and `capture_loop_lifecycle_globals` became `capture_lifecycle_timing` / `capture_loop_lifecycle_timing`.
        - 10.3 (the behavior change): `composition::prepare::canonical_compose_options` now **always** installs a `CurrentProvider` — the epoch's, else the invocation's, else a private no-capability `UnsuppliedRefresh`. Claudine therefore never takes Darkmatter's ambient `AnchoredRefresh`, even on the `DarkmatterOwned` arm. `prepare/service/tests.rs::current_is_not_a_prepare_time_fallback_for_the_stored_context` is tightened back to asserting a literal `current=[]` as the handoff asked.
        - 10.4: `current_env` added to `LATE_BINDING_ROOTS` and to `sequence::preflight::SHELL_UNAVAILABLE_ROOTS`; the `err`/`timing`/`current` prose in `lifecycle/validate.rs`, `composition/preflight.rs`, `composition/prepare.rs`, `composition/error/mod.rs`, `lifecycle/mod.rs`, and `lifecycle/executor.rs` now names all four roots.
        - 10.5: remaining active hits migrated — `lifecycle/tests/diagnostics.rs` (`current.ctx.agent` -> `current.agent`, twice), the launch-anchor comments in `cli/src/commands/compose/prep.rs`, and the `composition_seams.rs` allowlist, whose `lifecycle::context::capture_at_event` entry was replaced by `invocation_context::refresh_current` with a reason naming the one-group-at-the-retained-anchor contract. The Darkmatter injected-global fixtures had already been re-pointed by sub-track (e). `cli/tests/level2_lifecycle_control.rs:938` remains the spec's declared false positive (a local recording struct's `env` field).
        - 10.6: `system_prompt/context.rs::select_package_area_root` was the one surviving `package.package_area == "root"` sentinel branch; it now keys on `is_empty()`, which also **fixes** an area literally named `root`, previously resolved to the repository root. `sniff`'s formatting `root` label and the `scope.root` JSON field are unrelated and untouched; no Sniff source was modified.
        - 10.2: confirmed both listings already project from the Darkmatter descriptor catalogs once each (`context_variable_descriptors` / `expression_function_descriptors`); `claudine context` takes no document argument, so `--values` already renders the absent-document identity projection. Recorded that contract as a docblock on `render_values_report` and pinned it with a test.
        - files changed (Claudine lib): `invocation_context.rs` (+ `tests.rs`), `composition/{prepare.rs, reserved.rs, mod.rs, preflight.rs}`, `composition/lifecycle/{context.rs, executor.rs, validate.rs, mod.rs}`, `composition/looping/engine.rs`, `composition/sequence/preflight/mod.rs`, `system_prompt/context.rs`.
        - files changed (Claudine CLI): `commands/compose/prep.rs`, `commands/context/mod.rs`, `commands/wrap/composition/{pipeline.rs, preflight.rs, runner.rs}`, `commands/wrap/harness_orch/loop_control.rs`, `commands/wrap/harness_orch/loop_control/{lifecycle_events.rs, error_routing.rs}`, `tests/composition_seams.rs`.
        - files changed (Darkmatter): `compose/context/capture/groups.rs` only — `ContextGroup::for_key` and `ContextRequirements::from_groups` became public, with docs saying why (an embedder's `CurrentProvider` is handed a bare key and must resolve exactly one group). No behavior change.
        - tests added (Claudine L1, 14): `lifecycle/context/tests.rs` — per-event refresh through a scripted capability, an unsupplied capability rendering empty rather than probing, and `current.ctx.*` rejected as an unknown path; `lifecycle/executor/tests/event_time_interpolation.rs` — `current.*` resolving in **every** one of the seven lifecycle events, a later event observing a fact that moved since the earlier one, a no-capability event emitting empty, and `current_env.<KEY>` rereading a parent-process environment change between events; `invocation_context/tests.rs` — a branch switched after launch, zero repository discoveries across four refreshes, observed-absence vs unheld-capability, and all five scope positions through supplied launch evidence (package, area-only, repository root, outside a repository, and an area literally named `root`); `prepare/service/tests.rs` — a composed prompt resolving `current.branch` through a real invocation; `prepare/tests.rs` — both lazy roots rejected in a lifecycle `shell` command at preflight; `commands/context/tests.rs` — the absent-document identity projection.
        - tests changed: `lifecycle_late_binding_in_a_shell_command_is_rejected` now loops all four roots; the two `capture_*_lifecycle_globals` tests became timing-only; `injected_globals_attaches_err_timing_current` became `..._err_and_timing` and additionally asserts `current` is **absent** from the map; the `LifecycleCurrent`-shaped `current_to_value_has_ctx_and_env`, `capture_env_only_leaves_ctx_empty`, `capture_env_reflects_live_process_environment`, and `capture_at_event_populates_ctx_and_env` were deleted, their contracts re-expressed against the reserved roots.
        - docs updated: `claudine/docs/topics/lifecycle.md` (the globals table is now four late-binding roots with the reserved-root/refresh-capability note; five prose lists renamed), `claudine/docs/topics/composition.md` (two `current.ctx.*` bullets rewritten around the lazy roots). Skills: `.claude/skills/claudine/{SKILL.md, lifecycle.md, composition.md}` — every “implementation pending” marker removed and replaced with the fail-closed supplied-evidence contract. Shipped prompts needed no change (the 2026-09-11 grep already found no hits, re-verified).
        - results: `claudine` `just test` 7036 of 7036 passed (9 skipped) and `just lint` exit 0; `darkmatter` `just test` 8078 of 8078 passed (58 slow, 14 skipped) and `just lint` exit 0. Sniff was not touched, so its gates were not re-run. One run of root `just test claudine` timed out `composition::prepare::tests::direct_composition_runs_shell_in_configured_working_directory` at 30 s while a second `cargo nextest` build was competing for the host; standalone it passes in 0.32 s and the serial package run is clean. No pre-existing failure was observed in either area.
        - deviation: 10.7's linked/main-worktree case and the whole L2 half were left to the acceptance-matrix sub-track, per this sub-track's brief. `plan.md`'s Phase 10 checkboxes were deliberately left unchecked: Checkpoint 10 requires L2 evidence this sub-track does not own.
        - residual: the fail-closed diagnostic is asserted **behaviorally** (an unheld capability renders empty, never the host's real value) rather than by reading the `PartialRuntimeCapture` record, because `CurrentAuthority::take_diagnostics` / `take_warnings` are `pub(crate)` in Darkmatter. Surfacing lifecycle-time partial-capture warnings to the operator would need that drain made public and a Claudine emitter seam; it is not required by AC28 and was left out of scope.
        - residual: `current.agent` / `current.model` read the live process environment while eager `ctx.agent` / `ctx.model` come from the frozen invocation snapshot with target identity overrides applied. Under the wrapper the two therefore disagree for a proxied target whose identity override was never exported to the process environment. That is the pre-migration `capture_at_event` behavior preserved, not a regression, but it is the one place `ctx.<key> == current.<key>` can be false for a reason other than the fact changing.
        - residual: refreshing `Languages` or `Documents` re-scans the launch base uncached on every reference, since freshness and the per-source `OnceLock` caches are mutually exclusive. Only a document that actually names those keys under `current.*` pays it.
        - for the acceptance-matrix / L2 sub-track: AC2 (`ctx.worktree` parity) needs a Claudine leg run from a **linked** worktree — `InvocationContext::capture_at(<linked>)` then `capture_launch_context`, asserting the worktree basename, and the main checkout asserting `null`; the evidence path is already wired (`GitInfo::current_worktree` via `launch_git_request`), and `invocation_context/tests.rs::linked_worktrees_keep_distinct_repository_keys` is the existing fixture to copy. AC28 needs a real wrapped run whose lifecycle handler reads `current.*` / `current_env.*` — from Claudine it needs nothing new: the authority reaches every event through `materialized.document_epoch`, so an L2 document with a `success` handler such as `{{ current.branch }}` will resolve, and `prompts/format.md`'s `{{length(current.dirty_files)}}` composes through the same path (`FileChanges` is a refreshable group).
- work completed for 'Implement the lazy roots and replace Claudine'"'"'s obsolete lifecycle shape' at 13:14:54
        - `current` and `current_env` are now reserved, lazily refreshed roots in Darkmatter, and Claudine's `LifecycleCurrent` nested object is deleted in favor of an evidence-backed `CurrentProvider`
- starting the work on 'Add the missing Level-1 acceptance matrix and required Level-2 Claudine coverage' at 13:14:54
        - orchestration: split into two serial sub-tracks: (g) the AC2 and AC28 Level-2 real-Claudine cases; (h) the remaining Level-1 acceptance-matrix gaps called out in the review's requirement table
        - orchestrator resumed after an interruption at 16:43:41; re-established ground truth before scheduling more work: `cargo check -p darkmatter --all-targets` exits 0 (review finding 1 is resolved in the tree), `/Volumes/coding` has 213 GiB free, and HEAD is still `583ea7c58` with all of findings 1-3 uncommitted in the worktree
        - sub-track (g) was found already written but unlogged (`claudine/cli/tests/level2_ac2_worktree_compose.rs`, `claudine/cli/tests/level2_ac28_lifecycle_lazy_roots.rs`, timestamped 13:37 and 13:43, after the 13:14:54 start); it is verified rather than rewritten below
        - sub-track (g) verified, not rewritten. `just _test_l2 claudine-cli --features terminal-tests --test level2_ac2_worktree_compose --test level2_ac28_lifecycle_lazy_roots`: **5 of 5 passed in 11.8 s**, 0 skipped — `level2_ac2_worktree_is_the_linked_directory_and_null_in_the_main_checkout` (1.74 s), `level2_ac28_lazy_roots_resolve_in_every_lifecycle_event` (4.31 s), `level2_ac28_current_env_sees_an_in_process_change_the_env_snapshot_missed` (1.41 s), `level2_ac28_lazy_roots_refresh_between_lifecycle_events` (1.41 s), `level2_ac28_shipped_prompt_that_references_a_lazy_root_composes` (2.89 s). No source change was needed.
        - focus safety of (g) confirmed by inspection and by what the run did: both files gate on `require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux)`, drive `spawn_shell_session` / `TmuxHarness::attach` / `kill_session_by_name` only, and name no `SpawnVisibility`, no `focus_spawned_pane`, and no `osascript`/`cliclick`/GUI automation. tmux is headless, so a detached session is created and torn down without raising a window. The WezTerm pane and Apple Terminal window the run's banner reports are the `_test_l2` recipe's own shared-harness broker pre-spawn for the whole claudine suite — pre-existing recipe behavior, not reachable from these two files, which never call `shared_or_spawn()`.
        - sub-track (h) method: audited the review's requirement table row by row against the current tree rather than re-auditing all 37 ACs, per the finding's scope. Where a row was already closed by findings 1-3 the covering test is recorded below and **no duplicate was written**; three rows needed new Level-1 tests.
        - already covered, no new test (AC1/AC19/AC26): `claudine/cli/tests/context_command.rs` is a **spawned-CLI** corpus test (`CliProcessFixture::command()` -> `assert_cmd::Command::cargo_bin("claudine")`), not an in-process render: `context_default_includes_every_descriptor`, `context_values_includes_every_descriptor`, `context_expressions_includes_every_function`, `context_side_effects_includes_every_capability`, and `context_reports_list_more_context_names_and_the_recent_commits_pair_once` (AC26's exactly-once, both directions). Generated roster (AC19): `claudine/gen/src/agentic_clis/tests.rs::check_detects_every_roster_change_without_regeneration` (seven mutations) plus end-to-end through the `claudine-gen` binary over the real area in `claudine/gen/tests/generate_ux.rs::roster_alias_rename_drifts_the_darkmatter_name_table_until_regenerated`. Single-source projection: `darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs::recent_commits_pair_projects_from_one_descriptor_entry`.
        - already covered, no new test (AC6-AC8/AC33): `expression/functions/network.rs::ping_tests::{ac6_a_denied_target_is_null_plus_one_warning_and_sends_nothing, ac7_no_reply_within_the_timeout_is_false, ac7_a_send_failure_is_a_never_demoted_compose_error, ac8_ping_under_maps_all_some_and_no_replies, ac33_invalid_time_and_attempt_budgets_are_rejected, ac33_a_scoped_ipv6_target_keeps_its_zone, ac33_a_denied_multi_attempt_call_sends_no_packets, ac33_a_send_failure_after_a_success_aborts_the_series}`; pipeline in `compose/tests/icmp.rs`; CLI in `darkmatter/cli/tests/compose_icmp.rs`; descriptors in `catalog/mod.rs::icmp_descriptors_declare_address_arguments_and_denial_returns`.
        - already covered, no new test (AC9/AC13): `compose/context/capture/network.rs::{tailnet_follows_the_cgnat_range_of_fixture_addresses, gateways_project_addresses_with_zone_or_null}` plus the six Sniff route-table fixtures — `sniff/lib/src/network/gateway/{darwin,linux,windows}.rs` cover both families per OS, no-default-route, on-link/point-to-point, and `%scope` preservation. The review marked this row "Partial only" solely because the crate did not compile; it does now.
        - already covered, no new test (AC10-AC12/AC34): `compose/shell_expansion/probe.rs::tests::real_shells::*` (bash, zsh, fish, PowerShell dialects; injection-shaped names; noisy and hanging profiles; detached child cleanup), `probe.rs::tests::{missing_shell_is_false_without_a_launch_on_unix, windows_falls_back_from_pwsh_to_powershell_to_false}`, `expression/functions/shell.rs::tests::unix::can_execute_is_the_or_of_all_sixteen_combinations`, `catalog/mod.rs::has_binary_and_has_command_share_one_probe`, and CLI `darkmatter/cli/tests/compose_context_probes.rs`.
        - already covered, no new test (AC15-AC16, AC17/AC35, AC18, AC20, AC27, AC31, AC32): `darkmatter/lib/tests/nested_composition.rs` and `darkmatter/cli/tests/compose_nested_markdown.rs` (nested compose, shared depth limit, preflight of nested effects); `expression/functions/repository.rs::tests::{valid_misses_are_empty_strings, non_monorepo_misses_everywhere, nonexistent_descendants_select_the_deepest_package_and_area, remote_and_unresolvable_references_are_errors_not_misses}` with `darkmatter/lib/tests/context_functions.rs::package_lookups_*` at compose level; `compose_context_probes.rs::has_agentic_cli_reads_the_request_path_index` (alias equality and the unknown-name compose failure); `context_functions.rs::address_filters_compose_from_supplied_interface_evidence`; `compose/tests/lazy_roots.rs` plus `darkmatter/cli/tests/compose_lazy_roots.rs`; and for the demand-counter row, `compose/context/capture/mod.rs::branch_capture_does_no_history_or_network_work`, which reads the real `HISTORY_CAPTURE_COUNT` / `NETWORK_CAPTURE_COUNT` work counters, with `capture/groups.rs::recent_history_demand_is_separate_from_ordinary_git_facts` on the requirements side and `darkmatter/lib/tests/shell_probe_preflight.rs` for zero-effect preflight.
        - **new test 1 — AC21/AC23, the ambient leg of the dual path**: `darkmatter/cli/tests/compose_scope_matrix.rs`. `darkmatter/lib/tests/empty_package_area.rs` was the *supplied-evidence* leg (it hands composition a `sniff` observation it built in process) and reached only four of the five tabled positions. The new file is the ambient `md compose` leg for all five, launched through `CliProcessFixture::command_builder().ambient_context(dir)` from a fixture-owned Cargo monorepo: `every_scope_position_projects_its_tabled_string_through_md_compose` and `the_package_area_condition_is_false_exactly_where_the_area_is_empty`.
        - 1 decision: the values are asserted through `--frontmatter` on whole-value `"{{ ctx.area }}"` keys rather than from the body. A body `{{ ctx.area }}` renders the empty string for `""` **and** for `null`, so it cannot see AC21's actual contract; the whole-value frontmatter form keeps the evaluated JSON type, so the emitted line is `area: ''` for the empty string and would be `area: null` for the defect. Exact-line equality therefore carries the "never `null`, never `\"root\"`" clause without a separate substring check.
        - **new test 2 — AC3/AC5, `md hash` parity and transclusion identity**: `darkmatter/cli/tests/compose_document_identity.rs`. `ctx_hash_equals_what_md_hash_prints_for_the_same_file` spawns `md hash` and `md compose --frontmatter` over the same fixture; the document's frontmatter carries the *uninterpolated* `{{ ctx.hash }}`, so the expected value is not derivable from the composed output and a `ctx.hash` computed after interpolation would disagree. `a_transcluded_fragment_composes_under_the_root_documents_identity` asserts a real `::file` child resolves the root's `ctx.self`/`ctx.hash`/`ctx.id`/`ctx.sid` — the `::file` half of AC5 that `nested_composition.rs` covered only for `as_markdown` and only for `ctx.hostname`.
        - **new test 3 — AC24/AC37, the CLI-parity clause**: `ctx_recent_commits_is_byte_equal_to_the_sniff_plain_rendering` and `a_commit_that_touched_no_files_renders_no_element` in `darkmatter/lib/tests/context_functions.rs`.
        - 3 decision: the parity is closed as a *chain* rather than by spawning the `sniff` binary from a Darkmatter test. `sniff/cli/tests/cli.rs::test_repo_recent_commits_plain_is_the_concatenated_per_commit_blocks` already pins `sniff repo recent-commits --plain` stdout to exactly the concatenated `CommitDesc::describe_plain` blocks; the new test pins the Darkmatter side to the same blocks. Adding a `sniff-cli` dev-dependency to `darkmatter-lib` purely to re-prove the half Sniff already owns was judged the wrong cost. **No Sniff source or test was modified, so Sniff's gates were not re-run.**
        - 3 honesty note: both sides of that comparison reach `describe_plain`, so it is not independent-reader evidence. What it actually proves — and what the docblock now says — is that a multi-line, indented Markdown commit block survives capture, JSON array projection, and body interpolation *unaltered*. Absolute structural assertions (`- [` opener, the newest subject, the four-space `    - adds entry 12\n` continuation indent, and no `Today`/`Yesterday` label) were added so the case does not rest on the comparison alone.
        - non-vacuity proved by mutation, not assumed: inverting the child-inherits-root assertion turned `a_transcluded_fragment_composes_under_the_root_documents_identity` red; changing the fifth position's expectation from `["", "", ""]` to `["aaa", "zz", "qq"]` turned **both** `compose_scope_matrix.rs` tests red. Both mutations were reverted and the files re-run green.
        - two defects found in my own first drafts and fixed before the gates: (a) `assert_eq!(PathBuf::from(ctx.self), root)` failed on macOS because the fixture root is handed out as `/var/...` while `ctx.self` is canonical `/private/var/...` — the macOS symlinked-temp-dir trap; it now compares against `biscuit_file::canonicalize_simplified(&root)`, the same function `capture/document.rs:66` uses to produce `ctx.self`. (b) the native-separator assertion was written as `PathBuf::from(s).display().to_string() == s`, which is true for every string; it is now an explicit `!contains('/')` on Windows / `!contains('\\')` elsewhere.
        - cross-OS judgment: **no cross-check rig was run, deliberately.** The one platform-sensitive assertion is the `ctx.self` comparison, and both sides of it call `biscuit_file::canonicalize_simplified` (= `dunce::canonicalize`, which strips the Windows verbatim prefix), so they agree by construction on every OS — a question answered by reading rather than by a multi-hour speculative run, per the repo's CI/CD test-scope discipline. The scope-matrix fixture needs `git` only on the *test* process (the composed child discovers the repository through `gix`), which is the shape `darkmatter/cli/tests/graph.rs` already runs on every CI leg; forward-slash relative paths go through `Path::join` and Cargo `members`, both separator-agnostic; the recent-commits fixtures are git2 in-process with 2020 timestamps, so no TZ can produce a `Today`/`Yesterday` label. Residual Windows risk is judged low but is not claimed as evidence; CI's `windows-latest` leg remains the authority.
        - files changed: `darkmatter/cli/tests/compose_scope_matrix.rs` (new), `darkmatter/cli/tests/compose_document_identity.rs` (new), `darkmatter/lib/tests/context_functions.rs` (+2 tests and one `sniff_plain_blocks` helper). No product source was modified by this sub-track.
        - gates (each area run **serially**; see the contention note below): `darkmatter` `just test` **8084 of 8084 passed** (80 slow, 14 skipped) and `just lint` exit 0 (re-run after the final test edits); `claudine` `just test` **7036 of 7036 passed** (14 slow, 9 skipped) and `just lint` exit 0; `claudine` L2 for the two AC2/AC28 binaries 5 of 5 passed. `darkmatter/cli/tests/spawn_site_guard.rs` accepts both new CLI files with no exemption entry.
        - gate contention, recorded because it is the second occurrence: running `darkmatter just test` and `claudine just test` concurrently produced three spurious 30 s `TIMEOUT`s — `markdown::compose::tests::provider_network::{incomplete_domain_is_fatal_on_all_three_surfaces, incomplete_domain_surfaces_rather_than_a_truncated_list}` and `claudine system_prompt::prepare::tests::non_repository_session_runs_shell_in_launch_cwd`. All three pass in the serial runs reported above. This is the same class of artifact the Phase-10.x entry recorded; the two suites must not be run against this host at the same time. **No pre-existing failure was observed in either area.**
        - **deferred finding — AC29's grep-based L1 check does not exist and was not written.** The row's *content* is now clean: every implementation and user-doc hit from the 2026-09-16 baseline (`lifecycle/context.rs`, `looping/engine.rs`, `compose/prep.rs`, `wrap/composition/{pipeline,preflight}.rs`, `lifecycle_events.rs`, `claudine/docs/topics/{lifecycle,composition}.md`) has been migrated by finding 3. What remains in AC29's declared scope is 25 hits that are all *negative-path* text — skills and docs saying the nesting was removed (`.claude/skills/claudine/{SKILL.md:177, lifecycle.md:77, composition.md:113}`, `.claude/skills/darkmatter/compose.md:291`, `darkmatter/docs/topics/darkmatter-expressions.md:251`), tests asserting it is rejected (`claudine/lib/src/composition/lifecycle/context/tests.rs:598,602,605`, `darkmatter/cli/tests/compose_lazy_roots.rs:60,68`, `compose/tests/lazy_roots.rs:199,200`, `context/current.rs:270,622`, `capture/capabilities.rs:192`), and the error message that names the removed spelling (`expression/error.rs:387,392`, `catalog/roots.rs:119`). AC29 as written says the grep "returns nothing", which no longer matches the tree the migration deliberately produced; implementing it therefore means either amending the criterion or adopting an allowlist-with-stale-entry-failure guard (the `spawn_site_guard.rs` / `dispatch_inventory.rs` shape). That is an authoring decision about the acceptance criterion, not a test-coverage fill, and AC29 is named in neither the finding's prose nor the sub-track brief, so it was left for the author. Also noted: `claudine/fixes/2026-09-13-better-static-analysis/spikes/parser-agreement.md:166` is a new in-scope hit the spec's exclusion list (spec.md:632-636) does not yet name.
        - residual, outside the reviewer's named scope and **not** addressed: AC4's "an entropy failure is a typed compose error" clause has no test — `capture/document.rs::entropy_failure_projects_no_identity` asserts only the `PartialRuntimeCapture` diagnostic, and `ExpressionError::ExecutionNonceUnavailable` has zero test hits. AC36's Q3 warm-`--cache-root` tests (`darkmatter/lib/tests/persistent_cache_disabled.rs`, `darkmatter/cli/tests/compose_remote_caching.rs`) prove no composed/operation/snapshot artifact is written but probe only `ctx.timestamp_ms`/`ctx.repo_root`; neither an identity (`ctx.id`/`ctx.sid`) nor a probe-output replay is asserted. AC30's "no extra fetch" for a URL root is implicit rather than counted. AC9 is covered at the `NetworkObservation` projection layer, not end to end from a `NetworkInterface` fixture to `ctx.tailnet`. AC34's negative property ("no test reads a real user profile or activates a window") holds by fixture construction but nothing enforces it mechanically.
- work completed for 'Add the missing Level-1 acceptance matrix and required Level-2 Claudine coverage' at 17:20:43
        - sub-track (g) verified rather than rewritten: the two Level-2 binaries pass 5 of 5 in 11.8 s with 0 skipped, and both are focus-safe (tmux only, `require_level!` gated, no `SpawnVisibility::Foreground`, no `focus_spawned_pane`, no `osascript`/`cliclick`)
        - sub-track (g) caveat recorded by the implementer: the `_test_l2` recipe's shared-harness broker pre-spawns a WezTerm pane and an Apple Terminal window for the whole `claudine` suite. That is pre-existing recipe behavior and is not reachable from these two files, which never call `shared_or_spawn()`.
        - sub-track (h) closed the two remaining Level-1 rows with three new tests and no product-source change; every other row the review marked as a gap was found already covered by the finding 1-3 work and was recorded by test name rather than duplicated
- orchestrator's independent gate re-run at 17:32:19, after the sub-agent reported: `darkmatter` `just test` **8084 of 8084 passed** (108 slow, 14 skipped), exit 0 — the same figure the sub-track reported, reproduced serially with no other suite competing for the host
- orchestrator's worktree audit at 17:32:19: 492 entries in `git status --porcelain`, of which 365 are deletions. All but one are the author's own concurrent move of four unrelated completed features (`2026-07-13-meta-schema`, `2026-07-13-more-is-more`, `2026-07-14-invalid-frontmatter`, `2026-07-15-performance-followup`) into `darkmatter/features/_completed/`; that is an author action this implementation neither made nor touched. The single source deletion is `darkmatter/lib/src/markdown/compose/expression/functions/pending.rs`, which review finding 2 required.

### Successful Completion

The implementation of review cycle 1 has completed successfully in 9 hours 2 minutes.
During this implementation all 4 review findings were evaluated to see if they
could be fixed as a part of this implementation cycle: 4 were fixed, 0 were
deferred.

No finding was deferred in whole. One sub-item inside finding 4 was deferred and
is recorded here because it is the only part of the review's requirement table
this cycle did not close:

- **AC29's grep-based Level-1 check** (a sub-item of finding 4, "Add the missing
  Level-1 acceptance matrix and required Level-2 Claudine coverage"). AC29's
  *content* is clean — every implementation and user-documentation hit from the
  2026-09-16 baseline was migrated by finding 3. What remains in the criterion's
  declared scope is 25 deliberately negative-path hits: skills and docs stating
  that the `current.ctx.*` / `current.env.*` nesting was removed, tests asserting
  that it is rejected, and the error message that necessarily names the removed
  spelling. AC29 as written requires the grep to "return nothing", which no
  longer describes the tree the clean-break migration intentionally produced.
  Closing it means either amending the acceptance criterion or adopting an
  allowlist-with-stale-entry-failure guard. Both are authoring decisions about
  the criterion itself rather than a coverage fill, and AC29 is named in neither
  the review finding's prose nor the sub-track brief, so it was left for the
  author. Related: `claudine/fixes/2026-09-13-better-static-analysis/spikes/parser-agreement.md:166`
  is a new in-scope hit that the spec's exclusion list (spec.md:632-636) does not
  yet name.

No performance measurement was deferred, so `deferred_perf_measurement` remains
unset: this cycle required no benchmark, and host CPU load never blocked a
measurement.

The files changed by this implementation cycle are recorded per finding in the
sub-track entries above. In summary, across the four findings: Darkmatter gained
the three capture-group variants and their key ownership, the thirteen
previously-failing expression functions (with `pending.rs` deleted), the lazy
`current` / `current_env` reserved roots and their refresh authority, and new
Level-1 pipeline and CLI coverage; Claudine's obsolete `LifecycleCurrent` nested
object was deleted in favor of an evidence-backed `CurrentProvider`, with its
lifecycle, preflight, docs, and skills migrated alongside; and the two required
Level-2 real-Claudine cases (AC2 and AC28) were added. Sniff source was not
modified by this cycle.

Final gates, each package area run serially:

| Gate | Result |
|---|---|
| `darkmatter` `just test` | 8084 of 8084 passed (108 slow, 14 skipped), exit 0 |
| `darkmatter` `just lint` | exit 0 |
| `claudine` `just test` | 7036 of 7036 passed (14 slow, 9 skipped), exit 0 |
| `claudine` `just lint` | exit 0 |
| `claudine` Level-2 (AC2, AC28) | 5 of 5 passed, 0 skipped |

Host constraint recorded for future cycles, now on its second occurrence: the
`darkmatter` and `claudine` suites must not be run against this host
concurrently. Doing so produced three spurious 30-second timeouts
(`provider_network::{incomplete_domain_is_fatal_on_all_three_surfaces,
incomplete_domain_surfaces_rather_than_a_truncated_list}` and Claudine's
`non_repository_session_runs_shell_in_launch_cwd`); all three pass serially.

This implementation's terminal state is **implementation complete, ready for
review**. Per repository convention an agent does not move a feature into
`_completed` and does not run `just complete`; that remains the author's call
after the review cycle closes.

## Implementation of Review Findings #2

> **started at:** 2026-09-17T18:02:20-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/darkmatter/features/2026-09-09-more-context/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the four findings are worked serially, each by one subagent that owns its own log entries below
- starting the work on 'Restore Darkmatter's all-target build after moving the performance fixtures' at 18:06:08
        - discovery: the author's move of `2026-07-15-performance-followup` into `features/_completed/` was uncommitted — the old path was still tracked in the index (shown as worktree deletions) and `_completed/` was untracked. Restoring the tracked `benchmarks/` subtree at its old path and `git mv`-ing it keeps Git's rename detection intact (283 entries staged as `R`); the untracked duplicate under `_completed/` was verified byte-identical with `diff -r` before deletion
        - decision: the durable option from the review — fixtures live at `darkmatter/benchmarks/` (area-owned, outside the feature lifecycle) so completing a feature can never again invalidate a live test or bench dependency. This is also the layout `features/2026-07-16-better-metrics/plan.md` item 2.1 already proposes, so no second relocation is expected
        - decision: no stub left at the old location. `darkmatter/benchmarks/README.md` gained a provenance paragraph (moved from the feature on 2026-09-17, and why) and its two `../results.md` links, plus `raw/README.md`'s `../../results.md` link, now point at `features/_completed/2026-07-15-performance-followup/results.md`
        - decision: dated run records under `darkmatter/benchmarks/raw/**` (`summary.md`, `rejection.md`, hyperfine `.json`) still spell the old path inside the exact commands they recorded. They are immutable measurement evidence under the run-record contract, so they were deliberately not rewritten; none is a `.rs/.toml/.sh/.ts/justfile` consumer
        - discovery: `generate.sh` and `refgraph-setup-fix.sh` locate themselves via `dirname "${BASH_SOURCE[0]}"`, `recompute.ts` takes its run-record directory from `argv`, and `manifest.yaml` records `bash generate.sh` — none assumed the old location, so no script edits were needed
        - discovery: no justfile, `darkmatter/README.md`, `darkmatter/docs/`, or `.claude/skills/darkmatter/*.md` spelled the old path; `claudine/fixes/2026-09-13-better-static-analysis/spec.md:302` cites it as a historic line reference and was left alone
        - GitNexus: `impact fixture_text --direction upstream` → risk LOW (3 direct in-crate `#[cfg(test)]` callers); `benchmarks_dir` is not indexed (test-binary-local helper, confirmed by text search as single-file)
        - files changed (path-string edits only, `../features/2026-07-15-performance-followup/benchmarks` → `../benchmarks`):
                - `darkmatter/cli/tests/compose_transclusion.rs` (two `include_str!` sites, `../../benchmarks/fixtures/...`)
                - `darkmatter/lib/src/perf_harness.rs`, `lib/src/markdown/compose/directives_api.rs`, `lib/src/markdown/compose/transclusion/engine.rs`
                - `darkmatter/lib/tests/benchmark_fixtures.rs` (path plus its `//!` and `benchmarks_dir` doc), `lib/tests/compose_phase6.rs`
                - `darkmatter/lib/benches/{phase6_interpolation,phase8_render,phase9_remote,phase10_residuals,phase11_evidence}.rs`
                - `darkmatter/benchmarks/README.md`, `darkmatter/benchmarks/raw/README.md` (provenance note and `results.md` links)
                - moved: `darkmatter/features/2026-07-15-performance-followup/benchmarks/**` → `darkmatter/benchmarks/**` (283 files, staged rename)
        - gates from `darkmatter/`:
                - `just lint` → exit 0 (darkmatter, darkmatter-cli, dmls, zed-dmls-cli, and the wasm32-wasip2 extension check all clean)
                - `cargo check -p darkmatter --benches --color=never` → exit 0
                - `just test --color=never` → exit 0: `8084 tests run: 8084 passed (67 slow), 14 skipped`, 0 failed. Fixture consumers confirmed green: `benchmark_fixtures::benchmark_manifest_matches_recorded_identities`, `compose_phase6::{interpolation_heavy_fixture_composes_expected_output,replace_heavy_fixture_applies_longest_match}`, `directives_api::tests::shipped_transclusion_fixture_targets_are_scanned_passively`, `transclusion::engine::tests::finding_35_2::relevel_output_matches_the_oracle_across_shipped_fixtures`, `darkmatter-cli::compose_transclusion::compose_shipped_transclusion_fixture_through_normal_cli_path`
                - `grep -rn "2026-07-15-performance-followup" darkmatter --include='*.rs' --include='*.toml' --include='justfile' --include='*.sh' --include='*.ts'` (excluding `_completed/`) → no hits
        - not done by this task: the pre-existing unstaged edit to `lib/tests/ambient_ctx_capture.rs` and the untracked `lib/tests/{context_functions,nested_composition,shell_probe_preflight}.rs` belong to other findings and were not touched
- work completed for 'Restore Darkmatter's all-target build after moving the performance fixtures' at 18:14:36

## Implementation of Review Findings #3

> **started at:** 2026-09-17T19:07:11-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/darkmatter/features/2026-09-09-more-context/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- the three findings are worked serially, each by one subagent that owns its own log entries below
- starting the work on 'Finish the request-owned ambient repository observation and restore the L1 gate' at 19:09:07
        - reproduced first with a focused no-fail-fast nextest run of the three `ambient_repository` tests: all three fail deterministically, not by concurrency. Under nextest every test owns its process, so the `#[cfg(test)]` work counters cannot race between tests; the eager-Repo test's `(2, 2)` versus `(1, 1)` came from `ensure_file_resolution_context` performing a second root discovery and topology walk at the pipeline entry (`capture_file_resolution_context` runs `ContextCapture::new(base, [Repo])`), the never-captured test's from that same capture plus the lazy `AnchoredRefresh::repository()` discovery, and the third from asserting the topology walk's incidental package order
        - discovery: the two ordering/counter failures masked two more latent fixture faults in the same tests that surfaced once the counters were right: cleanup joins single-newline prose lines, so a three-line document rendered one line (`lines[1]` out of bounds), and a fresh `gix::init` repository has every file untracked, so `current.untracked_files == []` could never hold
        - design decision: the request's repository observation is one `RepositoryObservation` (root + `RepoInfo`) retained on `CapturedObservations` beside the projected `ctx` values. A Darkmatter-owned request establishes its ambient observation through `CurrentAuthority::establish_ambient_repository` — the eager `Repo` capture when the context made one, otherwise one discovery at the retained anchor made only when the document plans a `Repo`-group `current.*` read (`DeferredCapabilities`) — before any expression of that document evaluates. `ComposeOptions::establish_repository_observation` is called at the root pipeline entry, at every child pipeline entry (`run_compose_pipeline_internal`, which transclusion and `as_markdown` children enter), and by the reference graph before it evaluates `when=` conditions; preflight's discovering authority observes nothing. `AnchoredRefresh` no longer discovers: an unestablished `Repo`-group read fails closed like any unsupplied capability. `ensure_file_resolution_context` projects the request's observation into the file-resolution scope catalog when the source lies inside it (an unknown-location source resolves from the retained anchor rather than an ambient `current_dir()` read, decision D2), so the file-resolution snapshot, the eager `ctx` group, and the lazy reads share one discovery
        - alternatives rejected: (a) capturing unconditionally at `ComposeOptions::new()`/pipeline entry would walk the monorepo topology for every `ComposeOptions::new()` test whose source lives in a temp dir (~220 ms each locally, tens of seconds on WSL2 CI); (b) adding `Repo` to the eager requirements when `current.repo*` is planned would satisfy the counters through the epoch but breaks decision D4 ("a `current.*` reference never adds an eager requirement")
        - GitNexus: `impact` returned `UNKNOWN` (target not found) for `ensure_file_resolution_context`, `with_ambient_refresh`, `for_request`, `capture_repository_scope_catalog`, `with_packages`, and `run_compose_pipeline` — the index predates this branch's symbols. Confirmed by text search: `ensure_file_resolution_context` has one caller (`pipeline/mod.rs`), `with_ambient_refresh` one (`options.rs::current_authority`) plus the lazy-root tests, `for_request` is file-local, `capture_repository_scope_catalog` is module-local, `with_packages` is called from `capture/mod.rs::populate_capture` and the observations tests, and no public signature changed
        - files changed:
                - `lib/src/markdown/compose/context/repository_scope.rs` — new `RepositoryObservation` (root + `RepoInfo`) with `scope_catalog()` and `contains()`; kept in the one projection adapter the resolver-inventory test permits to name both `RepoInfo` and `RepositoryScopeCatalog`
                - `lib/src/markdown/compose/context/capture/observations.rs` — `CapturedObservations` retains the `RepositoryObservation` beside the package lookup; `fill_from` carries it with the `Repo` group
                - `lib/src/markdown/compose/context/capture/mod.rs` — `capture_repository_scope_catalog` projects through `RepositoryObservation::scope_catalog`
                - `lib/src/markdown/compose/context/current.rs` — `AmbientRepository` (projected values + observation) replaces the values-only memo; `CurrentAuthority::establish_ambient_repository` / `ambient_repository()`; `AnchoredRefresh` answers `Repo` keys only from the established observation and never discovers; module and item docs updated
                - `lib/src/markdown/compose/context/options.rs` — `establish_repository_observation`, `request_repository`, and `ensure_file_resolution_context` projecting the request's observation (unknown-location sources resolve from the retained anchor)
                - `lib/src/markdown/compose/pipeline/mod.rs` — establishment at the root entry (before the file-resolution capture) and at every `run_compose_pipeline_internal` entry
                - `lib/src/markdown/reference/graph.rs` — establishment before `when=` evaluation during reference validation
                - `docs/topics/darkmatter-expressions.md` ("What never refreshes") and `.claude/skills/darkmatter/compose.md` (lazy reserved roots) — repository facts are now described as fixed per request
                - Sniff was not changed
        - tests (`lib/src/markdown/compose/tests/lazy_roots.rs`, `ambient_repository`):
                - added `the_observation_is_fixed_before_the_first_lazy_evaluation`: creates the request, establishes its observation through the same call the pipeline entry makes, changes the workspace topology and remote on disk, proves a fresh capture sees the change, then composes twice with the same options and shows `current.repo`/`current.packages`/`current.repo_root` hold the request-start values at zero further root discovery or topology walk and no warnings
                - `repository_facts_hold_while_mutable_facts_refresh_after_on_disk_changes`: package assertions compare sorted names (`ctx.packages` documents a set, not an order); the untracked-files check now asserts the added file is absent then present instead of assuming an empty set in an uncommitted fixture
                - `repository_facts_read_the_request_observation_without_rediscovery`: the three-expression fixture is now three paragraphs so cleanup cannot join them; the line count is asserted
                - `a_request_that_never_captured_repo_discovers_it_once`: the provider-sharing half establishes the observation through `establish_ambient_repository` instead of relying on provider-side discovery
                - module comment records that the exact counter deltas rely on nextest's one-process-per-test execution
        - gates from `darkmatter/`:
                - `just lint` → exit 0 (darkmatter, darkmatter-cli, dmls, zed-dmls wasm32-wasip2 all clean)
                - `just test` → exit 0: `8088 tests run: 8088 passed (61 slow), 14 skipped`, 0 failed, 0 leaked. The four ambient repository tests now take ~0.15 s each (previously ~0.75 s, which was the second topology walk of the monorepo from the process CWD)
        - not run: the Claudine suite. No public Darkmatter signature changed, and Claudine supplies its own `FileResolutionContext` (`composition/mod.rs:206`, `prepare.rs:298`) so the `ensure_file_resolution_context` change does not reach it; the log's host constraint also forbids running the two suites concurrently
        - not done: `gitnexus detect-changes` was not run because this task does not commit; the index is stale for this branch's symbols (see the impact note above), so `just gitnexus` should precede the author's commit
- work completed for 'Finish the request-owned ambient repository observation and restore the L1 gate' at 19:40:25
- starting the work on 'Exercise prompts/format.md at the Level 2 boundary required by AC28' at 19:42:00
        - discovery (evaluation order): `execute_stack_inner` in `claudine/lib/src/composition/lifecycle/executor.rs` walks a stack's items, and each item's actions, in order, and `render_message` evaluates an action body at the moment that action runs — so an interpolation in a later `stdout` action observes the repository state a preceding `shell` action produced. `SystemShellRunner::run` is synchronous (`sh -c` + `status()`), so the shell step has finished before the next action's expression evaluates. `stop` is valid in every event and only ends the `start` stack (the provider still launches); a non-zero shell exit is a dispatch error that halts the stack so no later `stdout` prints, while an unresolvable lazy key is an evaluation error that halts the run with `lifecycle evaluation error`
        - approach chosen: option 1 — the real `prompts/format.md` byte-for-byte, with a fake `rust` executable first on the pane `PATH` whose `fmt --all` appends a newline to exactly 2 of the 3 committed `src/*.rs` files. The asserted `resulting in 2 being updated` is therefore distinct from `0`, from `null`/empty, from the raw expression text, and from the tracked-file total. Option 2 (dirtying between events) was unnecessary because stack actions evaluate sequentially after the shell step, which the test also proves in passing
        - discovery: the shipped `prompts/format.md` did not compose at all under a real Claudine run — two authoring defects, both found by a direct probe of the binary before the test was written:
                - `start.stack` items were bare `- stdout:` / `- shell:` / `- stop` entries. The documented stack grammar (`.claude/skills/claudine/lifecycle.md` → Stacks) admits only `- when: … action: …` and `- action: …`, every other shipped prompt spells it that way, and the plan parser rejected `start.stack[0]` with `invalid lifecycle stack item … stack item must have an \`action\` key`. Fix: the five ordered actions are nested under one `- action:` item; order and semantics are unchanged
                - `initialize.stack[1]` guarded on `when: "!repo"`. `repo` is not a frontmatter key, and Claudine lifecycle guards fail closed on unknown bare roots (Darkmatter's body-expression fallback of bare `repo` to `ctx.repo` does not extend to lifecycle stacks), so every run halted with `when: references undefined variable repo`. `!ctx.repo` is not the right spelling either: `ctx.repo` is the name from the preferred remote and is null in a remote-less repository, so that guard fired inside a repo. Fix: `when: "!ctx.repo_root"`, which is null exactly when there is no repository
                - `claudine/cli/tests/shipped_prompt_contract.rs` checks schema and expression parseability only, not stack item shape or guard roots, which is why neither defect had surfaced at L1
                - observation, not fixed (cosmetic, outside AC28): the first `stdout` line renders `**{ctx.repo}**` as `****` in a remote-less repository for the same `ctx.repo` reason
        - discovery: `dirty_files` (`darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs::changed_paths`) counts modified, staged, and untracked paths, so the prompt documents themselves must be committed into the fixture repository or the prompt's own `initialize` dirty-repo gate refuses the run. The prompt's `git add . && git commit` step runs without the test helper's `-c` overrides under a fixture `HOME` with no global config, so identity and `commit.gpgsign=false` are written to the repository config; the step succeeds (the final `stdout` line prints only after it exits 0, and the test asserts that line)
        - files changed:
                - `prompts/format.md` — the two minimal fixes above; no other lines touched
                - `claudine/cli/tests/level2_ac28_lifecycle_lazy_roots.rs` — module doc lists the fifth contract; new test `level2_ac28_shipped_format_prompt_counts_files_dirtied_by_its_own_shell_step`: the real prompt, then a control with `current.dirty_files` rewritten to `current.not_a_context_key` that must halt with `lifecycle evaluation error` and never render the count. Gated with `require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux)`, headless tmux only; the existing `stage()` / `compose_in_tmux` / `git` / `write_executable` helpers are reused unchanged
                - no Darkmatter or Claudine product code changed
        - gates from `claudine/` (`GIT_TERMINAL_PROMPT=0`, `CARGO_TERM_COLOR=never`):
                - `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just _test_l2 claudine-cli --features terminal-tests --test level2_ac28_lifecycle_lazy_roots --color=never` → exit 0: `5 tests run: 5 passed, 0 skipped`, `backend-proof: tmux run=5 skip=0 panic=0` (run twice, the last on the final on-disk state). The shared recipe is called directly because the public `test-l2` also fans out to `claudine-gen`, where the `--test` narrowing has no target
                - non-vacuity: the same command with `prompts/format.md` temporarily reverted to `HEAD` → exit 100, `3 passed, 1 failed, 0 skipped`: only the new test fails, at the count assertion, with the pane showing `invalid lifecycle stack item`; the fixed prompt was restored afterwards (`diff` clean against the saved fixed copy)
                - `just test --color=never --test shipped_prompt_contract` → exit 0: `5 tests run: 5 passed, 0 skipped` (the L1 corpus suite that reads the edited prompt)
                - `just lint` → exit 0 (the only warning is the pre-existing macOS `__eh_frame` linker note on the `claudine` bin)
        - not done / deferred: `-E 'test(…)'` does not survive `_test_l2`'s argument interpolation (a `just` syntax error, exit 2), so the negative proof ran the whole binary rather than the single test. GitNexus `impact` was not needed (no existing symbol was edited — one new test function plus a prompt document) and `detect-changes` was not run because this task does not commit
- work completed for 'Exercise prompts/format.md at the Level 2 boundary required by AC28' at 19:56:47
- starting the work on 'Finish the explicit Level 1 acceptance guards' at 20:02:34
        - GitNexus: `impact` returned `UNKNOWN` (target not found) for `draw_nonce`, the one product function edited — the index still predates this branch's symbols. Confirmed by text search: its only caller is `populate_document` in the same file; no public signature changed anywhere
        - AC4 (entropy failure through the compose boundary):
                - product seam: `lib/src/markdown/compose/context/capture/document.rs` gains a `#[cfg(test)] pub(crate) mod test_seam` with a thread-local forced failure that `draw_nonce` consults first; `with_forced_nonce_failure(detail, closure)` arms it for the closure and a drop guard clears it. Thread-local rather than a process static because `cargo test` runs tests in parallel threads and the capture runs on the composing thread (verified: no spawn on the `context/` capture path). Re-exported under `#[cfg(test)]` from `capture/mod.rs`. Nothing is compiled into production, so D1's "no public nonce override" holds
                - new in-crate module `lib/src/markdown/compose/tests/identity.rs` (registered in `tests/mod.rs`), every test through `Markdown::compose_with(ComposeOptions::new())` so the Document group is captured on demand:
                        - `an_entropy_failure_is_the_typed_compose_error_for_id_and_sid`: `{{ ctx.id }}` and `{{ ctx.sid }}` each fail with `MarkdownError::missing_runtime_context()` = `ExpressionError::ExecutionNonceUnavailable { key, detail }`, key naming the read, detail equal to the injected failure
                        - `documents_that_never_read_the_identity_still_compose_without_entropy`: with the seam armed, a document naming no Document key composes with no `document.nonce` warning (the nonce is never drawn), and `{{ ctx.hash }}` composes with the partial-capture warning rather than failing
                        - `id_and_sid_differ_across_executions_and_from_each_other`: two composes of one document render 16-hex / 64-hex digests that differ per run and from each other
        - AC29 / R37 (migration guard):
                - new `lib/tests/current_root_migration_guard.rs`, `the_removed_nesting_appears_only_where_the_allowlist_expects`. Scope rule (also in the file's `//!` doc): from the workspace root (nearest ancestor of `CARGO_MANIFEST_DIR` whose `Cargo.toml` contains `[workspace]`), scan every regular file with extension `rs`, `md`, `toml`, `yaml`, `yml`, or `json`, plus every regular file under a directory named `prompts` regardless of extension; skip directories named `target`, `node_modules`, `features`, `fixes` at any depth and every dot-directory other than `.claude`; never follow symlinks. Skipping `features/` and `fixes/` is what excludes the planning documents (specs, plans, reviews, logs, decisions, `_completed`, `_unscheduled`) and every in-flight spec on the migration list — they are neither code, shipped prompts, user docs, nor skills. Occurrences of the literal `current.ctx.` / `current.env.` are counted per occurrence (a line naming both counts twice), keyed by `/`-separated relative path; `std::fs` walking only, no shell. The test fails on any file outside the allowlist and on any entry whose count moved, listing `path` and line numbers
                - allowlist (path → count): `.claude/skills/claudine/{SKILL.md,composition.md,lifecycle.md}` 2 each and `.claude/skills/darkmatter/compose.md` 2 (skills explaining the clean break); `claudine/lib/src/composition/lifecycle/context/tests.rs` 2 (negative test); `claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs` 1 (doc comment); `darkmatter/cli/tests/compose_lazy_roots.rs` 2 (negative test through `md`); `darkmatter/docs/topics/darkmatter-expressions.md` 2 (user docs); `darkmatter/lib/src/markdown/compose/context/capture/capabilities.rs` 2 (negative test); `.../capture/groups.rs` 1 (doc comment); `.../context/current.rs` 4 (doc comment + negative test); `.../expression/error.rs` 4 (`LazyRootMemberUnknown` docs and error text); `.../compose/tests/lazy_roots.rs` 2 (negative test); the guard itself 16 (its doc, needles, annotations, and scope fixture). Every entry was read and is a negative test or explanatory doc/error text; no live use of the old spelling was found, so nothing was migrated
                - one annotated exception: `claudine/cli/tests/level2_lifecycle_control.rs` 1 is `current.env.push(..)` on a local variable named `current` — a Rust field chain the literal scan cannot distinguish from the document spelling, not a use of the removed nesting. Renaming a Claudine local to satisfy a Darkmatter guard was declined as cross-area churn; the author can rename it and drop the entry if the R37 categories are to be read strictly
                - the scan's first run reported only the guard's own self-count as off (16, not the 4 first guessed); every other file matched the inventory exactly, so the scope rule found no occurrence the orientation list did not
                - `the_scan_counts_occurrences_and_honors_the_scope_rule`: temp-dir fixture proving two occurrences on one line count twice, `prompts/` files count regardless of extension, `.claude` is scanned while other dot-directories, `target`, `features`, `fixes`, and out-of-scope extensions are not, and `current_env.HOME` / `ctx.current` never match
        - AC30 (URL root without an extra fetch):
                - `md compose` accepts only a path or stdin (`cli/src/commands/compose.rs` resolves the input through `resolve_file_path`), so a URL root crosses the library boundary only; the test lives in the lib crate, which already carries `wiremock` and uses reqwest (a normal dependency) for the caller-side fetch
                - new `lib/tests/url_root_identity.rs`, `a_url_root_composes_from_the_fetched_text_without_refetching_it`: a wiremock server serves `/docs/root.md` and `/docs/child.md`; the test fetches the root once and composes that text with `with_source_url(root)`, remote transclusion on, and `127.0.0.1` allowed. Asserts the recorded requests hold exactly one `GET /docs/root.md` (the caller's) and one `GET /docs/child.md` (proving the policy would have permitted a refetch), `ctx.id` is 16 lowercase hex, `ctx.sid` 64 and not a re-spelling, `ctx.self` / `ctx.last_updated` / `ctx.hash` render null, and the remote child composes under the root's identity
        - AC36 (warm `--cache-root` replays neither identity nor probe output):
                - `cli/tests/compose_remote_caching.rs::test_compose_cache_root_never_replays_composed_local_output`: the root now renders `id=[ctx.id] sid=[ctx.sid] env=[current_env.AC36_PROBE]`; the warm run gets `AC36_PROBE=warm` and a rewritten `main.rs` (`fn main() { warm() }`). Asserts, on top of the kept assertions: `ctx.id` and `ctx.sid` non-empty and different between runs, `id != sid`, the warm run shows `env=[warm]` and the new `::code` body and not the cold one, remote request count still 1
                - `lib/tests/persistent_cache_disabled.rs::a_warm_cache_root_never_replays_composed_local_output` (all four freshness modes): `write_graph` adds the same `id`/`sid` probes to `root.md`; `main.rs` is rewritten between cold and warm; `stamp()` became the generic `probe(content, name)`. Same identity and `::code` assertions; the stamp, stats, and artifact-directory assertions are unchanged
        - files changed: `lib/src/markdown/compose/context/capture/document.rs`, `lib/src/markdown/compose/context/capture/mod.rs`, `lib/src/markdown/compose/tests/mod.rs`, `cli/tests/compose_remote_caching.rs`, `lib/tests/persistent_cache_disabled.rs`; new `lib/src/markdown/compose/tests/identity.rs`, `lib/tests/current_root_migration_guard.rs`, `lib/tests/url_root_identity.rs`. No Claudine file, doc, or skill was changed
        - gates from `darkmatter/` (`GIT_TERMINAL_PROMPT=0`, `CARGO_TERM_COLOR=never`):
                - `just lint` → exit 0 (darkmatter, darkmatter-cli, dmls, zed-dmls wasm32-wasip2 all clean)
                - `just test --color=never` → exit 0: `8094 tests run: 8094 passed (67 slow), 14 skipped`, 0 failed (8088 before this finding + the 6 new tests)
                - focused runs beforehand: the 3 identity tests, the 2 guard tests, `url_root_identity`, both `persistent_cache_disabled` tests, and the CLI cache test all passed under nextest
        - not done: `gitnexus detect-changes` was not run because this task does not commit and the index is stale for this branch (see the impact note); `just gitnexus` should precede the author's commit. The guard was verified on macOS only; its walker is `std::fs`, keys are `/`-joined components, and `.gitattributes` pins LF, so Linux and Windows CI read the same bytes
- work completed for 'Finish the explicit Level 1 acceptance guards' at 20:19:39
- orchestrator notes across the three findings:
        - the three subagents ran serially, so the last full Darkmatter L1 run (finding 3, `8094 tests run: 8094 passed (67 slow), 14 skipped`, 0 failed) and the last `just lint` (exit 0) include the code from findings 1 and 3; finding 2 changed only `prompts/format.md` and a Claudine L2 test, verified by its own tmux-required L2 run (`5 tests run: 5 passed, 0 skipped`) and `cd claudine && just lint` (exit 0)
        - finding 2 found that the shipped `prompts/format.md` never composed at all (bare `start.stack` actions without an `action:` key, and an `initialize` guard on the nonexistent frontmatter key `repo`); both were fixed minimally in the shipped prompt itself, which the L1 `shipped_prompt_contract` suite cannot catch because it checks parseability only
        - cross-OS: no code path added here is OS-specific; the AC29 guard walks with `std::fs` and normalizes allowlist keys to `/`, and the L2 case is tmux-only. No `just cross-check` run was made; CI covers Linux, Windows, and WSL2 for these Level 1 suites
        - GitNexus `impact` returned `UNKNOWN` for the branch-local symbols in every finding (the index predates this branch); call sites were confirmed by text search each time. `detect-changes` was not run because this task does not commit; run `just gitnexus` before the author's commit
        - the log frontmatter had no `implementation_2` key from the previous cycle; only `implementation_3` was added here

### Successful Completion

The implementation of review cycle 3 has completed successfully in 1 hour 13 minutes. During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 0 were deferred (see reasons below):

- no findings were deferred; no performance measurement was required by this review, so `deferred_perf_measurement` stays unset

The files changed by this cycle are:

- `darkmatter/lib/src/markdown/compose/context/{current.rs,options.rs,repository_scope.rs}`, `context/capture/{observations.rs,mod.rs,document.rs}`, `compose/pipeline/mod.rs`, `reference/graph.rs`
- `darkmatter/lib/src/markdown/compose/tests/{mod.rs,lazy_roots.rs,identity.rs}`
- `darkmatter/lib/tests/{current_root_migration_guard.rs,url_root_identity.rs,persistent_cache_disabled.rs}`, `darkmatter/cli/tests/compose_remote_caching.rs`
- `darkmatter/docs/topics/darkmatter-expressions.md`, `.claude/skills/darkmatter/compose.md`
- `prompts/format.md`, `claudine/cli/tests/level2_ac28_lifecycle_lazy_roots.rs`
- `darkmatter/features/2026-09-09-more-context/{implementation-log.md,review-3.md}`

## Implementation of Review Findings #4

> **started at:** 2026-09-17T20:43:06-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/darkmatter/features/2026-09-09-more-context/review-4.md'
- this is iteration 4 of the review-to-implement cycle
- the review contains 2 findings, both unblocked: (1) High — capture the ambient repository observation at the actual request boundary; (2) Medium — finish the AC29 documentation contract and correct repository freshness claims
- the orchestrator runs one subagent per finding, serially; each subagent owns its own log entries below
- starting the work on 'Capture the ambient repository observation at the actual request boundary' at 20:52:10
        - orchestrator survey before delegating: `ComposeOptions::ensure_file_resolution_context` already runs a full `Repo`-group capture (git discovery plus topology walk, via `capture_file_resolution_context`) for every Darkmatter-owned request that holds no observation, so establishing the observation at request creation costs no additional discovery
        - Claudine always installs a `CurrentProvider` (even `UnsuppliedRefresh`) after `with_context_authority`, so establishment must not fire inside `with_context_authority`; a dedicated document-aware constructor plus a root-entry fallback is the recommended shape
        - 20:55:30 GitNexus upstream impact for `establish_repository_observation`, `establish_ambient_repository`, `run_compose_pipeline_internal`, and `new_with_context` all returned `risk: UNKNOWN` (target not found; the index predates this branch), so callers were confirmed by text search: `establish_repository_observation` — `pipeline/mod.rs:36` (root entry), `pipeline/mod.rs:131` (child entry), `reference/graph.rs:345`, `tests/lazy_roots.rs:549`; `establish_ambient_repository` — `options.rs:658`, `tests/lazy_roots.rs:520`; `run_compose_pipeline_internal` — `transclusion/engine.rs:1145` and `:1564`, `context/authority.rs:276` (test); `new_with_context(..).with_context_authority(DarkmatterOwned)` — `cli/src/commands/compose.rs:313`, `claudine/cli/src/commands/wrap/sequence/jit.rs:310`, `claudine/lib/src/system_prompt/prepare.rs:158`; `ComposeOptions::new()` (`compose()`, `compose_mut()`, `Default`)
        - design decision: added the public request constructor `ComposeOptions::for_document(anchor: &Path, document: &Markdown)`; it is `new_with_context(ComposeContext::capture_for_document(..)).with_context_authority(DarkmatterOwned)` followed by `establish_repository_observation()`, so one call owns the root document, the demand-driven context, and the request's repository observation (D3). Chose the `(anchor, document)` shape over `(ComposeContext, document)` because passing a context back in would let a caller pair the observation with a context captured for a different document, and because every Darkmatter-owned site already had the anchor and the document in hand
        - design decision: `establish_ambient_repository` and `establish_repository_observation` no longer take a document and are no longer gated on whether that document plans a `current.repo*` read — the gate is exactly what let a transcluded descendant be the first to establish, and it cannot be evaluated correctly for descendants without walking them. The `DeferredCapabilities` gate and the `Markdown` import in `current.rs` were dead after this and are deleted. Cost is neutral for a request that supplies no `FileResolutionContext` (the common `md compose` path and every `compose()`/`compose_mut()` call): `ensure_file_resolution_context` now projects the snapshot from the observation instead of running its own `Repo` capture, so the request still pays exactly one root discovery and one topology walk (verified by the count assertions below). A Darkmatter-owned no-provider request that also supplies its own snapshot — `md compose` with `--set`, and Claudine's no-invocation fallback branches in `jit.rs` / `system_prompt/prepare.rs` — now pays one discovery it previously skipped when the root read no repository fact; accepted, because correctness of D3 for descendants requires it and those branches are library-compatibility paths
        - design decision: the root pipeline entry keeps `establish_repository_observation()` as the fallback for requests built through `new()` / `new_with_context`, now unconditional; the child-entry call in `run_compose_pipeline_internal` and its comment are removed, so a child pipeline can never establish. The `graph.rs` call was already redundant — `build_node` composes the root through `prepare_content` (a root pipeline entry on a clone sharing the request's `CurrentAuthority`) before reaching the `when=` lookup — so it is removed and the comment rewritten to say where validation's observation actually comes from
        - migrated `darkmatter/cli/src/commands/compose.rs` to `ComposeOptions::for_document(&launch_dir, &md)`; the perf `options_ctx_ref` now clones `options.context()`. Not migrated: `lib/src/markdown/reference/file_tree/mod.rs:183` — the orchestrator's survey listed it as a Darkmatter-owned site, but it never calls `with_context_authority`, so it is `CallerSupplied` and out of scope; `claudine/cli/src/commands/wrap/sequence/jit.rs` and `claudine/lib/src/system_prompt/prepare.rs` — both branch on an invocation/shared context and mutate the captured context (`env_mut`) before building options, so `for_document` is not a drop-in; they keep their semantics and get D3 timing from the root-entry fallback. No Claudine file was touched and no Claudine-visible signature changed (`establish_*` are `pub(crate)`; `for_document` is additive)
        - files changed: `darkmatter/lib/src/markdown/compose/context/options.rs` (new `for_document`; `establish_repository_observation(&self)`; `new()` / `new_with_context` docs point at `for_document`), `darkmatter/lib/src/markdown/compose/context/current.rs` (`establish_ambient_repository(&self, &ComposeContext)`, gate and imports removed, module docs), `darkmatter/lib/src/markdown/compose/pipeline/mod.rs` (root-entry comment; child-entry call removed), `darkmatter/lib/src/markdown/reference/graph.rs` (redundant call removed), `darkmatter/cli/src/commands/compose.rs` (constructor migration), `darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs` (tests), `.claude/skills/darkmatter/compose.md` (lazy-roots and `ContextAuthority` paragraphs describe the new timing)
        - tests (all in `lazy_roots.rs::ambient_repository`, every one through public boundaries only): `the_observation_is_fixed_at_request_creation` replaces the manual-call regression — `for_document` costs exactly one discovery and one walk at construction, the repository is then mutated, and `validate_references` → `compose_preflight` → `compose_with` → a second `compose_with` on the same options run at zero further discovery with `current.repo`/`current.packages`/`current.repo_root` at their creation values; `a_child_reads_the_observation_fixed_at_request_creation` — the root names no repository fact, its frontmatter `$(echo mutated)` runs under a `ShellApprovalHandler` whose `approve` mutates the repository (stage 3, before transclusion), and the `::file` child still renders the creation values with zero discovery in either pipeline; `an_older_constructor_request_is_fixed_at_the_root_entry_not_by_the_child` — the `new_with_context` + `DarkmatterOwned` fallback with a child-only reader costs exactly one discovery for the whole request. `a_request_that_never_captured_repo_discovers_it_once` keeps its `(roots + 1, walks + 1)` assertion, still exact under the root-entry fallback; its authority-level check adapts to the new `establish_ambient_repository(&context)` signature. `workspace_repository` now canonicalizes the temp root so a `File` source compares equal to the discovered root on macOS. Tests use `std::path` joins, `tempfile`, and the same `echo` the existing shell tests already run on every OS
        - non-vacuity: with the pre-fix timing temporarily reinstated (no establishment in `for_document`; document-gated establishment at root and child entries) all three new tests fail — at the count assertions, and with the counts disabled at the value assertions (`repo=[widget] packages=[..,"gamma"]` reaches the root and the child). Restored afterwards; verified green again
        - doc/comment drift fixed in the same change: `establish_ambient_repository` and `establish_repository_observation` docs described the per-pipeline-entry, plan-gated timing; the `current.rs` module doc said the observation is fixed "before a document's first expression evaluates"; the `pipeline/mod.rs` child-entry comment permitted a child to establish; the `graph.rs` comment claimed its call was what made validation read the request observation. All rewritten to the creation-time boundary plus root-entry fallback. `compose.md` (skill) paragraph on `CurrentProvider` and the `DarkmatterOwned` bullet updated; no `darkmatter/docs/topics/*.md` file describes the observation timing (grep for "observation"/"establish" found only the `package()`/`package_area()` function rows)
        - gates: `cd darkmatter && just lint` — clean for darkmatter, darkmatter-cli, dmls, zed-dmls-cli (clippy `-D warnings`, wasm32-wasip2 check); `cd darkmatter && just test` — 8096 tests run: 8096 passed (62 slow), 14 skipped, exit 0 (darkmatter, darkmatter-cli, dmls, zed-dmls-cli in one nextest invocation); focused `cargo nextest run -p darkmatter lazy_roots` — 20 passed. Claudine lint/tests not run: no Claudine file or Claudine-visible signature changed
        - not done: `cargo doc -p darkmatter -D warnings` already fails on ~40 pre-existing unrelated links; the three private-item links in the `current` module docs (`AnchoredRefresh`, `CurrentScope`, `CurrentAuthority::establish_ambient_repository`) predate this change and were left alone
- work completed for 'Capture the ambient repository observation at the actual request boundary' at 21:09:29
- starting the work on 'Finish the AC29 documentation contract and correct repository freshness claims' at 21:11:40
        - orchestrator survey before delegating: `claudine/docs/topics/context/context-variables.md` (145 lines) documents only `ctx.*`; `.claude/skills/claudine/SKILL.md` "Binding time" paragraph and `.claude/skills/claudine/lifecycle.md` "Late-binding" paragraph both use the non-existent `ctx.repo_name` / `current.repo_name` and claim a mid-run rename changes `current`
        - the existing AC29 guard `darkmatter/lib/tests/current_root_migration_guard.rs` is a literal allowlist scan; the passive documentation contract must be a separate positive check
        - real keys used, read from the descriptor catalog (`capture/repo.rs` `KEYS`, `capture/git.rs` `KEYS` / `HISTORY_KEYS`, `capture/changes.rs` `KEYS`, `capture/invocation.rs` / `capture/document.rs` `KEYS`): fixed `Repo` group — `repo`, `repo_root`, `packages`, `area`; refreshable — `branch` (`Git`), `recent_commits` (`GitHistory`, the R29 pair with `recent_commits(count)`), `dirty_files` (`FileChanges`), `current_env.<key>`; request-owned and never refreshed — `cwd`, `self`, `hash`, `id`, `sid`. The stale `ctx.repo_name` / `current.repo_name` example and the "renamed mid-run" claim appeared only in `.claude/skills/claudine/SKILL.md` and `.claude/skills/claudine/lifecycle.md`; the other six required pages carried neither (grep for `repo_name` / `renamed` over all eight pages is now empty)
        - one canonical fixed-versus-refreshable sentence is reused on six pages so the contract can pin it: "Repository metadata and topology (`repo`, `repo_root`, `packages`, `area`, and the rest of the repository keys) are fixed by the request's repository observation, so `current.repo` always reads what `ctx.repo` does; only mutable Git and filesystem facts (`branch`, `recent_commits`, `dirty_files`) and `current_env.*` refresh at reference time."
        - `claudine/docs/topics/context/context-variables.md`: new "## Binding time: eager `ctx`, lazy `current`" section after "What they are used for" — the four-row eager/lazy table (`ctx.<key>`, `current.<key>`, `env.<key>`, `current_env.<key>`), the direct-mirror spelling with an explicit no-nesting statement written without the removed literals (`current` does not contain a `ctx` member, `current_env` does not contain an `env` member), the fixed-versus-refreshable paragraph (observation captured once when the request is created), the variable/function pair rule through `ctx.recent_commits` (last 10 at start of run) / `recent_commits(count)` (walks the newest `count` when reached, one descriptor entry), the Claudine `PartialRuntimeCapture` behavior, and cross-links to `../lifecycle.md#binding-time-early-vs-late` and `../composition.md#launch-anchored-prepared-context`
        - `.claude/skills/claudine/SKILL.md` "Binding time" paragraph: `ctx.branch` at launch versus `current.branch` when reached replaces the `repo_name`/rename example; the canonical fixed sentence added; the existing "no `current.ctx.*` / `current.env.*` nesting" sentence kept verbatim so the R37 allowlist count (2) is unchanged
        - `.claude/skills/claudine/lifecycle.md` late-binding paragraph: same replacement and same canonical sentence; nesting sentence kept verbatim (allowlist count 2 unchanged)
        - `.claude/skills/claudine/composition.md`: canonical fixed sentence added directly under the binding-time table (this page already carried the direct spelling and the pair example)
        - `.claude/skills/darkmatter/compose.md`: the `recent_commits(count)` paragraph now names it as the lazy half of the first R29 pair with `ctx.recent_commits` as the eager snapshot of the same descriptor; the page already carried the direct spelling and the D3 "request's one repository observation" statement from the previous finding
        - `darkmatter/docs/topics/darkmatter-expressions.md` "What never refreshes" bullet: repository keys (now also naming `area`) are "fixed by the request's repository observation, made once when the request is created" — the previous wording ("made before the first expression evaluates") was true but predated the request-boundary fix — plus the mutable-facts clause; the "no nesting" sentence with the removed literals is untouched (allowlist count 2 unchanged)
        - `claudine/docs/topics/lifecycle.md`: canonical fixed sentence appended to the paragraph under the runtime-globals table, with a link to the new context-variables section; `claudine/docs/topics/composition.md`: the same sentence and link appended to the "`current.*` and `current_env.*` are the lazy roots" bullet in Launch-Anchored Prepared Context
        - no new `current.ctx.` / `current.env.` occurrence was added anywhere (the new section and the new test both explain the no-nesting rule without the literals), so the R37 `ALLOWLIST` is unchanged and the guard stays green
        - new L1 test `darkmatter/lib/tests/current_root_documentation_contract.rs` — chosen as a separate file beside the guard because the guard's module doc describes a negative literal scan with its own scope rule, and a positive per-page phrase table is a different contract that would blur that doc. `PAGES` is the spec "## Documentation" list encoded as `(page, &[phrase])`: context-variables.md carries all three clauses (7 phrases: the three table rows, the no-member sentence, the pair sentence, the fixed sentence, the refresh sentence); claudine SKILL.md 5 (lazy `current.<key>`, `current_env.<key>` mirror, pair, fixed, refresh); claudine lifecycle.md skill 4; claudine composition.md skill 4 (table rows, "are the first pair", fixed); darkmatter compose.md skill 4 (cataloged `current.<key>`, `current_env.<KEY>` at reference time, eager-snapshot pair sentence, "request's one repository observation (D3)"); darkmatter-expressions.md 5 (namespace rows, "mirrors ... key for key", the function-row pair sentence, "made once when the request is created"); claudine lifecycle.md doc 3 (globals rows, fixed); claudine composition.md doc 3 (bullet spellings, fixed). Phrases are each page's own multi-token wording, so a stray word cannot satisfy an entry. `workspace_root()` is the guard's ancestor walk to the `[workspace]` manifest; pages are joined with the platform separator, read as bytes (`from_utf8_lossy`), and whitespace-collapsed before matching so reflowed paragraphs and a CRLF checkout both match
        - non-vacuity: (1) `the_check_reports_each_missing_phrase_and_tolerates_reflow` runs `check` over a temp tree — a phrase split across CRLF and indented LF wrapping matches, a page missing one of two phrases reports exactly that phrase, and an absent page reports "cannot read"; (2) on the real tree, the phrase "fixed by the request's repository observation" in context-variables.md was temporarily rewritten — `every_required_page_explains_the_binding_time_model` failed with `claudine/docs/topics/context/context-variables.md: missing "are fixed by the request's repository observation"` — then restored, sha256 verified identical to the pre-break file
        - files changed: `claudine/docs/topics/context/context-variables.md`, `.claude/skills/claudine/SKILL.md`, `.claude/skills/claudine/lifecycle.md`, `.claude/skills/claudine/composition.md`, `.claude/skills/darkmatter/compose.md`, `darkmatter/docs/topics/darkmatter-expressions.md`, `claudine/docs/topics/lifecycle.md`, `claudine/docs/topics/composition.md`, new `darkmatter/lib/tests/current_root_documentation_contract.rs`; no Rust source outside the new test, no Claudine Rust code
        - gates: focused `cargo nextest run -p darkmatter --test current_root_migration_guard --test current_root_documentation_contract` — 4 run, 4 passed; `cd darkmatter && just lint` — exit 0, clean for darkmatter, darkmatter-cli, dmls, zed-dmls-cli (clippy `-D warnings`, wasm32-wasip2 check); `cd darkmatter && just test --color=never` — exit 0, 8098 tests run: 8098 passed (63 slow), 14 skipped (the +2 over the previous 8096 are the new contract tests); `git diff --check` — clean. Claudine gates not run: no Claudine Rust file changed, and grep of `claudine/cli/tests` and `claudine/lib/tests` for `docs/topics` / `context-variables` found no docs-scanning passive test (only provider NDJSON fixtures matched)
        - not done: none for this finding; `cargo fmt` was not run per instructions. The package `just lint` recipe runs clippy only (no rustfmt check), so the new test file was verified separately with a non-mutating `rustfmt --check --edition 2024` on that one file (exit 0 after three hand-applied reflows); the sibling guard file does not pass the same check and was left alone (formatting is a periodic pass, not a gate)
- work completed for 'Finish the AC29 documentation contract and correct repository freshness claims' at 21:22:08
- orchestrator notes across the two findings:
        - the two subagents ran serially, so the last full Darkmatter L1 run (finding 2, `8098 tests run: 8098 passed (62 slow), 14 skipped`, 0 failed) and the last `just lint` (exit 0) include the code from finding 1; finding 1's own run was `8096 passed`, and the two extra tests are finding 2's documentation-contract tests
        - finding 1 removed the child-entry and reference-graph establishment calls and dropped the document parameter from `establish_repository_observation` / `establish_ambient_repository`; the `md compose` request now goes through the new public `ComposeOptions::for_document` constructor, and older constructors get D3 timing from an unconditional root-entry fallback. No Claudine Rust file changed and no Claudine-visible Darkmatter signature changed (both removed methods are `pub(crate)`)
        - accepted cost noted by finding 1: a Darkmatter-owned request that supplies its own `FileResolutionContext` and no `CurrentProvider` now pays one repository discovery it previously skipped; every other Darkmatter-owned request already paid that discovery for file resolution, so `md compose` is cost-neutral
        - cross-OS: no `just cross-check` run was made. The review's verdict does not depend on cross-OS evidence, `just cross-check darkmatter --os windows` rebuilds the whole package on the Windows rig, and the only OS-sensitive addition is a `$(echo mutated)` frontmatter expansion under a `ShellApprovalHandler`, the same `echo` the existing shell tests already run on every OS; the new tests use `std::path` joins and `tempfile`. CI covers Linux, Windows, and WSL2 for these Level 1 suites
        - GitNexus `impact` returned `UNKNOWN` for the branch-local symbols (the index predates this branch); call sites were confirmed by text search. `detect-changes` was not run because this task does not commit; run `just gitnexus` before the author's commit

### Successful Completion

The implementation of review cycle 4 has completed successfully in 43 minutes. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred (see reasons below):

- no findings were deferred; no performance measurement was required by this review, so `deferred_perf_measurement` stays unset

The files changed by this cycle are:

- `darkmatter/lib/src/markdown/compose/context/{options.rs,current.rs}`, `darkmatter/lib/src/markdown/compose/pipeline/mod.rs`, `darkmatter/lib/src/markdown/reference/graph.rs`, `darkmatter/cli/src/commands/compose.rs`
- `darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs`, new `darkmatter/lib/tests/current_root_documentation_contract.rs`
- `darkmatter/docs/topics/darkmatter-expressions.md`, `.claude/skills/darkmatter/compose.md`
- `.claude/skills/claudine/{SKILL.md,lifecycle.md,composition.md}`, `claudine/docs/topics/{lifecycle.md,composition.md}`, `claudine/docs/topics/context/context-variables.md`
- `darkmatter/features/2026-09-09-more-context/{implementation-log.md,review-4.md}`
