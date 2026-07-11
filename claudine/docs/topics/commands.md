# Command Reporting with Claudine CLI

The `claudine commands [FILTER...]` subcommand reports on the current state of **slash commands** linking from both a **User** and **Repo** based perspective (if CWD is not a git repo then only User scoped).

- **IMPORTANT:** This file describes the functional and reporting requirements for a CLI command but we need to make sure we are always focused on the division of responsibilities between a CLI and its underlying library: a CLI is for reporting only, all business logic MUST go into the library!

## CLI Arguments

| Argument / Flag     | Description                                                                 |
|---------------------|-----------------------------------------------------------------------------|
| `FILTER`            | Positional, repeatable. Filter commands by name. Supports negation (`-commit` or `!commit`) and exact match (`commit!`). |
| `--apply` / `--fix` | Fix missing command links for non-Claude providers.                         |
| `-v` / `--verbose`  | Global flag. Forces verbose rendering when more than 1 command matches.     |

## Reporting Sections

The reporting is broken down into the following sections:

1. Header Intro

   The initial four lines reported are always the same:

   - line 1: _blank line_
   - line 2: `<blue><b>Commands</b></blue>`
   - line 3: `<blue>==================</blue>`
   - line 4: _blank line_

   We then report on the **canonical** base providers:

   - the _canonical_ base providers will be defined in the user and repo configuration files and are set when the user runs `claudine init` (via an interactive Q and A).
       - obviously if the current working directory is **not** a git repo then we only report on the user scoped canonical provider
   - to provide symbolic links _to_ commands we need to isolate which provider will _provide_ the command sources ... the "canonical provider" is the designated provider of commands.
   - **Only Markdown-based providers** are eligible as canonical for commands. Gemini (TOML), Goose (MCP), and KimiCode (built-in only) are excluded because cross-format conversion is not yet supported. Eligible providers: Claude, Codex, OpenCode, Qwen.
   - based on this context here are two examples of what **line 5** of the Header Intro section might look like:
       - example 1 (user & repo): `<blue><b>Canonical Providers:</b></blue> user: <b>{user-provider}</b>, repo: <b>{repo-provider}</b>`
       - example 2 (user only): `<blue><b>Canonical Providers:</b></blue> user: <b>{user-provider}</b>`
   - If a canonical provider is not configured, the value is shown as `<i><red>not configured</red></i>`

   **Note:** The canonical provider for commands is resolved independently from the canonical provider for skills. A user might designate Claude as canonical for skills but OpenCode as canonical for commands, depending on their workflow.

2. Defined Commands

   Within the Defined Commands area we have three distinct ways of displaying this content:

      - **Detail View**
          - Shown when there is exactly 1 command being shown (typically due to a filter condition)
          - Whether the `-v` / `--verbose` flag was used has no effect
          - The first line of reporting on the command is the command name without a leading `/` (bold, OSC8 link to the command file) followed by the badge for the scope
          - The second line is the description of the command (dim, italics) with word wrapping; if no description exists, show `no description`
          - Then a blank line
          - Since commands are single files (not directory bundles), there is **no `FileSystem` tree**. Instead, show the command's **frontmatter properties** as a dim key-value list with a left margin of 2 characters:
              - Each recognized frontmatter property is rendered as `<dim>{key}: {value}</dim>`
              - Only properties that are actually present in the file are shown
              - The `model` property, if present, should additionally be flagged: `<dim>model: {value}</dim> <red><i>(not shareable)</i></red>` — see [Non-Portable Assets](non-portable-assets.md)
          - After the frontmatter summary, show the file size in bytes and the format type: `<dim>({size} bytes, {format})</dim>`

      - **Verbose**
          - If the number of commands (_after filtering_) is less than 6 (and more than 1) we will report using the verbose style.
          - If the user adds the `--verbose` or `-v` flag and there is more than 1 command then we will also report using the verbose style.
          - This mode lists all commands available (after filter) as an unordered list (leveraging `UnorderedList` component from biscuit-terminal)
              - Each item shows: OSC8-linked command name without leading `/` (bold), scope badge, then description (dim, italics)

      - **Normal**
          - When we have more than 5 commands (and verbose is not forced) we group commands by scope
          - Grouping uses `BTreeMap<CommandScope, Vec<&CommandInfo>>` so scopes appear in order: User, RepoMasked, Repo
          - Each scope section leads with the scope badge and a count: `{badge} <dim>(<i>{count}</i>)</dim>`
          - Followed by a blank line
          - Then command names (without leading `/`) rendered as space-separated OSC8 links (bold) with `WordWrap::BespokeProse(Some(50), ...)` to flow across the terminal width
          - Followed by another blank line

   **Note on command name display:** Command names are shown without a leading `/` character. Users understand they must press `/` to invoke commands; including it in the listing is redundant.

3. Fix Summary

   - This section is only shown when `--apply` / `--fix` was used
   - Rendered after the Defined Commands section
   - Shows: `<b>Fix Summary</b>` header then a dim comma-separated metric line:
     `directories_created={n}, links_created={n}, already_linked={n}, skipped={n}, aliases_inserted={n}, aliases_resynced={n}, format_incompatible={n}`

   **Note on `format_incompatible`:** Unlike skills (which are predominantly Markdown across providers), commands exhibit significant format heterogeneity. When the canonical provider uses Markdown but a target provider requires TOML (Gemini) or MCP (Goose), the command cannot be linked by symlink alone. These cases are counted as `format_incompatible` rather than silently skipped.

4. Auto-Init Behavior

   - When `--apply` is used in a git repo and the repo canonical provider is not configured, claudine automatically runs `claudine init --repo` before proceeding with the fix operation.

5. Auto Fixes

   When `--apply` / `--fix` is used, claudine can automatically resolve certain exceptions without human intervention. The following fixes are applied:

   **Symlink fixes:**

   - **Create missing directories**: If a target provider's command directory does not exist (but its parent does), create it. Tracked as `directories_created`.
   - **Create missing symlinks**: For each canonical command that has no corresponding entry in a compatible target provider's directory, create a symlink pointing back to the canonical source. Tracked as `links_created`.

   **Property aliasing (enrich canonical, keep symlinks):**

   CLIs ignore frontmatter properties they don't recognize. Rather than creating derived copies, claudine enriches the canonical source file with alias properties so that symlinks continue to work for all providers. This is the same strategy used for agents — see `agents.md` for the full rationale.

   Commands have relatively uniform frontmatter across providers, so aliasing is simpler than for agents. The main aliases:

   | Canonical Property | Alias Property | Target Provider | Notes                                      |
   |--------------------|----------------|-----------------|---------------------------------------------|
   | `name`             | (from filename) | All            | If `name` is missing, insert it from the filename. Not a cross-provider alias but a completeness fix. |
   | `description`      | `description`  | All             | Universal property — no aliasing needed      |
   | `argument-hint`    | `argument-hint`| Claude, Codex   | Same name across all providers that support it — passes through symlinks naturally |

   **Drift detection — `VariantLinkedProperty` exception:**

   If alias properties diverge from their canonical value (e.g., someone edits one but not the other), claudine reports a `VariantLinkedProperty` exception. The `--apply` flag re-syncs by copying the canonical property's value to its aliases.

   **Property passthrough:**

   Many properties — `argument-hint`, `user-invocable`, `disable-model-invocation`, `allowed-tools`, `template`, `subtask`, `agent`, `context`, etc. — are only recognized by some CLIs. Under the same simplifying assumption (extra properties cause no downside to CLIs that don't use them), these values can live in the canonical command file and pass through symlinks harmlessly. CLIs that understand a given property will use it; those that don't will ignore it. For the full analysis of which properties are safe to pass through and which block sharing, see [Non-Portable Assets](non-portable-assets.md).

   When the `--verbose` flag is used, the Footer Messages section includes per-property notes showing which CLIs actually consume each property present in the listed commands (see section 7).

   **Fixes that cannot be automated:**

   - **Format conversion** (Markdown → TOML for Gemini, Markdown → MCP for Goose): These require fundamentally different file structures. Tracked as `format_incompatible` and left for human resolution.
   - **`model` property translation**: Model values are provider-specific. Automated translation is planned for Phase 2 (see `model-property-design.md`). In Phase 1, commands with `model` are flagged as `ModelPropertyNotShareable`. See [Non-Portable Assets](non-portable-assets.md) for the full portability analysis.
   - **`prompt` property for Gemini**: Gemini commands require a `prompt` field in TOML format. This is a structural difference, not a property alias — it cannot be derived from Markdown frontmatter alone.
   - **Built-in only providers** (KimiCode): No custom command files are supported, so there is nothing to fix.

6. Exceptions

   - This area is only shown if there **are** exceptions (either `CommandException` entries or `CommandDiagnostic` entries)
   - The exceptions section does NOT include explicit `--fix` callouts; the Footer Messages section handles that
   - Exceptions are rendered in two groups: **format-incompatible providers** and **regular providers**

   **Format-incompatible providers** are rendered as simple one-liners in an unordered list, with no sub-bullets:
   - `<b>{provider}</b> — ❌ uses a non-standard format which is incompatible with Claudine.`
   - Examples: Gemini (TOML), Goose (MCP), Kimi Code (built-in)

   **Regular providers** are grouped by provider with a header line showing provider name, user command path, and repo command path:
   - `<b>{provider} [ user:</b> ~/{user_path}<b>, repo:</b> <magenta>{repo_path}</magenta> ]`
   - For providers where the command directory name differs from `commands/` (e.g., Codex uses `prompts/`), show the actual path

   **Directory-level diagnostics** (missing command directories) are rendered directly at the provider level, NOT nested under a "missing" category:
   - `All <b><yellow>{count}</yellow> {scope}</b> scoped commands are missing for <b>{provider}</b> because the directory for commands doesn't exist!`
   - These appear as immediate children of the provider item in the unordered list

   Within each regular provider, exceptions are further grouped by `ExceptionType`:

     - **Missing**: Comma-separated list of missing command names with word wrapping (directory-level diagnostics are shown separately above, not nested here)
     - **Invalid**: Each command shown individually as an OSC8 link with missing property details: `<b>{command}</b> (<i>missing the properties <red>{prop1}</red>, <red>{prop2}</red></i>)`
         - For Gemini commands, the required `prompt` property is validated
         - For other Markdown-based providers, validation checks their respective optional/required property sets
     - **NoLinks**: Comma-separated OSC8-linked command names with word wrapping
     - **VariantLinkedProperty**: Alias properties that have diverged from their canonical value. Auto-fixable with `--apply`.
     - **ModelPropertyNotShareable**: Commands that specify a `model` property are flagged here because model values are provider-specific and cannot be meaningfully shared across CLIs
         - Each entry shows: `<a href="{path}"><b>{command}</b></a> (<i>specifies <orange>model = {model_value}</orange></i>)`
         - Command names are OSC8 links to the command file
         - This is a **warning**, not a blocking error -- the command is still linked but the model property may cause issues in non-originating providers

   - Exceptions use the same _filtering_ rules as the Defined Commands section so we should ONLY report on those commands which match the fuzzy matching of the filter globs passed in
   - When filters are active, diagnostics are cleared entirely

   **Note on BrokenLink:** The `BrokenLink` exception type from skills is largely irrelevant for commands since commands are single files that do not contain inter-file markdown links. This exception type is retained in the enum for structural consistency but is not expected to surface in practice.

7. Footer Messages

   This section is optionally rendered, it depends on whether the current _state_ dictates that additional context should be provided to the user. The following are messages that _might_ be shown (including an explanation of when they should be):

   - **fix**
       - the message `<dim><i>use <red>--fix</red> to attempt to fix the reported issues</i></dim>`
       - only shown when there are exceptions being reported on AND `--apply` was NOT used
   - **user only**
       - the message `<dim><i>the current working directory is <b>not</b> a <b>git</b> repo so we are only showing user-based scope</i></dim>`
       - only shown when the CWD is not inside a git repo
   - **verbose**
       - the message `<dim><i>using the <green>--verbose</green> switch will provide not only command names but also descriptions</i></dim>`
       - only shown when there are more than 10 commands listed and the user has not used the `--verbose`/`-v` flag
   - **filtering**
       - the message `<dim><i>using parameters in the CLI call will act as <b>filters</b> to help reduce the commands to only those you are interested in</i></dim>`
       - only shown when no filter parameters were provided
   - **format warning**
       - the message `<dim><i>some providers use non-Markdown formats for commands (Gemini: TOML, Goose: MCP) -- these require manual configuration</i></dim>`
       - only shown when `--apply` was used AND at least one `format_incompatible` count was non-zero in the fix summary
   - **property passthrough notes** (verbose only)
       - only shown when the `-v` / `--verbose` flag is used AND at least one command in the listing has properties that are only consumed by a subset of CLIs
       - each such property that appears across the listed commands gets its own line indicating which CLIs consume it, e.g.:
           - `<b><yellow>argument-hint</yellow></b><dim> used by Claude and Codex; other CLI Agents will ignore</dim>`
           - `<b><yellow>template</yellow></b><dim> used by OpenCode; other CLI Agents will ignore</dim>`
           - `<b><yellow>subtask</yellow></b><dim> used by OpenCode; other CLI Agents will ignore</dim>`
       - only the properties that actually appear in the currently listed commands are shown

	If only a single message is to be displayed then it should just be displayed "as is" (indented with a leading space) with a leading blank line to separate it from the sections above.

	If _more_ than one message is to be displayed then the messages should be added to an `UnorderedList` struct. The leading blank line should be added in this use-case too.

## Provider Command Capabilities Reference

The following table summarizes how each provider handles commands, derived from the `capabilities_for()` definitions in `claudine/lib/src/linking/capabilities.rs`:

| Provider  | Support Level  | Format   | Repo Path              | User Path                     | Required Props | Optional Props                                                                                       |
|-----------|---------------|----------|------------------------|-------------------------------|----------------|------------------------------------------------------------------------------------------------------|
| Claude    | Full          | Markdown | `.claude/commands`     | `.claude/commands`            | (none)         | name, description, argument-hint, disable-model-invocation, user-invocable, allowed-tools, model, context, agent, hooks |
| Codex     | CustomFormat  | Markdown | (none)                 | `.codex/prompts`              | (none)         | description, argument-hint                                                                           |
| Gemini    | CustomFormat  | TOML     | `.gemini/commands`     | `.gemini/commands`            | prompt         | description                                                                                          |
| Goose     | CustomFormat  | MCP      | (none)                 | (none)                        | N/A            | N/A                                                                                                  |
| KimiCode  | Limited       | Built-in | N/A                    | N/A                           | N/A            | N/A                                                                                                  |
| OpenCode  | Full          | Markdown | `.opencode/commands`   | `.config/opencode/commands`   | (none)         | description, template, agent, model, subtask                                                         |
| Qwen      | Full          | Markdown | `.qwen/commands`       | `.qwen/commands`              | (none)         | description                                                                                          |

### Linking Implications

- **Full + Markdown** providers (Claude, OpenCode, Qwen): Commands can be symlinked directly from the canonical provider. Frontmatter properties not recognized by the target provider are silently ignored.
- **CustomFormat + Markdown** (Codex): User-scoped symlinks work since the format is Markdown, but Codex uses a `prompts/` directory instead of `commands/` and does not currently document repo-scoped prompt discovery.
- **CustomFormat + TOML** (Gemini): Cannot be symlinked from a Markdown-based canonical provider. Requires format conversion or manual creation. Counted as `format_incompatible` in the fix summary.
- **CustomFormat + MCP** (Goose): Commands are MCP-based, not file-based. Cannot participate in file-based linking. Counted as `format_incompatible` and noted in the provider exception header.
- **Limited / Built-in** (KimiCode): No custom command support. Skipped entirely during linking. Not counted as `format_incompatible` (there is nothing to link to).

## Key Differences from Skills Reporting

| Aspect                    | Skills                                          | Commands                                        |
|---------------------------|------------------------------------------------|------------------------------------------------|
| Resource unit             | Directory bundle with `SKILL.md` entry point    | Single file (e.g., `commit.md`)                |
| Detail View content       | `FileSystem` tree with tokens metric            | Frontmatter properties + file size/format      |
| Format heterogeneity      | Predominantly Markdown across all providers     | Markdown, TOML (Gemini), MCP (Goose), Built-in (KimiCode) |
| Fix summary metrics       | `directories_created`, `links_created`, etc.    | `files_created`, `links_created`, `format_incompatible`, etc. |
| Additional exception type | N/A                                             | `ModelPropertyNotShareable`                     |
| BrokenLink relevance      | Common (inter-file links within skill bundles)  | Rare (single files, no inter-file references)   |
| Codex path quirk          | `skills/` directory                             | `prompts/` directory (not `commands/`)          |
| Canonical provider key    | `LinkableResource::Skill`                       | `LinkableResource::Command`                     |
