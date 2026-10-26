# Color Discovery in the **Biscuit Terminal**

## Using `biscuit-terminal`'s Programmatic Discovery

### The ANSI palette

Besides the background and foreground colors, `biscuit-terminal` asks the
terminal for its 16 ANSI palette colors (indices 0–15: the black, red, green,
… bright white a user's theme defines) with OSC 4, an escape sequence that
asks the terminal "what color is index _n_?". It is sent in the same
once-per-process exchange as the OSC 10/11 foreground and background queries.
The result is 16 slots, each a measured
[`RgbValue`](../../lib/src/discovery/osc_queries/types.rs) (an 8-bit red,
green, blue triple) or unknown:

```text
terminal replies:  ESC]4;0;rgb:1a1a/1b1b/2626 BEL   ESC]4;1;#f7768e ESC\
palette:           slot 0 = (26, 27, 38), slot 1 = (247, 118, 142), slots 2–15 unknown
```

- A missing, cut-short, or malformed reply makes that slot unknown; it is
  never an error. The palette is only queried, never set.
- The query is sent only where the colors are: a terminal on stdout or
  stderr, not CI, no multiplexer (tmux, Zellij, GNU Screen), probing not
  turned off, and a terminal known to answer color queries (Kitty, WezTerm,
  iTerm2, Alacritty, Ghostty, Foot, Contour, and on native Windows also
  Windows Terminal). Anywhere else, every slot is unknown.
- **Native Windows sends the OSC 4 query too**, through the same console
  query backend as the colors; a terminal that stays silent leaves the
  palette unknown, and diagrams there use accents derived from the
  background color.
- The palette is not public API. `biscuit-visualized` reads it through the
  hidden (left out of the API docs)
  [`discovery::osc_queries::ansi_palette()`](../../lib/src/discovery/osc_queries/mod.rs):
  ANSI 1–6 become a diagram's accent colors in the terminal (see
  [Terminal palettes](../../../biscuit-visualized/docs/merman.md#terminal-palettes)).

[Live Terminal Queries](./terminal-queries.md#the-ansi-palette-osc-4)
describes the reply forms, the cost, and the caching in full.

## Using the `bt` CLI to discover color information

`bt` with no subcommand prints the terminal's colors in its **Colors**
section: the color depth, the light or dark mode, and the background,
foreground, and cursor colors. Each color is labeled `measured` when the
terminal answered the live query and `estimated` when it came from
`COLORFGBG` or a per-terminal default:

```text
Colors
  - Depth: TrueColor
  - Mode: Dark
  - Background: #1e1e1e (30, 30, 30) estimated
```

`bt --json` carries the same colors as `bg_color`, `text_color`, and
`cursor_color`, each with `r`, `g`, `b`, `hex`, and a `source` of `measured`
or `estimated` ([`output.rs`](../../cli/src/output.rs)). `bt` does not report
the ANSI palette.
