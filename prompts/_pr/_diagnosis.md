## Diagnosing Local Test Failures

A list of red test names is not a diagnosis. Before you report anything, every failure must be explained well enough that the caller can decide whether to fix it without re-running your investigation. For **each** failing test or gate you must establish:

1. **What failed** — the gate, the package, the test binary, and the exact test name, plus the assertion or error text that matters (trimmed; never a wall of log output).
2. **Why it failed** — the root cause. "The expected hash differs" is a symptom; "the shipped `implement-plan.md` gained a `log` schema property and the fixture copy was never re-derived" is a cause. Read the test, read the code or document it exercises, and follow the failure until you can name the change or condition that produces it.
3. **Where it came from** — exactly one attribution, backed by evidence:
    - `branch` — introduced by a commit in `origin/{{base}}..HEAD`; name the commit when you can
    - `working-tree` — caused by uncommitted changes in the checkout (a dirty checkout makes the hook replan from the working tree, so these can gate a push even though they are not being pushed)
    - `base` — already failing on `origin/{{base}}`; this branch did not cause it
    - `environment` — the host, toolchain, disk, network, a missing tool, a real-terminal backend, or flakiness; the code is not at fault
    - `unknown` — only after the evidence steps below were attempted and were inconclusive; say what you tried

### Where the evidence lives

- The hook prints the gate summary and a line of the form `Reports retained under <home>/.rusty-biscuit/ci-evidence/<sha>/<environment>`. The `L1/` and `L2/` directories there hold one JUnit XML file per package; each `<testcase>` with a `<failure>` child carries the panic text and captured output. Parse these rather than scrolling the console log.
- The hook log itself (the output you captured from `git push`) shows which gates ran, which were reused from prior evidence, and which failed.

### Establishing attribution

Do not attribute from file names alone. **A test whose files are unchanged on this branch can still be broken by this branch**: a library change can alter how an untouched fixture, prompt, or document parses or renders. Conversely, a test touched by this branch can be failing for an environmental reason. Use evidence, in roughly this order, stopping when the answer is clear:

1. **Reproduce in isolation on `HEAD`.** Re-run just the failing test with `cargo nextest run -p <package> --test <binary> <test-name>` using the same features the hook used (they appear in the hook's `cargo-nextest` command line, e.g. `--features terminal-tests`). Run it up to three times. Passing on a rerun points to `environment` (flaky); say so and include the pass/fail counts.
2. **Check the base.** Create a temporary, detached worktree of `origin/{{base}}` **outside this checkout** (for example with `git worktree add --detach <temp-dir> origin/{{base}}`), run the same single test there, and remove the worktree afterwards with `git worktree remove`. Failing on the base means `base`.
3. **Find the introducing commit.** When the base passes and `HEAD` fails, use `git bisect run` over `origin/{{base}}..HEAD` with the single-test command in the temporary worktree to find the commit, then read that commit's diff to explain *why* it causes the failure.
4. **Rule out the working tree.** If `git status --short` is not empty, determine whether the failure involves uncommitted files before you blame a commit.

Reproducing on the base and bisecting cost build time. That cost is expected; an unsupported attribution is not acceptable. When a step is genuinely impossible (for example an L2 test whose terminal backend is unavailable), say which step was skipped and why, and lower `confidence` accordingly.

### Group related failures

Several red tests frequently share one cause (for example two drift tests pinned to the same file). Explain the shared cause once and list every test it accounts for, so the caller sees the number of *problems*, not just the number of red tests.

### Do not fix anything

This stage diagnoses; it does not repair. Do not edit source, tests, fixtures, or prompts, do not refresh pinned hashes, and never bypass the hook (`--no-verify`, `RUSTY_BISCUIT_PRE_PUSH=warn|scope-only|off`). Whether to fix is the caller's decision, made in the next stage.

### Writing `issue`

`issue` is the entire briefing a fixing agent will receive, and that agent will not have your session. Make it self-contained:

- the branch, `HEAD` SHA, and base
- each distinct problem: the failing tests it accounts for, the root cause, the attribution with its evidence, and the introducing commit when known
- the exact commands that reproduce each failure
- anything you ruled out, so the fixing agent does not repeat it
- constraints the fix must respect (for example "the fixture must be re-derived from the shipped prompt, keeping only the documented removals")
