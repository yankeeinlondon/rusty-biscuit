# Protect Refactor Review 2

Reviewed against:

- `spec.md`
- `tech-design.md`
- current implementation in `claudine/lib/src/services/protect/`
- current dispatch/config integration in `claudine/lib/src/dispatch/`

Validation run:

- `cargo test -p claudine protect --lib`
- `cargo test -p claudine dispatch --lib`

The major issues from the first review are mostly fixed. The remaining suggestions are narrower and mostly concern edge-case correctness, integration coupling, and a few still-light test areas.

## Findings

### 1. MCP JSON scanning can now create false positives across unrelated fields

Files:

- `claudine/lib/src/services/protect/observe.rs:59-72`

The MCP JSON fallback now recursively collects all string leaves and joins them with `"\n"` before scanning. That closes the old blind spot for structured JSON, but it introduces a new one: regexes can now match across field boundaries even when no single field contained the dangerous phrase.

Example:

- field A: `"ignore all"`
- field B: `"previous instructions"`

After joining, the scanner sees `"ignore all\nprevious instructions"`, and the current prompt-injection pattern will match because `\s+` spans newlines.

That means structured JSON can be blocked for a phrase that never actually appeared contiguously in the original MCP payload.

Suggested fix:

- scan each collected string leaf independently instead of concatenating them
- if you want cross-field context, do it in a second explicit pass with tighter rules rather than by implicit newline joining

Suggested tests:

- two safe-but-adjacent fields that currently join into a blockable phrase
- one nested field containing the whole dangerous phrase to ensure the intended case still blocks

### 2. Protect still depends on explicit event bindings, so it can silently become inert

Files:

- `claudine/lib/src/dispatch/mod.rs:286-323`

Protect evaluation still happens only after dispatch successfully finds a provider/event binding. If the binding is missing, disabled, or filtered out by the matcher, dispatch returns early before Protect runs.

That leaves a real integration gap:

- `settings.protect` can be enabled
- `ProtectService` can be compiled and cached
- but runtime protection still does nothing for that event if the corresponding binding is absent

This is especially easy to hit in repo scope because repo provider configs fully replace user provider configs. A repo config that omits `before_tool`/`after_tool`-style bindings can effectively disable Protect even though merged settings still say it is enabled.

Suggested fix:

- decouple Protect from action bindings for the relevant scan surfaces
- or, at minimum, fail validation when Protect is enabled but required tool-response/tool-request events are not bound for a provider that supports them

Suggested tests:

- dispatch with `settings.protect` enabled and no `BeforeTool` binding should still enforce Protect, or should fail config validation explicitly
- repo override that removes user tool bindings should have a regression test either way

### 3. Existing-ancestor canonicalization is still not implemented for write paths

Files:

- `claudine/lib/src/services/protect/service.rs:81-93`

The relative-path fix is in place, but the implementation still only does lexical normalization after string-concatenating `cwd` and `path`. The technical design explicitly called for canonicalizing existing ancestors when possible.

That remaining gap matters when:

- `cwd` is a symlinked path
- the relevant ancestor exists but resolves somewhere else
- lexical normalization alone produces a different result than filesystem resolution

In those cases, a write targeting a sensitive location can still be misclassified.

Suggested fix:

- canonicalize the deepest existing ancestor before final prefix comparison
- fall back to lexical normalization only when ancestor resolution is unavailable

Suggested tests:

- a tempdir with a symlinked working directory whose real path traverses into a sensitive target

## Robustness Suggestions

### 4. Runtime config silently disables Protect if `ProtectService::new()` fails

Files:

- `claudine/lib/src/dispatch/loader.rs:241-244`

`compile_runtime_config_with_messaging()` uses `.ok()` when constructing the cached `ProtectService`. If that constructor ever returns an error, runtime config creation succeeds and Protect is silently absent.

Today that is unlikely because config validation already ran, but this is still the wrong failure mode for a safety subsystem. A future built-in regex mistake or a constructor change would degrade into “Protect off” instead of “config load failed”.

Suggested fix:

- propagate the error instead of swallowing it

## Coverage Gaps

Coverage is much better than the first pass, but a few important integration cases are still missing:

- No dispatch integration test proves that a dangerous `BeforeTool` event produces a provider-native deny response.
- No dispatch integration test proves that a dangerous MCP `AfterTool` event populates `protect_post` and overrides the action response.
- No regression test covers the current binding-coupling behavior when Protect is enabled but the relevant binding is missing.
- No test covers the new JSON cross-field false-positive case in MCP scanning.
- No test exercises symlink or existing-ancestor canonicalization behavior for write paths.
