---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T11:03:23-07:00
spec: 2026-07-13-more-is-more/spec.md
implemented: true
implemented_by: claude/default
log: darkmatter/features/2026-07-13-more-is-more/log.md
description: "A **feature** review of `2026-07-13-more-is-more/spec.md`"
feature: 2026-07-13-more-is-more/review-17.md
previous: 2026-07-13-more-is-more/review-16.md
next: 2026-07-13-more-is-more/review-18.md
---

# Review 17

## Summary

The feature is **not production ready**. The sole critical finding from review 16 remains unresolved: acceptance criteria 17–30 are still absent from production code and have no verification. The implementation record explicitly says this work was deferred and that no code, test, or specification change was made for it ([log.md:75](log.md#L75)). No More Is More implementation commit follows review 16; the subsequent commits belong to the separate meta-schema feature.

## Findings

### Critical: Acceptance criteria 17–30 remain explicitly deferred and unimplemented

The specification still promises all 30 acceptance criteria ([spec.md:1611](spec.md#L1611)). AC17–18 require indexed-file endpoint functions and object/array expression literals. AC19–21 require a shared preferred-remote resolver, live remote-branch observation, and vendor resolution. AC22–25 require structured PR and CI/CD provider APIs and expression functions. AC26–30 require cross-surface network policy, typed error preservation, catalog/DMLS/docs parity, and closed-enum return metadata ([spec.md:1670](spec.md#L1670)).

A fresh production-source search across `darkmatter/lib/src`, `darkmatter/dmls/src`, and `sniff/lib/src` finds no definitions for `find_first_index`, `find_last_index`, `branch_exists_on_remote`, `remote_vendor`, `pr_list`, or `cicd_list`. The expression AST still has no object- or array-literal variants, and the authored catalog has no closed-enum return representation. Sniff's existing `preferred_remote_url` helper remains only a precursor; it is not the shared `ResolvedRemote` authority required by AC19 and is not reused by the missing provider-query surfaces ([api.rs:229](../../../sniff/lib/src/filesystem/git/api.rs#L229)).

This is not a newly discovered partial implementation. The implementation log states that AC17–30 were deferred, describes them as multi-phase work, and confirms that no implementation or specification change was made ([log.md:75](log.md#L75)). That directly contradicts the premise that all previous suggestions were implemented.

Recommendation: implement AC17–30 as specified, or formally split the ratified specification into separately scheduled features and narrow this feature's acceptance criteria. Until one of those remedies is complete, the current 30-criterion feature cannot be marked ready.

Strongest verification present: **none** for AC17–30. These contracts require Level-1 parser/runtime tests and Wiremock-backed provider tests. Level 2 and Level 3 are not applicable because none of these requirements depends on terminal rendering or an OS-to-terminal input encoder.

## Verification Matrix

| Requirement group | Required level | Strongest present | Result |
|---|---:|---:|---|
| AC1–16: Git context and conflict prediction | Level 1 | Level 1 | Existing discriminating tests remain present; review 16 verified this subset |
| AC17: indexed-file endpoint functions | Level 1 | None | Missing production functions and tests |
| AC18: object/array expression literals | Level 1 | None | Missing AST/parser/evaluator support and tests |
| AC19–21: remote selection, branch observation, vendor resolution | Level 1, with Wiremock where probing is required | None | Missing public resolver/functions and tests |
| AC22–25: PR and CI/CD exact/list APIs and expressions | Level 1 with Wiremock | None | Missing structured provider APIs, functions, and tests |
| AC26–30: parity, policy, errors, catalog/DMLS, enum returns | Level 1 | None | Missing supporting runtime/catalog surfaces and tests |

No requirement in this feature concerns terminal glyphs, widths, SGR styling, scrolling, paste/IME/mouse behavior, hotkeys, or keyboard encoding. Level 2 and Level 3 tests are therefore not required.

## Checks Run

Review scope was derived from the specification/plan and repository discovery: `sniff`, `darkmatter`, `darkmatter-cli`, and `dmls`. GitNexus reported MEDIUM upstream risk for `merge_conflicts_with_branch_at`, HIGH risk for Git-context projection, and LOW risk for `predict_conflicts_fn`, supporting full Level-1 area gates for Sniff and Darkmatter.

Two fresh `cd darkmatter && just test` attempts were terminated with exit 130 after remaining in compilation beyond the non-interactive command-time ceiling. Neither reached the test phase or emitted a test failure. The interrupted gate is not counted as a pass or a product failure. No Sniff rerun was started after the shared Darkmatter dependency build exceeded the same ceiling.

Static verification was conclusive for the critical finding: the required production symbols/catalog entries are absent, while the implementation log explicitly records the deferral. Review 16's successful focused Level-1 and lint results remain historical evidence for AC1–16, not a fresh gate result for this iteration.

`biscuit-file` parsed the requested review/spec lifecycle frontmatter exactly. `md schema validate` could not validate the new review because the shared `schemas/feature-review.yaml` is itself invalid: it combines tagged-schema `kind`/`types` with unsupported top-level `$schema`/`description` keys. That pre-existing schema-authoring defect is outside this feature's implementation scope.

## Production Readiness

**Not ready for production.** Fourteen public acceptance criteria remain unimplemented and unverified. The implemented AC1–16 subset cannot establish readiness for the full specification.
