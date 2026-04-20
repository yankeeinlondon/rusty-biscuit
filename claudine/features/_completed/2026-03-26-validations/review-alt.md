# Review of Validations, Timeouts, and Handlers Implementation

The implementation of "Validations, Timeouts, and Handlers" in `claudine` is remarkably complete and aligns closely with the technical design. However, there are a few areas for improvement, functionality gaps, and potential bugs.

## Gaps in Functionality

### 1. Missing Provider-Aware Permission Probe
The technical design (Section: Write-Permission Validation) suggested a `HarnessPermissionProbe` trait to allow provider-specific checks on whether an agent is allowed to write to a path.

- **Current implementation:** `has_write_permission` only performs a filesystem-level write check using a probe file (`check_write_permission` in `claudine/lib/src/harness/validate.rs`).
- **Suggestion:** Implement the `HarnessPermissionProbe` trait and integrate it into the `has_write_permission` validation to honor provider-specific runtime policies.

### 2. Incomplete Programmatic Handler Payload
The JSON payload sent to programmatic handlers (`handle` frontmatter property) is missing several recommended fields.

- **Current implementation (`claudine/lib/src/harness/handlers.rs`):**

    ```json
    {
      "source_file": null,
      "attempt": 1,
      "session_id": "sess_123",
      "failure_event": "timeout",
      "message": "Timed out...",
      "subject_key": "some-key"
    }
    ```

- **Technical design recommendation:**

    ```json
    {
      "provider": "codex",
      "source_file": "/abs/path/file.md",
      "attempt": 1,
      "session_id": "sess_123",
      "termination": "timeout",
      "failure_event": "timeout",
      "failure_phase": "agent",
      "message": "Timed out after 5m",
      "check": {
        "name": "response_missing",
        "subject_key": "failed"
      },
      "response": {
        "text": "partial response here"
      }
    }
    ```

- **Suggestion:** Enrich the `FailureContext` to include `source_file`, `provider`, `termination`, `failure_phase`, and the `check` details, then update `execute_programmatic_handler` to emit the complete JSON payload.

## Potential Issues and Incomplete Implementation

### 3. Programmatic Handler Response Validation
The technical design states: "Programmatic handlers are not allowed to return `deviate`. That is enforced after JSON parsing."

- **Current implementation:** `execute_programmatic_handler` parses the response but does not explicitly reject `HandlerAction::Deviate`.
- **Suggestion:** Add a check after parsing the JSON response from a programmatic handler to ensure it doesn't contain a `deviate` action.

### 4. `frontmatter_prop_equals` Consistency
The `frontmatter_prop_equals` check re-reads the file from disk after the agent completes.

- **Observation:** If the agent fails or crashes halfway, the file on disk might be in an inconsistent state or stale.
- **Suggestion:** While re-reading from disk is correct for detecting the *current* state, we should ensure the error message correctly indicates when the file couldn't be read or the property was missing in the post-run state.

## Ergonomics and Performance

### 5. Resolution Context Caching
In `claudine/cli/src/commands/wrap/mod.rs`, the `HarnessResolutionContext` and `ShellApprovalOptions` are reconstructed in every iteration of the `run_harness_loop`.

- **Suggestion:** Initialize these once outside the loop and pass them in to avoid redundant allocations and setup.

### 6. Shell Approval Caching
Currently, every `shell_command`, `deviate`, or `handle` call triggers a fresh policy resolution and potentially an interactive prompt if the command isn't whitelisted.

- **Suggestion:** Implement a session-level cache for shell approvals within the `claudine` process so the user doesn't have to approve the same command multiple times during a multi-attempt retry/resume loop.

## Test Coverage

### 7. Integration Testing for Redirect and Resume
The current integration tests in `claudine/cli/tests/wrap_commands.rs` are extensive but could be strengthened for complex harness flows.

- **Suggestion:** Add explicit integration tests that exercise:
    - **Redirect:** A handler that redirects to a second Markdown document.
    - **Resume:** A handler that successfully resumes a timed-out session (mocking a provider that supports resume).
    - **Programmatic Handler:** A handler script that returns a `redirect` or `retry` JSON response.
