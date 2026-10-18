---
$schema: feature-review.yaml
ready: false
findings:
    - "high: Deferred task setup and teardown phrases are never reconsidered"
    - "high: Task phrases are prepared with different settings from playback"
observations: []
human_review: false
human-in-the-loop: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-14T09:10:38-07:00"
spec: "2026-10-22-tts-prewarm/spec.md"
implemented: true
description: "A **feature** review of `2026-10-22-tts-prewarm/spec.md`"
feature: "2026-10-22-tts-prewarm/review-1.md"
next: "2026-10-22-tts-prewarm/review-2.md"
log: claudine/features/2026-10-22-tts-prewarm/log.md
implemented_by: claude/opus
---

# Review of TTS Pre-warm

The feature is **not production ready**. Two gaps affect sequence task speech: phrases skipped during preflight are not revisited when their inputs become available, and preparation can use a different provider and voice from the later speech action. Ordinary document lifecycle speech passed the corresponding controls.

## Findings

### high: Deferred task setup and teardown phrases are never reconsidered

**Authority: spec.** “Every `say`/`speak` phrase in every stack except `initialize`” explicitly includes task `setup:`/`teardown:`. The early/late rules also require: “Reconsider unresolved phrases there,” referring to ordinary document preparation after sequence preflight. Acceptance criteria 2 and 4 require preparation with established `ctx`/`env` snapshots and later discovery of newly resolved early phrases.

**Defect class:** candidates deferred because preparation inputs are unavailable have no submission point when task preparation establishes those inputs.

Claudine's [sequence preflight collector](../../lib/src/composition/lifecycle/prewarm/sequence.rs) correctly skips captured-context reads and unresolved frontmatter. However, [prepare_task_context](../../cli/src/commands/wrap/sequence/task_run.rs) establishes the task's context without submitting its stacks. The same omission exists in [open_member_run and open_leg_run](../../cli/src/commands/wrap/sequence/task_run.rs), which establish serial and parallel group-member contexts. Later prompt preparation walks the prompt document's lifecycle, not the owning task's setup and teardown.

Reproduction: copy the existing three-prompt CLI fixture, retain one prompt task, and add these guarded phrases to its setup or teardown. Set child environment `PREWARM_NAME=Ada`, use the existing pre-warm recorder, and launch `claudine sequence --goose seq.md` with a recording stub provider:

```yaml
project: Widget
derived: "{{ project }} ready"
sequence:
  - name: first
    prompt: doc.md
    teardown:
      - when: "false"
        then:
          - say: literal control
          - say: "task env {{ env.PREWARM_NAME }}"
          - say: "task ctx {{ ctx.today }}"
          - say: "task derived {{ derived }}"
```

The command succeeds and the literal control is prepared. None of the other three task phrases is prepared, even after the task runs and the provider finishes. Corresponding `env` and `ctx` phrases in `doc.md` are prepared. False guards keep the reproduction silent; the spec explicitly requires walking both conditional bodies without evaluating their guards.

The sweep used the same three dependent phrases and literal control at every supported task execution site:

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Top-level task setup | `env`, `ctx`, interpolated `derived` | All three missing; literal prepared | Submit all three once the task inputs exist |
| Top-level task teardown | Same | All three missing; literal prepared | Same |
| Serial group-member setup | Same | All three missing; literal prepared | Same |
| Serial group-member teardown | Same | All three missing; literal prepared | Same |
| Parallel group-member setup | Same | All three missing; literal prepared | Same |
| Parallel group-member teardown | Same | All three missing; literal prepared | Same |
| Prompt document lifecycle | Equivalent `env` and `ctx` reads | Both prepared with resolved values | Both prepared; clean control |
| Group container | Authored `setup` or `teardown` | Rejected before execution | Rejected by the existing grammar; no additional speech site |

**Recommended repair:** submit parsed task stacks when their captured context and effective frontmatter are established, including serial and parallel members. Reuse the invocation's handle, preserve task scope and sequence-injected exclusions, and keep initialization excluded. Add L1 CLI tests that assert resolved preparation records for setup and teardown after preflight skips them. The existing collection and classification tests prove individual helpers work; they do not prove deferred task stacks are submitted later.

### high: Task phrases are prepared with different settings from playback

**Authority: spec.** “Same config as playback” requires building candidates through `tts_config_from_settings` with the settings the later speech action uses. The de-duplication contract is “One preparation attempt per distinct (resolved text, playback TTS config).” This is necessary for the feature's promised cache reuse without changing playback.

**Defect class:** preparation chooses configuration for a different speech execution path and fixes that configuration for every candidate in the invocation.

Claudine's new [submit_preflight_prewarm](../../cli/src/commands/wrap/sequence/mod.rs) loads the bridged user settings. The [preflight task walk](../../lib/src/composition/lifecycle/prewarm/sequence.rs) supplies those settings for both prompt lifecycles and task stacks. Existing task playback in [run_step_task](../../cli/src/commands/wrap/sequence/task_run.rs) uses `GlobalSettings::default()`, inherited by group members. Meanwhile, [PrewarmWorker](../../lib/src/composition/lifecycle/prewarm/worker.rs) keeps one configuration from its first submission, so later submissions cannot supply the appropriate configuration for a different speech path.

With this ordinary user configuration:

```json
{ "tts": { "provider": "kokoro", "voice": "af_heart" } }
```

a literal `say: task done` in setup or teardown is prepared with the specific Kokoro provider and `af_heart` voice. Its event-time configuration instead selects the first available provider and has no requested voice. Preparation therefore cannot supply the cache entry that playback requests and may spend CPU or incur a cloud charge for unusable work. This finding concerns the newly added preparation mismatch; it does not require changing the task playback behavior that predates the feature.

L1 CLI probes recorded the configuration passed to preparation and to the speech publisher, replacing only audio handoff with a recorder so no speech played:

| Site | Shape tested | Observed preparation / playback | Expected result |
| --- | --- | --- | --- |
| Top-level task setup | Literal speech, explicit provider and voice | Kokoro + `af_heart` / automatic + no requested voice | Configurations match |
| Top-level task teardown | Same | Same mismatch | Configurations match |
| Serial group-member setup | Same | Same mismatch | Configurations match |
| Serial group-member teardown | Same | Same mismatch | Configurations match |
| Parallel group-member setup | Same | Same mismatch | Configurations match |
| Parallel group-member teardown | Same | Same mismatch | Configurations match |
| Compose document lifecycle | Same user config, literal success speech | Kokoro + `af_heart` / Kokoro + `af_heart` | Match; clean control |
| Sequence prompt document lifecycle | Same user config, literal success speech | Kokoro + `af_heart` / Kokoro + `af_heart` | Match; clean control |
| Group container | Authored setup or teardown | Rejected by grammar | No candidate required; clean |

**Recommended repair:** carry playback configuration with each queued candidate and use that same configuration in its de-duplication key. Keep the user's pre-warm enablement separate from the configuration of individual playback paths. For task stacks, preserve existing playback semantics and prepare against their actual settings. Add a mixed sequence test that checks task and prompt preparation against their respective publication configurations under an explicit provider and voice. The current key tests vary configuration fields, but the running queue never varies its configuration.

## Verification and requirement coverage

The reviewed implementation includes the existing uncommitted changes present when this review began. Temporary reproduction tests and recorder instrumentation were removed, restoring the touched source files byte for byte. No implementation repair was made.

- `just test` in `claudine/`: **9,138 passed, 9 skipped**.
- `PLAYA_DRY_RUN=1 just lint` in `claudine/`: **passed**. The environment value silences any recipe notification; it does not change Clippy checks.
- After restoring the probes, `just test-cli tts_prewarm::`: **10 passed**.
- `git diff --check`: **passed**.
- Six capture probes reproduced the first finding; six configuration probes reproduced the second, with a passing document control. Group-container probes established that those stacks are rejected rather than being additional affected sites.

The tests added by this feature are compiled: library tests are declared in-module, and CLI tests are included by the existing `l1` target. The CLI tests require Unix stub scripts and `test-fixtures`; that feature is enabled by the local recipe and CI metadata. Their names select L1. Cross-platform evidence remains CI's responsibility and does not block this review.

| Acceptance criterion | Strongest relevant verification | Assessment |
| --- | --- | --- |
| 1: all speech stacks, aliases, bundles, nested bodies; initialize excluded | L1 parsed-stack tests and CLI collection records | Covered |
| 2: early roots resolve; late roots and sequence inputs stay late | L1 classification tables, captured-value controls, CLI probes | Task captured-value gap in first finding |
| 3: shell/functions excluded, returned strings remain data, empty speech skipped | L1 authored-expression tables and resolution tests | Covered |
| 4: sequence preflight submits all eligible prompts without delaying launch; unresolved inputs reconsidered | L1 graph tests and recording CLI provider | Deferred task-stack gap in first finding |
| 5: proxy target uses its overlay when prepared | L1 CLI proxy test | Covered |
| 6: disabled setting and command dry run prepare nothing | L1 worker gates and CLI tests | Covered |
| 7: configuration shapes, compatibility, validation, TUI edits and save | L1 public loader matrix, hook/lifecycle config tests, TUI state and save tests | Covered; object defaults and switch survive edits |
| 8: errors remain best effort, one sanitized warning, later candidates continue | L1 failing preparer and captured tracing tests | Worker failure policy covered |
| 9: blocked preparation does not delay provider launch | L1 blocked preparer and recording CLI provider | Covered |
| 10: fixture dry run leaves no cache or spool artifacts | L1 production-path CLI fixture with private cache/temp locations, gate tests | Covered |
| 11: lifecycle/config documentation and skill updated | Direct document review | Present; update alongside the repairs above |
| 12: shared ownership and de-duplication across rereads and re-entry | L1 handle/key tests and CLI initialization/retry/proxy tests | Text reuse covered; differing playback configurations require second finding's repair |
| 13: event-time publication remains independent; no-op and cancellation behavior | L1 fake publisher and controllable preparation futures | Covered at the worker boundary |
| 14: package tests and lint pass | Canonical package recipes | Passed |

L1 is appropriate for these preparation, configuration, and publication contracts. The change introduces no new terminal input encoding requirement or rendering behavior needing L2/L3 verification. The detached-playback boundary was not changed, so the spec's conditional `just test-l2` requirement does not apply.

The input robustness matrix exercises the public configuration loader and bridge for `tts`, optional `provider`, and `prewarm`: control values, absence, explicit null, wrong types, empty supported shapes, duplicate keys, invalid trailing content, and unknown keys. JSON and JSON5 share the JSON5 loader. Explicit null is rejected for the newly optional provider and switch; duplicate-key handling remains the loader's established last-wins behavior. Existing voice/gender parsing was not changed into a new contract by this feature.

No significant unresolved design decision or human-only verification is needed for these repairs. The changes to optional-field storage and the serialized configuration key are routine implementation choices. No next-phase spec or plan was supplied to refresh.

## Observations

None.
