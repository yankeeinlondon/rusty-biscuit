---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the clarify.md prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: "codex/gpt-6.1-sol"
reviewed_on: "2026-10-20"
review_iterations: 0
---

# Graphics Where Possible

People using `biscuit-terminal` and `biscuit-visualized` should see graphics
whenever their terminal can render them, including through tmux, Zellij, SSH,
and a Windows SSH server, without configuring anything that `biscuit-terminal`
could have worked out itself. Where graphics are impossible, they get a useful
fallback, and where a setting is blocking graphics, they are told which one.

This spec began because a WezTerm session into a Windows host showed no
graphics. Reconfiguring to WezTerm's multiplexer fixed that one setup; this
spec makes the general case work out of the box.

It also corrects the division of work between the two packages:
`biscuit-terminal` owns terminal detection and getting bytes onto the screen,
and `biscuit-visualized` owns diagrams for every target. Both changes land
here, in one spec, by the author's choice.

Research this spec builds on:

- [Kitty graphics](../../docs/research/graphics/kitty.md)
- [iTerm2 graphics](../../docs/research/graphics/iterm2.md)
- [Sixel graphics](../../docs/research/graphics/sixel.md)
- [Graphics through multiplexed sessions](../../docs/research/graphics/multiplexing.md)
- [Terminal standards](../../docs/research/terminal-standards/SKILL.md) (OSC 1337, modes 2026 and 2048)

## Current state

As of 2026-10-20:

- **Two formats are rendered today, not one.** Kitty everywhere detection
  says Kitty, and the iTerm2 format (`OSC 1337;File=`, legacy single-message
  form) for iTerm2. `protocol.rs` deliberately routes iTerm2 to the iTerm2
  encoder even when Kitty is reported.
- **Detection reads only environment variables**
  (`discovery/detection/image.rs`): `TERM_PROGRAM`, `TERM`, `ITERM_*`. It
  never probes and never consults the multiplexer detection that already
  exists (`detect_multiplexer()`, `multiplex_support()`). Consequences:
    - **tmux:** tmux sets `TERM_PROGRAM=tmux`, so detection finds no known
      terminal and returns `None`. Nothing wraps output for passthrough
      either.
    - **Zellij:** `TERM_PROGRAM` is inherited from the outer terminal. With
      iTerm2 outside, we send the iTerm2 format, which Zellij drops.
    - **SSH:** `TERM_PROGRAM` is usually not forwarded, so the result is
      `None` even when the local terminal is capable.
    - **Windows SSH server:** when `TERM_PROGRAM` *is* forwarded, detection
      picks Kitty, the system console host strips it, and the user sees
      reserved blank rows (see Evidence).
    - **mintty** (Git Bash, MSYS2, Cygwin, WSLtty) renders the iTerm2 format
      but is not recognized.
- **Fallbacks:** a Mermaid diagram with no image support falls back to its
  source as a ```` ```mermaid ```` code block; `bt image` and
  `TerminalImage` print nothing. Neither says why.
- `viuer` is a declared dependency of the library, but no `viuer::` call
  was found in its code.
- **Diagram components live in the terminal crate.** `MermaidDiagram`,
  `GraphExpression`, and `GitGraph` are in `biscuit-terminal/lib/src/components/`
  and implement terminal, tree, and (for the latter two) browser rendering.
  `biscuit-terminal` depends on `biscuit-visualized` through its `image`
  feature; `biscuit-visualized` has no other dependent. About 20 packages
  depend on `biscuit-terminal`; four of them (worktree-cli, sniff-cli,
  darkmatter, the `bt` CLI) enable `image` and compile the Mermaid renderer.
- **The render-tree renderer calls `MermaidDiagram` directly**
  (`render_tree/render.rs`) to turn a Mermaid code fence into a diagram when
  `TerminalMermaidMode::Image` is set.
- **Diagram PNGs are encoded twice (bug).** `biscuit-visualized` rasterizes a
  PNG, and `biscuit-terminal` decodes, resizes, and encodes it again
  (`encode_as_png`) even when the rendered width already matches the display
  width.

## Evidence gathered while scoping

### Kitty in iTerm2 (iTerm2 3.7.6, 2026-10-20)

| Case | Result |
| --- | --- |
| iTerm2 format (today's path) | Correct placement |
| Kitty with width only (`c=`) | Space reserved, **no image** |
| Kitty with width and height (`c=` + `r=`) | Correct placement |
| Kitty after an image with `q=2` | iTerm2 **ignores `q=2`** and replies `ESC _Gi=0;OK ESC \`; the reply lands in the shell's input buffer as junk |
| Kitty query (`a=q`, `i=31`) | Correct `OK` reply |

**Conclusion:** iTerm2 keeps the iTerm2 format. The comment at
`protocol.rs:49-50` ("more predictable for cursor placement") misstates the
reason and is corrected in this work.

### Windows OpenSSH server on `build-win-native` (2026-10-20)

The bytes a Windows process wrote inside an `ssh -tt` session were captured at
the client. The session's console is `C:\WINDOWS\system32\conhost.exe`.

| Sent by the Windows process | Arrived at the client |
| --- | --- |
| Kitty (APC) | **Removed** |
| iTerm2 format, BEL or ST terminated | Intact |
| iTerm2 format with 300 KB and 2 MB payloads | Intact, byte count exact |
| Sixel (DCS) | **Removed** |
| Kitty wrapped for tmux passthrough (DCS) | **Removed** |
| OSC 8 hyperlink (control) | Passes; conhost adds an `id=` |

WezTerm's multiplexer works on the same host because WezTerm installs its own
`conpty.dll` and `OpenConsole.exe` (`C:\Program Files\WezTerm\`) and does not
use the system console host.

### Binary cost of Sixel (macOS arm64, stripped release, 2026-10-20)

Adding `icy_sixel` 0.7 to a minimal `image` 0.25 program: 5,847,328 →
5,931,584 bytes, **+82 KB** (about 0.4% of the installed `bt`). It pulls in
`quantette` and its dependencies, which cost mostly compile time.

In the same test, Kitty output for a 400×400 icon was 1.1 MB, against 110 KB
as Sixel, which suggests our PNG encoding uses fast, low-compression settings.

## Goals

Each supported setup has one testable outcome. Under tmux, a positively
established native Sixel path takes precedence over the passthrough cases
below; this keeps images attached to the pane.

| Setup | Outcome |
| --- | --- |
| Direct, local terminal | Unchanged from today |
| Zellij 0.45+, graphics enabled and a working host path | Kitty after a successful pane query (until the probe ships: under a Kitty-family host identity, as today), ordinary placement; never the iTerm2 format |
| Zellij before 0.45, or Kitty support disabled | Sixel if Zellij reports it, otherwise fallback |
| WezTerm multiplexer domain | Unchanged from today (works) |
| tmux, Kitty or Ghostty outside, passthrough on | Kitty with Unicode placeholders; images follow the pane |
| tmux, WezTerm outside, passthrough on | Kitty through passthrough, ordinary placement |
| tmux, iTerm2 outside, passthrough on | iTerm2 format through passthrough |
| tmux built with Sixel, Sixel-capable outside | Sixel handled by tmux itself |
| tmux, passthrough off | Native Sixel when positively supported; otherwise fallback plus a hint naming `set -g allow-passthrough on` |
| SSH from a capable terminal, no forwarded hints | Graphics when a safe probe verifies a usable format; otherwise fallback with an unknown-route reason |
| SSH into a Windows server (system conhost) | iTerm2 format when the outer terminal is on the iTerm2 list; otherwise fallback. Never Kitty or Sixel. |
| mintty | iTerm2 format |
| Windows Terminal with Sixel support established | Sixel; `WT_SESSION` alone does not establish version support |
| Terminals without a supported pixel format, including Terminal.app and system conhost | Diagram source or the plain-image text fallback; no graphics probe to known incompatible terminals |
| Any setup, valid `BISCUIT_GRAPHICS` set | Selected format or disabled graphics, subject to output policy and a usable transport; never silently substitute another pixel format |

## Non-goals

- Restoring images after tmux detach/reattach, a fresh tmux client, or a
  cterm daemon reattach. A finished process cannot send its images again.
- Kitty animation, shared-memory or file transfer media.
- More than one level of nested tmux.
- tmux inside Zellij, or Zellij inside tmux. These need an adapter in the
  multiplexer itself.
- iTerm2 multipart (`MultipartFile`/`FilePart`) until someone hits tmux's
  message-size limit; WezTerm and cterm do not parse it.
- Switching the Mermaid renderer to Merman. Likely, but separate work; this
  spec only leaves room for the Unicode text fallback Merman would enable.

## Package boundaries

**`biscuit-visualized` decides what to show; `biscuit-terminal` decides how it
reaches the screen.**

| | `biscuit-terminal` | `biscuit-visualized` |
| --- | --- | --- |
| Owns | Terminal detection and capabilities; the graphics route; displaying pixels (`TerminalImage`, the Kitty, iTerm2, Sixel, and half-block encoders, tmux wrapping, placement) | Diagrams for every target: the diagram components (`MermaidDiagram`, `GraphExpression`, `GitGraph`) and their terminal, browser, and tree rendering |
| Terminal output | Turns pixels into escape sequences for the route | Asks `biscuit-terminal` for the route and cell geometry, chooses what to show (pixels, later a Unicode drawing, or the source), renders at the display size, and hands pixels to `TerminalImage` |
| Browser output | — | SVG, as today |
| Never | Knows about diagrams | Writes escape sequences or knows about formats and routes |

`TerminalImage` stays in `biscuit-terminal` because plain images are displayed
without any diagram involved (biscuit-icon, claudine, darkmatter), and because
sending pixels depends on the route (the placeholder grid needs tmux's outer
terminal; the Windows SSH rule changes the format).

**The dependency direction flips:** `biscuit-visualized` optionally depends on
`biscuit-terminal`, and `biscuit-terminal` drops `biscuit-visualized` but retains
optional `resvg` for plain SVG images and graphical rules. Packages that depend
on `biscuit-terminal` stop compiling the Mermaid renderer unless they use
diagrams.

> **Reader's note:** removing `resvg` as originally proposed would also remove
> unrelated SVG behavior. The terminal package's
> [SVG image loader](../../lib/src/components/terminal_image/mod.rs) and
> [graphical rules](../../lib/src/components/horizontal_rule/mod.rs) use it
> directly. Keeping that dependency preserves these public features while the
> expensive diagram dependency moves to its actual consumers.

```mermaid
flowchart LR
    subgraph before[Today]
        T1[biscuit-terminal] --> V1[biscuit-visualized]
    end
    subgraph after[After this spec]
        V2[biscuit-visualized] --> T2[biscuit-terminal]
    end
```

## Design

### 1. Route model

Replace the single answer with a **route**: what the image passes through to
reach a renderer. A route records:

- **format**: Kitty, iTerm2, Sixel, or none
- **transport**: direct, or wrapped for tmux passthrough
- **placement**: ordinary, or Unicode placeholders (Kitty only)
- **confidence**: forced, verified by probe, known terminal, or unknown
- **reason**: a human-readable explanation, including the fix when one exists

The terminal package's
[`ImageSupport` and `image_support()`](../../lib/src/discovery/detection/image.rs),
which expose supported pixel formats, become projections of the route. Add a
`Sixel` variant and update exhaustive matches, serialization tests, and JSON
consumers together. This is an intentional source-level API change; mapping
Sixel to `None` would tell consumers to suppress a working image route.
Half-blocks remain a text fallback and do not count as image-protocol support.
The route is authoritative: update the public capability field and render-tree
snapshot together, including caller-supplied terminal snapshots. Do not resolve
from the environment again for every image. `TerminalImage` consumes the route,
and diagram components read it to choose what to show; CLIs never make format
decisions.

Resolution order (the diagram shows automatic capability discovery; output
policy and usable transport still constrain a forced format):

```mermaid
flowchart TD
    A[stdout is a terminal?] -->|no| N[none]
    A -->|yes| O{BISCUIT_GRAPHICS set?}
    O -->|kitty, iterm2, sixel, none| F[forced route]
    O -->|invalid| N
    O -->|unset or auto| W{Windows and SSH_CONNECTION?}
    W -->|yes| WI{outer terminal on iTerm2 list?}
    WI -->|yes| I2[iTerm2 format, direct]
    WI -->|no| N
    W -->|no| M{multiplexer}
    M -->|Zellij| Z[Pane query verifies Kitty or Sixel, else none]
    M -->|tmux| T[tmux route, section 3]
    M -->|none| K{known terminal from env or LC_TERMINAL?}
    K -->|yes| KT[known route]
    K -->|no| P[probe, section 2]
```

**Format choice for a known terminal:** pick the format the endpoint handles
reliably, not everything it advertises.

| Terminal | Format |
| --- | --- |
| Kitty, Ghostty, WezTerm, Warp, Konsole, wast | Kitty |
| iTerm2, mintty, cterm | iTerm2 format |
| Windows Terminal (`WT_SESSION`) | Sixel only with a verified capability or a known supporting version |
| Terminal.app | none |

The **iTerm2 list** used by the Windows SSH rule and tmux is: iTerm2,
WezTerm, Konsole, mintty, cterm. The iTerm2 format is chosen only by terminal
identity, never by a generic graphics probe, because no reliable query exists
for it. These terminal names describe candidate formats; a blocked transport
still rules them out. Generic `xterm-256color`, a local `WT_SESSION` inherited
by a remote shell, and a multiplexer server's old environment do not prove the
identity of the active outer terminal. On conflicts, current client information
wins over inherited hints; if it remains ambiguous, use a safe probe or fallback.

#### Overrides and existing graphics policy

`BISCUIT_GRAPHICS` accepts exactly `auto`, `kitty`, `iterm2`, `sixel`, or
`none` (lowercase). Unset and `auto` use detection. An invalid value selects no
pixel route and records the accepted values in the reason; `bt` reports a
configuration error instead of silently selecting a different format.

A forced format skips format probing and terminal-identity preferences, but
still uses detected tmux wrapping and format-appropriate placement. It does
not enable tmux passthrough, lift a known message-size limit, or promise that
an incompatible endpoint will render it. Known impossible transports return
no pixel route with the requested format recorded in the reason. Thus forced
Kitty can test a newer Windows console, once that transport is explicitly known
to pass APC, without changing the default for the measured system conhost.

Existing rendering policy remains a ceiling. In the shared `renderable`
package, [`GraphicsMode`](../../../renderable/src/tree/mod.rs) controls whether
raster output is allowed, and `TerminalMermaidMode` independently opts Mermaid
fences into diagrams. `Off` permits text descriptions/source only; `Vector`
does not permit Sixel, Kitty, iTerm2, or raster-derived half-blocks. `Rich`
permits graphics but does not automatically promote Mermaid fences.
`BISCUIT_GRAPHICS=none` disables pixel protocols and half-blocks, leaving alt
text or source. The terminal renderer's existing
[`force_graphics`](../../lib/src/render_tree/options.rs) capability override
continues to work when the environment selection is `auto`; it cannot override
an explicit `none`, a lower fidelity ceiling, nonterminal stdout, or a blocked
transport. Rendering into a pipe never sends graphics controls or live queries.

A route describes capabilities, not every image's success. Distinguish no
support, unknown/probe unavailable, user-disabled graphics, blocked transport,
and invalid input. Only name a configuration fix when its blocker was actually
observed; do not suggest enabling passthrough for every failed probe.

### 2. Probe, only when the environment cannot decide

The probe runs when environment and route information cannot decide, including
an unknown SSH endpoint or Zellij whose active graphics configuration is unknown.
Known compatible direct terminals keep their current path without a probe.
Known incompatible terminals, CI, redirected stdout, explicitly disabled
graphics, and Windows SSH paths known to strip the query are never probed.

- Use a valid one-pixel Kitty query (`a=q`, direct data, fresh nonzero image
  ID, dimensions and pixel format matching its payload). Accept only a complete
  `OK` response with that ID; a rejection is distinct from silence. See the
  [Kitty query contract](https://sw.kovidgoyal.net/kitty/graphics-protocol/#querying-support-and-available-transmission-mediums).
- Parse DA1 structurally: Sixel is parameter `4` in a primary device-attributes
  reply, not the digit `4` anywhere in the input. Prefer a correlated Kitty
  success over Sixel when both are usable, subject to the tmux rule below.
- Use the terminal package's existing
  [query transport](../../lib/src/discovery/tty_query.rs) for locking, raw-mode
  restoration, bounded reads, deadlines, and silence budgets. Write queries
  through the controlling terminal, never through captured stdout. Query only
  when that terminal is the output endpoint and the caller owns input; never
  from rendering a component, a running TUI event loop, or a synchronized
  output block (mode 2026). A TUI supplies a previously resolved route.
- The existing engine is Unix-only, discards typeahead, strips the DA1 reply
  before returning, and appends an unwrapped DA1. It cannot be reused unchanged:
  graphics discovery needs structured query and DA1 results and a caller-chosen
  transport for **both** requests. Preserve existing color/cursor behavior;
  extending graphics probing does not enable every other probe inside a mux.
- Through tmux, send the query and its ordering sentinel through the same
  passthrough route. A DA1 answered by tmux can establish tmux-native Sixel;
  it cannot order an outer Kitty response. Keep native and outer results
  separate. Lost or out-of-order responses end at the existing deadline and
  yield unknown rather than a confident negative. Wrap no request twice.
- Cache stable probe results per process and output route, not globally across
  different transports or caller-supplied terminal snapshots. Skipping because
  input is busy is not a permanent negative capability result. Cell geometry
  is refreshed for each render and is not part of this capability cache.
- **Never automatically selects Kitty for iTerm2.** Reliable iTerm2 hints
  select its native format before probing. Ordinary Kitty transfers use quiet
  mode and nonzero IDs; probing owns and consumes its correlated replies.
- Native Windows currently has no live-query backend. It uses verified version
  information or known terminal hints, then text fallback when insufficient;
  `WT_SESSION` alone is insufficient. A native Windows probe is an author
  decision in Open questions, not an implicit Unix API port.

> **Reader's note:** this intentionally broadens the current environment-only
> graphics detection contract, which avoids probes because of visible query
> leakage and replies reaching the shell. The new path must remain bounded and
> input-owned. Safe handling of unrelated input is unresolved below; the new
> automatic probe must not be enabled until that decision is implemented.

### 3. tmux

- **Detect** tmux from `TMUX`; read `allow-passthrough` for the pane
  (`tmux show-options -Apv -t "$TMUX_PANE" allow-passthrough`).
- **Identify the outer terminal** from the attached client
  (`#{client_termname}`, `#{client_termtype}`) or `tmux show-environment`.
  Query the client associated with the target pane; a server can have clients
  using different terminal applications. Do not choose an arbitrary attached
  client or assume the server's startup environment is current. If clients
  disagree, use a format all relevant clients can handle or fallback.
  Without socket access, use a safe probe only when the transport is known;
  absent `TMUX` across SSH does not establish that the path is direct.
  Run read-only tmux commands without shell interpolation, with a deadline.
  A missing executable, inaccessible socket, unsupported option, or command
  failure produces unknown, not passthrough-off.
- **Wrap** each complete escape sequence as
  `ESC P tmux; <sequence with every ESC doubled> ESC \`. Each Kitty chunk is
  wrapped separately.
- **Placement:**
    - Kitty or Ghostty outside: upload through passthrough with a virtual
      placement (`U=1`) and print the Unicode placeholder grid (`U+10EEEE`,
      image ID in the foreground color, row/column combining marks) as
      ordinary text through tmux. Used only when the outer terminal is
      positively identified; Zellij rejects `U=1` and WezTerm ignores it.
      Require Unicode and the color fidelity needed to encode image IDs;
      otherwise use ordinary placement with its documented limits. Allocate
      distinct nonzero image IDs for live uploads, and reset placeholder colors
      after the grid so subsequent text is unaffected.
    - WezTerm or unidentified outside: ordinary placement through
      passthrough. Known limits (drift on resize, window switch, copy mode)
      are documented.
    - iTerm2 outside: iTerm2 format through passthrough.
- **Sixel handled by tmux itself:** when tmux was built with Sixel support
  and the outer terminal handles Sixel, prefer Sixel unwrapped, since tmux then
  redraws it with the pane. Prefer this native Sixel route over passthrough
  when it is positively established for the pane and endpoint, even if Kitty
  is also available. Otherwise prefer the reliable Kitty/iTerm2 route. A build
  feature alone or a DA1 answer from the outer terminal is insufficient to
  prove tmux's native Sixel path.
- Mode 2026 is never wrapped for passthrough; tmux handles it itself.
- Passthrough off blocks tunneled graphics, not native Sixel. Use native
  Sixel if established; otherwise fallback and name the setting.
- Before emitting an iTerm2 message, account for base64 expansion, metadata,
  ESC doubling, and the tmux envelope against the known parser limit. Where
  the installed tmux limit cannot be read, use the conservative 1 MiB limit
  for supported modern tmux; unverified older parsers use fallback. An oversized
  image falls back with a size-limit reason before reserving rows. Never split
  a legacy `File=` message into arbitrary OSC messages. Multipart remains out
  of scope; see [iTerm2's framing and limits](https://iterm2.com/documentation-images.html).

### 4. Zellij

- Kitty with ordinary placements only. Version 0.45+ establishes eligibility,
  not enablement: query the pane endpoint to establish active Kitty support.
  Never infer it from an inherited outer `TERM_PROGRAM` alone; never use tmux
  wrapping, placeholders, or the iTerm2 format.
- **Until the probe is enabled** (see the input-ownership open question),
  Zellij 0.45+ under an inherited Kitty-family identity keeps today's
  behavior: Kitty, ordinary placement, confidence "known terminal". Zellij
  enables its Kitty support by querying a Kitty-capable host, so that identity
  is reasonable interim evidence, and it is what works today. Without this,
  phase 3 would turn working Zellij sessions into fallback. The pane query
  replaces the interim rule once it ships.
- Earlier versions, unknown versions, or Kitty disabled: use a positive Sixel
  reply when available, otherwise fallback. Do not inspect a local config file
  as proof of the running server's state. Zellij's
  [graphics option](https://zellij.dev/documentation/options.html#support_kitty_graphics_protocol)
  depends on the host and requires a restart after changes.
- A query success does not establish that Zellij knows its host's cell size.
  Missing geometry or a placement failure gets a distinct reason and fallback;
  the library's estimated cell size does not fix Zellij's internal measurement.

### 5. Sixel

- Encoder: `icy_sixel`, no cargo feature flag.
- Detection: parameter `4` in a DA1 reply from the relevant renderer, or
  verified version support for Windows Terminal. `WT_SESSION` identifies the
  application, not the Sixel capability of every installed version.
- Sized by the rule in section 7. Sixel has no partial transparency:
  diagrams are rendered opaque on the terminal's background by
  `biscuit-visualized`; plain images are composited against that background
  by `TerminalImage`. If the measured background is unavailable, use the
  existing light/dark theme's opaque background and expose that estimate in
  diagnostics. Quantization uses a bounded palette and deterministic settings;
  crisp diagrams should not gain dithering noise by default.
- Preserve terminal mode state. Do not leave scrolling or palette-related
  private modes changed after an image; use the existing cursor ownership rules
  and test output at the bottom of the viewport.
- Never selected automatically for the measured Windows SSH system-conhost path.

### 6. Fallback and hints

- **Diagrams** keep the source code block they produce today. The choice
  belongs to the diagram components in `biscuit-visualized`, so a Unicode
  drawing (expected with Merman) can later slot in between graphics and the
  source without touching `biscuit-terminal`.
- **Plain images** (`bt image`, `TerminalImage`) render as half-blocks
  (`▀` with truecolor foreground and background), degrading to alt text (the
  file name) without truecolor or Unicode support. Explicit alt text wins over
  the file name. Sanitize descriptions and file names as literal text so they
  cannot inject terminal controls. With nonterminal stdout, output a plain text
  description and never color blocks.
- **Hint:** when the route's reason has a fix, the `bt` CLI writes one line
  to stderr, only when stderr is a terminal, at most once per distinct reason
  per command invocation. The library returns structured reasons without
  writing hints as a side effect. JSON output stays parseable and includes the
  reason as data, not a diagnostic mixed into stdout.
- The capability report (`bt` with no arguments, `--json`) shows the route
  and its reason, not just `Images: None`; retain the existing image-support
  field while adding the route fields.
- Resolve/load/encode before reserving rows or emitting image bytes. Once a
  write partially succeeds, return the I/O failure; do not append another pixel
  format to an incomplete control sequence. A valid image with no usable route
  gets fallback; invalid paths, policy violations, and corrupt images remain
  errors under existing error/strictness rules. Preserve the terminal package's
  image access policy and the shared render tree's existing placeholder style.
  The new plain-image half-block default does not bypass those explicit policies.

### 7. Sizing and the double-encoding fix

- **Render at the final display size.** The layout decides an image's width
  (a percentage, a character count, natural width, margins; rarely the full
  terminal width). Diagrams are rasterized at exactly that width in pixels
  (the visualized package's
  [`RenderRequest::target_width`](../../../biscuit-visualized/src/src/artifact.rs)).
  Preserve aspect ratio: compute height from that width, round the reserved
  cell rectangle up, and pad unused pixels rather than stretching the image.
  Reuse the terminal package's existing
  [image width resolver](../../lib/src/components/terminal_image/width.rs),
  which gives `ImageWidth` precedence and does not read `Layout::width` or
  `Layout::max_width`. Respect the parent's available width and indentation.
  Measure cell pixels when available, otherwise use the existing documented
  cell estimate; never use zero dimensions. Recompute after resize and enforce
  existing decode/allocation limits before allocating or reserving rows.
- **Fix: send diagram PNGs without re-encoding.** When the rendered size
  matches the display size, the PNG bytes from `biscuit-visualized` go to the
  Kitty/iTerm2 encoder unchanged, with pixel dimensions and cell footprint
  carried alongside them. Use an encoded-PNG input path in `TerminalImage`
  instead of handing it an already-decoded image. Existing path-based image
  validation still applies; the internal artifact path is not a bypass for
  caller-supplied files. Decoding is reserved for formats that need pixels
  (Sixel, half-blocks), validation that actually requires it, or an actual
  resize. Apply this to inline and standalone rendering; both currently have
  separate protocol dispatch paths.
- **Background with three meanings.** Replace the visualized package's
  `RenderRequest::transparent_background` with an explicit choice of theme
  background, transparent, or solid RGB. Preserve today's default theme
  background for exports; terminal adapters request transparency for Kitty and
  iTerm2 or a solid terminal background for Sixel. Apply the choice consistently
  to SVG and PNG and update all request literals. An optional color alone would
  conflate theme defaults with transparency and change existing exports.
- Include resolved background, target dimensions/padding, and encoding options
  in artifact cache identity; invalidate old artifacts when these semantics
  change. Keep using `biscuit-hash` for cache keys. Palette and display choices
  must not reuse an artifact prepared for a different background or geometry.

### 8. Moving the diagram components

- `MermaidDiagram`, `GraphExpression`, and `GitGraph`, with their tests and
  component docs, move from `biscuit-terminal` to `biscuit-visualized`.
- `biscuit-visualized` gains a dependency on `biscuit-terminal`;
  `biscuit-terminal` drops `biscuit-visualized` and the diagram half of its
  `image` feature, retaining `resvg` for SVG images and graphical rules.
  Make the new terminal dependency optional in `biscuit-visualized` behind a
  `terminal` feature, so browser/artifact-only users do not pull in terminal
  detection. That feature enables the existing visualization `image` support
  and the terminal package's `image` feature; neither package enables a feature
  that creates a reverse dependency. Preserve existing `clap` feature gating.
- **Render-tree hook.** The terminal render-tree renderer can no longer call
  `MermaidDiagram`. It gains a diagram-rendering hook (a trait object on the
  render options) that `biscuit-visualized` implements; with no hook
  installed, a Mermaid fence stays a code block, as with
  `TerminalMermaidMode::Code` today. Callers that want diagrams
  (darkmatter, the `bt` CLI) install it.
  Use the existing `Rc` hook ownership style in the terminal package's
  [render options](../../lib/src/render_tree/options.rs). The hook receives
  source, node attributes, effective terminal/route, available width and layout;
  it returns rendered output or a typed error. It is consulted only for Mermaid
  promotion at `Rich`, before the ordinary code-rendering hook, matching today's
  ordering. Preserve strict/warn/lossy error handling; a missing hook falls
  through to code rendering. Cloning child render options retains the hook.
  The adapter renders directly rather than recursively promoting its own fence.
- Consumers update imports: the `bt` CLI diagram commands, darkmatter,
  sniff-cli, worktree (lib and CLI). Search the entire workspace for diagram
  imports, prelude re-exports, examples, doctests, and diagram-only font/raster
  helpers; migrate every caller rather than assuming this list is exhaustive.
  Keep the existing lower-level artifact API distinct from the moved component
  names (for example expose the latter under `biscuit_visualized::components`),
  so two `MermaidDiagram` types do not collide. No terminal-crate re-export can
  remain, because that would restore the dependency cycle.
- Behavior is unchanged by the move itself; existing tests pass from their
  new location.

### 9. Housekeeping

- Correct the `protocol.rs` comment on why iTerm2 uses its own format.
- Review PNG encoding settings for transfer size in both packages. The existing
  size figures justify inspection, not a conclusion that one codec is always
  smaller. If a compression trade-off remains uncertain, use one host and a
  quick sample; no performance matrix or statistical gate is required. The
  measured +82 KB Sixel cost already answers the binary-size question, so do
  not add a size spike.
- Remove `viuer` if it is unused.

## Phases

| Phase | Content | Depends on |
| --- | --- | --- |
| 0. Evidence | Visual confirmation of the Windows SSH rule; whether conhost answers DA1 itself and claims Sixel; Ghostty placeholder support; cell pixel size inside tmux and Zellij; what Kitty and Ghostty do with an ordinary placement when tmux switches windows; verification of the native-Sixel preference and tmux message limits | — |
| 1. Component move | Section 8: diagram components to `biscuit-visualized`, dependency flip, render-tree hook, consumer imports. No behavior change. | — |
| 2. Sizing and double encoding | Section 7: render at display size, PNG pass-through fix, background color in `RenderRequest` | 1 |
| 3. Route model | Route type, resolution order, `BISCUIT_GRAPHICS`, Windows SSH rule, mintty, Windows Terminal identity, Zellij gating, capability report, stderr hints, housekeeping | 1 |
| 4. Probe | Kitty query plus DA1 through the query engine | 3 |
| 5. tmux transport | Passthrough detection, outer-terminal identification, wrapping, ordinary placement, iTerm2 format through tmux | 3, 4 |
| 6. Placeholders | Unicode placeholder encoder and its use under tmux for Kitty and Ghostty | 2, 5 |
| 7. Sixel | Encoder, detection, tmux's own Sixel handling | 2, 3, 4 |
| 8. Half-blocks | Half-block fallback for plain images | 3 |

The move comes first so that every later change to diagram behavior is
written once, in its final package. Phase 1 preserves behavior after consumer
migration. Phase 2 preserves aspect
ratio and intended footprint; padding, terminal backgrounds, and removing
resampling can visibly improve pixels. Phase 3 preserves the working direct
routes, while fixing Windows SSH blank rows and correcting blocked multiplexer
routes. Probe enablement waits for the input-safety decision below.

## Verification

- **L1:** the resolution order as an environment-variable matrix (every row
  of the Goals table that environment alone decides); tmux wrapping
  including ESC doubling and nested terminators; placeholder grid encoding;
  Sixel output for fixed fixtures; half-block output; the probe's reply
  parser over fragmented and out-of-order replies; diagram PNG bytes reach
  the encoder unchanged when sizes match; a render tree without the diagram
  hook leaves Mermaid fences as code blocks. Include policy precedence,
  invalid overrides, the new Sixel capability variant, known-incompatible
  terminals that must not be queried, Zellij enabled/disabled state independent
  of version, native versus tunneled DA1 results, busy-input skips, and tmux
  message-limit boundaries. Assert aspect ratio, padding, background cache
  separation, inline/standalone dispatch parity, and existing SVG/rule behavior.
- **Move:** the moved components' existing tests pass unchanged in
  `biscuit-visualized`, and `cargo tree -p biscuit-terminal` no longer
  contains `biscuit-visualized` or the Mermaid renderer, including with its
  `image` feature enabled. Optional `resvg` remains expected. Check that
  artifact-only `biscuit-visualized` does not pull in `biscuit-terminal`, and
  that enabling terminal support introduces no cycle.
- **L2:** tmux through the shared test harness, with passthrough on and off;
  Zellij where available; WezTerm.
- **Manual, recorded:** iTerm2 (iTerm2 format and no reply leak), Windows SSH
  server from WezTerm (iTerm2 format renders), Windows Terminal (Sixel),
  mintty (iTerm2 format), Ghostty under tmux (placeholders survive resize and
  window switch). Results are kept as dated evidence with terminal and
  multiplexer versions. These are behavioral checks, not performance spikes.
- Use the repository's nextest-backed `just test` / `just test-l2` recipes and
  shared Test Toolkit. Register moved tests in their new test targets and retain
  tier/feature gating. Level 2 terminal windows must never gain focus. Exercise
  platform-independent route/encoder logic on macOS, Linux, native Windows,
  and WSL2 through the normal test scope; distinguish compile evidence from
  live terminal behavior.

## Documentation

- A new `docs/` topic page on how `biscuit-terminal` chooses a graphics route,
  the route table, `BISCUIT_GRAPHICS`, what each hint means, and the known
  limits under tmux. Written for a developer new to the repository; it links
  to the research, not to this spec.
- `docs/components/terminal_image.md` updated for the route model and the
  half-block fallback. Update the live-query topic page for the intentional
  graphics-probe exceptions, input ownership, and platform limits. Document
  unbuilt behavior as planned until its implementation lands.
- The component docs for `MermaidDiagram`, `GraphExpression`, and `GitGraph`
  move to `biscuit-visualized/docs/`, along with
  `docs/data-visualization/` pages that describe them.
- Both packages' `docs/dependencies.md` for the dependency flip, `icy_sixel`,
  and `viuer` if removed; both READMEs where they describe what each package
  does.
- The `os` skill (`windows-console.md`): Windows OpenSSH's system conhost
  removes APC and DCS sequences and passes OSC; WezTerm ships its own ConPTY.
- The `biscuit-terminal` and `biscuit-visualized` skills for the route model
  and the new package boundary.

## Risks

- **Width-only Kitty placement** failed in iTerm2. iTerm2 no longer receives
  Kitty, but other terminals with partial Kitty support may behave the same
  way; sending both `c=` and `r=` everywhere is the cheap insurance if one
  turns up.
- **Windows console behavior may change.** A newer system conhost may stop
  removing these sequences; the Windows SSH rule is written against observed
  behavior and re-checked, not treated as permanent.
- **Placeholder cells after a fresh tmux client** render blank because the
  image data is gone; documented, not fixed.
- **Probe false positives** where an intermediary answers DA1 itself.
- **The move touches several packages** (the `bt` CLI, darkmatter, sniff-cli,
  worktree), which widens this spec's CI scope. Keeping phase 1 free of
  behavior changes keeps its review mechanical.
- **The render-tree hook is a new public seam.** A caller that forgets to
  install it silently gets code blocks instead of diagrams; the capability
  debug trace records Mermaid promotion requested without a hook; a terminal
  capability report cannot know which hooks an application installed.

## Open questions

### Who owns input while graphics discovery runs?

The existing query engine consumes unrelated input, and a mutex between queries
does not coordinate with an application's event reader. Automatically invoking
it from terminal construction could lose keystrokes or leave a late response at
the shell. Choose this contract before enabling the new automatic probe.

1. **Explicit startup discovery with an input owner (recommended).** CLI entry
   points run discovery before reading input; TUIs provide their own resolved
   route or perform discovery through their event reader. Return unrelated
   bytes to the input owner rather than dropping them. Pros: input ownership is
   visible, works with existing TUI readers, and keeps rendering free of I/O
   discovery. Cons: participating entry points need a startup call, and generic
   terminal construction remains environment-only. Recommended because it
   offers automatic behavior in the shipped CLIs without assuming control of
   every library consumer's keyboard input.
2. **A shared terminal-input broker.** Every query and application reader uses
   one dispatcher. Pros: routes late replies and preserves typeahead centrally.
   Cons: requires migration of readers beyond these packages and adds a larger
   lifecycle/API change than graphics routing itself.
3. **Keep unknown terminals environment-only.** Pros: no new input consumption
   or query leakage. Cons: unknown SSH endpoints continue to need an explicit
   format, so the automatic-discovery goal is reduced.

### How should remote programs declare a hidden tmux layer?

A program reached over SSH may have no `TMUX` or socket access even though the
client is inside tmux. A format override alone cannot express the needed wrapper.
No reliable local inference can reconstruct that missing path information.

1. **An independent transport override (recommended), such as
   `BISCUIT_GRAPHICS_TRANSPORT=auto|direct|tmux`.** Pros: describes the missing
   fact without overloading format choice; can be forwarded explicitly over SSH.
   Cons: another setting and no automatic proof that passthrough is enabled.
   Recommend it because it keeps format and transport independent. Treat a
   forced tmux route as unverified, require ordinary placement unless outer
   identity is established, and report blocked/unknown honestly.
2. **Combined format values such as `kitty-tmux`.** Pros: one variable.
   Cons: duplicates combinations, complicates parsing, and conflates placement
   and transport with pixel format.
3. **Document hidden tmux as unsupported.** Pros: smallest implementation.
   Cons: the opening SSH guarantee must explicitly exclude this topology.

### Should native Windows gain live graphics queries in this feature?

The current query transport uses Unix file descriptors and `/dev/tty`.
Windows Terminal identification alone cannot establish whether an older build
supports Sixel. The measured system-conhost SSH route must still avoid APC/DCS.

1. **Keep native Windows detection conservative for now (recommended).** Use
   known supporting version evidence when available, explicit overrides, and
   useful fallback otherwise. Version evidence does not need a live query:
   on native Windows (not over SSH), the terminal's executable can be found
   by walking the parent process chain to `WindowsTerminal.exe` and reading
   its file version (1.22 or later supports Sixel). Without this, Windows
   Terminal, the main reason Sixel joined this spec, would get Sixel only
   through `BISCUIT_GRAPHICS`. Pros: bounded scope and preserves the existing
   no-live-query contract. Cons: an unidentified capable Windows terminal may
   need an override. Recommended because the motivating Windows SSH route uses
   terminal identity and cannot safely rely on the stripped queries anyway.
2. **Add a native Windows query backend now.** Pros: supports automatic
   discovery in more local Windows sessions. Cons: needs console-handle input
   ownership, mode restoration, and behavior tests distinct from Unix; it is a
   substantive expansion of this feature.
3. **Assume Sixel whenever `WT_SESSION` exists.** Pros: trivial detection.
   Cons: misclassifies older versions and inherited environments, recreating
   blank-row failures. Not recommended.
