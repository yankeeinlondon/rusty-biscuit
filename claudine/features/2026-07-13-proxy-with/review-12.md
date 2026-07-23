---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T13:28:25-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-12.md
previous: 2026-07-13-proxy-with/review-11.md
next: 2026-07-13-proxy-with/review-13.md
---

# Review 12: Proxy With

## Verdict

The feature is **not ready for production**, but the remaining blocker is no
longer runtime behavior or test rigor. The implementation closes all four
review-11 findings, and the relevant user-observable requirements now have the
required Level 2 evidence. No requirement in this feature depends on the
terminal emulator's keyboard encoder, so Level 3 is not applicable.

One medium-severity specification gap remains: the authoritative Claudine
architecture skill and the feature's acceptance map still describe superseded
behavior and evidence. The specification explicitly requires stale
documentation to be corrected. Production sign-off should wait until those
authoritative records agree with the implementation and the current test names.

## Findings

### 1. Medium: authoritative architecture and acceptance evidence still describe the pre-fix system

The specification's Documentation section requires stale descriptions of the
reduced harness path and transition behavior to be corrected
(`spec.md:890-906`). That is not complete.

`.claude/skills/claudine/architecture.md:475-483` still labels target launch
inputs and loop recognition as a **Known gap**, claiming they are computed once
from the originally invoked document. The current implementation and the
adjacent composition skill say the opposite: composition handoffs re-enter the
command-owned pipeline, and retry/resume rebuild their attempt launch identity.
This is an authoritative skill document, so a maintainer following it receives
the wrong architecture and may design another workaround for a gap that no
longer exists.

The acceptance map has the same drift. Its AC10 and AC15 rows
(`notes/acceptance-map.md:371-376`) omit the review-11 closure evidence, and AC15
still cites
`target_launch::tests::body_mcp_tags_are_lexed_from_disk_and_only_when_mcp_is_enabled`,
a test name that no longer exists because disk lexing was the defect. The
replacement evidence is
`body_mcp_tags_come_from_the_prepared_document_and_only_when_mcp_is_enabled`,
`a_vanished_or_rewritten_source_cannot_erase_the_prepared_mcp_tags`, and the new
Level 2 interpolated-tag retry/resume rows.

Remove the obsolete **Known gap** paragraph and update AC10/AC15 with the
review-11 evidence: initial file-backed system-prompt lifetime, unavailable
provider selection, prepared MCP tags, and rebuilt-provider warnings. This is a
documentation/evidence correction; the code is already the authority where the
two disagree.

## Review-11 closure

All four prior findings are closed:

1. `CommandPhase::system_prompt_artifacts` now owns initial file-backed system
   prompts through `provider_run_handoff` and child exit
   (`composition/pipeline.rs:118-149`, `1467-1502`). L1
   `direct_compose_keeps_its_file_backed_system_prompt_readable_at_spawn` and L2
   `level2_lifecycle_direct_compose_delivers_a_readable_system_prompt_file`
   make fake Gemini and Codex children read the referenced bytes.
2. `select_rebuilt_provider` sends refreshed scalar/list hints through the same
   non-TTY selector as direct execution, and `resolve_binary_for` no longer
   substitutes a bare executable (`target_launch.rs:469-549`). L1 covers scalar,
   zero-runnable-list, and later-runnable-list cases; L2 compares the rendered
   direct/retry diagnostic and proves no unavailable child starts.
3. `MaterializedHarnessPrompt::mcp_body_tags` captures sorted tags from the
   canonically composed body before resume follow-up substitution
   (`harness_orch/prompt.rs:217-252`, `target_launch.rs:324-353`). L1 proves a
   missing or rewritten source cannot erase the set; L2 observes retry injection
   and resume refusal for tags that exist only after interpolation.
4. `LaunchPlan` carries structured model/output/system-prompt/sandbox warnings,
   and `emit_launch_warnings` applies the command's normal/quiet/silent policy
   (`launch_plan.rs:590-675`, `loop_control.rs:1210-1230`). L2 compares normal
   pane warnings against direct execution for unsupported system-prompt and
   sandbox requests; L1 pins quiet/silent suppression.

## Verification-level audit

| User-observable requirement | Strongest evidence | Required level | Result |
|---|---|---|---|
| Direct and proxied target prompt/frontmatter/context are equivalent | Level 2 direct/proxy tmux matrix | Level 2 | Present |
| Target lifecycle order, one `initialize`, clean source handoff, and loop ownership | Level 2 lifecycle-order and direct/proxy loop rows | Level 2 | Present |
| Provider/model/profile/binary/argv/environment/interactivity/dispatch/CWD rebuild from the active target | Level 2 child launch recorders and pane capture | Level 2 | Present |
| MCP server selection and injection, including composed tags across retry/resume | Level 2 child argv/injection rows and pane refusal row | Level 2 | Present |
| Initial and replay-created file-backed system prompts remain readable at spawn | Level 2 fake-child file reads, backed by L1 lifetime seams | Level 2 | Present |
| Unavailable refreshed provider matches direct typed selection and starts no child | Level 2 direct-versus-retry diagnostic/no-spawn row | Level 2 | Present |
| Rebuilt-provider capability warnings match direct execution | Level 2 pane comparison for system prompt and sandbox | Level 2 | Present |
| Shell discovery, approval/denial, and approved bytes equal executed bytes | Level 2 target-policy and shell-byte rows | Level 2 | Present |
| Overlay precedence, schema/computed-property behavior, retry/resume/loop lifetime, and chain forwarding | Level 2 end-to-end rows with L1 typed-state coverage | Level 2 for effects; L1 for pure state | Present |
| Typed failure identity and rendered diagnostic are route-equivalent | Level 2 three-route diagnostic matrix | Level 2 | Present |
| Inline closure ownership, sequence-step containment, stdout/stderr routing, and dry-run non-traversal | Level 2 composition-command rows | Level 2 | Present |
| Overlay values are absent from rendered status while remaining consumable | Level 2 tmux pane capture | Level 2 | Present |
| OS keyboard/modifier encoding | No such requirement | Level 3 | Not applicable |

The Level 2 matrix is Unix/tmux by the ratified platform policy: Linux CI and
macOS opt-in. Windows carries platform-neutral Level 1 proxy coverage because
the repository does not provide a Windows Level 2 terminal harness. The spec
documents this limitation, so it is not presented as cross-platform Level 2
sign-off.

## Validation performed

- Source inspection traced the review-11 implementation commits through command
  construction, canonical materialization, launch rebuilding, warning emission,
  and child launch.
- GitNexus confirmed `materialize_attempt_prompt_phase` is the production caller
  of `rebuild_launch_identity`; the rebuild has no alternate production caller
  that bypasses prepared MCP tags or refreshed selection.
- `git diff --check 1d25ca37f..HEAD` passed for Claudine source, tests, feature
  documents, and the Claudine skill.
- The error-transport and lifecycle-document guards passed.
- A full `just test-cli` and a focused system-prompt lifetime run were attempted,
  but this clean worktree was still compiling the cold dependency graph at the
  repository's approximately 60-second non-interactive limit. Both were stopped
  with exit 130 before any test executed; this review does not count them as
  passing evidence. The Level 2 tier was not started because it depends on the
  same unfinished build.
- `cargo fmt --check --manifest-path claudine/cli/Cargo.toml` could not run
  because the pinned stable toolchain lacks the `rustfmt` component. No
  formatting write command was run.

## Production readiness

`ready: false`. Runtime implementation and verification levels are sufficient,
but the specification's required authoritative documentation is still
internally contradictory. Correct the architecture skill and acceptance-map
evidence, then rerun the normal Claudine L1/L2 gates from a warm build before
production sign-off.
