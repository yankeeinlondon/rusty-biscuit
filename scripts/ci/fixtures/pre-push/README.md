# Pre-push fixtures

Shared inputs for the `ci-push` tests (`scripts/ci-push/`) and, where noted,
the Python constraint reader's tests.

- `plans/`: resolved plans valid under the current planner schema, one per
  scenario a constraint check distinguishes (executing, reused, absent).
- `constraints/`: valid constraint records. `minimal.json` sets only the
  required fields; `scoped.json` sets every optional one.
- `pull-requests/`: `gh pr list --json number,baseRefName` output.

Each is a **control**: a test reads it unchanged for the positive result, and
derives each negative case from it by changing exactly one field.
