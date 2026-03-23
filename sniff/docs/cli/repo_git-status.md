# Git Status from `sniff repo git-status`

the way we present worktrees in `sniff repo git-status` should be improved:




- the first line -- which talks about the "base repo" -- incorrectly refers to the worktree directory, NOT the base repo as it suggests!
    - this line should be formatted based on whether the user is anywhere in the base repo when they ran the CLI
        - in base repo: `<b>Base Repo:</b> you are in the base repo which is on the <blue-500>{{branch}}</blue-500> branch`
        - in a worktree: `Base Repo: <dim>the base repo is located at <blue-500>{base-filepath}</blue-500></dim>`
- the remaining lines in the Worktrees section are worktrees that currently exist:
    - Here again we should report differently when we are inside the particular worktree versus when not
        - in the worktree: `<b>{worktree}:</b> you are <green-500>8 ahead</green-500>, <red-500>2 behind</red-500> of <b>main</b>`
        - outside of worktree: `{worktree}: <dim>is <green-500>3 ahead</green-500>, <red-500>2 behind</red-500> of <b>main</b>`

