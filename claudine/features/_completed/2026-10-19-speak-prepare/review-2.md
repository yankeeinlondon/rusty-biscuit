---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-13T14:20:46-07:00
spec: 2026-10-19-speak-prepare/spec.md
implemented: false
description: "A **feature** review of `2026-10-19-speak-prepare/spec.md`"
feature: 2026-10-19-speak-prepare/review-2.md
previous: 2026-10-19-speak-prepare/review-1.md
---

# Speak Prepare review, iteration 2

The feature is **production ready**. Both high findings from review 1 are resolved. This review found no additional defect or verification gap that requires another repair cycle.

## Prior findings and repair log

Review 1 used a single Findings section; both findings were unblocked. It contained no blocked findings to reconsider. The repair log reports both fixed, with none deferred or disputed. Source inspection and the checks below support that report.

### ElevenLabs snapshots can reuse another account's default voice — resolved

The specification requires account separation and treats uncertainty as a cache miss. In biscuit-speaks, [the account-context calculation](../../../biscuit-speaks/lib/src/providers/cloud/elevenlabs/snapshot.rs) now requires an API-key source and a nonblank, caller-supplied account label. Without the label, neither snapshot publication nor snapshot lookup proceeds. The label must differ per account; credentials remain excluded from the identity and persisted record.

The repair follows review 1's recommended conservative fix. A fresh request without a label uses the existing helper to select the voice through the API, then reuses audio for that resolved voice. A returned prepared object still retains its settings. This preserves voice selection while making network-free default lookup conditional on a usable account identity. The README, caching topic, skill, and implementation log describe that condition.

The bounded class sweep checked the shared identity calculation, both production constructors, preparation and ordinary-synthesis snapshot publication, detached snapshot lookup, explicit-voice lookup, and retained settings. The production-constructor tests replace only transport and clock, so they exercise the repaired environment-derived identity.

| Site or path in biscuit-speaks | Shape checked | Result |
| --- | --- | --- |
| Shared identity through both production constructors | API key with absent, empty, or whitespace-only label | No trusted identity; no snapshot written or reused |
| Preparation followed by fresh detached lookup | Switch account keys under the same variable with no label | Miss without a network request in the requesting lookup |
| Snapshot writer and detached reader | Same labeled account and matching inputs | Ready cached file, without another API request |
| Same writer and reader | Change account label, API address, or credential-variable source | Miss for each change |
| Preparation and ordinary synthesis | Account voice list changes | Selection is resolved again and snapshot refreshed |
| Explicit-voice lookup | Requested voice and model | Uses the concrete audio identity without requiring a snapshot |
| Retained playback and helper handoff | Defaults change or prepared file disappears | Original settings retained; helper receives them on a miss |

### The new credential test depends on the developer's API-key environment — resolved

The specification requires pinned environment settings and hermetic Level 1 tests. In biscuit-speaks, [the ElevenLabs test fixtures](../../../biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs) now establish outer removal guards before overriding and dropping credential values. Dropping an inner guard restores controlled absence rather than the developer's original credential.

The sweep also checked the new account-switch tests, injected-context snapshot fixtures, host-provider fixtures, and detached-helper tests. The repair corrected a sibling in [the preparation integration tests](../../../biscuit-speaks/lib/tests/prepare.rs): the Kokoro/Echogarden fallback fixture now removes both credential variables and asserts the entire provider stack. This prevents an unexpected ElevenLabs fallback, including without `playa`.

| Incoming credential environment | Credential-exclusion test | Full preparation/fallback coverage |
| --- | --- | --- |
| Neither key variable set | Passed | Reviewed controlled removal guards |
| Primary key only, dummy value | Passed | Fallback fixture removes it |
| Alias key only, dummy value | Passed | Fallback fixture removes it |
| Both keys set to dummy values | Passed | Full suites passed with and without `playa` |

## Unblocked Findings

None.

## Blocked Findings

None. No human decision or manual test is required to close this review.

## Verification against the specification

The specification asks for **Level 1** verification: in-process tests and hermetic subprocess tests, with fake synthesis, playback, API transport, and clock boundaries. No requirement introduces keyboard handling, terminal rendering, or browser behavior requiring Level 2 or Level 3 verification.

| Acceptance criterion and user-facing requirement | Verification reviewed | Level |
| --- | --- | --- |
| 1 — no-op provider default and both feature configurations | Direct-speaker no-op tests; feature-enabled and feature-disabled compilation during nextest runs | L1 |
| 2 — sequential preparation creates reusable files without playback or spool publication | Recording executables for Say, Kokoro, gTTS, and Echogarden; fake ElevenLabs transport; file and spool assertions | L1 |
| 3 — returned and fresh objects reuse files through playback entry points and report actual settings | Prepared-file reuse tests, Say playback in both feature configurations, ElevenLabs cache/retained-setting tests, ready-job assertions | L1 |
| 4 — resolved defaults reuse correctly; unresolved defaults prepare nothing | Injectable Say resolver, production lookup fixtures, ElevenLabs gender/first/fallback tests, unresolved Echogarden fixtures | L1 |
| 5 — changed defaults/settings change identity; volume and baked speed behave correctly | Changed-default tests, synthesis-key tests, retained playback tests, speed and volume assertions | L1 |
| 6 — preparation errors propagate; playback preserves failover and helper settings | First-candidate failure, original-configuration fallback, specific-provider refusal, missing-file resynthesis, captured helper execution | L1 |
| 7 — preparation state, clone behavior, and builder invalidation | Builder state tests, no-op tests, volume retention, dry-run state retention | L1 |
| 8 — dry run precedes discovery and side effects | Builder/direct-provider tests, accepted-value tests, both feature configurations with ambient dry run set | L1 |
| 9 — failed/cancelled synthesis cleans up; concurrent publication leaves complete output | Staging cleanup, killed-child assertions, cancellation and concurrent prepare/retained-resynthesis tests | L1 |
| 10 — documentation describes current behavior | README, caching topic, affected skill pages, source comments, and repair-log departure inspected | Document review |
| 11 — snapshot trust, refresh, account isolation, and credential exclusion | Fake API/clock tests, persisted-record mutation matrix, production-constructor account tests, four incoming credential environments | L1 |
| 12 — package gates and dependent compilation | Passing package checks listed below | Local build/L1 checks |

Say's production normal rate deliberately remains unresolved. The specification explicitly permits this safe fallback when an authoritative lookup cannot be verified. The injected resolver proves resolved-rate behavior, while production tests and documentation preserve the no-op for unresolved normal speed. This review does not treat a mocked rate as proof of real system-rate support.

### Input robustness

The existing bounded matrices cover the feature's JSON read boundaries. The repair changes identity eligibility, not those grammars.

| Read boundary in biscuit-speaks | Load-bearing fields checked | Outcome and verification |
| --- | --- | --- |
| Persisted ElevenLabs snapshot | `voice`, `model`, `inputs`, `account_context`, `written_at`; nested `gender`, `language`, `model`, `fallback` | Public detached lookup misses for absent, null, wrong-type, empty, duplicate, mismatched, expired, future, and trailing-content mutations; an unedited provider-written record hits before and after the matrix |
| Say system-default JSON | `voice`, `rate` | Strict shape test rejects absent, null, wrong-type, empty/invalid, duplicate, and trailing content; provider-level fixtures verify unrecognized or unlisted defaults prepare nothing and play uncached |
| Detached preparation's new retained-settings field | `prepared` and its provider-specific settings | Absent remains the defined older/unprepared form; null, wrong types, missing settings, empty containers, unknown fields/providers, and multiple providers reject; valid settings round-trip and helper execution preserves them |

Collection-element mutations do not apply to the scalar snapshot or system-default fields. The helper receives an already-decoded map from Playa; duplicate-key handling in that pre-existing reader is unchanged, and broad cache-integrity hardening is outside the specification. No new permissive default or silently filtered field was introduced by the repair.

### Checks executed for this review

- `just test` in biscuit-speaks, with both credential variables and the account label set to dummy values and `PLAYA_DRY_RUN=1`: **574 passed, 20 skipped**.
- Feature-disabled L1 run using `cargo nextest run -p biscuit-speaks --no-default-features`, excluding higher-tier names, with the same dummy environment: **421 passed, 15 skipped**.
- Targeted credential-exclusion test with absent/primary-only/alias-only/both-key environments: **passed in all four cases**.
- `just lint` in biscuit-speaks: **passed**.
- `just test` in claudine: **8,738 passed, 9 skipped**.
- `just check-tier-coverage biscuit-speaks`: **zero stranded tests**.
- `git diff --check`: **passed**.

The selected preparation integration tests are compiled by Cargo's declared/default targets and selected by L1. Feature-specific detached tests ran with `playa` enabled. Cross-OS evidence remains governed by CI and is not a readiness condition for this review.

## Recurrence

None. The two classes raised in review 1 are resolved; this review raises neither again.

## Observations

None.
