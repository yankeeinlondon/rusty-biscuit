---
$schema: feature-review.yaml
ready: false
findings:
    - title: ElevenLabs snapshots can reuse another account's default voice
      priority: high
    - title: The new credential test depends on the developer's API-key environment
      priority: high
observations: []
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-13T14:04:00-07:00
spec: 2026-10-19-speak-prepare/spec.md
implemented: true
next: 2026-10-19-speak-prepare/review-2.md
description: "A **feature** review of `2026-10-19-speak-prepare/spec.md`"
feature: 2026-10-19-speak-prepare/review-1.md
log: claudine/features/2026-10-19-speak-prepare/log.md
implemented_by: claude/opus
---

# Speak Prepare review, iteration 1

The feature is **not production ready** because a fresh ElevenLabs detached request can reuse speech selected for a different account, and a new test fails when API-key variables are already set. The remaining reviewed preparation, retained-setting, playback, and cleanup behavior follows the specification, including the permitted no-op when macOS's normal speaking rate cannot be resolved reliably.

## Findings

### High — ElevenLabs snapshots can reuse another account's default voice

**Authority: spec.** “ElevenLabs default resolution snapshot” requires “the account context needed to tell callers apart,” says that “any other uncertainty” is a cache miss, and states that the snapshot “never changes which voice is selected.” Acceptance criterion 11 requires a snapshot for a different account context to miss. Switching between personal and work API keys under the same environment-variable name is a realistic input: it can make the fresh detached call select the previous account's voice rather than the current account's first or gender-matched voice.

**Defect class:** a persisted default-selection record is trusted using an identifier that does not distinguish the accounts whose selections it records.

In biscuit-speaks, [account_context](../../../biscuit-speaks/lib/src/providers/cloud/elevenlabs/snapshot.rs:64), which supplies the identity used to store and trust ElevenLabs selections, combines the API address, credential environment-variable **name**, and an optional account label. With the label unset, two different accounts using `ELEVEN_LABS_API_KEY` produce exactly the same identity. [cached_path](../../../biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:536), the detached cache lookup, therefore accepts the previous account's snapshot and returns its audio file. Expiration limits this to 24 hours but does not make the selection correct during that window.

**Reproduction:** reuse the existing ElevenLabs `RecordFixture`, `FakeTransport`, and `FakeClock`. Set a dummy account-A key, leave `BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT` unset, and call the public provider `prepare` method with no requested voice and female gender. Its fake account lists `m1` and `f1`. Replace only the key with a dummy account-B key; construct a fresh provider whose fake account lists `account-b-female`, using the production context calculation and the same cache directories. At one second after preparation, call the public `cached_detached_job` method. It returns `Some(PlayFile)` for account A's prepared file and makes zero requests to account B. The expected result is a miss, allowing the helper to select account B's voice. The temporary nextest reproduction confirmed this and was removed after the review.

The bounded sweep covered both production constructors, the shared snapshot writer and reader, and their preparation and playback callers. Existing tests also exercise refreshed selections and explicit-voice lookup. Results:

| Site or path | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Shared identity calculation used by `new` and `with_base_url` | Different keys/accounts, same address and variable, no label | Identical context | Distinct identity, or uncertainty that disables snapshot trust |
| `prepare` → snapshot writer → fresh `cached_detached_job` | Account A prepares; account B requests matching text/settings | Ready job for A's voice, no B request | Miss; helper selects B's voice |
| Same writer and reader | Same account and matching settings | Ready job, no request | Ready job, no request; clean control |
| Same writer and reader | Change account label | Miss | Miss; clean |
| Same writer and reader | Change API address | Miss | Miss; clean |
| Same writer and reader | Change credential source from primary variable to its alias | Miss | Miss; clean |
| Preparation and ordinary synthesis selection | Change fake account voice list, resolve again | New selection; snapshot rewritten | New selection; clean, covered by existing selection/refresh tests |
| Explicit-voice detached lookup | Explicit voice, no default snapshot needed | Key uses requested voice/model | Same; clean, covered by existing test |
| Retained prepared settings | Default changes after preparation | Keeps original voice | Same; intentional and covered by existing retained-settings test |

The persisted JSON parser is not the problem. Its existing public-lookup matrix passes for absent, null, mistyped, empty, duplicate, and trailing-content mutations of the snapshot fields, including `account_context`, as well as nested selection inputs. Collection-element mutations do not apply to its scalar fields. Explicitly mismatched account strings correctly miss; the defect occurs before parsing, when two accounts are assigned the same valid string.

The plan acknowledges this limitation, but the specification still requires account separation. Prefer a small conservative fix: require a reliable non-secret account discriminator before trusting a default-selection snapshot, and treat its absence as uncertainty. Do not persist or hash credentials. Add the account-switch reproduction alongside the existing label/address controls. If supporting network-free default lookup without an account discriminator would require a larger identity mechanism, document that limitation and reconcile the specification rather than building an authentication subsystem for this feature.

### High — The new credential test depends on the developer's API-key environment

**Authority: spec.** The acceptance-criteria introduction requires hermetic L1 tests and says to “Pin provider inventories, default lookups, and environment settings.” Criterion 12 requires `just test` to pass. A developer who has configured either supported ElevenLabs API-key variable can run the new test and get a failure unrelated to the implementation's correctness.

**Defect class:** an environment-dependent test mistakes restoration of a previous value for removal of the variable.

In biscuit-speaks, [the_account_context_and_record_hold_no_credential](../../../biscuit-speaks/lib/src/providers/cloud/elevenlabs.rs:2272), the new test proving snapshot credential exclusion and credential-source precedence, uses `EnvGuard::set_safe` for both key variables. It later drops those guards and assumes the variables are absent. The toolkit correctly restores their original values instead. With an existing primary key, the alias-precedence assertion fails; with only an existing alias key, the final “no key, no records” assertion fails. No actual credentials or network access are needed to reproduce this.

Run the existing test with `cargo nextest run -p biscuit-speaks --features playa -E 'test(the_account_context_and_record_hold_no_credential)'`, changing only its incoming key environment to the dummy values below. All four cases were executed:

| Site | Incoming shape | Observed result | Expected result |
| --- | --- | --- | --- |
| Credential-exclusion/source-precedence test | Both key variables absent | Pass | Pass; clean control |
| Same test | Primary key set to a non-secret sentinel | Fail at alias-precedence assertion | Pass |
| Same test | Alias key set to a non-secret sentinel | Fail at final absence assertion | Pass |
| Same test | Both keys set to non-secret sentinels | Fail at alias-precedence assertion | Pass |
| Other new snapshot/selection tests using `RecordFixture` and injected account contexts | Both variables set to sentinels | Pass in the full-suite sweep | Pass; clean |
| New preparation, retained-setting, dry-run, and helper-handoff fixtures | Both variables set to sentinels | Pass in the full-suite sweep | Pass; clean |

The sweep also inspected every new environment guard and explicit guard drop within the feature's source/test diff. This test is the only new site that drops a credential guard and then asserts absence. The other new fixtures retain their guards or compare restored values with the captured originals.

Fix this locally: establish outer removal guards for both API-key variables before the existing inner override guards. Dropping an inner guard will then restore the controlled absent state; the outer guards restore the developer's environment at test completion. Keep the absent/primary/alias/both controls to demonstrate isolation. No production-code change is needed.

## Verification

The specification explicitly calls for hermetic **Level 1** verification. This API introduces no keyboard, terminal-rendering, or browser behavior that requires Level 2 or Level 3 evidence.

| Acceptance criterion | Strongest relevant verification reviewed |
| --- | --- |
| 1 — no-op trait default and feature configurations | L1 direct-speaker no-op tests; builds with and without `playa` |
| 2 — sequential preparation, reusable files, no playback/spool | L1 recording-executable tests for Say, Kokoro, gTTS, Echogarden; injected ElevenLabs transport |
| 3 — returned/fresh reuse and playback metadata | L1 fake playback/publication tests; Say tests in both feature configurations; ElevenLabs retained/cached job tests |
| 4 — resolved defaults and unresolved fallback | L1 injectable Say resolver, malformed lookup tests, ElevenLabs selection tests, unresolved Echogarden tests |
| 5 — changed defaults, synthesis keys, volume and speed | L1 changed-default, key-sensitivity, retained-settings, and baked-speed tests |
| 6 — preparation error and playback failover/helper handoff | L1 recording failures, original-configuration fallback, missing-file resynthesis, captured helper-record execution |
| 7 — preparation state and builder invalidation | L1 builder state, clone, no-op, volume retention, and dry-run tests |
| 8 — dry run before discovery | L1 builder/direct-provider tests and accepted-switch spelling tests, including builds without `playa` |
| 9 — cancellation, failure and concurrent publication | L1 staged-output failure/cancellation tests, killed-child assertions, and concurrent publication tests |
| 10 — current documentation | README, caching topic, provider/API/configuration skill pages, and affected source comments inspected |
| 11 — snapshot trust and refresh | L1 injected clock/transport and malformed-record matrix; account-label mismatch is covered, but actual key/account switching fails the reproduction above |
| 12 — package gates and dependent compilation | Local L1 suites, lint, feature-disabled compile, and tier-coverage check |

Local checks during this review:

- `just test` in biscuit-speaks: **571 passed, 20 skipped**.
- Same full suite with both API-key variables set to dummy sentinels: **570 passed, 1 failed, 20 skipped**; the only failure was the new credential-isolation test.
- `just lint` in biscuit-speaks: **passed**.
- `cargo check -p biscuit-speaks --no-default-features`: **passed**.
- Feature-disabled L1 nextest run with both key variables explicitly removed: **418 passed, 15 skipped**. The initial run with the incoming environment failed at the credential test described above.
- `just test` in claudine: **8,738 passed, 9 skipped**.
- `just check-tier-coverage biscuit-speaks`: **zero stranded tests**.
- Temporary account-identity sweep: **passed its assertions demonstrating the incorrect cross-account hit**, with label, address, and credential-source miss controls. No production source or permanent test change remains.

Cross-OS evidence is left to the existing CI policy and does not affect this readiness decision. This is the first review, so there is no recurring finding.

## Observations

None.
