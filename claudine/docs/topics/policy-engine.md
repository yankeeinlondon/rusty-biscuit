`PolicyEngine` is Claudine's provider-agnostic permissions layer.

It loads provider-native policy files, composes configured and effective policy snapshots, answers structured permission queries, and plans persistent or one-shot permission changes.

Current query behavior worth knowing:

- Relative path queries are normalized against `PolicyContext.cwd` before matching.
- Path explanations classify the resolved path as `workspace`, `home`, `system`, `temp`, `provider-config`, or `external`.
- Snapshot warnings are propagated into `QueryResult`, including trust-state ambiguity.
- Trust-gated repo policy for Codex and Gemini is skipped when trust is unknown, and affected queries return ambiguous results instead of false confidence.
- Codex MCP policy is modeled from `mcp_servers` config, including server enablement and `enabled_tools` / `disabled_tools`.

