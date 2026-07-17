---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T13:36:20-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: false
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-4.md
previous: 2026-07-13-proxy-with/review-3.md
---

# Review 4: Proxy With

## Verdict

The feature is **not ready for production**. The `with:` surface, typed overlay
evaluation, staged target bootstrap, target reread, redacted handoff status, and
the tested initialize-proxy scenarios are working. The complete local Level 1,
Level 2, and lint gates pass.

The defining equivalence contract is nevertheless still incomplete. A proxy
target projects the already-selected provider and a recomputed model into its
identity environment, but does not rebuild its full launch plan; routing is
split between two coordinators and two hop/cycle
ledgers; and complete resume compatibility is inferred from frozen launch state
and covered only by extraction tests. Several user-observable requirements
therefore have either incomplete Level 2 evidence or only Level 1 evidence.

## Findings

### 1. Critical: the target still executes with most of the router's launch plan

`target_launch.rs` explicitly limits its rebuild to provider/model identity
projected into `AGENT`, `MODEL`, and `YOLO`, and says profile/binary, argv
entrypoint, MCP injection, system-prompt delivery, child CWD, and provider
switching are not rebuilt (`cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs:10-26`).
The function confirms that scope: it returns only environment overrides and a
prepared context (`target_launch.rs:57-103`). The attempt executor continues to
read provider, profile, binary, CWD, argv, base environment, structured mode,
output behavior, and dispatch context from the immutable router-owned
`state.run` (`cli/src/commands/wrap/harness_orch/loop_control.rs:1046-1071`).

This directly violates R3, R6, and acceptance criterion 10. A target-selected
provider can still run the router's provider binary and protocol, and target
MCP/system-prompt/argv/CWD changes cannot become the process launch that direct
invocation would produce.

The strongest test is Level 2, but it covers only `MODEL` while both arms pin
`--goose`; the fixture itself says provider movement is intentionally excluded
(`cli/tests/level2_lifecycle_control.rs:2383-2387,2433-2441`). The other tests
mapped to AC10 verify launch-area file resolution and invocation-owned
stdout/stderr routing, not the missing target-owned launch facets. Required
Level 2 direct/proxy rows for provider/profile/binary, MCP injection, system
prompt, argv, effective child environment/CWD, interactivity/structured mode,
and dispatch configuration do not exist.

The fix should make a complete prepared launch bundle active-document-owned and
rebuild it through the same canonical preparation service used by direct
execution. Adding more environment projections to `target_launch.rs` would
continue the split the specification was written to remove.

### 2. Critical: proxy ownership and hop/cycle accounting are split across two coordinators

The command path creates an invocation ledger in `compose/prep.rs:204-211` and
commits surfaced handoffs through it at `compose/prep.rs:415-424`. The provider
harness independently constructs `ActiveDocumentCoordinator`, which constructs
another `RunLedger` (`cli/src/commands/wrap/harness_orch/loop_control.rs:217-248`;
`loop_control/coordinator.rs:47-59`). This is not R1's one coordinator above both
the document loop and provider-attempt harness, and it resets the visited chain
and hop count when routing crosses the boundary.

Which coordinator receives an initialize proxy depends on an early raw-file
peek. Only a non-dry-run, non-sequence initialize proxy whose target already
declares `loop:` is hoisted (`cli/src/commands/wrap/composition/pipeline.rs:1403-1439`).
The peek resolves and reads the target, applies only the handoff overlay, and
calls `resolve_loop_config` before target initialize and full canonical
preparation (`pipeline.rs:1443-1476`). Consequently:

- a terminal-recovery proxy to a looping target stays in the harness and does
  not acquire the target's document loop;
- a sequence-step proxy is explicitly kept in the harness even when its target
  loops;
- a target whose initialize preparation introduces or changes `loop:` is
  classified from the stale pre-initialize file; and
- a handoff hoisted by the outer coordinator can later enter a fresh inner
  ledger, so a proxy back to an invocation ancestor is not checked against the
  complete chain.

The Level 2 loop tests cover only an initialize router and a target with an
already-authored loop. The Level 2 sequence test uses a non-looping target, and
the terminal-recovery test is also non-looping. Thus AC2, AC3, AC7, and the
invocation-wide part of AC16 are not established at the required level.

Remove `proxy_target_declares_loop` as a routing decision. Every proxy producer
should return the same transition to one command-owned coordinator and ledger;
that coordinator should prepare the adopted target fully and then decide loop
ownership from the stabilized prepared document.

### 3. High: resume compatibility does not represent the actual prepared launch and has no Level 2 refusal test

`session_compat_key` is computed from the same frozen `state.run` fields used by
the attempt executor (`loop_control.rs:1088-1118`). Because finding 1 prevents
the target launch bundle from being rebuilt, comparing those fields can report
compatibility simply because both attempts retained the same stale router
state.

The remaining facets are reconstructed heuristically from argv and environment:
MCP identity is documented as “best-effort,” while the provider-specific
`extra` map is always empty (`cli/src/commands/wrap/harness_orch/session_key.rs:73-75,123-157`).
System-prompt extraction also tries to read every inline value as a path
(`session_key.rs:95-115`); an inline prompt such as `README.md` is therefore
hashed as that file's contents when the path exists, even though the provider
receives the literal string. This can produce either a false incompatibility or
a false match. The helper also uses `DefaultHasher` instead of the monorepo's
`biscuit-hash` authority (`session_key.rs:160-164`).

The strongest incompatibility evidence is Level 1: unit tests call the key
extractor with manufactured fields. Level 2 verifies a compatible happy-path
resume and the follow-up message, but no real run mutates a launch facet between
attempts and verifies that the provider is not relaunched with the session and
that the typed, facet-naming retry recommendation renders. This is the exact
wrong-level mismatch called out by the review instructions and leaves AC15
unverified.

Build the key from typed fields on the final prepared launch plan rather than
scraping argv. Then add Level 2 resume-refresh cases for every non-renegotiable
facet and assert both non-launch and the rendered diagnostic.

### 4. High: required user-observable Level 2 coverage and CI enforcement remain incomplete

The local Level 2 suite is valuable and green, but the acceptance map's “30 of
30” statement overstates its rigor. In addition to the gaps above:

- AC6 inline proxy closure ownership has only an in-process ownership test;
  Level 2 covers dry-run and sequence containment, not the final rewritten file;
- AC17's exact approved-bytes-equal-executed-bytes assertion is Level 1; Level 2
  proves blacklist refusal, not equality through a real terminal run;
- AC26's overlay retention across retry, resume, and loop refresh is Level 1;
  Level 2 covers only multi-hop forwarding/omission; and
- the required Level 2 rows for target-specific provider/MCP, resume
  incompatibility, looping terminal/sequence handoffs, and library-loop routing
  are absent.

These are user-observable process selection, file output, shell execution, and
iteration behaviors, so Level 1 state tests are insufficient. Level 3 is not
required: this feature has no requirement whose correctness depends on a real
terminal's keyboard encoder.

There is also no continuous Level 2 gate. `claudine-tests.yml` runs only the
four Level 1 recipes on Ubuntu (`.github/workflows/claudine-tests.yml:38-58,111-113`).
The reusable `_area-ci.yml` provides the sanctioned Linux `just test-l2` job,
but Claudine does not call it. The spec now promises Linux CI plus macOS opt-in,
so the missing Linux job is a release-gate gap, not merely a platform expansion.

### 5. Medium: the sign-off artifacts contradict the implementation

`notes/acceptance-map.md:7-16` declares all 30 criteria mapped and describes R6
and AC15 as resolved. Its AC10 row maps model, file resolution, and output
routing to the entire launch contract, while `target_launch.rs` explicitly says
the other launch facets remain structural work. Its AC15 row maps Level 1 key
projection tests to the user-facing runtime refusal required by the spec.

The model Level 2 test also retains a stale “Why this is `#[ignore]`d” section
even though it is enabled (`cli/tests/level2_lifecycle_control.rs:2412-2444`).
Update the plan, acceptance map, and test documentation after the architecture
and missing tests land; they should not be used as production sign-off in their
current form.

## Verification-level audit

| User-observable requirement | Strongest present | Assessment |
|---|---:|---|
| Initialize proxy bootstrap, target initialize/reread, lifecycle order, overlay layering and redaction | Level 2 | Appropriate for the tested initialize route |
| Target loop ownership and iteration equivalence (AC7) | Level 2 | Incomplete: authored initialize-target loop only; terminal, sequence, dynamic loop, and complete cycle-chain routes are missing |
| Target launch equivalence (AC9/10) | Level 2 | Incomplete: model-only launch probe; most required launch facets remain frozen and untested |
| Retry/resume happy paths and follow-up delivery | Level 2 | Appropriate for the compatible path |
| Resume incompatibility and facet-naming diagnostic (AC15) | Level 1 | **Mismatch:** requires Level 2 runtime and pane evidence |
| Inline closure ownership (AC6) | Level 1 | **Mismatch:** final rewritten target is user-visible and requires Level 2 |
| Shell gating/denial | Level 2 | Appropriate for refusal behavior |
| Approved bytes equal executed bytes (AC17) | Level 1 | **Mismatch:** requires Level 2 end-to-end evidence |
| Overlay retention across retry/resume/loop refresh (AC26) | Level 1 | **Mismatch:** requires Level 2 provider/lifecycle observation |
| Parser, interpolation, type preservation, merge/null semantics (AC18-25) | Level 1, with selected Level 2 rows | Level 1 is appropriate for pure semantics; the selected process-visible layering rows are Level 2 |
| Overlay non-disclosure in rendered status (AC30) | Level 2 | Appropriate pane-text evidence |
| OS keyboard/input encoding | None | Not applicable; no Level 3 requirement in this feature |

## Validation performed

- `just test` — passed: catalog types 21, library 3,527, contract 47, CLI
  2,041, and generator 152 tests. One CLI root-help isolation test failed its
  first attempt and passed nextest's configured flaky retry.
- `just test-l2` — passed all 144 tests; three lifecycle tests were slow.
- `just lint` — passed for the complete Claudine package area.

The green gates validate the scenarios currently represented. They do not
exercise the missing routes and launch facets identified above.
