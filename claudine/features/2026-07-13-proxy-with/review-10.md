---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T22:04:32-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-10.md
previous: 2026-07-13-proxy-with/review-9.md
---

# Review 10: Proxy With

## Verdict

The feature is **not ready for production**. The review-9 work closes its five
reported items: retry launch adapters now follow the rebuilt provider, stale
provider-generated argv/environment values are patched, incompatible resume
uses the required lifecycle tail, AC25 has Level 2 coverage, and the shipped
`implement` route is tied to a real-terminal fixture plus a drift guard.

Four new high-or-higher runtime gaps remain in the same re-entry boundary. The
opening provider still determines which ambient credentials survive sanitation;
interactivity environment markers remain stale; terminal recovery after a
provider-switch attempt consults the opening provider; and provider-switch
system-prompt temp files are dropped before the child starts. Two acceptance-map
tests also fail on a clean host because they depend on ambient OpenCode model
configuration.

## Findings

### 1. Critical: provider-switch re-entry applies the opening provider's credential policy

R6 and AC10 require the effective child environment to be rebuilt for the
active document (`spec.md:492-505`, `spec.md:934-937`). The command pipeline
sanitizes the process environment once using the opening profile's
`allowed_env_keys()` (`cli/src/commands/wrap/composition/pipeline.rs:416-428`,
`cli/src/commands/wrap/env/mod.rs:263-271`). It then captures
`pre_provider_env` only *after* that sanitation
(`cli/src/commands/wrap/composition/pipeline.rs:463-478`). The replay baseline
therefore describes provider-generated mutations, not the ambient values the
opening sanitizer admitted or removed
(`cli/src/commands/wrap/composition/pipeline.rs:1009-1027`).

This fails in both directions:

- Goose admits no provider credential variables
  (`lib/src/provider/goose/data.rs:254`), so Goose → Codex cannot recover ambient
  `OPENAI_API_KEY` or `CODEX_API_KEY`, even though Codex admits them
  (`lib/src/provider/codex/data.rs:273`).
- Codex → Goose keeps those ambient credentials in the invocation base. The
  replay has no record that Goose's direct-launch sanitizer would remove them,
  so secrets can be exposed to a provider that should never receive them.

The existing Level 2 retry environment row checks generated values such as
model and OpenCode configuration, not provider allowlist changes. Fix this by
retaining invocation-neutral environment inputs and rerunning the same sanitizer
for the rebuilt profile (while preserving explicit `--include` intent), or by
constructing a target-specific patch from an unsanitized, immutable snapshot.
Add bidirectional Level 1 allowlist rows and Level 2 fake-provider rows that
inspect the actual spawned child's environment using non-secret fixture values.

### 2. High: retry/resume can launch with stale interactivity environment markers

The initial environment stamps both `INTERACTIVE` and `CLAUDINE_INTERACTIVE`
from the opening session mode (`cli/src/commands/wrap/env/mod.rs:293-301`,
`cli/src/commands/wrap/env/mod.rs:320-330`). A fresh-read rebuild correctly
derives `non_interactive`, argv, and structured-output behavior from the active
document, but `launch_env_overrides` updates only `AGENT`, `MODEL`, and `YOLO`
(`cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs:489-502`).
The child therefore receives refreshed argv/streaming behavior alongside stale
interactivity variables from the invocation base.

This is user-observable: `CLAUDINE_INTERACTIVE` gates hook behavior, and wrapped
providers and downstream processes consume `INTERACTIVE`. The current Level 2
launch-bundle equivalence row covers a proxy target, where the outer coordinator
reruns the pipeline; the provider-switch retry rows do not assert these two
environment values in both directions. Include both markers in the per-attempt
bundle and add Level 2 retry rows that capture the actual child environment for
non-interactive → interactive and interactive → non-interactive refreshes.

### 3. High: terminal recovery after a provider-switch attempt uses the opening provider

The attempt itself launches from `rebuilt.provider` and `rebuilt.profile`
(`cli/src/commands/wrap/harness_orch/loop_control.rs:1456-1484`). However,
`ExecutedHarnessAttempt` does not retain either value
(`cli/src/commands/wrap/harness_orch/loop_control.rs:374-378`), and
`classify_attempt_phase` reloads both from invocation-fixed `state.run`
(`cli/src/commands/wrap/harness_orch/loop_control.rs:1689-1708`). Those stale
values are passed to terminal recovery on both failure and success
(`cli/src/commands/wrap/harness_orch/loop_control.rs:1795-1815`,
`cli/src/commands/wrap/harness_orch/loop_control.rs:1933-1953`). Resume admission
then asks that profile whether it supports resume
(`cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:143-155`).

After Goose → Codex, a successful second attempt can consequently reject a
subsequent `resume` using Goose's capability even though the live session belongs
to Codex. The reverse direction can admit an action under the wrong profile and
fail later. Existing Level 2 provider-switch tests end after the switched
attempt; none chains a success/failure control action from that attempt. Carry
the executed attempt's provider/profile into classification and use them for all
terminal control and diagnostics. Add Level 2 switch → resume rows in both
capability directions.

### 4. High: provider-switch system-prompt temp files are deleted before spawn

When the provider changes, launch-plan replay reapplies system-prompt delivery
and deliberately stores its RAII artifacts on `LaunchPlan`
(`cli/src/commands/wrap/launch_plan.rs:480-510`,
`cli/src/commands/wrap/launch_plan.rs:568-575`). File-backed delivery creates a
`NamedTempFile` and places its path in argv or the child environment
(`cli/src/commands/wrap/system_prompt.rs:223-252`,
`cli/src/commands/wrap/system_prompt.rs:267-272`).

`rebuild_launch_identity` consumes the plan but copies only argv/environment and
other scalar fields into `RebuiltLaunchIdentity`
(`cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs:302-314`,
`cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs:391-407`). It
does not carry `plan.system_prompt_artifacts`. Dropping the plan therefore
deletes the file before `build_harness_launch` spawns the child. This breaks, for
example, retry into Gemini's environment-file delivery and Codex's replacement
config-file delivery.

The launch-plan unit test keeps the plan alive and cannot catch the production
lifetime loss; the existing Level 2 system-prompt equivalence row exercises a
proxy handoff, not a retry that switches provider. Keep the artifacts in the
rebuilt attempt bundle through child exit. Add a Level 1 lifetime test at the
`rebuild_launch_identity` seam and a Level 2 provider-switch test whose fake
Gemini/Codex process reads the referenced file.

### 5. High: two acceptance-map launch-identity tests are not hermetic and fail

The acceptance map cites
`frontmatter_interactive_moves_the_mode_and_structured_output` and
`the_permission_mode_records_what_yolo_achieved_not_what_was_asked` as Level 1
evidence for AC15. Both select OpenCode without supplying a model
(`cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs:406-454`).
On this host, `just test-cli 'target_launch::tests'` ran 17 tests and repeatedly
failed those two at launch-plan construction with “No model specified”; 15
tests passed.

Tests that prove required compatibility facets cannot depend on a developer's
OpenCode configuration. Give the fixture an explicit valid model or inject a
deterministic catalog/config input, then verify the targeted suite and the full
`just test` gate. Until then, the acceptance map's cited evidence is not passing
evidence.

### 6. Medium: the acceptance map overstates its current evidence

The map's header says review 8 is current
(`notes/acceptance-map.md:5-7`), while the body incorporates only selected
review-9 changes. Its AC25 row still cites only the three Level 1
`overlay_layering` tests (`notes/acceptance-map.md:273`), despite the new Level 2
overlay-installed shell rows. It also marks AC15 complete using the two failing
tests above.

Refresh the map after the runtime and test fixes. Each row should name the
strongest passing verification level, and the 30/30 headline must be derived
from that corrected evidence rather than retained while required tests fail.

## Verification-level audit

| User-observable requirement | Strongest verification present | Required level | Result |
|---|---|---|---|
| Basic direct/proxy launch-bundle equivalence | Level 2 tmux matrix | Level 2 | Present |
| Retry switches argv, generated env, and execution adapters | Level 2 retry rows | Level 2 | Present for the review-9 cases |
| Provider-specific credential sanitation after a switch | No provider-switch allowlist row | Level 2 child-env observation | **Gap — finding 1** |
| Refreshed retry interactivity reaches child env/hooks | Level 1 derivation; no retry child-env assertion | Level 2 child-env observation | **Gap — finding 2** |
| Recovery actions after a switched attempt use that attempt's provider | No chained provider-switch → control row | Level 2 | **Gap — finding 3** |
| File-backed system prompt survives provider-switch retry | Level 1 launch-plan test only | Level 2 child read | **Gap — finding 4** |
| Resume incompatibility names changed reachable facets | Level 2 refusal matrix; two supporting Level 1 tests fail locally | Level 2 plus passing Level 1 projections | **Gap — finding 5** |
| Control-plane overlay shell policy | Level 2 tmux rows | Level 2 | Present; map is stale |
| Shipped `implement` route matches direct execution | Level 2 tmux fixture plus Level 1 drift guard | Level 2 | Present |

No requirement in this feature depends on the terminal emulator's keyboard
encoder, so Level 3 OS keyboard injection is not applicable.

## Validation performed

- `just test`: `claudine-catalog-types` (21 passed), `claudine` (3,531 passed),
  and `claudine-contract` (47 passed) completed. The area-wide command was
  stopped under the non-interactive duration rule during `claudine-cli`, after
  313 additional tests had passed; the remaining CLI/gen tests were not run by
  that command.
- `just test-cli 'target_launch::tests'`: 15 passed, 2 failed as described in
  finding 5.
- `just test-l2 'level2_lifecycle_retry_'`: the nine selected tests could not be
  executed because the sandbox denied tmux socket access (`Operation not
  permitted`). This is an environment limitation and is not counted as a test
  failure against the implementation.
- Source inspection traced the environment, launch-plan, attempt execution, and
  terminal-recovery paths end to end. The GitNexus index was current for the
  reviewed branch and confirmed the relevant symbol relationships.

## Production readiness

`ready: false`. The credential-policy defect alone is release-blocking, and the
three additional runtime defects each violate R6/R8 on reachable retry/recovery
paths. Production readiness also requires passing hermetic Level 1 evidence and
new Level 2 rows for the uncovered child-visible behavior.
