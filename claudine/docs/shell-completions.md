# Shell Completions

Claudine ships dynamic shell completions. A one-time bootstrap line in your
shell rc wires your shell to invoke `claudine` itself for every `<TAB>`, so
completion output always reflects the currently installed binary — no need
to regenerate a static script when a new composition command, flag, or
file-reference behavior ships.

## Installation

`claudine completions <shell>` emits the bootstrap snippet for each
supported shell. Redirect the output into the matching rc file:

```sh
# Bash
claudine completions bash >> ~/.bashrc

# Zsh
claudine completions zsh >> ~/.zshrc

# Fish
claudine completions fish >> ~/.config/fish/config.fish

# PowerShell
claudine completions powershell >> $PROFILE

# Elvish
claudine completions elvish >> ~/.elvish/rc.elv
```

Open a new shell (or `source` the rc file) and completion is live. The
bootstrap line is stable — you only write it once and Claudine owns the
runtime output from that point on.

## How dynamic completion works

When the shell reaches a `<TAB>` on a `claudine` command line, it invokes
Claudine with the environment variable `COMPLETE=<shell>`. Claudine's
`main()` detects this and enters a completion-only code path
(`CompleteEnv::complete` in [`claudine/cli/src/completion/mod.rs`]) that
exits before any normal CLI startup — no config load, no telemetry, no
wrapper launch. The completer then inspects the current partial token,
discovers matching candidates, validates them against the active
composition command, and emits the results for the shell to present.

Because the completer is the same binary that runs the commands, any
change to supported subcommands, flags, or composition rules shows up on
the next `<TAB>` with zero user action.

## File reference completion

Dynamic completion targets the shared positional on the three composition
commands:

- `claudine compose …`
- `claudine inline-compose …`
- `claudine sequence …`

Each command has its own validity rules (see below); all three share the
same token classification and scope discovery.

### Token shapes

The first step is classifying the partial token in isolation:

| Partial               | Scope                                                        |
| --------------------- | ------------------------------------------------------------ |
| `@…`                  | Repo-wide magic: the current repo root plus `~/.claudine/prompts` and `~/.claudine/sequences` |
| `!…`                  | Current monorepo **package area** only                       |
| `./…`                 | Immediate children of the current working directory          |
| `../…`                | Immediate children of the parent directory                   |
| `<bare>` (no sigil)   | Curated landing menu (cwd, repo-area prompts/sequences, repo root, repo prompts/sequences) |
| `KEY=…`               | Setter — completion is suppressed                            |
| `vault:…`, `/abs/…`, `%…`, `{{…}}` | Explicitly unsupported in v1 — returns zero candidates |

Setter suppression is strict: the key must match
`^[A-Za-z_][A-Za-z0-9_]*=`. Hyphenated keys (`my-key=`) and dotted keys
(`foo.bar=`) are treated as file references at completion time, matching
the runtime parser's more forgiving shape.

### Per-command validators

The walker emits every matching entry in its scope, then a mode-specific
validator decides whether the file is actually a candidate:

| Command           | Validator                                                   |
| ----------------- | ----------------------------------------------------------- |
| `compose`         | `.md` / `.markdown` extension only — no frontmatter parse   |
| `inline-compose`  | `.md` extension **and** a non-empty string `prompt:` in frontmatter |
| `sequence`        | `.md` extension **and** a resolvable sequence plan (inline list, inline objects, or external YAML reference) |

Validation is fail-closed: any I/O, UTF-8, parse, or size failure silently
omits the candidate rather than surfacing a shell-visible diagnostic.
Directories always pass through unconditionally so a single `<TAB>` keeps
descending into a subtree.

### Safety limits

The walker is bounded by fixed constants in
[`claudine/cli/src/completion/file_reference.rs`]:

- Maximum recursion depth: 4 (children of the scope root sit at depth 1)
- Maximum candidates before dedup: 500
- Maximum file size for frontmatter parse: 1 MiB
- Skip list: `.git`, `target`, `node_modules`, `dist`, `build`, `.next`,
  `.venv`, `venv`, `__pycache__`, and the Claudine shadow tree
  (`.claudine/.shadow`)
- Symlinks are never followed, so completion is safe against cycles

### Intentionally unsupported prefixes

The v1 completer deliberately emits zero candidates for:

- `vault:` — vault-resolved references depend on secrets Claudine cannot
  safely enumerate at completion time.
- Absolute paths (`/…`) — the shell's own path completion is a better
  experience for filesystem-anchored references.
- `%…` and `{{…}}` — recursive composition and template tokens are
  resolved at runtime; shell-time completion would guess wrong.

These are dead ends, not errors — no candidates are returned and the
shell falls back to its default behavior.

[`claudine/cli/src/completion/mod.rs`]: ../cli/src/completion/mod.rs
[`claudine/cli/src/completion/file_reference.rs`]: ../cli/src/completion/file_reference.rs
