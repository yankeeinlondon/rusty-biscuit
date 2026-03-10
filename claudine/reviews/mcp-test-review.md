# MCP Mode Test Coverage Review

This document reviews the current test coverage for the MCP Mode functionality as specified in:
- `@claudine/docs/cli/mcp-mode.md` - Tag parsing, matching, and resolution
- `@claudine/docs/cli/mcp-catalog.md` - Catalog management and CLI commands

## Summary of Current Coverage

The MCP module has **adequate unit test coverage** for core functionality (catalog CRUD, validation, session computation, tag lexing) but is **lacking in several critical areas**:

1. **No doctests** in any MCP module
2. **Missing integration tests** for several CLI commands
3. **Incomplete coverage** for edge cases in tag matching
4. **No tests** for `--strict` flag behavior
5. **No tests** for reactive initialization from `--mcp` flag
6. **Missing tests** for unimplemented features (these should be noted but are lower priority)

---

## Critical Gaps

### 1. No Doctests in MCP Module

**What**: Every public function in the MCP library lacks doctests.

**Gap**: The spec documents complex behaviors (e.g., tag syntax, matching priority) that would benefit from inline examples.

**How to fix**: Add doctests to key functions in `lib/src/mcp/`:

- `session.rs`: Add doctest to `lex_tags()` demonstrating valid/invalid tag formats
- `catalog.rs`: Add doctest to `resolve()` showing exact/caseless/substring resolution priority
- `defaults.rs`: Add doctest showing user/repo scope layering
- `validation.rs`: Add doctest showing validation report structure

Example for `lex_tags`:
```rust
/// Lex `#tags` from a prompt string according to the MCP CLI spec.
///
/// A valid tag:
/// - starts with `#` at the start of the prompt or after whitespace
/// - has an alphabetic first character after `#`
/// - contains only alphanumeric, `-`, or `_` afterwards
/// - ends at whitespace or end-of-line
///
/// # Examples
///
/// ```
/// let (cleaned, tags) = claudine::mcp::session::lex_tags("fix #calendar integration");
/// assert_eq!(tags, vec!["calendar"]);
/// assert_eq!(cleaned, "fix integration");
/// ```
pub fn lex_tags(prompt: &str) -> (String, Vec<String>) {
```

---

### 2. Missing Tests for `--strict` Flag Behavior

**What**: The spec states (mcp-mode.md:79):
- "if the CLI command contains the `--strict` flag then instead of asking it will simply exit in an error"
- "if the `--strict` flag is used, however, we will immediately stop with an error"

**Gap**: No tests verify `--strict` behavior for:
- Ambiguous tag resolution (multiple matches)
- Missing tag resolution (no matches)

**How to fix**: Add integration tests in `cli/tests/mcp_cli.rs`:

```rust
#[test]
fn mcp_strict_fails_on_ambiguous_tag() {
    // Setup catalog with servers that would match "google" substring
    // Run with --strict flag
    // Assert failure with error message containing ambiguous candidates
}

#[test]
fn mcp_strict_fails_on_missing_tag() {
    // Setup catalog without "nonexistent" server
    // Run with --strict flag
    // Assert failure with error message about missing tag
}
```

---

### 3. No Tests for Reactive Initialization from `--mcp` Flag

**What**: Spec (mcp-catalog.md:28): "user starts a wrapped Agent with the `--mcp` flag (aka, reactively requesting use of MCP mode prior to initialization)"

**Gap**: The `bootstrap_mcp_state` function in `wrap/mod.rs` is not directly tested.

**How to fix**: Add integration test that:
1. Does NOT pre-initialize the catalog
2. Runs `claudine codex --mcp "#tag" --dry-run`
3. Verifies the catalog was created lazily with the default structure
4. Verifies the tag was resolved against defaults

---

### 4. Incomplete Tag Lexing Edge Cases

**What**: Current tests cover basic cases but miss edge cases from spec.

**Gap**: Missing tests for:
- `#tag` at end of string (no trailing whitespace)
- `#tag-with-dash` and `#tag_with_underscore` (valid characters)
- Case insensitivity of alphabetic first character (`#Calendar` should work)
- Multi-byte character handling (Unicode)

**How to fix**: Add to `session.rs` tests:

```rust
#[test]
fn lex_tags_handles_end_of_string_termination() {
    let (cleaned, tags) = lex_tags("check #calendar");
    assert_eq!(tags, vec!["calendar"]);
    assert_eq!(cleaned, "check");
}

#[test]
fn lex_tags_accepts_dashes_and_underscores() {
    let (cleaned, tags) = lex_tags("use #my_server and #tag-with-dash");
    assert!(tags.contains(&"my_server".to_string()));
    assert!(tags.contains(&"tag-with-dash".to_string()));
}

#[test]
fn lex_tags_case_insensitive_first_char() {
    let (cleaned, tags) = lex_tags("fix #Calendar and #SLACK");
    assert_eq!(tags, vec!["Calendar", "SLACK"]);
}
```

---

### 5. Missing CLI Command Tests

**What**: Several CLI commands lack integration tests.

**Gap**: The following commands have no test coverage:

| Command | Status |
|---------|--------|
| `claudine mcp list --alias <filter>` | Not tested |
| `claudine mcp config <name>` | Only tests `--json` flag |
| `claudine mcp remove` (alias removal path) | Not fully tested |
| `claudine mcp alias` (interactive prompts) | Not tested |

**How to fix**: Add tests in `cli/tests/mcp_cli.rs`:

```rust
#[test]
fn mcp_list_filters_by_alias() {
    // Seed catalog with server "calendar" having alias "gcal"
    // Run `claudine mcp list --alias cal`
    // Assert only calendar appears in output
}

#[test]
fn mcp_remove_alias_reports_remaining_aliases() {
    // Seed server with aliases ["gcal", "cal"]
    // Run `claudine mcp remove gcal`
    // Assert success and check output mentions remaining "cal" alias
}
```

---

### 6. No Tests for Algorithmic Naming (xxHash Fallback)

**What**: Spec (mcp-mode.md:61): "if no known means is available to provide a 'name' meaningfully... we will instead opt to hash the entire configuration JSON with xxHash"

**Gap**: Tests exist for `derive_server_name` but:
- Do not verify xxHash is actually used
- Do not test that the hash produces consistent output
- Do not test the `mcp-` prefix behavior

**How to fix**: Add test in `types.rs`:

```rust
#[test]
fn derive_server_name_xxhash_fallback_is_deterministic() {
    // Create server with no command, no args, only URL
    let mut server = test_server("ignored");
    server.command = None;
    server.args.clear();
    server.url = Some("https://example.com/mcp".into());
    
    let name1 = derive_server_name(&server, None);
    let name2 = derive_server_name(&server, None);
    
    assert_eq!(name1, name2); // Deterministic
    assert!(name1.starts_with("mcp-")); // Has prefix
}
```

---

### 7. Missing Validation Tests for Defaults References

**What**: Spec notes that `defaults.json` and repo `mcp.json` can reference missing catalog IDs.

**Gap**: Validation tests don't cover:
- User defaults referencing missing catalog IDs
- Repo defaults referencing missing catalog IDs
- Warning vs error severity distinction

**How to fix**: Add test in `validation.rs`:

```rust
#[test]
fn validation_warns_for_missing_default_references() {
    let catalog = McpCatalogStore::load_from(Path::new("/nonexistent")).unwrap();
    catalog.add_server(make_server("calendar"));
    
    let defaults = McpDefaults {
        version: 1,
        defaults: vec!["calendar".into(), "missing-server".into()],
    };
    
    let report = validate_defaults(&catalog, &defaults, &Scope::User).unwrap();
    assert!(report.issues.iter().any(|i| i.code == "missing-default"));
}
```

---

## Lower Priority Issues

### 8. Tests for Unimplemented Features

Some features in the spec are not yet implemented. These don't need tests until implemented:

- `claudine mcp add local` - Interactive interview (NOT IMPLEMENTED)
- `claudine mcp add remote` - Interactive interview (NOT IMPLEMENTED)
- `claudine mcp init` interactive flow - Multi-select widgets (NOT IMPLEMENTED)
- Post-init help display (NOT IMPLEMENTED)

---

### 9. Suspect Tests

**Test**: `resolve_prefix` in `catalog.rs:442-447`

**Concern**: This test uses `resolve("seq")` expecting it to match `sequential-thinking` via prefix. However, the spec describes substring matching, not prefix-only. The test passes but may not reflect the intended behavior.

**Recommendation**: Verify that `resolve` actually implements substring matching (starts with, ends with, contains) as specified in mcp-mode.md:74. If prefix-only is the current implementation, clarify in test comments that this tests prefix behavior specifically.

---

### 10. Timeout Observations

**Current state**: No tests have overly long timeouts. The integration tests use tempfile which is fast.

**Note**: If network-based tests are added for remote MCP servers (future `mcp check --live` feature), ensure reasonable timeouts (5-10s max) with clear skip conditions.

---

## Summary of Recommendations

| Priority | Gap | Location to Add Tests |
|----------|-----|---------------------|
| Critical | Doctests | `lib/src/mcp/*.rs` - Add `/// # Examples` to public functions |
| Critical | `--strict` flag | `cli/tests/mcp_cli.rs` |
| Critical | Reactive init | `cli/tests/mcp_cli.rs` |
| High | Tag lexing edge cases | `lib/src/mcp/session.rs` |
| High | CLI command coverage | `cli/tests/mcp_cli.rs` |
| Medium | xxHash fallback naming | `lib/src/mcp/types.rs` |
| Medium | Defaults validation | `lib/src/mcp/validation.rs` |

---

## Additional Recommendations from mcp-features.md Review

After reviewing `@claudine/docs/mcp-features.md`, several additional observations and recommendations apply:

### Feature Status Discrepancies

The `mcp-features.md` document states certain features are "Not Implemented" but code/tests exist:

1. **Inline Prompt Tag Syntax** - States "Not Implemented" but `session.rs` contains `lex_tags()` function with tests (lines 264-290). The implementation EXISTS but may not be wired into the CLI.

2. **Substring Match Resolution** - States "Not Implemented" but `catalog.rs:441-455` has tests for prefix and substring resolution.

3. **`claudine mcp check`** - States "Not Implemented" but integration test exists in `mcp_cli.rs:225-245`.

**Recommendation**: Update the feature review document to reflect actual implementation status, OR verify these features are correctly wired into the CLI entry points.

---

### Features Needing Tests When Implemented

The following features are correctly marked as "Not Implemented" - tests should be added when implementing:

1. **`claudine mcp add local`** - Interactive interview for adding local stdio servers
2. **`claudine mcp add remote`** - Interactive interview for adding remote HTTP servers  
3. **Interactive initialization** - Multi-select widgets for default selection
4. **Post-init help display** - Tutorial bullet points after init

For each of these, plan to add:
- Integration tests in `cli/tests/mcp_cli.rs`
- Edge case tests for the underlying library functions

---

### Additional Test Coverage for Implemented Features

Based on the features document, these implemented features need better test coverage:

1. **`claudine mcp list --alias`** - Not tested despite being in spec
2. **User/Repo default layering** - Not tested that repo defaults override user defaults correctly
3. **Provider-specific overrides** - Not tested that provider overrides are correctly applied

Add tests:

```rust
#[test]
fn defaults_repo_overrides_user() {
    // Set user defaults to ["calendar", "slack"]
    // Set repo defaults to ["github"]
    // Compute effective session set
    // Assert effective defaults = ["github"] (repo overrides user)
}
```

---

### Missing Tests for Provider Injection Behavior

The features document confirms provider-specific injection is implemented for Codex, Gemini, and OpenCode. Tests exist for dry-run behavior but missing:

1. **Actual execution** (non-dry-run) - Verify shadow files are cleaned up after execution
2. **Multiple server injection** - Verify all servers from defaults + tags are injected
3. **Provider fallback** - Test behavior when provider doesn't support runtime injection (Claude)

The Claude test exists (`mcp_cli.rs:430-456`) but could be expanded to verify:
- Error message accuracy
- Export command suggestion correctness

---

## Test Execution

Run tests with:

```bash
# Unit tests
cargo test -p claudine-lib --lib -- mcp

# Integration tests  
cargo test -p claudine-cli --test mcp_cli

# All MCP-related tests
cargo test -p claudine-lib -p claudine-cli -- mcp
```
