---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T11:30:20-07:00
spec: 2026-07-16-performance/spec.md
log: sniff/features/2026-07-16-performance/log.md
implemented: true
implemented_by: claude/default
description: "A **feature** review of `2026-07-16-performance/spec.md`"
feature: 2026-07-16-performance/review-9.md
---

# Review 9

## Findings

### High: native Linux and Windows execution and matched work-count artifacts are still absent

The production completion boundary still requires native macOS, Linux, and Windows tests plus
comparable per-OS work-count artifacts ([spec.md:397](spec.md#L397)). The current completion record
explicitly says those Linux and Windows runs and the matched artifact set remain absent
([phases/_completed/08-cross-platform-validation/spec.md:305](phases/_completed/08-cross-platform-validation/spec.md#L305)).
`git branch -r --contains af4751810e9bc66f3e3dbe5b883c864ce76c77a0` also returns no branch, so
the hosted matrices cannot have run for the final cycle-8 implementation.

This is especially load-bearing now: the deterministic Unix regression relies on `getppid()`
reparenting and Sniff's descendant sampling, whose Linux implementation discovers processes through
`/proc`; the Windows path relies on Job Object behavior that cannot execute on this macOS host. The
new fixture is green on macOS, and the workflow definitions are correct, but neither is native
Linux/Windows evidence. Publish one immutable final implementation identifier, retain green native
`just test` and `just lint` runs on all three OSes, and retain the three `work_counts` artifacts under
that identifier.

### High: Playa renders a timeout warning but discards the first-class timeout outcome

The newly reviewed contract makes installation timeout a distinct `InstallInterviewOutcome::TimedOut`
and narrows `Failed` to non-timeout failures ([spec.md:313](spec.md#L313)). Sniff CLI and So You Say
map that outcome to failure, but Playa only checks whether `run_install_interview` returned `Err` and
discards every `Ok` outcome ([playa/cli/src/main.rs:1009](../../../playa/cli/src/main.rs#L1009)). A
timed-out installer therefore emits the new warning, then the selected-install loop continues and the
command can finish successfully. Ordinary `Failed` and `NotInstallable` outcomes are discarded by the
same boundary.

This is an incomplete downstream migration of a user-observable failure contract. Match the outcome
explicitly: only `Installed`/`DryRun` should count as installation success; `TimedOut` must produce a
non-zero terminal verdict, while `Failed`, `NotInstallable`, and user-abort policy should be handled
deliberately. Add a Level-1 test that injects a timeout outcome at the Playa command boundary and
asserts both the warning and failure status. Level 2 is unnecessary unless exact terminal styling is
made part of the requirement.

### Medium: the public migration is recorded only in the feature spec

Cycle 8 accepted four public source breaks and documented their migration in the internal feature
spec, but the library's Unreleased changelog still lists only the older worktree API break
([CHANGELOG.md:5](../../lib/CHANGELOG.md#L5)). The public library README's program key types still
mentions only `InstallOptions` and `InstallResult` ([README.md:624](../../lib/README.md#L624)), and
the CLI installation section does not explain the timeout warning or best-effort Unix termination
([README.md:227](../../cli/README.md#L227)). The same library README also retains the already-known
stale claim that the shared executable index amortizes across eight categories even though the
authoritative count is nine ([README.md:698](../../lib/README.md#L698)).

Add the four breaking additions and matcher migration to the library changelog, document the timeout
warning/partial-install caveat on the public installation surfaces, and correct the stale category
count. The code and feature spec are authoritative; the public docs should be brought into agreement.

## Verification Levels

| User-observable requirement | Strongest present verification | Review result |
|---|---|---|
| Performance, aggregate, inventory, Git, remote, NTP, service, ownership, and output contracts | Level 1 unit, integration, spawned-CLI, snapshot, and work-counter tests on macOS | Appropriate tier and green locally; required native Linux/Windows execution remains absent. |
| Installation timeout is distinct from ordinary failure and warns before retry | Level 1 injected-runner/interview tests plus in-process Sniff terminal rendering on macOS | Appropriate for the library and Sniff CLI; Playa discards the terminal outcome and has no boundary test. |
| Between-samples Unix `setsid()` escape | Deterministic Level 1 manufactured-process fixture on macOS | Appropriate tier and now reaches a verdict on every local run; native Linux execution remains absent. |
| Windows Job Objects, registry/path behavior, and Linux `/proc` descendant discovery | Native Level 1 on macOS only; affected consumers compile on macOS | Insufficient for the explicit three-OS completion requirement. |
| CLI glyphs, widths, SGR styling, and scrolling | No changed presentation-specific contract | No new Level-2 requirement. |
| Keyboard, modifier, hotkey, paste, IME, and mouse behavior | No requirement | Level 3 is not applicable. |

## Checks Run

```text
sniff repo packages
  succeeded

just test
  sniff-lib: 1,677 passed, 19 skipped
  sniff-cli: 782 passed, 3 skipped

just lint
  passed

cargo check -p playa-cli -p biscuit-speaks-cli --all-targets
  passed on macOS

git branch -r --contains af4751810e9bc66f3e3dbe5b883c864ce76c77a0
  no remote branch contains the reviewed commit

git diff --check
  passed before review-file/frontmatter edits
```

The core performance implementation and the cycle-8 timeout fixes are green on macOS, and the prior
load-dependent Unix test gap is fixed. The explicit three-OS completion evidence is still missing,
and Playa does not preserve the new timeout failure contract end to end. Production readiness:
**not ready**.
