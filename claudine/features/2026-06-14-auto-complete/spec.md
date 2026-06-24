---
clarified: "claude/claude-opus-4-8"
---

# Shell Completions and Autocomplete Feature

## Headline Features

In this feature we will add two headline features:

1. **Shell Completions**
2. **Autocomplete**

Claudine already has _shell completions_ but we will optimize it substantially.

### Shell Extensions

Shell completions are extremely important for Claudine:

1. the allow a user to explore the API surface and not always be referring to documentation to understand what is available
2. when we run composition features of Claudine (compose, inline-compose, sequence) we refer to a Markdown document (or sometimes a YAML file) and we need to help the user resolve this file so that no accidental spelling mistakes creep in but also so that kicking off a job can be done as quickly as possible

### Autocomplete

We already try to help the user by providing an interactive dialog when a caller has not provided a required frontmatter property (per the schema). But in addition we will now:

- when a user passes in the value for a frontmatter property who's type is `file` we will first try to resolve it with `FileReference` struct but if that fails we will offer the user a select dialog of files we think they meant

## Shell Completions

### `compose` operation

When a user types `claudine compose ` they are now ready to specify the Markdown file that they will be using as a prompt.

- by default the glob pattern which is used to identify possible completions for the file is:
    - in a monorepo:
        - `{package-root}/prompts/**/*.md}`
        - `{package-area-root}/prompts/**/*.md}`
    - `{repo-root}/prompts/**/*.md}`
    - `{repo-root}/.claudine/prompts/**/*.md`
    - `~/.claudine/prompts/**/*.md`
    - all gitignore files will always be excluded
    - all files inside of a `node_modules` or `target` directory will be excluded
    - all files or files in a directory starting with `_` will be ignored
    - this glob pattern can be replaced by putting a replacement glob pattern into the repo's configuration at: `files.prompts.default_glob` in config file.
        - **Note:** this `files.prompts.default_glob` override key does NOT yet exist in `ClaudineConfig` and must be ADDED as part of this feature.
- because we support magic paths (e.g., those with a leading '@') it means that a single path reference by the user could possibly reference more than one valid completion ... however, the completion which is "closest" will always be the one which is resolved. This precedence is supported out of the box by `FileReference` but the main point in shell completions is to complete a "magic path" which is valid not a full file path:
    - if the user typed `claudine compose @plan` and pressed tab
    - assuming that both `~/.claudine/prompts/plan.md` and `{repo-root}/prompts/plan.md` existed
    - shell completions would recognize that the user wants to use the magic path syntax and that `@prompts/plan.md` is a valid completion
    - once the user presses ENTER and the claudine process kicks off it will do the final resolution back to the single file

### `sequence` operation

The `claudine sequence` command completion should 100% mimic that of `claudine compose` except that the glob pattern for files would include not just Markdown files but also YAML files which at the root level define: `kind: sequence`.

> **Note:** the internal format/schema of a YAML sequence file (its body/step definitions and what `$schema` means inside it) is defined by the existing `sequence` command and is OUT OF SCOPE for this feature. This feature only reads its top-level `name`/`description`/`$schema` keys to populate the detail block.

### `inline-compose` operation

The `claudine inline-compose <file>` operation is similar but not the same as `compose` and `sequence`:

- where as `compose` and `sequence` look for prompt files in a set of folder locations, 
- by comparison `inline-compose` has no associated set of folders to look in; instead it:
    - looks for Markdown files in the directory tree starting with `${CWD}` and keeps only those which define one of the following Frontmatter properties:
        - `prompt` (Note: this is today the way that a inline-compose file operates ... by using the `prompt` property as the prompt passed to the agentic CLI)
        - `sections` (Note: this references a future capability more so than a current one)
    - Note: unlike the other commands, the glob scoping is ALWAYS based on CWD and doesn't consider repo root, package roots, etc.


### Frontmatter Completion in `compose` / `inline-compose` / `sequence`

> **What's new:** schema-aware `property=<TAB>` file completion (driven by `file(match(...))`) is ALREADY IMPLEMENTED in the completion engine. This feature does NOT re-spec or rebuild that engine. What is NEW here is: (a) the bare-`file`/`file[]` → default-glob fallback (see below), and (b) the array `,`-continuation completion (autocomplete a single file, then a trailing `,` re-opens the glob excluding files already selected). The `,`-continuation is purely a SHELL-COMPLETION (TAB) behavior — the "already selected" set is parsed from the already-typed comma-list on the command line. The INTERACTIVE (ENTER) path never uses commas: a `file[]`-typed property is resolved via the `ChooseMany` multi-select chooser in one pass. The remainder of this section is background for those two additions.

- once a caller has typed `claudine {compose|inline-compose|sequence} <file> ` the remainder of the things that go into the call will be a combination of:
    - CLI Switches
        - all CLI switches start with the typical `--{switch}`/`-{short-name}` syntax
        - I believe today we do a good job of providing autocomplete for these CLI switches
    - Frontmatter Parameters
        - A user can use the syntax `{prop}={value}` to set Frontmatter properties in the referenced prompt file
        - We need to know what properties are defined in the referenced document's `$schema` property (which uses `SimpleSchema` to define types)
        - `number`, and `boolean` types will just autocomplete the `{prop}=` portion
        - `string` types are similar but include a quote mark: `{prop}="`
        - however, when the type of a frontmatter property is `file` we can offer a lot more help:
            - by default the glob pattern which will be used for identifying the possible file targets is: 
                - all Markdown files in the repo
                    - if not in a repo then all markdown files in the directory tree rooted in ${CWD}
                - all `.gitignore` files are excluded
                - all files in the "prompt" directories are ignored:
                    - it assumed these are associated more with `compose` than `inline-compose`
                - all files inside a _directory_ starting with `_` are ignored

                    > **Note:** all files within a _directory_ starting with `_` are ignored but files with a leading `_` will still match

            - the default glob pattern is used unless the document's schema not only expresses a property to be of the type `file` but also adds a glob pattern for this file reference. Property values use the authoritative inline-string `SimplifiedSchema` form (each value is a quoted string):

                ```yaml
                $schema:
                    uses_default_glob: "file"
                    just_images: "file(match('*.gif','*.jpg','*.png'))"
                    spec_files: "file(match('**/{features,fixes}/*spec.md'))"
                    attachments: "file[]"
                ```

            - a **bare** `file` or `file[]` property (one with no `match(...)`) falls back to the section's DEFAULT glob (the "all Markdown files in the repo" default described above). This is a deliberate behavior choice for autocomplete/this feature.

                > **Note:** the existing TAB-completion engine today emits zero candidates for an empty pattern; under this spec a bare `file`/`file[]` resolves to the default glob instead.
> **Note:** this `,`-continuation is TAB-only: any `file[]` array type will TAB-autocomplete a single file, and if you then type a `,` TAB will again autocomplete the same glob excluding files already named in the typed comma-list. The interactive (ENTER) path does NOT parse comma-lists — `ChooseMany` owns interactive multi-file selection in a single pass.

> **Note:** the comma-list is split on top-level `,` with surrounding whitespace trimmed; filenames are shell-quoted as usual (so spaces are handled by the shell). A literal comma inside a filename is an unsupported edge case for the exclusion set.


## Autocomplete

- shell completions will help a user tab-complete an incomplete value into a fully complete value but a nice companion to that is allowing a user to type in the incomplete value and have claudine then figure out what the user meant
- the hints that exist for shell completions will help us here too

### `compose` | `inline-compose` | `sequence` operation file

- if a user says `claudine <compose|inline-compose|sequence> plan` and presses enter we know that "plan" is a reference to a file and so we use glob patterns to find it:
- the glob pattern identifies _possible_ matches
    - we use the same glob patterns for `compose`/`sequence` and `inline-compose` here as we do for shell completions
    - the autocomplete (ENTER) path **reuses the shipped bounded completion walker** — same scope-priority order, same `.gitignore` exclusion, same `_`-prefix-directory and `node_modules`/`target` exclusions, one `sniff` call per invocation, the `MAX_CANDIDATES = 500` cap, and the existing ~100ms p95 budget — **verbatim except that the `*query*` substring filter is evaluated inside the walk** (pushed into the walker as a predicate) so the cap counts query-matching files, not raw discoveries. The autocomplete path therefore inherits every exclusion rule stated for the shell-completion sections above.
- the `*plan*` substring filter is applied DURING the walk (the walker only retains files whose filename/filepath contains `*plan*`), so the `MAX_CANDIDATES = 500` cap counts only files that MATCH the query
- when there is a **singular** match we should:
    - present a **confirmation dialog**:
        - **badge** and **name** 
            - the "badge" indicates to the caller whether the file is a "Compose", "Inline Compose", or "Sequence" operation (it should use background colors to separate it from the rest of the page ... similar to in style as the badges which Claudine already displays as part of it's "execution line" when starting up)
            - the badge and name reside on the same line and are separated by a single space
            - target document has not defined the `name` Frontmatter 
                - the filename with path (as an OSC8 link so that user can click through to see the referenced file)
            - target _has_ defined the `name` frontmatter
                - the name property's value is displayed in bold with the filepath in parenthesis in dim and blue text (the path is a OSC8 link)
        - **description**
            - the content is derived from the `description` Frontmatter property when available but defaults to "no description" if not defined
            - it will be vertically below the **name** but with a blank line in between
            - the text in the description should use BlockQuote formatting with a grey left vertical bar
        - **schema**
            - describe the `$schema` of the document if defined
            - if not defined then simply:
                - `**Schema:**
                - (blank line)
                - `- <dim><i>no schema defined</i></dim>`
        - **confirmation**
            - Finally add a confirmation dialog: 
                - `Use this file? (Y/n)`
        - **YAML sequence files.** A `sequence` candidate may be a YAML file declaring `kind: sequence` at the root rather than a Markdown file. For such YAML files this badge/name/description/schema block is populated IDENTICALLY to the Markdown path: the top-level `name`, `description`, and `$schema` keys are read exactly as the Markdown path reads frontmatter, with the same fallbacks ("no description", "no schema defined"), and the badge is "Sequence". Conceptually, a YAML sequence file is treated as frontmatter without a Markdown body.
- when there is more than one match we should present a two-pane chooser. This is a **separate presentation** from the single-match confirmation dialog above (which shows no chooser).
    - **chooser is type-driven:** the file argument for `compose`/`inline-compose`/`sequence` is a SINGLE file, so it uses the `ChooseOne` single-select component from `biscuit-tui`. (`ChooseMany` is reserved for `file[]`-typed frontmatter properties — see _Frontmatter Properties_ below — not for this single-file argument.)
    - the user can move up and down through the list of possible matches
    - the two-pane layout uses biscuit-tui's `SplitPane` with `SplitDirection::Auto`:
        - `SplitDirection::Auto` resolves to **Horizontal** (detail pane to the _right_ of the list) when the terminal is wider than it is tall
        - and to **Vertical** (detail pane _above_ the list) when the terminal is taller than it is wide
        - biscuit-tui owns the geometry; claudine drives the two independent render calls (one for the list, one for the detail pane)
    - the two panes are:
        - the list of possible files (the `ChooseOne` chooser)
        - a detail/info pane DERIVED from the currently **highlighted (active)** list item, recomputed each frame via `ChooseOne::active_option()`
            - its content is the same badge/name/description/schema block used by the single-match confirmation dialog, rendered as rich terminal text bridged into the pane (Prose → ansi-to-tui → ratatui `Paragraph`)
- if there are NO matches on the text the caller passed in (e.g., "plan" in this example) then we should immediately return with an error
- if the count of query-matching files **exceeds the `MAX_CANDIDATES = 500` cap**, autocomplete does NOT silently truncate the interactive list — it returns an error like `too many matches for '<query>', narrow your query`. Because the substring filter is evaluated inside the walk, the cap counts matching files, so this error fires on the true query-match count and is reachable (it would be unreachable if the cap truncated raw discovery before filtering)

    > **Note:** the walk early-aborts once the query-match count exceeds the cap, so the error reports "more than 500" rather than an exact total. This is chosen to preserve the latency budget.

> **Note:** autocomplete is only available with the terminal is TTY; if it is not then autocomplete will not be active and an incomplete file reference will result in an error

The autocomplete (ENTER) path therefore has three distinct failure modes: **NO matches → error**, **more than the cap → visible "narrow your query" error** (never a silent truncation), and **non-TTY → error**.

### Frontmatter Properties

- we already use TUI widgets to ask for _required_ properties in the schema of a page when the caller didn't provide them as part of the CLI invocation
- however, we must improve on this:
    - we currently consume the entire screen with the dialog for each missing Frontmatter property
    - we should instead only use _enough_ space for what we need to get the user's input
    - however, we will need more space than we currently do because we want to tell the user what we are expecting:
        - **string** type: 'The <b>{property}</b> _requires_ a <inverse>string</inverse> value; please input a value to continue:'
            - if there are min/max constraints then we should reinforce
        - **number** type: 'The <b>{property}</b> _requires_ a <inverse>numeric</inverse> value; please input a value to continue:'
            - if there are min/max constraints then we should reinforce
        - **boolean** type: 'The <b>{property}</b> _requires_ a <inverse>boolean</inverse> value:'
        - **file** type: 'The <b>{property}</b> _requires_ a valid <i>file reference</i>; choose from the files below:'
            - the chooser is **type-driven**: a `file` property uses `ChooseOne` (single-select); a `file[]` property uses `ChooseMany` (multi-select)
            - when more than one candidate is shown, this reuses the same `SplitPane`/`SplitDirection::Auto` two-pane chooser+detail layout described in the autocomplete _operation file_ section above; the detail pane is derived from the highlighted item via `ChooseOne::active_option()` (single-select) or `ChooseMany::hover()` (multi-select), recomputed each frame
    - after the intro statement we provide the form input control
    - after the form input, if the `$schema` property has defined a description then we will display the description of the property below the input in dim italics

## Acceptance Criteria / Definition of Done

- **Shared bounded walker.** Shell completion and autocomplete share a single bounded walker and the documented exclusion rules (scope-priority order, `.gitignore`, `_`-prefix directories, `node_modules`/`target`, one `sniff` call per invocation, `MAX_CANDIDATES = 500`). Verifiable by asserting both paths route through the same walker entry point. The autocomplete path additionally pushes the `*query*` substring filter into the walk so the cap counts query-matching files, not raw discoveries.
- **Latency.** An autocomplete latency assertion reuses the existing `completion_perf.rs` fixture and confirms p95 stays within the same ~100ms-class budget as completion.
- **Autocomplete failure modes.** All three are observable: NO matches → error; more-than-cap matches → a visible `narrow your query` error (no silent truncation), where "more-than-cap" counts query-matching files; non-TTY → error.
- **Type-driven chooser + layout.** A `file` property/argument drives `ChooseOne` and a `file[]` property drives `ChooseMany`; the two-pane `SplitPane` `SplitDirection::Auto` layout (detail right when wider-than-tall, detail above when taller-than-wide) is verified through the L2/L3 terminal harness.
- **Bare-file fallback.** A bare `file`/`file[]` schema property (no `match(...)`) resolves to the section's default glob rather than emitting zero candidates.
- **Two presentations.** A single match shows the lightweight Prose confirmation dialog ending in `Use this file? (Y/n)` (no `SplitPane`, no chooser); multiple matches show the two-pane chooser+detail view.
- **Resolved decisions.** (a) The over-cap `narrow your query` error is reachable because the substring filter is pushed into the walk and the cap counts query-matching files, not raw discoveries. (b) Comma-continuation is TAB-only with the already-selected set parsed from the typed comma-list; the interactive `file[]` path uses `ChooseMany` in a single pass and never parses commas. (c) YAML `sequence` files (root `kind: sequence`) populate the badge/name/description/schema detail block from their top-level keys identically to Markdown frontmatter, including the same fallbacks.
