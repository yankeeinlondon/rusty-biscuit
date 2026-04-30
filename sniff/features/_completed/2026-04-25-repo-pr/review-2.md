---
ready: true
agent: ""
model: ""
---

# Feature Review: `sniff repo pr` (Review 2)

I have performed a comprehensive review of the implementation of the `sniff repo pr` subcommand. Following the previous review cycle, all planned functionality has been implemented with strong alignment to the functional specification.

## Functionality Analysis

The implementation covers all core requirements:
- **Status Filtering**: `PullRequestState` correctly supports `Open`, `Closed`, `Merged`, `Draft`, and `All`.
- **Platform Support**: Verified for GitHub, GitLab, Gitea, and Bitbucket Cloud.
- **Normalized Data**: `PullRequestInfo` now includes `labels` and `body` (description) where supported by the provider.
- **Display Modes**: 
    - **Default**: Clean tabular view with ID, Title, State, and Author.
    - **Verbose (`-v`)**: Detailed block view including branch info, labels, and a markdown-rendered body preview.
    - **JSON**: Machine-readable output containing the full metadata.

## Code Quality & Architecture

- **Trait Integration**: `RemoteRepoProvider::list_pull_requests` was successfully updated to accept the state filter.
- **Graceful Fallback**: The "attempt unauthenticated first" strategy is correctly implemented across all providers, allowing public repository queries without tokens while providing clear instructions when credentials are required.
- **Consistency**: The implementation follows existing `sniff` patterns for error handling and output rendering.
- **Minor Observation**: In `sniff/cli/src/args.rs`, the `RepoAction::Pr` variant includes a `verbose: bool` field that is hardcoded to `false` in `to_repo_action`. While `commands.rs` correctly uses the global `cli.verbose` flag for rendering, this field in `RepoAction` is redundant and slightly inconsistent with other actions that rely on the global flag. This does not affect functionality but could be cleaned up for better alignment with the rest of the codebase.

## Test Coverage

The test coverage is excellent:
- **Unit Tests**: Comprehensive tests in `sniff/cli/src/output/remote.rs` for all rendering paths (table, verbose, empty states, draft markers).
- **Integration Tests**: Extensive test suite in `sniff/lib/tests/remote_providers.rs` covering success scenarios, label/body handling, and authentication fallback logic for all four providers.
- **Platform Edge Cases**: Specific tests for Bitbucket's lack of draft support and GitLab's state mapping are present.

## Performance & Ergonomics

- **Async Efficiency**: API calls are performed asynchronously using the established `schematic` client pattern.
- **Ergonomics**: The subcommand automatically detects the provider from the `origin` remote, providing a zero-config experience for common workflows.
- **Error Messages**: Failures (rate limits, missing credentials) return actionable, user-friendly messages.

## Conclusion

The feature is robust, well-tested, and ready for production.

**Verdict: Ready for Production**
