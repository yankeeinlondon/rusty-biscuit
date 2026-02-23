# Agent Wrapping

## Purpose

Claudine currently configures and reacts to agentic CLIs after they start. This feature adds an optional entrypoint where users start those CLIs through Claudine itself so Claudine can:

1. normalize runtime flags (`--yolo`, `--non-interactive`);
2. sanitize and enrich environment variables before handoff;
3. provide consistent install checks and diagnostics.

This is a wrapper/proxy execution feature, not a replacement for native CLI support.

## Command Surface

Add first-class subcommands on `claudine`:

- `claudine claude`
- `claudine codex`
- `claudine gemini`
- `claudine kimi`
- `claudine qwen`
- `claudine opencode`
- `claudine goose`

Each wrapper command supports:

- `--yolo`, `-y`
- `--include <ENV_NAME>` (repeatable)
- `--non-interactive`, `-n`, `--ni`
- passthrough args for the underlying CLI after wrapper flags

### CLI Parsing Contract

- Wrapper flags are consumed by Claudine.
- `--include` values are not forwarded.
- All other args are forwarded in order (after provider-specific mapping).
- `--` must be supported so users can force remaining args to passthrough unchanged.

## Execution Flow

Each wrapper subcommand follows this exact pipeline:

1. Capture `cwd` and raw wrapper invocation args.
2. Resolve provider profile from the subcommand (`claude`, `codex`, etc.).
3. Validate provider binary is installed (using existing Sniff-backed install detection).
4. Build child arg vector by translating wrapper flags to provider-specific flags/shape.
5. Build child environment:
   - clone process env;
   - remove sensitive keys unless explicitly allowlisted by `--include`;
   - inject Claudine wrapper context env vars.
6. Spawn provider process with inherited stdio and same `cwd`.
7. Return provider exit status as `claudine` exit status.

## Install Detection

Install check must happen before child spawn.

- Primary source: existing mapping in `Provider::sniff_ai_cli()` and Sniff install detection.
- Error output requirements:
    - identify missing binary by name;
    - identify which wrapper command failed;
    - include a docs URL if available from existing agent metadata.

Example error shape:

`error: cannot run wrapped codex session because 'codex' is not installed or not on PATH`

## Environment Sanitization

### Removal Rule

Remove environment variables whose names (case-insensitive) contain either:

- `API_KEY`
- `TOKEN`

### Allowlist Override

`--include <ENV_NAME>` exempts that exact env var name from removal.

- `--include` is repeatable.
- Invalid env names fail fast.
- Missing env names in the current process env generate a warning (not an error).

### Reporting Rule

Before child spawn, print removed variable names (sorted, unique) to stderr.

- names only, never values.
- if none removed, print nothing.

## Injected Environment Variables

The wrapper must inject:

- `AGENT`: one of `claude|codex|gemini|kimi|qwen|opencode|goose`
- `YOLO`: `true` or `false`
- `AGENT_PARAMS`: JSON string array of user-supplied args (excluding the wrapper subcommand itself)

If monorepo metadata can be resolved for current `cwd`, also inject:

- `PACKAGE_AREA`
- `PACKAGE`

### Monorepo Resolution

Use Sniff repo detection rooted at current `cwd`.

- only set `PACKAGE_AREA` and `PACKAGE` when:
    - repo is monorepo; and
    - current `cwd` is inside a detected package path.
- package selection rule: choose the package whose path is the longest prefix of `cwd`.

## Provider Mapping Profiles

Wrapper behavior is data-driven by provider profile definitions.

| Wrapper | Binary | `--yolo` mapping | `--non-interactive` mapping |
|---|---|---|---|
| `claude` | `claude` | `--dangerously-skip-permissions` | append `--print` |
| `codex` | `codex` | `--dangerously-bypass-approvals-and-sandbox` | force/ensure `exec` entrypoint |
| `gemini` | `gemini` | `--approval-mode yolo` | force prompt mode (`-p` or positional prompt path) |
| `kimi` | `kimi` | `--yolo` | append `--print` |
| `qwen` | `qwen` | `--yolo` | force prompt mode (`-p` or positional prompt path) |
| `opencode` | `opencode` | unsupported (warn only) | force `run` entrypoint |
| `goose` | `goose` | map to auto-approval mode | force `run` entrypoint |

Notes:

- Profiles are initial defaults sourced from existing capability metadata.
- Mapping is idempotent: do not duplicate provider flags if user already passed equivalent flags.
- When provider has no yolo equivalent (currently OpenCode), emit warning and continue.

## Argument Forwarding Rules

- Remove wrapper-only flags from forwarded args:
    - `--yolo` / `-y`
    - `--non-interactive` / `-n` / `--ni`
    - `--include <...>`
- Preserve remaining args order.
- Preserve quoted values exactly as parsed by Clap.
- Forward unknown flags untouched (wrapper should not become a policy gate for provider flags).

## Process and IO Rules

- Spawn child with inherited stdin/stdout/stderr in all modes.
- Do not mutate parent process env.
- Return child exit code exactly.
- If child terminates by signal, return standard non-zero wrapper exit behavior for platform.

## Error Handling Contract

Fail fast with clear stderr messages for:

- missing binary;
- invalid `--include` name;
- argument mapping conflicts that cannot be resolved safely.

Warn and continue for:

- unsupported yolo mapping;
- `--include` names not present in parent environment;
- monorepo detected but package for current `cwd` cannot be resolved.

## Proposed Code Layout

`claudine/cli`:

- `src/commands/wrap/mod.rs`
    - shared `run_wrapper(provider, args)` pipeline
- `src/commands/wrap/profile.rs`
    - provider profile table and mapping logic
- `src/commands/wrap/env.rs`
    - sanitization, include handling, injected vars
- `src/commands/wrap/exec.rs`
    - process spawning and exit propagation
- thin per-provider command modules (or enum variants) that call shared runner

`claudine/lib` (optional, if reuse is needed):

- expose helper(s) for provider install detection and provider metadata lookup if CLI should avoid duplicating mappings.

## Testing Strategy (Design-Level)

1. Unit tests:
   - sensitive env detection and allowlist behavior;
   - provider mapping idempotency;
   - arg stripping and passthrough ordering;
   - monorepo package resolution from `cwd`.
2. Integration tests:
   - wrapper fails cleanly when binary missing;
   - wrapper launches stub binary with expected args/env;
   - exit-code passthrough.
3. CLI contract tests:
   - help text for all seven new subcommands;
   - aliases (`-n`, `--ni`, `-y`) and repeated `--include`.

## Acceptance Criteria

- All seven wrapper subcommands are available and documented in CLI help.
- Sensitive env vars are removed by default and reported by name.
- Included env vars survive sanitization.
- `AGENT`, `YOLO`, and `AGENT_PARAMS` are always injected.
- `PACKAGE_AREA` and `PACKAGE` are injected when monorepo package context is resolvable.
- `--yolo` and `--non-interactive` are translated per provider profile.
- Non-wrapper args are forwarded faithfully.
- Missing binaries and unsupported features are reported with actionable diagnostics.

