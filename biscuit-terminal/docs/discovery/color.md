# Color Discovery in the **Biscuit Terminal**

## Using `biscuit-terminal`'s Programmatic Discovery

### The ANSI palette

Besides the background and foreground colors, `biscuit-terminal` asks the
terminal for its 16 ANSI palette colors (indices 0–15) with OSC 4, in the
same once-per-process exchange as the OSC 10/11 colors. The result is 16
slots, each a measured `RgbValue` or unknown:

```text
terminal replies:  ESC]4;0;rgb:1a1a/1b1b/2626 BEL   ESC]4;1;#f7768e ESC\
palette:           slot 0 = (26, 27, 38), slot 1 = (247, 118, 142), slots 2–15 unknown
```

- A missing, cut-short, or malformed reply makes that slot unknown; it is
  never an error. The palette is only queried, never set.
- The query is sent only where the colors are: a terminal on stdout or
  stderr, not CI, no multiplexer (tmux, Zellij, GNU Screen), probing not
  turned off, and a terminal known to answer color queries. Anywhere else,
  every slot is unknown.
- **Native Windows sends no OSC 4 query**, so its palette is always unknown,
  and diagrams there use accents derived from the background color.
- The palette is not public API. `biscuit-visualized` reads it through the
  hidden `discovery::osc_queries::ansi_palette()`. **Planned:** no diagram
  palette consumes it yet.

[Live Terminal Queries](./terminal-queries.md#the-ansi-palette-osc-4)
describes the reply forms, the cost, and the caching in full.

## Using the `bt` CLI to discover color information

