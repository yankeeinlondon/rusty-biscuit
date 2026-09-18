---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/default
created: 2026-09-17T02:21:47-07:00
spec: 2026-09-15-initialize-after-proxy/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
next: 2026-09-15-initialize-after-proxy/review-2.md
description: A **fix** review of `2026-09-15-initialize-after-proxy/spec.md`
fix: 2026-09-15-initialize-after-proxy/review-1.md
findings:
    - "[high] Staged failure handlers execute shell commands before approval"
---

# Review 1: Initialize Before Body Discovery

The fix is **not production ready**. The separate bootstrap type, shared
Darkmatter frontmatter projection, stabilized reread, and retained caller
context address the original ordering defect. However, the new staged failure
path can execute unapproved lifecycle shell commands. This violates R2 even
though the provider correctly remains unlaunched.

No human decision is required to address this finding. Cross-OS execution
evidence is left to CI and does not affect this verdict.

## Findings

### [high] Staged failure handlers execute shell commands before approval

**Locations:**

- `claudine/cli/src/commands/wrap/composition/staged_boot.rs:101–106`
  approves only `LifecycleSignal::Initialize`.
- `claudine/cli/src/commands/wrap/composition/staged_boot.rs:372`
  routes a stabilized-read failure directly into `blocked` and `finalize`.
- `claudine/cli/src/commands/wrap/composition/pipeline.rs:1879`
  delegates those events to the stack executor without an approval gate.
- `claudine/lib/src/composition/lifecycle/executor.rs:273–290`
  explicitly assumes shell commands have already been approved; the production
  runner executes them through the system shell.

**Trigger:** a document has a non-shell initialization action, an unresolved
body include, and shell actions in its `blocked` or `finalize` stack. Run
`claudine compose doc.md --claude` without `-y`, with stdin closed and no
approval handler. Initialization succeeds, preparation fails on the include,
and both catch stacks run before the later full lifecycle audit is reached.

Minimal document, with the shell destinations replaced by absolute paths in a
temporary fixture:

```yaml
---
initialize:
  stack:
    - action: {ensure_file: marker.md}
blocked:
  stack:
    - action: {shell: "touch /TEMP/unapproved-blocked"}
finalize:
  stack:
    - action: {shell: "touch /TEMP/unapproved-finalize"}
---
::file ./missing.md
```

**Observed in this review:** an isolated probe using the existing
`target/debug/claudine` exited 1 with `TransclusionError` / `File not found`,
created **both** shell marker files, and never launched the fake provider.
The probe used a temporary repository, an empty fixture user configuration,
a cleared child environment, fixture-only provider lookup plus `/usr/bin:/bin`,
disabled audio/reporting, and closed stdin. Its temporary files were removed.
This is corroborating executable evidence; that existing binary was not
rebuilt successfully during this review. The current source independently
shows the same missing gate.

**Required correction:** gate every shell command reachable through an early
failure before dispatching it, using the same approval policy and invocation
cache. This must cover initialization errors, refused proxy handoffs, and
stabilized reread/audit/schema failures, including failure/finalize events
reachable from a catch evaluation error. An unavailable or denied approval
must not fall back into another unapproved shell action. Preserve ordinary
non-shell catch behavior and exactly-once event routing. Apply the correction
at a shared boundary so direct, loop, and adopted-target paths agree.

**Missing regression coverage (L1 is the appropriate level):** the existing
`a_file_still_missing_after_initialize_blocks_once_without_launching` test uses
`append_line` catch markers and passes `-y`; it cannot detect this bypass.
Add `CliProcessFixture` tests with non-shell initialization and shell catch
actions, without `-y`, asserting no shell effects and no provider launch.
Cover missing dependencies, initialization errors, and rejected handoffs;
include an approved counterpart that verifies each catch executes once.
The existing L2 initialization-denial test tests the initialization command
itself, not commands in its failure handlers.

## Requirement and verification-level assessment

These are filesystem, composition, approval, and lifecycle contracts. L1
in-process and real-binary fixture tests are appropriate for their semantic
assertions. Terminal-visible diagnostics additionally have L2 detached-tmux
coverage. No requirement here concerns physical keyboard encoding, so L3 is
not required. The gap is missing failure-path approval coverage, not a need
to increase the terminal tier of unrelated tests.

The test names below were inspected in source. Passing suite results are
recorded in [evidence.md](evidence.md); they are not fresh passing runs from
this review.

| Requirement | Present verification | Assessment |
|---|---|---|
| AC1: absent generated include | L1 `compose_initialize_staged_boot::{compose_initialize_creates_a_file_the_body_includes,a_proxied_target_initialize_creates_a_file_the_target_body_includes}`; L2 proxied generated-transclusion tests | Appropriate levels; checks generated content and initialization/provider order. |
| AC2: nested guards at phase 1 and later | L1 `the_shipped_router_reaches_the_reported_guarded_log_include_at_each_phase`; L2 both guarded-phase cases | Appropriate levels. |
| AC3: unconditional include | L1 direct/proxied staged-boot tests | Appropriate level; no conditional-discovery shortcut. |
| AC4: preservation and repeated invocation | L1 `ensure_file_preserves_an_existing_include_the_prompt_reads` and `a_second_invocation_reads_the_persisted_file_and_initializes_once` | Appropriate level; persisted bytes and event counts are asserted. |
| AC5 / R2: approval integrity | L1 initialization denial, generated include shell denial/approval, and false-condition discovery; L2 direct initialization denial | **Incomplete:** early catch shells bypass approval, as detailed above. |
| AC6: mutation visibility | L1 `an_include_initialize_rewrites_reaches_the_prompt_with_its_new_content` and `initialize_mutating_the_document_itself_is_prompted_and_audited_from_the_reread` | Appropriate level; checks new content and denial of an appended body command. |
| AC7: control flow and exactly-once initialization | L1 proxy-chain, stop, error, skip, retry, resume, and two-iteration loop cases | Appropriate level for control flow; shell approval on exceptional exits remains part of the finding. `stop` ends the stack and proceeds, while `skip` ends the run. |
| AC8 / R5: still-missing dependency | L1 `a_file_still_missing_after_initialize_blocks_once_without_launching` | Checks one typed diagnostic, ordered catches, and no provider. Its approved/non-shell setup misses the finding. |
| AC9: entry parity and sequence boundary | L1 direct/inline, proxy/inline-proxy, loop-owning target, harness adoption, and `sequence_initialize_include_preflight`; L2 sequence guidance | Appropriate levels; sequence static preflight remains the binding Option A contract. |
| AC10: shipped artifacts | L1 shipped schema/expression and body/hash drift guards, shipped router/target CLI fixtures retaining the reported spec argument; L2 shipped-route cases recorded in evidence | Appropriate levels; fixture-owned spec/plan data and fake providers are used. |
| AC11: unchanged entry reasons | L1 no-initialize eager failure, dry-run no-effects, retry fresh-content, and resume/no-second-initialize cases | Appropriate level. |
| AC12: reference identity | L1 explicit-relative, repository-root/scoped, implicit shadowing, all-filesystem-effects and repository-escape CLI cases; Darkmatter containment unit tests | Appropriate level for path semantics; this does not claim fresh cross-OS execution evidence. |

Darkmatter's `frontmatter_surface_projection` suite also tests untouched body
content, absent dependencies, frontmatter command approval, caller-disabled
operations, and continued rejection of unapproved commands in full compose.
The bootstrap tests exercise lifecycle stamping and approval-cache reuse.

The shipped prompt already uses `dirname`; the implementation correctly
repairs the empty-log instructions instead of changing `parent_dir` semantics.
The shared option assembly and distinct bootstrap result are useful design
choices. No separate performance or ergonomics change is required for this
review's readiness decision.

## Validation and scope

- Read the specification, binding design rulings, implementation inventory,
  acceptance evidence, changed preparation/dispatch boundaries, projection,
  filesystem containment changes, and relevant tests.
- Used GitNexus against this worktree and refreshed it with `just gitnexus`.
  The refresh completed successfully; it reported graph coverage limits, so
  source inspection was used to verify the relevant paths. Impact analysis
  for the spec metadata edit returned `UNKNOWN` with no resolved callers or
  processes; text references confirmed its plan/log links. No Rust symbol was
  edited.
- Attempted `just test --test compose_initialize_staged_boot --test
  compose_initialize_acceptance` from `claudine/`. Compilation stopped with
  exit 101 because shared-target `.rmeta` files were not writable. **No tests
  ran in that invocation.** Shared cache permissions were not changed.
- As an alternative, ran the isolated existing-binary probe described above
  and verified the corresponding source path. The earlier Phase 8 evidence
  records Claudine L1 7,258/7,258 and L2 231 CLI + 3 generator passes, and
  Darkmatter L1 7,937/7,937 plus its L2/lint passes. Those results do not cover
  the newly identified catch-shell case.
- This review changes only this report and `review_iterations: 1` in the
  spec. The fix remains active and is not marked completed. No implementation
  changes, formatting, commits, or terminal-focus actions were performed.
