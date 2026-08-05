---
status: draft — three open decisions, listed below
created: 2026-08-03
area: tree-hugger
packages:
  - tree-hugger
  - tree-hugger-cli
  - biscuit-test-harness
---

# A path leaving the process should not carry its operating system with it

## Summary

On 2026-08-03 the `hug` CLI was fixed so that the paths it *prints* are
slash-separated on every platform. Six integration tests had been failing on
Windows for the same reason: the CLI printed `tree-hugger\cli\src\main.rs` where
every test, and every other platform, expected `tree-hugger/cli/src/main.rs`.

That fix covered the human-readable surface. **Two boundaries were left
native**, and both are places where a path stops being a path and becomes data
somebody else has to interpret:

1. The `--json` payload, which still emits `"C:\\Users\\ken\\rusty-biscuit"`.
2. The command line a Level-2 test types into its terminal pane, where a native
   Windows path is silently destroyed by the shell reading it.

Neither is cosmetic. The first makes a `hug --json` run non-comparable across
machines and puts the CLI at odds with its own library. The second is a live
test failure that eighteen files across six package areas are one Windows CI
run away from hitting.

This document proposes closing both.

## Background, for a reader new to this area

One idea is needed. Everything else follows from it.

A Rust `Path` is an opaque, OS-native thing. It has no spelling until something
turns it into text — and at that moment a decision gets made, usually by
accident. `Path::display()` and `serde`'s `PathBuf` impl both pick the *native*
spelling: `/` on Unix, `\` on Windows. That is the right default for handing a
path back to the same operating system. It is the wrong default the instant the
text crosses a machine boundary: into a JSON file someone diffs against a
Linux run, or into a shell that treats `\` as an escape character.

The monorepo already has a considered answer to this, in
[`biscuit-file/lib/src/path_text.rs`](../../../biscuit-file/lib/src/path_text.rs):

- `to_portable_string(path)` renders slash-separated text, reducing a Windows
  verbatim-disk prefix through `dunce` first.
- It deliberately **declines** to rewrite UNC, device-namespace, and
  unreducible verbatim paths, returning their native spelling instead — because
  `//?/C:/CON` is neither a path nor a URL, and silently renaming someone's file
  is worse than an ugly string.

That decline is the subtle part, and it matters later in this document.

## Defect one — the JSON payload is native-spelled

### What it emits today

Run on Windows, at this commit:

```console
$ hug symbols tree-hugger/lib/tests/fixtures/sample.rs --json
{
  "schema_version": { "major": 2, "minor": 0 },
  "root_dir": "C:\\Users\\ken\\rusty-biscuit",
  "language": "Rust",
  "files": [
    {
      "file": "C:\\Users\\ken\\rusty-biscuit\\tree-hugger\\lib\\tests\\fixtures\\sample.rs",
```

```console
$ hug god-files --json tree-hugger/cli
[ { "relative_path": "src\\main.rs", ... } ]
```

The same commands on macOS or Linux produce `/`-separated values throughout.

### Why this is a defect and not a platform difference

**The library already decided this question, and the CLI's payload disagrees
with it.** Two places in `tree-hugger/lib` normalize path text to forward
slashes before using it:

| Location | What it normalizes | Why |
|---|---|---|
| [`shared/schema_v2/mod.rs:96`](../../lib/src/shared/schema_v2/mod.rs) `stable_symbol_key` | the path component of a symbol's stable key | so a `SymbolId` is the same value on every platform |
| [`cache/mod.rs:612`](../../lib/src/cache/mod.rs) `normalize_path` | the path component of `FileCacheKey::stable_key` | so a cache entry is addressable across platforms |

Both do `path.to_string_lossy().replace('\\', "/")`.

So within one emitted record, the `id` is derived from
`tree-hugger/lib/tests/fixtures/sample.rs` while the `file` field sitting beside
it reads `tree-hugger\lib\tests\fixtures\sample.rs`. **The identity surface is
portable and the payload describing it is not.** That is not a considered
difference; it is the identity code having been fixed and the serialization
never having been looked at.

### What it costs a consumer

- A `hug --json` capture cannot be diffed between a Windows run and a Linux run
  of the same commit — every path-bearing line differs.
- Any `jq` filter, glob, or string match against `.files[].file` needs an
  OS conditional. The CLI's own README shows exactly this pattern
  (`hug symbols "lib/**/*.rs" --json > symbols.json`).
- `god-files --json` emits `relative_path` — a value whose entire purpose is to
  be a stable, machine-independent label — as `src\main.rs`.

### Deserialization: what actually needs to change, and what does not

This is the part worth being precise about, because it is where the change
looks risky and mostly is not.

**`Deserialize` on these types is load-bearing.** It is not a decorative derive.
[`PersistentCache`](../../lib/src/cache/mod.rs) round-trips `SymbolSnapshot`
through `serde_json` to disk between invocations, and `SymbolSnapshot` carries a
`FileCacheKey { file_path: PathBuf, .. }`. `FileSymbolIndex.file`,
`FileSummary.file`, `PackageSummary.root_dir`, `SymbolInfo.file`, and
`GodAnalysis.relative_path` are all `PathBuf` fields that serde handles
directly.

**But the read side already accepts portable text, on every platform.**
`PathBuf` deserializes from whatever string it is given, and Windows path APIs
accept `/` as a separator throughout. A `"tree-hugger/cli/src/main.rs"` read on
Windows opens the same file as `"tree-hugger\\cli\\src\\main.rs"`. So writing
portable and reading verbatim round-trips correctly on both platforms, and no
deserialization code has to change.

**And a stale on-disk entry degrades to a recompute, not a failure.** The cache
read is:

```rust
serde_json::from_slice(&bytes).ok()
```

A parse failure yields `None`, which means "recompute". Even in the worst case
this change costs one cold cache on one machine.

The one thing that genuinely does not work — and did not work before either — is
carrying an *absolute* path between machines. `C:/Users/ken/...` is meaningless
on Linux whichever way it is spelled. That is a separate question from the
separator, and it is open decision #1 below.

## Defect two — a native path typed into a POSIX shell is destroyed

### The mechanism

Every terminal-harness backend drives its pane through a POSIX shell.
`biscuit_test_harness::detect_shell` resolves `bash` → `sh` → `$SHELL` → `"sh"`,
on all platforms including Windows, and `send_command_with_env` *types the
command line as text* into that shell.

A Windows path in that text is not a path — it is a string full of escape
sequences. bash consumes each `\` and the character after it:

```text
$ W:/rusty-biscuit-target\debug\hug.exe god-files C:\Users\...\tmp1a2b
bash: W:/rusty-biscuit-targetdebughug.exe: No such file or directory
```

This is the failure that `level2_god_files_pretty_report_in_wezterm` was hitting
before 2026-08-03. It predates the path-rendering work and is unrelated to it.

### The blast radius is larger than one test

Eighteen test files across six package areas build a command string containing a
`CARGO_BIN_EXE_*` path and send it through this shell:

```text
biscuit-terminal/lib/tests/level2_terminal_osc_wezterm.rs
biscuit-tui/cli/tests/level3_chord_select.rs
claudine/cli/tests/  (12 files, level2_* and level3_*)
darkmatter/cli/tests/common/level2.rs
darkmatter/cli/tests/level2_schema_about.rs
sniff/cli/tests/level2_cicd_styling.rs
tree-hugger/cli/tests/level2_god_files.rs
```

Only the last one has been fixed, and it was fixed with a private helper local to
that file.

**Seventeen of these are latent rather than proven-broken**, and the reason is
worth recording: on Windows the tmux and kitty backends skip clean (no tooling),
so only the WezTerm backend actually executes. A file whose WezTerm test happens
not to interpolate a path, or whose backend list omits WezTerm, passes today and
fails the moment either changes.

### Why a private helper is the wrong resting place

The helper currently living in `tree-hugger/cli/tests/level2_god_files.rs` is:

```rust
fn shell_word(path: &Path) -> String {
    let portable = biscuit_file::to_portable_string(path);
    format!("'{}'", portable.replace('\'', "'\\''"))
}
```

Two concerns, both belonging to the harness rather than to any one test:
rendering a path for the shell the harness chose, and quoting it so a temp
directory with a space in it stays one word. Every one of the eighteen files
needs both. Left where it is, seventeen of them will either rediscover it or
ship the bug.

## What this proposes

### 1. Every path leaving `hug` as JSON is portable text

The fields, and where they live:

| Field | Type | Crate |
|---|---|---|
| `root_dir` | `PackageSummary`, CLI `JsonOutput` | lib, cli |
| `file` | `FileSummary`, `FileSymbolIndex`, `SymbolInfo`, `ImportSymbol`, `ReferencedSymbol` | lib |
| `relative_path` | `GodAnalysis` | lib |
| `tool_path`, `config_path`, `config_files`, `working_directory` | `AdapterConfig`, `AdapterMetadata` | lib |

The mechanism is an open decision (#3). The observable contract is not: a
`hug --json` run on Windows and the same run on Linux differ only in the values
that are genuinely machine-specific, never in separator spelling.

### 2. Deserialization is pinned by test, not changed by code

No read-side change is required (see above), but the round-trip is currently
unasserted. Add coverage that a payload written on one platform's spelling
deserializes and resolves on the other — this is what makes the "no change
needed" claim durable rather than incidental.

### 3. `shell_word` moves into `biscuit-test-harness`

Exposed from `biscuit_test_harness` alongside the existing `detect_shell` and
`cargo_bin_dir` helpers, documented with the bash-eats-backslashes rationale so
the next reader does not have to rediscover it from a failing capture. The
tree-hugger copy is deleted in the same change, and the remaining seventeen call
sites are migrated.

## The open decisions

### 1. Absolute or relative paths in the JSON payload?

Today `--json` emits absolute paths for `file` and `root_dir`, and a relative
path for god-files' `relative_path`. Normalizing the separator does not make an
absolute path portable between machines; only relativizing does.

**For relative:** it is what the pretty output already shows, it makes two runs
of the same commit on different machines byte-comparable, and `root_dir` is
right there to rebase against.

**Against:** it is a louder breaking change than the separator fix, and a
consumer that today feeds `.files[].file` straight to `open()` would break
where the separator change alone would not have touched it.

These can be decided independently, and the separator change should not be held
hostage to the harder question. But deciding them in the same pass avoids
breaking the same consumers twice.

### 2. Does this bump `schema_version`?

`JsonOutput` and `FileSymbolIndex` both carry `SchemaVersion { major: 2, minor: 0 }`.
The separator change is invisible to any Unix consumer and, on Windows, changes
values without changing shape. A minor bump to `2.1` signals it honestly; not
bumping keeps a version number that has so far tracked *structure* from tracking
value formatting too. Resolve alongside #1, since relativizing would clearly
warrant a bump.

### 3. What does the normalization, and do the three existing normalizers merge?

There are now three ways this monorepo turns a path into portable text:
`to_portable_string`, `stable_symbol_key`'s inline `replace`, and
`cache::normalize_path`'s inline `replace`. Consolidating is attractive and
**is not a mechanical swap**:

`to_portable_string` declines to rewrite UNC and unreducible verbatim paths,
returning the native spelling. That is correct for display — but dropping it
into `stable_symbol_key` would mean a symbol on a network share silently gets a
backslash-bearing ID, changing every `SymbolId` for files on UNC paths. A key
generator wants unconditional determinism; a renderer wants faithfulness. They
may genuinely be two functions.

Sub-question for the harness: `biscuit-test-harness` has three small
dependencies today and none of them is `biscuit-file`. Taking the dependency
buys the UNC handling and the shared boundary; inlining three lines keeps a
test-infrastructure crate lean. The UNC case is not hypothetical for a harness
that may run against a network-mounted checkout.

## Alternatives considered

**Leave the JSON native and document it.** Tell consumers to normalize. Rejected:
it pushes an OS conditional into every consumer of a machine-readable format,
and it leaves the payload contradicting the `SymbolId` sitting next to it — a
contradiction no consumer can be expected to anticipate.

**Serialize paths as `file://` URLs.** Unambiguous and already used for the OSC8
links. Rejected: it is heavier for the common case, it cannot express a relative
path at all, and it would force every consumer through a URL decoder to recover
something they only ever wanted as a string.

**A `PortablePath` newtype with a custom `Serialize`.** The most robust option —
it makes the portable spelling unforgeable rather than remembered, and a new
path-bearing field cannot regress. The cost is a type change rippling through
`tree-hugger` lib's public API and every construction site, which is larger than
the defect. Worth weighing under open decision #3; noted here so the cheap
option is chosen deliberately rather than by default.

**For the harness: prepend the binary's directory to `PATH`.** `cargo_bin_dir`
already exists for exactly this, and it removes the absolute binary path from
the command line. Rejected as a complete answer: it addresses the binary and not
the *arguments*, and the arguments are temp-directory paths — the half that also
carries spaces, and so needs the quoting half of `shell_word` regardless.

## Current state

Landed 2026-08-03 on `fix/windows-hugging`, closing the printed-output half:

- `display_path`, the lint diagnostic location line, and the god-files label
  render through `biscuit_file::to_portable_string`.
- `file_url` takes a `&Path` and uses `url::Url::from_file_path`, so a Windows
  OSC8 target is `file:///C:/repo/…` rather than the unresolvable
  `file://C:%5Crepo%5C…` a percent-encoder produced from native separators.
- `completion_candidate` replaces `Path::join` when building shell completion
  suggestions, which previously returned mixed spellings like `tree-hugger\cli/`.
- `tree-hugger/cli/tests/level2_god_files.rs` carries the private `shell_word`
  this document proposes promoting.
- Unit tests in `tree-hugger/cli/src/main.rs` pin all four, with
  `#[cfg(windows)]` cases for the drive-letter URI and separator rendering.

The `--json` payload and the other seventeen harness call sites are untouched.

## Verification

Complete when all of the following hold:

1. `hug symbols … --json` and `hug god-files --json`, run at the same commit on
   Windows and on Linux, differ only in machine-specific values — never in
   separator spelling. Asserted by a Windows-gated test on the emitted string,
   not by inspection.
2. Every field in the table under *What this proposes* is covered, including the
   adapter metadata paths, which no current test exercises.
3. A JSON payload written with portable separators deserializes on Windows and
   resolves to the same file as the native spelling.
4. A `PersistentCache` entry written before the change is either read correctly
   or recomputed — never surfaced as an error to the user.
5. `shell_word` is public API on `biscuit-test-harness`, the tree-hugger copy is
   gone, and all seventeen remaining call sites use it.
6. `level2_god_files_pretty_report_in_wezterm` still passes on Windows after the
   local helper is deleted — proving the promoted one is equivalent.
7. `just test` and `just test-l2` pass for every affected package area:
   tree-hugger, biscuit-terminal, biscuit-tui, claudine, darkmatter, sniff.

## Out of scope

- Making absolute paths portable *between machines*. Not achievable by spelling;
  see open decision #1 for the part that is.
- The pre-existing `cargo fmt --check` diffs in `tree-hugger/cli` (ten, all
  predating this work). They want their own formatting-only commit.
- Relativizing the lint diagnostic location line, which prints an absolute path
  while the file header above it prints a relative one. Real, cosmetic, and
  independent of the separator question.
- Reducing the number of terminal backends that skip clean on Windows. It is why
  seventeen call sites are latent rather than red, but it is a harness-coverage
  question, not a path-rendering one.
