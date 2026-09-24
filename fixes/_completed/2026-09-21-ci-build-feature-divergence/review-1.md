---
$schema: feature-review.yaml
ready: false
findings:
    - title: No change reduces the workspace rebuilds the fix targets
      priority: high
    - title: The attribution table counts packages with no library compile
      priority: high
    - title: The alignment what-if misses configurations collapsed within one owner
      priority: medium
    - title: The guard-test fallback ruling remains unresolved
      priority: medium
human_review: true
human_review_items:
    - |-
      Decide whether this fix should deliver a build-time reduction or close as a measurement-only investigation. The current code measures the problem but makes no change to reduce builds. The measured option is to let the API schema package use the YAML library directly instead of importing it through the general file library; the local model predicts six fewer builds and about 77 seconds saved for the measured package set. Please choose:

      1. Approve that small dependency change as part of this fix, followed by its tests and a new attribution run.
      2. Revise the specification to accept the diagnostic report as the final outcome, with no build-time reduction.
    - |-
      Resolve the guard-test rule in the specification. The current rule says to move a syntax-checking test into its own package and change CI scheduling if the shared feature-alignment package is not built. That package was not built, but the move was also not made because measurement predicts no time saved. Please choose:

      1. Amend the rule to leave the test where it is and keep the feature difference.
      2. Keep the original rule and require the test move and CI scheduling change.
reviewed_by: codex/gpt-6-sol
created: "2026-09-23T20:21:02-07:00"
spec: 2026-09-21-ci-build-feature-divergence/spec.md
implemented: true
next: 2026-09-21-ci-build-feature-divergence/review-2.md
implemented_by: claude/opus
log: fixes/2026-09-21-ci-build-feature-divergence/implementation-log.md
description: "A **fix** review of `2026-09-21-ci-build-feature-divergence/spec.md`"
fix: 2026-09-21-ci-build-feature-divergence/review-1.md
---

# Review 1

**Not production-ready.** The attribution diagnostic and its fixture tests run, but the fix has not reduced workspace rebuilds. The report also counts two configurations that Cargo does not compile, and its what-if timing calculation can miss savings when one owner builds multiple configurations that merge.

This review covers the specification, implementation log, diagnostic, fixtures, and current tree. It does not use missing cross-OS or later pull-request evidence as a reason for the readiness decision.

## Findings

### High — No change reduces the workspace rebuilds the fix targets

The specification's Outcome and acceptance criterion 5 require fewer workspace configurations after the remedy. The implementation log says no dependency edge or feature configuration changed, and the Phase 4 re-run reports the same 92 Linux configurations and 65 divergent configurations before and after (`implementation-log.md`, “Acceptance criteria status”; `attribution-2026-09-21.md`, “Phase 4 re-run”). The next ordinary pull request cannot show a saving caused by this fix until a remedy lands. Marking criteria 2 and 3 as met through decisions alone does not satisfy the Outcome.

The measured, narrow remedy is the incidental `schematic-define` → `biscuit-file` dependency: production code imports only its `serde_yaml_ng` re-export (`schematic/define/Cargo.toml:10-14`, `src/openapi/options.rs:8`, `src/openapi/import/builder.rs:12`). The implementation log's temporary edit predicts 92 → 86 Linux configurations and about 77 seconds saved on the macOS timed pass. Record a ruling that permits this remedy, implement and test it, then re-run the attribution. If the intended deliverable is measurement alone, revise the specification's Outcome and acceptance criterion before calling the fix complete. The later pull-request observation remains a non-gating follow-through item; it is not the basis of this finding.

### High — The attribution table counts packages with no library compile

`attribute` adds every reachable workspace `cargo tree` node to `per_crate` (`scripts/feature-attribution.rs:631-664`), but the timing join intentionally excludes binaries and test harnesses (`scripts/feature-attribution.rs:867-903`). Cargo metadata lists only `bin` and `test` targets for `claudine-cli`, and only `bin` targets for `repo-deps`; neither has a library or build-script target. Both still appear as one workspace configuration in the checked-in macOS report, each with zero compile seconds. Thus at least two of the reported 95 macOS and 92 Linux “workspace crate configurations” are package graph roots, not compiled library or build-script configurations. The report's claim that it lists every configuration the owner builds is inaccurate, and its 27-crate count includes these noncompiled roots.

The strongest automated check is **Level 1**, but it only compares `cargo tree` fixture graphs and manufactured timing events. `every_configuration_beyond_the_first_is_explained` tests the model's own output; `timed_pass_seconds_are_charged_to_the_configurations_an_owner_built_first` detects an unexpected compile but never rejects an expected compile that has no event. This is the wrong boundary for a claim about what Cargo actually compiles. Exclude roots without a compiled library or build-script target, and add a small Level-1 integration fixture that builds a bin-only owner with the rustc wrapper and reconciles predicted configurations in both directions against observed units. Refresh the published counts and table after that correction.

### Medium — The alignment what-if misses configurations collapsed within one owner

The timed join stores one number per `(owner, package)` and divides it equally among configurations first built by that owner (`scripts/feature-attribution.rs:666-699`). Under `--align`, `seconds_removed` increases only when the owner has **zero** configurations of that package left (`:677-683`). If the archive and sidecar each built a distinct configuration that alignment merges into one, the code charges both original compile times to the survivor and reports zero savings for the removed compile. The current selection has this shape: `claudine-cli` builds both `schematic-define` and `schematic-definitions` in its archive and sidecar, and the implementation log manually adds about 25 seconds that the generic join misses in its dependency-edge what-if (`implementation-log.md`, “Sizing option B”). The same bookkeeping would misstate an alignment scenario with that shape.

`co_causes_split_seconds_and_bound_what_each_removes` is Level 1 and checks one configuration per owner. Add a fixture with two configurations of the same workspace crate first built by one owner, then merged by a what-if. Preserve per-invocation or per-configuration event identity when joining timings, or label the computed savings as a lower bound and keep it out of remedy rankings. An equal split also cannot claim exact seconds per configuration when the two rustc durations differ.

The other side of the estimate is uncertain too: `Interner::with_alignment` drops feature-specific dependency edges and explicitly does not model features forwarded to dependencies (`scripts/feature-attribution.rs:251-291`). That can overstate how many configurations an actual alignment would collapse. Add a Level-1 fixture with a forwarding feature and compare the what-if to a real build after a temporary alignment declaration before using the broader alignment scenarios as savings estimates.

### Medium — The guard-test fallback ruling remains unresolved

The Open Questions ruling says that if the alignment crate is not warranted, option 2 applies: isolate the `claudine-cli` source-scan guard and add a planner watch so source changes still select it. The implementation concluded the crate is not warranted, left the guard in place, and did not update the ruling (`spec.md`, “Open Questions”; `implementation-log.md`, “Tasks 3.2–3.4”). The measured benefit is zero, so the decision to avoid a new planner mechanism is reasonable, but the written contract now disagrees with the implementation. Resolve the pending ruling explicitly. If the test stays in `claudine-cli`, amend the fallback to say so; if option 2 remains required, implement its scheduling and tests before claiming the requirement complete.

## Verification levels and test reachability

| Requirement | Strongest present verification | Assessment |
|---|---|---|
| Attribute owner feature differences and classify causes | Level 1 fixture workspace resolved through real `cargo tree`; 18 tests in the declared `feature-attribution` binary | Appropriate for feature-graph rules. |
| Report the workspace configurations Cargo actually compiles | Level 1 model tests plus a manually recorded timed pass | Level 1 integration proof against actual rustc events is missing; see finding. |
| Join compile seconds and estimate what a remedy removes | Level 1 manufactured events; one local timed pass | Same-owner collapse and per-configuration timing are untested; see finding. |
| Reduce owner-job rebuilds while preserving isolated package features | No implementation or regression test for a remedy | Missing behavior; see finding. |
| Keep the existing archive mechanism intact | Level 1 `scripts/ci-build-archive-tests.rs`, unchanged and reported passing | Appropriate for this contract. |

The spec defines no key press, paste, mouse, scrolling, color, or terminal-emulator rendering requirement. Its table is a diagnostic report, so no Level-2 or Level-3 input/render test is required by the stated contract. The new tests have no tier-marker segment, `cargo nextest list -p repo-deps --bin feature-attribution` lists all 18, the binary is declared in `scripts/Cargo.toml` with `local-tools` enabled by the package's CI test configuration, and `just check-tier-coverage repo-deps` reports no stranded tests. Fixture directory reads use a literal `repo_root().join(...)`, which the test-input index recognizes as a directory input.

I ran the 18 `feature-attribution` tests with Nextest; all passed. This confirms test reachability and the existing model assertions, not the missing compiled-unit reconciliation or the unimplemented speed-up.
