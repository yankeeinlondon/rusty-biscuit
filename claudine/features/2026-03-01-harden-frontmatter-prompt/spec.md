the `claudine {agent} --frontmatter-prompt <file-ref>` is not working well.

## Problem (original)

- Agents are writing their research and the writing their summary on top of the summary
- The execution line header was displaying the CLI switch text (`'--frontmatter-p..."`) instead of the actual prompt content from the frontmatter file

## Root Cause (identified 2026-03-17)

The original implementation captured the agent's `assistant_text` from the structured stream and wrote it as the file body. This was fundamentally wrong:

1. **The agent is responsible for writing to the file** (via tool calls like Write/Edit). Claudine should NOT capture stdout and write it to the file.
2. The agent's stdout text is a **summary for the user** — it should stream to the terminal normally.
3. Because ALL assistant text (thinking, progress, summary) was written as the file body, the actual research content the agent wrote via tools was overwritten by the summary.
4. The "no summary" warning could never fire because `assistant_text` always contained content (the agent's progress/summary text that was incorrectly being treated as file body).

## Correct Architecture

The `--frontmatter-prompt` pipeline should work like a normal non-interactive agent call:

1. Extract the `prompt` property from frontmatter, append guardrails
2. Send the prompt to the agent
3. **Stream agent output to the terminal normally** (agent writes to the file via its own tool calls)
4. After the agent completes, **validate the file from disk**:
    - Was the body updated? If not, report error
    - Was the frontmatter tampered with? If so, warn and restore original frontmatter (keeping the agent's body)
    - Update `last_updated` to today's date
5. If the agent provided no summary text to stdout, warn on stderr

## Implementation

### Guardrails file (`.claudine/frontmatter-prompt.md`)

- Created at `{repo-root}/.claudine/frontmatter-prompt.md` on first use if absent
- Appended to every inline prompt to prevent the agent from mangling frontmatter
- User-customizable; Claudine reads from disk if the file exists

### Execution line header

- Display the actual `prompt` property text (truncated to fit), not the CLI switch name

### Agent execution

- Use `run_child_stream` (structured) or `run_child` (legacy) — the same functions used for normal non-interactive sessions
- Agent stdout/stderr flows through to the terminal
- The agent is responsible for using its tools to write content to the referenced file

### Post-execution validation

- **Always** validate the file — even when the agent exits with an error (the agent may have successfully updated the file before an API error occurred)
- Read the file from disk after agent completion
- Hash comparison (darkmatter) to detect body and frontmatter changes
- If body was updated AND agent exited with error: treat as success (warn the user), override exit code to 0
- If frontmatter was tampered: restore original frontmatter, keep agent's body content, update `last_updated`
- If frontmatter was clean: update `last_updated` in the on-disk file directly
- If body was not updated despite successful exit: report error, set exit code to 1

### Validation output (stderr)

All validation checks are rendered as styled check lines:

- `✓ resolved the file reference to <link>` — on file resolution
- Prompt displayed as a truncated blockquote (max 10 lines) before execution
- `✓ {Agent} agent completed successfully` — or `⤫ {Agent} agent exited with error (code N)`
- `✓ Agent updated the target document's body` — body hash changed
- `✓ Agent left frontmatter untouched (*as instructed*)` — or `⤫ Agent ignored instruction...(*we have reverted their changes*)`
- `✓ Updated **last_updated** property to today's date`

### Formatting improvements (general, not frontmatter-specific)

- Execution header line (`Claudine ▸ Provider ...`) has a blank line before AND after
- Blank line after env/info messages to separate from execution output
- Session start line: `- *Claude* session ID {id}` (styled via Prose, capitalized provider name)

### Summary detection

- Check `assistant_text` (the agent's stdout output) after completion
- If empty and exit code is 0: warn on stderr that the agent did not provide a summary
- This does NOT change the exit code

### Pre-existing requirements (unchanged)

- Validate that the `prompt` property exists; error if missing
- Use darkmatter to hash body and frontmatter before execution
- Support both structured stream parsing and legacy I/O forwarding paths
- Account for `--interactive` override (limits some validation but most still applies)

> NOTE: some of this was written from the perspective of the `--frontmatter-prompt` being run inside of a non-interactive session (which is very common and the default behavior) but we need to account for the possibility that the session was forced into an `interactive` session. This will limit us in a few things but most of the validation is still valid.
