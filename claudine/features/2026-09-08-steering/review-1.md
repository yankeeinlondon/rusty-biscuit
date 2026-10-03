---
$schema: feature-review.yaml
ready: false
findings:
  - "High: Researched steering routes lack production adapters and discovery"
  - "High: Terminal steering presentation lacks real-terminal verification"
  - "Medium: Load-bearing duplicate keys are silently resolved by position"
  - "Medium: The Claudine agent skill omits the shipped steering workflow"
human_review: true
human_review_items:
  - |-
    Decide the release scope for steering. The specification asks for same-host discovery and delivery wherever technically possible, but this implementation enables only Claudine-managed Codex on macOS at version 0.157.1. Several researched provider routes still have no production adapter or native discovery. Choose one:

    - Complete the remaining implementable provider routes within this feature, with disposable-session verification before enabling each one.
    - Amend the feature scope to a Codex-first release and create separate tracked work for each deferred provider route. Keep the current unavailable explanations until those routes are verified.

    The review recommends the second option for a smaller release, but the specification currently retains the broader requirement.
has_blocked_findings: true
blocked: false
reviewed_by: codex/gpt-6-sol
recurrence: false
created: 2026-09-29T00:24:23-07:00
spec: 2026-09-08-steering/spec.md
implemented: false
description: A **feature** review of `2026-09-08-steering/spec.md`
feature: 2026-09-08-steering/review-1.md
---

# Steering implementation review

## Verdict

**Not production ready against the current specification.** The managed Codex route has meaningful unit, daemon, and disposable-provider tests. The remaining work is provider scope, real-terminal verification of the session picker, and strict handling of ambiguous input. Cross-OS proof and a later human review are not counted as readiness defects here.

## Findings

### High: Researched steering routes lack production adapters and discovery

**Defect class:** A researched provider route with a potential same-host control channel is presented as unavailable because the corresponding production adapter or native discoverer has not been built. This conflicts with the specification's request to discover and steer provider-launched sessions wherever technically possible; it is an implementation gap, not evidence that those providers cannot steer. The implementation log explicitly defers these routes pending a scope decision. This finding is **blocked by the human release-scope decision** in frontmatter.

| Site | Shape checked | Observed result | Expected result under the current specification |
| --- | --- | --- | --- |
| Claudine-managed Codex, macOS 0.157.1 | Active turn, disposable provider and real daemon | Implemented; real wrapper send is verified | Clean |
| Native Claude Code registry | Live registry record and native session | Listed unavailable; no peer-socket delivery adapter | Implement and verify delivery where the peer protocol can give a reliable receipt, or document a concrete technical blocker |
| Kilo and OpenCode HTTP control | Managed server and native endpoint discovery | No production adapter; native discovery gaps | Implement and verify the viable researched routes, or narrow the approved scope |
| Gemini and Goose ACP control | Managed ACP and native discovery | No production adapter; Goose also lacks local disposable-provider evidence | Implement after disposable verification; keep externally unverified cases unavailable |
| Qwen daemon follow-up | Managed daemon and native discovery | No production adapter | Implement manual follow-up where receipts are verifiable; automatic rescue remains unavailable for next-turn-only delivery |
| Kimi and Antigravity | Current-version interface | Unknown protocol details; no adapter | Clean to leave unavailable pending research of the current interface |
| Pi retained RPC | Reviewed blocked launch profile | Explicitly unavailable despite a test implementation | Clean: the recorded session-switch risk blocks activation |

The table is a sweep of the roster, using the [generated cases](../../lib/src/steering/generated.rs), [adapter inventory](../../lib/src/steering/adapters.rs), [local discoverer list](../../cli/src/commands/steer/service.rs), and implementation log's provider-by-provider outcome. The [real Codex wrapper test](../../cli/tests/real/real_codex_app_server.rs) is a positive control; the other rows cannot be reproduced as successful sends because the shipped command has no route for them. Complete or explicitly revise the contract before marking the feature ready.

### High: Terminal steering presentation lacks real-terminal verification

**Defect class:** User-visible terminal behavior is asserted from rendered strings or scripted picker input, but no test captures the running command through a real terminal emulator or multiplexer. The specification requires disabled rows to appear dim with a single strikethrough, narrow details to wrap, and selection to stay disabled. Level 1 proves what Claudine emits; Level 2 is needed to prove what a user sees.

| Site | Shape checked | Observed result | Expected verification |
| --- | --- | --- | --- |
| Session table | Available and unavailable rows at 44 columns | Level 1 checks ANSI fragments and line width | Level 2 pane capture checks the visible columns, details, dim styling, and single strikethrough |
| Session picker | Unavailable row beside an eligible row | Level 1 scripted interaction checks selection rules and emitted styling | Level 2 pane capture checks the actual disabled-row display and navigation without sending |
| Consent prompt | Interruption-required target | Level 1 scripted accept/decline tests | Level 2 pane capture checks the explanation and cancellation state |
| JSON list and explicit-ID send | Machine-readable output | Binary and daemon tests cover these paths | Clean; no terminal rendering claim |
| Automatic warning text | Wrapped run's STDERR status line | Level 1 notice tests and real Codex behavior test | Level 2 pane capture checks visible wrapping and status display |

There is no steering-focused `level2_` test under the Claudine CLI test tree. Add a focus-safe tmux or emulator capture test for the visual requirements. `just check-tier-coverage claudine` reports no stranded tests, so a properly named Level 2 test has a running tier.

### Medium: Load-bearing duplicate keys are silently resolved by position

**Defect class:** A document can provide two values for one decision-making key, and the value read depends on which appears last. The config matrix explicitly pins this behavior as a known gap; the native Claude registry reader has the same `serde_json::Value` behavior. This can change whether automatic help is enabled or which live session the listing claims. The activation-policy YAML parser is clean because it deserializes into typed structures and rejects duplicates.

| Site | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| User config, `steering.automatic.enabled` | Duplicate `false`, then `true` | Loader reports `true` | Reject ambiguous setting |
| Repo config, `steering.automatic.enabled` | Same duplicate | Loader reports `true` | Reject ambiguous setting |
| Native Claude registry, `pid`, `sessionId`, `startedAt`, `kind`, `entrypoint`, `status`, `cwd`, `name`, or `version` | Duplicate key with conflicting values | `read_record` first loads a JSON value, so the last value reaches discovery | Reject ambiguous record; do not list a target whose identity or state depends on key order |
| Steering activation policy YAML | Duplicate decision key | Typed YAML deserialization rejects it | Clean |

For the config field, the [existing matrix](../../lib/src/steering/automatic/tests.rs)'s control, absent, null, wrong-type, and trailing-content cells pass; `false` and `true` are the valid values, so an empty collection is a wrong type and is rejected. For the registry, the existing matrix rejects missing/null/wrong-type required fields and trailing content; optional fields may be absent, while null or wrong types fail. Its missing duplicate-key cell and its lack of a public listing assertion are test gaps. The Claudine library's [read_record](../../lib/src/steering/native/claude_registry.rs) reads Claude's registry record into a generic JSON value before validating fields, which loses duplicates. Add one real-file edit per duplicate field to the matrix and assert the public loader or discovery result. Keep the positive fixture control and the other shape cells together so one future parser change cannot fix only a single key.

### Medium: The Claudine agent skill omits the shipped steering workflow

**Defect class:** The current developer guidance does not describe new public behavior after implementation, so an agent using the package guidance cannot find the steering command or its activation limits. The specification explicitly requires updating the Claudine skill, and the implementation log says that edit was left undone.

| Site | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| [Claudine skill](../../../.claude/skills/claudine/SKILL.md) | Search for steering command, managed Codex profile, automatic help, and secret audit guidance | Only the provider research command mentions `steering` | Explain the shipped command, enabled profile, automatic behavior, and testing routes, with links to current topic docs |
| [CLI guide](../../docs/cli/steer.md) and [topic docs](../../docs/topics/steering-routing.md) | Same behavior | Present | Clean |

Update the skill from the shipped behavior and link to the more detailed docs. The skill's omission does not change runtime behavior, but it leaves a stated acceptance criterion incomplete.

## Verification performed

- `cargo nextest run -p claudine --lib -E 'test(steering::)' --color never`: 86 passed.
- `just check-tier-coverage claudine`: no stranded tests.
- Reviewed the shipped CLI, controller, daemon router, provider adapter inventory, native registry reader, config loader matrix, and declared test targets. No production code was changed.
