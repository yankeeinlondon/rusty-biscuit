# MCP Mode Final Review

**Date:** 2026-03-10  
**Scope:** Consolidated recommendations from:
- `claudine/reviews/mcp-feature-review.md`
- `claudine/reviews/mcp-spec-review.md`
- `claudine/reviews/mcp-test-review.md`

## Verdict

The MCP refactor is close to complete, but it should not be treated as fully complete yet.

The remaining work falls into three categories:

1. **Correctness and lifecycle gaps** that can leave MCP state inconsistent or produce incorrect repo-scoped behavior
2. **UX/spec alignment gaps** where the implementation is mostly present but still behaves in a surprising or under-documented way
3. **Test coverage debt** large enough that the remaining MCP behavior is not yet protected against regression

The recommendations below merge duplicate findings across the three reviews and resolve a few ambiguities where the reviews overlap.

## Completion Standard

MCP mode should be considered complete for v1 when all of the following are true:

- bootstrap, defaults resolution, tag resolution, removal, export, and runtime injection behave consistently
- repo-scoped behavior only activates when actually inside a repo
- runtime injection does not leave avoidable persistent state behind
- CLI behavior matches the documented contract closely enough that users are not surprised
- the core wrapper and CLI flows are covered by integration tests, not just unit tests

## Final Recommendations

### 1. Fix repo-root detection before anything else

**Priority:** High

`current_repo_root()` appears to always return `Some(...)`, which can make non-repo directories look repo-scoped. This is the most important unresolved issue because it can create or load repo defaults where no repo context actually exists.

**Recommendation:**
- Change repo detection so non-repo directories return `None`
- Audit all MCP init/default/export/bootstrap call sites that branch on repo presence
- Add tests for both:
  - inside a real git repo
  - outside any repo

**Why this is first:** This is the only finding that directly threatens core scoping correctness.

### 2. Make `mcp remove` clean up defaults automatically

**Priority:** High

Removing a catalog entry without scrubbing it from `~/.claudine/mcp/defaults.json` and `<repo>/.claudine/mcp.json` leaves the system in a noisy, inconsistent state.

**Recommendation:**
- When removing a server ID from the catalog, automatically remove that ID from:
  - user defaults
  - repo defaults for the current repo, when present
- When removing an alias, report the owning server and any remaining aliases
- Keep provider-native config cleanup optional and separate; that is not required for v1 completion

**Why this is high priority:** It fixes a persistent state integrity problem that users will keep tripping over.

### 3. Wire injector cleanup into wrapper teardown

**Priority:** High

Codex and Gemini already implement MCP cleanup for shadow-home injection, but the wrapper lifecycle does not invoke it. This is dormant correctness work, not missing design work.

**Recommendation:**
- Call `cleanup()` after wrapped execution for providers that write shadow-home config
- Preserve cleanup failures as non-silent outcomes:
  - either surface a warning
  - or fail only if the team wants cleanup treated as a hard guarantee
- Add integration coverage for non-dry-run execution that verifies cleanup actually occurs

**Why this is high priority:** The code already exists, and leaving temp provider state behind is unnecessary.

### 4. Finish the repo-default init re-entry UX

**Priority:** Medium

When `claudine mcp init` is re-run in a repo, the flow does not meaningfully show the already-selected user defaults before prompting for repo defaults, even though the spec expects that context.

**Recommendation:**
- Use the existing `current` parameter in `prompt_for_defaults(...)`
- Show user defaults during repo re-entry, either by:
  - preselecting them in the prompt, or
  - displaying them clearly before repo selection
- Add tests for the re-entry path

**Why this matters:** This is the main remaining usability gap in initialization.

### 5. Resolve the ambiguous-tag cancel behavior explicitly

**Priority:** Medium

Interactive ambiguous-tag prompting currently appears to hard-error if the user dismisses the selection prompt. That is stricter than the intended UX described in the spec review.

**Recommendation:**
- In interactive, non-strict mode:
  - if the user cancels disambiguation, warn and drop that tag
- In non-interactive mode and `--strict` mode:
  - keep ambiguity as a hard error
- Document this behavior explicitly

**Why this is medium priority:** The current behavior is safe, but it is rougher than the intended interactive workflow.

### 6. Remove the deprecated `claudine mcp sync <provider>` compatibility path

**Priority:** Medium

The positional `<provider>` form for `sync` currently redirects to export with a deprecation warning. That behavior works, but it muddies the contract between pull-style sync and push-style export.

**Recommendation:**
- Remove the positional provider form from `sync`
- Keep `sync` as catalog refresh only
- Keep `export <provider>` as the only push path
- Update help text and command docs accordingly

**Why this is medium priority:** This is mostly about making the CLI contract coherent before the interface hardens.

### 7. Lock the defaults policy and document it consistently

**Priority:** Medium

One review flagged user+repo merging as an open design question. The current MCP support documentation already states that repo defaults **replace** user defaults, and the implementation follows that policy.

**Recommendation:**
- Treat **replacement** as the v1 behavior unless product direction changes
- Update the more ambiguous catalog/spec docs so they say this clearly
- Add tests that assert replacement behavior

**Why this is the right resolution:** This is already the documented behavior in `claudine/docs/mcp-support.md`, so the remaining task is consistency, not redesign.

### 8. Close the critical test gaps before calling MCP mode done

**Priority:** High

The implementation coverage is decent at the unit level, but completion needs integration coverage for the real user-facing paths.

**Minimum test set required for completion:**
- `--strict` behavior:
  - ambiguous tag
  - missing tag
- reactive bootstrap from wrapper `--mcp`
- repo vs non-repo bootstrap/default behavior
- `mcp list --alias`
- `mcp config <id-or-alias>`
- `mcp remove`:
  - full server removal
  - alias-only removal
  - cascading default cleanup
- validation for missing catalog IDs referenced by defaults
- runtime injection behavior:
  - multiple server injection
  - cleanup after execution
  - unsupported-provider fallback message
- defaults policy test:
  - repo defaults replace user defaults

**Recommended but not blocking:**
- doctests for the public MCP functions
- xxHash fallback naming determinism tests
- additional lexing edge cases

**Why this is high priority:** The remaining risk is not just missing code; it is missing regression protection around the wrapper lifecycle.

## Documentation Follow-up

These are not major blockers, but they should be cleaned up in the same phase:

- make all MCP docs state that repo defaults replace user defaults
- clarify tag termination behavior around punctuation
- clarify that xxHash is used for fallback naming while fingerprinting may use a different hash
- remove or update any outdated “not implemented” status notes in MCP feature docs

## Not Required For v1 Completion

These are worthwhile, but they should stay out of the critical path for calling MCP mode complete:

- Claude runtime MCP injection research
- Goose / Kimi / Qwen MCP support
- live MCP server health checks
- tool discovery for `enabled_tools` / `disabled_tools`
- auto-sync via config file watching
- broad ergonomics refactors like `Display` impls and builder cleanup

Those items belong in the next expansion phase, not the v1 completion checklist.

## Recommended Execution Order

1. Fix repo-root detection
2. Cascade cleanup in `mcp remove`
3. Wire injector cleanup into wrapper teardown
4. Add the high-priority integration tests around those behaviors
5. Finish init re-entry UX and ambiguous-tag cancel handling
6. Remove deprecated `sync <provider>` behavior
7. Align docs and feature-status notes

## Final Assessment

The MCP mode refactor is structurally strong and already covers most of the intended surface area. The remaining work is concentrated and practical: fix one real scoping bug, close two lifecycle/state-integrity gaps, finish a few CLI UX edges, and add the integration coverage that proves the wrapper behavior is stable.

Once those items are complete, MCP mode can reasonably be called complete for v1 without waiting on new provider research or broader feature expansion.
