# Merge Conflict Resolution Plan — `main` → `renderable`

**Date:** 2026-06-04
**Merge:** `main` (`b46ee18fe`, *theirs*) into `renderable` (`ec38d073c`, *ours/HEAD*)
**Merge base:** `fbe87cd0f`

## Context

Two important, independent streams of work diverged at `fbe87cd0f`:

- **`renderable` (ours)** — the tree-rendering architectural shift. Public
  `Markdown::as_terminal` / `as_html` / `DarkmatterPage::render` now route
  through the render-tree document renderers; the **legacy `pulldown-cmark`
  event-stream serializers, `RuleProcessor`, and `for_terminal` are DELETED**
  (tree-cutover Phase 5). Snapshot/parity baselines were regenerated. Harness
  work (`biscuit-test-harness`) and L2 layout test assertions were updated.
- **`main` (theirs)** — new Darkmatter features: the composition engine with
  **remote cache + URL references** (`::file`/`::code` over HTTP(S), `FetchPolicy`,
  `--allow-host`, `--cache-root`), plus independent maintenance to the harness
  prompt-detection and the rust-testing benchmarking docs (restructured into
  `criterion.md` + `performance-testing.md`, deleting `benchmarking.md`).

Both streams must survive. The body of `main`'s feature work **already
auto-merged cleanly** (it lives in non-conflicting files/sections — e.g. the
`## Remote URL Referencing` section is present in the merged
`darkmatter/SKILL.md` body, and the remote-cache source landed without
conflict). The six conflicts below are all in places where **both branches
edited the same lines** — mostly test assertions, harness docs, and skill
frontmatter.

### Guiding principle

> Where the conflict is about **how renderable's new render-tree path is
> tested or described**, ours (HEAD) wins — because the legacy code main's
> side still references (`for_terminal`, the legacy serializer) **no longer
> exists** and the snapshot baselines were regenerated for the new path.
> Where the conflict is **main's new feature content or a strictly better
> test helper that compiles against the new path**, theirs wins. Where both
> converged on the same fix, keep the cleaner form. Additive doc rows from
> both sides are unioned.

## Conflicted files (6)

| # | File | Type | Resolution |
|---|------|------|------------|
| 1 | `darkmatter/lib/tests/render_tree_parity.rs` | modify/delete | **Delete** (`git rm`) |
| 2 | `darkmatter/cli/tests/level2_layout.rs` | content (2 hunks) | **Take main** (both hunks) |
| 3 | `darkmatter/lib/tests/layout_snapshots.rs` | content (3 hunks) | **Hand-merge** (main, main, HEAD) |
| 4 | `biscuit-test-harness/src/lib.rs` | content (2 hunks) | **Take HEAD** (both hunks) |
| 5 | `.claude/skills/darkmatter/SKILL.md` | content (frontmatter only) | **Hand-merge + rehash** |
| 6 | `.claude/skills/rust-testing/SKILL.md` | content (2 hunks) | **Hand-merge + rehash** |

---

## 1. `darkmatter/lib/tests/render_tree_parity.rs` — DELETE

**Conflict shape:** modify/delete. Ours (HEAD) **deleted** the entire 1355-line
file (commit `b2b957ef2`, "remove legacy render_tree_parity integration test",
Phase 5). Theirs (main) **modified** it — only the `tree_terminal_options()`
helper, swapping `TerminalRenderOptions::default()` for a
`Terminal::new_optimistic(80)` + `RenderStrictness::default()` construction.

**Why delete wins:** the file is a *parity harness* that renders each fixture
through **both** the new render-tree path **and** the legacy serializer
(`legacy_terminal()` → `for_terminal`, `as_html` legacy) and asserts equality.
The entire legacy half of that comparison was deleted in Phase 5; `for_terminal`
no longer exists. main's diff only refines how the *tree* side builds its
options — it does **not** add independent coverage, and the file as a whole
**cannot compile** against renderable's branch (it imports the deleted
`for_terminal`). The parity it guarded is now structurally guaranteed (one
renderer, no second path to diverge from).

**Action:**

```bash
git rm darkmatter/lib/tests/render_tree_parity.rs
```

> Note: a comment in `entrypoints.rs:1420` references an inline
> `render_tree_parity_raw_html` unit test — that is a *different*, in-`src`
> test and is unaffected by deleting this integration file.

---

## 2. `darkmatter/cli/tests/level2_layout.rs` — TAKE MAIN (both hunks)

Inside `fn level2_ul_color_inherits_into_li_body()`. Both sides rewrote the
`ul.color` inheritance assertions, differently:

- **HEAD** scans raw SGR strings (`red_semi`/`red_colon`, ordering, "reset must
  reopen") — a span-boundary heuristic.
- **main** uses the helper `foreground_at_text(&frame.raw, needle)` and asserts
  the resolved foreground at each body equals `red_500 = Some((251, 44, 54))`.

**Why main wins:**

- The helper `fn foreground_at_text(raw, needle) -> Option<Option<(u8,u8,u8)>>`
  (defined at ~line 1746, **outside** the conflict, present in the working tree)
  computes the *effective* foreground color at a text position and parses both
  the semicolon (`38;2;R;G;B`) and ITU colon (`38:2::R:G:B`) forms — so it is
  inherently tolerant of WezTerm SGR re-emission collapsing. It is strictly more
  robust than HEAD's reset/reopen heuristic and is a *stronger* statement of the
  invariant ("both bodies resolve to red-500").
- The two hunks are **interdependent**: HEAD's hunk-1 defines locals
  (`alpha_at`, `beta_at`, `red_semi`, `red_colon`) consumed in hunk-2, so they
  cannot be split. Verified: **no code after the second hunk references any of
  HEAD's locals** (the only post-hunk code touches `frame.plain`), so taking
  main verbatim leaves no dangling references.
- `frame` is defined before the hunks (line 1812); `red_500` is defined inside
  main's hunk-1. Compiles cleanly.
- Taking main also keeps `foreground_at_text` (and its `apply_sgr_foreground`
  helper) **used** — taking HEAD would orphan them as dead code.

**Exact resolved region** (replace `<<<<<<< HEAD` … `>>>>>>> main` spanning both
hunks, including the shared `frame.raw` / `);` tails, with):

```rust
    let red_500 = Some((251, 44, 54));
    assert_eq!(
        foreground_at_text(&frame.raw, "listbodyalpha").flatten(),
        red_500,
        "first list body must inherit ul.color. raw={:?}",
        frame.raw
    );
    assert_eq!(
        foreground_at_text(&frame.raw, "listbodybeta").flatten(),
        red_500,
        "second list body must inherit ul.color. raw={:?}",
        frame.raw
    );
```

Result is byte-identical to main's committed version of the function.

---

## 3. `darkmatter/lib/tests/layout_snapshots.rs` — HAND-MERGE (3 hunks)

All three hunks are inside / around `fn zero_config_prose_snapshot()`.

### Hunk 1 (~line 6) — top-of-file `#![allow(deprecated)]` comment → **take main**

Both sides add the **same** `#![allow(deprecated)]` attribute with different
explanatory comments. No compile/snapshot consequence. Take main's (more
current — references the deferred Spec A `Layout` migration):

```rust
// Exercises `DarkmatterPage`'s still-active `Page*` types; migration to
// `renderable::layout::Layout` is the deferred Spec A milestone. Mirrors the
// library's own allow in `darkmatter/lib/src/layout/mod.rs`.
#![allow(deprecated)]
```

### Hunk 2 (~line 98) — **take main** (`force_color`), **drop HEAD's local `use`**

- HEAD adds a **function-local** `use darkmatter::markdown::output::terminal::{ColorDepth, TerminalOptions};`.
- main adds `let _env = ColorSnapshotEnv::force_color();`.

main's surrounding (non-conflicted) scaffolding already added a **top-level**
`use darkmatter::markdown::output::terminal::{ColorDepth, TerminalOptions};`
(file line ~22) plus `#[serial_test::serial(layout_snapshot_env)]` on the fn.
Therefore HEAD's function-local `use` would be a **duplicate import** → keeping
it risks a redundant/duplicate-import failure under `-D warnings`. Drop it; take
main's `force_color` guard (matches the file's convention — sibling tests
`end_to_end_example_snapshot` and `pronounced_background_snapshot` both force
color). Resolved text:

```rust
    let _env = ColorSnapshotEnv::force_color();
```

### Hunk 3 (~line 110) — **take HEAD** (`as_terminal`)

- HEAD: `let direct_out = md.as_terminal(TerminalOptions::default()).unwrap();`
- main: `darkmatter::markdown::output::terminal::for_terminal(&md, terminal_options)`
  with `terminal_options.color_depth = Some(ColorDepth::TrueColor)`.

**`for_terminal` is DELETED** in renderable's branch — main's side **cannot
compile**. `Markdown::as_terminal` (`darkmatter/lib/src/markdown/mod.rs:654`)
is the only valid API. Take HEAD:

```rust
    let direct_out = md.as_terminal(TerminalOptions::default()).unwrap();
```

### Snapshot-baseline safety (the decisive check)

The `insta::assert_snapshot!` in this fn snapshots **`pinned`** (a
`max_width=120` + `ColorDepth::TrueColor` render via `as_terminal`), **not**
`direct_out`. The committed baseline
`darkmatter/lib/tests/snapshots/layout_snapshots__zero_config_prose_snapshot.snap`
(auto-resolved to HEAD; `expression: pinned`, body `\x1b[1m# Hello World\x1b[0m…`)
was **regenerated by renderable** (commit `b1c99f1ef`). main's snapshot
(`expression: page_out`, `\x1b[38;2;…m█ …`) is from the **deleted** renderer and
would fail. The above resolution reproduces the committed `pinned` baseline
exactly. `ColorDepth` remains **used** (lines ~106 and ~136), so no unused-import
warning.

**Resulting `zero_config_prose_snapshot` after resolution:**

```rust
#[test]
#[serial_test::serial(layout_snapshot_env)]
fn zero_config_prose_snapshot() {
    let _env = ColorSnapshotEnv::force_color();
    let term = Terminal::new_optimistic(120);
    let page = DarkmatterPage::new(&term).with_color_depth(ColorDepth::TrueColor);
    let md: Markdown = "# Hello World\n\nSome prose here.\n".into();

    let page_out = page.render(&md).unwrap();
    let direct_out = md.as_terminal(TerminalOptions::default()).unwrap();
    assert_eq!(page_out, direct_out);

    let mut pinned_opts = TerminalOptions::default();
    pinned_opts.max_width = Some(120);
    pinned_opts.color_depth = Some(ColorDepth::TrueColor);
    let pinned = md.as_terminal(pinned_opts).unwrap();
    insta::assert_snapshot!(pinned);
}
```

> Keep the top-level `use darkmatter::markdown::output::terminal::{ColorDepth,
> TerminalOptions};` (line 22, non-conflicted) and the
> `#[serial_test::serial(layout_snapshot_env)]` attribute — they are present in
> the merged working tree and are what make dropping HEAD's function-local `use`
> safe. Also preserve the non-conflicted `// Snapshot a width- and color-pinned
> render…` comment above the `pinned_opts` block verbatim (the function snippet
> above abbreviates it for illustration only — do not rewrite it).

---

## 4. `biscuit-test-harness/src/lib.rs` — TAKE HEAD (both hunks)

**Convergent change.** Both branches independently implemented the *same* fix —
`wait_for_prompt` must scan the **last non-blank line** because tmux's
`capture-pane` pads with trailing blanks (renderable: `4ec3ebcaa`; main:
`1b82864a2`; matches the documented `wait_for_prompt tmux padding` learning).
Only the doc-comment wording and variable naming differ; behavior is identical.

- **Hunk 1 (doc `## Notes`):** take HEAD — its note is more concrete (explains
  the 5 s timeout burn multiplied across `bt` invocations).
- **Hunk 2 (code):** take HEAD — the more compact form
  `if let Some(last_line) = frame.plain.lines().rev().find(|l| !l.trim().is_empty()) { let trimmed = last_line.trim_end();`.

Either side is functionally correct; HEAD is chosen for conciseness and to
minimize churn in renderable's own harness work. No further verification needed
beyond a compile.

---

## 5. `.claude/skills/darkmatter/SKILL.md` — HAND-MERGE frontmatter + rehash

**Only the frontmatter `hash` / `last_updated` conflicted.** The body
auto-merged and already contains **both** main's `## Remote URL Referencing`
section **and** renderable's tree-cutover Phase 5 notes (verified present).

Resolve the frontmatter block by keeping the HEAD shape (with `last_updated`),
set the date to today, then **regenerate the hash** (the merged body differs
from either parent, so the old hashes are stale regardless):

```yaml
hash: <regenerated>
last_updated: 2026-06-04
```

After removing the markers, run:

```bash
md hash --save .claude/skills/darkmatter/SKILL.md
```

(Per repo convention, also mirror to `~/.claude/skills/darkmatter/SKILL.md` if
that global copy is kept in sync — outside the merge, optional.)

---

## 6. `.claude/skills/rust-testing/SKILL.md` — HAND-MERGE (2 hunks) + rehash

### Hunk 1 (frontmatter ~line 7) — rehash

Same as #5: keep HEAD shape, `last_updated: 2026-06-04`, regenerate hash after
the body is finalized.

### Hunk 2 (Topic Pages table ~line 216) — **union both sides**

Both branches edited the same table:

- **HEAD added:** the *L2 Apple Terminal pitfalls* row
  (`apple-terminal-harness-pitfalls.md` — file **exists**).
- **main:** added a `biscuit-test-harness` cross-ref note to the WezTerm row,
  **removed** the `Benchmarking | benchmarking.md` row (file **deleted** by main —
  confirmed absent), and **added** `performance-testing.md` and `criterion.md`
  rows (both **exist**).

Merge = keep HEAD's Apple Terminal row + main's WezTerm cross-ref note + main's
Criterion/Performance rows; **drop the Benchmarking row** (target file gone).
Resolved table:

```markdown
| Topic | File |
|-------|------|
| L2 WezTerm capture gotchas (SGR collapsing, semicolon vs colon form). For backend selection / harness API, load the `biscuit-test-harness` skill via the Skill tool. | `wezterm-harness-pitfalls.md` |
| L2 Apple Terminal pitfalls (`do script` reuse, focus-steal, **resolved:** orphan leaks, plain-text capture) | `apple-terminal-harness-pitfalls.md` |
| CLI output (channels, color modes, completions, snapshots) | `cli-output-testing.md` |
| TUI rendering and event/reducer tests | `tui-testing.md` |
| Browser tests (computed-style assertions) | `browser-testing.md` |
| Integration tests | `integration-tests.md` |
| Unit tests | `unit-tests.md` |
| Snapshots and redaction | `snapshots.md`, `snapshot-redaction.md` |
| Doc tests | `doc-tests.md` |
| Mocking | `mocking.md` |
| Property testing | `property-testing.md` |
| Fuzzing | `fuzzing.md` |
| Performance testing tool choice (Criterion vs Divan) | `performance-testing.md` |
| Criterion benchmarking (getting started → deep dive → Bencher) | `criterion.md` |
| Nextest details | `nextest.md` |
```

Then:

```bash
md hash --save .claude/skills/rust-testing/SKILL.md
```

---

## Execution order

1. **Code files first** (so the build is green before touching docs):
   1. `git rm darkmatter/lib/tests/render_tree_parity.rs`
   2. Resolve `level2_layout.rs` (take main, both hunks).
   3. Resolve `layout_snapshots.rs` (main, main, HEAD per hunk).
   4. Resolve `biscuit-test-harness/src/lib.rs` (take HEAD, both hunks).
2. **Skill docs:**
   5. Resolve `darkmatter/SKILL.md` frontmatter.
   6. Resolve `rust-testing/SKILL.md` frontmatter + topic table.
3. `git add -A` the resolved files (the `git rm` already stages the deletion).

## Verification (success criteria — loop until all green)

```bash
# Compiles — the load-bearing check (for_terminal removal, no dangling vars, no dup import)
just build darkmatter            # or: cargo build -p darkmatter -p darkmatter-cli
cargo build -p biscuit-test-harness

# Lint — catches unused/duplicate imports and dead code under -D warnings
just lint darkmatter

# L1 + snapshots — must match the regenerated baselines
just test darkmatter             # includes layout_snapshots insta assertions

# L2 layout test — real terminal; run via the broker recipe ONLY, never raw cargo
just test-l2 darkmatter          # exercises level2_ul_color_inherits_into_li_body
```

- **Build green** for `darkmatter`, `darkmatter-cli`, `biscuit-test-harness`.
- **No live call to the deleted renderer remains.** Grep for the *qualified*
  symbol only — a bare `grep for_terminal` yields 40+ false positives from
  unrelated names (`render_markdown_for_terminal`, `GraphExpression::for_terminal`,
  the mermaid/image `*_for_terminal` helpers):

  ```bash
  grep -rn 'output::terminal::for_terminal(' --include='*.rs' .
  ```

  Must be **empty** after resolution. (Pre-merge, the only two call sites are
  `layout_snapshots.rs:113/122` inside the conflict — both removed by taking
  HEAD on hunk 3 — plus the deleted `render_tree_parity.rs`.) Stale **doc-comment**
  mentions of `for_terminal` in `darkmatter/lib/src/markdown/output/terminal.rs:159`,
  `layout/mod.rs:35`, and `layout/page.rs:838` are pre-existing, harmless, and
  out of scope — optionally clean up in a follow-up, not part of this merge.
- **`layout_snapshots` insta snapshots pass** against the committed
  (HEAD/render-tree) baselines — no `.snap.new` produced.
- **`level2_ul_color_inherits_into_li_body` passes** (or skips cleanly if no L2
  harness is available — never raw-`cargo` it; see rust-testing skill).
- **No conflict markers** anywhere:
  `grep -rn '^<<<<<<<\|^=======\|^>>>>>>>\|^||||||| ' .` → empty.
- **Both SKILL.md hashes regenerated** (`md hash` reports body hash matches).

## Execution addendum — 7th conflict (semantic, git-undetectable)

Discovered while running the L1 suite after resolving the six marked conflicts.
`darkmatter-cli::cli test_get_malformed_frontmatter_renders_status_block_with_offending_line`
failed — a **semantic merge conflict** git cannot flag because the two edits
live in *different* files:

- **main** (`22fc507ff`, "fix frontmatter parse error line numbers and **add
  YAML highlighting**") added `highlight_yaml_lines` in
  `darkmatter/lib/src/markdown/errors/blocks.rs`, so the offending YAML line in
  a frontmatter-parse status block is now syntax-highlighted (per-token SGR).
- **renderable** added the test in `darkmatter/cli/tests/cli.rs`, asserting the
  **contiguous plain substring** `'@' magic lookup emits results`. It passed
  pre-merge (plain rendering) but the highlighting now interleaves SGR escapes
  between characters, breaking the literal match.

Both behaviors are wanted. **Resolution: make the assertion ANSI-tolerant**
(strip ANSI via `darkmatter::testing::strip_ansi_codes` before the
`contains` checks — the idiom already used elsewhere in `cli.rs`, and matching
the rust-testing guidance "assert visible text on the *plain* form"). This
preserves main's highlighting feature *and* the test's intent. No production
code changed. Verified: the test passes, the full darkmatter L1 suite is green.

> Lesson for future merges of these two streams: a green build is necessary but
> not sufficient — run the L1 suite, because rendering-behavior changes on one
> side can break assertions on the other without any conflict marker.

## Risk notes / watch-items

- **`for_terminal` fallout beyond these files.** main's feature commits assume
  the legacy renderer in places that auto-merged. If the build surfaces *other*
  `for_terminal` / `RuleProcessor` references that main reintroduced in
  non-conflicting files, treat them the same way (port to `as_terminal` /
  the render-tree entrypoints). Run the grep above repo-wide after building.
- **Remote-URL feature interaction.** main's composition remote-cache/URL code
  merged cleanly but was never compiled against renderable's tree. The
  `just build`/`just test darkmatter` step is what proves it integrates; if it
  fails, the failure is in genuinely-merged feature code, **not** in these six
  resolutions.
- **Skill hashes are not self-verifying** for the frontmatter segment (the
  `hash` key hashes itself); only the body segment is a fixed point. Regenerate
  with `md hash --save` and don't hand-edit the hash.
- **Snapshot determinism** for `zero_config_prose_snapshot` is guaranteed by the
  *pinned* options, not by `force_color` — but keep `force_color` anyway for
  parity with sibling tests and to avoid a reviewer flag.
