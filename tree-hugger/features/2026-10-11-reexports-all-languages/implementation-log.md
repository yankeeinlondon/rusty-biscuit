---
spec: /Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
plan: tree-hugger/features/2026-10-11-reexports-all-languages/plan.md
implemented_by: claude/sonnet
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - tree-hugger/features/2026-10-11-reexports-all-languages/spec.md
docs_created_during_phase_1:
  - tree-hugger/features/2026-10-11-reexports-all-languages/audit/notes-module-syntax.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/audit/notes-static-conventions.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/audit/notes-interface-boundaries.md
  - tree-hugger/features/2026-10-11-reexports-all-languages/audit/samples/ (279 sample files across 16 languages)
skills_files_updated_during_phase_1: []
packages:
  - tree-hugger
---

# Implementation Log for 2026-10-11-reexports-all-languages (6 phases)

## Phase 1

Phase 1 is an evidence and design phase. No Rust source changed, so there are
no behavior changes to map to tests; the requirement-to-test mapping is
"none, audit only". The 279 samples are intended as Phase 3 regression fixtures.

### What was done

- Built a disposable AST probe outside the repo (`/tmp/th-probe`, lockfile-pinned
  grammars, tree-sitter 0.26.9) that prints named nodes, field names and
  ERROR/MISSING flags. It hides anonymous tokens; one worker made a private
  variant to read them.
- Wave 1: three foreground subagents audited, in parallel, module syntax (Rust,
  JS, TS, Swift, Scala), static conventions (Python, Go, Perl, Lua) and
  interface boundaries (Java, PHP, Bash, Zsh, C, C++, C#). Each delivered a notes
  file under `audit/` and positive/negative samples under `audit/samples/<lang>/`.
- Wave 2 (consolidate): added an "Audit results" section and an "Author decision
  package" to `spec.md`. Checked consumers: the placeholder
  `reexported_symbols()` is already removed (only the release note remains);
  `cli/src/main.rs` still builds `ImportSymbol` with `is_reexport: false`;
  `EXTRACTOR_REVISION` is "51"; `SchemaVersion::CURRENT` is V5_1.

### Findings that matter

- C++ blocker: tree-sitter-cpp 0.23.4 (newest published) cannot parse C++20
  modules; `import x;`/`module m;` parse as ordinary declarations with no error.
- Silent misparse in Perl (`@Pkg::EXPORT_OK`) with no error flag, so coverage
  cannot rest on `has_error`.
- Scala restricted/annotated `export` yields ERROR but the export is recovered,
  so a restricted export would read as public unless limited.
- TypeScript `export type *` yields ERROR; JS import attributes yield ERROR.
- Java, PHP, Bash, Zsh, C and C# `using`/`import` forms: no applicable
  construct (borderline cases listed for the author).
- Several semantic claims are marked "source not verified" in the notes (e.g.
  `pub extern crate`, Scala 3 release, `@_exported` on selective imports).

### Not done (blocked on the author)

- Task "Freeze support contract" remains unchecked: it requires the author's
  rulings, and this run is non-interactive. The decision package is in the spec;
  `human_review: true` is set on the spec. The spec status stays `draft-spec`.

### Gates

- Scope command (`affected_scope.py` over the working tree) returned
  `"packages": ["tree-hugger","tree-hugger-cli","test-toolkit"]`,
  `change_class: "package"` (the tree also holds earlier uncommitted work), so
  the gates were run.
- `just test` in `tree-hugger/`: 2089 passed, 0 failed, 26 slow, 0 skipped.
- `just lint`: clean for tree-hugger and tree-hugger-cli.
- No OS-specific code changed; no cross-check run was needed.
