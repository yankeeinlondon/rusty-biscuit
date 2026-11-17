---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
    - Retain building the shared clap model only when needed, which preserves root help for incomplete administrative commands; no action is needed unless the author prefers another internal structure.
    - Retain the observed token-order precedence for mixed root version and help requests; no change is recommended.
    - Accept updating existing regression tests and the early-exit guard during Phase 1 rather than Phase 2; the final coverage is present.
    - Retain the small shared macro that generates the three command examples; no change is recommended.
    - The log's claim that `--perf compose -h` fails is stale; the reviewed binary correctly prints compose help with exit 0 and empty stderr, which should be retained.
    - Retain editing the parsing documentation through its canonical files, which the skill already shares through a symbolic link.
    - Accept the implementer's broader `just test` run as including the required CLI suite; no duplicate full run is requested.
    - Leave other operating-system evidence to CI; the attempted Windows check stopped at SSH host-key verification and changed no trust settings.
    - Retain leaving unrelated formatting differences untouched; this review requests no formatting cleanup.
    - Accept omitting downstream package tests because this change is confined to the CLI executable and its private modules.
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-11T10:48:36-08:00
spec: 2026-11-08-composition-subcommand-help/spec.md
implemented: false
description: A **fix** review of `2026-11-08-composition-subcommand-help/spec.md`
fix: 2026-11-08-composition-subcommand-help/review-1.md
---

# Review 1: Composition subcommand help

The fix is production ready. No findings require repair, and no decision requires a human before the review cycle can finish.

## Findings

None. This is the first review; there are no previous findings to reconcile.

## Contract and verification

Reviewed the specification, plan, implementation log, implementation record, and changes from the pre-implementation commit `c26823f69` through `0366d7d26`. Two independent agents read the implementation decisions and specification in parallel before this assessment.

The CLI selects composition help from the original arguments before normalization, argument ownership, configuration initialization, and composition execution. The renderer builds the same clap model used for ordinary parsing before extracting a command, preserving inherited options and command-qualified usage. The normal positional parser and provider forwarding implementation are unchanged.

All requirements need Level 1 verification, as the spec explicitly states. These screens require no terminal input handling, emulator behavior, or keyboard injection; Level 2 and Level 3 evidence would not answer an additional acceptance question.

| Requirement | Strongest relevant verification | Assessment |
| --- | --- | --- |
| Correct help for all three commands, both spellings, with and without a file | Level 1 isolated CLI cases, selector tests, and generated-model tests | Covers command usage, options, positional explanation, stdout, exit 0, and empty stderr. |
| Preserve help forwarding after `--` | Level 1 CLI execution with the portable compiled fake provider | Inspects the provider's recorded arguments for exactly one `--help`, consumes the separator, and verifies provider output. |
| Preserve root, wrapper, administrative, and version behavior | Level 1 CLI regressions and parsing tests | Retains the grouped overview, custom wrapper help, and administrative clap help. Direct checks also confirmed root version precedence and incomplete administrative root help. |
| User-facing option text and complete provider shortcuts | Level 1 generated-model and catalog comparisons | Checks descriptions, all ten visible shortcuts, inherited options, visible aliases, enum values, omitted internal fields, and absence of maintainer history. Dry-run warnings appear in compact help. |
| Help despite missing or invalid arguments | Level 1 selector, parser controls, and isolated CLI cases | Covers missing option values, malformed setters, unknown providers/switches, and conflicting or incomplete options; no-help controls retain validation errors. |
| No composition effects; plain output and no suppression/performance report | Level 1 CLI trap, missing-file cases, forced-color controls, and quiet/silent/perf cases | Checks unchanged document bytes and absent lifecycle, launch, audio, and configuration markers. Forced-color controls make plain-output checks meaningful. |
| Completion preserves its role and shortcut coverage | Level 1 model/catalog tests and both completion entry points | Completion requests containing help tokens retain completion output; the shared declarations add the existing provider shortcuts without changing argument ownership. |
| Current documentation and skill guidance match the change | Source and documentation review | The guide, README overview, parsing topics, diagrams, and skill explain root help, command help, and explicit forwarding. |

New integration tests remain in declared consolidated test modules. Their names select Level 1, and tests using compiled provider fixtures use the `test-fixtures` feature enabled by both the canonical CLI recipe and CI metadata. No new configuration reader or deserializer was added or changed, so the Input Robustness Matrix does not apply.

## Implementation decisions (informational)

None of these items meets the test for stopping for a human. The recommendations preserve the spec, and the internal choices are inexpensive to reverse.

1. **Build the shared command model at the rendering boundary.** Retain returning an unbuilt model from the factory. Ordinary parsing can still apply clap's `ignore_errors` setting before building, so root help works even with an incomplete administrative command. I confirmed `claudine --help mcp alias` exits 0 with the grouped overview and empty stderr.
2. **Resolve mixed version/help requests by order.** Retain the implemented behavior: `claudine --version compose -h` prints the version, while `claudine --help --version compose` prints root help. Both direct CLI checks exited 0 with empty stderr. No public format or option needs redesign.
3. **Update regressions earlier in the implementation sequence.** Accept moving the existing help expectations, exit guard, and initial unit tests into Phase 1 so that phase could be verified. The wider matrix subsequently landed; this is no outstanding deferral.
4. **Share the command example footer.** Retain the small constant-string macro. It varies the command and example file without maintaining a second option inventory, and the spec expressly permits a generated footer.
5. **Place performance switches in valid composition invocations.** The tests put `--perf` after the command. However, the implementation log and implementation record additionally claim `claudine --perf compose -h` exits 2. That historical claim does not describe the reviewed code: the existing command recognizer skips flag-shaped root tokens, the new help selector recognizes compose, and a direct run prints compose help, exits 0, and writes nothing to stderr. Retain the observed behavior, which satisfies the spec's instruction that performance reporting must not interfere with help. This record corrects the reported decision; no product repair is needed.
6. **Update shared documentation once.** Retain the canonical topic-file edits. The skill's topics directory already links to these files, so separate copies would introduce drift.
7. **Use the broader package-area test recipe.** Accept the recorded `just test` run as containing the required CLI test set. The implementation record reports 11,917 passing tests and a passing `just lint`; those are implementation evidence, not new runs claimed by this review.
8. **Defer other-OS verification to CI.** The attempted native Windows check stopped at SSH host-key verification. Leaving trust settings untouched and relying on CI is appropriate; cross-OS proof is expressly outside this review's readiness decision.
9. **Leave unrelated formatting alone.** Accept leaving reported pre-existing formatting differences untouched. No formatting command or unrelated source cleanup is required by this review.
10. **Omit downstream package testing.** Accept this scope: the changed code belongs to the CLI executable and its private parsing/completion modules. There is no changed library API requiring consumer verification.

No functional implementation work remains recorded as deferred. Temporary Phase 1 test deferrals were completed in Phase 2. The only outstanding verification described by the implementer concerns other operating systems.

## Review verification

Review-time checks on macOS:

- `just test-cli composition_help`: 12 passed; no test reached 5 seconds.
- `just test-cli` with `BISCUIT_TEST_FILTER` selecting the help-hoisting/selection module, argv normalization, completion ownership, provider-shortcut consistency, no-file help, and removed-sequence-flag help: 187 passed; no test reached 5 seconds. This includes the portable fake-provider forwarding and no-effects trap cases.
- `just check-tier-coverage claudine`: passed, zero stranded tests.
- Direct binary checks for root version before command help, root help before version, incomplete administrative root help, and `--perf compose -h`: all exited 0 with the expected screen and empty stderr.
- `git diff --check`: passed.

The build emitted a macOS linker warning about a large unwind section; compilation and all selected tests succeeded. This review did not rerun the full area suite or lint; their passing implementation results are identified above as recorded evidence. Cross-OS evidence is left to CI, as required by the review instructions.
