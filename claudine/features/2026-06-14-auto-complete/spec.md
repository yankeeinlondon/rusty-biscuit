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

### `inline-compose` operation


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
                    - 
                - all files inside a _directory_ starting with `_` are ignored

                    > **Note:** all files within a _directory_ starting with `_` are ignored but files with a leading `_` will still match

            - the default glob pattern is used unless the document's schema not only expresses a property to be of the type `file` but also adds the a glob pattern for this file reference:

                ```yaml
                $schema: 
                    uses_default_glob: file
                    just_images: file(match('*.gif','*.jpg','*.png'))
                    spec_files: file(match('**/{features|fixes}/*spec.md'))
                ```



## Autocomplete
