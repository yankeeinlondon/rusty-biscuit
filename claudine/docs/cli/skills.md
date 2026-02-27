# Skill Reporting with Claudine CLI

The `claudine skills <filter>` subcommand reports on the current state of **skills** linking from both a **User** and **Repo** based perspective (if CWD is not a git repo then only User scoped).


## Reporting Sections

The reporting is broken down into the following sections:

1. Header Intro

   The initial four lines reported are always the same:

   - line 1: _blank line_
   - line 2: `<blue><b>Skills</b></blue>`
   - line 3: `<blue>==================</blue>`
   - line 4: _blank line_

   We then report on the **canonical** base providers:

   - the _canonical_ base providers will be defined in the user and repo configuration files and are set when the user runs `claudine init` (via an interactive Q and A).
       - obviously if the current working directory is **not** a git repo then we only report on the user scoped canonical provider
   - to provide symbolic links _to_ skills we need to isolate which provider will _provide_ the skill sources ... the "canonical provider" is the designated provider of skills.
   - based on this context here are two examples of what **line 5** of the Header Intro section might look like:
       - example 1 (user & repo): `<blue><b>Canonical Providers: </b></blue> user: <b>{user-provider}</b>, repo: <b>{repo-provider}</b>`
       - example 2 (user only): ``

2. Defined Skills

   Within the Defined Skills area we have three distinct ways of displaying this content:

      - **Detail View**
          - Shown when there is exactly 1 skill being shown (typically due to a filter condition)
          - Whether the `-v` / `--verbose` flag was used has no effect
          - The first line of reporting on the skill is the topic name (bold) followed by the badge for the scope
          - The second line is the description of the skill (dim, italics)
          - Then a blank line
          - Now we use the `FileSystem` struct from biscuit-terminal to show the skill's files.
              - Include the metric (tokens)

      - **Verbose**
          - If the number of skills (_after filtering_) is less than 6 (and more than 1) we will report using the verbose style.
          - If the user adds the `--verbose` or `-v` flag and there is more than 1 skill then we will also report using the verbose style.
          - This mode lists all skills available (after filter) as an unordered list (leveraging `UnorderedList` component from biscuit-terminal)
              - The list is sorted by "scope" first -- "User" -> "Repo (masked)" -> "Repo" -- and then alphabetically.
              - The topics are all OSC8 links to the `SKILL.md` file
              - 
      - **Normal**
          - When we have more than 5 skills we do not want to overwhelm the terminal with information so instead of displaying in the format we do in verbose mode, we instead
          - create sections for each scoping section ("scope" first -- "User" -> "Repo (masked)" -> "Repo")
          - each scoping section will lead with the badge for that scope and a blank line following
          - then we will list all the skills in a tab-delimited 

3. Exceptions

   - This area is only shown if there **are** exceptions
   - We will report on _user scoped_ and _repo scoped_ issues using color variance to distinguish between the two scopes. The _repo scoped_ topics will be colored in purple.
   - Exceptions show no difference regardless of the `--verbose` / `-v` 
   - However, Exceptions use the same _filtering_ rules as the Defined Skills section so we should ONLY report on those skills which match the fuzzy matching of the filter globs passed in

4. Footer Messages

   This section is optionally rendered, it depends on whether the current _state_ dictates that additional context should be provided to the user. The following are messages that _might_ be shown (including an explanation of when they should be):

   - **fix**
       - the message `<i><dull>use <red>--fix</red> to attempt to fix the reported issues</i>`
       - only shown when there are exceptions being reported on
   - **user only**
       - the message `<i><dull>the current working directory is </dull>not<dull> a <bold>git</bold> repo so we are only showing user-based scope</i>`
       - only shown when the CWD is not inside a git repo
   - **verbose**
       - the message `<i><dull>using the <green>--verbose</green> switch will provide not only topic names but also descriptions`
       - only shown when there is more than 10 skills listed and the user has not used the `--verbose`/`-v` flag
   - **filtering**
       - the message `<i><dull>using parameters in the CLI call will act as <bold>filters</bold> to help reduce the skills to only those you are interested in</dull></i>`

	If only a message is to be displayed then it should just be displayed "as is" but with a leading blank line to separate it from the sections above.

	If _more_ than one message is to be displayed then the messages should be added to an `UnorderedList` struct. The leading blank line should be added in this use-case too.

