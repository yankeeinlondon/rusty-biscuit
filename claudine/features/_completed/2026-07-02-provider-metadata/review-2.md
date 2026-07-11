---
ready: false
implemented: true
agent: codex/default
created: 2026-07-09T15:45:19
---

# Review 2: Provider Metadata Automation

## Verdict

Not production ready.

Two of Review 1's findings are materially fixed: the wrapper now carries both stdout and stderr tails into the exit-source payload, and the default model application reads `ProviderInfo::model_cli_flag`. The remaining Antigravity bespoke-signal finding was not implemented. Instead, the implementation added a third `documentation` detection mode that lets compiled records opt out of runtime detection and positive replay. That directly contradicts the ratified signal-catalog contract and allows `signals check` to exit successfully with only 83 of 85 records positively verified.

## Findings

### High: `documentation` mode bypasses the ratified runtime and replay contract

- Requirement: `detection` is exactly `declarative | bespoke`; detection records are explicitly “not documentation-only,” every record drives runtime detection, and `signals check` must replay every record through shipped behavior ([spec.md](../../features/2026-07-02-provider-metadata/spec.md#runtime-records-drive-detection-decided)). The supplemental design is equally explicit that every declarative and bespoke record requires a positive fixture and emitted `SignalEvent`.
- Implementation: `DetectionMode::Documentation` was added to the shared vocabulary and sidecar schema. The two Antigravity app-log records were changed from `bespoke` to `documentation` ([antigravity.md](../../docs/research/signals/antigravity.md)), even though their existing `bespoke_rationale` still describes the detector that is required. The production engine intentionally ignores them, and `signals check` records them as tolerated `documentation_only` entries without replaying their evidence or failing ([signals.rs](../../cli/src/commands/signals.rs)).
- Evidence: `cargo run -q -p claudine-cli -- signals check --json` exited zero with `records: 85`, `positives_passed: 83`, `documentation_only: 2`, and `failures: 0`. The two omitted positives are `app_log-provider_version-language-server` and `app_log-auth_invalid-not-logged-in`.
- Impact: the catalog claims two operational signals that can never reach the normalized signal sink. More broadly, any future missing detector can now be relabeled `documentation` and still pass schema validation, codegen drift, unit tests, and the CI checker. This removes the feature's core mechanical guarantee that verified catalog behavior is shipped behavior.
- Verification level: the strongest verification is Level 1, which is the appropriate level for deterministic signal classification. It verifies the wrong behavior: `antigravity_app_log_records_are_documentation_only_and_never_fire` asserts that the records do not emit, while the checker test asserts this omission is non-failing. The requirement therefore has no positive verification at any level. Per the review rubric, that is a high-severity gap.

Fix direction: remove `DetectionMode::Documentation` from Rust and the sidecar, restore both records to `bespoke`, implement the Antigravity app-log detector/replayers through the shared sink, and require `positives_passed == records` for a successful fleet check. If these app-log observations are intentionally future research rather than shipped signals, move them to `gaps` instead of compiling them as detection records.

### Medium: the repaired exit payload lacks wrapper-path integration coverage

- Requirement: Antigravity stdout authentication diagnostics observed by the real wrapper must reach the exit-source detector.
- Implementation: the structured wrapper now retains the final stdout lines and passes them to the shared `exit_source_payload`, fixing the prior payload-shape mismatch ([spawn.rs](../../cli/src/commands/wrap/exec/spawn.rs)). The library test proves that a manually constructed payload fires both generated records.
- Gap: no CLI integration test spawns a fake Antigravity child, exercises `run_child_stream_semantic`, and verifies that the stdout ring, post-wait payload synthesis, signal hub, and summary/reporting path preserve the signal. The current test starts after the wrapper-specific capture logic, which was the broken seam in Review 1.
- Verification level: Level 1 is appropriate; no terminal emulator or OS input encoder is involved. The present Level-1 unit coverage is at the correct tier but does not cover the production orchestration boundary.

Fix direction: add a Level-1 CLI integration test using a fake `agy` executable that writes each authentication diagnostic to stdout and exits nonzero, then assert the wrapper's emitted summary contains `auth_invalid`. Cover more than ten stdout lines so the bounded-tail behavior is exercised as well.

## Requirement Verification Matrix

| User-facing requirement | Strongest verification found | Assessment |
|---|---:|---|
| Deterministic research-to-generated-provider catalog and committed drift detection | Level 1 generator/drift tests | Appropriate; `claudine-gen` passed 91/91. |
| Catalog-driven default model flag delivery | Level 1 unit/source tests | Appropriate; Review 1's static-fact finding is fixed. |
| Antigravity exit diagnostics on stdout emit `auth_invalid` | Level 1 helper/engine unit test | Correct tier, but missing wrapper-path integration coverage. |
| Every compiled signal record drives runtime detection and positive replay | Level 1 checker/unit tests | Wrong behavior verified; two records are intentionally excluded. |
| Terminal glyph/style/scroll behavior | Not applicable | This change does not add terminal-rendering semantics requiring Level 2. |
| Keyboard, hotkey, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement exists for this feature. |

## Verification Performed

- `cargo nextest run -p claudine-gen --color=never`: 91 passed.
- `cargo nextest run -p claudine -E 'test(/signals/)' --color=never`: 87 passed.
- `cargo run -q -p claudine-cli -- signals check --json`: exited zero, but only 83/85 positive records passed; two were reported `documentation_only`.
- `just test`: `claudine-catalog-types` passed 21/21; the `claudine` suite reached 799 passing tests before it was interrupted after exceeding the non-interactive command budget. No complete full-suite result is claimed.
- A targeted `claudine-cli` nextest run was also stopped during compilation after exceeding the command budget; the production CLI checker subsequently compiled and ran successfully.
