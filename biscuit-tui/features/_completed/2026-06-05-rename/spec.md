---
status: ready for planning and implementation
reviewed: true
review_iterations: 3
---

# Spec: Rename `tui-chrome` to `biscuit-tui`

**Date:** 2026-06-05
**Status:** Ready for planning and implementation
**Area:** `biscuit-tui/`

> **Review note:** This inline review makes the rename boundary explicit,
> corrects the Cargo dependency-key explanation, adds omitted live
> documentation and planning surfaces, and defines a reproducible residual
> reference check. The rename is an intentional compile-time breaking change;
> no compatibility alias will preserve the old Rust import path.

## Goal

Rename the two crates in the `biscuit-tui/` package area so the package
names match the directory/area name (a historical inconsistency where the
crates were named `tui-chrome*` instead of `biscuit-tui*`):

| Role | Current crate name | New crate name | Import path change |
|------|--------------------|----------------|--------------------|
| Library | `tui-chrome` | `biscuit-tui` | `tui_chrome::` → `biscuit_tui::` |
| CLI | `tui-chrome-cli` | `biscuit-tui-cli` | n/a (no public lib) |

**Unchanged on purpose:**

- The CLI **binary name stays `question`** (`[[bin]] name = "question"`).
  No completion scripts, `cargo_bin("question")` calls, install recipes,
  or end-user invocations change.
- The **area/directory name `biscuit-tui/`** is already correct.
- Crate **version** (`0.1.0`) and **license** are unchanged.
- Public Rust types, modules, functions, behavior, CLI flags, output formats,
  and exit codes are unchanged.

> **Cargo dependency note.** Without an explicit `package = "..."`, a
> dependency table key is both the package lookup name and the Rust crate name
> visible to the dependent target (with hyphens normalized to underscores).
> Therefore the package rename requires changing each dependency key from
> `tui-chrome` to `biscuit-tui`, and every `tui_chrome::…` Rust path to
> `biscuit_tui::…`. The dependency `path` values do not change.

## Compatibility Decision

This is a clean internal breaking rename:

- Do not use Cargo's `package = "biscuit-tui"` syntax to retain a
  `tui-chrome` dependency alias.
- Do not add a compatibility crate or a `tui_chrome` re-export module.
- Update all live workspace callers in the same change.

The repository does not treat the old names as a supported release contract:
both packages are marked `release = false` in `release-plz.toml`. Preserving
an alias would leave two names for the same library and defeat the goal of
making the package area consistent. Any untracked out-of-workspace consumer
must update its dependency key and imports when it adopts this change.

## Success Criteria

- `sniff repo packages --package-area biscuit-tui` reports
  `biscuit-tui-cli, biscuit-tui` (no `tui-chrome*` remnants).
- `cargo metadata --no-deps` lists `biscuit-tui` and `biscuit-tui-cli`.
- `just build`, `just test`, `just doctest`, and `just lint` pass in
  `biscuit-tui/`, `claudine/`, and `biscuit-icon/` (the current external
  workspace callers are `claudine-cli` and `biscuit-icon-cli`).
- The residual-reference command in [Verification](#verification) — which
  searches only the **stale** identifiers `tui_chrome|tui-chrome` — returns no
  matches. `biscuit-tui` is the intended live package/area name and is expected
  to remain in manifests, docs, workflows, skills, and source comments, so it is
  not a residual identifier. Historical records and this feature's own records
  (the rename specification, plans, reviews, and validation records) are
  intentionally excluded.

---

## Part 1 — Changes inside the `biscuit-tui/` package area

### 1.1 Manifests

- **`biscuit-tui/lib/Cargo.toml`**
  - `name = "tui-chrome"` → `name = "biscuit-tui"`
- **`biscuit-tui/cli/Cargo.toml`**
  - `name = "tui-chrome-cli"` → `name = "biscuit-tui-cli"`
  - dependency `tui-chrome = { path = "../lib" }` →
    `biscuit-tui = { path = "../lib" }`
  - `[[bin]] name = "question"` — **unchanged**
  - benchmark opt-out `reason` strings mention "tui-chrome" — update prose.

### 1.2 Rust source — import path `tui_chrome` → `biscuit_tui`

All `use tui_chrome::…`, qualified paths, and intra-doc links.

- **CLI (`biscuit-tui/cli/src/`):** `main.rs`, `choice_normalize.rs`,
  `option_sources.rs`, `commands/mod.rs`, `commands/boolean_switch.rs`,
  `commands/choose_one.rs`, `commands/choose_many.rs`,
  `commands/common_choose.rs`, `commands/text_input.rs`,
  `commands/text_area_input.rs`, `commands/input_table/mod.rs`,
  `commands/input_table/columns.rs`, `commands/input_table/tests.rs`.
- **CLI tests (`biscuit-tui/cli/tests/`):** `choose_cli.rs`,
  `common/pty.rs`, `completions_shell.rs` (doc-comment
  `cargo test -p tui-chrome-cli` → `-p biscuit-tui-cli`; the `question`
  binary references stay).
- **Library doctests/module docs (`biscuit-tui/lib/src/`):** the `///`
  and `//!` examples in `prelude.rs`, `components/*.rs`,
  `components/input_table/table.rs`, `core/*.rs`,
  `helpers/choice_builders.rs` (these are compiled doctests — must
  update or they fail to resolve the crate).
- **Library API test (`biscuit-tui/lib/tests/public_api_names.rs`):**
  `use tui_chrome::{…}`.

The file lists above describe the known current surface but are not an
allowlist. The implementation must use a repository search to catch every
live Rust reference, including test modules and doctests added after this spec.

### 1.3 `biscuit-tui/justfile`

- `LIBRARY := "tui-chrome"` → `"biscuit-tui"`
- `CLI := "tui-chrome-cli"` → `"biscuit-tui-cli"`
- Echo/label strings: `"tui-chrome Library & CLI…"`, `"tui-chrome Library"`,
  `"tui-chrome CLI"`, install label `"tui-chrome"`, docs echo. The
  `install` recipe's binary arg `"question"` stays.

### 1.4 Area docs

- `biscuit-tui/README.md`, `biscuit-tui/lib/README.md`,
  `biscuit-tui/cli/README.md`
- `biscuit-tui/docs/cli-reference.md`
- `biscuit-tui/docs/components/*.md` (index, choose_one, choose_many,
  frame_chrome, input_table, text_input, text_area_input)
- `biscuit-tui/docs/theming.md` — includes `https://docs.rs/tui-chrome/latest/tui_chrome/…`
  URLs → `https://docs.rs/biscuit-tui/latest/biscuit_tui/…`
- `biscuit-tui/docs/dependencies.md` (if it names the crate)

---

## Part 2 — External callers

**Current dependent set (metadata-based reverse-dependency scan).** A
`cargo metadata --no-deps` reverse-dependency scan reports three packages that
depend on the `biscuit-tui` library: `biscuit-tui-cli` (in-area CLI),
`claudine-cli`, and `biscuit-icon-cli`. The two **external** workspace callers
are therefore `claudine-cli` and `biscuit-icon-cli`. Re-run the scan during
implementation rather than treating this statement as permanently exhaustive:

```bash
cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.dependencies[]?.name == "biscuit-tui") | .name'
```

### 2.1 `claudine/cli/Cargo.toml`

- `tui-chrome = { path = "../../biscuit-tui/lib" }` →
  `biscuit-tui = { path = "../../biscuit-tui/lib" }`
- *(Unrelated, do not touch as part of this rename:* claudine/cli pins
  `ratatui = "0.30"` while the library uses `0.29` — out of scope.)

### 2.2 `claudine/cli/src/` — import path

- `commands/schema_interactive.rs` — `use tui_chrome::prelude::*;`
- `commands/wrap/selection_ui.rs` — two `use tui_chrome::…` lines, the
  `//!` module-doc mentions of "tui-chrome", and intra-doc links
  `[`tui_chrome::ChooseOne`]`, `[`tui_chrome::InputTable`]`,
  `[`tui_chrome::run_standalone`]`.

### 2.3 `claudine/cli/tests/`

- `level2_schema_prompt_pty.rs:187` — comment mentions "tui-chrome"
  rendering path (prose only).

### 2.4 claudine docs (current, non-historical)

- `claudine/cli/README.md`
- `claudine/docs/pipeline.md`, `claudine/docs/topics/execution-flow.md`,
  `claudine/docs/topics/composition.md` — update any `tui-chrome`/`tui_chrome`
  prose references.
- Active or unscheduled implementation documents that must remain executable:
  - `claudine/features/2026-05-25-prompt-reporting-encapsulation/plan.md`
  - `claudine/features/_unscheduled/2-config-tui-refresh/spec.md`
  - `claudine/features/_unscheduled/4-using-biscuit-tui/integration-ideas.md`

> The CLI binary is still `question`, so no claudine runtime behavior,
> spawned-command names, or scripts change — this is a compile-time
> crate-name + import-path rename only.

### 2.5 `biscuit-icon-cli`

`biscuit-icon/cli` was added as a `biscuit-tui` consumer after the original
spec was written. Its manifest already uses the new crate name:
`biscuit-tui = { path = "../../biscuit-tui/lib" }` in
`biscuit-icon/cli/Cargo.toml`. A stale-identifier search over
`biscuit-icon/cli` for `tui_chrome` / `tui-chrome` returns no matches, so no
source edits are required there; the package is included in the final
validation matrix only to confirm it still builds against the renamed crate.

---

## Part 3 — Repo-wide config & docs

- **`release-plz.toml`** — two `name = "tui-chrome"` / `name = "tui-chrome-cli"`
  entries (these gate publish exclusion) → rename both.
- **`docs/dependencies.md`** (root) — `tui-chrome` / `tui-chrome-cli`
  entries and the `_Interactive prompt CLI…_` description.
- **`docs/topics/ci-cd.md`** — "No release for excluded packages"
  list names `tui-chrome`, `tui-chrome-cli`.
- **`.claude/skills/biscuit-tui/SKILL.md`** — crate table
  (`tui-chrome` / `tui-chrome-cli`), the `description:` frontmatter,
  prose, and all `use tui_chrome::…` examples. This file does not currently
  use a `hash:` frontmatter property, so do not introduce one as part of this
  rename.
- **`features/2026-05-24-testing-best-practices/plan.md`** — update the active
  package inventory entry from `tui-chrome-cli` to `biscuit-tui-cli`.
- **Root `justfile`** — `areas` list already uses `biscuit-tui` (the area
  name); **no change needed**.
- **Root `Cargo.toml`** — members are path-based (`"biscuit-tui/lib"`,
  `"biscuit-tui/cli"`); **no change needed**.
- **Workspace lockfile** — this checkout does not track a root `Cargo.lock`,
  so there is no lockfile edit. If the implementation environment generates
  or tracks one, confirm it contains only the new package names before
  completion.

---

## Out of Scope (do NOT rewrite)

These are point-in-time historical records; per repo convention they are
left as written:

- `biscuit-tui/features/_completed/**`
- `biscuit-tui/reviews/**`
- `claudine/features/_completed/**`
- `biscuit-terminal/features/_completed/**` (mentions
  `cargo test -p tui-chrome-cli` in a completed plan)
- Review and implementation records for the testing-best-practices feature,
  including `features/2026-05-24-testing-best-practices/review-*.md`; these
  record package names and commands that existed when the review ran.
- `.claude/skills/claudine/timeline.md`; its entry describes the historical
  migration away from `inquire`.
- Captured/generated agent output under `claudine/claudine-output/**`.
- This feature's own records (`biscuit-tui/features/2026-06-05-rename/spec.md`,
  `plan.md`, and `review-1.md`), which necessarily name both the old and new
  crates.

Active and unscheduled plans are not historical records and are in scope.

---

## Suggested Execution Order

1. Rename `name =` in both in-area `Cargo.toml`s + the in-area path dep.
2. Update `claudine/cli/Cargo.toml` path dep.
3. Sweep `tui_chrome` → `biscuit_tui` and `tui-chrome` → `biscuit-tui`
   across live `.rs` in `biscuit-tui/` and `claudine/cli/`
   (leave the `question` binary name and `-p` flags pointing at the right
   crate name).
4. `biscuit-tui/justfile`, `release-plz.toml`.
5. Docs and active plans: area docs, root `docs/`, claudine docs and
   forward-looking feature documents, and SKILL.md.
6. Verify: `cargo metadata --no-deps`, `sniff repo packages
   --package-area biscuit-tui`, then `just build|test|doctest|lint` for the
   area, for `claudine`, and for `biscuit-icon`.

## Verification

Run from the repository root:

```bash
cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[].name' \
  | rg '^(biscuit-tui|biscuit-tui-cli)$'
sniff repo packages --package-area biscuit-tui --list

rg -n --hidden 'tui_chrome|tui-chrome' . \
  -g '!target/**' \
  -g '!.git/**' \
  -g '!**/features/_completed/**' \
  -g '!**/reviews/**' \
  -g '!features/2026-05-24-testing-best-practices/review-*.md' \
  -g '!.claude/skills/claudine/timeline.md' \
  -g '!claudine/claudine-output/**' \
  -g '!biscuit-tui/features/2026-06-05-rename/**'
```

The residual-reference search targets only the **stale** identifiers
`tui_chrome` (the old Rust import path) and `tui-chrome` (the old package
literal). It deliberately does **not** search `biscuit-tui`: that is the
intended live package/area name and must remain throughout manifests, docs,
workflows, skills, and source comments.

The metadata command must print exactly the two new package names. The
`sniff` command must report the same two packages for the area. The final
`rg` command must produce no output. If it finds a newly added live file with
a stale `tui_chrome`/`tui-chrome` reference, update that file instead of adding
another exclusion.

Then run these commands from each named package-area directory:

```bash
just build
just test
just doctest
just lint
```

Do not run `cargo fmt` as part of this rename.

## Risk Notes

- **Doctests are compiled** — missing a `tui_chrome` in a `///`/`//!`
  example surfaces as a doctest failure, not a silent skip. Sweep lib
  source thoroughly.
- **Two literal forms** — `tui-chrome` (manifests, justfile, prose, docs.rs
  URLs) and `tui_chrome` (Rust paths). Grep for both.
- **`-p` flags** — any `cargo … -p tui-chrome-cli` in test docs/CI must
  point at `biscuit-tui-cli`; any `-p tui-chrome` → `biscuit-tui`.
- **Cargo target cache** — stale artifacts may still contain the old name, so
  searches must exclude `target/**`; metadata and clean compilation are the
  authority.
- **docs.rs URLs** — update both the package segment (`biscuit-tui`) and the
  normalized crate segment (`biscuit_tui`).

## Open Questions

None. The compatibility, historical-document, active-plan, and verification
boundaries are resolved above.
