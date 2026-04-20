# Preflight Review

Reviewed against:

- `claudine/docs/topics/pre-flight-checks.md`
- `docs/superpowers/specs/2026-04-01-preflight-shell-approval-design.md`

Validation performed:

- Traced the current implementation through Claudine and Darkmatter.
- Ran `just test` in `claudine/` and `darkmatter/`; both suites passed.

## Findings

### 1. High: Claudine never wires an interactive approval handler, so the designed approval flow does not exist

The design says unapproved commands should be shown to the user with `Allow exact`, `Allow command`, `Allow once`, `Deny`, and `Blacklist` choices before the session starts. The current CLI always builds `ShellApprovalOptions` with `approval_handler: None`, so any unwhitelisted command hard-fails instead of prompting. This affects compose, inline-compose, and wrapper preflight because they all reuse the same builder.

Evidence:

- `claudine/cli/src/commands/wrap/mod.rs:170-178`
- `claudine/lib/src/composition/preflight.rs:106-115`
- A reusable interactive handler already exists in `darkmatter/cli/src/approval.rs:10-24`

Impact:

- Designed functionality is missing.
- Even interactive Claudine sessions cannot approve new commands on demand during preflight.
- The current implementation only works for already-whitelisted commands.

### 2. High: Harness parsing still performs approval at parse time, so preflight is not the single approval authority

The design explicitly makes Claudine preflight the single place where shell approval decisions happen. The current harness parser still calls `validate_and_approve_command*()` while building the `HarnessPlan`, which means harness command approval still happens during parse, before `resolve_shell_approvals()` is even reached.

Evidence:

- `claudine/lib/src/harness/parse.rs:58-60`
- `claudine/lib/src/harness/parse.rs:912-929`
- `claudine/cli/src/commands/wrap/composition.rs:426-434`
- `claudine/cli/src/commands/wrap/mod.rs:1124-1132`

Impact:

- Preflight does not actually own harness approval.
- Once an approval handler is wired in, this path is likely to duplicate prompts or prompt too early.
- Discovery and authorization remain coupled in the harness path, which is the opposite of the intended boundary.

### 3. High: Harness preflight approvals are discarded and replaced with a second raw-source audit

For harness flows, Claudine calls `resolve_shell_approvals()`, but the returned approved set is not carried into runtime. In the composition wrapper the result is only used for logging, and in passthrough wrappers it is bound to `_harness_preflight` and dropped. Later, `run_harness_loop()` re-reads the source file, rebuilds auditable commands from raw text, and re-checks policy with fresh shell options.

Evidence:

- `claudine/cli/src/commands/wrap/composition.rs:447-460`
- `claudine/cli/src/commands/wrap/mod.rs:1134-1141`
- `claudine/cli/src/commands/wrap/mod.rs:2234-2268`

Impact:

- Session-local `AllowOnce` approvals would be lost even after the handler is added.
- Runtime behavior can diverge from preflight because the second pass scans raw source text instead of Darkmatter’s resolved document graph.
- The implementation does extra work and still does not satisfy the “pass the approved set to runtime” part of the design.

### 4. Medium: Template command provenance is wrong for transclusions, and Claudine throws away the metadata anyway

The design requires showing the user the command plus its source file and line number. `collect_shell_commands()` currently parses directives from the fully composed output, then assigns every discovered command to the root `ComposeOptions::source` file. That means transcluded commands cannot report their real file. Claudine then reduces each entry to `entry.normalized`, discarding all remaining provenance before error reporting.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:25-43`
- `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs:67-72`
- `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs:91-96`
- `claudine/lib/src/composition/preflight.rs:55-73`

Impact:

- Prompts and failures cannot accurately identify where a command came from.
- Transcluded commands will be misattributed to the root document.
- This also prevents the designed “scanner bug” runtime message from naming the real source location.

### 5. Medium: Discovery does not mirror the real compose pipeline closely enough

`collect_shell_commands()` only runs frontmatter interpolation, interpolation, and transclusion. The actual compose order also includes `TextReplacement` and `PageBlocks` before shell expansion. Because discovery skips those stages, it can collect commands from conditionally removed blocks or miss directives introduced by replacement.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs:54-61`
- `darkmatter/lib/src/markdown/compose/types.rs:170-198`

Impact:

- Preflight can ask for approvals that the real compose run would never execute.
- The “not pre-approved at runtime means scanner bug” guarantee is weaker than intended because discovery is not truly composition-equivalent.

### 6. Medium: Interactive wrapper commands still skip harness preflight entirely

The design says preflight should run for all wrapper commands, including interactive sessions, because shell commands execute before the provider session begins. The passthrough wrapper path only restores harness behavior when `effective_non_interactive` is true, so interactive `claudine claude/codex/...` sessions bypass harness preflight.

Evidence:

- `claudine/docs/topics/pre-flight-checks.md`
- `claudine/cli/src/commands/wrap/mod.rs:1109-1116`

Impact:

- Current wrapper behavior does not match the documented contract.
- Interactive wrapper sessions can still miss designed preflight validation.

### 7. Medium: Runtime error reporting is still flatter and less actionable than the spec

Darkmatter has a `NotPreApproved` error, but it only includes the command and a line number. Claudine then wraps compose failures as `compose failed: ...`, which loses the stronger preflight-specific framing the design calls for. Timeout and execution-failure errors also do not include the working directory or richer source context described in the spec.

Evidence:

- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:273-286`
- `claudine/lib/src/composition/prepare.rs:44-47`
- `claudine/lib/src/composition/prepare.rs:100-102`
- `claudine/lib/src/composition/error.rs:33-35`

Impact:

- The user experience is better than the original hang, but still below the designed error quality bar.
- Debugging missed-preflight cases will be harder than intended.

## Coverage Gaps

- `claudine/lib/src/composition/preflight.rs:168-240` only exercises whitelist-only flows; there are no preflight tests covering `AllowOnce`, `AllowExactPersist`, `AllowCommandPersist`, `Deny`, or `BlacklistPersist`.
- `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs:114-202` does not test `source_file` or `line` fidelity for transcluded directives, and it does not test `PageBlocks`/replacement interactions.
- `claudine/cli/tests/wrap_commands.rs` exercises harness validation broadly, but it does not contain end-to-end tests for preflight approval prompting, persisted approvals, session-local approvals, or runtime handoff of approved commands.
- There are no integration tests proving that retry/redirect/deviate flows preserve preflight shell approvals across harness attempts.

## Improvement Ideas

- Introduce a shared structured command type for preflight and harness runtime: `executable`, `args`, `normalized`, `source_file`, `line`, and origin. That would remove the current normalize-then-tokenize roundtrip in `resolve_shell_approvals()` and preserve provenance.
- Reuse `darkmatter::cli::approval::CliShellApprovalHandler` from Claudine instead of re-implementing approval prompting.
- Make harness parsing discovery-only, then feed a shared session approval state into execution. That aligns with the design and removes duplicated policy checks.
- If startup latency becomes noticeable, cache/share the resolved document-graph analysis between `collect_shell_commands()` and the subsequent compose pass instead of walking the graph twice.
