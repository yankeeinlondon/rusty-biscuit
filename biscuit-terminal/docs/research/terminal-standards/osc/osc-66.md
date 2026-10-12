---
$schema:
    description: string(required)
    apps_supporting: string[](required)
description: >-
    Kitty’s OSC 66 protocol lets terminal applications request larger or fractional text and assign explicit cell widths to text runs. Support varies by emulator, and some implementations provide only the width feature.
apps_supporting:
    - kitty
    - Foot
    - Contour
prompt: |-
    The OSC 66 informal standard is Kitty's text sizing protocol, which lets a program render text at multiple cell sizes (e.g., headings) and explicitly declare the cell width of a run of text.

    Your task is to research this informal standard and:

    - provide a detailed overview of the standard.
        - How it works.
        - Whether it can be tested for.
        - What visual representation does it provide? How does that vary by terminal app?
        - How does the explicit width declaration interact with Unicode width tables and wide or ambiguous-width characters?
        - Where and when it was introduced? How well it is supported? Is there continuing adoption?
    - which terminal apps support it currently?
        - Are there any known quirks for any of the apps that a developer should be aware of? How can these quirks be worked around?
        - Note: be sure to include _at least_ the following terminal apps:
            - Wezterm
            - Ghostty
            - iTerm2
            - cmux
            - kitty
            - konsole
            - AppleTerminal
            - Alacritty
            - Contour
            - Foot
            - GNOME Terminal
            - more is better so add others which have readily available info

    Once you've completed your research and have written the prose content to the body of this document, you will need to update the Frontmatter properties:

    - set `apps_supporting` as a list of those apps which are known to support OSC 66
    - set `description` as a one to two sentence description of this standard
hash: 256f73f3b35a2ac6-811fee56e689702a
last_updated: 2026-09-28
---
## Overview

OSC 66 is Kitty's informal text-sizing protocol. It extends the terminal's normal fixed-cell grid with “multicell” text runs: the application asks the terminal to allocate a rectangular group of cells and render UTF-8 text inside it. Terminals that do not implement the sequence generally ignore it as a control string, so ordinary text remains usable, but the requested style and width behavior are lost. The protocol was added in Kitty 0.40.0 and publicly proposed on January 18, 2025; it is not an ECMA/ISO standard. [Kitty specification](https://sw.kovidgoyal.net/kitty/text-sizing-protocol/), [Kitty RFC](https://github.com/kovidgoyal/kitty/issues/8226)

A sequence has the form `ESC ] 66 ; metadata ; text BEL` or uses `ESC \` (ST) instead of BEL. Metadata consists of colon-separated `key=value` pairs. Text is escape-safe UTF-8, limited to 4096 bytes per sequence. The principal keys are `s` (integer scale 1–7, default 1), `w` (assigned width in cells, 0–7, default 0), `n` and `d` (fractional scale numerator and denominator, with `d > n` when nonzero), and `v`/`h` (vertical/horizontal alignment: top/left 0, bottom/right 1, center 2). At integer scale `s`, a run occupies `s` rows and `s*w` columns. With `w=0`, the terminal segments the payload as normal text and expands each cell into an `s`-by-`s` region. With nonzero `w`, all payload text in that sequence must fit the requested width; producers should split text into appropriately sized chunks. Fractional sizing reduces the glyph rendering size within the allocated cell region, enabling superscripts, subscripts, and compact headings. Scale is relative to the user's configured base font size, and terminals may adjust or clip text that cannot fit.

For example, `printf '\033]66;s=2;Larger text\a\n\n'` asks a supporting terminal to draw the run about twice as large and use two grid rows. The visual result is still a terminal font glyph, not an image or a freely positioned rich-text object: Kitty's renderer tiles the larger glyph across cells; other implementations may support only the width override. Size and alignment can therefore look different with different fonts, cell aspect ratios, and implementations. Applications should keep their cursor/layout accounting consistent with the allocated cells and should not assume that visual ink exactly fills the rectangle.

### Width, Unicode, and graphemes

Ordinary terminal programs and emulators often use different Unicode versions, `wcwidth` tables, emoji-presentation rules, and grapheme segmentation. An ambiguous-width character may be one or two cells depending on locale or terminal policy; variation selectors can change emoji presentation; a ZWJ sequence, flag, combining sequence, or newly assigned code point can be measured differently by the two sides. With `w=0`, OSC 66 retains normal terminal width calculation (but at scaled cell dimensions), so it does not resolve those disagreements. With explicit nonzero `w`, the application assigns the cell width for the entire payload, bypassing the terminal's usual width decision for that run. This is useful for a grapheme cluster, icon, or sequence whose width the application has already measured. It does not itself segment Unicode text or shape complex scripts: the sender must choose safe text chunks and cell widths. The Kitty specification recommends using explicit width especially for non-ASCII text and fractional sizing, while allowing efficient `w=0` for ordinary ASCII.

Explicit width is an allocation contract, not a promise that every font has a glyph that visually fits the specified space. Wrong widths can cause padding, clipping, overlap, cursor/layout disagreement, or awkward reflow. Keep grapheme clusters intact where possible, use a Unicode segmentation/width library, and use the width rules of the destination protocol implementation when accuracy matters. Kitty specifically documents the text-presentation selector U+FE0E as a case where explicit width may be needed to avoid width disagreement. [Width and Unicode discussion](https://sw.kovidgoyal.net/kitty/text-sizing-protocol/#fixing-the-character-width-issue-for-the-terminal-ecosystem)

### Detection and testing

OSC 66 has no standard capability query or positive reply. Do not infer support from `TERM`, terminal branding, or the sequence merely disappearing: terminals commonly consume unknown OSC strings without displaying them. A practical probe saves the cursor position, emits a carefully chosen explicit-width payload or scaled space at a safe location, requests cursor position with CPR (`CSI 6 n`), and compares the reported movement; Kitty's documentation also provides `kitten __width_test__`. Test width and scale separately. Such probes can overwrite cells and may wrap or scroll at the edge of the screen, so use a cleared/sacrificial region, restore cursor/screen state, and account for asynchronous CPR replies and multiplexers. A width-only implementation will pass width tests and fail scale tests. Automated databases that merely report “consumed” sequences are measuring parser tolerance, not successful visual rendering.

## Terminal support

Support below distinguishes rendering from parsing/consumption. “Full” means size/scaling and width behavior are implemented; “width-only” means explicit `w` works while scale/alignment do not. Versions are evidence references, not a guarantee that every build or downstream package has the feature.

| Terminal app                  | Status                               | Notes                                                                                                                                                                                   |
|-------------------------------|--------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| kitty                         | Full                                 | Reference implementation, added in 0.40.0. OSC 66 may not pass through kitty's Python batch bridge; test in the actual terminal UI.                                                     |
| Foot                          | Width-only                           | Its control-sequence manual explicitly says only `w` is supported. Treat `s`, `n`, `d`, and alignment as unavailable.                                                                   |
| Contour                       | Advertised full support              | Current project metadata lists Kitty text sizing (multicell text, fractional scale, alignment). Verify the installed release; development/release channels can differ.                  |
| Ghostty                       | Not yet verified as rendered         | The project issue says parsing was added but the renderer still needed to attach metadata to cells and draw it. OSC consumption alone is not support. Recheck later releases.           |
| cmux                          | Not independently verified           | cmux uses Ghostty technology; do not assume the OSC parser implies sized-text rendering. Treat as unsupported until its shipped libghostty renderer documents or passes a visual probe. |
| WezTerm                       | No documented support found          | An open 2026 issue requests OSC 66 behavior for CJK fractional sizing, reporting malformed glyphs in the tested build. Do not enable sizing based solely on identifying WezTerm.        |
| iTerm2                        | No reliable rendering evidence found | Some compatibility listings say the sequence is consumed, but that does not establish width or scale rendering.                                                                         |
| Konsole                       | No reliable rendering evidence found | No primary documentation establishing OSC 66 rendering was found.                                                                                                                       |
| Apple Terminal (Terminal.app) | No reliable rendering evidence found | Compatibility listings conflict and may count ignored/consumed OSCs as support; require a visual and cursor-movement test.                                                              |
| Alacritty                     | No documented support found          | No OSC 66 implementation identified in current feature information.                                                                                                                     |
| GNOME Terminal / VTE          | No documented support found          | No OSC 66 implementation identified; unsupported OSC handling should not be mistaken for rendering.                                                                                     |
| VS Code terminal              | Unclear                              | Third-party compatibility reports claim consumption, but no primary evidence found for sized rendering.                                                                                 |
| Warp                          | Unclear                              | Third-party compatibility reports claim consumption; verify both rendering and width.                                                                                                   |
| Rio                           | No reliable evidence found           | No primary OSC 66 support documentation found.                                                                                                                                          |
| xterm                         | No documented support found          | No OSC 66 implementation identified.                                                                                                                                                    |

The positive claims here are deliberately narrow. Kitty's specification is authoritative for protocol semantics, Foot's manual explicitly limits its implementation to width, Contour's project metadata advertises the feature, and Ghostty's open implementation issue distinguishes parser work from unfinished rendering. [Foot control sequences](https://manpages.debian.org/unstable/foot/foot-ctlseqs.7.en.html), [Contour feature metadata](https://github.com/contour-terminal/contour/blob/master/metainfo.xml), [Ghostty implementation issue](https://github.com/ghostty-org/ghostty/issues/10333), [WezTerm issue](https://github.com/wez/wezterm/issues/7814)

## Adoption and implementation guidance

Adoption is emerging rather than broad. Kitty lists the protocol as added in 0.40.0 and published the RFC on January 18, 2025. The open discussion in Ghostty, Foot's deliberate width-only implementation, Contour's advertised implementation, and new application/library support for emitting and measuring OSC 66 show continuing interest. Most established terminal emulators still lack confirmed rendered support, however, and compatibility reports are inconsistent because they conflate parser acceptance with visible output.

For portable applications, retain a conventional one-cell rendering path and make decorative scale an enhancement. If the main need is stable Unicode cell placement, probe explicit-width support independently and use `w` without assuming fractional or integer scaling exists. Avoid probes in visible user content; capability checks that write spaces are destructive. When sending explicit widths, send short chunks whose total width is known, respect the 4096-byte limit, and test representative wide, ambiguous, emoji, variation-selector, combining, and ZWJ sequences in each supported terminal. Multiplexers and remote shells may filter or transform OSC traffic, so test through the actual deployment path as well as directly.
