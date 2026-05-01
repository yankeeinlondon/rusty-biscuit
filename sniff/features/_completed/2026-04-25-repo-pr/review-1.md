---
ready: false
agent: ${env.AGENT}
---

# Feature Review: `sniff repo pr`

## Overview

The `sniff repo pr` feature provides a unified interface for listing and inspecting pull requests across multiple Git hosting providers. The implementation covers the core requirements including state filtering, remote resolution, and rich terminal output (table and verbose views).

## Gaps in Functionality

### 1. Missing Labels in Metadata Expansion
The specification explicitly requested that `PullRequestInfo` include `labels: Vec<String>`. However, the implementation only populates this field for GitLab.
- **GitHub**: Hardcoded to `Vec::new()`. A comment in `sniff/lib/tests/remote_providers.rs` suggests `PullRequestSummary` in `schematic-schema` does not expose labels.
- **Gitea**: Hardcoded to `Vec::new()`.
- **Bitbucket**: Hardcoded to `Vec::new()`.

Labels are a critical piece of PR metadata for triage and overview. If the current schema is limited, it should be expanded to fulfill the specification.

### 2. Missing Body in Bitbucket
The specification requested `body: Option<String>` for use in the verbose view.
- **Bitbucket**: Currently returns `None` as it's not available in the list API. To fulfill the spec for the verbose view, a follow-up request to the single PR endpoint might be necessary when `-v` is used, or the limitation should be documented.

### 3. "Attempt Unauthenticated" Strategy
The specification and the `sniff` skill guidelines both mandate an "Attempt Unauthenticated" strategy where the CLI first tries to fetch data anonymously.
- **Implementation**: The `map_schematic_error` function in all providers catches `SchematicError::MissingCredential` and immediately returns `SniffError::MissingCredentials`. This causes the CLI to exit with an authentication error message before attempting a public request.
- **Requirement**: The provider should attempt the request without credentials if they are missing, only failing if the API actually rejects the unauthenticated request (e.g., for private repositories).

## Broken or Incomplete Features

- **Draft Filtering on Bitbucket**: The implementation correctly notes that Bitbucket doesn't support draft PRs and returns an empty list. This is acceptable as a platform limitation, but the help text could mention this.

## Test Coverage

- **Strengths**: Integration tests in `sniff/lib/tests/remote_providers.rs` are comprehensive, using `wiremock` to simulate various API responses and error states for all four providers. CLI argument parsing and output rendering also have good test coverage.
- **Weaknesses**: Assertions for `labels` and `body` are missing or verify that they are empty (even when the test fixture contains data). Tests should verify that labels are correctly mapped when supported (e.g., for GitLab).

## Ergonomics and Performance

- **Error Messages**: Failures are well-handled with descriptive messages and actionable instructions for setting environment variables.
- **Remote Resolution**: The logic for finding the upstream remote (origin or first remote) is robust.
- **Performance**: The use of `async_trait` and parallel execution for `fetch_report` is good. `sniff repo pr` as a standalone command is appropriately performant.

## Suggestions for Improvement

1. **Expand Schematic Schema**: Update `schematic/schema` to include `labels` in GitHub and Gitea pull request summaries.
2. **Bitbucket Mapping**: Map Bitbucket's `priority` or `kind` to the `labels` field to provide some level of categorization where labels aren't natively supported.
3. **Fallback to Unauthenticated**: Refactor providers to only return `MissingCredentials` if an unauthenticated request has already been attempted and failed with a 401/403, or if the repository is known to be private.
4. **Enhanced Tests**: Update integration tests to assert that `labels` and `body` are correctly populated for GitLab, and eventually for other providers once implemented.

## Conclusion

The feature is functionally solid for its primary purpose but falls short of the specification's requirements for metadata completeness (labels/body) and the mandatory unauthenticated access strategy.

**Ready for Production: No** (Pending metadata expansion and unauthenticated fallback)
