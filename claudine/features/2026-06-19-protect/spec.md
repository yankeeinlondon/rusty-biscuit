# Priority 2 — `protect` Security Posture

This is a **decision plus implementation**, not a single patch. The review's position: `protect` is currently relied upon as a curated security control but is trivially bypassable and fails open. The team must choose Posture A or Posture B and make the choice explicit and tested. **Recommended: Posture B for the load-bearing rules, with Posture A framing for the catalog as a whole** — i.e. document best-effort at module level *and* harden the parts that must hold plus fail closed on unparseable command/write tools.

## Decision Required

- **Posture A (best-effort):** Explicitly label the catalog as   defense-in-depth, not a security boundary, at the module level. Make fail-open   intentional and tested. Lowest effort; honest about the weak boundary.
- **Posture B (real boundary):** Tokenize commands, split on shell separators,   add `(?i)`, fail closed on unparseable command/write tools, and tighten   allow-path matching.

**DECISION:** Posture B should be taken

The remaining P2 items implement the union needed regardless of choice (documentation + bypass-corpus tests are mandatory in both; the hardening items are mandatory under B and recommended under A for the rules that must hold).

## P2.1 — Deny catalog is trivially bypassable (must be framed correctly)

- **Severity:** High · **Confidence:** high
- **Location:** `claudine/lib/src/protect/catalog.rs`;   `claudine/lib/src/protect/matcher.rs:90-109` (`Regex::find` on raw string);   `claudine/lib/src/protect/service.rs:76-120`;   module framing at `protect/mod.rs:1-2`.

### Problem

Every rule runs a regex over the **literal, unparsed** command string. The executing shell performs word-splitting, quote/variable expansion, and chaining the regex never sees. Known evasions defeat the catalog:

- `rm -rf / 2>/dev/null` (defeats the `$` anchor on `rm_root`)
- `rm -fr /`, `rm  -rf  /` (flag order / whitespace)
- `\rm -rf /` (leading backslash)
- `X=rm; $X -rf /` (variable indirection)
- `curl ...|BASH` (case — only MCP rules use `(?i)`)
- `git push origin +main` (refspec force — uncovered)

The bypassability is undocumented while prior project work re-engineered `protect` as the curated security control.

### Proposed Solution

1. **Document** at the module level (`protect/mod.rs`) that the catalog is    best-effort/defense-in-depth layered atop the provider's own permission    system, **not** a security boundary.
2. For rules that must hold, reuse the existing `tokenize_command_words`    (`permissions/query.rs`): split on shell separators (`;`, `&&`, `||`, `|`),    scan each segment, anchor on **tokens** not substrings.
3. Add `(?i)` to command rules (match the MCP-rule convention).
4. Add a **bypass-corpus test** that documents the real (post-fix) boundary and    locks it.

### Tests

Bypass corpus asserting *failure to bypass* for: `rm  -rf  /`, `rm -fr /`, `RM -RF /`, `\rm -rf /`, `X=rm;$X -rf /`, `rm -rf / 2>/dev/null`, `curl ...|BASH`, `git push origin +main`. Under Posture A, the corpus instead documents which of these are *intentionally* not caught.

## P2.2 — `protect` fails open on unrecognized tool/command shapes

- **Severity:** High · **Confidence:** high
- **Location:** `claudine/lib/src/dispatch/mod.rs:261-284` and `:350-374`;   `claudine/lib/src/protect/observe.rs:52-101`.

### Problem

Protect blocks only when extraction returns `Some` *and* evaluation returns blocked. Anything preventing extraction silently allows the action:

- `extract_command_string` only reads a `command` key (or bare string); a   Bash-family tool nesting its command under `cmd`/`script`/`input`/an array   yields `None` → allowed.
- Tool-name gating is substring-based (`contains("bash"|"shell"|"exec")`), so a   provider tool named `run_command`/`terminal` is never scanned.
- `extract_path_string` checks only `["path","file_path","file","target"]`; a   write tool using `filename`/`dest`/`paths[]` bypasses the sensitive-path guard.

Both dispatch call sites use `extract_protect_request(...)?` — a `None` request short-circuits to "no decision" → no block. There is no fail-closed branch.

### Proposed Solution

1. For a security control, the unparseable/unknown case on `BeforeTool` /    `PermissionRequest` for command- or write-shaped tools should **fail closed**    (or at least Ask) with a loud `warn!`.
2. Broaden key coverage: command keys to include `cmd`/`script`/`input` and    array forms; path keys to include `filename`/`dest`/`paths[]`.
3. Broaden tool-name gating beyond the three substrings (or, per the    Rust-idiomaticity note below, model the decision type so "couldn't parse" is    distinct from "parsed + allowed").

### Tests

A Bash-like tool with the command under `cmd`/`script`/array, and a write tool with the path under `dest`/`filename`, must be **blocked-or-ask**, not silently allowed.

## P2.3 — Path allow-list matching is too loose

- **Severity:** Medium · **Confidence:** high
- **Location:** `claudine/lib/src/protect/path.rs:173-188`.

### Problem

- Relative allow entries match any path **segment**: `allow=["build"]` permits   `/etc/build/passwd`.
- Absolute entries use bare `target.starts_with(allowed)` with no boundary:   `allow=["/var/tmp"]` permits `/var/tmpevil`.

Combined with P2.1 this widens the hole.

### Proposed Solution

Require a path boundary for absolute entries (reuse `is_prefix_match`); for relative entries match an **anchored component sequence**, not any-segment- anywhere.

### Tests

`allow=["build"]` must not permit `/etc/build/passwd`; `allow=["/var/tmp"]` must not permit `/var/tmpevil`.

## P2.4 — Sensitive write-path prefix list omits high-value credential locations

- **Severity:** Medium · **Confidence:** medium
- **Location:** `claudine/lib/src/protect/path.rs:14-29`.

### Problem

This is the only write-path control. The home-relative list is just `.ssh`/`.gnupg`, omitting `.aws`, `.kube`, `.docker/config.json`, `.netrc`, `.npmrc`, `.git-credentials`, `.config/gh`, and the provider config dirs. The absolute list omits `/Library/LaunchDaemons`, `/sbin`, `/bin`, `/opt`, `/root`.

### Proposed Solution

Extend both lists (at least `.aws`, `.kube`, `.netrc`, `.git-credentials`, `.npmrc`, `/Library/LaunchDaemons`, `/sbin`, `/bin`). Consider making the home-relative list configurable. Mind cross-platform: the absolute additions are Unix-shaped; gate or document Windows behavior.

### Tests

Each newly-added prefix is blocked for a write target.

## P2.5 — Custom protect patterns only apply to bash commands (silent scope trap)

- **Severity:** Medium · **Confidence:** high
- **Location:** `claudine/lib/src/protect/matcher.rs:74-82` (`compile_custom`   hardcodes `ScanSurface::BashCommand`); `evaluate_mcp` never consults custom   patterns.

### Problem

A user adding a `custom_patterns` rule to block an exfiltration phrase in MCP output finds it silently ignored — the operator believes a deny rule is active when it is not.

### Proposed Solution

Let `CustomPattern` declare a `surface` (default `BashCommand`) and route accordingly so `evaluate_mcp` consults MCP-surface custom patterns. If the bash-only limit is intentional, document it and add a pinning test.

### Tests

A `custom_patterns` rule targeting the MCP surface fires on an MCP response; default (no surface) still applies to bash.

## P2.6 — `extract_target_paths` mis-parses non-`rm` operands; `find -delete` allow_paths dead

- **Severity:** Low · **Confidence:** medium
- **Location:** `claudine/lib/src/protect/path.rs:139-159`.

### Problem

The extractor is an `rm`-shaped flag-skip heuristic, but `find_delete`/`chmod`/ `chown` rules advertise `supports_allow_paths` while their operand grammar differs, so allow_paths almost never suppresses for them. `rm -rf ./*` is a single literal token (not glob-expanded), so `allow=["."]` won't match.

### Proposed Solution

Set `supports_allow_paths=false` for `find_delete`/`chmod`/`chown` (unreliable extraction) **or** implement per-command operand parsing. Document the limit.

### Tests

`find . -delete` with allow_paths to pin real behavior.

## P2.7 — MCP prompt-injection scan has no payload size/leaf cap (DoS surface)

- **Severity:** Low · **Confidence:** medium
- **Location:** `claudine/lib/src/protect/observe.rs:52-93`, `matcher.rs:90-109`.

### Problem

`collect_json_strings` gathers every string leaf of an untrusted MCP response with no cap, then runs the RegexSet over each. Rust's `regex` is linear (no catastrophic backtracking), but a multi-MB hostile response × user `custom_patterns` is O(payloads × patterns × len) CPU per tool response.

### Proposed Solution

Cap total scanned bytes / leaf count; truncate oversized leaves. Document the linear-time guarantee as the reason builtins are safe.

### Tests

A response exceeding the cap is truncated/limited; scan completes within bound.

---
