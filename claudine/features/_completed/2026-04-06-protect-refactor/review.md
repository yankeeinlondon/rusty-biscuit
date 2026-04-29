# Protect Refactor Review

Reviewed against:

- `spec.md`
- `tech-design.md`
- current implementation in `claudine/lib/src/services/protect/`
- dispatch/config integration in `claudine/lib/src/dispatch/`

Validation run:

- `cargo test -p claudine protect --lib`
- `cargo test -p claudine dispatch --lib`

## Findings

### 1. `allow_paths` can bypass rules that were explicitly marked non-bypassable

Files:

- `claudine/lib/src/services/protect/matcher.rs:20-28`
- `claudine/lib/src/services/protect/matcher.rs:83-99`
- `claudine/lib/src/services/protect/service.rs:50-62`
- `claudine/lib/src/services/protect/catalog.rs:202-208`

`RuleDefinition.supports_allow_paths` is defined per rule, but compilation collapses that into a single group-wide boolean with `any()`. At evaluation time, `ProtectService` checks only `group.supports_allow_paths`, not whether the matched rule itself supports an allowlist bypass.

That means a config like:

```json
{
  "protect": {
    "rules": {
      "filesystem_destruction": {
        "enabled": true,
        "allow_paths": ["boot"]
      }
    }
  }
}
```

can suppress the `rm_boot` rule even though `rm_boot` is explicitly declared with `supports_allow_paths: false`.

This is a correctness bug, not just a design nit. The implementation currently allows users to punch holes through rules that the catalog author marked as non-bypassable.

Suggested fix:

- carry `supports_allow_paths` through the matched rule path, not just the compiled group
- make allowlist suppression rule-specific
- add regression tests for `rm_boot` and any future rule marked `supports_allow_paths: false`

### 2. MCP prompt-injection scanning is both over-broad and under-implemented

Files:

- `claudine/lib/src/services/protect/observe.rs:14-20`
- `claudine/lib/src/services/protect/observe.rs:50-55`
- `claudine/lib/src/events/event_meta.rs:80-92`

The design says Protect should scan only MCP-originated response payloads. The implementation does not check that.

Current behavior:

- every `AfterTool` and `AfterModel` event is treated as an MCP candidate
- any string `tool_response` is scanned, regardless of whether the tool was MCP-backed
- non-string JSON responses are skipped entirely with a `future enhancement` comment

That creates two distinct gaps:

1. False positives: normal shell/file/tool output can be blocked by MCP prompt-injection regexes.
2. False negatives: structured MCP responses are not scanned at all, even though prompt injection is likely to appear inside JSON fields.

There is already a `ToolName::is_mcp_tool()` helper in `event_meta.rs`, but Protect extraction does not use it.

Suggested fix:

- gate MCP scanning on actual MCP tool identity
- recursively scan string leaves inside object/array payloads
- add integration tests proving:
  - non-MCP `AfterTool` output is ignored
  - MCP string payloads are scanned
  - MCP JSON payloads are scanned

### 3. Sensitive-path enforcement can be bypassed with relative paths

Files:

- `claudine/lib/src/services/protect/service.rs:10-13`
- `claudine/lib/src/services/protect/service.rs:75-92`
- `claudine/lib/src/services/protect/path.rs:24-45`
- `claudine/lib/src/services/protect/path.rs:54-77`

The tech design calls for:

1. extracting the tool path
2. expanding `~`
3. resolving relative paths against the event working directory
4. canonicalizing existing ancestors when possible
5. lexically normalizing `.` and `..`

The current implementation only does step 2 plus a lexical normalize. `ProtectRequest::WritePath` does not even carry `cwd`, so `SensitivePathChecker` has no way to resolve relative paths against the event context.

Practical consequence: a write/edit/delete aimed at something like `../../.ssh/config` or `../etc/hosts` can evade `sensitive_paths` if the working directory is inside the relevant tree.

Suggested fix:

- include `cwd` in `ProtectRequest::WritePath`
- resolve relative paths against `cwd`
- canonicalize existing ancestors when possible before prefix comparison
- add tests for relative traversal into `~/.ssh`, `/etc`, `/var`, and `/usr`

### 4. Exact sensitive directory targets do not match

Files:

- `claudine/lib/src/services/protect/path.rs:3-9`
- `claudine/lib/src/services/protect/path.rs:24-45`

All sensitive prefixes end with `/`, and matching is a simple `starts_with(prefix)`.

That means these paths are currently not considered sensitive:

- `/etc`
- `/var`
- `/usr`
- `/boot`
- `/System`
- `~/.ssh`
- `~/.gnupg`

For write/edit this is bad enough; for delete-like tools it is worse, because deleting the directory root itself is exactly the kind of operation Protect should stop.

Suggested fix:

- treat both the directory root and descendants as sensitive
- add tests for exact-directory targets, not just descendants like `/etc/passwd`

### 5. Protect config merging is not field-by-field and drops user settings

Files:

- `claudine/lib/src/dispatch/loader.rs:462-476`

`merge_protect_configs()` clones the repo config and only preserves one user-level bit: `enabled = true`.

Everything else from the user config is discarded when a repo config exists:

- per-group toggles
- `allow_paths`
- `custom_patterns`

That does not match the repo-wide merge behavior described elsewhere in the codebase, and it breaks a core Protect feature: user custom deny patterns disappear silently in repos that define any Protect config at all.

Suggested fix:

- merge `enabled`, per-group toggles, `allow_paths`, and `custom_patterns` explicitly
- add unit tests for user-only, repo-only, and mixed protect configs

### 6. `sensitive_paths` rejects `allow_paths`, which does not match the technical design

Files:

- `claudine/lib/src/services/protect/config.rs:96-120`

The tech design says filesystem-targeted groups should accept `allow_paths`. `sensitive_paths` is one of those groups, but validation explicitly rejects it.

That may be intentional, but if so it should be documented as a design change. Right now it reads as an implementation gap, and it removes a useful escape hatch for trusted automation that must touch a known sensitive path.

Suggested fix:

- either support `allow_paths` for `sensitive_paths`
- or explicitly update the spec/design/docs to say this group is intentionally non-allowlistable

### 7. Removed config fields are silently ignored instead of rejected or translated

Files:

- `claudine/lib/src/services/protect/config.rs:31-72`
- `claudine/lib/src/services/protect/mod.rs:22-35`

The technical design offered two acceptable migration paths:

- explicit reject/translate compatibility handling
- a hard cut

The current behavior is neither. Unknown top-level Protect fields are silently ignored because the expanded form does not deny unknown fields, and there is even a regression test that codifies silent acceptance of `"posture"`.

That creates a bad migration story:

- old config appears to load successfully
- removed fields have no effect
- the user receives no signal that Protect is configured differently than they think

Suggested fix:

- reject unknown top-level fields in the expanded Protect config
- or implement an explicit compatibility shim with warnings/errors for removed keys

## Test Coverage Gaps

The protect-specific unit tests are decent for happy paths, but the coverage is still light in the places most likely to regress:

- No tests exercise `merge_protect_configs()` in `dispatch/loader.rs`.
- No dispatch integration tests assert that Protect actually blocks `BeforeTool` bash/path requests.
- No dispatch integration tests assert that post-tool MCP blocking only happens for MCP-originated responses.
- No tests cover structured JSON MCP payloads.
- No tests cover relative-path traversal into sensitive locations.
- No tests cover exact sensitive directory roots like `/etc` and `~/.ssh`.
- No tests cover the per-rule `supports_allow_paths: false` contract using a rule like `rm_boot`.

Given the spec’s expectation of strong unit and integration coverage, these gaps are significant.

## Ergonomics And Performance

### Cache `ProtectService` in the runtime config

Files:

- `claudine/lib/src/dispatch/mod.rs:322-323`

Dispatch currently reconstructs `ProtectService` from config on every event. That means recompiling the catalog and regex sets repeatedly inside long-running wrapper sessions.

`DispatchRuntimeContext` already caches the compiled runtime config. Protect should be cached there too.

Suggested improvement:

- compile `ProtectService` once during runtime-config construction
- store it in `RuntimeConfig`
- reuse it across all dispatches in the same wrapper process

This is both more ergonomic and materially better for steady-state wrapper performance.

### Validate config inside `ProtectService::new`

Files:

- `claudine/lib/src/services/protect/service.rs:27-35`
- `claudine/lib/src/services/protect/config.rs:96-129`

Today, config validation only happens when Protect arrives through `HookerConfig::validate()`. Library callers that construct `ProtectService` directly do not get the same validation guarantees.

Suggested improvement:

- call `config.validate()` inside `ProtectService::new()`

That makes the public API safer and easier to use correctly.
