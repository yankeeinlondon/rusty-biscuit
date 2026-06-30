---
status: ready for planning and implementation
reviewed: true
created: 2026-06-26
area: claudine
packages:
    - claudine
review_iterations: 2
---

# OpenCode YOLO Does Not Bypass Permissions for Subagent Sessions

## Symptom

Non-interactive OpenCode composition runs hang indefinitely. The same
`prompts/implement-suggestions.md` composition was run twice — once with
`kimi-for-coding/k2p7`, once with `zai-coding-plan/glm-5.2` — and both hung in
the same place, despite the run being in **YOLO** mode (the compose UI shows the
`YOLO` badge).

Walking the GLM run (`ses_0f9a7f10`): the orchestrator session executed cleanly,
then dispatched a `rust-developer` subagent (child session `ses_0f9a06d08`). The
subagent ran normally through ~15 steps, then went **totally silent** — no
further `llm_call`, `http_response`, or `step_loop` for 20+ minutes. The last
line it emitted was:

```
permission_evaluated external_directory:/tmp/*
```

The subagent had written lint output to `/tmp/jl.log` (via a `just lint >
/tmp/jl.log` shell redirect) and then attempted to **read** `/tmp` back — i.e. a
tool call touching a path outside the working directory.

Crucially, the **parent** orchestrator session had earlier touched an external
directory and proceeded fine:

```
permission_evaluated external_directory:/Users/ken/.cargo/registry/.../expectrl-0.8.0/src/*
read:../../../../.cargo/registry/.../expectrl-0.8.0/src/lib.rs
← Read(successful, .../expectrl-0.8.0/src/lib.rs)
```

So external-directory access was auto-approved for the **parent** session but
**not** for the **subagent** session. That asymmetry is the whole bug.

## Root cause

Three facts converge:

1. **`external_directory` defaults to `"ask"`** in OpenCode — it "controls tool
   calls that touch paths outside the working directory"
   (`claudine/docs/research/permissions/opencode.md:49,75`). The related
   `doom_loop` guard also defaults to `"ask"`.

2. **`--dangerously-skip-permissions` (YOLO) only covers the _parent_ session.**
   Claudine's own research records the exact contract
   (`claudine/docs/research/non-interactive-sessions/opencode.md:190-191`):

   > - if `--dangerously-skip-permissions` is passed, **parent-session**
   >   permission requests are auto-approved once
   > - otherwise parent-session permission requests are auto-rejected and a
   >   warning is printed to stderr

   The installed binary's flag help agrees: *"auto-approve permissions that are
   not explicitly denied (dangerous!)"* — and verified on `opencode` **1.17.11**,
   `opencode run` exposes **only** `--dangerously-skip-permissions`; there is no
   `--yolo` on the `run` subcommand.

3. **Child/subagent (Task) sessions are not the parent session**, so they do not
   receive the parent's auto-approval. A subagent tool call against
   `external_directory` therefore falls back to the `"ask"` default. In a
   non-interactive `opencode run` there is no TTY to answer the prompt, so the
   subagent blocks forever.

Aggravating context: these runs execute inside a **git worktree**
(`/Users/ken/.claudine/worktrees/...`). OpenCode is known to fire
`external_directory` prompts for paths that resolve outside a worktree
(`claudine/docs/research/permissions/opencode.md` Workaround #10), so external
access is reached easily here.

### What Claudine does today

For non-interactive YOLO, `OpencodeWrapper::apply_yolo_for_mode`
(`claudine/cli/src/commands/wrap/profile/opencode.rs:27-47`) pushes
`--dangerously-skip-permissions` onto the argv. This is the only available CLI
flag, but per the contract above it is structurally **parent-session only**. The
`env_overrides` parameter to that method is currently unused for OpenCode — no
config-level permission state is injected.

### Why the failure presents as a ~30-minute freeze, not a fast error

Claudine's silence watchdog has an OpenCode-specific grace
(`claudine/cli/src/commands/wrap/exec/watchdog/evaluate.rs`): while a step is
in-flight (`step_in_flight`) and **either** the structured-event clock **or**
the raw-byte clock is within budget, the `step_timeout` rule is suppressed. A
genuinely blocked child produces zero bytes, so both clocks only go stale at the
default 30-minute `step_timeout`. The watchdog *was* behaving as designed — it
just surfaces a real hang slowly. (Tuning that grace window is **out of scope**
here; the fix below removes the hang at its source.)

## Regression framing

YOLO is intended to mean "auto-approve everything for this run" — historically
including subagent permission gates. The durable mechanism for session-wide
auto-approval (parent **and** children) is not a CLI flag but a **config-level**
permission block delivered via `OPENCODE_CONFIG_CONTENT`, which OpenCode applies
to the whole session. If a prior Claudine version injected such a permission
block and it was later dropped (or silently clobbered by another
`OPENCODE_CONFIG_CONTENT` writer — see below), that is the regression vector.
The implementer should confirm this against git history during planning, but the
fix is correct regardless of how the gap arose.

## The principle

> YOLO means YOLO. When the user opts into auto-approval for a run, **every**
> permission gate must be neutralized for the whole session — parent and
> subagent alike — including the `external_directory` and `doom_loop` guards
> that default to `"ask"`. A bypass that only covers the parent session is not
> YOLO; it is a trap that hangs the moment a subagent touches an external path.

## Proposed fix

When YOLO is effective for a **non-interactive** OpenCode launch, inject a
**config-level permission block** through `OPENCODE_CONFIG_CONTENT` that
auto-allows all permission gates session-wide, in addition to the existing
`--dangerously-skip-permissions` flag.

Reader note: OpenCode also has permission-related env vars such as
`OPENCODE_YOLO` and `OPENCODE_PERMISSION`, but this spec intentionally uses the
inline config path. Claudine already uses `OPENCODE_CONFIG_CONTENT` for OpenCode
runtime MCP and appended system prompts, OpenCode gives this inline config the
highest precedence in its documented config stack, and a single merged JSON
object is inspectable in tests. Adding another env surface would make the final
permission state harder to reason about and would not solve the existing
`OPENCODE_CONFIG_CONTENT` clobber.

### Permission block shape

```json
{
  "permission": {
    "*": "allow",
    "external_directory": "allow",
    "doom_loop": "allow"
  }
}
```

- `"*": "allow"` covers ordinary tool permissions (bash/edit/read/etc.) for
  child sessions, which `--dangerously-skip-permissions` does not reach.
- `external_directory` and `doom_loop` are set **explicitly** because the
  research records them as special guards that default to `"ask"` independently
  of the tool wildcard; do not assume `"*"` subsumes them.
- `.env` and `.env.*` read denies are accepted as overridden under explicit
  YOLO if OpenCode's resolver treats `permission["*"]="allow"` as a global
  allow. This is intentional: the user asked Claudine for full auto-approval for
  this run. Non-YOLO runs must retain OpenCode's default `.env` protection.
- The block contains **no host-specific paths**, so the emitted env-var JSON is
  byte-identical on macOS, Linux, and Windows.

Keep pushing `--dangerously-skip-permissions` (belt-and-suspenders: it remains
the parent-session one-shot grant and matches the documented contract).

### `OPENCODE_CONFIG_CONTENT` must be merged, not overwritten

`OPENCODE_CONFIG_CONTENT` is currently written by **two independent producers
that each overwrite the variable**:

- MCP injection — `claudine/lib/src/mcp/inject.rs:103`
  (`env.insert("OPENCODE_CONFIG_CONTENT", json!({"mcp": ...}))`).
- System-prompt append — `claudine/cli/src/commands/wrap/profile/opencode.rs:64-70`
  (`{"instructions": [<temp file>]}`).

Whichever runs later wins, silently dropping the other's keys. This is already a
latent bug (MCP + system-prompt-append clobber each other); adding a third
contributor (the permission block) makes a merge mandatory.

Introduce a single deep-merge for `OPENCODE_CONFIG_CONTENT` so `mcp`,
`instructions`, and `permission` coexist. OpenCode deep-merges distinct config
layers, but Claudine has only one inline-config env var for all of its runtime
overlays; Claudine therefore must merge its own overlays before spawn.

- Add a small shared helper in the claudine library for OpenCode inline config
  overlays, rather than copying merge logic between the library MCP injector and
  CLI wrapper profile.
- Parse any existing `OPENCODE_CONFIG_CONTENT` value as a JSON object; start
  from `{}` when unset. If a user-provided value is present but unparseable or
  not a JSON object, fail the OpenCode launch with a typed/actionable error
  rather than silently discarding the user's config. This env var may contain
  credentials, so diagnostics must name the variable but must not echo its raw
  value.
- Deep-merge JSON objects recursively (`mcp`, `permission`, and nested
  permission objects). Arrays are replaced by the later overlay; this is needed
  for `instructions`, where append-mode system prompt delivery intentionally
  controls the instruction file list for the run.
- Apply Claudine's YOLO permission overlay last among Claudine-generated
  overlays so `permission["*"]`, `external_directory`, and `doom_loop` cannot be
  weakened by MCP or system-prompt assembly.
- Re-serialize once at a single assembly point whenever possible. If the
  existing call graph makes a true single point too invasive, each producer must
  call the same shared merge helper against the current env value; direct
  `env.insert("OPENCODE_CONFIG_CONTENT", ...)` writes are the bug to remove.

### Policy metadata cleanup

The provider catalog is already correct for OpenCode:
`YoloSupport::NonInteractiveOnly { non_interactive_flag:
"--dangerously-skip-permissions" }`. During implementation, keep that catalog as
the authority and do not introduce an OpenCode `--yolo` argv path.

The permission backend still contains stale OpenCode assumptions in
`claudine/lib/src/permissions/providers/opencode.rs`:

- `parse_cli_overrides` recognizes `--yolo` but not
  `--dangerously-skip-permissions`.
- `build_one_shot_plan(SetApprovalMode(AutoApprove))` emits `--yolo`.

Update these if the implementation touches policy planning or uses
`PolicyEngine` for the new overlay. One-shot OpenCode auto-approve should emit
the same non-interactive flag as the wrapper and/or the same merged
`OPENCODE_CONFIG_CONTENT.permission` overlay. It must not emit `--yolo` for
`opencode run`, because the installed 1.17.11 `run` subcommand does not expose
that flag.

### Scope gating

- Inject the permission block **only** when YOLO is effective **and** the run is
  non-interactive. Interactive TUI YOLO is already reported `applied = false`
  for OpenCode and is untouched here.
- When YOLO is **not** effective, inject **no** permission block — never silently
  widen permissions on a normal run.

## Acceptance criteria

1. A non-interactive YOLO compose under OpenCode where a **subagent** touches a
   path outside the working directory (writes then reads `/tmp`, or reads
   `~/.cargo`) completes without hanging: `external_directory` is auto-allowed
   for child sessions, not just the parent.
2. When YOLO is effective and non-interactive, the assembled
   `OPENCODE_CONFIG_CONTENT` contains
   `permission["*"]=="allow"`, `permission["external_directory"]=="allow"`, and
   `permission["doom_loop"]=="allow"`.
3. When YOLO is **not** effective, `OPENCODE_CONFIG_CONTENT` contains **no**
   `permission` key contributed by Claudine.
4. With system-prompt append **and** MCP servers **and** YOLO all active, the
   assembled `OPENCODE_CONFIG_CONTENT` contains all of `instructions`, `mcp`, and
   `permission` — none clobbers another (regression guard for the existing
   two-writer clobber).
5. `--dangerously-skip-permissions` is still passed on the argv (parent-session
   coverage retained).
6. The `doom_loop` gate is auto-allowed so a long agentic loop is not blocked by
   its `"ask"` default.
7. Cross-platform: the permission block's serialized JSON is identical on macOS,
   Linux, and Windows (no path-dependent content).
8. Existing user-supplied `OPENCODE_CONFIG_CONTENT` is either merged if it is a
   JSON object or rejected with a redacted, actionable error if it is invalid;
   it is never silently replaced.
9. OpenCode policy/planning code no longer emits or recognizes native
   `opencode run --yolo` as the canonical auto-approve path. Claudine's
   provider-agnostic user-facing `--yolo` flag may still request YOLO; the
   OpenCode native argv it produces must be `--dangerously-skip-permissions`
   and/or the merged permission overlay.

## Test plan

- **unit (permission block builder):** the YOLO-permission constructor emits the
  expected `{"permission": {"*":"allow","external_directory":"allow","doom_loop":"allow"}}`
  shape.
- **unit (merge):** deep-merging `instructions` + `mcp` + `permission` preserves
  all three top-level keys; a later contributor does not drop an earlier one;
  an unset starting value yields a `{}` base; malformed JSON and non-object JSON
  are rejected with redacted diagnostics.
- **unit (gating):** non-interactive + effective YOLO injects the permission
  block; non-YOLO does not; interactive YOLO does not (matches existing
  `not_applied`).
- **unit (existing env):** an existing object-valued `OPENCODE_CONFIG_CONTENT`
  deep-merges with Claudine overlays; malformed JSON and non-object JSON produce
  a redacted error that names `OPENCODE_CONFIG_CONTENT` but does not print its
  contents.
- **unit (policy backend):** OpenCode CLI parsing treats
  `--dangerously-skip-permissions` as auto-approve and does not require or emit
  native `opencode run --yolo`.
- **integration (spawn spec):** assembling the OpenCode child `Command` for a
  non-interactive YOLO compose results in argv containing
  `--dangerously-skip-permissions` **and** an `OPENCODE_CONFIG_CONTENT` whose
  parsed JSON carries the permission block (plus instructions/mcp when present).
- **regression:** a non-YOLO run's `OPENCODE_CONFIG_CONTENT` is unchanged from
  today (only `mcp`/`instructions` as applicable).

## Resolved design decisions

### 1. Is `"*": "allow"` plus explicit guards the right shape, or should each tool key be enumerated?

Decision: use `"*": "allow"` for tool permissions **plus** explicit
`external_directory` / `doom_loop` (which default to `"ask"` outside the
wildcard). Enumerating every tool key is more brittle and drifts as OpenCode
adds tools. Implementation should include a focused verification against the
installed OpenCode behavior; if `"*"` is not honored for child sessions, keep
the explicit guard keys and add the minimum enumerated tool keys required by the
current OpenCode permission schema (`read`, `edit`, `bash`, `task`) in the same
overlay.

### 2. Does `"*": "allow"` override the default `.env` read-deny, and is that acceptable under YOLO?

Decision: accept the override under YOLO for consistency with the principle.
Research notes `.env` / `.env.*` reads are denied by default. A broad
`"*": "allow"` may override that deny, but the user explicitly opted into full
auto-approval for this run. Preserving `.env` as a carve-out would make YOLO
provider-specific and would still leave a class of non-interactive hangs if a
subagent asks for the denied read. Non-YOLO runs must not receive the permission
overlay.

### 3. Confirm the parent-only behavior against the installed version.

Decision: treat config-level permission injection as the authoritative fix. The
parent-vs-subagent asymmetry is established empirically from the transcript and
from Claudine's research docs. Planning may still verify the current OpenCode
source or release notes for 1.17.x, but the implementation must not rely on the
CLI flag alone because the observed failure proves it is insufficient for the
subagent path Claudine needs to support.

## Out of scope

- Tuning or removing the 30-minute `step_timeout` grace window. The config fix
  removes the hang at its source; watchdog tuning is a separate concern.
- Interactive TUI YOLO for OpenCode (already reported `applied = false`).
- YOLO behavior for any other provider.
- Steering subagents away from `/tmp` toward a repo-local scratch dir. It would
  reduce `external_directory` triggers, but Claudine cannot control where a
  model writes scratch files, so it is not a fix — only a partial mitigation.
