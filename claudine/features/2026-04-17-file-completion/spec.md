# File Completions

## Summary

Claudine's `compose`, `inline-compose`, and `sequence` commands each accept exactly one `biscuit-file` `FileReference` positional (the Markdown source) plus zero or more `key=value` shorthand setters, interleaved in any order. This feature adds `FileReference`-aware shell completion for the file-reference position: completion candidates are scoped by the sigil the user has typed, filtered by per-command validity rules, and suppressed entirely when the current partial is a setter token. The goal is to let users discover valid source documents through `<TAB>` instead of typing paths like `@prompts/review.md` from memory.

## Motivation

Today Claudine installs a static completion script via `clap_complete::generate` (see `claudine/cli/src/commands/completions.rs`). That script offers the same generic file completion for every positional: no sigil awareness, no `.md`/`.markdown` filter, no per-command frontmatter validation, and no suppression for `key=value` setter tokens. Users who compose frequently (`claudine compose @prompts/…`) must type the entire reference from memory, and users on `inline-compose` / `sequence` routinely attempt files that have no valid `prompt:` / `sequence:` frontmatter and only discover the problem at execution time.

## Scope

### In scope for v1

- Dynamic completion for the file-reference positional of `compose`, `inline-compose`, and `sequence`.
- Sigil-aware candidate sources: bare (empty partial), `./` / `../`, `@` magic, `!` package.
- Per-command validity filter (hybrid — extension-only vs frontmatter-aware; see below).
- Strict setter-token suppression via the regex `^[A-Za-z_][A-Za-z0-9_]*=`.
- Walk bounds (depth cap, result cap, skip list) and a file-size cap applied during frontmatter parsing.

### Out of scope for v1 (tracked follow-ups)

- `vault:` completion — the prefix exists in `FileReference` but no candidate source is wired.
- Absolute-path completion (prefix `/`) — users type the full path literally.
- Setter-name completion (parsing the resolved file's frontmatter to suggest valid setter keys such as `topic=`).
- Setter-value completion.
- Position-aware setter suppression (suppressing file candidates *after* the positional file reference has already been matched earlier on the command line).
- HOME walk beyond the two hardcoded paths `~/.claudine/prompts` and `~/.claudine/sequences`.

## Commands affected

| Command | Positional shape | Validity contract for candidates |
|---|---|---|
| `claudine compose` | one `FileReference` + zero-or-more `key=value` setters | Extension-only: path ends in `.md` or `.markdown`. No frontmatter parsing. Matches the gate in `claudine/cli/src/commands/wrap/composition.rs::resolve_composition_source`. |
| `claudine inline-compose` | same | Frontmatter-aware: file parses as YAML frontmatter and contains a non-empty string `prompt:` field. Mirrors the validation surface of `CompositionError::InlineCompositionMissingPrompt` / `NonStringPrompt` in `claudine/lib/src/composition/resolve.rs`. |
| `claudine sequence` | same | Frontmatter-aware: file has a `sequence:` frontmatter field (inline scalar list, inline object list, or string reference to an external YAML file). Mirrors `resolve_sequence_plan` in `claudine/lib/src/composition/sequence.rs`. |

Behavior for files that fail a validity check: **hide, do not annotate.** Non-candidate files are omitted from the list entirely so users never see noisy strike-through or warning-decorated entries.

Frontmatter parsing is **only** performed after the user's prefix has narrowed the candidate pool, so the parse cost is bounded by the number of matches, not by the total file count in scope.

## Completion strategy

Completion is delivered via `clap_complete`'s dynamic completion engine.

- Enable the `unstable-dynamic` feature on `clap_complete` in `claudine/cli/Cargo.toml`.
- Wire `clap_complete::CompleteEnv` into `claudine/cli/src/main.rs` so that on `<TAB>` the shell re-enters the `claudine` binary to request candidates. The binary becomes the authoritative completion source; completion logic lives next to the CLI definition rather than in a generated script.
- Attach a `CompletionCandidates` implementation to the positional arg of `compose`, `inline-compose`, and `sequence`. This is the extension point that receives the current partial and returns the filtered candidate list.

### Installation impact

The `claudine completions <shell>` command changes meaning. Today it emits a self-contained static script. Under dynamic completion it emits a one-time bootstrap snippet whose job is to instruct the shell to delegate completion back to the `claudine` binary (the canonical form is `COMPLETE=<shell> claudine`). The rewritten output must be documented in `claudine/docs/` so users understand that:

1. They still run `claudine completions <shell> > <destination>` (or equivalent) once.
2. After that, updates to completion behavior ship with the `claudine` binary itself; no re-generation step is required.

### Fallback for shells without dynamic support

The static `clap_complete::generate` path may be retained as a fallback for shells or install flows that cannot host the dynamic engine. If retained, document clearly which shells are served by which path and note that the static path will not get the per-command validity filter or sigil-aware scopes. If removed, the spec should explicitly call out the drop and cite the minimum `clap_complete` feature matrix required.

## Candidate sources by prefix

The completion scope depends on the sigil in the user's current partial:

| Prefix | Scope |
|---|---|
| *(empty — bare `<TAB>`)* | Depth-1 listing of: repo root; cwd; `<repo>/prompts/` (if present); `<repo>/sequences/` (if present); `<package-area>/prompts/` (if present) for every package area; `<package-area>/sequences/` (if present) for every package area. |
| `./` or `../` | cwd-relative walk, standard filesystem path completion semantics. |
| `@` (magic) | Recursive walk of: repo root (all subdirectories); `~/.claudine/prompts`; `~/.claudine/sequences`. Subject to the skip list and walk bounds below. |
| `!` (package) | Package-area walk only, corresponding to `FileReference::Package` resolution. |
| `vault:` | **Out of scope for v1.** No candidates produced; the user types the remainder literally. |
| Absolute path (`/…`) | **Out of scope for v1.** No candidates produced. |

### Package-area semantics

"Package area" means a top-level directory in the workspace that hosts a Cargo package (e.g. `claudine/`, `darkmatter/`, `biscuit-file/`). The authoritative enumeration is given in the `Monorepo Structure` section of `CLAUDE.md`, but the completion code **must not** hard-code the list. Package areas should be discovered at runtime — for example via `cargo metadata --no-deps --format-version 1`, or by scanning top-level directories for a `Cargo.toml`. The discovery mechanism is an implementation choice but must remain resilient to new areas being added without a `claudine` rebuild.

### Walk bounds

Applies to every recursive walk (most notably the `@` sigil). The three bounds below are **recommended starting values** and should be exposed as named constants so they can be tuned without reshaping the implementation.

| Bound | Recommended starting value | Rationale |
|---|---|---|
| Max recursion depth | `4` | Balances repo exploration against completion latency. |
| Max candidate count | `500` | Cap on total results returned to the shell. |
| Max file size for frontmatter parsing | `1 MiB` | Any file larger than this is treated as "not a candidate" without reading the body. |

### Skip list

Recursive walks must not descend into any directory whose name matches the curated deny list. Treat this list as extensible; it is not a security boundary, just a relevance and performance safeguard.

Initial deny list: `.git`, `target`, `node_modules`, `.claudine/.shadow`, `dist`, `build`, `.next`, `.venv`, `venv`, `__pycache__`.

## Setter-token handling

When the current partial matches the regex `^[A-Za-z_][A-Za-z0-9_]*=`, completion returns an empty candidate list for that position. This is the "strict prefix" rule: once the user has typed an identifier followed by `=`, the token is a setter and no `.md` candidates should be offered.

This rule is intentionally partial-string-local. It does not inspect earlier tokens on the command line, so a user who has already supplied the file reference as an earlier positional and is now typing a second setter will still see no file candidates for the setter partial — but a user who has not yet supplied a file reference and types `foo` (no `=`) will still see file candidates. The position-aware version (suppressing file candidates *after* the file reference is matched) is tracked as a follow-up.

## Performance and safety constraints

- Frontmatter parsing is gated behind the prefix filter: only files that survive the path-prefix narrowing are parsed.
- I/O or parse failures during candidate evaluation are swallowed and treated as "not a candidate". Completion must never surface an error in the shell.
- Files larger than the size cap are skipped without reading.
- Recursive walks honor the depth cap, result cap, and skip list.
- Symlink cycles must not cause the walker to hang. The implementation should either disable symlink following or track visited canonical paths and break cycles.
- Missing directories (e.g. a package area without a `prompts/` subdirectory) are silently skipped.

A concrete end-to-end latency budget (e.g. "p95 completion response under X ms on a cold cache") is **not** fixed in this spec; it needs empirical measurement on a representative repo before we commit to a number. Flag this as an open measurement task during implementation.

## Failure modes

| Condition | Behavior |
|---|---|
| Malformed YAML frontmatter | File omitted from candidate list. No error surfaced. |
| Unreadable file (permissions, vanished between listing and parsing) | File omitted. No error surfaced. |
| Oversized file (> size cap) | File omitted without opening. |
| Symlink cycle | Walker terminates the cycle; walk continues elsewhere. |
| Missing `prompts/` or `sequences/` directory | Silently skipped. |
| Non-existent package area (discovery returns stale entry) | Silently skipped. |
| Non-UTF-8 path | Omitted from candidate list. |

## Acceptance criteria

- `claudine compose @pro<TAB>` lists `.md` and `.markdown` files reachable via the `@` walk whose path segments match `pro` (case sensitivity follows the shell convention).
- `claudine compose ./<TAB>` lists cwd-relative `.md` / `.markdown` files using standard filesystem path completion.
- `claudine compose !<TAB>` lists `.md` / `.markdown` files within the resolved package area only.
- `claudine inline-compose @<TAB>` does **not** list any `.md` file that lacks a non-empty string `prompt:` frontmatter field.
- `claudine sequence @<TAB>` does **not** list any `.md` file that lacks a `sequence:` frontmatter field (inline or external).
- `claudine compose topic=<TAB>` returns zero candidates.
- `claudine compose _internal=<TAB>` returns zero candidates (leading underscore is valid in the setter regex).
- `claudine compose foo.bar=<TAB>` does **not** trigger suppression (dot-paths are not setters).
- Recursive walks never descend into `.git`, `target`, `node_modules`, `.claudine/.shadow`, or other directories in the skip list.
- A file larger than the size cap is never parsed and never appears for `inline-compose` / `sequence` completion.
- Completion returns cleanly (no error, possibly zero candidates) for `vault:<TAB>` and `/abs<TAB>` in v1.
- Completion returns without panicking when walking a directory tree that contains a symlink cycle.
- End-to-end completion latency on a cold cache is measured and documented during implementation. [Budget TBD — see performance note.]

## Open questions and tracked follow-ups

- **Vault completion.** What is the candidate source for `vault:`? Likely the resolved vault root(s) from Claudine config, but vault resolution is not wired into completion today.
- **Absolute-path completion.** Should we offer `.md`/`.markdown` suggestions under `/`, or defer to the shell's built-in filesystem completion? Deferring to the shell is the path-of-least-resistance default, but it means `inline-compose` / `sequence` validity filtering does not apply to absolute paths.
- **Setter-name completion.** Parsing the referenced file's declared frontmatter keys to offer valid setter names (e.g. `topic=`, `depth=`) after the file reference is known. Requires deciding whether completion should re-parse the referenced file on each `<TAB>`, cache results, or surface only a static catalog.
- **Setter-value completion.** Not pursued in v1.
- **Position-aware suppression.** Suppressing file candidates after the file positional has already been matched. Requires parsing prior tokens on the command line.
- **HOME walk expansion.** Today only `~/.claudine/prompts` and `~/.claudine/sequences` are in scope for `@`. Should other well-known user-scope directories participate?
- **Latency budget.** Pick a concrete p95 ceiling after measuring.
- **Static-fallback retention.** Decide whether to keep `clap_complete::generate` as a secondary install path.
- **Discovery mechanism for package areas.** `cargo metadata` vs. directory scan vs. a baked-in allowlist refreshed at build time — pick one and document the tradeoff.

## Dependencies and risks

- **`clap_complete` `unstable-dynamic`.** The dynamic completion engine is behind an explicitly unstable feature flag. Upstream API changes may force a rework of the `CompletionCandidates` wiring. Pin the `clap_complete` version and track its release notes.
- **Bootstrap install flow is new.** Users upgrading from the static generator will need to re-install their completion snippet. Release notes must call this out prominently.
- **Frontmatter parsing at completion time.** `inline-compose` and `sequence` completion introduce disk I/O and YAML parsing into the `<TAB>` path. The prefix filter, size cap, and result cap together bound the cost, but the introduction itself is a departure from pure path completion. Measurement before rollout is essential.
- **Package-area discovery.** Runtime discovery via `cargo metadata` adds a subprocess invocation per completion session unless cached. A directory-scan fallback avoids the subprocess but risks drifting from the true workspace membership. Choose deliberately.
