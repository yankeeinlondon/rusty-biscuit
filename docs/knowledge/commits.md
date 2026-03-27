# Knowledge Learned about Committing

> This document is a bulleted list of things learned about commits which may not be intuitive and obvious with just a knowledge of git and this repo.

- I am not responsible for running tests, this will already have been done before i'm handed the job of making git commits

- When viewing commit history, use `git log --oneline -n` directly — `sniff git commits` is not a valid command in this repo

- The repo uses lowercase after the colon in conventional commits (e.g., `docs(darkmatter):` not `Docs(Darkmatter):`)

- When committing agent/skill files in `.claude/`, use `docs(<area>)` as the prefix for documentation restructuring changes

- Subagents may see a different set of staged files than what the user specifies in the prompt (due to concurrent work or a filtered list). When this happens, simply stage only the files assigned without resetting or unstaging anything. Never use `git reset *` and never try to "fix" staged files by unstaging and restaging groups

- When multiple related files are staged together (e.g., a directory rename like `transform/` → `compose/`), git commits them as an atomic unit. In such cases, subagents will not be able to split them into separate granular commits even if semantically distinct groups were planned - the files must be committed together as they were staged

- Using `git commit -- path/to/file` does NOT limit the commit to only those paths if other files are also staged. Git will commit ALL staged files. To commit only specific files, ensure ONLY those files are staged (not using `git add` broadly) or use `git commit -m "message" -- path1 path2` with explicit paths when you are certain only those files are staged

- When files are renamed and staged as renames (e.g., `old.md -> new.md`), specifying explicit paths to `git commit -m "..." -- new.md` only commits the "new" side; the "deleted" side remains staged even when rename detection is complete (R100). Always verify both sides of a rename are committed with `git diff --staged --name-status`

- When committing files in directories with spaces in their names (e.g., `darkmatter/features/2026-03-24. reference-validation/`), paths must be properly quoted when passed to `git commit -m "..." -- "path with spaces/file.md"`. Failure to quote results in git treating each space-separated token as a separate path argument.

- Do not second guess the files which were staged; the user intentionally chose which files they were interested in committing.

- When the user lists files as "(created)" in the prompt, those files are untracked (not staged) and must be added with `git add` before they can be committed. Check `git status` to verify the actual state when subagents report files as untracked rather than staged.
