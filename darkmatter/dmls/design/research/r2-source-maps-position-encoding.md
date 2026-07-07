---
prompt: |-
  We are building DMLS, the Darkmatter Language Server (Rust, lsp-server +
  lsp-types, LSP 3.17). See @darkmatter/features/2026-07-04-dmls/design.md
  ("Document and Text Model") for the planned source-map module: per-version
  line index with byte offsets and per-line UTF-16 unit counts, plus
  frontmatter-relative range projection.

  Research the position-encoding reality across our four target editors —
  VS Code, Zed, Neovim, and Helix:

  1. Which position encodings does each client negotiate via the LSP 3.17
     `positionEncoding` capability today? Which default to UTF-16 silently,
     which offer UTF-8, and which have known bugs in negotiation?
  2. Catalog the classic failure cases an LSP source map must handle: CRLF
     and lone-CR line endings, multibyte UTF-8, astral-plane characters
     (emoji, CJK extension), combining characters, BOM, and final lines
     without newline. For each, state the correct byte ↔ UTF-16 conversion
     behavior and cite how mature servers (rust-analyzer, marksman,
     vscode-markdown-languageservice) handle it.
  3. Survey how existing Rust LSPs structure their line-index code
     (rust-analyzer's `line-index` crate and alternatives). Should DMLS
     depend on `line-index` rather than writing its own? Compare API shape,
     maintenance, and UTF-16/UTF-8/UTF-32 support.
  4. Describe the standard technique for projecting sub-document regions
     (frontmatter, code fences) into host-document coordinates and the
     off-by-one traps around delimiter lines.

  Deliverables: an editor × encoding matrix, a test-case checklist DMLS
  should encode as unit tests, and a recommendation (with rationale) on
  depending on `line-index` versus a bespoke implementation.
last_updated: 2026-07-06
hash: 5361a3ebbff0d26a-3f51487f90038a42
---
# DMLS Source Map Research: Position Encodings and Range Projection

LSP 3.17 made position encoding negotiable, but UTF-16 is still the compatibility floor. If `general.positionEncodings` is absent or does not mention `utf-16`, servers may still assume UTF-16; if the server omits `capabilities.positionEncoding`, the result defaults to UTF-16. The spec also says conversion needs the file or line text, so it belongs near the document text model. See the LSP 3.17 position encoding rules and default behavior in the spec: [positionEncodings](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initialize) and [Position](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#position).

## Editor Encoding Matrix

| Editor                               | Current advertised `general.positionEncodings` | Practical default                                     | Notes for DMLS                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
|--------------------------------------|-----------------------------------------------:|-------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| VS Code, via `vscode-languageclient` | `["utf-16"]`                                   | UTF-16                                                | The client advertises only UTF-16 and throws if a server returns a non-UTF-16 `capabilities.positionEncoding`. DMLS must return UTF-16 for VS Code. Source: [`client.ts`](https://github.com/microsoft/vscode-languageserver-node/blob/main/client/src/common/client.ts#L1479-L1484), [`positionEncodings = ['utf-16']`](https://github.com/microsoft/vscode-languageserver-node/blob/main/client/src/common/client.ts#L2139-L2144).                                                                                                                                                                                                                                                                                                                           |
| Zed                                  | `[UTF16]`                                      | UTF-16                                                | Current open-source client initialization advertises only UTF-16. Treat Zed like VS Code unless this changes. Source: [`crates/lsp/src/lsp.rs`](https://github.com/zed-industries/zed/blob/main/crates/lsp/src/lsp.rs#L782-L786).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Neovim                               | `["utf-8", "utf-16", "utf-32"]`                | Server-selected; falls back to UTF-16                 | Current Neovim advertises all three and stores the server-selected `positionEncoding` as `offset_encoding`. Historically, Neovim had trouble when multiple clients with different encodings attached to one buffer, and there are reported UTF-8 integration bugs with multibyte text. Sources: [`protocol.lua`](https://github.com/neovim/neovim/blob/master/runtime/lua/vim/lsp/protocol.lua#L343-L352), [`client.lua`](https://github.com/neovim/neovim/blob/master/runtime/lua/vim/lsp/client.lua#L619-L621), [multiple encoding warning](https://github.com/neovim/nvim-lspconfig/issues/2184), [promotion issue](https://github.com/neovim/neovim/issues/30034), [Tinymist UTF-8/Chinese report](https://github.com/Myriad-Dreamin/tinymist/issues/766). |
| Helix                                | `[UTF8, UTF32, UTF16]`                         | Server-selected; invalid or absent defaults to UTF-16 | Helix advertises UTF-8 first, UTF-32 second, UTF-16 third, and maps server `position_encoding` into internal `OffsetEncoding`. It also has a real-world initialization-order trap: diagnostics sent before `InitializeResult` cannot be decoded because the position encoding is not known yet. Sources: [`client.rs`](https://github.com/helix-editor/helix/blob/master/helix-lsp/src/client.rs#L756-L761), [`offset_encoding`](https://github.com/helix-editor/helix/blob/master/helix-lsp/src/client.rs#L413-L427), [early diagnostic discussion](https://github.com/helix-editor/helix/discussions/7466?sort=old).                                                                                                                                         |

Recommendation for negotiation: DMLS should support UTF-8 and UTF-16 now, and UTF-32 cheaply if the chosen index already supports it. Pick the first client-offered encoding DMLS supports, but force UTF-16 when the client offers only UTF-16. Never return UTF-8 to VS Code or current Zed.

## Failure Cases the Source Map Must Handle

| Case                                               | Correct behavior                                                                                                                                                                                                                                                                      | Mature-server behavior                                                                                                                                                                                                                                                                                                                                                                                                        |
|----------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| LF final newline and final line without newline    | Line starts are zero-based. `a\n` has line 0 content `a` and an empty line 1 at offset after `\n`; `a` without newline has only line 0. Ranges that include the newline end at next line, character 0.                                                                                | `vscode-languageserver-textdocument` computes a new line offset after every EOL and clamps positions to valid text. [`main.ts`](https://github.com/microsoft/vscode-languageserver-node/blob/main/textDocument/src/main.ts#L319-L389). `line-index` records offsets after `\n` and has explicit trailing-newline tests. [`tests.rs`](https://github.com/rust-lang/rust-analyzer/blob/master/lib/line-index/src/tests.rs).     |
| CRLF                                               | Treat `\r\n` as one line ending. A position cannot point between `\r` and `\n`; a range including it ends at next line character 0. Internally, byte offsets still need to account for two bytes.                                                                                     | `vscode-languageserver-textdocument` treats CRLF as one EOL and strips EOL when computing line ranges. [`main.ts`](https://github.com/microsoft/vscode-languageserver-node/blob/main/textDocument/src/main.ts#L390-L417). rust-analyzer normalizes CRLF to `\n` and remembers DOS endings for edits. [`line_index.rs`](https://github.com/rust-lang/rust-analyzer/blob/master/crates/rust-analyzer/src/line_index.rs#L1-L55). |
| Lone CR                                            | Treat `\r` as a line ending for LSP compatibility. A source map that only scans `\n` will be wrong on legacy files.                                                                                                                                                                   | VS Code’s text document implementation treats lone CR as EOL. rust-analyzer’s reusable `line-index` scans `\n`; its server wrapper normalizes CRLF but not lone CR, which is acceptable for Rust source assumptions but not ideal for Markdown. DMLS should explicitly test lone CR.                                                                                                                                          |
| Multibyte UTF-8, BMP scalar, e.g. `é`, `β`, `宽`   | Byte column advances by 2 or 3; UTF-16 column advances by 1; UTF-32 column advances by 1. Byte offsets inside the scalar are invalid and should return `None`/error rather than silently map to a position.                                                                           | rust-analyzer’s `LineIndex::try_line_col` rejects offsets in the middle of multibyte characters, and `to_wide` subtracts the UTF-8/UTF-16 width delta. [`line-index`](https://github.com/rust-lang/rust-analyzer/blob/master/lib/line-index/src/lib.rs#L70-L141).                                                                                                                                                             |
| Astral-plane scalar, e.g. emoji or CJK extension B | UTF-8 advances by 4 bytes; UTF-16 advances by 2 code units; UTF-32 advances by 1 scalar. A position after `a💡b` is byte col 5, UTF-16 col 3, UTF-32 col 2.                                                                                                                           | LSP’s example uses this exact UTF-16 surrogate-pair behavior. `line-index` treats 4-byte UTF-8 chars as width 2 in UTF-16 and 1 in UTF-32. [`WideChar::wide_len`](https://github.com/rust-lang/rust-analyzer/blob/master/lib/line-index/src/lib.rs#L45-L67).                                                                                                                                                                  |
| Combining characters, e.g. `e\u{0301}`             | LSP positions count code units, not grapheme clusters. The base `e` is one UTF-16 unit; the combining mark is another UTF-16 unit and two UTF-8 bytes. Positions may legally fall between them, even if UI cursoring tends to treat the cluster visually.                             | `line-index` works at Unicode scalar/UTF-8 boundary level, not grapheme level. This matches LSP. Do not use display width or grapheme segmentation in the source map.                                                                                                                                                                                                                                                         |
| BOM, `\u{FEFF}` at start                           | If the BOM is present in the server’s `String`, it is character 0 on line 0: byte range `0..3`, UTF-16 col `0..1`. If file loading strips BOM, all offsets must be built over the stripped text and diagnostics must never refer to the removed bytes. Choose one policy and test it. | VS Code and Marksman live on decoded strings, so a BOM that remains is just a UTF-16 code unit. In Rust, DMLS should avoid hidden stripping unless every parser sees the same stripped text.                                                                                                                                                                                                                                  |
| Final line without newline                         | End-of-document maps to the last line’s content length. There is no synthetic newline; insertions at EOF use that position.                                                                                                                                                           | Both `vscode-languageserver-textdocument` and rust-analyzer’s `line-index` allow offset equal to text length.                                                                                                                                                                                                                                                                                                                 |

Marksman is worth treating as a cautionary comparison, not a Rust model to copy. It stores text as .NET strings and maps offsets directly to LSP `Position`, so its offsets are already UTF-16 code-unit offsets. It also has TODOs around precise character stepping. Source: [`Text.fs`](https://github.com/artempyanykh/marksman/blob/main/Marksman/Text.fs).

## Existing Rust Line-Index Options

| Option                                     | API shape                                                                                                                    | Encoding support                                            | Maintenance                                                                                | Fit for DMLS                                                                                               |
|--------------------------------------------|------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------|--------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------|
| `line-index` from rust-analyzer            | `TextSize` byte offsets ↔ `LineCol { line, col }`; `to_wide(WideEncoding, LineCol)` and `to_utf8(WideEncoding, WideLineCol)` | Native UTF-8 byte columns plus UTF-16 and UTF-32 projection | Maintained by rust-analyzer, tested against broad Unicode input, optimized with SIMD paths | Best foundation. Needs a DMLS wrapper for line-ending policy, frontmatter projection, and LSP negotiation. |
| Bespoke DMLS implementation                | Can match exact DMLS names and frontmatter APIs                                                                              | Whatever we implement                                       | DMLS owns every Unicode and line-ending bug forever                                        | Not justified unless `line-index` cannot be adopted cleanly.                                               |
| `vscode-languageserver-textdocument` model | String-offset API, EOL aware                                                                                                 | JavaScript string offsets are UTF-16                        | Mature, but TypeScript/JS, not Rust                                                        | Useful behavior reference only.                                                                            |
| Marksman text model                        | Line map over .NET strings                                                                                                   | UTF-16 by runtime representation                            | Markdown-specific server, but not reusable in Rust                                         | Useful for Markdown semantics, not for Rust byte offsets.                                                  |

Recommendation: depend on `line-index`, but wrap it in `darkmatter/dmls/src/source_map`.

The wrapper should own:

- the negotiated DMLS `PositionEncoding` enum, including UTF-8 and UTF-16 immediately, UTF-32 if exposed;
- explicit CRLF and lone-CR line-ending handling before or alongside `line-index`;
- strict conversion APIs: `byte_to_lsp`, `lsp_to_byte`, `byte_range_to_lsp`, `lsp_range_to_byte`;
- version stamping, so ranges cannot be mixed across document versions;
- sub-document projection helpers for frontmatter and future code fences.

Do not expose raw `line-index` throughout DMLS. Keep it as the implementation detail behind the source-map module.

## Sub-Document Projection

Use a region descriptor:

```text
Region {
  host_start_byte,
  host_start_line,
  host_start_character_utf8,
  content_start_byte,
  content_end_byte,
}
```

For frontmatter, keep three ranges separate:

- opening delimiter line: `---\n`
- frontmatter content: starts after the opening delimiter line
- closing delimiter line: the line containing the second `---`

Diagnostics from a YAML parser usually use frontmatter-content coordinates, not whole-document coordinates. Project them by adding the content base offset, then convert through the host document source map. Prefer byte-base composition over manually adding line/column pairs; line/column addition fails around CRLF, astral characters, and diagnostics that span multiple lines.

Off-by-one traps:

- Do not include the opening delimiter in frontmatter-relative line 0 unless the parser actually saw it.
- Do not include the closing delimiter in YAML content ranges.
- If the frontmatter content is empty, the valid content range is zero-width between the delimiter lines.
- A diagnostic at YAML line 0, character 0 should usually map to document line 1, character 0.
- A diagnostic at the end of the last YAML content line maps before the closing delimiter newline, not onto the closing delimiter.
- For a range that includes a line ending, project its end as the start of the next line.
- Preserve the original document source map for display ranges; do not build LSP positions from a normalized YAML substring.

The same technique applies to code fences: distinguish the opening fence/info-string line, fence body, and closing fence. A diagnostic from an embedded language server should be relative to the body start, not the backticks. Info-string diagnostics are relative to the opening fence line.

## DMLS Unit Test Checklist

Encoding negotiation:

- VS Code-like client with no UTF-8: DMLS selects or omits UTF-16 only.
- Zed-like `["utf-16"]`: DMLS returns UTF-16.
- Neovim-like `["utf-8", "utf-16", "utf-32"]`: DMLS selects UTF-8 if supported.
- Helix-like `["utf-8", "utf-32", "utf-16"]`: DMLS selects UTF-8.
- Missing `general.positionEncodings`: DMLS defaults to UTF-16.
- Unsupported-only list plus implicit UTF-16 rule: DMLS still safely uses UTF-16.

Source-map conversions:

- empty document
- single ASCII line with and without trailing newline
- multiple LF lines
- CRLF lines
- lone-CR lines
- mixed LF/CRLF/CR input
- final empty line after newline
- final non-empty line without newline
- BMP multibyte: `aéβ宽b`
- astral scalar: `a💡b`
- CJK extension B scalar
- combining sequence: `e\u{0301}`
- BOM at byte 0
- invalid middle-of-codepoint byte offsets return error/`None`
- LSP character beyond line length is clamped only at protocol boundary, not inside internal APIs unless deliberately specified
- range ending at newline projects to next line character 0
- range spanning multibyte and astral characters round-trips byte → LSP → byte

Projection:

- frontmatter with one key
- empty frontmatter
- frontmatter without trailing body newline
- frontmatter with CRLF delimiters
- diagnostic on first YAML content character
- diagnostic on last YAML content character
- diagnostic spanning multiple frontmatter lines
- diagnostic must not land on opening or closing delimiter unless it is a delimiter diagnostic
- code fence body diagnostic maps after opening fence line
- code fence info-string diagnostic maps on opening fence line
- missing closing fence uses EOF as body end without fabricating a delimiter range
