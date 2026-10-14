---
operation: "merge-conflict"
conflicts: {{ ctx.merge_conflicts }}
interactive: true
initialize:
    - when: "!conflicts"
      then:
          - info: |-
              There are **no** merge conflicts in this branch! Exiting the "merge-conflict" prompt.
          - skip
start:
    info: starting the merge conflict analysis for the `{{ctx.branch}}` branch; there are **{{ length(conflicts) }}** conflicts to be addressed.
    message: starting the merge conflict analysis for the `{{ctx.branch}}` branch; there are **{{ length(conflicts) }}** conflicts to be addressed.
---
## Context

You are in the **{{ctx.branch}}** branch of the **{{ctx.repo}}** repo. A merge, rebase, or cherry-pick has left the following files with conflicts:

{{ as_unordered_list(conflicts) }}

Recent git commits include (last 10):

{{ ctx.recent_commits }}

## File Type Tips

- `.vscode/settings.json`: often just a case of both branches having added new words to the "cSpell.words" dictionary. In these cases you should include a unique list of all words added across the two branches
- `codebook.toml`: same principle -- aka, additive -- but for the codebook spell checker used in Zed editor
- `.zed/settings.json`:
    - treat conflicts as "additive" where possible (e.g., if both branches mutated different _keys_ in the JSON then accept both)
    - if the key with the conflict is an array or object then we should try to be "additive" at this level unless there is a good reason not to be
        - if we're taking an "additive" approach to array entries always be sure that the new array is a _unique_ list of items
    - if there is any other form of conflict at the key level it's almost always the newer version which should be kept; "newer" means the side whose last commit touching that key is more recent (check with `git log -L` or `git blame`)
- `Cargo.lock`: never hand-merge it. Take either side, then run `cargo check` (or `cargo update -w`) so Cargo regenerates it
- generated files (e.g., `claudine/lib/src/provider/*/data.rs`): resolve the _inputs_ to the generator, then regenerate (e.g., `claudine providers generate`); never merge generated output by hand
- `docs/` pages and specs: conflicts are usually two edits to prose; keep both and reconcile them rather than picking a side

## Task

1. Identify the operation in progress by checking for `MERGE_HEAD`, `REBASE_HEAD`, or `CHERRY_PICK_HEAD` in the git directory, and state which side is "ours" and which is "theirs" (they swap meaning during a rebase).
2. Create a plan for resolving each conflicted file; group the low-risk files (spell-check dictionaries, lockfiles, generated files) separately from the rest.
3. Resolve the low-risk files first.
4. For any file where a resolution could cause a regression:
    - analyze both branches' history with the file so you understand the rationale for each branch's current state
    - present the history and reasoning that each branch followed to arrive at its current state
    - describe the set of options you think are reasonable to address this
    - recommend one and explain why you are recommending this choice (including any assumptions you are making)
    - **wait for the user's decision** before resolving that file; batch these decision points rather than interrupting for each one
5. Verify the resolution:
    - no conflict markers remain (`git diff --check`, and search the resolved files for `<<<<<<<`, `=======`, and `>>>>>>>`)
    - `cargo check` passes for every package area containing a resolved file
6. Conclude the operation (`git commit` for a merge or cherry-pick, `git rebase --continue` for a rebase):
    - keep git's default merge message and append a short summary of how the conflicts were resolved
    - the commit must be signed; never bypass signing with `--no-gpg-sign`
    - do not add agent attribution or co-authorship trailers
    - run `git verify-commit HEAD` afterward
7. Summarize the conflicts and how each was resolved.

You are done when no unmerged paths remain, verification passes, the operation has been concluded, and `git verify-commit HEAD` succeeds.
