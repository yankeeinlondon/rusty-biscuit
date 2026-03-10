We have performed three reviews on a recent refactor of the "MCP mode" functionality in the `claudine` and `claudine-cli` packages. Each review had a slightly different focus but you find that there is some overlap too.

Your task is to review all three reviews and detail out a final set of recommendations which will make the MCP mode functionality complete.

The three reviews were:

1. Feature Review (@claudine/reviews/mcp-feature-review.md)
2. Spec Review (@claudine/reviews/mcp-spec-review.md)
3. Test Review (@claudine/reviews/mcp-test-review.md)

## Feature Review

Summary of Recommended Changes:

   1. Unused Cleanup Logic (Priority: Low)
      - Issue: The cleanup method for removing injected shadow-home configuration files is fully implemented for both Codex
        and Gemini providers within the McpInjector trait but is never invoked by the wrapper lifecycle.
      - Recommendation: Hook the cleanup routine into the wrapper process teardown so it does not leak persistent
        configuration state between sessions.
   2. Orphaned References on Server Removal (Priority: Medium)
      - Issue: Running claudine mcp remove successfully deletes an entry from the central catalog but fails to scrub the
        server's ID from user or repo defaults (defaults.json and .claudine/mcp.json). This eventually results in noisy
        warnings being surfaced to the user during validation and wrapper startup.
      - Recommendation: Enhance the removal command to automatically cascade the deletion to active default sets so the
        system's state remains consistent and noise-free.
   3. Missing sync CLI Argument Clarity (Priority: Low)
      - Issue: The CLI currently accepts claudine mcp sync <provider> and implicitly redirects it to run_export with a
        deprecation warning. This causes friction as sync logically implies an "import/pull" operation while export implies a
        "push".
      - Recommendation: Strictly enforce the separation of concerns by removing the <provider> positional argument from sync.

DETAILS: @claudine/reviews/mcp-feature-review.md

## Spec Review

Overall: The refactor plan has been substantially executed. The implementation covers the core spec surface well across 10 library modules and a comprehensive CLI layer.

High priority:

- current_repo_root() always returns Some — likely a bug that could create repo-scope defaults in non-repo directories

Medium priority:

- effective_defaults replaces rather than merges user+repo defaults — spec is ambiguous, needs a design decision
- Init re-entry ignores user defaults when prompting for repo defaults (let _ = current; dead code)
- Ambiguous tag cancellation (user presses Esc on disambiguation prompt) hard-errors instead of dropping the tag

Research May be needed:

- Claude runtime MCP injection (medium urgency — most popular provider)
- Goose/Kimi/Qwen support (low urgency)

## Test Review

I've completed the MCP Mode test coverage review and written it to @claudine/reviews/mcp-test-review.md. The review covers:
Critical Gaps Identified:

1. No doctests - All MCP library functions lack doctests
2. Missing --strict flag tests - No tests for strict mode behavior
3. No reactive init tests - Tests for --mcp flag bootstrap are missing
4. Incomplete tag lexing - Missing edge cases (end-of-string, dashes/underscores, case insensitivity)
5. Missing CLI tests - --alias filtering, alias removal, config command not fully tested
6. xxHash fallback tests - Missing verification of deterministic hashing
7. Validation gaps - No tests for defaults referencing missing catalog IDs

Additional findings from mcp-features.md:

- Found discrepancies where features are marked "Not Implemented" but code/tests exist (e.g., lex_tags, substring matching, mcp check)
- Added recommendations for tests once unimplemented features are built (add local/remote, interactive init)
- Noted missing tests for user/repo default layering and provider injection behavior
