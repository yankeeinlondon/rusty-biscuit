# fzf

`fzf` is an interactive **fuzzy finder** for the terminal. It reads lines from STDIN (or walks the filesystem if STDIN is a TTY) and presents a live, filterable picker. As you type, it scores candidate lines using a fuzzy matching algorithm and prints the chosen line(s) to STDOUT on accept.

It is designed to compose: any pipeline that produces newline-delimited text can become an interactive picker by piping through `fzf`. Common use cases include:

- File pickers driven by `find`, `fd`, `rg --files`, or `git ls-files`
- Branch/commit pickers around `git`
- Process pickers around `ps`
- History pickers (the official `--bash`/`--zsh`/`--fish` integrations bind it to `Ctrl-R`)
- Multi-stage filters built with `--bind`, `--preview`, and `reload`/`change-query` actions

- **Project URL:** <https://github.com/junegunn/fzf>
- **Documentation:** `fzf --man`
- **Version covered here:** 0.71.x

## How fzf gets its input

By default, when STDIN is **not** a terminal, fzf reads lines from STDIN. When STDIN **is** a terminal, fzf either runs `$FZF_DEFAULT_COMMAND` or falls back to its built-in directory walker (see `--walker*` flags below).

```sh
# Pipe input
rg --files | fzf

# Use the built-in walker (no STDIN, no FZF_DEFAULT_COMMAND)
fzf

# Provide a default command via environment
FZF_DEFAULT_COMMAND='fd --type f' fzf
```

---

## Search

Flags that control how the query is matched against input lines.

### `-e`, `--exact`

Disable fuzzy matching and require exact substring matches. Use when fuzzy matches produce too much noise — for example when filtering log lines or large symbol tables where you know the exact token.

### `+x`, `--no-extended`

Disable extended-search mode (the default). Extended mode lets you combine terms (`foo bar`), invert (`!baz`), anchor (`^src`, `\.rs$`), and require exact (`'word`). Turn it off only when your input legitimately contains those operator characters and you want them treated literally.

### `-i`, `--ignore-case`

Force case-insensitive matching. Use in scripts where you don't want behavior to depend on whether the query has uppercase letters.

### `+i`, `--no-ignore-case`

Force case-sensitive matching. Useful when filtering identifiers where case is meaningful (e.g. Rust types vs. snake-case fns).

### `--smart-case` (default)

Case-insensitive if the query is all lowercase, case-sensitive otherwise. Generally the right default.

### `--scheme=SCHEME`

Pick a scoring scheme: `default`, `path`, or `history`.
- `path` — boosts matches at path component boundaries; use when filtering file paths.
- `history` — biases toward more recent items; use when feeding shell history or recent-files lists.
- `default` — generic text.

### `-n`, `--nth=N[,..]`

Limit matching to specific fields of each line (1-indexed; supports ranges like `2..4`, negative indices, and comma lists). Use when lines have structure and you want to ignore noise — e.g. matching only on the filename column of `git ls-files -s` output.

### `--with-nth=N[,..]`

Like `--nth`, but rewrites the **displayed** form of each line. The original line is still returned on accept. Use when you want a clean UI but need the full line downstream.

### `--accept-nth=N[,..]`

Pick which fields are printed when the user accepts. Use when you want to take a structured line in but emit only one column (e.g. take `pid command args` and emit just the pid).

### `-d`, `--delimiter=STR`

Field delimiter (regex) used by `--nth`/`--with-nth`/`--accept-nth`. Defaults to AWK-style whitespace. Set to `:` for `grep -n`-style output, `\t` for TSV, etc.

### `+s`, `--no-sort`

Preserve input order instead of sorting by score. Use when input is already meaningfully ordered (e.g. shell history newest-first, or a curated list).

### `--literal`

Don't normalize Latin-script letters (e.g. `é` won't match `e`). Use when normalization would create false matches in language-sensitive data.

### `--tail=NUM`

Keep at most `NUM` items in memory; older lines are dropped. Use when watching an unbounded stream (e.g. `tail -f` of a log file) so fzf doesn't grow without bound.

### `--disabled`

Disable searching entirely. Use as a starting state when the query is meant to drive an external command via `change:reload` bindings (server-side filtering pattern).

### `--tiebreak=CRI[,..]`

Sort criteria when scores tie: `length` (default), `chunk`, `pathname`, `begin`, `end`, `index`. Use `pathname` for file pickers, `begin` to favor matches earlier in the line.

---

## Input/Output

### `--read0`

Read NUL-delimited input. Pair with `find -print0`, `fd -0`, or `rg --null` to handle filenames containing newlines safely.

### `--print0`

Emit NUL-delimited output. Pair with `xargs -0` for safe file handling downstream.

### `--ansi`

Process ANSI color codes in input (preserve color in the picker). Use when piping `git log --color=always`, `rg --color=always`, etc.

### `--sync`

Wait for the full input before showing the UI, so subsequent stages (e.g. `--bind` actions) see a stable list. Use in multi-stage pipelines where timing matters.

---

## Global Style

### `--style=PRESET`

Bundle of layout/border defaults: `default`, `minimal`, or `full[:BORDER_STYLE]`. Quick way to opt into a richer or stripped-down look.

### `--color=COLSPEC`

Pick a base scheme (`dark`, `light`, `base16`, `bw`) and/or override individual colors (`fg:#ff0000,bg:-1,...`). Use to match your terminal theme or to brand a launcher.

### `--no-color`

Disable colors. Use in dumb terminals or when capturing output for diffing.

### `--no-bold`

Suppress bold text. Use on terminals where bold is hard to read.

---

## Display Mode

### `--height=[~][-]HEIGHT[%]`

Render fzf inline below the cursor at a given height instead of fullscreen. Negative = `term_height - HEIGHT`. `~` auto-sizes to input. Use for shell integration so fzf doesn't take over the screen.

### `--min-height=HEIGHT[+]`

Minimum height when `--height` is a percentage. `+` auto-grows to fit other layout elements (preview, header, etc.).

### `--popup[=OPTS]` / `--tmux[=OPTS]`

Open fzf in a tmux (3.3+) or Zellij (0.44+) popup window. Options control position (`center|top|bottom|left|right`) and size. Use to keep your scrollback intact while running fzf.

---

## Layout

### `--layout=LAYOUT`

`default` (prompt at bottom, list grows up), `reverse` (prompt at top, list grows down), `reverse-list` (prompt at bottom, list reversed at top). `reverse` is the most common modern choice.

### `--margin=MARGIN`

Outer margin around the whole UI. Accepts `TRBL`, `TB,RL`, `T,RL,B`, or `T,R,B,L`. Use to prevent the UI from hugging terminal edges.

### `--padding=PADDING`

Inner padding inside the border. Same syntax as `--margin`.

### `--border[=STYLE]`

Draw a border around fzf. Styles: `rounded`, `sharp`, `bold`, `block`, `thinblock`, `double`, `horizontal`, `vertical`, side-only (`top`/`bottom`/`left`/`right`), `line`, `none`. Pairs well with `--popup`.

### `--border-label=LABEL` / `--border-label-pos=COL`

Print a label on the border, with positional control (positive = from left, negative = from right, `:bottom` to anchor at the bottom edge). Use to title the picker (e.g. ` Files `).

---

## List Section

### `-m`, `--multi[=MAX]`

Enable multi-select with `Tab`/`Shift-Tab`. Optional cap. Essential when you want to pick several files to operate on at once.

### `--highlight-line`

Highlight the entire current line, not just the matched substring. Improves readability when lines are long.

### `--cycle`

Wrap-around scrolling so `Up` from the top jumps to the bottom and vice versa.

### `--wrap[=MODE]`

Wrap long lines (`char` or `word`). Use when items are paragraph-like (e.g. commit messages).

### `--wrap-sign=STR`

Indicator drawn on continuation lines when wrapping is on.

### `--no-multi-line`

Force single-line display even when `--read0` input contains newlines.

### `--raw`

Show **non-matching** items too (greyed out). Useful for context — you can see what's being filtered out.

### `--track`

Keep the current selection pinned across reloads/result updates. Use with `reload`/`change-query` bindings so the user doesn't lose their place.

### `--id-nth=N[,..]`

Define which fields make up an item's identity. Affects `--track` and similar cross-reload operations.

### `--tac`

Reverse input order before display. Common with shell history, `git log`, etc.

### `--gap[=N]` / `--gap-line[=STR]`

Insert blank lines (or a separator string) between items. Improves scan-ability for multi-line items.

### `--freeze-left=N` / `--freeze-right=N`

Pin the first/last N fields visible while scrolling horizontally. Useful for tabular data (e.g. always show the PID column).

### `--keep-right`

When a line is too wide, keep the right end visible (default keeps the left). Use when the tail of a path or log line is more meaningful than the head.

### `--scroll-off=LINES`

Minimum lines kept above/below the cursor when scrolling. Like vim's `scrolloff`.

### `--no-hscroll` / `--hscroll-off=COLS`

Disable horizontal scrolling, or set how many columns to keep right of the matched substring.

### `--jump-labels=CHARS`

Characters used by jump (`easy-motion`-style) actions. Customize for non-QWERTY layouts.

### `--gutter=CHAR` / `--gutter-raw=CHAR`

Character drawn in the leftmost gutter column (for the current item / for filtered items in `--raw` mode).

### `--pointer=STR` / `--marker=STR` / `--marker-multi-line=STR`

Customize the pointer to the current line, the multi-select marker, and the multi-line variant.

### `--ellipsis=STR`

String shown when a line is truncated.

### `--tabstop=SPACES`

How wide a `\t` renders.

### `--scrollbar[=C1[C2]]` / `--no-scrollbar`

Customize or hide the scrollbar (one char each for list and preview panes).

### `--list-border[=STYLE]` / `--list-label=LABEL` / `--list-label-pos=COL`

Border and label specifically for the list section (independent of the outer `--border`).

---

## Input Section

The "input section" is the prompt/query area.

### `--no-input`

Hide the input box entirely. Use when fzf is a pure menu and the query is irrelevant.

### `--prompt=STR`

Replace the default `> ` prompt. Useful for indicating mode (` files> `, ` branches> `).

### `--info=STYLE`

Where the match counter goes: `default`, `right`, `hidden`, `inline[-right][:PREFIX]`. `inline` is compact and common in modern setups.

### `--info-command=COMMAND`

Use the output of a shell command as the info line. Use to embed live data (e.g. current git branch).

### `--separator=STR` / `--no-separator`

Customize or hide the horizontal line drawn under the info area.

### `--ghost=TEXT`

Placeholder text shown when the query is empty. Useful as a hint (e.g. `Type to filter…`).

### `--filepath-word`

Make `Alt-B`/`Alt-F` (word-wise movement) treat path separators as word boundaries. Essential when editing path queries.

### `--input-border[=STYLE]` / `--input-label=LABEL` / `--input-label-pos=COL`

Border and label specifically for the input section.

---

## Preview Window

### `--preview=COMMAND`

Run a shell command for the highlighted line and show its output in a side pane. `{}` expands to the current item; `{1}`, `{2}`, … to fields. Examples:

```sh
fzf --preview 'bat --color=always {}'
fzf --preview 'git show --color {1}'
```

This is fzf's killer feature for file/commit/log pickers.

### `--preview-window=OPT`

Configure the preview pane. Accepts position, size, wrap, follow, scroll, header lines, hidden state, border, and a *responsive* alternative layout via `<SIZE_THRESHOLD(ALTERNATIVE_LAYOUT)`. Example: `right:60%,border-left,~3,<80(down:50%)`.

### `--preview-border[=STYLE]`

Shorthand for setting just the preview's border style.

### `--preview-label=LABEL` / `--preview-label-pos=N`

Label on the preview border, like `--border-label` for the main UI.

### `--preview-wrap-sign=STR`

Continuation indicator inside the preview pane when wrapping is on.

---

## Header

### `--header=STR`

Pinned text at the top of the list. Use for usage hints (`Tab: select | Enter: open | Ctrl-R: reload`).

### `--header-lines=N`

Treat the first N lines of input as a (non-matchable) header. Perfect for piping `ps`, `df`, or other tools whose first line is a column header.

### `--header-first`

Render the header above the prompt rather than between prompt and list.

### `--header-border[=STYLE]` / `--header-label=...` / `--header-label-pos=...`

Border + label for the header section.

### `--header-lines-border[=STYLE]`

Give the `--header-lines`-derived header its own border (or just a separator with `none`).

---

## Footer

### `--footer=STR`

Pinned text at the bottom (mirror of `--header`).

### `--footer-border[=STYLE]` / `--footer-label=...` / `--footer-label-pos=...`

Border + label for the footer section.

---

## Scripting

Flags for using fzf non-interactively or as part of a pipeline.

### `-q`, `--query=STR`

Pre-fill the query. Use when fzf is launched from a context that already implies a filter.

### `-1`, `--select-1`

If exactly one line matches the initial query, accept it and exit immediately. Use for "fuzzy unless ambiguous" UX.

### `-0`, `--exit-0`

Exit immediately with no selection if the initial query produces zero matches. Pairs with `--select-1`.

### `-f`, `--filter=STR`

Run as a non-interactive filter: print lines matching `STR` and exit. Equivalent to a fuzzy `grep`. Useful in pipelines and tests.

### `--print-query`

Prepend the final query string as the first output line. Use when downstream scripts need to know what the user typed (e.g. to "create new" if no match).

### `--expect=KEYS`

Comma-separated list of keys that should also accept the selection. The pressed key is printed as the first output line so scripts can branch on it (e.g. `Enter` = open, `Ctrl-E` = edit).

---

## Key/Event Binding

### `--bind=BINDINGS`

Define custom key and event bindings. Bindings map a trigger (`ctrl-r`, `change`, `focus`, `start`, `result`, …) to one or more actions (`reload(...)`, `execute(...)`, `change-query(...)`, `toggle-preview`, `accept`, …). This is fzf's most powerful feature — almost every advanced workflow (live `rg` search, branch switcher with preview, multi-stage menus) is a `--bind` recipe.

```sh
# Live ripgrep search
fzf --disabled \
    --bind 'change:reload(rg --column --line-number --no-heading --color=always {q} || true)' \
    --bind 'enter:become(vim {1} +{2})' \
    --ansi
```

---

## Advanced

### `--with-shell=STR`

Override the shell used for `execute`/`reload`/`preview` child processes (e.g. `--with-shell='bash -c'` or `--with-shell='nu -c'`).

### `--listen[=[ADDR:]PORT]` / `--listen=SOCKET_PATH`

Start an HTTP server fzf listens on; external processes can POST actions to drive the picker. Use for IPC between fzf and an editor or other tools. Add `--listen-unsafe` to allow remote `execute` (powerful and risky — gate behind trusted input only).

---

## Directory Traversal

Used only when STDIN is a TTY **and** `$FZF_DEFAULT_COMMAND` is unset.

### `--walker=OPTS`

Comma list of: `file`, `dir`, `follow`, `hidden`. Default: `file,follow,hidden`.

### `--walker-root=DIR [...]`

One or more roots to walk. Defaults to `.`.

### `--walker-skip=DIRS`

Comma-separated directory **names** to prune. Default: `.git,node_modules`.

---

## History

These flags govern fzf's *query* history (separate from shell command history).

### `--history=FILE`

Persist queries to FILE so `Ctrl-N`/`Ctrl-P` can recall them in future fzf runs.

### `--history-size=N`

Cap on stored entries (default 1000).

---

## Shell Integration

Print a shell-specific snippet to source. The official integrations add `Ctrl-T` (paste files), `Alt-C` (cd into dir), and `Ctrl-R` (history search).

### `--bash`

```sh
eval "$(fzf --bash)"
```

### `--zsh`

```sh
source <(fzf --zsh)
```

### `--fish`

```fish
fzf --fish | source
```

---

## Help

### `--version`

Print version and exit.

### `--help`

Short usage summary (the basis of this document).

### `--man`

Render the full man page. Use this when `--help` isn't enough — it documents the full `--bind` action grammar, preview placeholders, and all environment variables.

---

## Environment Variables

These are not switches, but they shape every fzf invocation.

### `FZF_DEFAULT_COMMAND`

Command run to populate input when STDIN is a TTY. Common choices: `fd --type f --hidden --exclude .git`, `rg --files`, `git ls-files`.

### `FZF_DEFAULT_OPTS`

Default flags applied to every fzf run. Typical setup:

```sh
export FZF_DEFAULT_OPTS='--layout=reverse --height=40% --border --info=inline'
```

### `FZF_DEFAULT_OPTS_FILE`

Path to a file containing default options (one per line, `#` comments allowed). Useful when `FZF_DEFAULT_OPTS` would be unwieldy.

### `FZF_API_KEY`

Value sent in the `X-API-Key` header for `--listen`. Required when using `--listen-unsafe` to gate remote action execution.

---

## Quick recipes

```sh
# File picker with preview, edit on enter
fzf --layout=reverse --height=80% --border \
    --preview 'bat --style=numbers --color=always --line-range=:200 {}' \
    --bind 'enter:become(${EDITOR:-vim} {})'

# Multi-select files into xargs (NUL-safe)
fd --type f -0 | fzf --read0 --print0 -m | xargs -0 wc -l

# Branch picker with commit log preview
git branch --all --color=always | \
    fzf --ansi --preview 'git log --color=always --oneline -n 30 {1}' \
        --bind 'enter:become(git switch $(echo {} | sed "s|^[* ] ||;s|^remotes/[^/]*/||"))'

# Live ripgrep
INITIAL_QUERY="${*:-}"
RG_PREFIX="rg --column --line-number --no-heading --color=always --smart-case"
: | fzf --ansi --disabled --query "$INITIAL_QUERY" \
        --bind "start:reload($RG_PREFIX -- {q})+unbind(change,ctrl-f)" \
        --bind "change:reload:sleep 0.1; $RG_PREFIX -- {q} || true" \
        --delimiter : \
        --preview 'bat --color=always {1} --highlight-line {2}' \
        --preview-window 'up,60%,border-bottom,+{2}+3/3,~3' \
        --bind 'enter:become(${EDITOR:-vim} {1} +{2})'
```
