---
created: 2026-09-15
status: proposed
reviewed: true
reviewed_by: codex/gpt-5.6-sol
reviewed_on: 2026-09-15
implemented: true
implemented_by: claude/default
area: claudine
packages:
    - claudine
    - claudine-cli
---

# Run Initialization Before Body Discovery

## Problem

A prompt cannot reliably create files during `initialize` for later
composition. The reported route reaches the prompt through `proxy`, but the
same command-level preparation order also affects a directly invoked document:
template shell discovery scans body transclusions before initialization runs.
When an included file does not exist yet, that scan fails and the action
intended to create it never executes.

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
commit have not yet been established. GitNexus was bound to `rusty-biscuit` at
commit `a97a7c7`, matching the current commit during review; its results were
also checked against the working-tree source.

- `claudine/cli/src/commands/compose/prep.rs`,
  `prepare_and_run_active_document`: performs template shell discovery through
  `resolve_shell_approvals(Some(&source.markdown), ...)` before calling
  `execute_loop_or_single`. The discovery occurs for proxy targets too.
  Deferring the schema verdict does not defer transclusion reads.
- `claudine/lib/src/composition/preflight.rs`, `resolve_shell_approvals`:
  invokes Darkmatter's `compose_preflight` to discover template commands.
- `claudine/lib/src/composition/prepare/service.rs`, `prepare_document`, and
  `claudine/lib/src/composition/prepare.rs`, `prepare_direct_with_prompt`:
  even with the schema verdict deferred, canonical preparation calls
  `Markdown::compose_with` for the entire document to construct a
  `PreparedComposition`. Moving only the earlier shell-discovery pass would
  therefore reveal a second pre-initialization read of the same missing
  transclusion rather than fix the ordering.
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

## Design Decisions

### Body discovery means dereferencing, not parsing

This specification uses **body discovery** to mean any operation that follows a
body dependency or evaluates body content: resolving a `::file` transclusion,
discovering a `::shell` command inside the body or its transclusions, composing
the body, or executing a body directive. Parsing the root Markdown document or
lexically inspecting it to determine context requirements is permitted before
`initialize` as long as that inspection performs no body-dependent I/O or
effects. This distinction keeps target selection and one document-epoch context
capture early without recreating the failure through a differently named scan.

### Live document entry uses one staged boot

For a live direct document or newly adopted proxy target that declares
`initialize`, the required order is:

1. load and parse the root document, assemble caller inputs and any proxy
   overlay, resolve target identity, and capture the document epoch;
2. prepare only the lifecycle surface needed by `initialize` and approve every
   shell command that event could select;
3. emit `initialize` exactly once;
4. reread the active document from disk and reapply the same caller inputs,
   origins, overlay, file-resolution context, and document epoch;
5. discover and approve body/transclusion commands and all remaining lifecycle
   commands from that stabilized read, then compose the body and apply the
   schema verdict;
6. recognize loop ownership and launch the provider.

A `skip`, `error`, or further `proxy` at step 3 ends, fails, or transfers the
current boot without entering steps 4-6 for the abandoned document. A document
without `initialize` keeps the existing eager validation and full-discovery
path. Retry, resume, and later loop iterations retain their established stage
matrix and do not acquire a second initialization.

The post-initialization read remains in the same document epoch. It may extend
the retained context snapshot for newly demanded groups, but it must not
recapture launch state or reinterpret caller-authored file values.

The bootstrap is a distinct phase of the canonical preparation service, not a
partially valid `PreparedComposition`. Its output contains only the resolved
root identity, retained input/provenance state, and the effective lifecycle
surface needed to gate and emit `initialize`; it does not claim to contain a
composed prompt. Both bootstrap and stabilized preparation must use the same
Darkmatter composition primitives and option assembly. If Darkmatter lacks a
frontmatter/lifecycle-only projection, add that projection at the shared
composition boundary rather than reimplementing interpolation in Claudine.

### Dry run remains side-effect-free

`--dry-run` continues to stop before lifecycle dispatch. It does not run
`initialize`, create its declared outputs, or traverse a dynamic proxy. A
direct dry run may therefore still report a missing transclusion that only
`initialize` would create; pretending the side effect occurred would make the
rendered preview untrustworthy. This fix changes live execution ordering, not
the dry-run contract.

## Required Behavior

### R1. Prepare only what initialization needs before running it

For each live direct document or newly adopted proxy target that declares
`initialize`, prepare its identity, caller inputs, frontmatter, and lifecycle
configuration only as far as necessary to run that event. This bootstrap must
not dereference body transclusions, discover or execute body shell commands, or
fully compose the body before initialization can create its prerequisites.

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
commands can execute. The body must be composed from the same stabilized read
that was audited. Reuse qualifying approvals from the invocation cache.

Do not fix this by ignoring missing-file errors, making approval discovery
condition-dependent, or disabling discovery under `-y`.

### R3. Initialize each adopted document exactly once

An initialization that proceeds must finish before the active document's body
is discovered or composed. An initialization that exits, fails, skips, or
proxies again must preserve its existing control-flow semantics; no abandoned
document's body may be discovered or composed and no provider may launch for
it. Status/header rendering based only on the root document and resolved target
is not body discovery and may remain before initialization.

For a proxy chain, apply the same ordering at every newly adopted document.
Stabilized rereads and loop iterations must not emit duplicate initialization
events. Preserve existing retry/resume event semantics.

### R4. Share one ordering across entry paths

Use the existing staged initialization machinery and `DocumentEntryReason`
stage matrix as the design authority. Extend canonical preparation with an
explicit initialize-bootstrap phase, and move or parameterize the premature
command-level template audit so the coordinator owns the ordering. Do not add
a prompt-specific exception, a second lifecycle engine, a second composer, or
a placeholder prompt that masquerades as a completed `PreparedComposition`.
Apply the live invariant to direct and proxy-target entry through `compose` and
`inline-compose`. Sequence execution must retain its separate static-preflight
contract described in the open question below.

Keep the captured file-resolution context, document epoch, and caller-origin
information intact. Parse authored references with
`biscuit_file::FileReference` and resolve them through the existing explicit
`FileResolutionContext`; do not introduce prefix checks or ambient resolution.
`ensure_file` and a subsequent include of that file must agree on one resolved
identity on macOS, Linux, native Windows, and WSL2.

### R5. Preserve useful failures

If the referenced file remains missing after initialization, fail through the
existing typed diagnostic and the active target's ordinary `blocked`/`finalize`
routing, exactly once. Do not launch a provider with an incomplete prompt. A
failure during initialization must not be obscured by an earlier attempt to
read the body, and a bootstrap-gate failure must retain its established
pre-ownership routing.

### R6. Preserve entry-reason and loop semantics

Documents without `initialize` must retain their current eager body discovery,
schema verdict, and failure timing. Retry and resume perform their existing
fresh read and full audit without rerunning `initialize`; a later loop iteration
continues to use the already-audited stamped structural plan. The first
iteration of a loop-owning direct or proxy-target document follows the staged
boot before loop ownership is acted upon.

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
   Also cover `skip`. Abandoned bodies are not read, and initialization does
   not run twice during target adoption, retry, resume, or ordinary loop
   continuation.
8. **Missing after initialization:** a file not created by initialization still
   yields the typed missing-transclusion error and prevents provider launch.
9. **Entry-path parity:** exercise direct invocation and proxy adoption through
   `compose` and `inline-compose` at their shared boundaries, including a
   loop-owning target. For sequences, preserve static preflight and add the
   characterization/negative coverage selected by the open-question ruling.
10. **Shipped artifact:** add passive coverage of the relevant shipped prompts
    and a hermetic end-to-end regression using the implementation router and
    target through the normal CLI path. Preserve the original spec argument
    spelling in a fixture, with fixture-owned spec/plan data and fake providers.
11. **Unchanged entry reasons:** a document without `initialize` still fails on
    a missing transclusion at its existing preparation boundary. Retry and
    resume audit a fresh read without another initialization, and a dry run
    performs no initialization side effects.
12. **Resolution parity:** cover an explicit-relative reference and at least one
    repository-scoped or magic reference, proving the lifecycle effect and
    transclusion use the same request-scoped resolution on supported path
    syntaxes. Do not assert host-specific separator text.

Use the repository's nextest-backed `just test` recipes and appropriate lint
checks. Tests must use `CliProcessFixture`, isolate filesystem and provider
state, suppress lifecycle audio, and never launch real providers or focus
terminal/browser windows. Reuse qualifying passing OS evidence; otherwise run
the required coverage. Report the exact evidence and any remaining gaps.

## Scope and Completion

This fix owns the initialization/body-discovery ordering and its regression
coverage for direct, `compose`, and `inline-compose` proxy routes. Correct
`prompts/_implement/implement-plan.md`'s shipped `log` expression from
`parent_dir(spec)` to `dirname(spec)` as a separate, identified prompt repair
within the change; review its logging instructions so they remain accurate when
initialization has already created an empty file. Do not change the meaning of
`parent_dir`.

Do not redefine `parent_dir`, weaken transclusion failures, redesign the
expression language, or broaden approval permissions. Update composition and
lifecycle documentation and the Claudine skill where they describe the affected
ordering. Review comments on changed symbols for drift.

The fix is complete when the generated-file flow succeeds through the normal
invocation path, all applicable acceptance cases pass, approval and lifecycle
semantics remain intact, and validation evidence is recorded. This document is
a specification; implementation and a separate implementation plan remain future
work.

## Open Questions

### OQ1. How should sequences handle an initialization-created transclusion?

Sequence execution currently performs a recursive, side-effect-free graph
preflight and approves every reachable shell command before any step starts.
An absent transclusion that `initialize` will create cannot be inspected during
that pass, so claiming full sequence parity here would either weaken that
contract or pretend to know the future file's contents.

#### Option A — Keep generated transclusions unsupported in sequences for this fix (recommended)

Treat the missing dependency as a static-preflight failure for a sequence task,
while fixing direct `compose`, `inline-compose`, and proxy adoption. Add a
characterization test and a diagnostic/note that directs authors to create the
artifact before starting the sequence.

- **Pros:** preserves complete pre-start approval, side-effect-free dry runs,
  and the rule that sequence execution never introduces a new approval prompt;
  keeps this defect fix bounded to the reported execution path.
- **Cons:** a document with this initialization pattern is not fully
  route-equivalent when used as a sequence prompt task; authors need a prior
  task or preexisting file.

#### Option B — Allow a post-initialization just-in-time audit in sequences

Let static preflight record the unresolved dependency, run the task's
`initialize` at its turn, and audit the stabilized body before provider launch.

- **Pros:** provides the broadest route parity and naturally supports arbitrary
  initialization-generated content.
- **Cons:** intentionally changes the sequence safety/UX contract; interactive
  runs can prompt or fail after earlier tasks have already produced effects,
  and `--dry-run` can no longer prove the executable command set.

#### Option C — Add a declared generated-artifact contract

Require sequence prompt documents to declare initialization outputs and either
constrain them to shell-free content or provide immutable source content/hash
that static preflight can inspect.

- **Pros:** can eventually preserve static analyzability while supporting a
  useful subset of generated dependencies; makes the dependency explicit.
- **Cons:** introduces a new authoring schema, validation model, and content
  trust boundary far beyond this fix; arbitrary initialization shell commands
  still cannot describe their future output safely.

**Recommendation:** choose Option A for this fix. It is the only option that
does not silently relax an established sequence guarantee. If sequence support
becomes a priority, design Option C as a separate feature with an explicit
kinded artifact contract; do not adopt Option B as an incidental consequence
of moving this preflight.
