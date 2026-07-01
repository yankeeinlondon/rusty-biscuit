---
phases: 5
created: 2026-06-11
start_phase: 5
source_files_during_phase_5: []
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - biscuit-tui/README.md
  - biscuit-tui/lib/README.md
  - biscuit-tui/cli/README.md
  - biscuit-tui/docs/cli-reference.md
  - biscuit-tui/docs/theming.md
  - biscuit-tui/docs/components/index.md
  - biscuit-tui/docs/components/choose_one.md
  - biscuit-tui/docs/components/choose_many.md
  - biscuit-tui/docs/components/frame_chrome.md
  - biscuit-tui/docs/components/input_table.md
  - biscuit-tui/docs/components/text_input.md
  - biscuit-tui/docs/components/text_area_input.md
  - .claude/skills/biscuit-tui/SKILL.md
  - features/2026-05-24-testing-best-practices/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/biscuit-tui/SKILL.md
source_files_during_phase_3:
  - claudine/cli/src/commands/schema_interactive.rs
  - claudine/cli/src/commands/wrap/selection_ui.rs
  - claudine/cli/tests/level2_schema_prompt_pty.rs
docs_updated_during_phase_3:
  - claudine/cli/README.md
  - claudine/docs/pipeline.md
  - claudine/docs/topics/execution-flow.md
  - claudine/docs/topics/composition.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_1:
  - biscuit-tui/lib/Cargo.toml
  - biscuit-tui/cli/Cargo.toml
  - biscuit-tui/justfile
  - claudine/cli/Cargo.toml
  - release-plz.toml
docs_updated_during_phase_1:
  - docs/dependencies.md
  - docs/topics/ci-cd.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - biscuit-tui/lib/src/prelude.rs
  - biscuit-tui/lib/src/components/text_area_input.rs
  - biscuit-tui/lib/src/components/text_input.rs
  - biscuit-tui/lib/src/components/boolean_switch.rs
  - biscuit-tui/lib/src/components/input_table/table.rs
  - biscuit-tui/lib/src/components/choose_one.rs
  - biscuit-tui/lib/src/components/choose.rs
  - biscuit-tui/lib/src/components/choose_many.rs
  - biscuit-tui/lib/src/core/sort.rs
  - biscuit-tui/lib/src/core/frame.rs
  - biscuit-tui/lib/src/core/fuzzy.rs
  - biscuit-tui/lib/src/core/event.rs
  - biscuit-tui/lib/src/core/theme.rs
  - biscuit-tui/lib/src/core/mod.rs
  - biscuit-tui/lib/src/helpers/choice_builders.rs
  - biscuit-tui/lib/tests/public_api_names.rs
  - biscuit-tui/cli/src/main.rs
  - biscuit-tui/cli/src/option_sources.rs
  - biscuit-tui/cli/src/choice_normalize.rs
  - biscuit-tui/cli/src/commands/choose_one.rs
  - biscuit-tui/cli/src/commands/choose_many.rs
  - biscuit-tui/cli/src/commands/common_choose.rs
  - biscuit-tui/cli/src/commands/text_input.rs
  - biscuit-tui/cli/src/commands/text_area_input.rs
  - biscuit-tui/cli/src/commands/boolean_switch.rs
  - biscuit-tui/cli/src/commands/mod.rs
  - biscuit-tui/cli/src/commands/input_table/mod.rs
  - biscuit-tui/cli/src/commands/input_table/columns.rs
  - biscuit-tui/cli/src/commands/input_table/tests.rs
  - biscuit-tui/cli/tests/choose_cli.rs
  - biscuit-tui/cli/tests/common/pty.rs
  - biscuit-tui/cli/tests/completions_shell.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - biscuit-tui/lib
  - biscuit-tui/cli
  - claudine/cli
  - biscuit-icon/cli
---

# Plan: Rename `tui-chrome` to `biscuit-tui`

This plan converts `biscuit-tui/features/2026-06-05-rename/spec.md` into an ordered, checkable execution plan. The work is a compile-time breaking change only: public Rust APIs, CLI flags, the `question` binary name, and end-user behavior stay the same.

## Notes for implementers

- Two literal forms are in scope: `tui-chrome` (manifests, justfile, docs.rs URLs, prose) and `tui_chrome` (Rust import paths and intra-doc links).
- Leave historical records untouched (completed features, reviews, captured agent output, `claudine/.claude/skills/claudine/timeline.md`, and this specification).
- This plan document intentionally names the old crates; exclude it from the residual-reference grep to avoid false positives.

## Phase 1 — Rename crate identities and shared plumbing

*Goal: make Cargo, just, and repo config reflect the new package names before any source imports are updated.*

- [x] Rename `package.name` from `tui-chrome` to `biscuit-tui` in `biscuit-tui/lib/Cargo.toml`.
- [x] Rename `package.name` from `tui-chrome-cli` to `biscuit-tui-cli` in `biscuit-tui/cli/Cargo.toml`.
- [x] Update the in-area dependency key from `tui-chrome = { path = "../lib" }` to `biscuit-tui = { path = "../lib" }` in `biscuit-tui/cli/Cargo.toml`.
- [x] Update `biscuit-tui/justfile`: change `LIBRARY := "tui-chrome"` to `"biscuit-tui"`, `CLI := "tui-chrome-cli"` to `"biscuit-tui-cli"`, and update all echo/label strings that mention the old names (keep the `question` binary arg unchanged).
- [x] Update `release-plz.toml`: rename both `name = "tui-chrome"` and `name = "tui-chrome-cli"` entries.
- [x] Update root `docs/dependencies.md`: rename `tui-chrome` / `tui-chrome-cli` entries and the `_Interactive prompt CLI…_` description.
- [x] Update root `docs/topics/ci-cd.md`: rename `tui-chrome` and `tui-chrome-cli` in the excluded-package list.
- [x] Update `claudine/cli/Cargo.toml`: change dependency key from `tui-chrome = { path = "../../biscuit-tui/lib" }` to `biscuit-tui = { path = "../../biscuit-tui/lib" }`.

**Validation checkpoint:**

```bash
cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[].name' \
  | rg '^(biscuit-tui|biscuit-tui-cli)$'
```

Must print exactly `biscuit-tui` and `biscuit-tui-cli`.

## Phase 2 — Rename in-area Rust source

*Goal: eliminate every `tui_chrome::` reference in the library, CLI, and tests. This phase can be split into library work and CLI work that run in parallel.*

- [x] Update all `use tui_chrome::…`, qualified `tui_chrome::…` paths, and intra-doc links in `biscuit-tui/lib/src/` (including doctests in `prelude.rs`, `components/*.rs`, `components/input_table/table.rs`, `core/*.rs`, and `helpers/choice_builders.rs`).
- [x] Update `use tui_chrome::{…}` in `biscuit-tui/lib/tests/public_api_names.rs`.
- [x] Update all `tui_chrome::` references in `biscuit-tui/cli/src/` (`main.rs`, `option_sources.rs`, `choice_normalize.rs`, and every file under `commands/`).
- [x] Update `biscuit-tui/cli/tests/choose_cli.rs`, `biscuit-tui/cli/tests/common/pty.rs`, and `biscuit-tui/cli/tests/completions_shell.rs`, including any doc-comment `cargo test -p tui-chrome-cli` flags (`-p biscuit-tui-cli`).
- [x] Run a focused search over `biscuit-tui/` (excluding `target/`, completed features, reviews, this spec, and this plan) for `tui_chrome` and `tui-chrome`, and fix any stragglers.

**Validation checkpoint:**

```bash
cargo check --manifest-path biscuit-tui/lib/Cargo.toml
cargo check --manifest-path biscuit-tui/cli/Cargo.toml
```

Both must pass without unresolved-import errors.

## Phase 3 — Update external caller (`claudine`)

*Goal: make the sole external workspace consumer compile against the new crate name. This phase is independent of Phase 2's documentation work but depends on Phase 1's manifest change.*

- [x] Update `use tui_chrome::prelude::*;` in `claudine/cli/src/commands/schema_interactive.rs`.
- [x] Update the two `use tui_chrome::…` lines, module-doc prose mentions of "tui-chrome", and intra-doc links (`[`tui_chrome::ChooseOne`]`, `[`tui_chrome::InputTable`]`, `[`tui_chrome::run_standalone`]`) in `claudine/cli/src/commands/wrap/selection_ui.rs`.
- [x] Update the prose comment mentioning "tui-chrome" at `claudine/cli/tests/level2_schema_prompt_pty.rs:187`.
- [x] Update `claudine/cli/README.md`, `claudine/docs/pipeline.md`, `claudine/docs/topics/execution-flow.md`, and `claudine/docs/topics/composition.md` for any `tui-chrome` / `tui_chrome` references.
- [x] Update active/unscheduled implementation documents:
  - `claudine/features/2026-05-25-prompt-reporting-encapsulation/plan.md`
  - `claudine/features/_unscheduled/2-config-tui-refresh/spec.md`
  - `claudine/features/_unscheduled/4-using-biscuit-tui/integration-ideas.md`

**Validation checkpoint:**

```bash
cargo check --manifest-path claudine/cli/Cargo.toml
```

Must pass.

## Phase 4 — Update area docs, skill, and active plans

*Goal: bring all live documentation and the biscuit-tui skill into alignment. This phase is parallelizable with Phase 2 and Phase 3 once code references are stable.*

- [x] Update `biscuit-tui/README.md`, `biscuit-tui/lib/README.md`, and `biscuit-tui/cli/README.md`.
- [x] Update `biscuit-tui/docs/cli-reference.md`.
- [x] Update `biscuit-tui/docs/components/*.md` (index, choose_one, choose_many, frame_chrome, input_table, text_input, text_area_input).
- [x] Update `biscuit-tui/docs/theming.md`: change `https://docs.rs/tui-chrome/latest/tui_chrome/…` URLs to `https://docs.rs/biscuit-tui/latest/biscuit_tui/…`.
- [x] Update `biscuit-tui/docs/dependencies.md` if it names the old crate.
- [x] Update `.claude/skills/biscuit-tui/SKILL.md`: crate table, `description:` frontmatter, prose, and all `use tui_chrome::…` examples. Do **not** add a `hash:` frontmatter property.
- [x] Update the active package inventory entry from `tui-chrome-cli` to `biscuit-tui-cli` in `features/2026-05-24-testing-best-practices/plan.md`.

**Validation checkpoint:**

```bash
rg -n --hidden 'tui_chrome|tui-chrome' biscuit-tui/ .claude/skills/biscuit-tui/SKILL.md features/2026-05-24-testing-best-practices/plan.md claudine/features/2026-05-25-prompt-reporting-encapsulation/plan.md claudine/features/_unscheduled/2-config-tui-refresh/spec.md claudine/features/_unscheduled/4-using-biscuit-tui/integration-ideas.md \
  -g '!target/**' \
  -g '!**/features/_completed/**' \
  -g '!**/reviews/**' \
  -g '!biscuit-tui/features/2026-06-05-rename/**'
```

This searches only the stale identifiers `tui_chrome` (old import path) and
`tui-chrome` (old package literal); `biscuit-tui` is the intended live name and
is not a residual reference. Must produce no output.

## Phase 5 — Repository-wide verification

*Goal: confirm the rename is complete and all affected code compiles, tests, and lints.*

- [x] Run `cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | rg '^(biscuit-tui|biscuit-tui-cli)$'` and confirm exactly the two new names are printed.
- [x] Run `sniff repo packages --package-area biscuit-tui --list` and confirm it reports `biscuit-tui-cli, biscuit-tui` (no `tui-chrome*` remnants).
- [x] Run the residual-reference search (for the stale identifiers `tui_chrome|tui-chrome`) across the whole repository (excluding `target/`, `.git/`, historical records, and this feature's own records). `biscuit-tui` is the intended live name and is excluded from the search. If it produces output, update the newly matched live file rather than adding another exclusion.
- [x] From `biscuit-tui/`, run `just build`, `just test`, `just doctest`, and `just lint`.
- [x] From `claudine/`, run `just build`, `just test`, `just doctest`, and `just lint`.
- [x] From `biscuit-icon/`, run `just build`, `just test`, `just doctest`, and `just lint` (added as a `biscuit-tui` dependent after the original scan; see Part 2.5 of the spec). Recorded in `validation-1.md`: all four recipes pass.
- [x] If the repository tracks a root `Cargo.lock`, confirm it contains only `biscuit-tui*` package names and no stale `tui-chrome*` entries.

**Validation checkpoint:** All commands above pass and the residual `rg` returns no unexpected matches.
