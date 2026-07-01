---
title: biscuit-tui Sentrux Review
date: 2026-05-04
package_area: biscuit-tui
quality_signal: 0.6951
suggestions: 15
suggestions_critical: 1
suggestions_urgent: 6
---

# biscuit-tui Sentrux Review (2026-05-04)

Baseline metrics from `.sentrux/baseline.json`:

| Metric | Value | Reading |
|---|---|---|
| `quality_signal` | 0.695 | Mid-range; coupling and concentration drag the score down. |
| `coupling_score` | 0.727 | High — 32 of 44 import edges (73%) cross module boundaries. |
| `cycle_count` | 0 | **Acyclicity is clean.** No circular imports detected. |
| `god_file_count` | 0 | Sentrux's threshold not tripped, but five files are >1500 LOC (see Equality below). |
| `complex_fn_count` | 12 | Twelve high-cyclomatic functions; concentrated in `choose_one`, `choose_many`, `standalone`, `choice_render`, `input_table/table`. |
| `max_depth` | 3 | **Depth is healthy** — `lib/src/{core,components,helpers}` is a flat three-level tree. |

### Metric-by-metric framing

- **Modularity (Newman 2004):** components form one logical cluster but the leak from `choose_many` into `choose_one`'s private helpers (`HOTKEY_DISPLAY_FALLBACK`, `build_effective_hotkeys`, `first_enabled_index`, `last_enabled_index`, `modifier_only_mode`, `sticky_toggle_mode`) creates a peer-to-peer edge instead of a clean `core`/`components` partition. Cross-module edge ratio is 73%.
- **Acyclicity (Martin 2003):** zero cycles. Nothing to fix here.
- **Depth (Lakos 1996):** import depth caps at 3. Tree shape is fine; the issue is breadth (file size), not depth.
- **Equality (Gini 1912):** node properties — LOC, function count, test mass — are highly concentrated. Five files (`choose_one.rs` 2536, `choose_many.rs` 2159, `standalone.rs` 1837, `choice_render.rs` 1703, `input_table/table.rs` 1644) account for ~40% of total LOC across the package area. `complex_fn_count: 12` clusters in these same files.
- **Redundancy (Kolmogorov):** four high-impact duplications — (1) `choose_one`/`choose_many` state structs and event-handling logic, (2) `lib.rs`/`prelude.rs` re-exports, (3) `helpers/choice_builders.rs` vs `cli/option_sources.rs` parsers, (4) `cli/src/main_tmp.rs` orphan, plus hand-rolled case-conversion in `choice_normalize.rs` that duplicates the `heck` crate.

---

## biscuit-tui

The `lib/` crate. Quality drag is concentrated in five over-large files and the `choose_one` ↔ `choose_many` peer coupling.

### `critical`: Eliminate `choose_one` ↔ `choose_many` peer coupling

**Problem.** `choose_many.rs` reaches across to `super::choose_one::{HOTKEY_DISPLAY_FALLBACK, build_effective_hotkeys, first_enabled_index, last_enabled_index, modifier_only_mode, sticky_toggle_mode}`. Two siblings should never have a private-API back-channel — this is the single largest modularity violation in the package, inflates `cross_module_edges`, and makes either component impossible to refactor or remove without rewriting the other.

**Files touched.**
- `lib/src/components/choose_one.rs` (currently exports the helpers as `pub(crate)`)
- `lib/src/components/choose_many.rs:45-48` (the offending import)

**Fix.** Promote the shared helpers into a new `lib/src/components/choice_state.rs` module owned by neither. Both peers import from it; nothing in `choose_one` is `pub(crate)` for the sibling's benefit.

```rust
// lib/src/components/choice_state.rs
use std::time::Duration;
use super::choose::{ChoiceOption, HotkeyDisplayMode, HotkeySpec};

pub(crate) const HOTKEY_DISPLAY_FALLBACK: Duration = Duration::from_millis(300);

pub(crate) fn build_effective_hotkeys<V>(
    options: &[ChoiceOption<V>],
) -> (HashMap<char, usize>, HashMap<char, usize>) { /* ... */ }

pub(crate) fn first_enabled_index<V>(options: &[ChoiceOption<V>]) -> Option<usize> { /* ... */ }
pub(crate) fn last_enabled_index<V>(options: &[ChoiceOption<V>]) -> Option<usize> { /* ... */ }
pub(crate) fn modifier_only_mode(/* ... */) -> HotkeyDisplayMode { /* ... */ }
pub(crate) fn sticky_toggle_mode(/* ... */) -> HotkeyDisplayMode { /* ... */ }
```

Then in both files:

```rust
use super::choice_state::{
    HOTKEY_DISPLAY_FALLBACK, build_effective_hotkeys, first_enabled_index,
    last_enabled_index, modifier_only_mode, sticky_toggle_mode,
};
```

### `urgent`: Extract a shared `ChoiceCommonState<V>` struct

**Problem.** `ChooseOneState<V>` and `ChooseManyState<V>` differ in only **two fields** — `selected: Option<usize>` vs `Vec<bool>` and `initial_selected: Option<usize>` vs (none) — yet duplicate **15+ identical fields** (`hover`, `scroll_offset`, `ctrl_hotkeys`, `alt_hotkeys`, `label`, `theme`, `bindings`, `validation_error`, `filter`, `filter_visible`, `cached_labels`, `layout_cache`, `hotkey_display`, `hotkey_display_deadline`, `hotkey_display_override`, `hotkey_display_sticky`, `terminal_style`) plus dozens of identical builder methods (`with_label`, `with_theme`, `with_bindings`, etc.). This is the single biggest contributor to the equality (Gini) imbalance — `choose_one.rs` is 2536 LOC and `choose_many.rs` 2159 LOC, together ~20% of the package.

**Files touched.**
- `lib/src/components/choose_one.rs`
- `lib/src/components/choose_many.rs`

**Fix.** Lift the common substructure into an internal helper:

```rust
// lib/src/components/choice_state.rs (same module as the helpers above)
#[derive(Debug, Clone)]
pub(crate) struct ChoiceCommonState<V> {
    pub input: ChoiceInput<V>,
    pub hover: usize,
    pub scroll_offset: usize,
    pub ctrl_hotkeys: HashMap<char, usize>,
    pub alt_hotkeys: HashMap<char, usize>,
    pub label: Option<Label>,
    pub theme: ComponentTheme,
    pub bindings: KeyBindings,
    pub validation_error: Option<String>,
    pub filter: FuzzyFilter,
    pub filter_visible: bool,
    pub cached_labels: Vec<String>,
    pub layout_cache: ChoiceLayout,
    pub hotkey_display: HotkeyDisplayMode,
    pub hotkey_display_deadline: Option<Instant>,
    pub hotkey_display_override: Option<HotkeyDisplayMode>,
    pub hotkey_display_sticky: Option<HotkeyDisplayMode>,
    pub terminal_style: TerminalStyle,
}

pub struct ChooseOneState<V = String> {
    common: ChoiceCommonState<V>,
    selected: Option<usize>,
    initial_selected: Option<usize>,
}

pub struct ChooseManyState<V = String> {
    common: ChoiceCommonState<V>,
    selected: Vec<bool>,
}
```

Add a `with_*` macro or trait so the builder methods (`with_label`, `with_theme`, etc.) are written once. Expect a 30–40% LOC drop across the two files and a corresponding boost in equality and quality_signal.

### `urgent`: Split test modules out of the giant component files

**Problem.** Five files exceed 1500 LOC, and tests dominate them: `choose_one.rs` (2536 total / ~1530 lines of tests), `choose_many.rs` (2159 / ~1270 tests), `standalone.rs` (1837 / ~815 tests), `choice_render.rs` (1703 / ~990 tests), `input_table/table.rs` (1644 / ~685 tests). Sentrux's equality metric penalises this LOC concentration; reviewers and the `complex_fn_count` heuristic also suffer because impl code and test code interleave at the file level.

**Files touched.**
- `lib/src/components/choose_one.rs:1007-2536`
- `lib/src/components/choose_many.rs:890-2159`
- `lib/src/core/standalone.rs:1022-1837`
- `lib/src/components/choice_render.rs:713-1703`
- `lib/src/components/input_table/table.rs:960-1644`

**Fix.** For each file `foo.rs` with a giant inline test module, convert to a sibling module:

```
lib/src/components/
├── choose_one/
│   ├── mod.rs        # the production code, now ~1000 LOC
│   ├── tests.rs      # was the inline `#[cfg(test)] mod tests` body
│   └── tests_render.rs   # split further by topic
```

```rust
// choose_one/mod.rs
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_render;
```

This is mechanical (`#[path]` or directory-module conversion), keeps test discovery via cargo intact, and leaves no tests in files >1200 LOC.

### `urgent`: Decompose `choice_render::ChoiceRenderContext::render` into per-orientation modules

**Problem.** `choice_render.rs` ships a 700-line non-test body, of which `render_vertical` (lines 362–517) and `render_horizontal` (lines 518–712) are nearly the whole crate. These are the two most complex functions in the package and likely supply most of the `complex_fn_count: 12`. They share little except the ctx; splitting them flattens the function-size distribution (better Gini) without changing behaviour.

**Files touched.**
- `lib/src/components/choice_render.rs:362-712`

**Fix.** Split into `choice_render/{mod.rs, vertical.rs, horizontal.rs, badge.rs, highlight.rs}`. Move `badge_text`/`badge_span` (lines 50–122) into `badge.rs`; `build_highlighted_spans` (line 670) into `highlight.rs`. `ChoiceRenderContext::render` becomes a six-line dispatch:

```rust
pub fn render<V, F>(&self, /* ... */) where /* ... */ {
    match self.orientation {
        Orientation::Vertical => vertical::render(self, /* ... */),
        Orientation::Horizontal => horizontal::render(self, /* ... */),
    }
}
```

Inside each sub-module, split the long inner pieces (option-row painting, scrollbar, "no matches" placeholder) into private helpers. Target: no function over 80 lines.

### `urgent`: Collapse duplicated re-exports between `lib.rs` and `prelude.rs`

**Problem.** `lib/src/lib.rs:18-37` and `lib/src/prelude.rs:9-23` re-export the **identical** 30+ symbol list. Every new public type has to be added in both places; future drift is inevitable. Pure Kolmogorov redundancy.

**Files touched.**
- `lib/src/lib.rs`
- `lib/src/prelude.rs`

**Fix.** Make `prelude` the single authority and have `lib.rs` glob-re-export it:

```rust
// lib/src/lib.rs
pub mod components;
pub mod core;
pub mod helpers;
pub mod prelude;

// Mirror the prelude at the crate root so existing
// `use tui_chrome::ChooseOne;` paths keep working.
pub use prelude::*;
```

```rust
// lib/src/prelude.rs — unchanged; sole authority for the public surface
pub use crate::components::{ /* ... */ };
pub use crate::core::{ /* ... */ };
```

This removes ~30 lines of duplication and makes the public surface single-sourced.

### `important`: Decompose `core::standalone` into focused submodules

**Problem.** `standalone.rs` is 1837 LOC and packs three concerns: (a) `drive_event_loop` / `drive_event_loop_with_hint` / `drive_event_loop_with_chrome` orchestration, (b) terminal lifecycle (`prepare_terminal`, `restore_terminal`, `drain_pending_events`), and (c) inline-viewport math (`finalize_inline_viewport`, `maybe_recompute_inline_height`, `prepare_chrome_for_hint`, `hint_horizontal_extent`, `resolve_height_spec`). Three responsibilities in one file hurts both modularity and equality.

**Files touched.**
- `lib/src/core/standalone.rs`

**Fix.** Split into `core/standalone/{mod.rs, loop_driver.rs, terminal_lifecycle.rs, inline_viewport.rs}`. Keep the public symbols (`run_standalone`, `drive_event_loop`, `StandaloneState`, `HandleEvent`, `LoopExit`, `ABORTED_KIND`, `CANCELLED_KIND`) re-exported from `mod.rs` so the crate API does not change.

### `important`: Replace hand-rolled case conversions with the `heck` crate

**Problem.** `cli/src/choice_normalize.rs:185-237` reimplements `to_camel_case`, `to_pascal_case`, `to_kebab_case`, `to_snake_case`, `to_title_case` from scratch. The `heck` crate (already pulled in transitively by clap-derive) is the de-facto Rust solution and is well-tested for unicode edge cases. Removing ~50 lines of bespoke string manipulation reduces redundancy and bug surface.

**Files touched.**
- `cli/src/choice_normalize.rs:172-251`
- `cli/Cargo.toml` (add `heck = "0.5"` if not already transitively available)

**Fix.**

```rust
use heck::{ToKebabCase, ToLowerCamelCase, ToPascalCase, ToSnakeCase, ToTitleCase, ToUpperCamelCase};

pub fn apply_convention(s: &str, convention: NamingConvention) -> String {
    match convention {
        NamingConvention::Caps => s.to_uppercase(),
        NamingConvention::Lowercase => s.to_lowercase(),
        NamingConvention::CamelCase => s.to_lower_camel_case(),
        NamingConvention::PascalCase => s.to_pascal_case(),
        NamingConvention::KebabCase => s.to_kebab_case(),
        NamingConvention::SnakeCase => s.to_snake_case(),
        NamingConvention::TitleCase => s.to_title_case(),
    }
}
```

### `important`: De-duplicate parsing helpers between `helpers/choice_builders` and `cli/option_sources`

**Problem.** Bullet-list, numbered-list, dictionary, and CSV parsers are written **twice** with near-identical bodies in `lib/src/helpers/choice_builders.rs:120-220` and `cli/src/option_sources.rs:170-554`:

| Helper | `choice_builders.rs` | `option_sources.rs` |
|---|---|---|
| `strip_bullet_prefix` | 155 | 484 |
| `strip_numbered_prefix` | 164 | 497 |
| `yaml_value_to_string` | 196 | 541 |
| `options_from_csv` / `parse_csv` | 120 | 170 |
| `options_from_dictionary` / `parse_dictionary` | 178 | 511 |
| `parse_markdown_list_line` | 136 | 200 (`strip_simple_markdown_list_prefix`) |

Two implementations will diverge under maintenance pressure and any bug fix has to be applied twice.

**Files touched.**
- `lib/src/helpers/choice_builders.rs`
- `cli/src/option_sources.rs`

**Fix.** Promote the markdown/dictionary/CSV parsers in the library to be the canonical implementations (they already return typed `ChoiceOption<String>`s), then have `cli/option_sources.rs` call them via the public lib API. The CLI's `RawOption` struct should be constructed from `ChoiceOption<String>` rather than re-parsing the same source format.

### `nice-to-have`: Re-export the `helpers` module's free functions in the prelude

**Problem.** `lib/src/helpers/choice_builders.rs` exposes nine `pub fn`s (`choose_{one,many}_from_{csv,markdown_list,dictionary}` etc.) that callers will reach for, but neither `lib.rs` nor `prelude.rs` re-exports them. Forces `use tui_chrome::helpers::choice_builders::choose_one_from_csv;` while every other public item is one path segment shorter.

**Files touched.**
- `lib/src/prelude.rs`
- `lib/src/helpers/mod.rs`

**Fix.** Add to `prelude.rs`:

```rust
pub use crate::helpers::choice_builders::{
    choose_many_from_csv, choose_many_from_dictionary, choose_many_from_markdown_list,
    choose_one_from_csv, choose_one_from_dictionary, choose_one_from_markdown_list,
};
```

### `nice-to-have`: Inline `choice_layout::navigate_row` if it is a single-call helper

**Problem.** `navigate_row` (line 209 in `choice_layout.rs`) is imported separately from `ChoiceLayout` in both `choose_one.rs:41` and `choose_many.rs:42`. If it is only used in those two call sites and operates entirely on `ChoiceLayout` data, it should be a method on `ChoiceLayout` rather than a free function — fewer cross-module edges and one less symbol to import.

**Files touched.**
- `lib/src/components/choice_layout.rs:209`
- `lib/src/components/choose_one.rs:41`
- `lib/src/components/choose_many.rs:42`

**Fix.** Convert `pub fn navigate_row<V>(layout: &ChoiceLayout, ...)` into `impl ChoiceLayout { pub fn navigate_row<V>(&self, ...) }`. Drop the standalone import; call as `state.layout_cache.navigate_row(...)`.

---

## biscuit-tui-cli

The `cli/` crate. Smaller package, but carries three concrete redundancies and two over-large command files.

### `urgent`: Delete `cli/src/main_tmp.rs`

**Problem.** `cli/src/main_tmp.rs` is a 26-line scratch file that prints a generated zsh completion. It is not referenced from `cli/Cargo.toml` (the only `[[bin]]` is `main.rs`), is not declared as a module, and does not build into the crate. It is pure orphaned scaffolding — exactly the redundancy Sentrux's Kolmogorov metric flags.

**Files touched.**
- `cli/src/main_tmp.rs` (delete)

**Fix.** Remove the file:

```bash
git rm cli/src/main_tmp.rs
```

If a debugging script is genuinely useful, move it to `cli/examples/dump_completions.rs` so it is a discoverable example with a real build target.

### `urgent`: Lift duplicated source-resolution flags into `ChooseSourceArgs`

**Problem.** `ChooseOneArgs` (in `cli/src/commands/choose_one.rs:23-205`) and `ChooseManyArgs` (in `cli/src/commands/choose_many.rs:23-243`) both declare ~13 identical clap fields with **identical doc comments** (`positional`, `csv`, `list`, `rows`, `file`, `md`, `options_from_file`, `options_from_dictionary`, `label`, `label_position`, `delimiter`, `numeric_hot_keys`, `label_convention`, `value_convention`, `required`). `ChooseChromeArgs` already exists in `common_choose.rs` for the chrome flags — the source flags should be lifted into a peer struct with the same `#[command(flatten)]` pattern.

**Files touched.**
- `cli/src/commands/choose_one.rs:23-205` (consume)
- `cli/src/commands/choose_many.rs:23-243` (consume)
- `cli/src/commands/common_choose.rs` (add the new struct)

**Fix.**

```rust
// cli/src/commands/common_choose.rs
#[derive(Debug, Args, Clone, Default)]
pub struct ChooseSourceArgs {
    #[arg(value_name = "OPTIONS")]
    pub positional: Vec<String>,

    #[arg(long = "csv", alias = "options", value_name = "TEXT")]
    pub csv: Option<String>,

    #[arg(long, value_name = "TEXT")]
    pub list: Option<String>,

    #[arg(long, value_name = "TEXT")]
    pub rows: Option<String>,

    #[arg(long, value_name = "PATH")]
    pub file: Option<PathBuf>,

    #[arg(long, value_names = ["PATH", "PROP"], num_args = 2)]
    pub md: Option<Vec<String>>,

    // ...remaining shared flags...
}
```

```rust
// cli/src/commands/choose_one.rs
pub struct ChooseOneArgs {
    #[command(flatten)]
    pub source: ChooseSourceArgs,

    #[command(flatten)]
    pub chrome: ChooseChromeArgs,

    // Single-select-only:
    #[arg(long, conflicts_with = "initial")]
    pub selected: Option<String>,
}
```

Expect a >50% reduction in both command files and zero risk of doc-comment drift.

### `important`: Extract generic `command run` plumbing

**Problem.** The `run()` functions in `commands/choose_one.rs` and `commands/choose_many.rs` walk the same script: resolve raw options → normalise → build state → call `run_standalone_with_chrome` → format with `OutputMode`. The duplicated control flow is most of why those files are 847 / 926 LOC. After lifting `ChooseSourceArgs`, the bulk of `run()` becomes shareable.

**Files touched.**
- `cli/src/commands/choose_one.rs:run`
- `cli/src/commands/choose_many.rs:run`
- `cli/src/commands/common_choose.rs`

**Fix.** Add a generic helper:

```rust
// cli/src/commands/common_choose.rs
pub fn build_choose_state<V, S, F>(
    source: &ChooseSourceArgs,
    chrome: &ChooseChromeArgs,
    build: F,
) -> io::Result<(S, FrameChromeConfig, HotkeyDisplayMode)>
where
    F: FnOnce(ChoiceInput<String>) -> S,
{
    let raw = resolve_raw_options(/* ... */)?;
    let options = normalize_options(raw, /* ... */)?;
    let input = ChoiceInput::new("", "").with_options(options);
    let state = build(input);
    let chrome_cfg = build_chrome(chrome);
    let badges = resolve_hotkey_badges(/* ... */);
    Ok((state, chrome_cfg, badges))
}
```

Each subcommand's `run()` collapses to ~30 lines: build state, call `run_standalone_with_chrome`, format output.

### `important`: Split `cli/src/commands/input_table.rs` into argument and runtime modules

**Problem.** `cli/src/commands/input_table.rs` is 799 LOC with 526 lines of source plus 273 of tests. It mixes (a) a `--columns` JSON-schema parser, (b) clap arg definitions, and (c) the runtime that builds the state and dispatches. Consistent with the equality (Gini) goal, splitting helps balance file sizes across the CLI.

**Files touched.**
- `cli/src/commands/input_table.rs`

**Fix.** Split into:

```
cli/src/commands/input_table/
├── mod.rs       # public Args, run()
├── columns.rs   # parse_columns_json, ColumnSpec, validation
└── tests.rs     # the existing test body
```

Keeps the public symbols (`InputTableArgs`, `run`) at the same path so `main.rs` is unchanged.

### `nice-to-have`: Trim `cli/src/option_sources.rs` once the lib helpers are reused

**Problem.** `cli/src/option_sources.rs` is 1085 LOC; ~half of it (markdown/dictionary/CSV/YAML helpers) becomes dead once the redundancy fix above promotes the lib's parsers to the canonical implementation.

**Files touched.**
- `cli/src/option_sources.rs`

**Fix.** After landing the `helpers/choice_builders` consolidation, walk through `option_sources.rs` and delete any helper that now has a public counterpart in `tui_chrome::helpers`. Expect 300–400 LOC removed.
