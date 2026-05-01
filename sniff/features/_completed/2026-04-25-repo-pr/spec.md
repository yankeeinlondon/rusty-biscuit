# Functional Specification: `sniff repo pr`

## Overview
The `sniff repo pr` subcommand provides a way to list and inspect Pull Requests (or Merge Requests) for the current repository across supported Git hosting platforms (e.g., GitHub, GitLab).

## CLI Interface

### Command
`sniff repo pr [flags]`

### Flags
- `--status <status>`: Filter PRs by status.
    - **Default**: `open`
    - **Normalized Options**: `open`, `closed`, `merged`, `draft`, `all`.
- `-v`, `--verbose`: Enable detailed block view for each PR.
- `--json`: Output results in JSON format.
- `-h`, `--help`: Display help information.

## Requirements & Behavior

### 1. Filtering & Scoping
- By default, the command lists only **OPEN** Pull Requests.
- The `--status` flag allows users to query PRs with different states.
- **Library Integration**: The library trait `RemoteRepoProvider::list_pull_requests` must be updated to accept the `PullRequestState` filter.

### 2. Status Normalization
- The CLI and library use a normalized `PullRequestState` enum: `Open`, `Closed`, `Merged`, `Draft`, `All`.
- The library is responsible for mapping these normalized values to platform-specific values (e.g., `opened` for GitLab, `open` for GitHub).
- `--status draft` is a valid and supported filter.

### 3. Metadata Expansion
- `PullRequestInfo` will be expanded to include:
    - `labels: Vec<String>`: A list of labels assigned to the PR.
    - `body: Option<String>`: The full description/body of the PR (used for the verbose view).

### 4. Display Formatting

#### Default View (Tabular)
A concise table containing:
- **ID**: The PR number/identifier.
- **Title**: Summary of the PR.
- **Author**: The user who created the PR.
- **State**: The current state (e.g., OPEN, CLOSED).

#### Verbose View (`-v`, `--verbose`)
A detailed block view for each PR.

**Mockup**:
```
#123: Feature: Add repo pr command
---------------------------------
Author:  @username
Status:  OPEN [draft]
Branch:  feature/repo-pr -> main
Labels:  enhancement, cli
Created: 2024-03-20

Description:
This PR adds the `sniff repo pr` subcommand to list...
```

#### JSON Output (`--json`)
Machine-readable format containing all available metadata for the PRs (including `labels` and `body`). This is a standard pattern for Sniff CLI commands.

### 5. Authentication & Rate Limiting
- **Strategy**: **Attempt Unauthenticated**.
- The CLI must first attempt to fetch data anonymously.
- **Graceful Failure**: If a 401 (Unauthorized), 403 (Forbidden), or rate limit error is encountered:
    - Fail gracefully.
    - Provide a clear error message.
    - Instruct the user on how to provide credentials (e.g., "Set the `GITHUB_TOKEN` environment variable").

### 6. Platform Detection
- The subcommand must automatically detect the upstream Git hosting provider (GitHub, GitLab, etc.) based on the repository's remotes.
- It should delegate the actual API interaction to the `sniff` library.

## Error Handling
- **No Remote Found**: If the current directory is not a Git repository or has no remotes, display a clear error.
- **Unsupported Platform**: If the detected platform is not yet supported by Sniff's PR feature, inform the user.
- **Invalid Status**: If a user provides a status not in the normalized list, show a helpful error listing the valid options (`open`, `closed`, `merged`, `draft`, `all`).
- **Network Errors**: Handle timeouts and connectivity issues with descriptive messages.

## Validation
- Ensure `sniff repo pr` works across different platforms.
- Verify flag combinations (e.g., `--status merged --json`).
- Test unauthenticated vs. authenticated behavior.
