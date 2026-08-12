# Pre-Flight Shell Approval

Before any Claudine wrapper session launches a provider, it needs to know that every shell command the session might execute has been authorized. This is what the pre-flight check does: it scans all possible shell commands, resolves their approval status, and asks the user to approve anything that is not already whitelisted. Once the pre-flight completes, the session runs without further shell-related prompts.

## Why This Exists

Shell commands can appear in three places during a Claudine session:

1. **Template `::shell` directives** — Darkmatter's compose pipeline executes these during document composition. A prompt like `commit.md` might contain `::shell sniff repo packages` to inject dynamic content.
2. **Frontmatter `$(cmd)` expressions** — top-level frontmatter string values of the form `$(command arg ...)` are evaluated by Darkmatter's frontmatter shell expansion phase, with the command's trimmed `stdout` written back into the frontmatter. Example: `today: $(date +%Y-%m-%d)`.
3. **Lifecycle `shell` stack actions** — positional `shell: "…"` actions and key/value `{ action: shell, command: "…" }` actions declared in any reachable lifecycle stack (`initialize`, `start`, `success`, `blocked`, `failure`, `finalize`, `loop`). Some are conditional (guarded by `when:` or reachable only on a recovery path) but they still need pre-authorization because there is no opportunity to prompt the user mid-session.

Without pre-flight, a shell command that lacks whitelist coverage would either block the process waiting for interactive approval that will never come (in a non-interactive session) or fail with a confusing error deep inside the composition pipeline. The pre-flight eliminates both problems by resolving all approvals upfront.

## How It Works

The pre-flight runs as part of every wrapper command — `claudine compose`, `claudine inline-compose`, `claudine claude`, `claudine codex`, and all other provider wrappers. It sits between prompt resolution and provider launch:

```
Prompt resolved → Pre-flight shell approval → Provider launches
```

### Two-Phase Discovery

Claudine discovers shell commands in two phases because lifecycle stack properties can only be read from the **effective (composed) frontmatter**, which is not available until after Darkmatter composition runs. But composition itself needs the pre-approved command set to execute `::shell` directives. This creates a dependency:

```
Phase 1: Discover template ::shell directives
         → approve them
         → pass approved set to Darkmatter composition
         → composition produces effective frontmatter

Phase 2: Parse lifecycle stacks from effective frontmatter
         → discover lifecycle shell commands
         → approve them (reusing the shared approval cache)
```

Both phases share a single **approval cache** — an `Arc<Mutex<HashMap>>` that maps normalized command strings to approval decisions. When a command approved in phase 1 also appears in phase 2 (e.g. the same `curl` call in both a template directive and a pre-check), the cache hit skips the duplicate prompt. From the user's perspective this appears as a single approval loop.

The `claudine claude` / `claudine codex` passthrough path uses only phase 2 because it has no template composition step — it parses the lifecycle stacks directly from the source file's frontmatter and preflights their shell commands in a single pass.

The `claudine sequence` orchestrator runs both phases **per step** during its upfront discovery loop, so every template and lifecycle command across every step is approved before any provider session starts. See [Sequence Execution](#sequence-execution) below.

### Phase 1: Template Directives

Claudine asks Darkmatter to walk the full document graph and return every `::shell` directive it finds. Darkmatter runs interpolation first (using the same state that will be used during actual composition) so that template variables and dynamic transclusion paths resolve correctly. The result is a list of concrete commands with their source file and line number.

Each command is checked against shell policy (blacklist, whitelist, approval cache) and, if not already approved, the user is prompted. Once all template commands are approved, the approved set is passed to Darkmatter as `pre_approved_commands` on the `ComposeOptions` and composition proceeds.

### Phase 2: Lifecycle Shell Commands

After composition, Claudine walks every reachable lifecycle stack in the effective frontmatter and discovers its `shell` actions — positional `shell: "…"` actions and key/value `{ action: shell, command: "…" }` actions across `initialize`, `start`, `success`, `blocked`, `failure`, `finalize`, and `loop`.

These commands flow through the same `resolve_shell_approvals` function and the same shared approval cache. Any command already approved in phase 1 is a cache hit. Only genuinely new commands trigger additional prompts.

### Per-Attempt Audit (Passthrough Only)

In the passthrough wrapper path (`claudine claude`, `claudine codex`), the harness loop re-audits shell commands on every attempt (`Retry`, `Proxy`). This is necessary because the source file may change between iterations — a `Proxy` action can point to a different file with different `::shell` directives. The per-attempt audit reads the raw source text and discovers source-page directives via line-level scanning.

Composition flows (`claudine compose`, `claudine inline-compose`) do **not** re-audit on each attempt. Template directives were discovered through Darkmatter's graph walker (which respects `::block when="false"` guards), and lifecycle shell commands were approved in phase 2. The approval handler is frozen after the first attempt so `Proxy`/`Retry` iterations cannot trigger new interactive prompts — only cached or whitelisted commands pass.

### Approval Policy

Each command is checked against Claudine's shell policy:

- Built-in blacklist (dangerous commands like `rm`, `dd`, `chmod`)
- User blacklist (`.darkmatter-shell-blacklist`)
- User whitelist (`.darkmatter-shell-whitelist`)

Commands that match the whitelist are marked as approved. Commands that match a blacklist are rejected immediately with a clear error. Everything else is presented to the user.

### User Prompt

Any command not already covered by the whitelist or approval cache is presented to the user one at a time. The user sees the command, its source file, and line number, and can choose:

- **Allow this exact command** (persisted to whitelist)
- **Allow all commands from this executable** (persisted to whitelist)
- **Allow once** (this session only — cached but not persisted)
- **Deny** — the session aborts immediately
- **Blacklist** (persisted to blacklist, session aborts)

If the user denies any command, Claudine stops. No provider session is started. The error message states exactly which command was denied and confirms that nothing was executed.

### Shared Approval Cache

The approval cache is an in-memory `HashMap<String, CachedApprovalDecision>` wrapped in `Arc<Mutex<...>>`. It serves three purposes:

1. **Cross-phase deduplication**: A command approved during template discovery (phase 1) is not re-prompted during harness discovery (phase 2).
2. **Cross-step reuse for sequences**: The sequence orchestrator builds a fresh `ShellApprovalOptions` per step but clones the same `Arc` cache into each one. An "allow once" approval from step 1 carries forward to step 5 without re-prompting.
3. **Freeze enforcement**: After pre-flight completes, composition modes remove the interactive approval handler from the `ShellApprovalOptions`. The cache remains, so previously approved commands still pass, but new uncached commands are denied without prompting. This enforces the contract that all shell approvals are resolved before the provider session starts.

## The Claudine/Darkmatter Boundary

Claudine and Darkmatter each have shell approval infrastructure, but they serve different roles during pre-flight:

**Darkmatter's role is discovery.** It knows how to walk the document graph — following transclusions, resolving interpolation, parsing `::shell` directives, and scanning top-level frontmatter values for `$(...)` shell expressions. It exposes a function (`collect_shell_commands`) that returns every shell command in the document tree. It does not check any policy files or make any approval decisions during this call.

**Claudine's role is authorization.** It takes the list from Darkmatter, combines it with commands from the harness, checks everything against the whitelist, and prompts the user for anything that is missing. Claudine is the single source of truth for what is allowed.

**During composition**, Darkmatter receives the pre-approved set and trusts it completely. It skips its own whitelist, blacklist, and approval handler checks. This means Darkmatter's standalone approval flow (which still works when used outside of Claudine) is bypassed entirely when Claudine provides pre-approvals.

This separation ensures that:

- Claudine never needs to understand transclusion, interpolation, or the document graph
- Darkmatter never needs to understand the harness, interactive prompting, or session lifecycle
- There is exactly one place where shell approval decisions are made (Claudine)
- There is exactly one place where shell commands are discovered in templates (Darkmatter)

## Error Messages

The pre-flight system produces three categories of error, each designed to identify the problem immediately:

### Command Denied During Pre-Flight

The user chose to deny a command. The session was never started.

```
Aborted: shell command 'rm -rf /' was denied during pre-flight approval.
No provider session was started.
```

### Command Not Pre-Approved at Runtime

A shell command was encountered during composition or harness execution that was not in the pre-approved set. This should never happen — it means the pre-flight scanner missed a command.

```
Shell command 'sniff repo packages' was not pre-approved and cannot
be approved during an active session. This is a bug in the pre-flight
scanner -- please report it.

Source: prompts/commit.md:23
```

### Command Execution Failure

A pre-approved command was executed but failed (non-zero exit or timeout).

```
Shell command 'sniff repo packages' failed after 10s (timeout).

Source: prompts/commit.md:23
Working directory: /path/to/repo

This command was approved and executed but did not complete within
the timeout. If this command normally completes quickly, it may be
blocked by another process or waiting on a resource.
```

## Interaction with Existing Features

### Whitelist Files

The pre-flight uses the same `.darkmatter-shell-whitelist` and `.darkmatter-shell-blacklist` files that Darkmatter and the harness already use. Commands that the user approves with "allow exact" or "allow command" during pre-flight are persisted to the whitelist, so they will not require approval again in future sessions.

### Non-Interactive Sessions

Pre-flight is especially important for non-interactive sessions where there is no terminal available for mid-session prompts. But it runs on all wrapper commands — including interactive sessions started with a prompt — because the shell commands in the template and lifecycle stacks execute before the interactive session begins.

### Lifecycle Shell Actions

Shell actions declared anywhere in the lifecycle stacks (positional `shell: "…"` actions and key/value `{ action: shell, command: "…" }` actions) are included in the pre-flight scan. This means all shell commands across the entire session lifecycle are authorized upfront, not just those in the template.

### Sequence Execution

When running a sequence (`claudine sequence <file>`, declared via the `sequence` frontmatter property), the sequence orchestrator runs a single upfront discovery loop before any provider session starts. For each step it:

1. Builds the step-specific `ComposeOptions` (because `--set` overlays differ per step).
2. Runs the **template pre-flight** to discover and approve `::shell` directives (and frontmatter `$(...)` expressions) for that step.
3. Prepares the composition so the effective frontmatter is available.
4. Walks the lifecycle stacks in the effective frontmatter and runs the **lifecycle shell pre-flight** to approve every reachable `shell` action for that step.
5. Caches the prepared composition for reuse during execution.

All steps share the same approval cache, so a command approved on step 1 is not re-prompted on step 5. Cumulatively approved commands are merged and passed to each step's composition run. Once the discovery loop finishes, **every shell command across every step — template and lifecycle — has already been approved**, and the operator can walk away while the sequence executes. A failure during Phase 1 (bad template, unparseable lifecycle stacks, denied approval) aborts the whole sequence before any step runs.
