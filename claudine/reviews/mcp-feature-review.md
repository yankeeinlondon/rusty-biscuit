# MCP Feature Review

## Overview

Based on a comprehensive review of the features specified in `@claudine/docs/mcp-support.md` and their implementations within the `claudine` package and `claudine-cli`, the MCP module appears to be remarkably complete and strictly adheres to the functional specification. 

All core workflows, including import/sync fingerprint resolution, export capabilities with backups, wrapper runtime injection, and catalog management commands, are faithfully implemented. Provider-specific logic properly aligns with their distinct architectures (e.g., OpenCode environment variable injection vs. Codex/Gemini shadow-home file configurations). 

However, there are a few areas where the implementation, while matching the documented "limits", presents minor gaps in UX or unused code that could be easily resolved to improve the system's robustness.

## Findings & Recommendations

### 1. Unused Cleanup Logic in Wrapper Runtime
**Description:** The specification mentions as a limit: *"Shadow-home runtime injection for Codex and Gemini writes under `~/.claudine` and currently leaves those shadow config files in place after the wrapped process exits."* 
While this is documented behavior, an inspection of `claudine/lib/src/mcp/inject.rs` reveals that a `cleanup(&self, result: &InjectionResult)` method is already fully implemented for `CodexInjector` and `GeminiInjector`. However, these cleanup routines are never invoked by the wrapper lifecycle (e.g., in `claudine/cli/src/commands/wrap/mod.rs` or `hooks.rs`). The code exists to clean up the shadow files but lies dormant.
**Priority:** **Low**
**Recommendation:** Integrate the existing `cleanup` trait method into the wrapper teardown phase or process exit hooks so that the shadow directories (`~/.claudine/.codex` and `~/.claudine/.gemini`) do not leak persistent state across sessions. Once wired up, the documented limitation can be removed from the specification.

### 2. Orphaned References After Server Removal
**Description:** As stated in the limits: *"claudine mcp remove deletes the catalog entry only. It does not clean defaults or native provider configs for you."* 
This implementation behavior can lead to a degraded UX over time. If a user deletes a server that is currently active in their `user` or `repo` defaults, the ID remains orphaned in `defaults.json` or `.claudine/mcp.json`. This surfaces later as persistent warnings during `claudine mcp check` or when launching wrapped commands until the user manually fixes their defaults. 
**Priority:** **Medium**
**Recommendation:** Enhance the `claudine mcp remove` command (in `claudine/cli/src/commands/mcp.rs`) to automatically scrub the deleted server's ID from `user` and `repo` defaults (`~/.claudine/mcp/defaults.json` and `<repo>/.claudine/mcp.json`). If desired, an optional flag (e.g., `--cascade`) could be introduced to also prune the deleted server from provider-native configs (`provider-state.json`), but updating the `defaults` files is the highest priority to prevent systemic warning noise.

### 3. Missing `sync` CLI Argument Clarity
**Description:** The CLI accepts `claudine mcp sync <provider>`, but immediately emits a warning: *"Warning: `claudine mcp sync <provider>` is deprecated; use `claudine mcp export <provider>`."* and routes it internally to `run_export`. While functional, the documentation defines `sync` as "Refresh the catalog from provider configs", which logically sounds like a pull/import action, whereas `export` is a push. Overloading `sync` to do an `export` might cause user confusion.
**Priority:** **Low**
**Recommendation:** Formally deprecate and remove the `<provider>` positional argument from `claudine mcp sync` entirely to enforce the separation of concerns: `sync` strictly pulls from the native environment to refresh the catalog, and `export` strictly pushes catalog effective defaults back to the native environment.

---
*Review complete. All other listed provider coverages, command family endpoints, and session resolution paths (`#tags`, `--use`) operate seamlessly exactly as defined in the spec.*