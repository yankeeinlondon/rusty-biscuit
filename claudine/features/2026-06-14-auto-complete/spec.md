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
- because we support magic paths (e.g., those with a leading '@') it means that a single path reference by the user could possibly reference more than one valid completion ... however, the completion which is "closest" will always be the one which is resolved. This precedence is supported out of the box by `FileReference` but the main point in shell completions is to complete a "magic path" which is valid not a full file path:
    - if the user typed `claudine compose @plan` and pressed tab
    - assuming that both `~/.claudine/prompts/plan.md` and `{repo-root}/prompts/plan.md` existed
    - shell completions would recognize that the user wants to use the magic path syntax and that `@prompts/plan.md` is a valid completion
    - once the user presses ENTER and the claudine process kicks off it will do the final resolution back to the single file

### `sequence` operation

The `claudine sequence` command completion should 100% mimic that of `claudine compose` except that the glob pattern for files would include not just Markdown files but also YAML files which at the root level define: `kind: sequence`.

### `inline-compose` operation

The `claudine inline-compose <file>` operation is similar but not the same as `compose` and `sequence`:

- where as `compose` and `sequence` look for prompt files in a set of folder locations, 
- by comparison `inline-compose` has no associated set of folders to look in; instead it:
    - looks for Markdown files in the directory tree starting with `${CWD}` and keeps only those which define one of the following Frontmatter properties:
        - `prompt` (Note: this is today the way that a inline-compose file operates ... by using the `prompt` property as the prompt passed to the agentic CLI)
        - `sections` (Note: this references a future capability more so than a current one)
    - Note: unlike the other commands, the glob scoping is ALWAYS based on CWD and doesn't consider repo root, package roots, etc.


### Frontmatter Completion in `compose` / `inline-compose` / `sequence`

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

            - the default glob pattern is used unless the document's schema not only expresses a property to be of the type `file` but also adds the a glob pattern for this file reference:

                ```yaml
                $schema: 
                    uses_default_glob: file
                    just_images: file(match('*.gif','*.jpg','*.png'))
                    spec_files: file(match('**/{features|fixes}/*spec.md'))
                ```
> **Note:** any file array type will also autocomplete for a single file but if you add a `,` afterward it will again allow autocomplete for the same glob of files (excluding those already selected).


## Autocomplete

- shell completions will help a user tab-complete an incomplete value into a fully complete value but a nice companion to that is allowing a user to type in the incomplete value and have claudine then figure out what the user meant
- the hints that exist for shell completions will help us here too

### `compose` | `inline-compose` | `sequence` operation file

- if a user says `claudine <compose|inline-compose|sequence> plan` and presses enter we know that "plan" is a reference to a file and so we use glob patterns to find it:
- the glob pattern identifies _possible_ matches
    - we use the same glob patterns for `compose`/`sequence` and `inline-compose` here as we do for shell completions
- when the user passes in "plan" then we can filter the glob matches by those with `*plan*` in the filename/filepath
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
- when there is more than one match we should:
    - present a list of files that match
    - this will need to use a TUI based widget so that:
        - the user can move up and down through the list of possible matches
        - this will likely use the `ChooseMany` TUI component from `biscuit-tui`
        - however, this TUI has two main windows:
            - the list of possible files
            - the information about that file (same information as we used in the confirmation dialog)
        - how these two windows are laid out depends on the dimensions of the terminal window:
            - If the terminal is wider than it is tall, we will put the information to the _right_ of the file list
            - If the terminal is taller than it is wide then we will put the information _above_ the file list.
- if there are NO matches on the text the caller passed in (e.g., "plan" in this example) then we should immediately return with an error

> **Note:** autocomplete is only available with the terminal is TTY; if it is not then autocomplete will not be active and an incomplete file reference will result in an error

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
    - after the intro statement we provide the form input control
    - after the form input, if the `$schema` property has defined a description then we will display the description of the property below the input in dim italics
