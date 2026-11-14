# Configuring DMLS

DMLS reads settings from two places: a `.dmls.toml` file, and settings the
editor sends. Every key is optional; no file means the built-in defaults.
`md` and other Darkmatter tools never read `.dmls.toml`, and frontmatter
never configures the server.

## Where the file is found

DMLS looks for `.dmls.toml` **directly inside each workspace folder**, in
the order the editor lists the folders, and uses the first one that loads.
It never searches parent or child folders and never merges several files.
`dmls --config <path>` replaces this search with one file.

```text
~/work/                 workspace folder 1: no .dmls.toml, skipped
~/notes/.dmls.toml      workspace folder 2: used
~/notes/docs/.dmls.toml never read
```

- **Broken files.** Invalid TOML, or a wrong value type outside
  `[formatting]`, makes DMLS ignore the whole file, which leaves defaults.
  An invalid `[formatting]` value drops only the formatting settings.
  Unknown keys outside `[formatting]` are ignored silently.
- **Live reload.** Edits apply without a restart. DMLS reloads when the
  editor reports a change to `.dmls.toml`, or on any
  `workspace/didChangeConfiguration`. Neovim on Linux doesn't report file
  changes, so there an edit applies on the next configuration change or a
  restart.
- **Messages.** Problems with the formatting settings appear as one `dmls:`
  message in the editor each time the configuration loads.

## Precedence

Each setting comes from the highest of these that sets it:

1. editor settings, from `workspace/didChangeConfiguration`;
2. `.dmls.toml`;
3. the built-in defaults.

Editor settings use the same key names, either wrapped in a `dmls` object or
not: `{"dmls": {"wiki": {"enable": false}}}`. Sections deep-merge, and
`[formatting]` merges one rule at a time. DMLS never asks the editor for
settings (it doesn't use `workspace/configuration`). The shipped VS Code
extension sends no settings, so **in VS Code, use `.dmls.toml`**.

## Every key

| Key | Default | Effect |
|---|---|---|
| `workspace.root` | unset | list only this folder; relative paths are joined to each workspace folder |
| `workspace.include` | `["**/*.md", "**/*.markdown"]` | which files are Markdown documents; also the globs DMLS asks the editor to watch |
| `workspace.exclude` | `[]` | globs removed from the listing, for Markdown and YAML alike; wins over `include` |
| `wiki.enable` | `true` | master switch for every wiki-link feature, including its tokens |
| `wiki.wiki_root` | unset | the folder wiki links resolve against |
| `wiki.path_style` | `"shortest"` | what completion inserts: `shortest`, `relative`, or `root-relative` |
| `wiki.heading_completion_style` | `"text"` | `text` (the heading as written) or `slug` |
| `schema.strict` | `false` | `true`: missing required keys and unknown keys are errors (by default missing keys aren't reported and unknown keys are warnings) |
| `schema.extensions.<name>.path` | required | a document schema merged over the baseline for matching files |
| `schema.extensions.<name>.globs` | `[]` | which files it applies to, relative to the workspace folder; no globs means it never applies |
| `schema.slow_validation_ms` | `200` | log slow validations at `warn`; affects logging only |
| `expressions.strict` | `false` | `true`: also warn on call arguments that may be `null` or have unknown types |
| `shell.policy_path` | unset | a **directory** containing `.darkmatter-shell-whitelist`; hover then marks the commands it lists as approved. Use an absolute path |
| `code_actions.categories.<category>` | all on | `false` turns a quick-fix category off (list below) |
| `formatting.markdown.<rule>` | none | workspace defaults for Format Document; a document's own `style.markdown` overrides them |
| `symbols.frontmatter` | `false` | add top-level frontmatter keys to the outline |
| `semantic_tokens.enable` | `true` | master switch for semantic tokens |
| `style.strict`, `hints.inlay`, `diagnostics.debounce_ms` | — | accepted but have no effect yet |

The code-action categories are `create-missing-file`, `add-missing-key`,
`migrate-deprecated-style`, `close-directive-block`,
`wrap-in-interpolation-literal`, `reference-dash-separated-key`, and
`rewrite-with-concatenation`.

The `formatting.markdown` rule keys are the same ones a document uses under
`style.markdown`, for example `indentation`, `list-item-padding`
(`preserve|none|one`), `fixed-width` (`1..=1000`, `true` for 120, or
`false`), and `default-language`. The older `formatting.cleanup` and
`formatting.fixed_width` keys still work, with a migration warning. A source
can't mix them with `[formatting.markdown]`.

### A fuller example

```toml
# .dmls.toml
[workspace]
include = ["docs/**/*.md", "*/README.md"]
exclude = ["target/**", "drafts/**"]

[wiki]
wiki_root = "notes"
path_style = "root-relative"

[schema]
strict = true

[schema.extensions.team]          # always-on schema for some paths
path = "schemas/team-prompt.yaml" # a document schema, never a trigger file
globs = ["prompts/**", ".claude/**"]

[expressions]
strict = true

[shell]
policy_path = "/Users/me/.config/darkmatter"

[code_actions.categories]
"create-missing-file" = false

[formatting.markdown]
indentation = 2
list-item-padding = "none"
fixed-width = 100

[symbols]
frontmatter = true
```

## What `[workspace]` does and doesn't control

- It controls the **file list**: what wiki links resolve against, what
  workspace symbols search, and what the index holds.
- It does **not** control which schemas apply. The schema roots come from
  each document's location. `exclude = ["schemas/**"]` hides those YAML files
  from the list, but their triggers still apply.
- **YAML files** (`.yaml`, `.yml`) are always listed and watched, whatever
  `include` says. `exclude` still removes them.
- **Ignore files.** `.gitignore`, `.ignore`, `.git/info/exclude`, and your
  global Git excludes are honored even outside a Git repository.
- **Hidden folders** such as `.claude/` are listed. `.git` is never listed,
  and symlinks aren't followed.

## Environment variables

These are read once, when the editor starts the server. Restart the server
after changing one.

| Variable | Effect |
|---|---|
| `SCHEMAS_DIR` | an **absolute** path to a folder that becomes the first schema root (the folder itself, not a `schemas/` inside it) |
| `HOME` (`USERPROFILE` on Windows) | `~` references and the `~/schemas` root |
| any `VAR` | `{{{VAR}}}` segments in file references |
| `DMLS_CACHE_DIR` | where the index store lives. Unset: the user cache dir (`~/Library/Caches/darkmatter/dmls/` on macOS). An absolute path: that directory. `off`: memory only. It's deliberately not a `.dmls.toml` key, because the cache location belongs to the machine, not the repository. Keep it on a local disk (not NFS or `/mnt/c` under WSL); it's always safe to delete |

Example: `DMLS_CACHE_DIR=off SCHEMAS_DIR=/opt/team/schemas code .`
