# Preflight Review 2

Reviewed against:

- `claudine/features/2026-04-01-preflight/spec.md`
- current Claudine/Darkmatter implementation

Validation performed:

- Read the updated preflight implementation and call sites in Claudine and Darkmatter.
- Ran targeted tests:
  - `cargo test -p claudine composition::preflight`
  - `cargo test -p darkmatter shell_expansion::discovery`
- Ran two manual repros against `target/debug/claudine`:
  - `compose --interactive` with an unapproved `::shell`
  - `inline-compose` with harness enabled and a `::shell` hidden inside a false `::block`

## Findings

### 1. High: `compose --interactive` and `inline-compose --interactive` still disable preflight prompting

The wrapper path now wires an interactive approval handler, but the dedicated `compose` and `inline-compose` entrypoints still call `build_harness_shell_options(..., false)` unconditionally. That means an explicitly interactive composition session still fails with “no approval handler is available” instead of prompting the user.

Evidence:

- `claudine/cli/src/commands/compose.rs:280-288`
- `claudine/cli/src/commands/compose.rs:385-392`

Verified repro:

```bash
claudine compose --interactive --codex /tmp/test.md
```

with:

```md
# Test
::shell echo needs-approval
```

Result:

```text
pre-flight shell approval failed: Shell command 'echo needs-approval' ... requires approval but no approval handler is available. Add to whitelist or run interactively.
```

Suggested fix:

- Thread `args.interactive` into `build_harness_shell_options()` for both `run_compose_inner()` and `run_inline_compose_inner()`.

### 2. High: the approval prompt still shows synthetic provenance (`dummy`, line `0`) instead of the real source

`resolve_shell_approvals()` now collects real `(source_file, line)` tuples, but it still delegates prompting through `validate_and_approve_command_parts()`, which manufactures a `ShellApprovalRequest` with `ComposeSource::File(root.join("dummy"))` and `line: 0`. So the user-facing prompt is still wrong even though Claudine now knows the correct provenance.

Evidence:

- `claudine/lib/src/composition/preflight.rs:95-103`
- `claudine/lib/src/harness/shell.rs:97-101`
- `claudine/lib/src/harness/shell.rs:168-179`

Impact:

- The key UX goal from the spec, “show the command, source file, and line number,” is still not met for interactive approvals.
- This affects both template commands and harness commands because all prompting still flows through the harness shell adapter.

Suggested fix:

- Add a provenance-aware approval entrypoint that accepts `source_file` and `line`, or extend `ShellApprovalOptions` / `validate_and_approve_command_parts()` to take prompt metadata so `ShellApprovalRequest` is built from real provenance instead of a placeholder path.

### 3. High: the harness loop still re-audits raw source-page `::shell` directives after composition, which can disagree with preflight

The updated preflight scanner now correctly excludes directives hidden by false `::block`s and includes commands introduced by replacement/transclusion. But `run_harness_loop()` still reads the raw source file and passes it to `collect_auditable_commands()`, which parses raw `::shell` directives directly. That reintroduces the old divergence between “what preflight saw” and “what runtime audit sees.”

Evidence:

- `claudine/cli/src/commands/wrap/mod.rs:2251-2256`
- `claudine/lib/src/harness/audit.rs:75-84`
- `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs:79-86`

Verified repro:

```md
---
prompt: ignored
pre_checks:
  file_exists: "ok.txt"
include_shell: false
---
::block when="include_shell"
::shell echo should-not-be-seen
::end-block
Body
```

Running:

```bash
claudine inline-compose --codex test.md
```

produced:

```text
1 shell command was audited:
echo should-not-be-seen denied by policy
shell audit failed for source-page directives — cannot proceed
```

even though the updated Darkmatter discovery logic correctly excludes that directive during preflight.

Suggested fix:

- Do not raw-parse source-page directives in the harness loop for composition-driven flows; reuse the already-composed/preflighted command set instead.
- If source-page audit is still needed for passthrough wrapper harnesses, keep it limited to that path rather than running it unconditionally inside `run_harness_loop()`.

## Coverage Gaps

- No CLI integration test covers `compose --interactive` or `inline-compose --interactive` with an unapproved shell command. The current regression slipped through because the existing tests cover interactive provider behavior, not interactive preflight approval.
- No test asserts the contents of the `ShellApprovalRequest` passed during preflight, so the synthetic `dummy:0` prompt provenance is currently untested.
- There is good unit coverage for Darkmatter discovery excluding false page blocks, but no integration coverage proving the harness loop does not re-audit raw source directives after composition. The manual repro above should become a regression test.

## Suggestions

- Treat “composed prompt preflight” and “raw source-page audit” as two distinct modes. Right now they are mixed together in the harness loop, and that is what keeps reintroducing divergence.
- Push real provenance all the way into the approval callback, not just into `CompositionError`.
- Add one end-to-end CLI test per entrypoint family:
  - `compose --interactive` prompts successfully
  - wrapper passthrough interactive preflight prompts successfully
  - composition + harness does not fail on a `::shell` hidden by a false page block
