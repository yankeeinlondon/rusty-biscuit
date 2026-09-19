---
$schema: feature-review.yaml
ready: false
findings:
  - title: Capture the ambient repository observation at the actual request boundary
    priority: high
  - title: Finish the AC29 documentation contract and correct repository freshness claims
    priority: medium
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T20:21:37-07:00
spec: 2026-09-09-more-context/spec.md
implemented: true
implemented_by: claude/fable
log: darkmatter/features/2026-09-09-more-context/implementation-log.md
description: A **fix** review of `2026-09-09-more-context/spec.md`
fix: 2026-09-09-more-context/review-4.md
previous: 2026-09-09-more-context/review-3.md
next: 2026-09-09-more-context/review-5.md
---

# Review 4: More Context

## Verdict

The feature is **not production ready**. Review 3's Level 2 `format.md` gap and
its four explicit Level 1 acceptance guards are now closed, and the complete
Darkmatter L1 and lint gates pass. The request-owned repository observation,
however, is still captured at a pipeline entry rather than at the request
boundary required by D3. Its new regression manually invokes the private
establishment method and therefore does not exercise the public lifecycle it
claims to prove. AC29 also remains incomplete and contains two stale freshness
claims.

This verdict does not depend on cross-OS execution evidence or human review.
The remaining work is deterministic and requires no new product decision.

## Prior Review Closure

- **Finish the request-owned ambient repository observation and restore the L1
  gate:** partially implemented. Repository values now share one retained
  observation after it has been established, redundant root/topology discovery
  is removed, order-dependent package assertions are fixed, and all L1 tests
  pass. The observation is still established by the root or child pipeline,
  not when the request is created, so the central D3 timing defect remains.
- **Exercise `prompts/format.md` at the Level 2 boundary required by AC28:**
  implemented. The focus-safe tmux test runs the real shipped prompt, makes a
  fake formatter dirty exactly two tracked files, proves
  `current.dirty_files` observes two, reaches the prompt's commit completion,
  and includes an unknown-key control that fails non-vacuously. The test also
  exposed and fixed two real prompt-authoring defects.
- **Finish the explicit Level 1 acceptance guards:** implemented. Composition
  now tests typed entropy failure for `ctx.id` and `ctx.sid`; the checked-in
  migration scan rejects new and stale occurrences; URL-root request counting
  proves one caller fetch and no compose refetch; and cold/warm cache tests
  prove identity and probe output are not replayed.

Review 3's `## Blocked Findings` section was `None`, so no blocked finding was
available to become unblocked before this implementation.

## Unblocked Findings

### High — Capture the ambient repository observation at the actual request boundary

D3 requires the repository observation to be captured once per request in
`ComposeOptions`, and review 3 explicitly required a regression that creates a
request, mutates repository metadata/topology, and then performs the first lazy
evaluation. The implementation still calls
`establish_repository_observation` at the root pipeline entry after
`ComposeOptions` already exists (`pipeline/mod.rs:30-37`). It also calls the
same method at each child entry and explicitly permits a child to be the first
document that establishes the observation (`pipeline/mod.rs:123-131`). A
repository change between request creation and root composition therefore
becomes the supposedly request-fixed value. If only a transcluded or nested
child plans a repository read, earlier root stages may run before that value is
fixed.

The new regression does not expose this. It constructs the options, then calls
the private `options.establish_repository_observation(&markdown)` itself before
changing the repository (`tests/lazy_roots.rs:539-558`). The comment relabels
that manual call as request creation, but no public constructor or normal
compose entry performs it at that point. Removing the manual call would let the
pipeline observe the changed repository, which is the defect the test is meant
to catch.

Move observation establishment to the real request-creation boundary, or
introduce an explicit document-aware request constructor that atomically owns
the root document, source/resolution context, and repository observation. Then
test only through that public boundary: construct the request, mutate the
remote and workspace membership, compose without calling a private setup
method, and prove the original values survive with zero downstream discovery.
Include a root-with-child case where only the child references
`current.repo`, `current.repo_root`, or `current.packages`, and mutate the
repository during an earlier root stage to prove a descendant cannot establish
request state late.

### Medium — Finish the AC29 documentation contract and correct repository freshness claims

AC29 names `claudine/docs/topics/context/context-variables.md` as the user page
that must explain the eager/lazy pairing. The file still documents only
`ctx.*`; it contains no `current`, `current_env`, `recent_commits(count)`, or
variable/function pair rule (`context-variables.md:1-145`). The migration guard
cannot catch this kind of omission because it scans only for obsolete literal
spellings.

Two required skills also contradict D3 and the implemented fixed-repository
behavior. `.claude/skills/claudine/SKILL.md:177` and
`.claude/skills/claudine/lifecycle.md:77` use `ctx.repo_name` /
`current.repo_name`, which are not authored context keys, and say a repository
rename during the run can change `current`. Repository metadata and topology,
including `repo`, are fixed by the request observation; only mutable Git and
filesystem facts such as branch, history, and dirty files refresh.

Add the required binding-time section to the context-variables page and replace
the stale examples with real keys and the fixed-versus-refreshable distinction.
Add a passive documentation contract that checks the required pages contain
the direct lazy-root spelling, pair example, and fixed repository statement;
the literal migration allowlist alone proves only that the removed nesting did
not return.

## Blocked Findings

None. Both findings can be implemented and verified with deterministic Level 1
fixtures and passive documentation checks.

## Requirement Verification Levels

No requirement asserts terminal glyph geometry, SGR styling, scrolling,
keyboard input, paste/IME, or mouse behavior. Level 3 is not applicable. Level
2 is required only by AC2 and AC28's explicit real-Claudine boundary; AC14 is a
real network-resource test rather than a terminal tier.

| Requirements | Required level | Strongest evidence and assessment |
| --- | --- | --- |
| AC1, AC19, AC26 | Level 1 passive corpus, generator drift, and listing CLI | Present at the correct level. |
| AC2 | Level 2 real Claudine | Linked/main-worktree coverage was accepted in review 2; no related path changed in this cycle. Correct level. |
| AC3-AC5, identity part of AC36 | Level 1 fixed vectors and normal compose/CLI paths | Hash parity, root retention, uniqueness, fixed vectors, and typed entropy failure are present. Correct level. |
| AC6-AC8, AC33 | Level 1 deterministic ICMP transport/policy fixtures | Present at the correct level. |
| AC9, AC13 | Level 1 interface/route fixtures and projection | Present at Level 1; cross-OS execution belongs to CI. |
| AC10-AC12, AC34 | Level 1 controlled shell/profile fixtures | Present at the correct level; no terminal window or real user profile is required. |
| AC14 | Real network resource on each required OS | Tests exist; cross-OS results do not affect this readiness verdict. |
| AC15-AC18, AC20, AC32, AC35 | Level 1 compose/function fixtures | Present at the correct level. |
| AC21-AC23 | Level 1 ambient and supplied-evidence fixtures | Present at the correct level. |
| AC24-AC25, AC37 | Level 1 fixed repositories and CLI-format parity | Present at the correct level. |
| AC27, AC31 | Level 1 scripted provider, pipeline, and work counters | The present tests pass and prove reuse after explicit establishment, but they do not prove D3's request-boundary timing or a descendant-only lazy repository read. **High implementation and coverage gap.** |
| AC28 | Level 2 real Claudine, supported by Level 1 executor/provider tests | The tmux binary passed 5/5 with no skips, including real `plan.md` and `format.md` cases. Correct level. |
| AC29 | Level 1 passive documentation and migration guards | The literal allowlist exists, but a required user page omits the new binding model and two required skills describe impossible/stale repository behavior. **Medium documentation gap.** |
| AC30 | Level 1 source-kind and request-count fixtures | URL-root request counting now proves one caller fetch, one child fetch, and no root refetch. Correct level. |
| Cache portion of AC36 | Level 1 cold/warm cache fixture | CLI and library tests now prove fresh `ctx.id`/`ctx.sid`, changed probe/code output, and no composed/operation/snapshot persistence. Correct level. |

## Verification

- `cd darkmatter && just test --color=never` passed: 8,094 tests executed,
  8,094 passed, 14 higher-tier tests skipped.
- `cd darkmatter && just lint` passed for `darkmatter`, `darkmatter-cli`,
  `dmls`, `zed-dmls-cli`, and the `wasm32-wasip2` extension check.
- `cd claudine && BISCUIT_TEST_REQUIRED_BACKENDS=tmux just _test_l2
  claudine-cli --features terminal-tests --test
  level2_ac28_lifecycle_lazy_roots --color=never` passed 5/5 with no skips;
  backend proof recorded `tmux run=5 skip=0 panic=0`.
- `cd claudine && just lint` passed. The macOS linker emitted the existing
  `__eh_frame` size warning while building the CLI test binary.
- `git diff --check` passed after writing this review.
- GitNexus was bound to the `rusty-biscuit` index for this worktree at commit
  `583ea7c58`. The index reports `incremental-in-progress`, and its concept
  query found the branch-local `CurrentProvider` definition but no execution
  flow, so direct source and call-site inspection supplied the review evidence.
- GitNexus `detect-changes --scope all` completed without truncation or a
  partial result, reporting 132 changed symbols across 478 files and no mapped
  process. Because the shared worktree diff is broad and the index remains
  incremental, that zero-process result was not treated as proof of no impact.
