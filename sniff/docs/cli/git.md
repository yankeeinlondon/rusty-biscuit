# The `sniff git` Subcommand

## Status Section

The status section provides a single pane of glass to report on the local repo/branch's current status.

- Title: Prose::new(`<b><u>Status</u></b>`)
- The output is rendered as an `UnorderedList` (from biscuit-terminal library)
- If no commits and no file changes, render: Prose::new(`<dim>No changes</dim>`)

1. Commits

   - Recent commits are listed oldest-first (most recent at the bottom)
   - Count defaults to 10, configurable via `--history` / `-h` switch
   - Each commit is parsed as a `ConventionalCommit`
   - Conventional commits render as:
       - Prose::new(`[<b>{short-sha}</b>] <b><yellow>{operation}</yellow></b>(<dim>{scope}</dim>) <i>at</i> <blue><b>{time}</b></blue> {date-prefix}<blue>{date}</blue>{ref-decorations}: <dim>{description}</dim>`)
       - the `{date-prefix}` is `<i>on</i> ` when the commit is not from today, empty otherwise
       - the `{scope}` part (including parentheses) is omitted when there is no scope
       - the `{ref-decorations}` are formatted ref names (branches, tags, HEAD) pointing to this commit
   - Non-conventional commits render as:
       - Prose::new(`[<b>{short-sha}</b>] <dim>{message}</dim> {date-prefix}<blue><b>{date}</b></blue>{ref-decorations}`)
       - the `{message}` is truncated to 50 chars (with `...` suffix) if longer

2. Staged files

   - Each staged file renders as:
       - Prose::new(`<lime>staged: {dir}<b>{filename}</b></lime>`)
       - includes files with status `Staged` or `Both`

3. Modified files

   - Each modified file renders as:
       - Prose::new(`<yellow>modified: {dir}<b>{filename}</b></yellow>`)
       - includes files with status `Modified` or `Both`

4. Untracked files

   - Each untracked file renders as:
       - Prose::new(`<red>untracked: {dir}<b>{filename}</b></red>`)

## Worktrees Section

- this section is only displayed IF the repo has worktrees defined
- Prose::new(``)

## Meta Section

- The Meta section should be fully defined as a nested `UnorderedList` (from biscuit-terminal)
- The sub-sections (aka, the top level items in the UnorderedList) are:
    - Prose::new(`<b>Local:</b>`)
    - Prose::new(`<b>Remotes:</b>`)
    - Prose::new(`<b>Config:</b>`)

Now let's address each top level item individually:

1. Local

   - Verbose Mode
       - Prose::new(`<b>Branches:</b>`)
       - then add the current branch as:
           - Prose::new(`<bold><blue>{current}{dirty}</blue></bold> [<dim>{short-hash}</dim>](<dim><i>current</i></dim>)`)
           - the `{dirty}` value is `<red>+</red>` if branch is currently "dirty"
       - then add each _other_ branch as a child node in the list:
           - Prose::new(`{branch} [<dim>{short-hash}</dim>] - {ahead-behind}`)
           - see the [ahead/behind](#ahead--behind-rendering) section for rendering
   - Normal Mode
       - Prose::new(`<b>Branches:</b> <blue>{current}</blue> (<dim>{other-branches}</dim>)`)
       - do not add the parenthesis if there is only one branch
       - the other branches should be split by ", "

2. Remotes

   - each remote is listed out
       - Prose::new(`<b>{remote}:</b> {ahead-behind} <i>of</i>{branch} - <a {URL}><blue></blue>{org}/{repo}</a> <i>on</i> {provider}`)
       - see the [ahead/behind](#ahead--behind-rendering) section for rendering
       - the `{branch}` is the _default branch_ from the perspective of the remote
       - ensure that the `{org}/{repo}` is an OSC8 link to the remote
       - the `{provider}` is bold faced and has values like `Github`, `Bitbucket`, etc.
   - if in verbose mode
       - a child list is added to each remote (that has branches other than it's default), we will list the _other_ branches (aka, those not deemed the _default branch_ from the perspective of the remote)
       - each branch will be rendered with `<b>{branch}</b> <i>branch is at</i> {{short-hash}} - {{ahead-behind}}`

3. Config

   - The following **subsections** are each rendered as a child UnorderedList
   - User Info:
       - `<b>User Info:</b> <blue>{full name}</blue> &gt<dim>{email}</dim>&lt;`
   - Crypto:
     - this section is only rendered with the `--verbose` / `-v` flag is used
     - add item Prose::new(`<b>Crypto</b>`)
     - then add these underneath this section:
         - Prose::new(`<b>GPG:</b> use-agent: <blue>{gpg.use-agent}</blue>, program: <blue>{gpg.program}</blue>, helper: <blue>{credential.helper}</blue>`)
         - Prose::new(`<b>GPG Key</b>: <blue>{user.signingkey}</blue>`)
         - Prose::new(`<b>Signing:</b> commit: <blue>{commit.sign}</blue>, tags: <blue>{tag.gpg}</blue>,  `)
   - Pager:
       - this section is only rendered with the `--verbose` / `-v` flag is used
       - Render with: Prose::new(`<b>Pager:</b> <blue>{core.pager}</blue>`)
       - if the `core.pager` is equal to **delta** then:
           - Add the following child items:
               - Prose::new(`theme: <dim>{delta.syntax-theme}</dim>`)
               - Prose::new(`light-mode: <dim>{delta.light | false}</dim>`)
               - Prose::new(`side-by-side: <dim>{delta.side-by-side}`)

4. Gutter

      - this area of the reporting is at the end and most line items only render _conditionally_
      - if more than one item is going to be rendered then we will make them all into an UnorderedList
      - **Remote**
          - if the user has _not_ used the `--remote` flag then we will render
          - Prose::new(``)

## Ahead / Behind Rendering

There are a few locations where we're specifying the ahead/behind status. For each of these we will render that with:

- Prose::new(`<green>{arrow}{#} ahead</green>, <red>{arrow}{#} behind`); where:
    - if the terminal is not using a nerdfont then `{arrow}` will always be a empty string
    - if the terminal IS a nerdfont then:
        - the `{arrow}` it an empty string when the corresponding number/count is 0
        - the `{arrow}` is a `f0737` character when non-zero and associated to **ahead**
        - the `{arrow}` is a `f072e` character when non-zero and associated to **behind**
