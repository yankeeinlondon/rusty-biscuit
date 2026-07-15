# Darkmatter Language Server (DMLS)

`dmls` is the editor-facing language server for Markdown, with first-class
intelligence for the Darkmatter DSL, SimplifiedSchema-driven frontmatter, and
wiki-style links. It speaks standard **LSP 3.17 over stdio** (`lsp-server` +
`lsp-types`) and uses the `darkmatter` library as its sole semantic authority —
it never re-implements parsing, compose, schema, style, `LanguageGrammar`,
cleanup, or Markdown-aware hashing rules.

It serves two audiences at once:

1. **Ordinary Markdown authors** — a credible, standalone Markdown LSP for
   documents that use no Darkmatter features.
2. **Darkmatter and Claudine authors** — schema-validated frontmatter, directive
   and interpolation intelligence, transclusion navigation, and safety-aware
   (read-only, never-executing) handling of shell and remote content.

## Safety: passive by construction

DMLS runs inside editors and agents. Passive requests parse anything and resolve
local files, but they **never** execute shell commands (`$(...)`, `::shell`),
fetch remote URLs, mutate files, or run side-effecting compose phases. Shell and
remote surfaces are *explained* statically (hover + diagnostics report what
compose would do and whether policy allows it). This is proven by
[`tests/no_side_effects.rs`](tests/no_side_effects.rs) (spec acceptance
criterion 7).

## Feature layers (v1)

| Layer | Capabilities |
|-------|--------------|
| **0 — Markdown baseline** | go-to-definition, references/backlinks, document + workspace symbols, document links, folding, hover, path/anchor/fence-language completion, broken-link/anchor and duplicate-heading diagnostics. |
| **1 — Wiki links** | `[[target]]` / `[[target#heading]]` / `[[target\|alias]]` completion, definition, references, hover, and the full `wiki.*` diagnostic taxonomy (case-sensitive matching, basename/path-suffix/root-relative resolution, ambiguity + portability collisions). |
| **2 — Frontmatter intelligence** | effective-schema (base + configured extensions + repository-scoped trigger schemas + document `$schema`) diagnostics with precise key/value ranges, key/enum/`file(...)`/`style.*`/`suggest(...)` completion, type/constraint/`->`-description hover, `$schema`/`file(...)` navigation, frontmatter folding + symbols. Claudine activates as **pure config** (globs → baseline schema), no server-side special cases. |
| **3 — Darkmatter DSL** | directive (`::file`/`::code`/`::shell`/`::block`/disclosure …) name/option completion, hover, folding, and diagnostics; transclusion links + definition + cycle detection + broken-path; interpolation (`{{ }}`) completion/hover/definition + malformed/unknown diagnostics; read-only shell-policy hover + `darkmatter.security.*`; fenced-language diagnostics. |
| **Editing** | file + heading rename with workspace-wide reference updates (refusing ambiguous/unsafe edits atomically), the v1 code-action set, and `Markdown::cleanup`-backed formatting (byte-equivalent to `md clean`). |
| **Semantic tokens** | LSP semantic tokens classifying interpolations (`macro.interpolation`, `+inert` literals), directive keywords/closers (`macro.directive`, `+closer`), targets/options (`string`/`property.directive`), and wiki frames/segments (`macro.wiki` / `string.wiki`) for theme-driven de-emphasis. `full` + `range`, non-overlapping in UTF-8/UTF-16, fence-excluded, capability-gated, `[semantic_tokens] enable` master switch. |

For exact v1 scope and out-of-scope items see
[spec.md](../features/2026-07-04-dmls/spec.md); for architecture see
[design.md](../features/2026-07-04-dmls/design.md).

## Installation

Every editor integration boils down to the same thing: a native `dmls`
binary the editor can launch over stdio, plus (for VS Code and Zed) a thin
shipped extension that starts it.

**1. Install the binary** (any one of these):

```bash
# from the darkmatter/ package area — the canonical repo recipe
just install-dmls

# equivalent, from the repo root
cargo install --path darkmatter/dmls --force
```

Or, without a checkout: extract a release archive (see
[Packaging](#packaging)) and put `dmls` (`dmls.exe` on Windows) on `PATH`.

**2. Verify it runs:**

```bash
dmls --version
```

**3. Wire up your editor** — see [`docs/editors/`](docs/editors/):

| Editor | Install path |
|--------|-------------|
| VS Code | `just install-vscode-package` (packages + installs the shipped [`vscode-dmls/`](vscode-dmls/) extension) |
| Zed | Install [`zed-dmls/`](zed-dmls/) as a dev extension ([guide](docs/editors/zed.md)) |
| Neovim | Built-in LSP config only ([guide](docs/editors/neovim.md)) |
| Helix | `languages.toml` entry only ([guide](docs/editors/helix.md)) |

GUI editors do not always inherit your shell's `PATH`; if the editor cannot
find `dmls`, point it at the absolute path (usually `~/.cargo/bin/dmls`) —
both shipped extensions expose a binary-path setting. More failure modes are
covered in the [editor-setup troubleshooting section](docs/editors/README.md#troubleshooting).

## Architecture

- **One workspace graph** (`dmls::graph`): a single arena carrying every node
  kind and eight edge kinds (`references`, `includes`, `transcludes`,
  `uses_schema`, `uses_file`, `uses_variable`, `defines_anchor`,
  `defines_symbol`) with one reverse index and a wiki basename `KeyIndex`. Every
  navigation/diagnostic/refactor feature is a projection of that graph.
- **Source-map discipline** (`dmls::source_map`): all positions convert through
  one `line-index`-backed API (byte offsets ↔ negotiated UTF-8/UTF-16 LSP
  positions, frontmatter-relative → document ranges; CRLF + lone-CR aware).
- **Provider registry** (`dmls::providers`): each capability is an ordered
  provider chain (substrate Markdown first, overlay providers appending) with a
  per-provider `catch_unwind` boundary and deterministic merge policies.
- **Concurrency** (AD-3): a main protocol thread plus a crossbeam worker pool
  for indexing/diagnostics, with immutable generation-stamped snapshot swaps —
  no async runtime.
- **Extension model**: baseline schema extensions activate by config + globs
  (the generic mechanism Claudine is the first consumer of).

## CLI

```
dmls [--stdio] [--config <path>] [--log-level <level>] [--log-file <path>]
dmls --version
dmls --bench-index <dir> [--json]        # R-6 stage timings, graph counts, peak RSS
dmls --gen-corpus <tier> <dir>           # deterministic synthetic corpus
```

Logs go to stderr or `--log-file` only; stdout is reserved for LSP framing.

## Configuration

`.dmls.toml` at the workspace root (also the editor root marker), layered under
LSP `workspace/configuration` and reloadable without restart. Keys cover wiki
behavior, baseline schema extensions, strict schema/style modes, shell policy
discovery, code-action categories, formatting, semantic tokens
(`[semantic_tokens] enable`), and diagnostics debounce. See
[spec.md](../features/2026-07-04-dmls/spec.md) § Configuration.

## Editor setup

Per-editor guides (VS Code, Zed, Neovim, Helix) plus a manual smoke checklist
live in [`docs/editors/`](docs/editors/). The Zed extension is a thin WASM shim
launching the native binary; a ready-to-extract scaffold is in
[`zed-dmls/`](zed-dmls/) (workspace-excluded).

## Packaging

`just dist` (in `darkmatter/justfile`) builds a release archive for the host,
named per the cross-platform distribution matrix
(`dmls-<version>-macos-universal.tar.gz`, `…-linux-x86_64.tar.gz`,
`…-linux-aarch64.tar.gz`, `…-windows-x86_64.zip`) that the Zed extension resolves
against. CI wires the full per-target build.

## Testing

From the `darkmatter/` package area:

```
just test        # L1 (unit + in-process LSP-session) across lib, cli, dmls
just test-l2     # L2 real-editor tests (Neovim + tmux; skip cleanly if absent)
just lint
```

Testing follows `.claude/skills/rust-testing/SKILL.md` (nextest, L1 default).
The in-process JSON-RPC session tests (`tests/lsp_session.rs`) are L1. The L2
tier (`tests/level2_editor_neovim.rs`) drives Neovim's real LSP client against
the built `dmls` binary — headless token-decode probes plus a tmux-rendered
SGR capture — and skips cleanly when `nvim`/`tmux` are missing
(`BISCUIT_TEST_LEVEL_REQUIRED=2` hard-fails instead). The remaining manual
editor verification lives in
[`docs/editors/smoke-checklist.md`](docs/editors/smoke-checklist.md).
