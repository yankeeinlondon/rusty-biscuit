---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T02:26:16-07:00
spec: 2026-07-13-more-is-more/spec.md
implemented: false
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-16.md
previous: 2026-07-13-more-is-more/review-15.md
---

# Review 16

## Summary

The feature is **not production ready**. Review 15's two high-severity defects in the implemented Git context/conflict-prediction subset have been fixed and now have discriminating Level-1 regression tests. However, the critical scope gap is unchanged: acceptance criteria 17–30 still have no production implementation or required Level-1 verification.

## Findings

### Critical: Acceptance criteria 17–30 remain unimplemented

The specification still requires all 30 acceptance criteria ([spec.md:1611](spec.md#L1611)). AC17–18 require indexed-file functions and object/array expression literals; AC19–21 require preferred-remote, live remote-branch, and vendor-resolution surfaces; AC22–25 require structured PR and CI/CD provider APIs and expression functions; and AC26–30 require their network policy, typed error preservation, catalog/DMLS/docs parity, and closed-enum return metadata ([spec.md:1670](spec.md#L1670)).

A fresh production-source search across Darkmatter, DMLS, and Sniff finds no definitions for `find_first_index`, `find_last_index`, `branch_exists_on_remote`, `remote_vendor`, `pr_list`, or `cicd_list`; it likewise finds no object/array literal AST variants or catalog enum-return representation. The implementation log explicitly records this finding as deferred and says no code, test, or specification change was made for it ([log.md:75](log.md#L75)). A private preferred-remote helper is only a precursor and does not satisfy AC19 because the provider-query surfaces that must share it do not exist.

Recommendation: implement and verify AC17–30, or formally split the specification so AC1–16 define this delivery and the remaining contracts move into separately scheduled features. The current 30-criterion feature cannot be marked complete while 14 public acceptance criteria are absent.

Strongest verification present: **none** for AC17–30. These requirements need Level-1 in-process/runtime coverage, including Wiremock-backed provider tests where the specification requires network behavior. Level 2 and Level 3 are not applicable because these contracts do not depend on real-terminal rendering or a terminal emulator's input encoder.

## Resolved Since Review 15

- Unsafe merge configuration is now rejected against committed participating paths before the built-in merge runs ([merge_conflicts.rs:54](../../../sniff/lib/src/filesystem/git/merge_conflicts.rs#L54)). Level-1 regressions cover clean built-in merges and trivial no-op/fast-forward exemptions ([merge_conflict_prediction.rs:544](../../../sniff/lib/tests/merge_conflict_prediction.rs#L544), [merge_conflict_prediction.rs:584](../../../sniff/lib/tests/merge_conflict_prediction.rs#L584)).
- Bare repositories now remain valid for HEAD/ref queries while checkout-dependent capture degrades independently ([types.rs:596](../../../sniff/lib/src/filesystem/git/types.rs#L596), [snapshot.rs:98](../../lib/src/markdown/compose/context/capture/snapshot.rs#L98)). The Level-1 integration fixture asserts the branch, null worktree, empty conflicts, and no discovery diagnostic ([git_context_integration.rs:261](../../lib/tests/git_context_integration.rs#L261)).

No additional correctness, ergonomics, or performance blocker was found in the implemented AC1–16 subset during this iteration.

## Verification Matrix

| Requirement group | Required level | Strongest present | Result |
|---|---:|---:|---|
| AC1–16: Git context and conflict prediction | Level 1 | Level 1 | Passes focused tests; review 15's two high-severity gaps are resolved |
| AC17: indexed-file endpoint functions | Level 1 | None | Missing |
| AC18: object/array expression literals | Level 1 | None | Missing |
| AC19–25: remote selection, branch/vendor, PR, and CI/CD APIs/functions | Level 1, with Wiremock for network behavior | None | Missing |
| AC26–30: surface parity, network policy, errors, catalog/DMLS, enum returns | Level 1 | None | Missing |

No requirement in this feature depends on terminal glyph rendering, scrolling, mouse/paste/IME behavior, or terminal keyboard encoding. Level 2 and Level 3 tests are therefore not required.

## Checks Run

The following focused macOS gates passed during this review:

```text
cd sniff && just test       # sniff 1350/1350; sniff-cli 769/769
cd sniff && just lint
cd darkmatter && just test  # darkmatter 5783/5783; darkmatter-cli 560/560; dmls 568/568
cd darkmatter && just lint
```

These are Level-1 verification. This review did not independently execute Windows or Linux builds.

Darkmatter's frontmatter reader parsed all requested review lifecycle values exactly. `md schema validate` could not validate either review because the shared `schemas/feature-review.yaml` definition itself uses an invalid mixed schema form (`kind`/`types` together with top-level `$schema`/`description`); that pre-existing schema-authoring issue is outside this feature's implementation scope.

## Production Readiness

**Not ready for production.** The implemented AC1–16 subset is now clean under the focused Level-1 and lint gates, but 14 of the specification's 30 acceptance criteria remain unimplemented and unverified. Passing tests for the delivered subset cannot establish readiness for public functionality that does not exist.
