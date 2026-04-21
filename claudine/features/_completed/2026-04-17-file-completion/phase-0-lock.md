---
phase: 0
status: locked
created: 2026-04-17
---

# Phase 0 — Seam Lock

Pre-implementation lock for the file-completion feature. Captures the decisions
that Phases 1–5 must honor: write scope, runtime boundaries (with current line
numbers), test-home assignments per acceptance criterion, and the emitted-value
contract for the empty-partial landing menu and sigil-scoped candidates.

Both validation checkpoints pass on the baseline:

- `cargo test -p claudine-cli --test command_routing` — 8 passed
- `cargo test -p claudine-cli --test argv_normalization` — 8 passed

## 0.1 — File-Set Scope

The feature lives entirely inside the CLI crate (`claudine/cli/`) and the
feature-level docs. No edits land in `claudine/lib`, `sniff`, `darkmatter`, or
any other workspace member.

Files to be created:

- `claudine/cli/src/completion/mod.rs`
- `claudine/cli/src/completion/command_factory.rs`
- `claudine/cli/src/completion/file_reference.rs`
- `claudine/cli/src/completion/validate.rs`
- `claudine/cli/src/completion/bootstrap.rs`
- `claudine/cli/tests/completion_cli.rs`
- `claudine/docs/shell-completions.md` (new; referenced from plan.md step 4.2)

Files to be modified:

- `claudine/cli/Cargo.toml` — add `clap_complete` `unstable-dynamic` feature
- `claudine/cli/src/main.rs` — `mod completion;` + early `maybe_complete()` hook
- `claudine/cli/src/commands/mod.rs` — (no new module; `completion` is a sibling
  of `commands`, not a command)
- `claudine/cli/src/commands/completions.rs` — rewrite as bootstrap emitter
- `claudine/cli/tests/command_routing.rs` — update `completions_write_to_stdout_for_supported_shells`
- `claudine/docs/topics/composition.md` — document the new completion behavior

Out of scope in every phase: `claudine/lib/src/composition/*`,
`sniff/lib/src/filesystem/repo/*`. Completion calls into them read-only via
existing public API.

## 0.2 — Runtime Boundaries (Verified Against Current Tree)

Confirmed from the code, not from the design doc:

- **argv normalization is gated on `COMPLETE`** —
  `claudine/cli/src/argv.rs:92-114`. `normalize_inner` returns raw argv
  unchanged when `completion_mode_active()` is true
  (`argv.rs:404`, reading `COMPLETE`). This guarantee is re-asserted in
  Phase 1 step 1.4 and must not regress.
- **`main.rs` reads argv exactly once** — `claudine/cli/src/main.rs:109`.
  `color_eyre::install()` runs first (`main.rs:107`); the completion hook
  will be inserted between those two lines in Phase 1.
- **Three composition commands share one positional** —
  `ComposeArgs.args` (`claudine/cli/src/commands/compose.rs:197`),
  `InlineComposeArgs.args` (`commands/compose.rs:211`),
  `SequenceArgs.args` (`commands/sequence.rs:22`). All three are `Vec<String>`.
  This is the single attachment point for every dynamic completer.
- **Static generator is still the only completion path** —
  `claudine/cli/src/commands/completions.rs:29-47` calls
  `clap_complete::generate(...)`. No `CompleteEnv` wiring exists yet.
- **`sniff` is already a direct dependency** — `claudine/cli/Cargo.toml:17`
  (`sniff = { path = "../../sniff/lib" }`). No new dep graph cost.
- **`sniff` entry points are public** —
  `sniff::filesystem::repo::RepoInfo::package_area_for_dir`
  (`sniff/lib/src/filesystem/repo/types.rs:272`) and
  `sniff::filesystem::repo::detect_repo_structure`
  (`sniff/lib/src/filesystem/repo/types.rs:530`). Usable as-is.

## 0.3 — Test Homes per Acceptance Criterion

Each spec acceptance criterion has a committed destination before any
completer code lands. Unit tests live next to the module under test;
subprocess-level tests live in a dedicated CLI integration file;
`command_routing.rs` remains the retirement target for the old static
completion contract.

| Acceptance criterion (from `spec.md`) | Home | Phase |
|---|---|---|
| `compose @pro<TAB>` lists only matching `.md`/`.markdown` | `claudine/cli/tests/completion_cli.rs` | 4 |
| `compose ./<TAB>` lists cwd-relative markdown | `claudine/cli/tests/completion_cli.rs` | 4 |
| `compose !<TAB>` scoped to package area | `claudine/cli/tests/completion_cli.rs` | 4 |
| `inline-compose @<TAB>` omits files without non-empty `prompt:` | `claudine/cli/tests/completion_cli.rs` | 4 |
| `sequence @<TAB>` omits files without valid `sequence:` | `claudine/cli/tests/completion_cli.rs` | 4 |
| `topic=<TAB>` zero candidates | `claudine/cli/tests/completion_cli.rs` + `file_reference.rs` unit | 2, 4 |
| `_internal=<TAB>` zero candidates (leading `_` is valid) | `completion/file_reference.rs` unit | 2 |
| `foo.bar=<TAB>` does **not** suppress (dot breaks setter shape) | `completion/file_reference.rs` unit | 2 |
| Skip-list enforced (`.git`, `target`, `node_modules`, `.claudine/.shadow`, ...) | `completion/file_reference.rs` unit | 2 |
| Oversized file never parsed, never offered for inline/sequence | `completion/validate.rs` unit | 3 |
| Symlink cycle terminates | `completion/file_reference.rs` unit | 2 |
| `vault:<TAB>` / `/abs<TAB>` return cleanly with zero candidates | `completion/file_reference.rs` unit + `completion_cli.rs` subprocess | 2, 4 |
| Bootstrap snippet emitted per shell | `completion_cli.rs` + `command_routing.rs` replacement | 4 |
| `completions` subcommand retires the old static markers | `claudine/cli/tests/command_routing.rs` (rewrite) | 4 |
| Empty-partial landing menu rendering | `completion/file_reference.rs` unit | 2 |
| `@` / `!` emission format | `completion/file_reference.rs` unit | 2 |
| Depth cap / candidate cap enforcement | `completion/file_reference.rs` unit | 2 |

The `command_routing.rs` test
`completions_write_to_stdout_for_supported_shells`
(`claudine/cli/tests/command_routing.rs:146-179`) is the single existing
regression target for the old static contract; Phase 4 step 4.4 is the only
authorized edit point.

## 0.4 — Frozen Emitted-Value Contract

This is the `<TAB>`-insert contract for v1. Phases 1–5 must not renegotiate
these shapes without editing this document first.

### Sigil-prefixed input (user has already committed to a scope)

| Typed prefix | Walk root | Emitted completion `value` |
|---|---|---|
| `@` | repo root | `@<path-relative-to-repo-root>` |
| `@` | `~/.claudine/prompts/` | `@prompts/<relative>` |
| `@` | `~/.claudine/sequences/` | `@sequences/<relative>` |
| `!` | current package-area root | `!<path-relative-to-area-root>` |
| `./` | cwd | `./<child>` (directories trailing `/`) |
| `../` | `cwd/..` | `../<child>` (directories trailing `/`) |

### Bare partial (empty token — the landing menu)

The bare landing menu is a **union of scopes** whose emitted values preserve
their sigil so the shell can insert a selectable token. Mixed-scope emission
is the whole point of this freeze: without sigil prefixes, repo-scoped
and area-scoped suggestions would not be insertable from an empty partial.

| Source of candidate | Emitted `value` |
|---|---|
| cwd immediate children | `<child>` (bare, directories trailing `/`) |
| repo root immediate children | `@<child>` |
| `<repo>/prompts/` depth-1 children | `@prompts/<child>` |
| `<repo>/sequences/` depth-1 children | `@sequences/<child>` |
| `<area>/prompts/` depth-1 children (current area only) | `!prompts/<child>` |
| `<area>/sequences/` depth-1 children (current area only) | `!sequences/<child>` |

Deterministic source ranking for dedup (lowest rank wins on equal `value`):

1. cwd-local bare suggestions
2. package-area suggestions
3. repo-wide magic suggestions
4. home-scoped magic suggestions

Within a rank, sort lexically by the emitted `value`.

### Explicitly unsupported prefixes (v1)

Zero candidates, no error, no annotation:

- `vault:`
- absolute `/…`
- `%` (recursive)
- `{{…}}` (env interpolation)

### Setter suppression

Strict regex `^[A-Za-z_][A-Za-z0-9_]*=` applied to the current partial only.
Matching partials return zero candidates immediately. This is narrower than
runtime setter parsing (which accepts hyphens); the drift is intentional
for v1 and documented in `tech-design.md` §5.

## Validation

- `cargo test -p claudine-cli --test command_routing` — ok (8/8)
- `cargo test -p claudine-cli --test argv_normalization` — ok (8/8)

No source code was modified in Phase 0. The only artifact is this lock
document; Phase 1 begins once the file-set, boundaries, test homes, and
emitted-value contract are stable.
