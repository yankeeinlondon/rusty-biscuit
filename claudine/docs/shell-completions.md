# Shell Completions

Claudine ships dynamic shell completions for `bash`, `zsh`, and `fish`.
The generated script registers a callback that shells out to the hidden
`claudine __complete` subcommand on every `<TAB>`. The callback handles
the file-reference argument positions documented below; everything else
falls back to each shell's default behavior.

`powershell` and `elvish` retain the legacy one-line `COMPLETE=<shell>`
bootstrap that activates the older `CompleteEnv` runtime path. The
supplement acceptance matrix covers bash/zsh/fish only.

## Installation

`claudine completions <shell>` emits the script for each supported shell.
Redirect the output into the shell's completion file:

```sh
# Bash — redirect into your bash-completion completions directory
claudine completions bash > ~/.local/share/bash-completion/completions/claudine

# Zsh — redirect into the first directory in $fpath (usually autoloaded)
claudine completions zsh > "${fpath[1]}/_claudine"

# Fish — redirect into the user fish-completions directory
claudine completions fish > ~/.config/fish/completions/claudine.fish

# PowerShell (legacy one-line bootstrap)
claudine completions powershell >> $PROFILE

# Elvish (legacy one-line bootstrap)
claudine completions elvish >> ~/.elvish/rc.elv
```

Open a new shell (or source the rc file) and completion is live. The
bash/zsh/fish scripts only need to be regenerated if Claudine changes the
callback wiring itself — the completion candidates are always produced by
the currently installed binary via the `__complete` subprocess.

> **Backwards compatibility.** Users who previously sourced the old
> `COMPLETE=<shell> claudine` one-liner continue to reach the legacy
> completion engine until they run `claudine completions <shell>` and
> reinstall the generated script. There is no automatic migration.

## How dynamic completion works

On every `<TAB>`, the registered callback collects the shell's current
word list and the index of the token being completed, then invokes:

```text
claudine __complete --current <INDEX> -- <argv...>
```

where `<argv...>` is the full command line the user typed, starting with
the binary name at position 0. The hidden `__complete` subcommand is a
parallel code path that does not load configuration, telemetry, or the
wrapper launch pipeline. It classifies the cursor position, applies the
supplement's candidate-selection rules, and prints one candidate per line
on stdout. When the engine has no candidates to offer (non-targeted
positions, errors), each shell falls back to its default file completion.

## File reference completion

Dynamic completion fires at exactly these argument positions — everywhere
else the shell's default behavior applies.

### Targeted positions

| Position | Subcommands |
| --- | --- |
| Positional `<FILE>` (index 0) | `compose`, `inline-compose`, `sequence` |
| `--append-system-prompt <FILE>` / `--asp <FILE>` | `compose`, `inline-compose`, `sequence`, `claude`, `codex`, `gemini`, `goose`, `kimi`, `opencode`, `qwen` |
| `--replace-system-prompt <FILE>` / `--rsp <FILE>` | same 10 subcommands |

### Supported token shapes

| Partial | Scope |
| --- | --- |
| `@…` | `@`-prefixed magic path — enumerated against the repo root plus the user home (`~/` and `~/.claudine/`). |
| `prompts/…`, `docs/…`, any implicit-relative path | Implicit-relative — enumerated against the repo root only, matching `FileReference`'s implicit-relative contract. |

Other token shapes (`vault:`, absolute `/path`, `./`, `../`, `!`, `%…`,
`{{…}}`) return no candidates. The supplement's new engine does not
recognize `./` / `../` traversal UI or the `!` package sigil; those
behaviors are documented on the legacy `COMPLETE`-based path only and
will not reappear in freshly generated scripts.

### Character counting and candidate scope

The spec's "meaningful query characters" drive how much of the filesystem
is walked. Counting excludes the leading `@` sigil and resets after every
`/` path separator.

| Typed token | Meaningful chars | Scope |
| --- | --- | --- |
| empty, `@`, `prompts/` | 0 | curated only |
| `@p`, `@pr`, `prompts/a` | 1–2 | curated only |
| `@pro`, `prompts/abc` | 3+ | curated **plus** `.gitignore`-aware broad repo scan |

Curated scope is fixed — `prompts/` and `sequences/` under each of these
roots (when applicable):

- `<repo>/`
- `<package-root>/` (nearest enclosing Cargo package, via `sniff`)
- `<package-area-root>/` (area directory containing the package, via `sniff`)
- `~/`
- `~/.claudine/`

When the cursor is not inside any git repository, the curated user-scope
directories still apply; the 3+-character broad scan never activates.

### Matching semantics

Matching is **case-insensitive substring** on the filename with the
trailing `.md` stripped for matching only. Directory components are never
considered. The returned candidate keeps its `.md` extension so the shell
inserts a valid file reference.

- `@pr<TAB>` → `@prompts/prompt.md`, `@prompts/my-prompt.md`,
  `@prompts/suppress.md`, and any other curated-scope markdown whose
  filename contains `pr`.
- `@omp<TAB>` → `@prompts/prompt.md` (mid-filename substring).
- `./local` → zero candidates; `./`-prefixed paths are not a supported
  supplement entry form.

### Markdown-only

Every candidate is a `*.md` file. Directories, non-markdown files,
setters (`KEY=`), and `./`/`../` traversal tokens are all explicitly
filtered out before emission.

### Broad scan exclusion policy

The 3+-character broad scan reuses `sniff`'s `.gitignore`-aware markdown
walker ([`collect_markdown_files`](../../sniff/lib/src/filesystem/docs.rs)),
which is configured with `git_ignore`, `git_global`, and `git_exclude`
enabled. Files under `target/`, `node_modules/`, the Claudine shadow tree,
or any path matched by repo / global / `.git/info/exclude` rules never
reach the candidate list.

### Deduplication

Candidates whose canonicalized resolved paths collide are emitted at most
once. This matters in single-crate areas (`biscuit-visualized`, `tabby`,
`tui`) where the package root and package-area root coincide.

## Architecture reference

| File | Role |
| --- | --- |
| [`claudine/cli/src/completion/supplement.rs`](../cli/src/completion/supplement.rs) | Supplement completion engine (`run`, `emit_candidates`, meaningful-char counting, curated-scope enumeration) |
| [`claudine/cli/src/commands/completions.rs`](../cli/src/commands/completions.rs) | `claudine completions <shell>` + hidden `__complete` entry points |
| [`claudine/cli/src/completion/bootstrap.rs`](../cli/src/completion/bootstrap.rs) | Shell-specific script rendering (bash/zsh/fish full scripts; powershell/elvish legacy bootstraps) |
| [`claudine/cli/src/completion/mod.rs`](../cli/src/completion/mod.rs) | Legacy `CompleteEnv` entry point preserved for stale installations |
| [`biscuit-file/lib/src/file_reference/resolve.rs`](../../biscuit-file/lib/src/file_reference/resolve.rs) | `FileReference::complete_partial` — the partial-token API the supplement consumes |
| [`sniff/lib/src/filesystem/docs.rs`](../../sniff/lib/src/filesystem/docs.rs) | `collect_markdown_paths` — the `.gitignore`-aware markdown walker |

## Open questions

The supplement spec deliberately leaves several items open; they are not
resolved by the current implementation and may change in future features:

- **Ordering.** Candidates are currently emitted in sorted order (backed
  by a `BTreeSet`), but the spec does not commit to any specific ordering.
- **Caching.** The 3+-character broad scan re-walks the repo on every
  keypress. No caching layer is in place.
- **Performance budget.** No explicit latency budget is enforced.
- **Stale script migration.** No deprecation warning is emitted from the
  legacy `COMPLETE`-based path.
- **`HOME` unset behavior.** User-scope roots silently skip when
  `$HOME` is not set or the directories don't exist.
- **Symlink policy inside curated scopes.** Not specified; the
  `ignore::WalkBuilder` default governs broad-scan behavior.
- **Observability.** The `__complete` subcommand does not emit tracing
  output; a shell completion pipeline is the wrong sink for diagnostic
  logs without further design work.
