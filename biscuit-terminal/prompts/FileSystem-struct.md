/planning:plan we are going to create a new terminal component called `FileSystem`

**IMPORTANT:** use `rust`, `biscuit-terminal`, and 

## FileSystem struct

The `FileSystem` struct will be responsible for rendering a textual representation a filesystem

- child directories will be indented
- folder and files will use specific icons to represent themselves
    - the actual icons will be based on whether terminal is using nerdfonts (detected already by biscuit-terminal); see icons section below
- the directory or filename will follow the icon representing it
- we will use line drawing characters to make the directory tree look connected
- struct will implement the `Default` trait and when doing so will use the current working directory to show
- struct will implement `Try` and `TryFrom` for any combination of string types and `Prose` struct
- we will offer both a `new(dir)` and `new_with_formatting(dir)` function:
    - the `new(dir)` establishes the root directory but no default formatting
    - the `new_with_formatting(dir)` establishes the root directory and the following formatting:
        - files and directories which start with a `.` are italicized
        - files which are covered by the glob patterns in `.gitignore` are dimmed
        - directories matching the glob pattern in the .gitignore file will be shown but not recursed into
- there will be a number of builder functions included to aid configuration including:
    - `dim_gitignore()`
    - `italicize_dot_files()`
    - `italicize_dot_directories()`
    - `hide_dot_files()`
    - `hide_dot_directories()`
    - `filter_files(Vec<String>)`
        - this will only show files which meet the glob patterns provided
        - only subdirectories which have 1 or more files in them will be shown when filtering for files
    - `do_not_recurse_gitignore_dirs()`
    - `highlight_with_red<T: Into<String>>(Vec<T>)`
        - any filename which is a fuzzy match to the patterns passed in will be made red and bold
    - `highlight_with_green<T: Into<String>>(Vec<T>)`
        - any filename which is a fuzzy match to the patterns passed in will be made green and bold
    - `depth(d: u32)`
        - indicates how many levels into the directory structure to recurse
        - default is 20
        - when the depth limit has been reached the sub-folders at that level will be shown but no more recursion will take place
- `FileSystem` struct will implement the Renderable trait

All attempts will be to make this struct as ergonomic as possible.

## Formatting Features

## Icons

### Directories / Folders

When the terminal doesn't have nerdfonts we will use these icons for directories:

- 📂 BASE folder
- 📁 (folder/directory when at depth limit)

When the terminal is using a nerdfont then we will start with the base folder but allow overrides for known folders:

- Base Folder: `e5fe`  
- Base Folder (at depth limit): `e652`
- .git folder: `e5fb`
- .github folder: `e5fd`
- Utils folder: `f19fc` (a folder named `utils`, `util`, or `utilities`)
- Docs folder: `ebdf` (a folder named `docs` or `documents`)

### Files

For non-nerdfonts we will always use the `📄` icon. In contrast, if a nerdfont is available then we will use `ea7b` as the BASE file but with the following overrides:

- Markdown: `f0354` (except for `README.md`, `Claude.md`, `Agents.md`, `Gemini.md` and `SKILL.md` variants)
- Readme: `f02e` (capitalization does not matter)
- `Claude.md`: `f0721` (capitalization does not matter, must be in repo root)
- `Agents.md`: `f21b` (capitalization does not matter, must be in repo root)
- `Gemini.md`: `f21b` (capitalization does not matter, must be in repo root)
- `SKILL.md`: `f113c` (capitalization DOES matter)
- Symlink File: `eaee`
- CSV: `eefc`
- Rust (.rs): `e7a8`
- Typescript (.ts): `e8ca`
- Javascript (.js): `e781`
- TOML: `e6b2`
- YAML: `e8eb` (both `.yml` and `.yaml`)
- JSON: `eb0f` (includes `.json`, `.json5`, `.jsonl`, and `.jsonc` files)
- XML: `f05c0`
- HTML: `e736`
- CSS: `e74a`
- AIFF, MP3, WAV, OPUS: `f0384`
- Excel: `f102d`
- PDF: `f1c5`
- Word: `e6a5`
- Powerpoint: `f1c4`
- Google Sheet: `f09f7`
- Scalar Image: `f1c5`
- Vector Image: `e698`
- Video File: `f022b`
- Text: `f15c` (includes `.txt` and `.text`)
- Keys File: `f0306` (includes `id_rsa`, `id_github`, `*.pub`)
- SSH: `f08c0` (includes `known_hosts` and `authorized_keys`)
- Config: `e615` (includes files named `config` and files ending with `.cfg` or `.config`)
- AI Model files: `ee9c` (includes `.gguf`, `.safetensor`, etc.)
- `.gitignore`: `e702`
- Apple OS files: `e711` (includes `.DS_Store` files)
- `.editorconfig`: `e652`
- `.env`: `eafa`
- User's home directory: `f10b5`
- justfile: `ee0d`
