# Skill Reporting with Claudine CLI

The `claudine skills [FILTER...]` subcommand reports on the current state of **skills** linking from both a **User** and **Repo** based perspective (if CWD is not a git repo then only User scoped).

- **IMPORTANT:** This file describes the functional and reporting requirements for a CLI command but we need to make sure we are always focused on the division of responsibilities between a CLI and its underlying library: a CLI is for reporting only, all business logic MUST go into the library!

## CLI Arguments

| Argument / Flag     | Description                                                                 |
|---------------------|-----------------------------------------------------------------------------|
| `FILTER`            | Positional, repeatable. Filter skills by name. Supports negation (`-rust` or `!rust`) and exact match (`rust!`). |
| `--apply` / `--fix` | Fix missing skill links for non-Claude providers.                           |
| `-v` / `--verbose`  | Global flag. Forces verbose rendering when more than 1 skill matches.       |

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
       - example 1 (user & repo): `<blue><b>Canonical Providers:</b></blue> user: <b>{user-provider}</b>, repo: <b>{repo-provider}</b>`
       - example 2 (user only): `<blue><b>Canonical Providers:</b></blue> user: <b>{user-provider}</b>`
   - If a canonical provider is not configured, the value is shown as `<i><red>not configured</red></i>`

2. Defined Skills

   Within the Defined Skills area we have three distinct ways of displaying this content:

      - **Detail View**
          - Shown when there is exactly 1 skill being shown (typically due to a filter condition)
          - Whether the `-v` / `--verbose` flag was used has no effect
          - The first line of reporting on the skill is the topic name (bold, OSC8 link to SKILL.md) followed by the badge for the scope
          - The second line is the description of the skill (dim, italics) with word wrapping
          - Then a blank line
          - Now we use the `FileSystem` struct from biscuit-terminal to show the skill's files.
              - Include the metric **tokens** via `.show_tokens()`
              - Make files OSC8 links with `.with_file_links()`
              - Left margin of 2 characters

      - **Verbose**
          - If the number of skills (_after filtering_) is less than 6 (and more than 1) we will report using the verbose style.
          - If the user adds the `--verbose` or `-v` flag and there is more than 1 skill then we will also report using the verbose style.
          - This mode lists all skills available (after filter) as an unordered list (leveraging `UnorderedList` component from biscuit-terminal)
              - Each item shows: OSC8-linked topic name (bold), scope badge, then description (dim, italics)

      - **Normal**
          - When we have more than 5 skills (and verbose is not forced) we group skills by scope
          - Grouping uses `BTreeMap<SkillScope, Vec<&SkillInfo>>` so scopes appear in order: User, RepoMasked, Repo
          - Each scope section leads with the scope badge and a count: `{badge} <dim>(<i>{count}</i>)</dim>`
          - Followed by a blank line
          - Then skill names rendered as space-separated OSC8 links (bold) with `WordWrap::BespokeProse(Some(50), ...)` to flow across the terminal width
          - Followed by another blank line

3. Fix Summary

   - This section is only shown when `--apply` / `--fix` was used
   - Rendered after the Defined Skills section
   - Shows: `<b>Fix Summary</b>` header then a dim comma-separated metric line:
     `directories_created={n}, links_created={n}, already_linked={n}, skipped={n}, names_inserted={n}`

4. Auto-Init Behavior

   - When `--apply` is used in a git repo and the repo canonical provider is not configured, claudine automatically runs `claudine init --repo` before proceeding with the fix operation.

5. Exceptions

   - This area is only shown if there **are** exceptions (either `SkillException` entries or `SkillDirectoryDiagnostic` entries)
   - Exceptions are grouped **by provider**, not by scope
   - Each provider gets a header line showing the provider name, user skill path, and repo skill path:
     `<b>{provider} [ user:</b> ~/{user_path}<b>, repo:</b> <magenta>{repo_path}</magenta> ]`
   - Within each provider, exceptions are further grouped by `ExceptionType`:

     - **Missing**: Shows directory-level diagnostics first (if any), then a comma-separated list of missing topic names with word wrapping
     - **Invalid**: Each topic shown individually as an OSC8 link with missing property details: `<b>{topic}</b> (<i>missing the properties <red>{prop1}</red>, <red>{prop2}</red></i>)`
     - **BrokenLink**: Grouped by topic. Each topic is an OSC8 link, with individual broken links listed beneath showing: `<dim><i>in the file <blue>{source_file}</blue> the link <orange>[{link_text}]</orange><red>({link_target})</red> uses an invalid file reference!</i></dim>`
       - In verbose mode, broken link entries also show a `FileSystem` tree for the skill directory with tokens metric and left margin of 4, composed with the link list using `Compose`
     - **NoLinks**: Comma-separated OSC8-linked topic names with word wrapping

   - Exceptions use the same _filtering_ rules as the Defined Skills section so we should ONLY report on those skills which match the fuzzy matching of the filter globs passed in
   - When filters are active, diagnostics are cleared entirely

6. Auto Fixes

   When `--apply` / `--fix` is used, claudine can automatically resolve certain exceptions without human intervention. The following fixes are applied:

   - **Create missing directories**: If a target provider's skill directory does not exist (but its parent does), create it. Tracked as `directories_created`.
   - **Create missing symlinks**: For each canonical skill that has no corresponding entry in a target provider's directory, create a symlink pointing back to the canonical source. Tracked as `links_created`.
   - **Insert missing `name` property**: If a canonical SKILL.md has frontmatter but no `name` field, insert `name: {topic}` (derived from the directory name) immediately after the opening `---`. If there is no frontmatter at all, prepend a new frontmatter block. Tracked as `names_inserted`.
   - **Insert missing `description` property**: If a canonical SKILL.md has a `name` but no `description`, attempt to extract a description from the first paragraph or heading of the body content and insert it into the frontmatter. _(Planned — not yet implemented.)_
   - **Property aliasing (enrich canonical, keep symlinks)**: CLIs ignore frontmatter properties they don't recognize, so we can add alias properties directly to the canonical SKILL.md and continue to symlink everywhere. This avoids derived copies and the drift risk they carry. For skills, frontmatter is relatively uniform across providers so aliasing is rare, but the mechanism is available. Tracked as `aliases_inserted`.

   **Drift detection — `VariantLinkedProperty` exception:**

   If alias properties in a canonical file diverge from their source property (e.g., `name` and `title` no longer match), claudine reports a `VariantLinkedProperty` exception. The `--apply` flag re-syncs them. See `agents.md` for the full alias table — skills have fewer aliases but the same detection mechanism applies.

   **Property passthrough:**

   Many properties — `allowed-tools`, `user-invocable`, `disable-model-invocation`, `context`, `agent`, `hooks`, etc. — are only recognized by some CLIs. However, under the same simplifying assumption that drives the alias strategy (extra properties cause no downside to CLIs that don't use them), these values can live in the canonical SKILL.md and pass through symlinks harmlessly. CLIs that understand a given property will use it; those that don't will ignore it. For the full analysis of which properties are safe to pass through and which block sharing, see [Non-Portable Properties](non-portable-properties.md).

   When the `--verbose` flag is used, the Footer Messages section includes per-property notes showing which CLIs actually consume each property present in the listed skills (see section 7).

   **Fixes that cannot be automated** (and remain as reported exceptions):
   - **BrokenLink**: Broken internal links require human judgement to determine the correct target
   - **NoLinks**: Skills that exist in the canonical provider but have no body content — these need human authoring
   - **Invalid** (beyond name/description): Missing properties that have no derivable default

7. Footer Messages

   This section is optionally rendered, it depends on whether the current _state_ dictates that additional context should be provided to the user. The following are messages that _might_ be shown (including an explanation of when they should be):

   - **fix**
       - the message `<dim><i>use <red>--fix</red> to attempt to fix the reported issues</i></dim>`
       - only shown when there are exceptions being reported on AND `--apply` was NOT used
   - **user only**
       - the message `<dim><i>the current working directory is <b>not</b> a <b>git</b> repo so we are only showing user-based scope</i></dim>`
       - only shown when the CWD is not inside a git repo
   - **verbose**
       - the message `<dim><i>using the <green>--verbose</green> switch will provide not only topic names but also descriptions</i></dim>`
       - only shown when there are more than 10 skills listed and the user has not used the `--verbose`/`-v` flag
   - **filtering**
       - the message `<dim><i>using parameters in the CLI call will act as <b>filters</b> to help reduce the skills to only those you are interested in</i></dim>`
       - only shown when no filter parameters were provided
   - **property passthrough notes** (verbose only)
       - only shown when the `-v` / `--verbose` flag is used AND at least one skill in the listing has properties that are only consumed by a subset of CLIs
       - each such property that appears across the listed skills gets its own line indicating which CLIs consume it, e.g.:
           - `<b><yellow>allowed-tools</yellow></b><dim> used by Claude and Goose; other CLI Agents will ignore</dim>`
           - `<b><yellow>user-invocable</yellow></b><dim> used by Claude; other CLI Agents will ignore</dim>`
           - `<b><yellow>disable-model-invocation</yellow></b><dim> used by Claude; other CLI Agents will ignore</dim>`
       - only the properties that actually appear in the currently listed skills are shown

	If only a single message is to be displayed then it should just be displayed "as is" (indented with a leading space) with a leading blank line to separate it from the sections above.

	If _more_ than one message is to be displayed then the messages should be added to an `UnorderedList` struct. The leading blank line should be added in this use-case too.
