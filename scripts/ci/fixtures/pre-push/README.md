# Pre-push fixtures

Shared inputs for the `ci-push` tests (`scripts/ci-push/`) and, where noted,
the Python constraint reader's tests.

- `plans/`: resolved plans valid under the current planner schema, one per
  scenario a constraint check distinguishes (executing, reused, absent).
- `constraints/`: valid constraint records. `minimal.json` sets only the
  required fields; `scoped.json` sets every optional one.
  `parity.json` is the shared case table described below.
- `pull-requests/`: `gh pr list --json number,baseRefName` output.

Each is a **control**: a test reads it unchanged for the positive result, and
derives each negative case from it by changing exactly one field.

## Constraint parity cases

`constraints/parity.json` is not a control. It is a table of cases that both
constraint readers run: the Rust one (`scripts/ci-push/tests/constraints.rs`)
and the Python one (`scripts/ci/test_constraints.py`, `ParityTests`). A case
that one reader passes and the other fails is a disagreement to fix, not a
case to delete.

- `identities`: a remote URL, the `repository_identity` it normalizes to, and
  the `store_segments` of that identity.
- `cases`: a constraint store and what reading it must produce.
  - `store`: store-relative path (always `/`-separated) to file content. A
    string is written byte-for-byte, so malformed JSON, duplicate keys, and
    syntax extensions can be spelled; any other value is serialized as JSON.
    `null` means the store directory does not exist.
  - `repository`, `branches`, `today`: the push being checked. `today` is
    explicit so an expiry case never depends on the day the test runs.
  - `plan`: resolved-plan text, written byte-for-byte. Absent means no plan,
    which satisfies nothing.
  - `expected`: store-relative paths in reading order for `active`,
    `expired`, and `unsatisfied` (computed over `active`), and
    `malformed` as `{path, location}` where `location` is the JSON field the
    error names (`""` for the document as a whole). `plan_error` is the
    location a refused plan names; when present, `unsatisfied` is not checked.

Add a case here, rather than to one reader's tests, whenever matching or
validation behavior changes.
