# Knowledge Learned about Committing

> This document is a bulleted list of things learned about commits which may not be intuitive and obvious with just a knowledge of git and this repo.

- I am not responsible for running tests, this will already have been done before i'm handed the job of making git commits

- When viewing commit history, use `git log --oneline -n` instead of `sniff git commits -n` which doesn't support the `-n` flag

- The repo uses lowercase after the colon in conventional commits (e.g., `docs(darkmatter):` not `Docs(Darkmatter):`)

- When committing agent/skill files in `.claude/`, use `docs(<area>)` as the prefix for documentation restructuring changes

- Subagents may see a different set of staged files than what the user specifies in the prompt (due to concurrent work or a filtered list). When this happens, simply stage only the files assigned without resetting or unstaging anything. Never use `git reset *` and never try to "fix" staged files by unstaging and restaging groups

- When multiple related files are staged together (e.g., a directory rename like `transform/` → `compose/`), git commits them as an atomic unit. In such cases, subagents will not be able to split them into separate granular commits even if semantically distinct groups were planned - the files must be committed together as they were staged

- Using `git commit -- path/to/file` does NOT limit the commit to only those paths if other files are also staged. Git will commit ALL staged files. To commit only specific files, ensure ONLY those files are staged (not using `git add` broadly) or use `git commit -m "message" -- path1 path2` with explicit paths when you are certain only those files are staged
