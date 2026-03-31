# Policy Engine Follow-up Review 3

This pass is much tighter. The previous second-review items were addressed:

- MCP tool queries now inherit server-level denies
- providers without a native local override concept now reject `PolicyChangeTarget::LocalOverride`
- round-trip mutation coverage was added for Codex, Gemini, and Qwen
- CLI sensitivity is now query-type aware instead of applied to every configured answer

## Remaining Finding

### P2: `can_use_mcp_tool()` still does not inherit non-deny server-level MCP decisions

Files:

- `claudine/lib/src/permissions/query.rs`
- `claudine/lib/src/permissions/providers/gemini.rs`

`resolve_mcp_tool_query()` now checks server-level MCP rules first, but only short-circuits when the server result is `Deny`.

That still leaves a gap for providers that model MCP at the server level without emitting tool-level wildcard rules. Gemini is the clearest example:

- `settings.mcp.allowed` becomes `McpServerRule { effect: Allow }`
- there is no corresponding `McpToolRule { server_id: ..., tool_name: "*", effect: Allow }`

So `snapshot.can_use_mcp_server("filesystem")` can return `Allow`, while `snapshot.can_use_mcp_tool("filesystem", "read_file")` still falls through to `No matching rules found` unless a separate tool rule exists.

That is still below the design target for `can_use_mcp_tool(server, tool)` as a canonical query surface.

Recommended fix:

1. Treat a matching server-level `Allow` or `Ask` as the fallback result for tool queries when no more specific tool rule matches.
2. Add a Gemini regression test that asserts an MCP tool on an allowed server is also allowed.
3. More generally, define the precedence explicitly:
   - tool rule beats server rule
   - otherwise server rule becomes the fallback tool answer

## Additional Suggestion

### Add one generic query-layer test for server-allow fallback

The new provider round-trip tests are good, but the MCP inheritance rule now lives in the shared query layer. A small query-level unit test would make that behavior harder to regress across providers.

Suggested cases:

- server deny + tool allow => deny
- server allow + no tool rule => allow
- server ask + no tool rule => ask
- server allow + tool deny => deny

## Verification

I verified the current implementation with:

```bash
cargo +nightly test -p claudine
```

Result:

- passed: 869 unit tests
- passed: 2 doctests
