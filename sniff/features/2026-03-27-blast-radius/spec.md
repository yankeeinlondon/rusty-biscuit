# Blast Radius

In this feature we are going to provide additional features in the Sniff library and CLI to help with working with "blast radiuses".

> **IMPORTANT:** always use [CLI Best Practices](@prompts/cli-best-practices) when implementing the functionality in this feature

## `sniff repo` subcommands

We are going to add a few additional subcommands to `sniff repo`:

1. `sniff repo dirty-source-code <filter>`

    - Functionality:
        - returns all the source code files which have changes in them since the last commit (staged, unstaged, and untracked)
        - Returned structurally as a file per line:
            - if the `--list` switch is used then will render as an unordered list (using `- ` as the list item prefix)
            - if the `--csv` switch is used then will render as a comma separated list of files
        - All files are rendered for Terminal output:
            - Files are OSC8 links
            - The displayed text is a relative path (from repo root)
            - The linked file is an absolute path (as is required by OSC8)
            - The displayed filepath will dim all path segments up to the final segment and bold face the last segment
            - If the optional `--no-path` CLI switch is used then:
                - only the last segment of the filepath will be shown
                - still an OSC8 link
                - no dim or bold styling needed

            > **Note:** this command exposes the global `--plain` switch which will strip all terminal based escape codes

        - if NO files are returned then the call will return with an error exit code (with no output to STDOUT or STDERR)
            - if the user includes the `--no-error` flag then a lack of any results will return the 0/ok exit code
            - if the user includes the `--on-error <msg>` flag then the message provided will be passed through the `Prose` struct (to allow for formatting) and rendered to STDERR
                - the `--on-error` flag can be used with the `--no-error` and if BOTH are provided then the message will be returned to STDOUT instead of STDERR
    - Parameter based filtering:
        - optionally allows 1 or more _filters_ as parameters
        - filters will reduce the returned source code files to only those which include the filter in their filepath
        - if more than 1 filter, the filters are logically OR'ed together
    - `--package <pkg>` filtering:
        - another way to filter down results is explicitly by the `package` in a monorepo (this subcommand should provide a meaningful/useful error message if used in a non monorepo)
            - when the `--package <pkg>` filter is used _with_ regular filter parameters:
            - the source code files are FIRST reduced to the specified package, then parameter based filters are applied
            - make sure that _shell completions_ correctly allows the valid set of package names in the repo
    - `--package-area <area>` filtering:
        - similar to the `--package` CLI switch, this switch will filter down to the specified package area
        - make sure that _shell completions_ correctly allows the valid set of package area names in the repo

2. `sniff repo staged-source-code <filter>`

    - exactly the same as `dirty-source-code` except that only files which have been _staged_ for commit are shown

3. `sniff repo unstaged-source-code <filter>`

    - exactly the same as `dirty-source-code` except that only source code files which are NOT _staged_ for commit are shown

4. `sniff repo dirty-files <filter>`

   - exactly the same as `dirty-source-code` except that:
       - all files (not just source code files) are displayed

5. `sniff repo staged-files <filter>`

   - exactly the same as `dirty-source-code` except that:
       - all files (not just source code files) are displayed
       - only files which have been _staged_ for commit are shown

## `sniff docs` updates

1. We will add a new `--blast-radius` switch to the `sniff docs` subcommand:

   - when this flag is provided we will only show documents which have their "blast_radius" property in frontmatter set

2. Fix STDOUT/STDERR:

   - the current implementation of this command makes the mistake of reporting everything to STDOUT!
       - The Heading section which reports `Docs (# documents)` should be sent to STDERR along with the blanks rows providing vertical padding
       - The footer message `Use --verbose / -v to include title and last updated` should also be send to STDERR along with vertical padding

3. Footer Message

   - Change the footer text which says `Use --verbose / -v to include title and last updated` (when the --verbose flag is not present) and change it to `Use --verbose / -v to include metadata for documents`

4. Verbose Output

   - When the user chooses the `--verbose`/`-v` flag we will change how we render this
   - Instead of adding metadata to the right of the filename we will place it below as a Nested List
   - Nested List Items include:
       - `<b>title:</b> {title} (<dim><i>from {source}</i></dim>)`, where `source` is "title property", "H1 heading", "<yellow>none</yellow>" (match in this order)
       - `<b>updated:</b> {title} (<dim><i>from {source}</i></dim>)`, where `source` is "updated property" or "file metadata" (match in this order)
       - `<b>frontmatter properties:</b> <i>{props}</i>`, where `props` is a comma separated list of frontmatter properties which have been set

## `sniff blast-radius <scope: staged | dirty | last-commit>` command

The new "blast-radius" command is used to identify which documents may need to be updated based on the detected source code changes.

- if the `scope` parameter is not provided the command will default to "dirty"
- both `--package <pkg>` and `--package-area <area>` can be used in monorepos to isolate to a package or package area respectfully
    - make sure that _shell completions_ correctly allows the valid set of package and package area names for these switches
- the documents identified as candidates for update are:
    - documents which have a `blast_radius` frontmatter property set
    - the `blast_radius` parameter is a YAML list (representing files)
    - the _set_ of source code files "in scope" (e.g., staged, dirty, part of last commit) is listed in the document's `blast_radius`
- output is a list of filepaths, one per line:
    - if the `--list` switch is used then will render as an unordered list (using `- ` as the list item prefix)
    - if the `--csv` switch is used then will render as a comma separated list of files
- All files are rendered for Terminal output:
    - Files are OSC8 links
    - The displayed text is a relative path (from repo root)
    - The linked file is an absolute path (as is required by OSC8)
    - The displayed filepath will dim all path segments up to the final segment and bold face the last segment
    - If the optional `--no-path` CLI switch is used then:
        - only the last segment of the filepath will be shown
        - still an OSC8 link
        - no dim or bold styling needed

    > **Note:** this command exposes the global `--plain` switch which will strip all terminal based escape codes

- if NO files are returned then the call will return with an error exit code (with no output to STDOUT or STDERR)
    - if the user includes the `--no-error` flag then a lack of any results will return the 0/ok exit code
    - if the user includes the `--on-error <msg>` flag then the message provided will be passed through the `Prose` struct (to allow for formatting) and rendered to STDERR
        - the `--on-error` flag can be used with the `--no-error` and if BOTH are provided then the message will be returned to STDOUT instead of STDERR


