---
agent: claude
phases: 6
created: 2026-06-09
start_phase: 1
archived: 2026-06-10
archived_reason: >
  Superseded by plan.md (regenerated 2026-06-10). This plan was fully executed
  and reviewed (review-1.md), but predates the spec's "Argument validation"
  section (zero-token error + 3-character minimum), which it does not cover.
  Preserved verbatim as the execution + review record of the original effort.
source_files_during_phase_1:
  - biscuit-icon/lib/src/iconify/client.rs
  - biscuit-icon/lib/src/cache/store.rs
  - biscuit-icon/lib/src/catalog.rs
  - biscuit-icon/cli/src/commands.rs
  - biscuit-icon/lib/tests/catalog.rs
  - biscuit-icon/cli/tests/cli.rs
  - biscuit-icon/cli/tests/level2_terminal.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .opencode/skill/biscuit-icon/cache.md
source_files_during_phase_2:
  - biscuit-icon/lib/src/domain/mod.rs
  - biscuit-icon/lib/src/icon.rs
  - biscuit-icon/lib/src/catalog.rs
  - biscuit-icon/lib/src/cache/mod.rs
  - biscuit-icon/lib/src/lib.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - biscuit-icon/cli/Cargo.toml
  - biscuit-icon/cli/src/args.rs
  - biscuit-icon/cli/src/main.rs
  - biscuit-icon/cli/src/commands.rs
  - biscuit-icon/cli/tests/cli.rs
  - darkmatter/lib/src/markdown/render_tree/fold.rs
docs_updated_during_phase_3:
  - biscuit-icon/docs/dependencies.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/biscuit-icon/cli.md
source_files_during_phase_4:
  - biscuit-icon/cli/src/commands.rs
  - biscuit-icon/cli/tests/cli.rs
  - biscuit-icon/cli/tests/level2_terminal.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - biscuit-icon/cli/src/commands.rs
  - biscuit-icon/cli/tests/cli.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - biscuit-icon/lib/src/cache/store.rs
  - biscuit-icon/lib/src/icon.rs
  - biscuit-icon/lib/src/catalog.rs
  - biscuit-icon/cli/src/args.rs
docs_updated_during_phase_6:
  - biscuit-icon/README.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/biscuit-icon/SKILL.md
  - .claude/skills/biscuit-icon/cache.md
  - .claude/skills/biscuit-icon/library.md
source_code:
  - biscuit-icon/lib/src/iconify/client.rs
  - biscuit-icon/lib/src/cache/store.rs
  - biscuit-icon/lib/src/catalog.rs
  - biscuit-icon/cli/src/commands.rs
  - biscuit-icon/lib/tests/catalog.rs
  - biscuit-icon/cli/tests/cli.rs
  - biscuit-icon/cli/tests/level2_terminal.rs
  - biscuit-icon/lib/src/domain/mod.rs
  - biscuit-icon/lib/src/icon.rs
  - biscuit-icon/lib/src/cache/mod.rs
  - biscuit-icon/lib/src/lib.rs
  - biscuit-icon/cli/Cargo.toml
  - biscuit-icon/cli/src/args.rs
  - biscuit-icon/cli/src/main.rs
  - darkmatter/lib/src/markdown/render_tree/fold.rs
documentation:
  - biscuit-icon/docs/dependencies.md
  - biscuit-icon/README.md
packages:
  - biscuit-icon
  - biscuit-icon-cli
---

# Execution Plan — biscuit-icon: Icons and Cache Reporting

Derived from [`spec.md`](./spec.md). Implements the `show` (default) command
redesign, `--meta` tabular metadata, inexact-match picker/list behavior, the
`cache list` / `cache clear [filter]` rework, the curated-only `domain`
command, and the supporting SQLite v2→v3 migration + Iconify metadata parsing.

## Orienting Facts (verified against the code)

- **Cache** lives in `biscuit-icon/lib/src/cache/store.rs`. Schema is at
  `user_version = 2`; migrations run inside `open_at`. `SetInfo` currently has
  `prefix, title, license, license_title, license_url, total`. `clear()` wipes
  both `icons` and `sets`.
- **Iconify client** (`lib/src/iconify/client.rs`): `CollectionMeta` parses only
  `name, license, total`; `CollectionInfo` exposes `prefix, title, license,
  total`. Author / category / tags are **not** parsed yet.
- **Icon** (`lib/src/icon.rs`): has `id()`, `body()`, `svg()`, `unicode_char()`,
  `nerd_font_char()`, `source()`, the terminal ladder (`TerminalRenderable`),
  and `render_image` under the `image` feature. **No `css()` method exists.**
- **Domain enums** (`lib/src/domain/`): 16 enums, each deriving
  `Display + EnumIter + EnumString` with `serialize_all = "snake_case"` and
  implementing `DomainIcon` (`iconify_id`, `body`, `glyph`). `icon_for_id` and
  `all_iconify_ids` already aggregate them. Canonical set names (from the
  `domain_ctor!` literals): `os, emoji, arrow, data, file, hardware, timing,
  button, control, network, dev_ops, actors, nav, sport, brand, social`.
- **CLI args** (`cli/src/args.rs`): top-level `Commands::Icons | Sets | Cache |
  Completions`; `CacheAction::Clear` (no filter). Default `icon <filter>` maps
  to `Icons` in `main.rs`.
- **CLI command bodies** in `cli/src/commands.rs`; `set_info_from_collection`
  maps the client struct into `SetInfo`.
- **Picker**: `biscuit-tui` crate (`biscuit-tui/lib`) exposes `ChooseManyState` +
  `run_standalone`. Abort (Esc) → `io::ErrorKind::ConnectionAborted`; Ctrl-C →
  `io::ErrorKind::Interrupted`; zero selection (not `required`) → `Ok(vec![])`.
  Requires a TTY.
- **Code-block highlight**: `darkmatter` crate — build a Markdown from a fenced
  ```` ```svg ```` block and call `Markdown::as_terminal(TerminalOptions)`.
- **Capability detection**: `Terminal::new()` populates `is_tty`,
  `image_support` (`ImageSupport::{None,Kitty,ITerm}`), `osc_link_support`,
  `is_nerd_font: Option<bool>`, `color_depth`. `new_optimistic` hardcodes.
- **Tests**: L1 via `assert_cmd` + `wiremock`, isolating the cache with a temp
  `HOME`; L2 via `biscuit-test-harness` (tmux/WezTerm). Recipes: `just test`,
  `just test-l2` (passes `--features image`), `just lint`, `just doctest`.

## Conventions for every phase

- Library lives in `biscuit-icon/lib`; CLI in `biscuit-icon/cli`. Binary is
  `icon`.
- Prefer **US English** in symbols/docs. No `# H1` in rustdoc.
- Build/test **targeted only**: `cargo … -p biscuit-icon` / `-p
  biscuit-icon-cli` (never a bare root `cargo build`). Use the package
  `justfile` recipes where possible.
- Never run `cargo fmt` unless explicitly asked.
- Match existing comment density; comment-only churn stays out of behavior
  commits (repo Scope discipline rule).

---

## Phase 1 — Library: set-metadata schema (v2→v3) + Iconify metadata parsing

Goal: the `sets` table and the Iconify client carry `author_name`,
`author_url`, `tags`, `category`, so `--meta`, `cache list`, and the metadata
resolver have an offline data source. Cache gains the query/clear methods the
new CLI needs.

- [x] **1.1** In `lib/src/iconify/client.rs`, extend `CollectionMeta` to
  deserialize the collection metadata fields: `author` (an object with `name`
  and `url`), `category` (String), and `tags` (`Vec<String>`). Add a serde
  struct for the author object (`#[serde(default)]` on all new fields — the
  `/collections` payload omits them for some sets).
- [x] **1.2** Add the matching fields to `CollectionInfo`
  (`author_name: Option<String>`, `author_url: Option<String>`,
  `category: Option<String>`, `tags: Vec<String>`) and populate them in
  `fetch_collections`. (Parallelizable with 1.3.)
- [x] **1.3** In `lib/src/cache/store.rs`, extend `SetInfo` with
  `author_name: Option<String>`, `author_url: Option<String>`,
  `tags: Option<String>` (comma-separated), `category: Option<String>`.
  Update every `SetInfo { … }` literal in the file's tests to compile.
- [x] **1.4** Add the **v2→v3 migration** block in `open_at` (mirror the
  existing `if version < 2` transactional pattern): inside one transaction,
  for each of `author_name`, `author_url`, `tags`, `category`, run
  `ALTER TABLE sets ADD COLUMN <col> TEXT` **only when the column is absent**
  (PRAGMA `table_info(sets)` guard, like the `total` guard) so re-runs are
  idempotent; then `PRAGMA user_version = 3`.
- [x] **1.5** Update `put_set`, `search_sets`, and `all_sets` SQL + row mapping
  to write/read the four new columns. Keep column order stable.
- [x] **1.6** Add cache query/maintenance methods used by the CLI:
  - `list_icons(&self) -> Result<Vec<CachedIcon>>` returning every row in
    `icons` (prefix, name; enough to build `Set`/`Icon`/`Categories`/`Tags`),
    ordered by `prefix, name`.
  - `set_title(&self, prefix: &str) -> Result<Option<String>>` (or reuse a
    join) so the `Set` column shows the human title and falls back to prefix.
  - `clear_filtered(&self, filter: &str) -> Result<usize>` deleting rows from
    `icons` where `lower(prefix || ':' || name)` contains `lower(filter)`,
    leaving `sets` intact (D5). Keep the existing `clear()` (full wipe) for the
    no-filter path.
  - Define a small `CachedIcon` struct (prefix, name) if helpful.
- [x] **1.7** In `cli/src/commands.rs`, update `set_info_from_collection` to map
  the new `CollectionInfo` fields into `SetInfo` (join `tags` with `, `).

### Phase 1 validation

- [x] **V1.1** New L1 tests in `store.rs`: fresh DB is `user_version = 3` and
  has all four new `sets` columns; a `v2` DB migrates to `v3` preserving
  existing rows; the migration is idempotent when columns already exist; the
  new `SetInfo` fields round-trip through `put_set` → `search_sets`/`all_sets`.
- [x] **V1.2** L1 test: `clear_filtered("home")` removes only matching `icons`
  rows (case-insensitive) and leaves `sets` untouched; `clear()` still empties
  both.
- [x] **V1.3** `wiremock` test in `client.rs`: a `/collections` payload with
  `author`/`category`/`tags` populates `CollectionInfo`; a payload omitting them
  yields `None`/empty without error.
- [x] **V1.4** `cargo test -p biscuit-icon` green; `just lint` clean for the lib.

---

## Phase 2 — Library: domain registry, CSS url(), suggestions, metadata resolver

Goal: the pure helpers the CLI needs, each independently testable. (2.1, 2.2,
2.3 are mutually parallelizable; 2.4 depends on Phase 1's `SetInfo`.)

- [x] **2.1** Add a **domain registry** in `lib/src/domain/mod.rs` (or a new
  `domain/registry.rs`):
  - `domain_sets() -> Vec<(&'static str, usize)>` — the 16 set names with
    variant counts (use `EnumIter::iter().count()` per enum via a macro mirroring
    `all_iconify_ids`).
  - `domain_variants(set: &str) -> Option<Vec<DomainVariant>>` where
    `DomainVariant { name: String, glyph: Option<Glyph>, iconify_id: &'static
    str }`; `name` comes from the enum `Display` (snake_case). Returns `None`
    when `set` is not one of the 16 names.
  - `is_domain_set(set: &str) -> bool` and a helper to resolve a
    `set:variant` string to an `Icon` infallibly *when it matches*, else `None`
    (used by `icon domain emoji:happy`). Reuse `EnumString::from_str` per enum.
  - Add a `domain_set_name_for_prefix`/`set_name` lookup if needed by the
    metadata resolver (curated `Categories` = enum-set name per D1).
- [x] **2.2** Add `Icon::css(&self) -> String` in `lib/src/icon.rs`: assemble
  the styled SVG (`self.svg()`), percent-encode per D3, and wrap as
  `url('data:image/svg+xml,<encoded>')` with single quotes. Implement a small
  hand-written encoder (no new dependency, Rule 2) that encodes at minimum
  `#→%23`, `<→%3C`, `>→%3E`, `"→%22`, `'→%27`, plus whitespace/newlines
  (`space→%20`, `\n`, `\t`, `\r`). Add a unit test asserting the exact mappings
  and that a colored icon's `#` is encoded.
- [x] **2.3** Add a **suggestion helper** for failed `:` lookups (D6) in
  `lib/src/catalog.rs`: `suggestions(cache, needle) -> Result<Vec<String>>`
  returning ids from the unified offline catalog (domain ids ∪ cached names)
  whose **name part** (substring after `:`) contains `needle` (case-insensitive).
  Reuse `all_iconify_ids` + `cache.search_names`. Offline-only; no network.
- [x] **2.4** Add a **metadata resolver** (`lib/src/catalog.rs` or a new
  `metadata.rs`) producing the `--meta` / `cache list` columns per D1:
  `IconMeta { set: String, icon: String, categories: String, tags: String,
  author: Option<String>, license: Option<String> }`.
  - `set`: cached set title joined on prefix, else the prefix.
  - `icon`: the local name.
  - `categories`: enum-set name for curated icons (via 2.1), else `N/A`.
  - `tags`: the set-level `tags` from cache, else empty.
  - `author`/`license`: from the cached `SetInfo` when populated.
  - Input is an `Icon` (or `prefix:name` + `Source`) plus the `IconCache`.

### Phase 2 validation

- [x] **V2.1** L1 tests: `domain_sets()` returns 16 entries with correct counts;
  `domain_variants("emoji")` lists the Emoji variants with glyph + iconify_id;
  `domain_variants("sport")` is `Some`; `domain_variants("nope")` is `None`;
  resolving `emoji:happy` yields an `Icon` whose id is the fluent-emoji id.
- [x] **V2.2** L1 test: `Icon::css()` for `Os::Apple.icon().color("#d97706")` contains
  `url('data:image/svg+xml,` and `%23d97706`, no raw `#`/`<`/`>`.
- [x] **V2.3** L1 test: `suggestions` over a seeded cache returns name-substring
  matches and excludes non-matches; empty for a nonsense needle.
- [x] **V2.4** L1 test: metadata resolver yields `N/A` categories +
  set-title `Set` for a network icon, and the enum-set name for a curated icon.
- [x] **V2.5** `cargo test -p biscuit-icon` green; `just lint` clean.

---

## Phase 3 — CLI: argument restructure, new deps, dispatch, exclusivity

Goal: the clap surface matches the spec; `main.rs` dispatches the new commands;
mutually-exclusive format flags are rejected. Bodies are stubbed/minimal here
and filled in Phases 4–5. **Phase 3 gates Phases 4 and 5.**

- [x] **3.1** Add CLI dependencies in `cli/Cargo.toml`:
  `biscuit-tui = { path = "../../biscuit-tui/lib" }` and
  `darkmatter = { path = "../../darkmatter/lib" }`. Update
  `biscuit-icon/docs/dependencies.md` (and root `docs/dependencies.md` if it
  enumerates per-crate deps) per the Drift rule.
- [x] **3.2** Rewrite `cli/src/args.rs` `Commands`:
  - Replace `Icons { filter, from }` with
    `Show { ids: Vec<String>, from: Option<String>, svg: bool, code_block:
    bool, css: bool, meta: bool, list: bool, pick: bool }` (keep
    `add = ArgValueCompleter::new(icon_name_completer)` on `ids`). `--nerd`
    stays the existing global flag.
  - Change `Cache` to hold `CacheAction::List` and
    `CacheAction::Clear { filter: Option<String> }` (removes the old bare
    `Clear`).
  - Add `Domain { arg: Option<String> }`.
  - Keep `Sets` and `Completions` unchanged.
  - Keep the top-level positional `filter`/`from` + the default-to-`show`
    mapping (back-compat: `icon <filter>` still works).
- [x] **3.3** Update `main.rs`: map the bare-positional default to
  `Commands::Show { ids: cli.filter.into_iter().collect(), from: cli.from,
  …defaults }`; keep the `Completions` short-circuit and tracing init.
- [x] **3.4** Add a **format-exclusivity guard**: if more than one of `--svg`,
  `--code-block`, `--css` is set, return a CLI error and exit status `1`
  (clap `ArgGroup` with `multiple(false)`, or an explicit check in `show`
  before dispatch). Spec §"Formatting Exclusivity".
- [x] **3.5** Update `commands.rs` `run`/`run_with_client` match arms to the new
  variants (temporary minimal bodies that compile; real logic in Phases 4–5).
  Preserve the injectable-client test seam.

### Phase 3 validation

- [x] **V3.1** `cargo build -p biscuit-icon-cli` compiles (with and without
  `--features image`).
- [x] **V3.2** L1 test: `icon cache clear` parses as `Cache { action: Clear {
  filter: None } }` and the **old** subgroup-free behavior is gone (no parse
  regression); `icon cache list` parses; `icon domain` parses.
- [x] **V3.3** L1 test: `icon show mdi:home --svg --css` exits `1` with a
  mutual-exclusivity error on stderr.

---

## Phase 4 — CLI: `show` command (default) — render / table / picker / formats

Goal: full `show` behavior per spec §`show`, §Multiple icons, §Inexact Matches,
§`--meta`. **Depends on Phases 2 and 3. Parallelizable with Phase 5.**

- [x] **4.1** Implement id resolution: for each arg containing `:`, resolve via
  `lookup_icon` (domain-first, then `Icon::iconify_with`). If **any** id fails,
  print an error to stderr (with suggestions from 2.3 when the failure is a
  name miss) and exit `1` — spec §Multiple icons.
- [x] **4.2** Single resolved id (no `--meta`): emit one render per the active
  format (default ladder / `--svg` / `--code-block` / `--css` / `--nerd`),
  **no table**.
- [x] **4.3** Two-or-more resolved ids (or `--meta`): emit a `Table`
  (biscuit-terminal, mirror `sets_table.rs` usage) — col 1 the
  fully-qualified `{set}:{name}`, col 2 the displayed value for the active
  format.
- [x] **4.4** Format flags:
  - `--svg` → `icon.svg()` plain text.
  - `--css` → `icon.css()` (2.2).
  - `--code-block` → darkmatter: build `Markdown` from ```` ```svg\n{svg}\n``` ````
    and `as_terminal(TerminalOptions::default())`; pass the terminal's
    color depth (leave `color_depth: None` for auto-detect).
  - `--nerd` → prefer the Nerd Font glyph (existing `nerd_font(true)` path).
  - default → ladder render (`icon.render(&term)`), which already degrades to
    SVG/identifier text off-TTY.
- [x] **4.5** `--meta` (spec §`--meta`): force a table regardless of arg count;
  columns `Set`, `Icon`, `Categories`, `Tags`, plus `Author` and `License`
  **only when populated** for the matched icon. The display column follows the
  active format flag. Build rows from the metadata resolver (2.4).
- [x] **4.6** Inexact-match dispatch for a single `<filter>` arg **without** `:`
  (spec §Inexact Matches):
  - Gather matches = offline (`catalog::offline_icons`) ∪ online
    (`client.search_icons`), deduplicated (reuse the existing merge logic from
    the old `icons`).
  - `0` matches → error `"no icons match"`, exit `1`.
  - `--list` (any TTY state) **or** non-TTY → list every match one per line as
    `<render>  <id>`.
  - `--pick` → force the picker; **error in non-TTY**.
  - TTY + exactly `1` match → auto-render that single icon.
  - TTY + `≥2` matches (and not `--list`) → launch the `choose_many` picker.
- [x] **4.7** `<id>` with `:` that does **not** resolve → error
  `"icon does not exist"` + suggestion list (names matching the substring after
  `:`), exit `1` (spec §Inexact Matches bullet 2).
- [x] **4.8** Picker integration (`biscuit-tui` `ChooseManyState` +
  `run_standalone`): options are the matched ids (label = id). On result:
  - `Ok(picked)` with `len == 1` → render that single icon.
  - `Ok(picked)` with `len ≥ 2` → render as a table (same as 4.3).
  - `Ok(picked)` empty (picked zero) → no-op, exit `0`.
  - `Err(Interrupted)` / `Err(ConnectionAborted)` (Ctrl-C / Esc) → abort,
    exit `130`. (Thread an explicit `process::exit(130)` or a typed error the
    `main` mapping honors — document the chosen mechanism.)
- [x] **4.9** TTY detection uses `Terminal::new().is_tty` (or
  `std::io::stdout().is_terminal()`); keep the `BISCUIT_TERM_WIDTH/HEIGHT`
  override path used by `sets` for deterministic table widths in tests.

### Phase 4 validation (L1 via assert_cmd + wiremock + temp HOME)

- [x] **V4.1** `icon show mdi:home` (mock) — single render, no table.
- [x] **V4.2** `icon show mdi:home mdi:account` — 2-row table.
- [x] **V4.3** `icon show mdi:home --svg` — plain `<svg…>` text, no ladder;
  `--css` — `url('data:image/svg+xml,…')`; `--code-block` — output differs from
  raw SVG (highlighting markers present).
- [x] **V4.4** `icon show mdi:home --meta` — 1-row table with `Set`, `Icon`,
  `Categories`, `Tags` (and `Author`/`License` when the seeded set has them).
- [x] **V4.5** `icon show <filter>` piped/non-TTY — lists every match one per
  line; `--list` does the same; `--pick` in non-TTY errors with exit `1`.
- [x] **V4.6** `icon show homex` — error + exit `1`; `icon show uil:bad-name` —
  error `"icon does not exist"` + a suggestion line, exit `1`.
- [x] **V4.7** L2 (`just test-l2`, tmux/WezTerm) — `icon show grinning` renders
  the Unicode glyph; an ambiguous filter on a TTY launches the picker (assert
  the prompt frame). Gate with `require_level!`.

---

## Phase 5 — CLI: `domain` and `cache` commands

Goal: spec §Domain and §Cache. **Depends on Phases 1, 2, 3. Parallelizable with
Phase 4.**

- [x] **5.1** `icon domain` (no arg) — table of the 16 enum names; columns
  `Domain Set`, `Variant Count` (from `domain_sets()`, 2.1). No network, no
  cache.
- [x] **5.2** `icon domain <enum>` — list variants of that enum; columns
  `Variant`, `Glyph`, `Iconify ID` (from `domain_variants`). When `<enum>`
  is not a known set, treat as a substring filter over the 16 names:
  `0` matches → error; `≥1` → list matching set names (spec §Domain — no
  picker, offline-only).
- [x] **5.3** `icon domain <enum>:<variant>` — render the single curated icon
  infallibly when it matches (resolver from 2.1). A `prefix:variant` whose
  prefix is **not** a curated enum (e.g. `sport:baseball` — `sport` IS an enum,
  but the example to error on is a non-enum prefix) → error
  `"not a curated enum"`, exit non-zero. Use `icon domain emoji:happy` as the
  happy-path example.
- [x] **5.4** `icon cache list` — table of cached icons (from `list_icons` +
  metadata resolver); columns `Set`, `Icon`, `Display`, `Categories`, `Tags`.
  The `Display` column is **omitted entirely** (D4) unless the terminal can
  render a visual: `--nerd`/`ICON_NERD_FONT` set, or the icon has a curated
  Unicode glyph, or (`image` feature on **and** `term.image_support !=
  None`). When omitted, columns are `Set`, `Icon`, `Categories`, `Tags`.
- [x] **5.5** `icon cache clear` (no filter) — full wipe of `icons` **and**
  `sets` via `clear()`; print confirmation. `icon cache clear <filter>` —
  `clear_filtered(filter)` (icons only, D5); print the count cleared.

### Phase 5 validation

- [x] **V5.1** L1: `icon domain` lists 16 sets with counts; `icon domain emoji`
  lists Emoji variants with glyph + iconify id; `icon domain emoji:happy`
  renders (exit `0`); `icon domain sport:baseball` errors non-zero (per
  acceptance row); a non-matching `icon domain zzz` errors.
- [x] **V5.2** L1: `icon cache list` over a seeded temp-HOME cache shows the
  rows; with no visual capability the `Display` column is absent; with
  `ICON_NERD_FONT=1` (or a Unicode-glyph icon) it is present.
- [x] **V5.3** L1: `icon cache clear` empties both tables; `icon cache clear
  home` removes only matching `icons` rows and leaves `sets` rows (assert via a
  follow-up `cache list` / direct `IconCache` read).
- [x] **V5.4** L1: the removed bare `icon cache clear` form — confirm
  `cache clear` now means full wipe and parses cleanly (covered with V3.2).

---

## Phase 6 — Integration sweep, docs, drift, final validation

Goal: every acceptance row green; docs and skills reflect the new surface; the
whole package area builds/lints/tests clean.

- [x] **6.1** Walk the spec §Acceptance Criteria list and ensure each row has a
  covering test (L1 where possible, L2 for TTY-only picker/glyph rows). Fill any
  gap.
- [x] **6.2** Update **README** (`biscuit-icon/README.md` and the untracked
  `biscuit-icon/lib/README.md`): document `show` (default), the format flags,
  `--meta`, the inexact-match/picker behavior, `domain` (curated-only),
  `cache list` / `cache clear [filter]`, and the back-compat notes
  (spec §Backwards Compatibility).
- [x] **6.3** Update the **skill** docs under
  `.claude/skills/biscuit-icon/` (`cli.md`, `cache.md`, and `SKILL.md` if the
  command table changes), then regenerate each edited skill file's `hash:`
  frontmatter with `md hash <file>` (per the skill-hash rule). Mirror to the
  global `~/.claude/skills/biscuit-icon/` copy if the workflow requires it.
- [x] **6.4** Update `biscuit-icon/docs/dependencies.md` for the new
  `biscuit-tui` + `darkmatter` CLI deps (and `cache.md`'s schema section for
  `user_version = 3`).
- [x] **6.5** Run the full gate from the package area:
  `just build`, `just test`, `just test-l2`, `just lint`, `just doctest`.
- [x] **6.6** Comment/doc pass over every symbol whose behavior changed
  (migration block, `SetInfo`, `clear`/`clear_filtered`, the new CLI commands)
  — fix or delete drifted `///`/`//` comments in the same change (repo
  Authoring-discipline rule).

### Phase 6 validation (definition of done)

- [x] **V6.1** Every spec §Acceptance Criteria row maps to a passing test or a
  demonstrated CLI behavior; note any row that is L2/manual-only.
- [x] **V6.2** `just build && just test && just test-l2 && just lint && just
  doctest` all pass for `biscuit-icon` + `biscuit-icon-cli`.
- [x] **V6.3** No `serde_yaml`/format conversions added by hand (use
  `biscuit-file` if any arise); no raw escape codes emitted (rendering goes
  through `Prose`/`Table`/`TerminalImage`).
- [x] **V6.4** Out-of-scope items confirmed untouched: `icon sets`, shell
  completions, `domain` network lookups, `cache show`, and any `--meta` column
  beyond the listed set (spec §Out of Scope).

---

## Dependency graph (phase level)

```
Phase 1 ─┐
         ├─► Phase 2 ─┐
         │            ├─► Phase 4 ─┐
Phase 3 ─┴────────────┤            ├─► Phase 6
                      └─► Phase 5 ─┘
```

- **Phase 1 ∥ Phase 3** can start together (lib schema vs CLI arg shape are
  independent until wiring).
- **Phase 2** needs Phase 1 only for task 2.4 (metadata resolver); 2.1–2.3 may
  proceed alongside Phase 1.
- **Phase 4 ∥ Phase 5** once Phases 2 and 3 land.
- **Phase 6** closes after 4 and 5.

## Risks / decisions to confirm during execution

- **Picker exit-code plumbing (4.8).** `run()` returns `Result`; signalling
  `130` for abort means either an explicit `process::exit(130)` inside `show`
  or a typed error the `main` mapping inspects. Pick one and keep it consistent
  with the existing `exit(1)` path in `main.rs`.
- **Table cells with image output (4.3).** A multi-line terminal-image render
  inside a table cell will not align cleanly. Acceptance tests run off-TTY (the
  ladder degrades to glyph/identifier text), so the common path is fine; treat
  image-in-cell alignment as a known cosmetic limitation, not a blocker.
- **Iconify `/collections` metadata shape (1.1).** Field names for author
  (`author.name`/`author.url`), `category`, `tags` should be confirmed against a
  live/sample payload or the iconify skill doc before locking the serde structs;
  keep all new fields `#[serde(default)]` so an unexpected shape degrades to
  empty rather than failing the whole `sets` fetch.
- **CSS encoder scope (2.2).** Encoding only the spec-mandated characters plus
  whitespace is sufficient for the acceptance test; if a real icon body
  contains other URL-hostile bytes, widen the encoder set (still no new
  dependency).
