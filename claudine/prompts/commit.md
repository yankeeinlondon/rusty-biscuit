---
timeout: 10m
step_timeout: 7m
success:
    stderr: "Git commit flow completed"
    message: "Git commit flow completed"
failure:
    message: "Git commit flow failed"
---
# Commit Changes

You are responsible for committing the current implementation state.

## Requirements

- Inspect the git working tree and staged files.
- If no files are staged, stage the appropriate files for the completed flow step.
- Do not stage unrelated user changes.
- Do not revert user changes.
- Group changes into coherent commits when multiple independent changes are present.
- Use Conventional Commit style:
    - `feat(scope): summary`
    - `fix(scope): summary`
    - `docs(scope): summary`
    - `chore(scope): summary`
- Keep the first line under 72 characters.
- Include a blank line and concise bullet body when the change needs explanation.
- Commit only the files that belong to the semantic group being committed.
- Do not push to any remote.

## Closure

- Report each commit hash and subject.
- Report any files intentionally left uncommitted.
- Do not ask the user for feedback or permission; this is a non-interactive session.
