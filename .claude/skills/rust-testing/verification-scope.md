# Scope Verification Gates by Blast Radius

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

Before running a final build, test, or lint gate:

1. Record the changed packages and package areas.
2. Find downstream consumers of changed symbols with GitNexus upstream
   impact, then map them to packages with `sniff repo packages`,
   `sniff repo package-areas`, and `sniff repo package-dependencies`.
3. Run `just build`, `just test`, and `just lint` in every affected package
   area, or an exact package selector where a narrower recipe exists. To run
   exactly what CI's `lint` and `test` gates would run for the branch, use
   `just ci-local` at the root (`--dry-run` prints the scope).
4. Report the selected scope and commands with the results.

Root `just test` runs the selected packages in one nextest invocation and
accepts exact package names or package-area paths as selectors
(`just test claudine darkmatter`). After editing a `test` recipe, run
`just check-test-interrupts` to confirm it still exits `130` on Ctrl+C.

Do not use `cargo build/check/test --workspace`, a bare root `cargo`
command, or an unscoped root `just` lifecycle recipe as a generic safety net.
A changed public API is a reason to include its actual consumers, not every
workspace member. Run workspace-wide only when the user asks for it or a
documented repository-wide invariant cannot be verified from the derived
scope, and record that reason first. A non-comment change to a global path
such as `.config/nextest.toml` already selects the whole workspace in
`ci-local` and CI; budget for that before touching runner configuration.

How CI derives and gates its scope (package metadata, per-package cells,
evidence reuse, the pre-push hook) is owned by the `rust-devops` skill and
the repository `CLAUDE.md`; a test author needs only that a test added to a
package runs in that package's own cell. CI's own suites follow the same rule:
`repo-deps` (`scripts/`) owns the planner and `scripts/ci/test_*.py`
contracts, and `test-toolkit` owns `ci_workflow_contracts`.

## When work is split across agents

The steps above are the **coordinator's** final gates, run once per phase
after every agent's work is merged and no agent is still running. An agent
working on one slice runs only the tests for what it changed (its new or
edited tests and those of the modules it touched), selected with a nextest
filter or the area recipe's positional filter, never the area's full
`just test`.

Concurrent full suites on one host starve each other: timing-sensitive tests
fail, each failure prompts another full run, and that run adds more load.
When a test fails during agent work, rerun it alone first. A test that passes
alone is load-sensitive; record that rather than answering it with another
suite run.
