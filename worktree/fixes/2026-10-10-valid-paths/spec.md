---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: implemented
implemented: true
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-11
review_iterations: 1
completed: true
human_review: false
message_to_agent: |-
  All 5 phases are implemented and verified; the fix is ready for review.
  - Gates: `just test` (1173 passed) and `just lint` are clean from
    worktree/ on macOS. Cross-check passed on native Windows (worktree 558,
    worktree-cli 561 including the incident-replay integration tests), Linux
    (worktree 582), and WSL2 (worktree config tests).
  - Two departures from the spec, both reflected in the docs: a duplicate
    `base_dir` key is rejected (not last-wins), and a
    BaseDirectoryUnreadable error variant carries non-NotFound I/O causes.
  - Follow-up outside this fix: scripts/kache-host.sh and
    docs/initialization.md claim to resolve the worktree base "the way wt
    does" but skip the new validation rules.
  - Do not move this fix to _completed; the author does that after review.
---

# Valid base-directory paths

## Outcome

`wt` only uses a worktree base directory that is an **absolute path on the
current host**. A path written for a different operating system, such as
`b:\wt` on Linux or `/home/me/wt` on Windows, is rejected before anything is
created on disk or saved to `~/.worktree.json`. The error names the problem,
where the value came from (`WT` or `~/.worktree.json`), and how to fix it.

## Incident

On a WSL2 host, the user ran `wt create fix/wsl` with `WT` unset and no
`~/.worktree.json`, so the interactive setup ran. The user thought they were on
native Windows, chose "Other", and entered `b:\wt`.

1. The worktree-cli package’s [setup prompt](../../cli/src/commands/create.rs)
   turned the input into a
   `PathBuf`. On a POSIX host `b:\wt` is a valid but **relative** file name
   with a colon and a backslash in it.
2. `create_dir_all` created `./b:\wt/` relative to the current directory, which
   was the root of the repository.
3. The worktree package’s [configuration writer](../../lib/src/config.rs) saved `"base_dir": "b:\\wt"`.
4. The worktree package’s [creation function](../../lib/src/worktree.rs) added a real worktree at
   `<repo>/b:\wt/rusty-biscuit/fix-wsl` on the new branch `fix/wsl`. It showed
   up as an untracked `b:\wt/` directory in the main checkout.
5. After the directory was removed, every `wt create` failed with
   `base directory 'b:\wt' does not exist`. That message doesn't say the value
   comes from `~/.worktree.json`, so the user couldn't tell what to fix.

A relative base directory is wrong even when it uses the host's own syntax
(for example `wt`, or `~/wt`, which Rust doesn't expand). Where it resolves
depends on the directory `wt` happens to run in, and it usually lands inside
the repository.

## Current behavior

| Source | Checks today | Missing |
| --- | --- | --- |
| `WT` environment variable | exists; is not a repo root; empty value falls through | is absolute; is a directory; source named in errors |
| `~/.worktree.json` | parses; exists; is not a repo root | is absolute; is a directory; "not found" error doesn't name the file |
| Interactive setup | is not a repo root (after `create_dir_all`) | is absolute, checked **before** `create_dir_all` and `save_config` |
| Configuration writer | none; path is converted to display text | is absolute; saved text preserves the path |

## Required behavior

### Shared syntax validation

Add a public validator in the worktree package’s
[configuration module](../../lib/src/config.rs), which currently chooses the
base directory and saves setup choices:

```rust
pub fn validate_base_dir(raw: &str, origin: BaseDirSource) -> Result<PathBuf, WorktreeError>
```

`BaseDirSource` identifies the value’s origin: `Env`, `ConfigFile(PathBuf)`,
or `Prompt`. Define it in that module and give it `Debug`, `Clone`,
`PartialEq`, `Eq`, and a plain-text `Display` for library errors. The error
module may refer to this type; it performs no rendering or host discovery.

The validator performs no filesystem access or host discovery. It checks:

1. Reject an empty or whitespace-only value with `BaseDirectoryEmpty`.
2. Accept a host-absolute path, preserving its spelling.
3. For a path that is not host-absolute, recognize obvious foreign syntax and
   return `BaseDirectoryForeignPath`.
4. Otherwise return `BaseDirectoryNotAbsolute`.

The host’s `Path::is_absolute()` is the authority for acceptance. In
particular, on Windows a drive-relative path and a path rooted on the current
drive are not absolute. See [Rust’s absolute-path contract](https://doc.rust-lang.org/std/path/struct.Path.html#method.is_absolute).

Do not silently make a relative value absolute by joining it to the current
working directory, canonicalizing it, or expanding variables. Do not strip
quotes. Trim manually entered prompt text, as today; preserve environment and
JSON values exactly, using trimming only to detect blank values. Thus an
absolute directory whose name ends in a space is not silently changed.

**Reader’s note:** the original draft proposed a complete Windows/POSIX
classifier that could accept paths on either style from any host. This review
uses Rust for acceptance and a small pure helper only for foreign-path error
messages. Maintaining a second Windows path parser would risk disagreement
with the actual filesystem operations. Acceptance tests must run on the target
OS; testing an error-message helper on macOS does not prove Windows behavior.

### Foreign-path diagnostics and native paths

A private pure helper takes text and `PathStyle` (`Posix` or `Windows`) and
recognizes foreign syntax, rather than implementing absolute-path acceptance.
The host style uses `cfg!(windows)`; WSL1 and WSL2 use POSIX paths.

For nonabsolute POSIX input, recognize an ASCII letter followed by `:`, or
an initial pair of backslashes (including Windows namespace prefixes), as
Windows syntax. For nonabsolute Windows input, recognize a leading `/` as
POSIX syntax. Call this helper only **after** the native absolute-path check:
`//server/share/wt` is absolute on POSIX and is also a Windows network-share
spelling. It must not be rejected just because it resembles another OS’s path.

| Input | On a POSIX host | On a Windows host |
| --- | --- | --- |
| `/home/me/wt`, `/mnt/b/wt` | Accept syntax | Foreign-path error |
| `b:\wt`, `B:/wt` | Foreign-path error | Accept syntax |
| `\\server\share\wt` | Foreign-path error | Accept syntax |
| `//server/share/wt` | Accept syntax | Accept syntax |
| `\\?\C:\wt`, `\\?\UNC\server\share\wt` | Foreign-path error | Accept syntax |
| `b:wt`, `b:` | Foreign-path error | Not-absolute error |
| `\wt` | Not-absolute error | Not-absolute error |
| `wt`, `./wt`, `../wt`, `~/wt`, `~` | Not-absolute error | Not-absolute error |
| empty or whitespace-only text | Empty error | Empty error |

“Accept syntax” does not prove the directory exists or that Git can use it.
Foreign-path errors describe the syntax as belonging to another OS; they must
not claim `b:wt` is absolute on Windows. An absolute POSIX path such as
`/home/me/b:\wt` is allowed: a colon or backslash inside a filename does not
make the entire path foreign. Windows drive and network-share roots are
allowed if they pass the directory checks. Device-specific names have no new
support promise; their usability remains subject to filesystem and Git errors.

### Resolution, setup, and persistence

The worktree package’s [base-directory resolver](../../lib/src/config.rs)
keeps its existing priority: `WT`, then `~/.worktree.json`, then setup in the
CLI if neither is configured.

- An unset or exactly empty `WT` continues to mean “no environment override.”
  This preserves the existing fallback behavior. A whitespace-only, relative,
  foreign, or otherwise invalid nonempty `WT` is an error; never fall back to a
  valid saved value and hide the mistake.
- Validate JSON `base_dir` before checking its filesystem state. Malformed
  JSON or a missing/non-string field remains `ConfigInvalidFormat`.
- After syntax validation, read metadata: a missing path gives
  `BaseDirectoryNotFound`; an existing file gives `BaseDirectoryNotDirectory`;
  other I/O failures retain their cause and name the path and origin. Do not
  collapse permission failures into “does not exist.” Then apply the existing
  check for `.git` directly under the base directory, whether file or folder.
  Symlinks to directories remain allowed; this fix adds no containment policy.
- Apply syntax validation to both suggested and typed setup choices **before**
  any directory creation or config write. Reject an existing file or repository
  root before saving. If the directory is absent, create it, then verify it is
  a directory and not a repository root before saving. Invalid choices show the
  reason and return to selection or text entry; genuine I/O failures stop with
  an error. Canceling retains the existing `Cancelled` behavior (exit 0).
- Only enter setup when the worktree-cli package’s
  [interactivity check](../../cli/src/env.rs) permits it: stdin and stderr are
  terminals, and `CI` is unset or empty. Otherwise return
  `BaseDirectoryNotConfigured` with instructions and exit 1, without prompting.
- The worktree package’s [configuration writer](../../lib/src/config.rs)
  validates syntax before serializing or opening its destination. Use the
  actual config path as `ConfigFile(path)` in its errors. It remains a writer,
  not a directory creator: callers establish filesystem suitability separately.
  Invalid input leaves an existing config byte-for-byte unchanged and creates
  neither a config file nor directories.

The text API and JSON schema require Unicode. Read `WT` with `var_os` so a
non-Unicode value cannot silently become “unset”; return a source-aware
`BaseDirectoryInvalidEncoding` error. Before saving a `Path`, use `to_str`
and reject an unrepresentable value with that error rather than saving lossy
display text. Selected candidate paths must also be converted losslessly before
creation. Human diagnostics may display such a value lossily, but filesystem
operations and persistence must never use that display string.

Use the biscuit-file package’s [home-directory helper](../../../biscuit-file/lib/src/file_reference/context.rs)
for config location and setup candidates. Enable its existing `file-reference`
feature on the worktree dependency. This intentionally replaces `dirs::home_dir`
in these configuration functions with the repository’s environment-first,
absolute-home contract. On native Windows `USERPROFILE` controls this lookup;
`HOME` alone does not. No home-path canonicalization or config migration is
introduced. Update the area’s dependency documentation for this feature change;
retain `dirs` if other modules still use it. No changes to cache or remote-store
locations are in scope.

The worktree package’s [worktree creation API](../../lib/src/worktree.rs)
continues to require callers to resolve and validate `base` beforehand, as its
current documentation states. This fix protects `wt` configuration paths and
the config writer; it does not add redundant filesystem checks to every library
operation. Amend that API’s documentation to make the prerequisite explicit.

### Errors and recovery

Extend the worktree package’s [error type](../../lib/src/error.rs), used by
library callers and the CLI, with source-aware path errors:

- `BaseDirectoryEmpty { origin }`
- `BaseDirectoryForeignPath { value, origin }`
- `BaseDirectoryNotAbsolute { value, origin }`
- `BaseDirectoryNotDirectory { value, origin }`
- `BaseDirectoryInvalidEncoding { value: PathBuf, origin }`
- Change `BaseDirectoryNotFound` to `{ value, origin }`.

Use `origin`, not `source`, for the `BaseDirSource` field: `thiserror` interprets
`source` as an underlying error implementing `std::error::Error`. A path’s
origin is context, not an underlying error. A contextual I/O error may have a
separate real `std::io::Error` cause. Update affected constructors, matches,
and tests; these public enum changes need no deprecation in this early-stage
repository. Retain the existing repository-root variants, but ensure their
messages and prompt rendering identify the origin too.

Every new error’s plain-text display names the value (when available), origin,
and remedy. The CLI enriches it through biscuit-terminal’s `Prose` and
`TerminalRenderable`, escaping user-controlled values and quoting link
attributes. A config error names `~/.worktree.json` and links to the actual
config path using the existing setup-error rendering approach. Send diagnostics
to stderr, exit 1, and emit no shell `cd:` protocol line. A rich error must not
be rendered twice or swallowed by returning `Ok(())`.

**Reader’s note:** the worktree-cli package’s current
[create command](../../cli/src/commands/create.rs) renders malformed-config and
config-repository errors and returns success. Correct those two branches to
exit 1 as well, consistent with the package’s [exit-code mapping](../../cli/src/exit.rs).
Otherwise an automation caller cannot distinguish a refused creation from a
successful one. Their recovery instructions also need host-valid example paths
and must not promise interactive setup in a noninteractive invocation.

Remedies depend on the origin:

- `WT`: replace or unset the environment override. Editing the JSON file alone
  will not override a nonempty `WT`.
- Config file: edit `base_dir`, or remove the config file and rerun creation in
  an interactive terminal. For a missing mount, restore the mount instead of
  changing config if that was the intended location.
- Prompt: explain why the choice cannot be used and ask for another absolute
  path. A literal `~/wt` gets a hint to enter the full home-directory path.

For example: `Base directory 'b:\wt' from ~/.worktree.json uses Windows path
syntax on this POSIX host. Set base_dir to an absolute directory on this host.`

**WSL hint.** Keep host discovery outside the pure validator. Only after a
foreign drive-path rejection, use the sniff package’s focused
[runtime detector](../../../sniff/lib/src/os/runtime.rs). Do not perform a full
host inventory or spawn Git. For a drive-absolute value such as `B:\wt` or
`B:/wt`, suggest `/mnt/b/wt` as the usual mount spelling, explicitly conditional
on that drive being mounted there. WSL can customize mount locations; never
claim the suggested path exists. Do not suggest `/mnt/b/wt` for `b:wt`, `b:`,
network shares, or namespace paths. Keep suggestion formatting pure and test it
with an injected WSL/native result; no test needs a real WSL mount.

### Existing invalid configuration and mutation boundary

`wt` never repairs or rewrites an existing invalid config automatically. The
next `wt create` reports its origin and remedy. Syntax rejection occurs before
base-directory creation, config writing, branch creation, worktree registration,
or include copying. This fix does not clean up the incident’s directory,
worktree registration, or branch; removing those requires a separate deliberate
user action. Once syntax succeeds, later filesystem or Git failures may leave
created directories, as today; this fix makes no general rollback promise.

## Non-goals

- Translating paths between operating systems automatically. The WSL hint is
  only a suggestion.
- Making the base-directory check reject paths *inside* a git repository
  instead of only a repository root (see Open Questions).
- Changing where worktrees go under the base directory.

## Testing

All new coverage is L1: pure logic or isolated filesystem/subprocess tests.
Follow the repository’s `rust-testing` skill and use the Test Toolkit for
environment guards. Add its dev-dependency where needed. No real terminal or
browser is required, and tests must never read or write the real user’s config.

Library coverage in the worktree package’s
[configuration tests](../../lib/src/config.rs):

- Test the foreign-syntax helper for both styles on every host; separately test
  native acceptance against the table on the matching target OS, including
  Windows network-share and extended-length syntax without accessing a server.
- Check empty, whitespace-only, relative, and foreign errors from all origins.
  Check that the validator preserves spaces in nonblank absolute values and
  does not expand `~`, quotes, or variable text.
- Test precedence: unset/empty `WT` falls back; an invalid nonempty override
  blocks a valid config; a valid override wins over malformed config. Guard
  process environment changes with the existing `serial(wt_env)` group and
  restore the prior values with `test_toolkit::EnvGuard` even after a panic.
- Supply a temporary config path to a private resolver/writer helper for
  filesystem tests. Cover relative/foreign JSON values, missing paths, files,
  repository roots with `.git` files and folders, and a valid directory. Check
  origin and actual config path on failures. I/O error classification can use
  an injected metadata result rather than brittle permission fixtures.
- For invalid saves, prove a missing config stays absent and an existing config
  stays byte-for-byte unchanged. Save and reload a valid absolute path.
- Where the target permits it, construct a non-Unicode environment/path value
  and assert explicit rejection with no write or fallback.

CLI coverage in the worktree-cli package:

- Run the incident case in a temporary repository and temporary home, with
  `WT` removed and invalid saved `base_dir` (`b:\wt` on POSIX,
  `/home/me/wt` on Windows). Set both `HOME` and `USERPROFILE` for the child
  after adopting the home helper above; assert config resolution uses that
  fixture. Set `NO_COLOR=1`, close stdin, and use archive-safe
  `biscuit_test_harness::bin_exe!("wt")`.
- Assert exit **1**, stderr naming the config and remedy, no `cd:` line, config
  bytes unchanged, no unexpected directory in the repository, no new branch,
  and no added worktree registration. Also cover an invalid `WT` overriding a
  valid config and a missing base directory with source-aware recovery text.
- Cover malformed config and config pointing at a repository root exiting 1
  after their custom error rendering. Test that user-provided markup is shown
  literally and config-link attributes are quoted through existing helpers.
- With neither source configured and closed stdin, assert exit 1 without setup.
  Test the setup decision loop with scripted choices: invalid input followed
  by valid input saves only the valid choice; invalid input followed by cancel
  saves nothing. The adapter into the existing prompt library stays thin.

At implementation time, run `just test` and `just lint` from `worktree/`.
Target-specific acceptance remains required on macOS, Linux, native Windows,
and WSL2 through the repository’s existing coverage workflow; the pure helper
is not a substitute. No additional CI matrix, performance spike, benchmark, or
L2 terminal test is warranted for these short string checks. Detect WSL only
on the error path, so valid resolution gains no runtime-discovery cost.

## Documentation

Update these alongside implementation, not during this draft review:

- The worktree [README](../../README.md), “Directory Resolution”: describe the
  absolute-path and directory rules, empty-`WT` fallback, foreign-path errors,
  and setup retry behavior.
- Add `worktree/docs/base-directory.md` describing `WT`, `~/.worktree.json`,
  precedence, host-valid examples (including WSL), setup, each rejection, and
  recovery. There is no existing configuration topic page. It must state the
  repository-root-only check accurately, without claiming all paths inside
  repositories are blocked, and must not refer readers to this fix’s snapshot.
- Review docs and comments for each behavior-changing function, including
  configuration resolution, writing, setup, and create-error handling. Existing
  setup comments saying “inside a git repo” drift from the root-only check;
  correct the prose to the actual check unless the author chooses the broader
  policy below.
- Update the worktree skill’s `cli-contracts.md` with validation, precedence,
  exit behavior, home isolation, and WSL’s POSIX path rule. Update any testing
  notes affected by the new config-home lookup without implying that unrelated
  cache/store lookups have changed.
- Update the area’s dependency documentation for enabled biscuit-file features
  and any Test Toolkit dev-dependency additions.

## Open Questions

The required behavior above uses the recommendations below as its draft
baseline. A different author choice needs corresponding changes to requirements
and tests before implementation.

1. **Should configuration accept home-relative references such as `~/wt`?**
   This would change the absolute-input contract, not merely improve an error.

   - **Reject with a full-path hint (recommended).** Pros: one rule across all
     sources, no hidden dependence on which user launches `wt`, and no new
     reference-resolution behavior. Cons: users must enter the full path;
     quoted shell values and JSON do not get the expansion users may expect.
   - **Expand only during interactive setup.** Pros: convenient typing while
     still saving an absolute path. Cons: the same text behaves differently in
     the prompt and JSON; supported reference forms need a separate contract.
   - **Expand in every source.** Pros: consistent convenience for `~/wt`.
     Cons: meaning depends on the runtime home, existing values can change
     interpretation, and serialization must specify whether it saves the
     reference or the resolved absolute path.

   Recommend rejection for this fix because it closes the wrong-OS creation
   bug without introducing another path language. If expansion is chosen, use
   the biscuit-file package’s [FileReference resolver](../../../biscuit-file/lib/src/file_reference/resolve.rs)
   under an explicit context; do not hand-roll tilde replacement. Restrict its
   accepted forms explicitly so this choice does not accidentally enable
   repository searches, URL fetching, or variable interpolation.

2. **Should all base directories inside repositories be rejected?**
   The current check rejects only directories with `.git` directly underneath,
   not `/home/me/repo/wt`. Broader rejection changes where users can store
   worktrees and how symlinks, bare repositories, and environment-selected Git
   directories are interpreted.

   - **Keep the root-only check in this fix (recommended).** Pros: surgical
     change that addresses the incident; no new repository discovery on the
     successful path. Cons: an intentionally or accidentally nested absolute
     path still passes.
   - **Reject any directory discovered inside a repository.** Pros: prevents
     nested storage. Cons: intentionally nested layouts stop working, discovery
     must also handle not-yet-created paths and symlink targets, and a `.git`
     ancestor walk alone does not cover bare or environment-selected repos.
   - **Warn during setup but allow nesting.** Pros: flags likely mistakes while
     preserving layouts. Cons: adds a prompt policy that environment and JSON
     users do not see and leaves automation without the same warning.

   Recommend retaining the current boundary because the absolute-path rule
   already prevents current-directory-relative creation. Decide broader
   containment separately with concrete layout requirements. If chosen, use
   focused sniff repository discovery and biscuit-file path identity rather
   than string-prefix checks or an ad hoc ancestor walk; document its extra
   discovery work and symlink policy before implementation.

3. **Should a missing saved directory restart setup?**
   A missing mount and a deleted directory look alike at this stage.

   - **Keep a hard error with recovery guidance (recommended).** Pros: preserves
     config, does not conceal unavailable storage, works consistently in scripts.
     Cons: recovery may require editing a file or restoring a mount.
   - **Offer explicit reconfiguration in an interactive terminal.** Pros:
     easier recovery. Cons: more states and confirmation handling; must retain
     the old config on cancel or failure and never activate in scripts.
   - **Recreate the missing directory automatically.** Pros: quick recovery
     when it was simply deleted. Cons: can create local storage where a mount
     should have been, silently putting worktrees on the wrong filesystem.

   Recommend the hard error: identifying the source and intended path gives
   enough information to repair it without changing storage unexpectedly.
