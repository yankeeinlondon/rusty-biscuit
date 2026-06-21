---
created: 2026-06-17
reviewed: true
status: ready for planning and implementation
---

# CLI Atheist — Modularizing the `md` CLI

The `darkmatter-cli` crate has four god-files. Three of them are in `src/`
and one is split across two integration-test files. They have grown by
accretion: every new subcommand, flag, or rendering knob landed in the
same file because there was no natural home for it. The result is a small
number of files with hundreds of unrelated top-level symbols, deep nesting,
and high coupling.

This spec defines a target module layout that splits the four god-files
by responsibility, and surfaces the **business logic that has leaked from
the `darkmatter` library into the CLI** — those leaks are the deeper
reason the CLI files keep growing.

The name is tongue-in-cheek: the CLI should have **no god-files**, only
small modules with clear ownership.

> **Reader's note from inline review.** This reviewed version tightens the
> extraction boundaries, especially around renderable color parsing and Cargo
> integration-test discovery. The intended change remains a behavior-preserving
> modularization; any public behavior change called out below must be guarded by
> explicit before/after captures and tests.

## Scope — The Four God-Files

| File                              | Lines | Top-level symbols | Imports | Source of pain |
|-----------------------------------|-------|-------------------|---------|----------------|
| `darkmatter/cli/src/args.rs`      | 2093  | 33                | 20      | 5 unrelated concerns + 1 library leak (Tailwind table) |
| `darkmatter/cli/src/output.rs`    | 1357  | 31                | 47      | 6 unrelated concerns; view logic mixed with flag-to-builder glue |
| `darkmatter/cli/tests/cli.rs`     | 5130  | ~292              | 39      | every CLI integration test in one file |
| `darkmatter/cli/tests/level2_layout.rs` | 3077 | ~85            | 14      | every L2 layout/styling test in one file |

A fifth file, `darkmatter/cli/src/commands.rs` (1037 lines, ~31 symbols),
is on the edge. It is included in the analysis because it shares the same
root cause (one file doing dispatch, shared helpers, three subcommand
implementations, and a large block of JSON serializers that belong on
library types) and will be reshaped by the same migration.

## Audience

Maintainers of `darkmatter`, `darkmatter-cli`, and adjacent renderable
code. Assumes familiarity with the CLI surface (`md`, `md compose`,
`md validate refs`, etc.) and the darkmatter skill's responsibility split
between `darkmatter` (parsing, composition, rendering) and
`biscuit-terminal` (terminal components).

## Goals

1. **No god-files.** Each file under `darkmatter/cli/src/` and
   `darkmatter/cli/tests/` has one responsibility and stays under ~500
   lines (soft cap; the cap is a signal, not a hard CI gate).
2. **Push business logic out of the CLI.** Anything that is not
   CLI-specific (argument shape, terminal plumbing, user prompts) moves
   to the `darkmatter` library or to a sibling crate where it already
   belongs. See [Library Leaks](#library-leaks-found-in-the-cli).
3. **Keep public behavior byte-for-byte equal.** No CLI flag renames, no
   output format changes, no exit-code changes. The migration is a pure
   re-shuffle capped by library extraction.
4. **Make each subcommand independently testable.** Integration tests
   are split so a change to `md compose` only needs to run `md compose`
   tests, not the whole suite.
5. **Make the precedence rules live in one place.** Today the
   CLI-flag-to-`DarkmatterPage` precedence (`margin > mx > mt`, etc.)
   is encoded **twice** — once in `apply_cli_layout_flags` (apply the
   value) and once in `page_style_overrides_from_cli` (claim the field
   so frontmatter does not stomp it). Collapse to one source of truth.
6. **Don't break the L2/L3 test taxonomy.** The split of `tests/cli.rs`
   must preserve the existing L1 (unit, in-module `#[cfg(test)]`),
   L2 (real-terminal layout), and L3 (full CLI subprocess via
   `assert_cmd`) separation documented in the `rust-testing` skill.

## Non-Goals

1. **No new CLI features.** No new flags, no new subcommands, no new
   output formats. If a missing feature surfaces during the refactor it
   gets a separate feature folder.
2. **No rewrites of the darkmatter library APIs the CLI calls.**
   `DarkmatterPage`, `ComposeOptions`, `Markdown::delta`, etc. keep
   their current shapes. The library leaks identified below are
   extracted, not redesigned.
3. **No churn to `biscuit-terminal` and only targeted churn to
   `renderable`.** Color parsing may add narrow public helpers to
   `renderable` because that crate already owns `Tailwind`, `Color`,
   `RgbColor`, and `PaintColor`. The existing public surface stays;
   new helpers must be additive and covered by renderable unit tests.
4. **No change to the binary name (`md`) or install path.**
5. **No new dependencies.** Re-use `clap`, `clap_complete`,
   `color-eyre`, `assert_cmd`, `tempfile`, etc.
6. **No reordering of clap subcommands.** Userscripts depend on the
   current surface.

## Library Leaks Found in the CLI

These are the pieces of business logic that should not be in the CLI at
all. Each one is called out because **the god-file problem is partly a
symptom of these leaks**: with no library home for the logic, it kept
landing in whatever CLI file was being edited.

### Leak 1 — `tailwind_from_str` (args.rs:1317–1572, 255 lines)

A 250-line `match` table mapping `("slate", 50) => Slate50` for the
entire Tailwind palette. The CLI uses it for `--page-bg-color
red-500` parsing. This is pure data lookup that belongs on
`renderable::color::Tailwind` next to the enum it mirrors.

**Proposed home:** `Tailwind::from_kebab_name(&str) -> Option<Tailwind>`
on `renderable::color::Tailwind` (the enum already lives there,
alongside `kebab_name()` which is the inverse).

The CLI then calls `Tailwind::from_kebab_name` and the entire 255-line
table disappears from `args.rs`.

### Leak 2 — color-string parsing (args.rs:1213–1315)

`parse_page_bg_color`, `parse_hex_color`, `parse_rgb_triple` accept
`#RGB`, `#RRGGBB`, `R,G,B`, Tailwind names, and the CSS keywords
(`transparent`, `currentColor`, `inherit`). This is a CSS color string
parser producing a `PaintColor`. It belongs next to `PaintColor` /
`Color` in `renderable::style` or `renderable::color`.

**Proposed home:** `PaintColor::from_css_str(&str) -> Result<PaintColor,
ParseColorError>` implemented in `renderable::style::paint`, plus an
internal or public `renderable::color::Color::from_css_str` helper if
the implementation benefits from parsing the underlying color first.
This keeps the constructor on the type it returns and avoids making
`renderable::color` own style-layer alpha/paint semantics. The CLI flag
parser then becomes a thin `value_parser = PaintColor::from_css_str`
wrapper.

The accepted grammar must stay byte-for-byte compatible with the
existing CLI parser unless the implementation PR explicitly documents a
behavior change:

- `#RGB` and `#RRGGBB`
- `R,G,B` with decimal 0-255 channels and no alpha channel
- Tailwind kebab names such as `red-500`
- CSS keywords currently accepted by the CLI: `transparent`,
  `currentColor`, and `inherit`

The error type should be a small `ParseColorError` in `renderable`
using existing dependencies only. It should preserve the rejected input
and a concise reason so clap can surface useful value-parser errors.

### Leak 3 — JSON serializers for reference types (commands.rs:730–974)

Nine free functions — `source_to_json`, `kind_to_json`,
`target_to_json`, `syntax_to_json`, `directive_kind_to_json`,
`reference_record_to_json`, `insertion_to_json`, `graph_node_to_json`,
`validation_report_to_json` — hand-serialize `darkmatter::markdown::reference`
types into `serde_json::Value`. The library types live in
`darkmatter/lib/src/markdown/reference/types.rs` but do not derive
`Serialize`. The CLI is hand-rolling what should be serde derives.

**Proposed home:** Add `#[derive(serde::Serialize)]` (with
`#[serde(tag = "type")]` / `#[serde(rename_all = "snake_case")]` as
needed) to `ReferenceKind`, `ReferenceTarget`, `ReferenceSyntax`,
`ReferenceOrigin`, `ReferenceRecord`, `ReferenceInsertionContext`,
`ReferenceInsertion`, `ReferenceGraphNode`, `ReferenceValidationIssue`,
`ReferenceIssueCode`, `ReferenceSeverity`, `ReferenceValidationReport`,
and `ComposeSource` (which already has a JSON shape — it just needs an
impl). Delete the nine CLI helpers.

The serde shapes are public API once exposed through `md validate refs
--json` and `md graph --json`, so this extraction must be compatibility
tested against the current hand-rolled output. Do **not** accept serde's
default externally-tagged enum format unless it exactly matches the
existing CLI JSON. Use explicit serde attributes or manual `Serialize`
impls on individual enums where that is the smallest way to preserve the
current shape.

This is also a **correctness improvement**: hand-rolled JSON shapes
drift from the underlying enums. The CLI's `kind_to_json` returns
`"html_video"` while the enum variant is `HtmlVideo` — derive would
produce the same thing, but today the mapping is maintained by hand.

### Leak 4 — validation-report terminal rendering (commands.rs:598–728)

`format_validation_issues` and `reference_kind_category_label` build a
`Prose` + `UnorderedList` view of a `ReferenceValidationReport`. This
is view-layer code over a library type and is duplicated in spirit by
the `md compose` error path which also formats validation issues. It
should be a `TerminalRenderable` component — either in
`darkmatter::markdown::reference::validate` (as `impl
TerminalRenderable for ReferenceValidationReport`) or as a small
adapter in `biscuit-terminal` if the dependency direction forbids it
in darkmatter proper.

**Proposed home:** `darkmatter::markdown::reference::validate::ReportView`
(or `impl TerminalRenderable`) — the CLI keeps only the
`report.render(&term)` call.

### Leak 5 — CLI style-claim tracking (output.rs:297–419)

Six functions (`page_style_overrides_from_cli`, `list_style_overrides_from_cli`,
`component_style_overrides_from_cli`, `hr_style_overrides_from_cli`,
`disclosure_style_overrides_from_cli`, `bespoke_style_overrides_from_cli`)
translate CLI flags into the library's `*StyleOverrides` "I've already
claimed this field" structs. The mapping (e.g. `--mx` claims left +
right; `--align-lists` claims ul + ol + li) **duplicates** the same
precedence rules encoded in `apply_cli_layout_flags`. When the rules
change, both call sites must change in lockstep — there is no test that
ties them together.

**Proposed home:** Introduce a single `darkmatter::style::CliStyleClaims`
struct that captures the already-resolved claim set once, and have the
library expose `apply_cli_claims(page, claims)` plus
`style_overrides_from_claims(claims)`. The CLI builds the claims struct
from its flags in `style_claims.rs`; the library does the rest. This
collapses the duplication and makes future flag additions a one-line
change.

Important boundary: `darkmatter` must not depend on `darkmatter-cli`,
and the library type must not mention clap or CLI-only wrapper types.
`CliStyleClaims` is a neutral data model expressed in library/layout
types (`PageComponent`, `Layout`, `PaintColor`, etc.). The CLI builder
is the only code that knows about `Cli`, `CliFill`, and clap aliases.

### Leak 6 — delta and TOC terminal rendering (output.rs:662–1124)

`print_toc_tree` / `print_toc_node` and `print_delta` /
`format_code_block_change` are large `writeln!` blocks hand-writing
ANSI escapes (`\x1b[1m`, `\x1b[7m`, etc.). They bypass the
`TerminalRenderable` / `Prose` model that every other CLI output path
uses. They are also the largest single symbols in `output.rs`
(`print_delta` is 261 sloc).

**Proposed home:** Two new components in the darkmatter library (the
types already live there):

- `darkmatter::markdown::toc::TocTree` — `impl TerminalRenderable`,
  wraps `MarkdownToc`. Replaces `print_toc_tree`.
- `darkmatter::markdown::delta::DeltaReport` — `impl
  TerminalRenderable`, wraps `MarkdownDelta`. Replaces `print_delta`.

The CLI keeps only `print!("{}", report.render(&term))`. As a bonus the
components become available to library callers and to other CLIs in the
monorepo.

### Leak 7 — boolean / env parsing (args.rs:1171–1178, output.rs:632–653)

`parse_bool_str` and `parse_bool_env` are tiny but duplicated across
files and are not CLI-specific. They are candidates for
`biscuit-terminal::env` or a small shared utility crate, but they are
low-priority — call them out as a minor leak, not a blocker.

### Leak 8 — output-artifact file plumbing (output.rs:586–630)

`emit_or_show_artifact`, `open_output_artifact`,
`write_output_artifact_file` manage temp-file write + `open::that`. This
is generic across any CLI that emits artifacts, not specific to
darkmatter. It can stay CLI-local (it is small) but should move to its
own `crate::io::artifact` module rather than sit inside `output.rs`.

## Proposed Source Layout

```
darkmatter/cli/src/
├── main.rs                         # ~120 lines — arg-parse, tracing init, dispatch
├── lib.rs                          # re-exports, module declarations
│
├── args/                           # everything clap-derived (was args.rs)
│   ├── mod.rs                      #   pub use surface; the Cli struct, global args
│   ├── cli.rs                      #   struct Cli (top-level flags only)         ~250
│   ├── command.rs                  #   enum Command (subcommand shapes)          ~300
│   ├── target.rs                   #   ValidateTarget, SchemaTarget              ~120
│   ├── enums.rs                    #   OutputFormat, CodeBlockOutput,
│   │                               #   RemoteFreshness, GraphFormat, HashKind,
│   │                               #   SchemaValidateFormat, SchemaDetectFormat,
│   │                               #   ValidateOutputFormat  + From<…> impls     ~180
│   ├── wrappers.rs                 #   PageBackgroundArg, CodeBlockArg,
│   │                               #   PageAlignmentArg, CliFill  + From impls  ~140
│   ├── parsers.rs                  #   parse_indent_size, parse_theme_name,
│   │                               #   parse_cli_fill, parse_cli_length,
│   │                               #   parse_max_width, reject_width_flag,
│   │                               #   parse_bool_str                          ~120
│   └── completion.rs               #   complete_markdown_files[_from],
│                                   #   complete_compose_args[_from],
│                                   #   complete_indent_values,
│                                   #   complete_theme_names                     ~180
│
├── style_claims.rs                 # NEW: builds darkmatter::style::CliStyleClaims
│                                   # from a &Cli — the one place CLI flag
│                                   # precedence is encoded (replaces the six
│                                   # *_style_overrides_from_cli helpers)        ~120
│
├── render.rs                       # render_terminal_output + ResolvedTheme +
│                                   # apply_cli_layout_flags (the value side of
│                                   # the layout precedence)                    ~200
│
├── artifact.rs                     # OutputArtifact + emit/open/write helpers,
│                                   # terminal_image_mode_from_env               ~120
│
├── io/
│   └── mod.rs                      # load_markdown, resolve_file_path,
│                                   # read_from_stdin (shared file-input helpers
│                                   # extracted from commands.rs)                ~80
│
├── approval.rs                     # unchanged (350 lines, already its own file)
│
├── output_dispatch.rs              # thin: pick artifact by OutputFormat and
│                                   # dispatch to render/artifact modules        ~80
│
└── commands/                       # was commands.rs (1037 lines) + commands/
    ├── mod.rs                      #   run_subcommand dispatch only             ~120
    ├── render.rs                   #   run_render (was in commands.rs)          ~120
    ├── clean.rs                    #   run_clean, apply_cleanup,
    │                               #   resolve_list_spacing                    ~80
    ├── toc.rs                      #   run_toc (uses darkmatter TocTree)        ~40
    ├── delta.rs                    #   run_delta (uses darkmatter DeltaReport)  ~50
    ├── graph.rs                    #   run_graph + JSON view (slimmed once
    │                               #   serde derives land on library types)    ~200
    ├── validate.rs                 #   run_validate + ReportView rendering      ~120
    ├── compose.rs                  #   unchanged shape; ~1000 lines is OK once
    │                               #   JSON helpers move out                   ~700
    ├── code_block.rs               #   unchanged                                ~300
    ├── frontmatter.rs              #   run_get / run_set / run_rm / run_edit    ~330
    ├── hash.rs                     #   unchanged                                ~275
    └── schema/                     #   unchanged
        ├── mod.rs
        ├── about.rs
        ├── assignment.rs
        ├── detect.rs
        └── validate.rs
```

### What each top-level file looks like after the move

- `main.rs` — argument parse, `init_tracing`, `reset_sigpipe`,
  `--completions` / `--list-themes` early exits, error rendering.
  ~120 lines.
- `lib.rs` — module declarations and the three public re-exports
  (`Cli`, `CliCommand`, `OutputFormat`). Drops the long prose docs in
  favor of a pointer to the README; the prose is library usage that
  duplicates `darkmatter/lib/src/lib.rs`.
- `args/` — every clap-derived declaration. Parsers, completions, and
  enum wrappers each get their own file. Total stays around 1300 lines
  but split across eight files.
- `render.rs` — owns `render_terminal_output`, the `ResolvedTheme`
  helper, and the value side of `apply_cli_layout_flags` /
  `apply_component_alignment` / `apply_component_fill`.
- `style_claims.rs` — owns the **claim** side of CLI-flag precedence,
  producing a `CliStyleClaims` consumed by both `render.rs` and the
  library. Replaces the six `*_style_overrides_from_cli` helpers.
- `artifact.rs` — artifact emission (write-to-temp, open, dry-run).
- `io/` — file-input helpers shared by every subcommand.

## Proposed Test Layout

```
darkmatter/cli/tests/
├── common/
│   ├── mod.rs                      # md_cmd, md_file, MockHttpServer,
│                                   # shared fixtures                              ~150
│   ├── fixtures.rs                 # large canned documents used by >1 file
│   └── level2.rs                   # WezTerm harness, md shim, L2 gating
│
├── help.rs                         # --help, --version, --list-themes            ~80
├── render_basic.rs                 # default-mode render, stdin, show, aliases   ~250
├── clean.rs                        # md clean, --save, indent, compact/loose     ~250
├── toc.rs                          # md toc text + json                           ~150
├── delta.rs                        # md delta text + json + verbose               ~250
├── get_set_rm.rs                   # md get / set / rm / edit                     ~400
├── hash.rs                         # md hash, --save, --diff, --strict            ~300
├── validate_refs.rs                # md validate refs, --graph                    ~350
├── graph.rs                        # md graph, --follow, --validate, --json       ~250
├── code_block.rs                   # md code-block                                ~250
├── schema_validate.rs              # md schema validate                           ~250
├── schema_detect.rs                # md schema detect                             ~250
├── schema_about.rs                 # md schema about                              ~250
├── compose_basic.rs                # plain compose, --frontmatter, --show
├── compose_state_set.rs            # --state, --set, shorthand setters
├── compose_interpolation.rs        # {{ }}, $(...), env/doc/ctx namespaces
├── compose_transclusion.rs         # ::file, ::code, remote URLs
├── compose_page_blocks.rs          # ::block / ::end-block
├── compose_shell.rs                # ::shell, ::shell-block, pre-flight, --shell
├── compose_refs_and_missing.rs     # --allow-missing-*, --allow-host
├── compose_perf.rs                 # --perf, performance reports
├── compose_remote_caching.rs       # --cache-root, --remote-ttl,
│                                   # --remote-refresh, mock_http_server
├── compose_layout.rs               # compose + layout-flag interaction
├── layout_flags.rs                 # CLI flag precedence (margin/mx/mt, etc.)
├── layout_style_frontmatter.rs     # style.page.* / style.table.* wiring
├── layout_alignment.rs             # alignment flags + frontmatter
├── layout_fill.rs                  # fill flags + frontmatter
├── level2_layout_dimensions.rs     # split from level2_layout.rs
├── level2_code_block_styling.rs
├── level2_frontmatter_tables.rs
├── level2_frontmatter_images.rs
├── level2_ordered_lists.rs
├── level2_horizontal_rules.rs
├── level2_disclosure_blocks.rs
├── level2_errors.rs                # unchanged (already split)
└── level2_schema_about.rs          # unchanged (already split)
```

The `mock_http_server` helper currently defined in `tests/cli.rs:51–92`
moves to `tests/common/mod.rs` so both `compose_remote_caching.rs` and
any future remote-validation tests can share it.

Cargo does not discover arbitrary nested files under `tests/` as
integration-test crates. Keep the actual test binaries as top-level
files and use prefixes (`compose_*`, `layout_*`, `level2_*`) for
filtering. `tests/common/mod.rs` is the only nested module required by
this spec; every top-level test file that uses shared helpers declares
`mod common;`.

### Naming convention

- One file per subcommand for simple subcommands (`hash.rs`, `toc.rs`).
- One filename prefix per subcommand for subcommands with many test
  groups (`compose_*`, `schema_*`).
- `level2_` stays at the start of every Level 2 test filename and test
  function so the test-runner filter `level2_` keeps working with the
  existing `just` recipes documented in the `rust-testing` skill.

## Per-File Decomposition Plan

### `args.rs` → `args/` (8 files)

| Current section in `args.rs` | New home |
|------------------------------|----------|
| Lines 1–56 (`OutputFormat`, `CodeBlockOutput`, `RemoteFreshness`) | `args/enums.rs` |
| Lines 58–450 (`enum Command`) | `args/command.rs` |
| Lines 452–489 (`ValidateTarget`) | `args/target.rs` |
| Lines 491–617 (`SchemaTarget`, `ValidateOutputFormat`, `GraphFormat`, `HashKind`, `SchemaValidateFormat`, `SchemaDetectFormat`) | `args/enums.rs` + `args/target.rs` |
| Lines 619–630 (`impl From<HashKind>`) | `args/enums.rs` |
| Lines 632–876 (`struct Cli`) | `args/cli.rs` |
| Lines 878–1015 (completion helpers) | `args/completion.rs` |
| Lines 1017–1202 (parsers, `reject_width_flag`) | `args/parsers.rs` |
| Lines 1204–1315 (color parsers) | **deleted** — moved to `PaintColor::from_css_str` in `renderable::style::paint` (Leak 2) |
| Lines 1317–1572 (`tailwind_from_str`) | **deleted** — moved to `Tailwind::from_kebab_name` (Leak 1) |
| Lines 1574–2093 (in-file tests) | stays as `#[cfg(test)] mod tests` in each new file |

The `args/mod.rs` re-exports the public surface so callers (`main.rs`,
`commands/mod.rs`) keep the same `use crate::args::{Cli, Command, …}`
paths.

### `output.rs` → 4 files

| Current section in `output.rs` | New home |
|--------------------------------|----------|
| Lines 20–51 (`OutputArtifact`, `ResolvedTheme`) | `artifact.rs` (`OutputArtifact`), `render.rs` (`ResolvedTheme`) |
| Lines 53–105 (`render_terminal_output`) | `render.rs` |
| Lines 107–295 (`apply_cli_layout_flags`, `apply_component_alignment`, `apply_component_fill`) | `render.rs` |
| Lines 297–419 (six `*_style_overrides_from_cli`) | **deleted** — replaced by `style_claims.rs` building `CliStyleClaims` (Leak 5) |
| Lines 421–510 (`apply_style_frontmatter`, `log_style_warnings`) | `render.rs` (calls library's `apply_cli_claims`) |
| Lines 512–653 (artifact helpers, env parsers) | `artifact.rs` |
| Lines 655–765 (TOC tree printing) | **deleted** — replaced by `darkmatter::markdown::toc::TocTree` (Leak 6) |
| Lines 767–1124 (delta printing, `format_code_block_change`) | **deleted** — replaced by `darkmatter::markdown::delta::DeltaReport` (Leak 6) |
| Lines 1126–1357 (in-file tests) | distributed to `render.rs` / `style_claims.rs` / `artifact.rs` as appropriate |

`output.rs` itself is **deleted**. The name was always too generic for
what it contained.

### `commands.rs` → `commands/` (existing dir, plus new siblings)

| Current section in `commands.rs` | New home |
|----------------------------------|----------|
| Lines 17–27 (submodule declarations, `use` of `run_*`) | `commands/mod.rs` |
| Lines 29–53 (`validate_subcommand_usage`) | `commands/mod.rs` |
| Lines 55–269 (`run_subcommand` dispatch) | `commands/mod.rs` |
| Lines 271–328 (`resolve_list_spacing`, `run_clean`, `apply_cleanup`) | `commands/clean.rs` |
| Lines 330–386 (`run_render`) | `commands/render.rs` |
| Lines 388–452 (`load_markdown`, `resolve_file_path`, `read_from_stdin`) | `crate::io/mod.rs` |
| Lines 454–616 (`run_validate`, report text printers) | `commands/validate.rs` (uses `ReportView` from Leak 4) |
| Lines 598–728 (`format_validation_issues`, `reference_kind_category_label`) | **deleted** — moved to darkmatter library (Leak 4) |
| Lines 730–974 (nine JSON serializers) | **deleted** — replaced by `#[derive(Serialize)]` on library types (Leak 3) |
| Lines 976–1036 (`run_graph`) | `commands/graph.rs` |

`commands.rs` (the file) is deleted; its dispatch moves into
`commands/mod.rs` and the three subcommand implementations it was
holding (`render`, `clean`, the validation text printer) move into
their own files alongside the existing `compose.rs`, `hash.rs`, etc.

### `tests/cli.rs` → `tests/{per-subcommand}.rs` + `tests/common/`

The file already has 28 `// ===` section dividers — those are the file
boundaries. The split is mechanical:

1. Extract `md_cmd`, `md_file`, `MockHttpResponse`, `MockHttpServer`,
   `mock_http_server` into `tests/common/mod.rs`.
2. Each `// =====` section becomes its own file under `tests/`.
3. The compose-related sections (which are the largest cluster) become
   top-level `tests/compose_*.rs` files.
4. The layout/style sections become top-level `tests/layout_*.rs` and
   `tests/level2_*.rs` files.

The L2/L3 distinction is preserved: anything that needs a real terminal
(`level2_*` prefix today) stays in a `level2_*` file and keeps
`#[serial(level2_terminal)]`; everything else is L3 (`assert_cmd`) by
default.

### `tests/level2_layout.rs` → top-level `tests/level2_*.rs` files

85 top-level test functions covering layout, style frontmatter, code
blocks, lists, tables, images, HRs, and disclosures. Split by the same
concern boundaries the test names already encode
(`level2_style_frontmatter_*`, `level2_code_block_*`,
`level2_style_hr_*`, etc.).

## Phased Migration Plan

Each phase is independently shippable, each phase leaves the CLI
working, and each phase can land as its own PR. **No phase depends on
the library leaks being extracted first** — the leaks are interleaved
so the CLI is never left in a half-extracted state.

### Phase 0 — Library extractions (parallelizable)

These can land independently and in any order. Each one shrinks the
CLI god-files without touching CLI structure.

- **P0a** — `Tailwind::from_kebab_name` in `renderable::color`. Delete
  `tailwind_from_str` from `args.rs`. (~255-line CLI drop.)
- **P0b** — `PaintColor::from_css_str` in `renderable::style::paint`
  (with a color-layer helper only if useful). Delete `parse_page_bg_color`,
  `parse_hex_color`, `parse_rgb_triple` from `args.rs`. (~100-line
  drop.)
- **P0c** — `#[derive(serde::Serialize)]` on the reference types in
  `darkmatter::markdown::reference`. Delete the nine JSON helpers in
  `commands.rs`. (~240-line drop.)
- **P0d** — `darkmatter::markdown::toc::TocTree` (impl
  `TerminalRenderable`). Delete `print_toc_tree` / `print_toc_node`.
  (~80-line drop.)
- **P0e** — `darkmatter::markdown::delta::DeltaReport` (impl
  `TerminalRenderable`). Delete `print_delta` /
  `format_code_block_change`. (~300-line drop.)
- **P0f** — `darkmatter::markdown::reference::validate::ReportView`
  (impl `TerminalRenderable`). Delete `format_validation_issues` /
  `reference_kind_category_label` from `commands.rs`. (~130-line drop.)

Phase 0 alone removes ~1100 lines from the CLI without touching its
shape.

Compatibility gates for Phase 0:

- Capture current `md validate refs --json` and `md graph --json`
  outputs with local paths, remote URLs, fragments, data URIs, inline
  CSS/script/meta records, validation errors, and graph insertions.
  The serde-backed output must match exactly unless the PR explicitly
  documents an intended JSON-shape migration.
- Capture current invalid color parser errors for representative inputs
  (`#12`, `300,0,0`, unknown Tailwind name, unknown keyword). If
  renderable emits clearer text, tests should assert the new behavior
  and the PR should call out the user-facing error improvement.
- Add renderable unit tests for `Tailwind::from_kebab_name`,
  `Color::from_css_str` if added, and `PaintColor::from_css_str`.

### Phase 1 — Split `args.rs` (mechanical)

Move declarations into `args/{cli,command,target,enums,wrappers,parsers,completion}.rs`.
Pure file-move commit; no behavior change. Lands as one PR because the
`use` paths change atomically.

Verification: `just test` (or `cargo test -p darkmatter-cli`) must pass
with no snapshot diffs.

### Phase 2 — Split `commands.rs` (mechanical)

Move `run_render` → `commands/render.rs`, `run_clean` →
`commands/clean.rs`, `run_validate` → `commands/validate.rs`,
`run_graph` → `commands/graph.rs`, and the shared file-input helpers →
`crate::io`. `commands.rs` is deleted; `commands/mod.rs` owns only the
dispatch.

### Phase 3 — Collapse the style-claim duplication

Introduce `darkmatter::style::CliStyleClaims` (the library type) and
`style_claims.rs` (the CLI builder). Migrate `apply_cli_layout_flags`
and `apply_style_frontmatter` to consume it. Delete the six
`*_style_overrides_from_cli` helpers.

This is the only phase that touches library behavior (a new public type
and a new apply path), so it carries the most review weight. Land it
last among the structural phases.

### Phase 4 — Split `output.rs`

By this phase, `output.rs` is already much smaller (Phase 0 deleted
the TOC/delta blocks; Phase 3 deleted the override helpers). Move
what's left into `render.rs` (rendering entrypoints, layout-flag
application) and `artifact.rs` (output-artifact plumbing). Delete
`output.rs`.

### Phase 5 — Split `tests/cli.rs` (mechanical, can run in parallel with Phases 1–4)

Extract `tests/common/mod.rs` first, then split the file along its
existing `// =====` section boundaries. Each new top-level test file
imports `mod common;`. The compose tests land as `tests/compose_*.rs`
files so Cargo discovers them without a custom harness.

### Phase 6 — Split `tests/level2_layout.rs` (mechanical)

Same pattern as Phase 5. The file splits cleanly along the
`level2_*` prefix groups in the test names. Move the shared real-terminal
harness bootstrap from `level2_layout.rs` into `tests/common/level2.rs`
so every split L2 file keeps using the just-built `md` binary and the
same Level 2 skip/enforce policy.

### Phase 7 — Documentation pass

Update `darkmatter/cli/README.md` and the darkmatter skill's "module
layout" topic to point at the new structure. Note the new library
surfaces (`TocTree`, `DeltaReport`, `CliStyleClaims`,
`Tailwind::from_kebab_name`, `PaintColor::from_css_str`).

## Decisions Made During Review

1. **`PaintColor::from_css_str` lives with `PaintColor`.** The
   reviewed design chooses `renderable::style::paint` as the public
   constructor home because it returns `PaintColor` and therefore owns
   opacity/paint semantics. A lower-level `Color::from_css_str` helper
   may live in `renderable::color` if useful, but it must not be the
   only public parser for paint values.

2. **Integration tests stay as top-level files.** Cargo does not
   recursively discover `tests/compose/basic.rs` as an integration-test
   crate. The reviewed design uses top-level files with prefixes
   (`compose_basic.rs`, `level2_layout_dimensions.rs`) and shared
   modules under `tests/common/`.

3. **Serde extraction must preserve CLI JSON.** Moving reference JSON
   serialization to library types is still the right fix, but derived
   serde shapes are not automatically accepted. Compatibility tests own
   the contract.

## Decisions to Make (Open Questions)

These are still open after review. Each one is a small ADR-sized
decision; resolve them before implementation starts.

1. **Does `CliStyleClaims` belong on `darkmatter::style` or on a new
   `darkmatter::layout` module?** The claim types already span both
   (`PageStyleOverrides` lives in `style`, but the underlying fields
   are layout). Recommendation: `darkmatter::style`, because that's
   where `apply_*_style` already lives.

2. **Are `TocTree` and `DeltaReport` terminal-only components, or do
   they also get `BrowserRenderable` / `MarkdownRenderable`
   impls?** The CLI only renders them to terminal today. Adding the
   other targets is a separate feature; this spec only requires the
   terminal impl.

3. **Should the `commands/` source split land in lockstep with the
   `compose_*` test split?** Landing them together makes review
   easier (the test imports change in the same PR as the source
   moves). Landing them apart makes each PR smaller. Recommendation:
   together.

4. **L1 unit tests for `args/` parsers — do they stay in-module or
   move to `tests/`?** In-module is the existing convention and keeps
   each parser file self-contained. Recommendation: stay in-module.

5. **Is the ~500-line soft cap worth enforcing in CI?** A
   `#[warn(clippy::too_many_lines)]` per-file gate would prevent
   regression but also prevents legitimate long tables. Recommendation:
   no CI gate; rely on review and a `just lint-files` script that
   reports files over the cap.

6. **Does `darkmatter-cli` re-export `Tailwind` and `PaintColor`?**
   Today the CLI's public surface includes `args::Cli` etc. but not
   the color types. After Leak 1 / Leak 2 the CLI no longer owns the
   color parsers. If anything outside the CLI depended on them (it
   appears not — these were `fn`, not `pub fn`), they get the
   library path.

7. **How much of the L2 harness should move into `tests/common`?**
   `level2_layout.rs` currently has substantial shared real-terminal
   setup, including the just-built `md` shim and Level 2 gating. The
   recommended option is to move all harness setup and fixture-running
   helpers into `tests/common/level2.rs`, leaving each `level2_*` test
   file to contain only assertions for its concern.

   - Pros: no duplicated WezTerm setup, consistent skip/enforce
     behavior, lower risk of accidentally testing a host-installed
     `md`.
   - Cons: `tests/common` becomes more substantial and needs careful
     namespacing.

   Alternative: duplicate the minimal helpers in each L2 file.

   - Pros: each test file is self-contained.
   - Cons: high drift risk, slower review, easy to lose the binary-shim
     safety contract.

   Alternative: keep a single `level2_layout.rs` harness and include
   submodules from a nested `level2/` directory.

   - Pros: preserves one integration-test binary and one shared pane.
   - Cons: does not fully solve the god-file problem for tests, and
     makes per-concern test selection coarser.

   Recommendation: move shared harness code to `tests/common/level2.rs`
   and keep top-level `level2_*` test files.

## Verification Plan

Each phase's PR must demonstrate:

1. `just test` (or `cargo test -p darkmatter-cli` for the focused
   phases) passes with no new failures.
2. `just lint` passes (`cargo clippy` clean, no new warnings).
3. No snapshot diffs in any existing `insta` snapshot under
   `darkmatter/cli/tests/` or `darkmatter/lib/`.
4. The `md --help` output is byte-for-byte equal before and after
   Phases 1–6 (clap derives the same surface). Phase 0 may produce
   slightly improved error messages from the new library parsers; those
   get explicit before/after captures in the PR.
5. The `darkmatter/cli/README.md` "binary overview" section still
   matches the implemented subcommands (the surface is unchanged, so
   this is a no-op check, but worth running).
6. Focused integration-test filters still work after the split:
   `cargo test -p darkmatter-cli --test compose_basic`,
   `cargo test -p darkmatter-cli --test layout_flags`, and
   `cargo test -p darkmatter-cli level2_`.
7. `cargo metadata --no-deps --format-version 1` still reports the same
   workspace package set; no crate is added or removed by this work.

## Out of Scope (Explicitly)

To keep this spec tractable, the following adjacent work is **not**
included even though it is related:

- Splitting `darkmatter/cli/src/commands/compose.rs` itself (~1000
  lines). Once Phase 0c lands (JSON serializers move to the library)
  and Phase 2 lands (helpers move to `crate::io`), `compose.rs` drops
  under 700 lines and is no longer a god-file. A further split
  (`compose/{preflight,pipeline,report}.rs`) is a separate feature.
- Splitting `darkmatter/cli/src/approval.rs`. It is 350 lines, single
  responsibility, and not a god-file.
- Splitting `darkmatter/cli/src/commands/schema/`. It is already
  split across `about.rs`, `assignment.rs`, `detect.rs`,
  `validate.rs`. The largest (`assignment.rs`, 537 lines) is on the
  edge but not over the cap.
- Replacing `color-eyre` with `miette`. Out of scope; the god-file
  problem is orthogonal to the error-reporting crate.
- Migrating any subcommand to a trait-based dispatch (e.g.
  `impl Subcommand for Render`). The `match` in `commands/mod.rs` is
  fine; the spec targets file size and responsibility split, not
  dispatch style.

## Accepted Over-Cap Exceptions

Goal 1 sets a ~500-line soft cap, not a CI gate. After Phases 1–8 (and
the review-3 harness-robustness follow-up) the six files below remain
over the cap. They are accepted exceptions — each is a
single-responsibility module that grew past the cap without becoming a
god-file. `just lint-files` reports them inline with the recorded
reason so the exception list stays explicit, reviewed, and drifts
loudly if a new over-cap file appears.

| File | Lines | Why accepted |
|------|-------|--------------|
| `darkmatter/cli/src/commands/compose.rs` | 1021 | Compose subcommand; the further split (`compose/{preflight,pipeline,report}.rs`) is a separate feature per spec non-goals. |
| `darkmatter/cli/src/commands/schema/about.rs` | 626 | One command (`md schema about`), one builder struct (`SchemaAboutReport`). Sections are visually separated report sections, not unrelated responsibilities. |
| `darkmatter/cli/src/commands/schema/assignment.rs` | 537 | Assignment parsing + schema-aware value coercion for `md schema validate`. Single responsibility shared by the CLI and library surface. |
| `darkmatter/cli/tests/code_block.rs` | 747 | Exhaustive per-flag coverage of the `md code-block` surface. Splitting by flag family would create artificial fragmentation across one cohesive subcommand. |
| `darkmatter/cli/tests/schema_validate.rs` | 555 | Per-flag coverage of the `md schema validate` surface. Same rationale as `code_block.rs`. |
| `darkmatter/cli/tests/common/level2.rs` | 539 | Shared Level 2 harness module — shim creation/integrity, sentinel polling, fixture runners, and frame/color assertion helpers for the real-terminal suite. Grew past the cap after the review-3 cross-platform shim-robustness fix (`link_or_copy` + `is_same_binary` fallback ladder). Splitting would fragment one cohesive test-harness concern. |

The god-file pattern this feature targets — "hundreds of unrelated
top-level symbols" mixed in one file — does not apply to any of these
files: each has one responsibility, and the line count is driven by
coverage of that responsibility rather than by accretion of unrelated
concerns. The `just lint-files` script (Phase 8 / ADR-5) records the
list inline so any *new* over-cap file is flagged for review without a
matching accepted-exception entry.

## Summary

The CLI has four god-files because business logic that should live in
the `darkmatter` library (or in `renderable`) kept landing in CLI
files that had no natural home for it. The fix is two-layered:

1. **Extract the leaks** (Phase 0) — eight identified pieces of
   business logic move to their owning crate. ~1100 CLI lines deleted.
2. **Split what remains by responsibility** (Phases 1–6) — `args.rs`,
   `output.rs`, `commands.rs`, `tests/cli.rs`, and
   `tests/level2_layout.rs` decompose into small source modules and
   top-level integration-test files.

The end state has no file in `darkmatter/cli/src/` over ~500 lines
except the legitimately large `commands/compose.rs`, and no file in
`darkmatter/cli/tests/` over ~500 lines. The CLI becomes a thin layer
of argument parsing, terminal plumbing, and dispatch over a library
that owns the actual behavior.
