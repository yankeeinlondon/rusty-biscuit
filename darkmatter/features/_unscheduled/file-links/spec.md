# Document Links Directive

In Darkmatter we already provide the [`::toc-linking`](@darkmatter/docs/) directive which allows an author to point at a file and instead of just a normal hyperlink to the external file, this directive determines the table of contents for the file reference and brings in a structured tree of links to each item in the TOC.

In this feature we'll add a related feature called the **File Links Directive** and syntactically represented with `::file-links`

## Syntax

The base syntax is:

> `::document-links <glob>`

- the _glob pattern_ is evaluated to create a set of files in a file tree

    The set of files which the glob pattern produces will be filtered down to only files which are seen as valid hyperlink
    targets:
    - Markdown files
    - Other document or text formats (.txt, .doc, .docx, .xls, .xlsx, .pdf)

    Note: image files, audio files, or any binary files will **not** be included

- if a user prefers to just specify a particular directory then they can do that with `--dir <path>` which would replace
  the glob pattern requirement
    - when the user opts for the `--dir <path>` approach another option is available `--depth <#>` which indicates what depth
      of subdirectories should be recursed into. By default this is set to `0` (meaning that it will NOT recurse into subdirectories)

## Reporting

Regardless of whether the user uses a glob based pattern or a directory, we expect all files to be part of a common directory
tree. For that reason the default rendering style is to use the directory rendering that the [`FileSystem`](@biscuit-terminal/docs/components/file_system.md) component from biscuit-terminal provides.

- this component provides directory hierarchy for the file graph as well nice file-type icon depiction for users
- when a file references it's own directory, it will never self-reference a link to itself!
- while the files an author wants to produce hyperlinks to may be spread across multiple sub-directories it is quite common
  that the files all reside in the same directory and in fact one of the most common patterns authors like to use is
  to include use `.` as the directory for which they are pointing to.

    All three of these scenarios (files spread across multiple directories, all files in a single directory, all files in the
    current file's directory) require us to examine how we represent the "base directory":
    - the default behavior should be to represent the "base directory" as a relative path from the repo's root (or the
      caller's CWD if not in a repo)
    - Example:
        - if the files we want to represent are in the docs/topics folder of a repo then:
            1. we would use the directive `::document_links --dir docs/topics`
            2. the directive should render not as a two-deep nested tree but instead with the root folder being called:
                - `{repo}/docs/topics` when files are part of a repo

                    ```sh
                     <dim>{repo-icon}/docs/</dim>topics
                    ├── 󰍔 foo.md
                    ├── 󰍔 bar.md
                    └── 󰍔 baz.md
                    ```

                    Note:
                    - rather than showing "docs" and the visually showing "topics" as a subdirectory we will just
                      show the file path leading up to the base from the "base directory" which is the repo root
                      in this example
                    - we use a "pretty-print" style for directories at the roo

                - `{alias}/docs/topics` when files are

## Operation Position in Pipeline

## Documentation

- we need to add all changes to the FileSystem component back into the existing [FileSystem](@biscuit-terminal/docs/components/file_system.md) document
- we need to add a new Darkmatter document for this directive in
