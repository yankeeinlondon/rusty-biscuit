# Policy Engine Follow-up Review

This follow-up review focuses on what remains after the first round of fixes.

## Findings

### P1: `can_use_mcp_tool()` still ignores server-level MCP policy

Files:

- `claudine/lib/src/permissions/query.rs`
- `claudine/lib/src/permissions/providers/qwen.rs`

The MCP improvements fixed the missing Codex axis and the trust/path issues, but there is still one correctness hole in the generic query layer: `resolve_mcp_tool_query()` only evaluates `McpToolRule` values. It does not compose server-level MCP decisions into the tool-level answer.

That means `can_use_mcp_tool(server, tool)` can disagree with `can_use_mcp_server(server)`.

The clearest example is Qwen CLI allowlisting. The backend adds:

- explicit `McpServerRule` allows for listed servers
- a `McpServerRule { server_id: "*", effect: Deny }` for unlisted servers
- a fallback `McpToolRule { server_id: "*", tool_name: "*", effect: mcp_default }`

Because the tool resolver ignores server rules, an unlisted server can still produce an allow/ask result for `can_use_mcp_tool()` if the fallback tool rule is permissive. The current test only checks `can_use_mcp_server("github")`, not `can_use_mcp_tool("github", "...")`, so this gap is still untested.

Recommended fix:

1. Make tool queries consult server rules before or alongside tool rules.
2. Add a regression test for `qwen_cli_allowed_mcp_servers_deny_unlisted_servers` that asserts an MCP tool on an unlisted server is denied as well.
3. Audit other providers with server-level MCP policy to ensure tool queries inherit that policy correctly.

## Additional Suggestions

### Explicit `LocalOverride` targeting should not silently downgrade to repo config on providers that do not support it

Files:

- `claudine/lib/src/permissions/providers/codex.rs`
- `claudine/lib/src/permissions/providers/gemini.rs`
- `claudine/lib/src/permissions/providers/opencode.rs`
- `claudine/lib/src/permissions/providers/qwen.rs`

Claude now treats `LocalOverride` correctly, but several other providers still alias `PolicyChangeTarget::LocalOverride` to repo config. That is workable as an implementation shortcut, but it is surprising for callers because an explicit target request is being silently rewritten.

Recommended improvement:

- Return an unsupported-target warning or error when a provider has no local override concept, rather than remapping it to repo config invisibly.

### `MayChangeWithCli` is still broader than it needs to be

File:

- `claudine/lib/src/permissions/query.rs`

Configured snapshots now correctly distinguish configured vs effective policy, but `base_stability()` marks every stable configured result as `MayChangeWithCli`. That is conservative, but it overstates uncertainty for providers or axes where CLI args do not actually affect the answer.

Recommended improvement:

- Make CLI sensitivity backend- or axis-aware instead of applying it uniformly to all configured results.

### Round-trip coverage is improving, but still concentrated in Claude

Files:

- `claudine/lib/src/permissions/providers/claude.rs`
- `claudine/lib/src/permissions/providers/codex.rs`
- `claudine/lib/src/permissions/providers/gemini.rs`
- `claudine/lib/src/permissions/providers/qwen.rs`

The new Claude round-trip test is a real improvement. I still did not find equivalent round-trip reload coverage for Codex, Gemini, or Qwen mutation paths, and the MCP server/tool interaction bug above would likely have been caught by one.

Recommended improvement:

1. Add round-trip mutation tests for Codex MCP edits.
2. Add round-trip mutation tests for Gemini settings plus policy-file edits.
3. Add Qwen coverage for server-level MCP denies propagating to tool queries.

## Verification

I verified the current implementation with:

```bash
cargo +nightly test -p claudine
```

Result:

- passed: 869 unit tests
- passed: 2 doctests

Notes:

- The unqualified `cargo` entrypoint in this environment resolves to an older toolchain and is not reliable for this workspace.
- Pinning the nightly toolchain was necessary to run the test suite successfully.
