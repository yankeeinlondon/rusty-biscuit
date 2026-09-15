---
created: 2026-09-15
status: proposed
reviewed: false
implemented: false
area: claudine
packages:
    - claudine
    - claudine-cli
---

# Run Proxy Target Initialization Before Body Discovery

## Problem

A proxied prompt cannot reliably create files during `initialize` for later
composition. Command preparation scans the target's transclusions before its
initialization runs. When an included file does not exist yet, that scan fails
and the action intended to create it never executes.

This violates the staged initialization contract already expressed by the
harness: approve initialization's shell commands, run initialization, reread
the document, and then audit and compose the resulting document.

## Reported Failure

From the repository root:

```sh
claudine compose prompts/implement.md spec='fixes/2026-09-14-cicd-improvements/spec.md' -y --claude --model opus && compose prompts/review.md spec='fixes/2026-09-14-cicd-improvements/spec.md' -y --codex --model gpt-5.6-sol
```

The implementation router reads `implemented: false` and redirects to
`prompts/_implement/implement-plan.md`. Claudine prints the target's header and
then fails:

```text
TransclusionError: I/O failure
File not found: 2026-09-14-cicd-improvements/implementation-log.md
```

The user restored this action at the beginning of the target's initialization
stack and reported the same failure:

```yaml
initialize:
    stack:
        - action:
            - ensure_file: "{{log}}"
```

The body includes the log inside nested conditions:

```markdown
::block when="file_exists(log)"
::block when="phase > 1"
::file {{log}}
::end-block
::end-block
```

The plan starts at phase 1. The review command after `&&` is not reached.

### Separate prompt defect

The current `log` expression uses `parent_dir(spec)`, which returns only the
parent directory's name. It drops the preceding `fixes/` component.
`dirname(spec)` is the appropriate directory-path operation here. Correcting
that expression alone does not repair the initialization ordering: even the
correct log path can be absent until `ensure_file` runs.

The regression must therefore also be demonstrated with an unambiguous valid
path, so success cannot be attributed solely to repairing the shipped prompt.

## Evidence and Root Cause

The failure is user-reported and the ordering is confirmed by current source
inspection. An isolated executable reproduction and the first regressing
commit have not yet been established. GitNexus located the relevant symbols;
its index was reported six commits behind during follow-up investigation, so
the conclusions below were checked against the working-tree source.

- `claudine/cli/src/commands/compose/prep.rs`,
  `prepare_and_run_active_document`: performs template shell discovery through
  `resolve_shell_approvals(Some(&source.markdown), ...)` before calling
  `execute_loop_or_single`. The discovery occurs for proxy targets too.
  Deferring the schema verdict does not defer transclusion reads.
- `claudine/lib/src/composition/preflight.rs`, `resolve_shell_approvals`:
  invokes Darkmatter's `compose_preflight` to discover template commands.
- `darkmatter/lib/src/markdown/compose/preflight/collect.rs`,
  `collect_recursive`: retains conditional body regions, resolves every
  Markdown transclusion without evaluating its condition, and propagates a
  missing-target error. This intentional condition-independent discovery
  ensures command approval covers all possible branches.
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`,
  `run_initialize_stages` and `bootstrap_adopted_document_phase`: already
  describe and implement a narrow initialization-shell gate followed by
  initialization, a stabilized reread, and a full lifecycle audit. The earlier
  command-level scan can fail before this sequence is reached.

The defect is the placement of body discovery before initialization, not the
fact that a real missing transclusion produces a fatal error.

## Required Behavior

### R1. Prepare only what initialization needs before running it

For each newly adopted proxy target, prepare its identity, caller inputs,
frontmatter, and lifecycle configuration sufficiently to run `initialize`.
This bootstrap must not read body transclusions, execute body shell commands,
or fully compose the body before initialization can create its prerequisites.

Frontmatter and lifecycle interpolation needed to resolve initialization's
inputs remains permitted. This requirement does not prohibit all
interpolation before initialization, nor does it defer resolution of explicitly
supplied eager file inputs that must already exist for initialization to use.

### R2. Preserve the approval boundary

Approve the shell commands initialization could execute before executing any
of them. A denied command must prevent its effects and provider launch.
Non-shell effects such as `ensure_file` retain their existing path-resolution
and mutation policies.

After initialization, reread the document and reapply the retained caller
inputs and their origins. Discover and approve body/transclusion commands and
the remaining lifecycle commands against this stabilized state before those
commands can execute. Reuse qualifying approvals from the invocation cache.

Do not fix this by ignoring missing-file errors, making approval discovery
condition-dependent, or disabling discovery under `-y`.

### R3. Initialize each adopted document exactly once

An initialization that proceeds must finish before the target's body is
discovered or composed. An initialization that exits, fails, or proxies again
must preserve its existing control-flow semantics; no abandoned target's body
may be composed and no provider may launch for it.

For a proxy chain, apply the same ordering at every newly adopted document.
Stabilized rereads and loop iterations must not emit duplicate initialization
events. Preserve existing retry/resume event semantics.

### R4. Share one ordering across entry paths

Use the existing staged initialization machinery as the design starting point.
Avoid a prompt-specific exception or a second independent lifecycle engine.
Apply the invariant to proxy adoption through `compose`, `inline-compose`, and
sequence execution, and verify that direct invocation of a prompt with the
same initialization-created dependency works too.

Keep the captured file-resolution context and caller-origin information intact.
`ensure_file` and a subsequent include of that file must agree on its identity
on macOS, Linux, native Windows, and WSL2.

### R5. Preserve useful failures

If the referenced file remains missing after initialization, fail through the
existing typed diagnostic and lifecycle error handling. Do not launch a
provider with an incomplete prompt. A failure during initialization must not
be obscured by an earlier attempt to read the body.

## Acceptance Criteria and Regression Coverage

1. **Absent generated file:** a router proxies to a target whose initialization
   creates an absent Markdown file. The target includes its content, and a fake
   provider receives the completed prompt. Assert initialization ran once and
   creation preceded discovery/composition.
2. **Reported guarded shape:** cover the nested `file_exists(log)` and
   `phase > 1` blocks with phase 1 and a later phase. Phase 1 excludes the log
   from the rendered body; the later phase includes it after initialization.
3. **Unconditional include:** cover an unconditional include of the generated
   file as well, proving the fix establishes ordering rather than bypassing
   conditional discovery.
4. **Existing file and repeated invocation:** `ensure_file` preserves existing
   contents. A second invocation succeeds, reads the persisted file correctly,
   and runs initialization once for that invocation.
5. **Approval integrity:** deny an initialization shell command and assert no
   command effects or provider launch. Have initialization create or rewrite an
   included document containing a shell directive; prove its command is audited
   before execution. An existing false-condition include still contributes its
   commands to the approval set.
6. **Mutation visibility:** initialization can update a body dependency or the
   target document, and the resulting prompt and approvals use the reread state,
   not a cached bootstrap body.
7. **Control flow:** cover an initialization proxy chain, clean exit, and error.
   Abandoned bodies are not read, and initialization does not run twice during
   target adoption or ordinary loop continuation.
8. **Missing after initialization:** a file not created by initialization still
   yields the typed missing-transclusion error and prevents provider launch.
9. **Entry-path parity:** exercise direct invocation and proxy adoption through
   compose, inline-compose, and sequence execution at their shared boundaries.
10. **Shipped artifact:** add passive coverage of the relevant shipped prompts
    and a hermetic end-to-end regression using the implementation router and
    target through the normal CLI path. Preserve the original spec argument
    spelling in a fixture, with fixture-owned spec/plan data and fake providers.

Use the repository's nextest-backed `just test` recipes and appropriate lint
checks. Tests must use `CliProcessFixture`, isolate filesystem and provider
state, suppress lifecycle audio, and never launch real providers or focus
terminal/browser windows. Reuse qualifying passing OS evidence; otherwise run
the required coverage. Report the exact evidence and any remaining gaps.

## Scope and Completion

This fix owns the initialization/body-discovery ordering and its regression
coverage. Correct the shipped log-path expression as a separate, identified
prompt repair within the change; review its logging instructions so they remain
accurate when initialization has already created an empty file.

Do not redefine `parent_dir`, weaken transclusion failures, redesign the
expression language, or broaden approval permissions. Update composition and
lifecycle documentation and the Claudine skill where they describe the affected
ordering. Review comments on changed symbols for drift.

The fix is complete when the generated-file flow succeeds through the normal
invocation path, all applicable acceptance cases pass, approval and lifecycle
semantics remain intact, and validation evidence is recorded. This document is
a specification; implementation and a separate implementation plan remain future
work.
