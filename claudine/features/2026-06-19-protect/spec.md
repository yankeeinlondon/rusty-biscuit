---
status: ready for planning and implementation
created: 2026-06-19
area: claudine
packages:
    - claudine
reviewed: true
source_review: ../_completed/2026-06-19-review-findings/spec.md
---

# Protect Posture and Extraction Hardening

This specification covers the Priority 2 `protect` remediation from the
comprehensive review. It is a posture decision plus a small hardening pass, not
a redesign of `protect` into a full shell security boundary.

Reader note: the pre-review draft recommended "Posture B" as a real boundary.
That conflicts with the current `protect` contract in
[`docs/topics/protect-service.md`](../../docs/topics/protect-service.md):
`protect` is a best-effort deny catalog layered on top of provider permission
systems and `claudine-contract` filesystem isolation. This reviewed spec keeps
that contract and hardens the cheap fail-open cases that make the current
best-effort layer weaker than documented.

## Goals

1. State clearly that `protect` is defense-in-depth, not the load-bearing
   security boundary.
2. Prevent command- and write-shaped tools from bypassing `protect` merely
   because their payload uses a common alternate key or array shape.
3. Tighten `allow_paths` so allow-list entries cannot accidentally permit
   unrelated sensitive paths.
4. Expand sensitive write-path coverage for common credential and provider
   configuration locations.
5. Make custom patterns explicit about which scan surface they target.
6. Bound MCP response scanning so hostile payload size cannot create excessive
   per-response work.

## Non-Goals

- Full shell parsing, variable expansion, glob expansion, or Windows command
  grammar support.
- Provider-specific complete schemas for every tool emitted by every supported
  agentic CLI.
- Reintroducing `ProtectPosture`, `ProtectSeverity`, ask/warn modes,
  per-invocation bypass, or `PolicyEngine` integration.
- User-configurable sensitive-path catalogs. The static built-in list is enough
  for this remediation unless implementation proves it creates unacceptable
  false positives.

## Acceptance Criteria

1. Module-level docs and `docs/topics/protect-service.md` describe `protect` as
   best-effort defense-in-depth and identify provider permissions/sandboxing as
   the load-bearing controls.
2. `extract_protect_request` no longer encodes every non-request as `None`.
   Command/write-shaped unparsed tools are represented distinctly and handled
   defensively at dispatch.
3. Alternate command and path payload keys are covered by tests.
4. `allow_paths` uses boundary-aware matching for absolute entries and anchored
   component-sequence matching for relative entries.
5. The sensitive-path catalog covers the added credential and provider config
   paths on supported platforms.
6. `CustomPattern` supports a `surface` field for `bash_command` and
   `mcp_response`, defaults to `bash_command`, and rejects unsupported surfaces
   clearly.
7. MCP response scanning has a documented cap on total string leaves, total
   scanned bytes, and per-leaf bytes.
8. `just test` passes in the `claudine` package area.

## P2.1 - Document Protect as Defense-in-Depth

- **Severity:** High
- **Confidence:** High
- **Locations:** `claudine/lib/src/protect/catalog.rs`,
  `claudine/lib/src/protect/matcher.rs`,
  `claudine/lib/src/protect/service.rs`,
  `claudine/lib/src/protect/mod.rs`,
  `claudine/docs/topics/protect-service.md`

### Problem

`protect` rules match regexes against literal, unparsed command text. The shell
later performs quoting, variable expansion, word splitting, globbing, and
command chaining. Ordinary shell variants can therefore bypass a raw regex
catalog even when the obvious form is blocked.

Examples include `rm -fr /`, `rm -rf / 2>/dev/null`, `\rm -rf /`,
`X=rm; $X -rf /`, case variants such as `curl ... | BASH`, and force-push
refspecs such as `git push origin +main`.

### Reviewed Design Decision

Keep `protect` as **best-effort defense-in-depth**. Do not claim it is a hard
security boundary in this remediation.

Making `protect` a real boundary would require shell-aware parsing,
provider-specific tool schemas, Windows command parsing, and compatibility
decisions for ambiguous tool payloads. That is a separate design effort. This
spec instead documents the real posture and closes low-risk fail-open gaps.

### Required Work

- Add explicit posture language to module docs and the protect service topic
  doc.
- Add a bypass-corpus test suite that encodes the chosen posture:
  obvious destructive commands are blocked; known non-boundary shell variants
  are either blocked where cheap or documented as outside the guarantee.
- Keep provider permission systems and `claudine-contract` sandboxing named as
  the load-bearing isolation controls.

## P2.2 - Distinguish Unparsed Command/Write Tools from No Opinion

- **Severity:** High
- **Confidence:** High
- **Locations:** `claudine/lib/src/dispatch/mod.rs`,
  `claudine/lib/src/protect/observe.rs`

### Problem

Protect blocks only when extraction produces a request and evaluation returns
`Block`. A tool can look command- or write-shaped and still bypass the guard if
its payload uses a key the extractor does not recognize.

Known gaps:

- command keys: `cmd`, `script`, `input`, and string arrays;
- command-like tool names: `run_command`, `terminal`;
- path keys: `filename`, `dest`, and `paths[]`.

### Required Work

Introduce an explicit observation type:

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

Dispatch must treat `Unparsed` from clearly command- or write-shaped tools as a
defensive block with a `warn!` and a secret-free reason. Unrelated tools remain
`NoOpinion` and continue normally.

Extraction must recognize at least:

- command payloads under `command`, `cmd`, `script`, `input`, or string arrays;
- write targets under `path`, `file_path`, `file`, `target`, `filename`,
  `dest`, or `paths[]`;
- command-shaped tool names containing `bash`, `shell`, or `exec`, plus exact
  or normalized names for `run_command` and `terminal`.

### Tests

- A Bash-like tool with command data under `cmd`, `script`, `input`, and a
  string array is scanned or defensively blocked, not silently allowed.
- A write-like tool with `filename`, `dest`, and `paths[]` is scanned or
  defensively blocked.
- An unrelated tool with no relevant payload remains `NoOpinion`.

## P2.3 - Tighten `allow_paths` Matching

- **Severity:** Medium
- **Confidence:** High
- **Location:** `claudine/lib/src/protect/path.rs`

### Problem

Relative allow entries must not match any same-named segment anywhere in an
absolute path. `allow_paths = ["build"]` must not permit
`/etc/build/passwd`. Absolute allow entries also need component boundaries, so
`allow_paths = ["/var/tmp"]` must not permit `/var/tmpevil`.

### Required Work

- Absolute allow entries match only the exact path or descendants separated by a
  path component boundary.
- Relative allow entries match an anchored component sequence for the evaluated
  target, preserving the common project-local cases such as `node_modules`,
  `target`, `dist`, `build`, and `.cache`.
- For destructive bash commands, all extracted target operands must be allowed;
  one unallowed target still blocks.

### Tests

- `/etc/build/passwd` is not allowed by `allow_paths = ["build"]`.
- `/var/tmpevil` is not allowed by `allow_paths = ["/var/tmp"]`.
- `rm -rf node_modules` and `rm -rf target` remain suppressible when explicitly
  allowed.

## P2.4 - Expand Sensitive Write-Path Coverage

- **Severity:** Medium
- **Confidence:** Medium
- **Location:** `claudine/lib/src/protect/path.rs`

### Problem

The sensitive-path list is the only write-path guard. It should cover common
credential files, provider config directories, and high-impact system
locations.

### Required Work

Add built-in coverage for at least:

- home-relative: `~/.aws`, `~/.kube`, `~/.docker/config.json`, `~/.netrc`,
  `~/.npmrc`, `~/.git-credentials`, `~/.config/gh`, `~/.claude`, `~/.codex`,
  `~/.gemini`, `~/.goose`, `~/.opencode`, `~/.qwen`, `~/.roo`;
- Unix-like absolute paths: `/bin`, `/sbin`, `/root`, `/opt`;
- macOS absolute paths: `/Library/LaunchDaemons`.

Platform-shaped paths may be harmlessly present cross-platform if matching is
purely lexical, but tests should be OS-gated when path normalization differs.

### Tests

Writes to the added credential and provider config paths are blocked. Added
absolute paths are covered with OS-appropriate tests.

## P2.5 - Add Scan Surface to Custom Patterns

- **Severity:** Medium
- **Confidence:** High
- **Locations:** `claudine/lib/src/protect/config.rs`,
  `claudine/lib/src/protect/matcher.rs`,
  `claudine/lib/src/protect/service.rs`

### Problem

Custom patterns currently apply only to bash commands. A user who expects a
custom pattern to block an MCP response gets no block and no clear signal that
the pattern is on the wrong surface.

### Required Work

Add a `surface` field to `CustomPattern`.

Supported values:

- `bash_command`
- `mcp_response`

The default is `bash_command` for backward compatibility. Reject `write_path`
and unknown surfaces at config validation with a clear error unless write-path
custom matching is implemented in the same change.

Route `mcp_response` custom patterns through the MCP response evaluation path.

### Tests

- A custom pattern with `surface = "mcp_response"` blocks an MCP payload.
- A custom pattern with no `surface` still applies to bash commands.
- Unsupported surfaces fail config validation.

## P2.6 - Do Not Advertise `allow_paths` for Unreliable Operand Parsers

- **Severity:** Low
- **Confidence:** Medium
- **Location:** `claudine/lib/src/protect/path.rs`,
  built-in rules with `supports_allow_paths`

### Problem

The current target extractor is effectively an `rm` operand heuristic. Rules
such as `find ... -delete`, `chmod`, and `chown` have different operand
grammars. Advertising `allow_paths` for those rules makes suppression unreliable
and can mislead users.

### Required Work

For this remediation, mark `supports_allow_paths = false` for rules whose target
grammar is not parsed correctly, unless a small dedicated extractor for that
command is added in the same change. Document the limitation in the protect
service topic doc.

### Tests

`find . -delete` with `allow_paths = ["."]` must not silently claim reliable
suppression unless a dedicated `find` extractor exists. `rm`-shaped
allow-path behavior remains covered by P2.3.

## P2.7 - Bound MCP Response Scanning

- **Severity:** Low
- **Confidence:** Medium
- **Locations:** `claudine/lib/src/protect/observe.rs`,
  `claudine/lib/src/protect/matcher.rs`

### Problem

MCP responses are untrusted. Walking every string leaf and matching every regex
against every byte can create excessive CPU work for a large hostile response,
even though Rust's `regex` engine avoids catastrophic backtracking.

### Required Work

Set explicit scan caps for MCP responses:

- maximum string leaves scanned;
- maximum total scanned bytes;
- maximum bytes per leaf.

When a cap is reached, truncate the scan input, continue evaluating the retained
prefixes, and emit a `warn!` that does not include secret payload content.
Document the caps in the protect service topic doc.

### Tests

- Oversized MCP responses are clipped to the configured caps.
- A match in the retained prefix still blocks.
- The cap warning is emitted without leaking the response body.

## Open Questions

No open questions block planning and implementation. The larger question of
turning `protect` into a real shell security boundary is intentionally deferred
to a separate feature because it requires provider-specific schemas, shell
grammar design, and cross-platform command parsing.
