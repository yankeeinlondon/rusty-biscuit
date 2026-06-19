---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: The mandatory 500 ms performance target is not met

The specification requires both default text and JSON execution to complete in
under 500 ms (`spec.md:69-77`). The implementation record reports approximately
800 ms for text and 780 ms for JSON on the target repository
(`plan.md:90-93`), but marks both tasks complete anyway.

This is a failed acceptance criterion, not a successful optimization result.
Continue profiling the end-to-end command and remove the remaining dominant
work. In particular, the default request still enables repository metadata
whose local-branch collection performs one ahead/behind walk per branch
(`sniff/lib/src/request.rs:416-429`), and worktree enumeration still scans dirty
status for every non-current worktree
(`sniff/lib/src/filesystem/git/remote_refresh.rs:648-712`). Record a repeatable
text and JSON benchmark under 500 ms before release.

### High: Case A drops the current worktree when the main worktree is on a non-main branch

Case A is selected solely from `current_branch != "main"`, but current details
are then found only among `git.worktrees` entries
(`sniff/cli/src/output/filesystem/mod.rs:616-654`). That map contains linked
worktrees only; the main worktree is not included. If a user checks out
`feature-x` in the main worktree, `base_repo_root` is `None` and no entry has
`is_current`, so the Worktrees section prints neither the main location nor the
required current-worktree details. Detached HEAD and repositories whose primary
branch is named `master` have the same structural problem.

Represent the current and main worktrees explicitly instead of inferring them
from branch spelling and a linked-worktree-only map. Add Level 1 integration
fixtures for a non-main branch in the main worktree, detached HEAD, and a
`master`-based repository.

### High: Repositories with no linked worktrees omit required Case B output

The renderer gates the entire Worktrees section on
`!git.worktrees.is_empty()` (`sniff/cli/src/output/filesystem/mod.rs:608-663`).
The specification still requires Case B to show the current main worktree and
an other-worktree count, which is zero in this case (`spec.md:29-38`). The new
test explicitly asserts the opposite behavior
(`sniff/cli/src/output/filesystem/mod.rs:1501-1522`).

Render the section for the main worktree even when there are no linked
worktrees, including `there are 0 other active worktrees in this repo`, and
replace the contradictory test.

### High: Terminal styling has only ineffective Level 1 coverage

The double-underline requirement is user-observable terminal rendering and
therefore requires Level 2 verification. The only new style test merely checks
that the word `Status` appears and explicitly does not assert double underline
or fallback behavior (`sniff/cli/src/output/filesystem/mod.rs:1309-1319`).
There is no `level2_*` git-status test.

Add real-terminal capture coverage through the sniff `test-l2` recipe for
double underline, graceful fallback, hyperlinks, and the exact blank-row
layout. The existing Level 1 spacing test also checks only that adjacent lines
are blank; it does not reject a second blank line, despite the exact-one-row
contract (`sniff/cli/src/output/filesystem/mod.rs:1321-1382`).

### Medium: The target worktree name and relative path are not rendered

Case A requires the worktree name and a relative path label
(`spec.md:21-25`). The renderer substitutes `current_wt.branch` for the
worktree name and displays only `path.file_name()` for every link
(`sniff/cli/src/output/filesystem/mod.rs:548-558,638-653`). A worktree named
`login-fix` on branch `feature/login`, or any path requiring more than its
basename for context, therefore produces the wrong output.

Preserve the registry worktree name in `WorktreeInfo` or iterate the map entry,
and compute the visible path relative to the current repository/worktree as
specified. Add fixtures where worktree and branch names differ and where the
relative label contains multiple components.

### Medium: Text and JSON behavior lack end-to-end Level 1 fixtures

The lazy-loading tests call the private `get_worktrees` helper directly, while
the rendering tests construct `GitInfo` manually. The CLI JSON test checks only
top-level object shape (`sniff/cli/tests/cli.rs:719-761`). No test creates a
repository with multiple linked worktrees and proves that:

- default text shows current details and count-only others;
- default JSON has current ahead/behind while non-current worktrees are not
  computed;
- `--refresh-remotes` or `full_worktree_details(true)` restores all details;
- text and JSON select the same current worktree.

These are Level 1 integration requirements. Add real temporary-repository CLI
fixtures rather than relying only on fabricated output structs.

## Verification Levels

| Requirement | Required level | Strongest evidence | Result |
|---|---|---|---|
| Lazy current-only worktree calculations | Level 1 | Private helper unit tests | Partial; no public/CLI fixture |
| Case A and Case B text behavior | Level 1 | Fabricated `GitInfo` unit tests | Gaps for main-worktree Case A and zero linked worktrees |
| Default JSON lazy behavior | Level 1 | JSON shape smoke test | Gap |
| Exact blank-row layout | Level 1 plus Level 2 capture | Loose unit assertion | Gap |
| Double underline and graceful degradation | Level 2 | Header-text-only unit test | Gap |
| Hyperlink rendering/fallback | Level 2 | No git-status terminal test | Gap |
| Performance under 500 ms | Benchmark, outside L1-L3 | Recorded 780-800 ms | Failed |

Level 3 is not applicable because this feature has no keyboard, mouse, paste,
or terminal-input-encoder requirement.

## Verification

- Reviewed the specification, execution plan, effective working-tree diff,
  library request and worktree logic, CLI rendering, documentation, and tests.
- `git diff --check` was reviewed through the effective diff; no patch-format
  issue was observed.
- Rust tests, Clippy, and doctests could not be rerun because rustup has no
  installed or configured toolchain in this session.

## Decision

Not ready for production. The recorded implementation misses the hard
performance target, omits required output for valid repository states, and has
no Level 2 proof for the terminal-rendering requirements.
