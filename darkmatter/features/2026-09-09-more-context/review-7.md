---
$schema: feature-review.yaml
ready: false
findings:
  - title: Complete the capture-group wiring so the implementation compiles
    priority: critical
  - title: Replace all thirteen pending expression handlers with implementations
    priority: high
  - title: Implement reserved lazy globals and migrate Claudine lifecycle lookup
    priority: high
  - title: Add the missing end-to-end and required Level-2 acceptance coverage
    priority: high
human_review: true
human_review_items:
  - |-
    Confirm the execution-identifier choice already adopted in the specification: include a fresh random value in each execution's identifiers, so two otherwise identical runs differ. Alternatives are repeatable identifiers that may collide between runs, or secret-key identifiers that also require key management. The adopted random-value option is recommended.
  - |-
    Confirm when live context values should refresh. The adopted choice keeps repeated reads of one value consistent within one expression, then refreshes it in the next expression. Alternatives are keeping the first value throughout the document or refreshing on every read. The adopted per-expression choice is recommended.
  - |-
    Confirm the persistent-cache scope: the adopted behavior saves only raw downloaded responses and never saves composed documents. The alternative is to disable persistence of downloaded responses too until a separate freshness policy is designed. This confirmation is separate from completing and testing the implementation.
reviewed_by: codex/default
created: "2026-09-17T01:02:22-07:00"
spec: 2026-09-09-more-context/spec.md
implemented: false
description: "A **fix** review of `2026-09-09-more-context/spec.md`"
fix: 2026-09-09-more-context/review-7.md
previous: 2026-09-09-more-context/review-6.md
---

# Review 7 — More Context

**Not production ready.** The reviewed working tree contains useful Sniff and descriptor groundwork, but the feature is only partially implemented. This verdict is based on source defects and missing behavior/coverage, not missing cross-OS results or pending human confirmation.

## Previous review closure

Review 6 was unavailable. Neither `@prompts/_reviews/darkmatter/features/2026-09-09-more-context/review-6.md` nor `@darkmatter/features/2026-09-09-more-context/review-6.md` resolved through `bf reference` (the FileReference-backed CLI). An ignored-file-inclusive search and tracked-file inventory also found no review for this feature. Therefore neither its **Unblocked Findings** nor its **Blocked Findings** could be checked. No claim that they were implemented or unblocked is warranted, and no replacement review 6 was fabricated. Its requested `next` and `implemented` updates remain unperformed because the file is missing.

The available `decisions.md` records Q1–Q3 recommendations as adopted on September 16, pending confirmation. That permits reviewing the selected contracts, but does not establish closure of an unavailable previous review. The implementation log explicitly records Phases 6–10 and most of Phase 11 as halted. The spec's existing `implemented: true` is inconsistent with those records and the code; this review changes only the requested `review_iterations` field.

## Unblocked Findings

### Critical — Complete the capture-group wiring so the implementation compiles

`darkmatter/lib/src/markdown/compose/context/capture/groups.rs:9–34` still defines the original eleven `ContextGroup` variants. It does not define `Document`, `Network`, or `GitHistory`. Nevertheless, the working-tree implementation references those variants in `capture/mod.rs:201–213`, `capture/snapshot.rs:303–304,769`, `context/runtime.rs:330,540`, and `claudine/lib/src/invocation_context.rs:1071–1094`. These unconditional Rust references cannot type-check against the current enum. This is a source-level defect, distinct from the environment error encountered while attempting tests.

Finish the group/key ownership, requirements planning, projection, and Claudine evidence wiring together. Preserve the spec's public Git-group contract with a separate recent-history demand flag rather than casually exposing a new public history group. Verify that every descriptor key has an owning capture path, that unrelated Git reads do not load history, and that supplied evidence stays fail-closed. Run the affected Darkmatter and Claudine builds/tests after the wiring is complete.

### High — Replace all thirteen pending expression handlers with implementations

`darkmatter/lib/src/markdown/compose/expression/functions/pending.rs:17–55` registers `package_area`, `package`, `recent_commits`, `ipv4`, `ipv6`, `has_alias`, `has_builtin_function`, `has_user_function`, `can_execute`, `has_agentic_cli`, `as_markdown`, `ping`, and `ping_under` with handlers that always return “is declared but not implemented yet.” These are reachable registry bindings, not unused sketches. Only the new `has_binary` alias has a real implementation in `functions/paths.rs`.

Consequently the repository lookup, call-time history, address filtering, shell probes, agent detection, nested composition, and ICMP permission/preflight contracts are absent. Descriptor listings and the generated provider roster do not supply runtime behavior. The existing pending-binding test intentionally verifies failure; a passing result would not satisfy any positive acceptance criterion for these functions.

Implement the owning handlers and request-scoped dependencies, including shared recursion/consent for `as_markdown` and typed ICMP grants isolated from HTTP permissions. Replace the placeholder expectations with positive and negative behavioral tests covering the acceptance matrix below.

### High — Implement reserved lazy globals and migrate Claudine lifecycle lookup

`expression/catalog/roots.rs` declares `current` and `current_env` as metadata and explicitly says evaluator reservation is future Phase 6 work. `subtree.rs:262–270` still recognizes only `ctx`, `env`, and `doc` as built-in roots. Claudine's `LifecycleCurrent::capture_at_event` still captures the full ambient context; `to_value` still returns nested `ctx` and `env` objects (`claudine/lib/src/composition/lifecycle/context.rs:497–511`). `reserved.rs:19` omits `current_env` from `LATE_BINDING_ROOTS`.

Thus the required direct spelling, all-surface availability, per-expression/per-key freshness, non-shadowing, invocation-owned refresh authority, and lifecycle shell preflight rejection are not implemented. Existing tests still use `current.ctx.today`; user documentation still teaches the old shape (`claudine/docs/topics/lifecycle.md:437`, `composition.md:813,884`).

Implement the shared provider/evaluation scope first, then migrate lifecycle lookup, validation, preflight, shipped prompts, tests, and docs together. Include a controlled state change between expressions and an intra-expression repeated-key check; do not simulate parent environment mutation with a child shell. Remove ambient recapture and old nested aliases as required by R33.

### High — Add the missing end-to-end and required Level-2 acceptance coverage

The new document/network projection tests are in-process tests. The catalog tests prove names and signatures; the pending-handler test proves deliberate failure. None substitutes for successful composition through the real entry points. In particular, the new document identity lacks the required CLI hash comparison and nested compose contract coverage, and the current implementation cannot exercise the new lazy globals at all.

AC2 requires a real Claudine compose in linked and main worktrees; the existing `linked_worktrees_keep_distinct_repository_keys` test proves repository keys, not the composed `ctx.worktree` value. AC28 requires real lifecycle execution of the new roots across events and migrated shipped prompts. No matching new L2 acceptance coverage was found. These are explicit verification-level gaps even after compilation and handler implementations are repaired.

Add the L1 pipeline/CLI contracts and the specified L2 Claudine cases, using fixture-owned repositories, environments, and profiles. Preserve terminal/browser focus. Do not count existing tests of the old lifecycle shape as coverage for the replacement.

## Blocked Findings

No implementation finding above requires a new design decision before work can continue against the adopted specification. Human confirmation of Q1–Q3 is tracked separately in frontmatter and does not determine `ready`. The unavailable review 6 prevents historical closure auditing; the test environment prevented executable verification during this review.

## Requirement verification levels

“Present” below means test source was inspected, not that it passed in this review. All acceptance criteria are mapped; related requirements share rows where they require the same boundary.

| Requirement | Appropriate verification | Current evidence and gap |
| --- | --- | --- |
| AC1, AC19, AC26: catalogs, generated roster, single-source pair | L1 passive corpus, generator drift, spawned listing CLI | Descriptor/generated artifacts and listing tests are present. They do not verify execution. No passing run obtained. |
| AC2: composed worktree identity | Spec-required L2 real Claudine run | Existing L1 worktree identity tests are insufficient; required composed-output case missing. Finding 4. |
| AC3–5: hash, IDs, canonical source, transcluded identity | L1 fixed-vector/unit tests plus real CLI and pipeline fixtures | Untracked `capture/document.rs` has vector, nonce, and retained-source tests; capture does not compile and CLI/transclusion acceptance coverage is missing. Findings 1 and 4. |
| AC6–8, AC33: ping consent, outcomes, budgets, preflight, HTTP isolation | L1 deterministic transport/policy fixtures through compose | Sniff transport unit tests exist; Darkmatter handlers are placeholders. Finding 2. |
| AC9, AC13: tailnet and both gateway families | L1 interface/route fixtures and projection | Sniff gateway fixture suites and new `capture/network.rs` projection tests exist; group integration is incomplete. Finding 1. |
| AC10–12, AC34: shell categories, safe bounded probes, binary alias | L1 controlled shell/process fixtures | `has_binary` delegates to `has_command`; the other probes are placeholders. Alias-resolution tests cannot prove the new classification and cleanup contracts. Finding 2. |
| AC14: actual IPv4/IPv6 ICMP round trips | Real network-stack resource tests, not terminal rendering or keyboard injection | `sniff/lib/tests/network_primitives.rs` contains both real loopback cases, selected by the real-resource tier. Appropriate boundary; cross-OS execution evidence is left to CI. |
| AC15–16: nested full compose and recursion | L1 pipeline fixtures | `as_markdown` is a placeholder. Shared depth, base directory, identity, and consent are unverified. Finding 2. |
| AC17, AC35: repository lookup, lexical descendants, typed path errors | L1 captured-topology/path fixtures | Lookup functions are placeholders. Finding 2. |
| AC18: agent aliases, unknown-name rejection | L1 controlled PATH and dispatch tests | Generated mapping exists; handler is a placeholder. Finding 2. |
| AC20: address filtering | L1 fake interfaces through expression dispatch | Sniff primitives exist; `ipv4`/`ipv6` are placeholders. Finding 2. |
| AC21–23: scope strings, sentinel removal, conditional truthiness | L1 Sniff and both compose entry points | Sniff groundwork is present, but the changed Darkmatter/Claudine capture cannot compile; full five-position compose parity is not established. Findings 1 and 4. |
| AC24–25, AC37: history formatting, freshness, empty/error cases | L1 fixed-history repositories, CLI comparison, controlled mid-compose commit | Formatter groundwork exists; history group is undefined and function is pending. Findings 1, 2, and 4. |
| AC27: lazy roots and memoization on every surface | L1 controlled provider and pipeline fixtures | Metadata only; evaluator/provider migration is absent. Finding 3. |
| AC28: lifecycle freshness and shipped prompts | Spec-required L2 real lifecycle execution, backed by L1 provider tests | Old nested lifecycle implementation remains; new-root L2 coverage missing. Findings 3 and 4. |
| AC29: documentation/migration consistency | L1 passive corpus and targeted migration checks | Old shape remains in code, tests, and user docs. Finding 3. |
| AC30: local/URL/in-memory source identity and nesting | L1 retained-source and nested compose fixtures | Local source unit groundwork exists; nested `as_markdown` is absent. Findings 1, 2, and 4. |
| AC31: demand-driven capture and fail-closed fresh evidence | L1 work counters and injected providers | Requirements still track original groups; lazy provider absent. Findings 1 and 3. |
| AC32: passive tools and nested preflight safety | L1 zero-effect instrumentation and pipeline fixtures | New roots/functions lack runtime integration; nested effects cannot yet be verified. Findings 2 and 3. |
| AC36: same-clock unique IDs, freshness, no persistent replay | L1 nonce/provider fixtures and repeated cache-backed CLI runs | Identity unit groundwork and cache-disable prerequisite are recorded; integrated identity/freshness behavior is incomplete. Findings 1–4. |

The spec introduces no modifier, hotkey, paste, IME, mouse, or terminal scrolling behavior. L3 keyboard injection is not applicable. Neither a Rust subprocess test nor a real ICMP socket test is classified as real-terminal L2 rendering evidence.

## Verification and review limits

- Reviewed HEAD `153717fa5` plus the existing modified and untracked implementation files. No production source was edited by this review.
- Read the specification, decision record, implementation status, capture/group wiring, function registrations, lazy-root descriptors, lifecycle implementation, and relevant test sources. Applied the darkmatter and Rust testing guidance; used biscuit-file reference resolution for the missing predecessor.
- GitNexus was bound explicitly to this worktree. Its index was six commits behind. A concept query located capture/lifecycle flows; `just gitnexus` refresh failed with **No space left on device**, followed by a WAL-corruption diagnostic. Graph results were treated as incomplete, not proof of absent callers. No function/class/method edit or commit was made.
- `cd darkmatter && just test` exited **101** while compiling dependencies because `.rmeta` outputs under `target/debug/deps` were not writable. No tests ran successfully; the compiler did not reach the feature's own errors. Fix the artifact/storage environment before rerunning the affected test and lint recipes. Do not infer passing results from the source inspection.
- No L2/L3 windows were opened. No cross-OS evidence gap was used as a readiness blocker.
- Saved this review and advanced the spec to iteration 7. The missing review 6 metadata update remains outstanding.
