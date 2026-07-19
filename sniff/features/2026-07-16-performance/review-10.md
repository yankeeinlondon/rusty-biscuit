---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T12:16:20-07:00
spec: 2026-07-16-performance/spec.md
implemented: false
description: "A **feature** review of `2026-07-16-performance/spec.md`"
feature: 2026-07-16-performance/review-10.md
---

# Review 10

## Findings

### High: native Linux and Windows execution and matched work-count artifacts remain absent

The production completion boundary still requires native macOS, Linux, and Windows tests plus
comparable per-OS work-count artifacts ([spec.md:397](spec.md#L397)). The current completion record
still says the Linux and Windows native runs and matched artifact set are absent
([phases/08-cross-platform-validation/spec.md:307](phases/08-cross-platform-validation/spec.md#L307)),
and `git branch -r --contains 77b3ea5ed0b9fffbc8a88bcca1fcd2bcd9302023` returns no branch. Therefore
neither the three-OS test workflow nor the scheduled work-count matrix could have executed the final
reviewed tree.

The macOS Level-1 suites are green, and Windows GNU cross-compilation succeeds for both Sniff and
the newly changed Playa consumer. Those checks prove local behavior and target compilation; they do
not execute Linux `/proc` descendant discovery or Windows Job Object, registry, path, and service
behavior. Publish one immutable final implementation identifier, retain green native `just test`
and `just lint` results on all three operating systems, and retain all three `work_counts` artifacts
for that same identifier.

### Medium: Playa's timeout regression tests stop short of the command boundary

Playa now handles the outcome correctly: `install_players` maps an error verdict to `error_exit`
([main.rs:1009](../../../playa/cli/src/main.rs#L1009)). Its tests do not exercise that composition.
`install_verdict_timeout_fails_and_names_the_deadline` calls only the pure verdict helper
([main.rs:1823](../../../playa/cli/src/main.rs#L1823)), while
`timeout_warning_prose_reaches_the_rendered_line` passes an arbitrary string directly to
`render_prose_line` ([install_ui.rs:182](../../../playa/cli/src/install_ui.rs#L182)). The latter
would remain green if the `TimeoutWarning` event arm were removed, and neither test would fail if
the selected-install loop resumed discarding the verdict.

The appropriate tier is Level 1, but the missing seam is the one that regressed in review 9. Add an
injected interview runner or a command helper returning a terminal verdict, drive it with
`InstallInterviewOutcome::TimedOut`, and assert that the timeout event reaches the delegate and the
command result is non-zero. Exact SGR styling is not part of this contract, so Level 2 is not
required.

### Medium: the category migration still leaves authoritative and public documentation contradictory

The required Sniff skill still says program detection has nine categories and calls test runners
the ninth ([SKILL.md:33](../../../.claude/skills/sniff/SKILL.md#L33)), while the source and updated
library README correctly describe ten. The public CLI README also says every category supports an
`install` subcommand ([README.md:229](../../cli/README.md#L229)), but notification helpers and test
runners intentionally expose no install action
([args/mod.rs:313](../../cli/src/args/mod.rs#L313)). The actual contract is ten detectable categories,
eight installable categories.

Correct both live documentation surfaces. The source is authoritative; this is documentation drift,
not a request to add installation behavior to the two report-only categories.

## Verification Levels

| User-observable requirement | Strongest present verification | Review result |
|---|---|---|
| R1-R14 work reduction, aggregate reuse, inventory, Git, remote, NTP, service, ownership, and output contracts | Level 1 unit, integration, spawned-CLI, snapshot, and work-counter tests on macOS | Appropriate functional tier and green locally; the explicit native Linux/Windows completion requirement remains unverified. |
| Installation timeout warns and produces a non-zero terminal verdict | Level 1 interview/event tests in Sniff plus isolated Playa render and verdict helper tests | Correct tier, but Playa lacks the composed command-boundary regression required by review 9. |
| Linux `/proc` descendant discovery and Windows Job Object, registry/path, and service behavior | macOS native Level 1 plus Windows target cross-compilation | Insufficient for the specification's native three-OS requirement. |
| CLI JSON/text/plain output and exit/channel contracts | Level 1 spawned CLI tests and snapshots on macOS | Appropriate tier; native cross-platform execution remains pending. |
| CLI glyphs, widths, SGR styling, and scrolling | No changed presentation-specific contract | No new Level-2 requirement. |
| Keyboard, modifier, hotkey, paste, IME, and mouse behavior | No requirement | Level 3 is not applicable. |

## Checks Run

```text
bf reference @sniff/features/2026-07-16-performance/spec.md
  resolved the requested spec path

sniff repo packages --json
  succeeded

just test  # sniff/
  sniff: 1,677 passed, 19 skipped
  sniff-cli: 782 passed, 3 skipped

just lint  # sniff/
  passed on macOS

just test  # playa/
  playa: 50 passed
  playa-cli: 19 passed

just lint  # playa/
  passed on macOS

cargo check -p sniff --all-targets --features remote --target x86_64-pc-windows-gnu
  passed with four warnings in Windows/test-only configurations

cargo check -p playa-cli --all-targets --target x86_64-pc-windows-gnu
  passed with pre-existing cross-target warnings

git branch -r --contains 77b3ea5ed0b9fffbc8a88bcca1fcd2bcd9302023
  no remote branch contains the reviewed commit

git diff --check
  passed before review-file/frontmatter edits
```

The core performance implementation remains green on macOS, the review-9 Playa behavior fix is
present, and Windows cross-compilation succeeds. The required final native Linux/Windows execution
and matched work-count artifacts are still missing, Playa's regression test does not cover the
command boundary, and two live documentation surfaces remain stale. Production readiness:
**not ready**.
