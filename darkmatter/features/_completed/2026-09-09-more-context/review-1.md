---
$schema: feature-review.yaml
ready: false
findings:
  - title: Complete the capture-group integration so Darkmatter compiles
    priority: critical
  - title: Implement the thirteen expression functions that still deliberately fail
    priority: high
  - title: Implement the lazy roots and replace Claudine's obsolete lifecycle shape
    priority: high
  - title: Add the missing Level-1 acceptance matrix and required Level-2 Claudine coverage
    priority: high
human_review: true
human_review_items:
  - |-
    Confirm the execution-identifier choice already adopted by the specification: include a fresh random value in each execution's identifiers so two otherwise identical runs differ. The alternatives are repeatable identifiers that may collide between runs, or secret-key identifiers that also require key management. The adopted random-value option is recommended.
  - |-
    Confirm when live context values should refresh. The adopted choice keeps repeated reads of one value consistent within one expression, then refreshes it in the next expression. The alternatives are keeping the first value throughout the document or refreshing on every read. The adopted per-expression choice is recommended.
  - |-
    Confirm the persistent-cache boundary already adopted by the specification: save only raw downloaded responses and never save composed documents, operation results, or context snapshots. The alternative is to disable persistence of downloaded responses too until a separate freshness policy is designed.
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T03:36:11-07:00
spec: 2026-09-09-more-context/spec.md
log: darkmatter/features/2026-09-09-more-context/implementation-log.md
implemented: true
implemented_by: claude/opus
description: A **fix** review of `2026-09-09-more-context/spec.md`
fix: 2026-09-09-more-context/review-1.md
next: 2026-09-09-more-context/review-2.md
---

# Review 1: More Context

## Verdict

The feature is **not production ready**. The current Darkmatter library does
not compile, thirteen specified expression functions are registered only as
intentional errors, and the `current` / `current_env` runtime and Claudine
migration have not been implemented. Most of the acceptance matrix therefore
has no executable end-to-end evidence, including the two explicitly required
Level-2 Claudine cases.

This verdict is based on implementation and verification defects. Missing
cross-OS execution evidence and the pending human confirmation of Q1-Q3 are not
used as readiness blockers.

## Findings

### Critical — Complete the capture-group integration so Darkmatter compiles

The capture implementation now references `ContextGroup::Document`,
`ContextGroup::GitHistory`, and `ContextGroup::Network` in
`capture/mod.rs:163-205`, `capture/snapshot.rs:302-304,769-783`, and
`context/runtime.rs:330,540-553`. However,
`capture/groups.rs:9-32` still declares only the original eleven variants, and
its `all`, `name`, and `projected_keys` mappings at lines 34-95 likewise omit
the three new groups. An isolated `cargo check -p darkmatter` reaches product
code and fails with twelve `E0599` errors for those missing variants.

This is on a CRITICAL path: refreshed GitNexus impact analysis found 34
upstream symbols across eight modules and both the Darkmatter compose pipeline
and Claudine sequence composition flow. Add the variants and complete their
key ownership, stable names, requirements scanning, cache-name round trips,
and exhaustive matches as one coherent change. Preserve the spec's separation
between ordinary Git facts and expensive recent-history demand. Then compile
and run the affected package tests before relying on any of the new capture
unit tests.

### High — Implement the thirteen expression functions that still deliberately fail

`expression/functions/pending.rs:41-55` binds `package_area`, `package`,
`recent_commits`, `ipv4`, `ipv6`, `has_alias`, `has_builtin_function`,
`has_user_function`, `can_execute`, `has_agentic_cli`, `as_markdown`, `ping`,
and `ping_under` to one handler that always returns “not implemented yet.” Its
test at lines 69-93 intentionally verifies that every public binding fails.
Only the `has_binary` alias has executable behavior.

Consequently the repository lookup, call-time Git history, address filtering,
shell classification, executable composition, agent detection, nested
composition, and ICMP consent/probe contracts do not exist. Descriptor and
generator coverage proves discoverability, not functionality. Implement the
handlers through the request-owned resolution, repository, context, recursion,
and effect-policy authorities specified by the design, and replace the pending
failure ratchet with positive, negative, and adversarial dispatch tests.

### High — Implement the lazy roots and replace Claudine's obsolete lifecycle shape

`expression/catalog/roots.rs:1-7` explicitly says `current` and `current_env`
are metadata only. The evaluator's known-root check in `subtree.rs:262-269`
recognizes only `ctx`, `env`, and `doc`. Claudine still materializes the old
`current.ctx.*` / `current.env.*` object in
`composition/lifecycle/context.rs:443-510`, including a fresh ambient
`ComposeContext::capture_for_dir` at event time. `composition/reserved.rs:19`
still omits `current_env` from `LATE_BINDING_ROOTS`.

This violates the clean-break spelling, reserved-root, fail-closed supplied
evidence, fixed request authority, per-expression memoization, and
per-lifecycle-event freshness contracts. Implement a request-owned refresh
provider and separate eager requirements from lazy capabilities; integrate the
roots across frontmatter, body, conditions, branches, nested composition, and
preflight; then migrate Claudine code, tests, prompts, user documentation, and
skills together. The remaining active `current.ctx.` / `current.env.` hits and
“implementation pending” notices demonstrate that AC29 is also incomplete.

### High — Add the missing Level-1 acceptance matrix and required Level-2 Claudine coverage

The new document and network tests are in-process projection tests. They cover
useful internals such as identity encoding, retained source bytes, tailnet
classification, and gateway projection, but the library currently cannot
compile and those tests do not establish behavior through normal `md compose`
or Claudine entry points. There is no successful test path for any of the
thirteen pending functions or either lazy root.

AC2 explicitly requires a Level-2 real Claudine compose from linked and main
worktrees. AC28 explicitly requires Level-2 real lifecycle execution for every
event, live environment refresh, and shipped-prompt composition. No matching
feature-specific Level-2 tests were found. Add those cases without focusing a
terminal window, along with the Level-1 CLI/pipeline, policy, fixture, passive
tooling, and demand-counter cases in the specification. Existing Level-2 tests
of unrelated Claudine behavior and Level-1 tests of the obsolete nested
lifecycle object are not evidence for the new contract.

## Requirement Verification Levels

The feature changes deterministic composition, repository and host discovery,
bounded process/network probes, passive tooling, and Claudine lifecycle
evaluation. Level 1 is appropriate for all deterministic contracts and
fixture-backed probes. AC2 and AC28 require Level 2 because the specification
requires real Claudine execution. AC14 requires real network-stack resource
tests, which are not terminal-rendering Level 2. Nothing depends on terminal
input encoding, keyboard events, paste/IME, mouse behavior, or scrolling, so
Level 3 is not applicable.

| Requirements | Appropriate level | Strongest evidence present and assessment |
| --- | --- | --- |
| AC1, AC19, AC26: catalogs and generated roster | Level 1 passive corpus, generator drift, and spawned listing CLI | Catalog/generator unit coverage exists, but checkpoint 4 remains open and the package does not compile. Not verified end to end. |
| AC2: composed linked/main worktree identity | Level 2 real Claudine execution | No feature-specific Level-2 case found. **Wrong/missing level; high gap.** |
| AC3-AC5, AC30, identity portions of AC36: source identity, hashes, unique IDs, nesting | Level 1 fixed vectors plus normal CLI/pipeline fixtures | New in-process `document.rs` tests cover vectors and retained bytes. No compiling pipeline, `md hash` parity, transclusion, or nested-compose acceptance path. Gap. |
| AC6-AC8, AC33: ICMP consent, outcomes, budgets, and HTTP-policy isolation | Level 1 deterministic transport/policy fixtures through compose | Sniff transport groundwork exists; Darkmatter `ping` handlers are intentional errors and no effect-policy integration exists. Gap. |
| AC9, AC13: tailnet and gateways | Level 1 interface/route fixtures and context projection | Sniff parser fixtures and Darkmatter projection unit tests exist, but capture-group integration does not compile. Partial only. |
| AC10-AC12, AC34: shell classification and executable checks | Level 1 controlled process/profile fixtures | `has_binary` aliases existing behavior; the other required handlers are intentional errors. Gap. |
| AC14: IPv4/IPv6 loopback ICMP | Real network-stack resource tests on each required OS | Sniff real-resource tests exist at the correct boundary. Cross-OS execution evidence is left to CI and does not affect this verdict. |
| AC15-AC16: full nested compose and shared recursion | Level 1 pipeline fixtures | `as_markdown` is an intentional error. Gap. |
| AC17, AC35: repository lookup and typed paths | Level 1 captured-topology and `FileReference` fixtures | Both handlers are intentional errors. Gap. |
| AC18: agent aliases and unknown-name rejection | Level 1 controlled PATH and dispatch fixtures | Generated roster exists; runtime handler is an intentional error. Gap. |
| AC20: address filtering | Level 1 injected interface fixtures through expression dispatch | Network primitives exist; `ipv4`/`ipv6` handlers are intentional errors. Gap. |
| AC21-AC23: empty-string scope and conditional truthiness | Level 1 Sniff plus ambient and supplied compose paths | Sniff normalization coverage exists. Darkmatter/Claudine capture cannot compile, and the complete five-position dual-path matrix was not found. Gap. |
| AC24-AC25, AC37: eager and live recent history | Level 1 fixed repositories, CLI parity, and controlled mid-compose mutation | Sniff formatter groundwork and eager projection code exist; the history group is undefined and `recent_commits` is an intentional error. Gap. |
| AC27: lazy roots on every expression surface | Level 1 controlled provider and pipeline fixtures | Descriptor metadata only; evaluator/provider implementation is absent. Gap. |
| AC28: lifecycle roots, freshness, and shipped prompts | Level 2 real Claudine lifecycle execution, supported by Level-1 provider tests | Claudine still implements the old nested object; no matching Level-2 case found. **Wrong/missing level; high gap.** |
| AC29: documentation and clean-break migration | Level 1 passive corpus and scoped drift check | Active code/docs/skills retain old nesting and “implementation pending” markers. Gap. |
| AC31: demand-driven and fail-closed capture | Level 1 injected providers and work counters | Some capture counters exist, but undefined groups prevent execution and no lazy provider exists. Gap. |
| AC32: passive tools and nested preflight safety | Level 1 zero-effect instrumentation and pipeline fixtures | Descriptor work exists; nested composition, lazy roots, and ICMP effects are absent. Gap. |

## Verification and Review Limits

- Reviewed HEAD `583ea7c58` with the existing deletion of `review-7.md` and the
  caller's reset of `review_iterations` preserved before advancing it to 1.
- Refreshed the GitNexus index. The seeded capture entry point has CRITICAL
  upstream risk: 34 impacted symbols, two execution processes, and eight
  modules. The `ContextGroup` enum lookup itself was UNKNOWN, so direct text
  search and compiler evidence were used rather than treating zero graph
  callers as safety evidence.
- `cd darkmatter && just test` failed before product compilation because the
  shared `target/debug/deps` artifacts are not writable. An isolated writable
  target allowed `cargo check -p darkmatter` to reach the crate and reproduce
  the twelve missing-variant errors. No passing Darkmatter test or lint result
  is claimed.
- No Level-2 or Level-3 terminal windows were opened. No cross-OS evidence gap
  was used as a readiness blocker.
