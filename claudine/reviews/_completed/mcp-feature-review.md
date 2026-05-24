# MCP Feature Review

This document reviews the implementation of "MCP mode" in the `claudine` and `claudine-cli` packages against the functional specification.

## Summary of Findings

The MCP implementation is substantially complete and follows the architectural goal of treating MCP as a cross-provider capability. However, several discrepancies between the implementation and the specification were identified, ranging from missing cleanup logic to terminology drift.

## Critical Issues (Priority: High)

### 1. Missing Cleanup Logic in Composition Commands
**Description:** The `McpInjector` trait defines a `cleanup` method intended to remove temporary configuration files (e.g., in the shadow-home directory) after execution. While this is correctly invoked in the standard `claudine <agent>` wrapper (`claudine/cli/src/commands/wrap/mod.rs`), it is **entirely missing** from the `claudine compose` and `claudine inline-compose` execution paths (`claudine/cli/src/commands/wrap/composition.rs`).
**Impact:** Leaves stale configuration files in the shadow-home directory after every composition run, potentially leading to configuration drift or disk clutter.
**Recommendation:** Implement `mcp_cleanup` logic in `execute_composition_request_inner` within `composition.rs`, following the pattern established in `mod.rs`.

---

## Technical Discrepancies (Priority: Medium)

### 2. Terminology Swap: `sync` vs `export`
**Description:** The specification defines `claudine mcp sync <provider>` as the command to write effective Claudine-managed MCP settings back into the provider's native config. The CLI implements this as `claudine mcp export`. Meanwhile, the CLI uses `claudine mcp sync` to refresh the local catalog from native configs (a "pull" operation).
**Impact:** Confusing for users following the official documentation.
**Recommendation:** Align CLI command names with the specification. Consider aliasing `export` to `sync` and renaming the current `sync` to `refresh` or similar.

### 3. Missing "Prefix" Match Tier
**Description:** The specification requires a deterministic resolution order for `#tags` that includes a "Prefix" match tier (using `starts_with`) between "Caseless Exact" and "Substring" matches. The implementation in `claudine/lib/src/mcp/catalog.rs` jumps directly from caseless exact matches to substring matches using `contains`.
**Impact:** Less predictable resolution when multiple servers share a common prefix (e.g., `#cal` matching both `calendar` and `local-calendar`).
**Recommendation:** Add a specific rank for prefix matches in `McpCatalogStore::resolve_outcome`.

### 4. Incomplete Normalized Exact Match
**Description:** The specification requires that "Normalized Exact Match" treats both case-insensitivity AND the equivalence of `-` and `_` as a match. The current implementation in `catalog.rs` only handles case-insensitivity via `eq_ignore_ascii_case`.
**Impact:** Tags like `#my_server` will fail to match a catalog entry named `my-server`.
**Recommendation:** Update the normalization logic in `catalog.rs` and `session.rs` to treat `-` and `_` as equivalent.

### 5. Incomplete Roo Code Support
**Description:** The `native_config_path` in `claudine/lib/src/mcp/export.rs` lacks support for the `RooCode` provider in the `User` scope. While `import.rs` correctly identifies the macOS path for Roo Code settings, the export logic is restricted to `Scope::Repo`.
**Impact:** Users cannot sync their global Claudine MCP defaults to Roo Code.
**Recommendation:** Add the platform-specific `User` scope path to `native_config_path` for `Provider::RooCode`.

---

## Minor Improvements (Priority: Low)

### 6. Overly Restrictive `#tag` Lexing
**Description:** The `lex_tags` function in `claudine/lib/src/mcp/session.rs` rejects candidate tags if they are immediately followed by punctuation (e.g., `#calendar,`). The specification defines terminal conditions only as whitespace or end-of-line.
**Impact:** Natural language usage like "check #calendar, then #slack" fails to extract the first tag.
**Recommendation:** Relax the lexer to allow punctuation as a terminal delimiter, or at least treat it as a valid terminator even if the punctuation itself is not part of the tag.

### 7. Missing Integration Tests for Default Layering
**Description:** While `load` and `save` are tested for defaults, there are no integration tests verifying the "REPLACE" strategy in `effective_defaults` where repo-scope defaults should entirely mask user-scope defaults.
**Impact:** Potential for regressions in the layering logic.
**Recommendation:** Add a test case to `claudine/lib/src/mcp/defaults.rs` that specifically verifies the interaction between user and repo defaults.
