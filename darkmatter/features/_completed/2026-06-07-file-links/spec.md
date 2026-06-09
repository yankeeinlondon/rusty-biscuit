# File Links Directive

In Darkmatter we already provide the [`::toc-linking`](@darkmatter/docs/inline/toc-linking.md) directive which allows an author to point at a file and instead of just a normal hyperlink to the external file, this directive determines the table of contents for the file reference and brings in a structured tree of links to each item in the TOC.

In this feature we'll add a _related feature_ called the **File Links Directive** and syntactically represented with `::file-links`

## Syntax

The base syntax is:

> `::file-links <glob>`

- the _glob pattern_ is evaluated to create a set of files in a file tree

    The set of files which the glob pattern produces will be filtered down to only files which are seen as valid hyperlink
    targets:
    - Markdown files (`.md`)
    - Other document or text formats (`.txt`, `.doc`, `.docx`, `.xls`, `.xlsx`, `.pdf`)

    Note: image files, audio files, or any binary files will **not** be included. If no files match the pattern, the directive will render a subtle "No matching files" warning or be omitted depending on the environment's strictness settings.

- if a user prefers to just specify a particular directory then they can do that with `--dir <path>` which would replace
  the glob pattern requirement
    - when the user opts for the `--dir <path>` approach another option is available `--depth <#>` which indicates what depth
      of subdirectories should be recursed into. By default this is set to `0` (meaning that it will NOT recurse into subdirectories)

## Reporting

Regardless of whether the user uses a glob based pattern or a directory, we expect all files to be part of a common directory
tree. For that reason the default rendering style is to use the directory rendering that the [`FileSystem`](@biscuit-terminal/docs/components/file_system.md) component from biscuit-terminal provides.

### Component Configuration

The directive will configure the `FileSystem` component with the following:
- `.with_file_links()`: Enable OSC8 hyperlinks for all rendered files.
- `.italicize_dot_files(true)` and `.dim_gitignore(true)`: Follow standard Darkmatter styling.
- `.show_root(true)`: The root line will be rendered using a new **Dimmed Prefix** style (see below).

### Path Resolution and Boundaries

- **Resolution**: Paths provided to `<glob>` or `--dir` are resolved relative to the document containing the directive.
- **Self-Exclusion**: When a file references its own directory (e.g., `::file-links --dir .`), the rendered tree will **not** include a link to the current file itself to avoid recursive or redundant navigation.
- **Repository Boundary**: The `::file-links` directive is restricted to files within the current repository or under the current working directory (if not in a repo). Any paths resolving outside this boundary will be ignored for security and path-logical consistency.
- **Base Directory Rendering**: The root of the tree should represent the path from the repo's root (or CWD) to the target directory. 
    - The path segment *leading up* to the target should be dimmed.
    - The *target directory* name itself should be highlighted.
    - The `repo-icon` should be used if the path starts at the repo root.

Example:
If the files we want to represent are in the `docs/topics` folder of a repo:
1. We would use the directive `::file-links --dir docs/topics`
2. The `FileSystem` root line would render as:
   ` <dim>{repo-icon}/docs/</dim>topics`

### Required Component Enhancements

To support this directive, the following enhancements are required in `biscuit-terminal`:
1. **Icon Mapping**: Add Nerd Font and Unicode mappings for `.pdf`, `.doc`, `.docx`, `.xls`, `.xlsx`, and `.txt` to `icons.rs`.
2. **Dimmed Root Styling**: Add a method to `FileSystem` (e.g., `.with_dimmed_root_prefix(prefix: &str)`) that allows rendering the root directory line with a dimmed prefix string.
3. **Extension Filtering**: Ensure the filtering logic can strictly include only the allowed document extensions, ignoring case.

## Operation Position in Pipeline

The `::file-links` directive operates during the **Transclusion** phase of the Darkmatter compose pipeline. This allows it to run concurrently with other file-system-hitting directives like `::file`, `::code`, and `::toc-linking`.

## Documentation

- **FileSystem Component**: All enhancements made to support this directive (such as improved icon mapping or the dimmed path prefix) must be documented in the [FileSystem](@biscuit-terminal/docs/components/file_system.md) component docs.
- **Darkmatter Directive**: A new entry for `::file-links` should be added to the Darkmatter directive reference, explaining the syntax, filtering rules, and the repository boundary constraint.
