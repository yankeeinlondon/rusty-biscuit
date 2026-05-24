---
ready: true
agent: gemini
---

# Review 2: Agent Selection Feature

The implementation of the agent-selection feature has been reviewed against the specification and technical design. The prior review's suggestions have been addressed, and the feature is now in a highly complete and robust state.

## Functionality Analysis

- **Layered Resolver:** The resolution logic correctly implements the split between TTY (always picker unless explicit flag) and non-TTY (strict chain: explicit > frontmatter > favorite > error) modes.
- **Frontmatter Hints:** Both `agent` and `model` hints support singular and list values, with author-order preservation and fuzzy matching for providers.
- **Model Catalog:** The catalog service successfully combines static sources, dynamic fetching (`opencode models`), and user overrides (additive/replace). Stale-cache fallback is implemented correctly.
- **TTY Components:** The transition to `tui-chrome::ChooseOne` and `tui-chrome::InputTable` for interactive selection and sequence review provides a consistent and ergonomic UX.
- **Sequence UX:** The front-loading of all agent/model decisions before execution starts is correctly implemented in `sequence.rs`, honoring the requirement for non-interactivity once the session begins.
- **OpenCode Special Case:** The hard error for missing model in non-interactive OpenCode sessions is correctly implemented and provides actionable guidance.

## Test Coverage

The feature has strong unit and integration test coverage:
- `prepare.rs` includes tests for frontmatter parsing of single/list agents and models.
- `select.rs` exhaustively tests the resolution priority chains for both modes.
- `service.rs` covers catalog validation, overrides, and case-insensitive matching.
- `selection_ui.rs` validates the decoding of table rows back into execution targets.
- `composition.rs` and `sequence.rs` tests cover the orchestration and propagation of resolved targets.

## Suggestions for Improvement

- **Performance:** `ModelCatalogService::refresh_blocking` currently shells out to `opencode` on every command invocation. While it runs in a background thread and has cache fallback, introducing a TTL (e.g., 24 hours) for the cache would further reduce latency for repeated commands.
- **Ergonomics:** The `SelectionUnavailable` error message in `error.rs` is informative but could be further refined in the CLI output layer to suggest specific commands (e.g., `claudine config set favorite-agent <provider>`) based on the detected environment.

## Conclusion

The implementation is complete, well-tested, and adheres strictly to the architectural direction. The code is idiomatic and follows the project's conventions.

**Status:** Ready for production.
