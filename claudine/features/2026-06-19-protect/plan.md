---
agent: open_code/kimi-for-coding/k2p7
phases: 5
created: 2026-06-23
start_phase: 1
yolo: "true"
---

# Protect Posture and Extraction Hardening — Execution Plan

This plan implements the Priority 2 `protect` remediation defined in
[`spec.md`](./spec.md). The work is bounded to posture documentation,
extraction hardening, path-matching tightening, and scan-safety caps. It does
not redesign `protect` into a full shell security boundary.

## Dependency Overview

- Phase 1 is independent and can land in parallel with any code phase.
- Phases 2–4 are mostly independent; the only shared surface is the
  `ScanSurface` enum and `ProtectRequest` variants.
- Phase 5 (validation) must run after all prior code phases are complete.
- `claudine/lib/src/protect/` is the primary package area; `dispatch/mod.rs`
  consumes `extract_protect_request` and `ProtectObservation`.

## Phase 1 — Document Protect as Defense-in-Depth

*Goal: state the real posture and encode it in a bypass-corpus test gate.*

- [ ] Update module-level docs in `claudine/lib/src/protect/mod.rs` to call out
  that `protect` is best-effort defense-in-depth and that provider permissions
  plus `claudine-contract` sandboxing are the load-bearing controls.
- [ ] Update module-level docs in `claudine/lib/src/protect/catalog.rs`,
  `claudine/lib/src/protect/matcher.rs`, and
  `claudine/lib/src/protect/service.rs` with the same posture language where
  they describe the rule catalog or evaluation model.
- [ ] Update `claudine/docs/topics/protect-service.md`:
  - [ ] Add a “Posture” section near the top defining best-effort
    defense-in-depth.
  - [ ] Document known non-boundary shell variants (uppercase commands,
    variable substitution, chained commands, refspec force-push).
  - [ ] Cross-reference provider permissions and `claudine-contract`
    filesystem isolation as the authoritative controls.
- [ ] Add a `bypass_corpus_matches_posture` regression test in
  `claudine/lib/src/protect/mod.rs` that asserts the current catalog behavior
  against a documented list of obvious destructive commands and known bypass
  cases.

**Validation checkpoint:**

- [ ] `cargo nextest run -p claudine protect::` passes.
- [ ] `grep -R "defense-in-depth" claudine/lib/src/protect/ claudine/docs/topics/protect-service.md`
  shows hits in every expected module doc and the topic doc.

## Phase 2 — Distinguish Unparsed Command/Write Tools from No Opinion

*Goal: close the fail-open gap where command/write-shaped tools with unknown
payload keys bypass the guard.*

- [ ] Introduce `ProtectObservation` in `claudine/lib/src/protect/observe.rs`:
  ```rust
  pub enum ProtectObservation<'a> {
      Request(ProtectRequest<'a>),
      NoOpinion,
      Unparsed {
          surface: ScanSurface,
          reason: &'static str,
      },
  }
  ```
- [ ] Update `extract_protect_request` to return `ProtectObservation` instead
  of `Option<ProtectRequest>`.
- [ ] Expand `extract_command_string` to handle:
  - [ ] keys `command`, `cmd`, `script`, `input`;
  - [ ] top-level string arrays joined with spaces.
- [ ] Expand `extract_path_strings` to handle:
  - [ ] keys `path`, `file_path`, `file`, `target`, `filename`, `dest`;
  - [ ] `paths[]` arrays.
- [ ] Update bash-like tool detection to cover names containing `bash`,
  `shell`, or `exec`, plus exact/normalized `run_command` and `terminal`.
- [ ] Update write-like tool detection to cover names containing `write`,
  `edit`, `create`, or `delete`.
- [ ] Emit `Unparsed { surface: ScanSurface::BashCommand, .. }` when a
  bash-shaped tool has no extractable command string.
- [ ] Emit `Unparsed { surface: ScanSurface::WritePath, .. }` when a
  write-shaped tool has no extractable path.
- [ ] Update `claudine/lib/src/dispatch/mod.rs`:
  - [ ] Consume the new `ProtectObservation`.
  - [ ] Treat `Unparsed` from bash/write surfaces as a defensive block with a
    `warn!` and a secret-free reason.
  - [ ] Map the synthetic block through `map_protect_block` so the provider
    receives a structured deny response.
- [ ] Add unit tests in `observe.rs` and `dispatch/mod.rs` covering:
  - [ ] bash-like tools with `cmd`, `script`, `input`, and string arrays;
  - [ ] write-like tools with `filename`, `dest`, and `paths[]`;
  - [ ] unrelated tools remain `NoOpinion`;
  - [ ] unparsed bash/write-shaped tools are defensively blocked.

**Validation checkpoint:**

- [ ] `cargo nextest run -p claudine observe dispatch::` passes.
- [ ] New tests fail if the `Unparsed` handling is reverted to `NoOpinion`.

## Phase 3 — Tighten `allow_paths` Matching

*Goal: prevent relative and absolute allow-list entries from over-matching.*

- [ ] Update `claudine/lib/src/protect/path.rs`:
  - [ ] Implement anchored component-sequence matching for relative allow
    entries (`node_modules` matches `node_modules/foo` but not
    `/etc/build/passwd`).
  - [ ] Implement boundary-aware prefix matching for absolute allow entries
    (`/var/tmp` matches `/var/tmp/file.txt` but not `/var/tmpevil`).
  - [ ] Ensure `all_targets_allowed` returns `false` when no target operands
    are extracted.
- [ ] Update `evaluate_bash_command` in `service.rs` to require *all*
  extracted target operands to be allowed; a single unallowed target keeps the
  block.
- [ ] Add unit tests in `path.rs`:
  - [ ] `/etc/build/passwd` is not allowed by `allow_paths = ["build"]`.
  - [ ] `/var/tmpevil` is not allowed by `allow_paths = ["/var/tmp"]`.
  - [ ] `rm -rf node_modules` and `rm -rf target` are suppressible when
    explicitly allowed.
  - [ ] Mixed allowed/unallowed targets still block.

**Validation checkpoint:**

- [ ] `cargo nextest run -p claudine path::` passes.
- [ ] The three anti-examples above are asserted in the test suite.

## Phase 4 — Expand Sensitive Write-Path Coverage

*Goal: cover common credential and provider configuration locations in the
static deny catalog.*

- [ ] Extend `SENSITIVE_HOME_PREFIXES` in `claudine/lib/src/protect/path.rs` to
  include:
  - [ ] `~/.aws`
  - [ ] `~/.kube`
  - [ ] `~/.docker/config.json`
  - [ ] `~/.netrc`
  - [ ] `~/.npmrc`
  - [ ] `~/.git-credentials`
  - [ ] `~/.config/gh`
  - [ ] `~/.claude`
  - [ ] `~/.codex`
  - [ ] `~/.gemini`
  - [ ] `~/.goose`
  - [ ] `~/.opencode`
  - [ ] `~/.qwen`
  - [ ] `~/.roo`
- [ ] Extend `SENSITIVE_PREFIXES` in `claudine/lib/src/protect/path.rs` to
  include:
  - [ ] `/bin`, `/sbin`, `/root`, `/opt` (Unix-like absolute);
  - [ ] `/Library/LaunchDaemons` (macOS absolute).
- [ ] Add OS-gated tests in `path.rs`:
  - [ ] Linux: writes under `/bin`, `/sbin`, `/root`, `/opt` are blocked.
  - [ ] macOS: writes under `/Library/LaunchDaemons` are blocked.
- [ ] Add cross-platform tests for all new home-relative credential and
  provider config paths.
- [ ] Update `claudine/docs/topics/protect-service.md` to list the new
  sensitive prefixes.

**Validation checkpoint:**

- [ ] `cargo nextest run -p claudine path::` passes on the current host.
- [ ] All new home-relative paths have dedicated assertions.

## Phase 5 — Custom Pattern Surface, MCP Bounds, and Final Sign-Off

*Goal: make custom patterns surface-aware, bound MCP scanning, and verify the
full package area.*

### 5.1 Add scan surface to custom patterns

- [ ] Add a `surface` field to `CustomPattern` in
  `claudine/lib/src/protect/config.rs`:
  - [ ] Default to `ScanSurface::BashCommand` for backward compatibility.
  - [ ] Accept `bash_command` and `mcp_response`.
  - [ ] Reject `write_path` and unknown surfaces at `ProtectConfig::validate`.
- [ ] Update `claudine/lib/src/protect/matcher.rs` to compile `mcp_response`
  custom patterns into the MCP evaluation path and `bash_command` custom
  patterns into the command evaluation path.
- [ ] Update `claudine/lib/src/protect/service.rs` so `evaluate_mcp_response`
  also checks the compiled custom MCP group.
- [ ] Add tests in `config.rs`, `matcher.rs`, and `mod.rs`:
  - [ ] `surface = "mcp_response"` blocks an MCP payload.
  - [ ] Omitted `surface` still applies to bash commands.
  - [ ] Unsupported surfaces fail config validation.

### 5.2 Do not advertise `allow_paths` for unreliable operand parsers

- [ ] Audit built-in rules in `claudine/lib/src/protect/catalog.rs` and set
  `supports_allow_paths = false` for rules whose operand grammar is not parsed
  by the `rm` heuristic (e.g. `find -delete`, `chmod`, `chown`).
- [ ] Update `claudine/docs/topics/protect-service.md` to document that
  `allow_paths` is ignored for those rules.
- [ ] Add a regression test in `mod.rs` or `service.rs` asserting that
  `find . -delete` is still blocked even when `allow_paths = ["."]`.

### 5.3 Bound MCP response scanning

- [ ] Define constants in `claudine/lib/src/protect/observe.rs`:
  - [ ] `MAX_SCAN_LEAVES` (e.g. 10,000);
  - [ ] `MAX_SCAN_BYTES` (e.g. 1 MiB);
  - [ ] `MAX_LEAF_BYTES` (e.g. 64 KiB).
- [ ] Implement a bounded `LeafCollector` that:
  - [ ] stops collecting after the leaf-count cap or total-byte cap;
  - [ ] truncates any single leaf to `MAX_LEAF_BYTES` (or remaining budget);
  - [ ] tracks whether truncation occurred.
- [ ] Update `extract_mcp_response_request` to use the bounded collector.
- [ ] Emit a `warn!` when a cap clips input; the warning must not include
  payload content.
- [ ] Update `claudine/docs/topics/protect-service.md` to document the caps.
- [ ] Add tests in `observe.rs`:
  - [ ] Oversized MCP responses are clipped to the configured caps.
  - [ ] A match in the retained prefix still blocks.
  - [ ] The cap warning does not leak response body content.

### 5.4 Final validation

- [ ] Run `just test` in the `claudine` package area and confirm all tests pass.
- [ ] Run `just lint` in the `claudine` package area and confirm no new
  warnings were introduced.
- [ ] Run `cargo fmt --check` (read-only) and confirm the touched files match
  the repo formatting baseline.
- [ ] Verify the acceptance criteria in [`spec.md`](./spec.md) are satisfied:
  - [ ] AC1: posture documented.
  - [ ] AC2: unparsed command/write tools handled defensively.
  - [ ] AC3: alternate payload keys covered by tests.
  - [ ] AC4: `allow_paths` uses boundary-aware/anchored matching.
  - [ ] AC5: sensitive-path catalog expanded.
  - [ ] AC6: `CustomPattern.surface` implemented and validated.
  - [ ] AC7: MCP scan caps documented and bounded.
  - [ ] AC8: `just test` passes.

## Parallelizable Work

Phases 2, 3, 4, and the three sub-sections of Phase 5 can run in parallel
once Phase 1 has established the shared posture language and regression-test
pattern. The only hard ordering constraints are:

- Phase 5.1 depends on the `ScanSurface` enum and `CompiledCatalog` shape
  (stable across the codebase).
- Phase 5.2 is easiest after Phase 3 has finalized the `allow_paths`
  semantics.
- Phase 5.4 must run after all code changes are in.

## Risks and Notes

- The existing tests already encode much of the intended behavior; review
  them before writing new tests to avoid duplication.
- Do not change `ProtectRequest` field shapes unless required by a downstream
  consumer; the public API is listed in `docs/topics/protect-service.md`.
- Keep all regex additions linear-time; avoid unbounded repetition or nested
  alternations that could explode match cost.
- Cross-platform path tests should be gated with `#[cfg(target_os = ...)]`
  when filesystem normalization differs.
