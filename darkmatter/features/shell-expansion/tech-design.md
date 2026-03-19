# Shell Expansion Tech Design

This document defines the implementation-ready technical design for the `shell-expansion` feature in Darkmatter's Stage 1 compose pipeline. It is derived from:

- `darkmatter/features/shell-expansion/spec.md`
- `darkmatter/docs/preparation/shell-expansion.md`
- the current compose pipeline in `darkmatter/lib/src/markdown/compose/`
- the current compose CLI flow in `darkmatter/cli/src/commands.rs`

The design goal is to preserve the behavior required by the spec while fitting the existing Darkmatter architecture cleanly and safely.

## Prerequisite

Darkmatter recently adopted `biscuit-file::FileReference` for path resolution in transclusion and CLI file loading. Shell expansion does not resolve content references itself, but the feature should align with the same environment model:

- repository-root detection should behave consistently with the rest of the project
- compose operations that originate from a file path should keep source-aware behavior
- any future shell-expansion options that accept file-like references should route through `biscuit-file` instead of inventing new path semantics

For v1, that means policy-file discovery should use repository detection compatible with the current repo-aware behavior, while command execution itself remains plain process execution with no file-reference expansion.

## Purpose

The feature allows Markdown documents to inject the output of explicitly approved host commands using:

```md
::shell <command> <params>
```

The design focuses on four outcomes:

1. preserve the functional contract in the shell-expansion spec
2. keep the library safe enough to expose in both library and CLI usage
3. fit the current Stage 1 and Stage 2 pipeline architecture without bending existing compose semantics
4. keep policy and approval behavior deterministic across recursive document composition

## Scope

In scope:

1. `::shell` directive parsing
2. Stage 1 ordering and pipeline integration
3. command tokenization and validation
4. built-in blacklist enforcement
5. persisted whitelist and persisted user blacklist behavior
6. approval flow for library and CLI callers
7. command execution, timeout, and output capture
8. recursive runtime behavior across Stage 2 transclusion
9. testing strategy

Out of scope for v1:

1. general-purpose shell interpretation
2. pipes, redirection, command substitution, or shell control operators
3. OS sandboxing or container isolation
4. caching command output
5. remote or shared approval stores
6. output post-processing beyond literal replacement

## Current Baseline

The current compose pipeline in `darkmatter/lib/src/markdown/compose/mod.rs` runs Stage 1 in this order:

1. replacement
2. interpolation
3. TOC linking
4. cleanup
5. normalization

Stage 2 then runs transclusion with a recursive `TransclusionRuntime`.

Relevant existing implementation points:

- `ComposeOptions` in `compose/types.rs` is the public configuration surface
- `Stage1Stages` controls Stage 1 toggles
- `ComposeReport` records stage counts and warnings
- `ComposeSource` and `TransclusionOptions` already carry source context
- `parse_utils::find_code_regions()` is already used to skip directives inside fenced code blocks
- `run_compose()` in `darkmatter/cli/src/commands.rs` already builds `ComposeOptions` and sets `with_source_file(...)` for file-backed compose runs

Current gaps:

- there is no shell-expansion stage or module
- there is no approval callback or runtime
- there is no persisted whitelist or user blacklist support
- recursive compose state only covers transclusion depth and cycle detection

## Functional Contract Summary

The stage must implement the following behavior from the specification:

1. parse `::shell <command> <params>` directives in markdown
2. reject commands that do not exist on the host
3. reject commands that match the global blacklist
4. request approval for commands not already allowed
5. execute approved commands and replace the directive with captured output
6. capture both stdout and stderr
7. terminate commands that exceed the timeout
8. fail the pipeline on non-zero exit status and include stdout and stderr in the error
9. remove the directive line when a successful command emits no output
10. support approval outcomes for exact allow, command-level allow, allow-once, deny, and blacklist

## Primary Recommendation

### Execute directly, not through a shell

Even though the feature is named shell expansion, v1 should not invoke:

- `/bin/sh -c`
- `bash -lc`
- `zsh -c`
- `cmd /C`
- `powershell -Command`
- any equivalent shell interpreter wrapper

Instead, the implementation should:

1. parse the command line into an executable and argv tokens
2. reject unsupported shell syntax before execution
3. execute the process directly with `std::process::Command`

This is the most important design choice in the feature.

Reasons:

1. it makes blacklist enforcement much more reliable
2. it avoids accidentally supporting redirection, pipelines, and command substitution
3. it makes approvals stable because matching is based on normalized argv, not equivalent shell spellings
4. it aligns with the spec's security intent better than a real shell interpreter would

Implication:

The feature supports shell-like tokenization, not shell semantics.

## Stage Placement and Ordering

The new Stage 1 order should be:

1. replacement
2. interpolation
3. TOC linking
4. shell expansion
5. cleanup
6. normalization

This preserves the documented position in `darkmatter/docs/darkmatter-pipeline.md`.

Rationale:

1. interpolation must run before shell expansion so `{{ ... }}` values can contribute to command arguments
2. cleanup and normalization should operate on the markdown emitted by shell commands
3. keeping TOC linking before shell expansion preserves the currently documented pipeline and avoids runtime-generated headings affecting already-generated TOC content

Tradeoff:

Headings emitted by shell expansion will not affect `::toc-linking` output in the same document. This is acceptable for v1 because it keeps pipeline behavior consistent and predictable.

## Proposed Public API Changes

### `Stage1Stages`

Add a new stage toggle in `darkmatter/lib/src/markdown/compose/types.rs`:

```rust
pub struct Stage1Stages {
    pub replacement: bool,
    pub interpolation: bool,
    pub toc_linking: bool,
    pub shell_expansion: bool,
    pub cleanup: bool,
    pub normalization: bool,
}
```

Defaults:

- `shell_expansion = true`

Add a convenience constructor:

```rust
impl Stage1Stages {
    pub fn only_shell_expansion() -> Self {
        Self {
            shell_expansion: true,
            ..Self::none()
        }
    }
}
```

### `ComposeOptions`

Extend the public options surface with shell settings:

```rust
pub struct ComposeOptions {
    pub stages: Stage1Stages,
    pub stage2: Stage2Stages,
    pub transclusion: TransclusionOptions,
    pub shell: ShellExpansionOptions,
    pub external_state: Option<serde_json::Value>,
    pub fail_fast: bool,
    // existing internal fields...
}
```

```rust
pub struct ShellExpansionOptions {
    pub timeout: std::time::Duration,
    pub policy_root: Option<std::path::PathBuf>,
    pub working_directory: Option<std::path::PathBuf>,
    pub approval_handler: Option<std::sync::Arc<dyn ShellApprovalHandler>>,
}
```

Recommended defaults:

- `timeout = 10 seconds`
- `policy_root = None`
- `working_directory = None`
- `approval_handler = None`

Meaning:

1. `policy_root` overrides where `.darkmatter-shell-whitelist` and `.darkmatter-shell-blacklist` are resolved
2. `working_directory` overrides the cwd used for process execution
3. `approval_handler = None` means the library must fail with an approval-required error instead of trying to prompt

`policy_root` is a better name than `policy_dir` because the resolution target is conceptually the root directory that contains the policy files. This supersedes the spec's `policy_dir` naming.

`ComposeOptions::new()` must initialize `shell: ShellExpansionOptions::default()` and a `with_shell(mut self, shell: ShellExpansionOptions) -> Self` builder should be added following the existing builder pattern.

### Approval callback

The library should expose approval as a callback contract and never own terminal prompting:

```rust
pub trait ShellApprovalHandler: Send + Sync {
    fn approve(
        &self,
        request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, ShellExpansionError>;
}
```

```rust
pub struct ShellApprovalRequest {
    pub source: ComposeSource,
    pub line: usize,
    pub raw_command: String,
    pub executable: String,
    pub args: Vec<String>,
    pub normalized_exact: String,
    pub whitelist_path: std::path::PathBuf,
    pub blacklist_path: std::path::PathBuf,
}
```

```rust
pub enum ShellApprovalDecision {
    AllowExactPersist,
    AllowCommandPersist,
    AllowOnce,
    Deny,
    BlacklistPersist,
}
```

### `ComposeReport`

Extend reporting with shell-expansion counters:

```rust
pub struct ComposeReport {
    pub replacements_applied: usize,
    pub interpolations_applied: usize,
    pub toc_links_generated: usize,
    pub shell_expansions_applied: usize,
    pub shell_approvals_used: usize,
    pub cleanup_changed: bool,
    pub normalization_report: Option<NormalizationReport>,
    pub transclusions_applied: usize,
    pub transclusions_skipped: usize,
    pub max_transclusion_depth: usize,
    pub warnings: Vec<ComposeWarning>,
}
```

Semantics:

- `shell_expansions_applied` counts successful directives, including directives removed because output was empty
- `shell_approvals_used` counts approvals granted during the current top-level compose invocation

`ComposeReport::has_changes()` and `ComposeReport::summary()` should also include shell-expansion changes.

Required updates:

```rust
// has_changes() must include shell expansions
fn has_changes(&self) -> bool {
    // ...existing checks...
    || self.shell_expansions_applied > 0
}

// summary() must include shell expansion counts
fn summary(&self) -> String {
    // ...existing summary lines...
    // add: shell_expansions_applied and shell_approvals_used
}
```

## Internal Runtime Changes

The current recursive path only threads `TransclusionRuntime`. That is not sufficient for shell approvals because allow-once and file-backed policy updates must remain visible across recursively composed documents.

Introduce an internal pipeline runtime:

```rust
pub(crate) struct PipelineRuntime {
    pub transclusion: TransclusionRuntime,
    pub shell: ShellExpansionRuntime,
}
```

```rust
pub(crate) struct ShellExpansionRuntime {
    pub allow_once: std::collections::HashSet<String>,
    pub whitelist: ShellRuleSet,
    pub user_blacklist: ShellRuleSet,
    pub policy_paths: Option<ShellPolicyPaths>,
    pub approvals_used: usize,
}
```

```rust
pub struct ShellPolicyPaths {
    pub whitelist: std::path::PathBuf,
    pub blacklist: std::path::PathBuf,
}
```

```rust
pub struct ShellRuleSet {
    pub entries: Vec<ShellRuleEntry>,
}

pub enum ShellRuleEntry {
    Exact(String),
    Prefix(String),
}
```

```rust
impl ShellExpansionRuntime {
    pub fn take_recent_approval_count(&mut self) -> usize {
        let count = self.approvals_used;
        self.approvals_used = 0;
        count
    }
}
```

Design requirements:

1. `run_compose_pipeline()` creates `PipelineRuntime` once at the root
2. `run_compose_pipeline_internal()` receives `&mut PipelineRuntime`
3. transcluded child documents reuse the same shell runtime

This preserves the intended meaning of:

1. allow-once for a single compose run instead of a single document
2. persisted approvals taking effect immediately for later directives in the same run
3. policy files being loaded once instead of repeatedly per document

## Module Layout

Add a new compose module:

```text
darkmatter/lib/src/markdown/compose/shell_expansion/
├── mod.rs
├── executor.rs
├── parser.rs
├── policy.rs
├── store.rs
├── tokenize.rs
└── types.rs
```

Responsibilities:

- `mod.rs`: stage orchestration, stage entry points, and the `apply_replacements_in_reverse` helper that applies `Vec<(Range<usize>, String)>` replacements from highest byte offset to lowest
- `parser.rs`: markdown line scanning and directive extraction
- `tokenize.rs`: shell-like argv tokenization
- `policy.rs`: built-in blacklist logic, whitelist matching, normalization
- `store.rs`: policy-file discovery, loading, and append helpers
- `executor.rs`: process spawning, timeout, and output capture
- `types.rs`: directive types, policy types, runtime state, and shell-expansion errors

`compose/mod.rs` should:

1. `mod shell_expansion;`
2. re-export the public approval types if they are part of the public API
3. insert `run_shell_expansion_stage(...)` between TOC linking and cleanup
4. switch the recursive runner from `TransclusionRuntime` to `PipelineRuntime`

## Directive Parsing

### Syntax

Supported v1 syntax:

```text
::shell <command line>
```

Examples:

```md
::shell git status --short
::shell rg "TODO|FIXME" darkmatter/lib/src
::shell printf "hello\n"
```

Directive rules:

1. the directive must occupy its own logical line
2. leading and trailing whitespace around the directive line are ignored
3. directives inside fenced code blocks are ignored
4. directives in inline code are naturally ignored because parsing is line based and requires a line prefix
5. v1 does not support `key=value` directive options

### Parser strategy

Reuse the current compose pattern already used by Stage 2 parsing:

1. compute excluded code regions with `parse_utils::find_code_regions()`
2. scan content line by line
3. match trimmed lines beginning with `::shell`
4. capture the remainder of the line as raw command text
5. tokenize that raw command text into executable and args

Proposed directive type:

```rust
pub struct ShellDirective {
    pub raw_command: String,
    pub executable: String,
    pub args: Vec<String>,
    pub span: std::ops::Range<usize>,
    pub line: usize,
}
```

## Tokenization and Validation Rules

### Supported tokenization

The tokenizer should support:

1. unquoted tokens
2. single-quoted strings
3. double-quoted strings
4. backslash escapes for spaces, quotes, and backslashes

### Rejected syntax

The tokenizer should reject:

1. empty command lines
2. unterminated quotes
3. raw metacharacters outside quotes:
   - `>`
   - `>>`
   - `<`
   - `|`
   - `;`
   - `&&`
   - `||`
   - `` ` ``
   - `$(`

This is intentionally stricter than the textual blacklist in the spec. Since v1 never launches a shell interpreter, these tokens provide risk without adding legitimate value.

### Additional rejection rules

The implementation should also reject interpreter-wrapper forms that reintroduce shell semantics indirectly:

1. `sh -c ...`
2. `bash -c ...`
3. `bash -lc ...`
4. `zsh -c ...`
5. `fish -c ...`
6. `cmd /C ...`
7. `powershell -Command ...`
8. `pwsh -Command ...`

This is a design recommendation where the spec is ambiguous. Technically these could be executed directly via `Command`, but allowing them would bypass the central safety goal of the feature.

Note that the spec's directive examples include `bash -lc` as an example, but this tech design recommends rejecting interpreter-wrapper forms as a security hardening measure.

## Validation Model

For each directive, the validation order should be:

1. parse the directive and tokenize argv
2. reject unsupported syntax
3. confirm the executable exists on the host
4. reject built-in blacklist matches
5. reject persisted user blacklist matches
6. allow exact whitelist matches
7. allow command-prefix whitelist matches
8. allow in-memory allow-once matches
9. otherwise request approval or fail with `ApprovalRequired`

This order ensures:

1. blacklisted commands never become approvable
2. approval is only requested for syntactically valid and executable commands
3. allow-once never bypasses blacklist rules

## Executable Resolution

Use the `which` crate to resolve the executable token before execution.

Behavior:

1. resolution uses the current process `PATH`
2. matching is done on the executable token only
3. policy matching uses the executable basename, not the resolved absolute path

If lookup fails, return a hard error matching the spec's behavior.

## Blacklist Design

### Built-in blacklist

The spec's blacklist should be encoded as structured rules, not string contains checks.

Recommended rule model:

```rust
pub enum BlacklistRule {
    Executable(&'static str),
    ExecutablePrefix(&'static str),
    SubcommandPrefix {
        executable: &'static str,
        prefix: &'static [&'static str],
    },
    ArgExact {
        executable: &'static str,
        arg: &'static str,
    },
    ArgPrefix {
        executable: &'static str,
        arg_prefix: &'static str,
    },
    RawToken(&'static str),
}
```

Examples:

1. `rm` becomes `Executable("rm")`
2. `mkfs*` becomes `ExecutablePrefix("mkfs")`
3. `git reset` becomes `SubcommandPrefix { executable: "git", prefix: &["reset"] }`
4. `find*-delete` becomes `ArgExact { executable: "find", arg: "-delete" }`
5. `redis-cli FLUSH*` becomes `ArgPrefix { executable: "redis-cli", arg_prefix: "FLUSH" }`
6. redirection tokens become `RawToken(">")` and `RawToken(">>")`

Matching should operate on the executable basename and argv tokens to keep policies stable across machines.

### User blacklist

The spec's `blacklist` approval action requires a persisted user blacklist in addition to the built-in blacklist.

Policy file:

- repo mode: `<repo-root>/.darkmatter-shell-blacklist`
- home mode: `${HOME}/.darkmatter-shell-blacklist`

User blacklist checks should run after built-in blacklist checks and before whitelist checks.

## Policy File Discovery and Storage

### Policy root resolution

Resolve policy files using this order:

1. `options.shell.policy_root`, if set
2. otherwise the compose source file's parent directory when `ComposeSource::File` is available
3. otherwise `std::env::current_dir()`
4. if that base path is inside a git repository, use the repository root
5. otherwise use `${HOME}`

This is the one intentional improvement over the literal spec. The spec talks about CWD, but the current Darkmatter compose flow already carries source-file context, and file-backed compose should prefer file-rooted behavior over caller-shell quirks. When the source is unknown or stdin-backed, the behavior falls back to CWD and then HOME exactly as the spec expects.

Policy file names:

- `.darkmatter-shell-whitelist`
- `.darkmatter-shell-blacklist`

### File format

Use a line-oriented, human-editable format:

```text
# comments are allowed
exact git status --short
exact rg "TODO|FIXME" darkmatter/lib/src
prefix git
prefix rg
```

Rules:

1. blank lines are ignored
2. lines beginning with `#` are comments
3. `exact <command line>` matches the normalized executable and full argv
4. `prefix <command>` matches the executable with any argv tail
5. policy entries are parsed with the same tokenizer used for `::shell`

### Persistence strategy

Persist approvals and blacklist additions by append-only writes.

Reasons:

1. it avoids read-modify-write complexity
2. duplicate entries are harmless and can be deduplicated in memory
3. it is easy to explain and robust in the CLI approval flow

Normalization rules:

1. store the executable basename, not the resolved absolute path
2. serialize args with stable quoting only when required
3. always append a trailing newline

## Approval Flow

### Library behavior

If a command is valid but not already approved:

1. call `approval_handler` when provided
2. otherwise return `ShellExpansionError::ApprovalRequired`

The library never prompts directly.

### Decision handling

`AllowExactPersist`:

1. append `exact <normalized command line>` to `.darkmatter-shell-whitelist`
2. update the in-memory whitelist
3. execute the command

`AllowCommandPersist`:

1. append `prefix <executable>` to `.darkmatter-shell-whitelist`
2. update the in-memory whitelist
3. execute the command

`AllowOnce`:

1. insert the exact normalized signature into `runtime.allow_once`
2. increment `runtime.approvals_used`
3. execute the command

`Deny`:

1. return a hard error
2. do not mutate policy files

`BlacklistPersist`:

1. append `exact <normalized command line>` to `.darkmatter-shell-blacklist`
2. update the in-memory user blacklist
3. return a hard error

## CLI Integration

`run_compose()` in `darkmatter/cli/src/commands.rs` should continue to build `ComposeOptions`, then attach a CLI approval handler only when prompting is actually safe.

Prompting conditions:

1. input is a file path, not stdin
2. stdin is a terminal
3. stderr is a terminal

If those conditions are not met, leave `approval_handler` unset and let the library return `ApprovalRequired`.

The CLI should then surface an error that includes:

1. the command
2. the whitelist path
3. the blacklist path
4. the exact manual entries the user can add

Recommended prompt shape:

```text
Unapproved shell command in docs/example.md:12
  git status --short

1. allow exact and save
2. allow command and save
3. allow once
4. deny
5. blacklist and stop
```

This keeps `md compose -` and other pipe-driven workflows non-interactive and deterministic.

## Execution Model

### Working directory

Use this resolution order:

1. `options.shell.working_directory`, if set
2. the directory containing the file-backed compose source when available
3. `options.shell.policy_root`, if set and valid
4. `std::env::current_dir()`

This keeps file-backed compose runs intuitive while still letting callers override the cwd explicitly.

This intentionally extends the spec's three-step resolution order by inserting source-file-aware resolution, which is more intuitive for file-backed compose runs.

### Process spawning

Execute commands with `std::process::Command`:

```rust
fn build_command(
    resolved_executable: &std::path::Path,
    directive: &ShellDirective,
    working_dir: &std::path::Path,
) -> std::process::Command {
    let mut command = std::process::Command::new(resolved_executable);
    command
        .args(&directive.args)
        .current_dir(working_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    command
}
```

Important behavior:

1. stdin is always null
2. interactive commands are unsupported in v1
3. commands cannot quietly block waiting for input

### Timeout handling

Default timeout is 10 seconds.

Implementation shape:

1. spawn the child process
2. drain stdout and stderr concurrently
3. poll `try_wait()` until the process completes or the timeout expires
4. kill the child on timeout
5. wait for cleanup
6. return a hard timeout error

### Output capture and replacement

Capture:

1. stdout
2. stderr
3. a combined replacement buffer

Recommended behavior for the combined replacement buffer:

1. preserve best-effort interleaving when practical
2. fall back to stdout followed by stderr if exact ordering is too complex for v1

That is an implementation detail the spec does not constrain. The exact ordering only matters for successful replacement text, not for approval or safety.

Replacement rules:

1. exit code `0` and both streams empty means replace the directive with an empty string
2. exit code `0` and any output means replace the directive with the captured output verbatim
3. non-zero exit means fail the pipeline and do not mutate the document

All bytes should be decoded using `String::from_utf8_lossy()`.

## Error Model

Introduce a dedicated shell-expansion error type:

```rust
pub enum ShellExpansionError {
    ParseDirective { line: usize, message: String },
    CommandNotFound { command: String, line: usize },
    Blacklisted { command: String, reason: String, line: usize },
    ApprovalRequired {
        command: String,
        whitelist_path: std::path::PathBuf,
        blacklist_path: std::path::PathBuf,
        line: usize,
    },
    Denied { command: String, line: usize },
    Timeout {
        command: String,
        timeout: std::time::Duration,
        line: usize,
    },
    ExecutionFailed {
        command: String,
        code: i32,
        stdout: String,
        stderr: String,
        line: usize,
    },
    PolicyIo {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}
```

These errors should convert into the compose pipeline's existing `MarkdownError::Compose(...)` surface the same way other stage failures do today.

Important behavior:

Shell-expansion failures should always be hard failures. They should not degrade to warnings when `fail_fast = false`.

Reason:

1. the spec explicitly says the pipeline exits on these failures
2. continuing after a denied or failed command creates ambiguous output
3. silent degradation would weaken the security model

## Pipeline Algorithm

Per document:

1. parse shell directives outside code regions
2. if none exist, return early
3. ensure policy files are resolved and loaded once
4. validate and execute directives in source order
5. collect replacement spans
6. apply replacements from end to start
7. update the compose report

Recommended stage entry point:

```rust
fn run_shell_expansion_stage(
    document: &mut Markdown,
    options: &ComposeOptions,
    runtime: &mut PipelineRuntime,
    report: &mut ComposeReport,
) -> MarkdownResult<()> {
    let directives = shell_expansion::parse_directives(&document.content)?;
    if directives.is_empty() {
        return Ok(());
    }

    let policy_paths = shell_expansion::resolve_policy_paths(
        &options.shell,
        &options.transclusion.source,
    )?;
    runtime.shell.ensure_loaded(&policy_paths)?;

    let mut replacements = Vec::new();

    for directive in directives {
        let replacement = shell_expansion::execute_directive(
            &directive,
            options,
            &policy_paths,
            &mut runtime.shell,
        )?;
        replacements.push((directive.span.clone(), replacement));
        report.shell_expansions_applied += 1;
    }

    apply_replacements_in_reverse(&mut document.content, replacements);
    report.shell_approvals_used += runtime.shell.take_recent_approval_count();
    Ok(())
}
```

## Interaction With Stage 2 Transclusion

Recursive composes triggered by `::file` and related Stage 2 features must share the same shell-expansion runtime.

Required behavior:

1. child documents must reuse allow-once approvals granted earlier in the same compose run
2. policy changes made during a parent document compose must be visible to child documents and vice versa
3. approval prompts should not repeat for the same allow-once command during the same top-level compose invocation

This is the main reason the feature should move from a single-purpose `TransclusionRuntime` to a broader `PipelineRuntime`.

## Testing Strategy

### Unit tests

Tokenizer and parser:

1. parse simple commands
2. parse quoted args
3. parse escaped whitespace and quotes
4. reject unterminated quotes
5. reject unsupported metacharacters
6. ignore directives inside fenced code blocks

Blacklist matching:

1. `rm`
2. `find . -delete`
3. `git reset --hard`
4. `docker system prune -af`
5. `psql -c "DROP TABLE x"`
6. `bash -lc '...'`

Policy store:

1. load `exact` entries
2. load `prefix` entries
3. ignore comments and blank lines
4. append normalized entries
5. deduplicate in memory
6. resolve repo-root versus HOME policy paths

Runtime behavior:

1. `AllowOnce` matches only the exact normalized signature
2. `AllowCommandPersist` matches future arg variants
3. blacklist checks run before approval checks

Executor:

1. stdout-only success
2. stderr-only success
3. mixed stdout and stderr success
4. empty success removes the directive
5. non-zero exit returns captured streams in the error
6. timeout kills the child and errors

### Integration tests

Pipeline integration:

1. interpolation runs before shell expansion
2. cleanup runs after shell expansion
3. recursive transclusion shares shell runtime
4. allow-once works across child documents within one compose run
5. hard failures abort the compose even when `fail_fast = false`

CLI integration:

1. `md compose file.md` can install an approval handler
2. `md compose -` does not prompt
3. manual-whitelist guidance is emitted in non-interactive mode
4. persisted approvals are reused on later runs

Recommended crates:

- `tempfile` for policy-file tests
- `serial_test` for tests that manipulate environment variables or current directory

## Rollout Risks

1. shell-interpreter loopholes would undermine the entire security posture if not explicitly blocked
2. policy-root resolution can become surprising if source-file and cwd behavior are mixed inconsistently
3. shared runtime changes touch recursive pipeline plumbing and can regress Stage 2 if introduced carelessly
4. child-process timeout handling can leak processes if kill-and-wait behavior is incomplete
5. literal output insertion can expose formatting oddities that cleanup may or may not normalize as intended

## Implementation Phases

### Phase 1

1. add Stage 1 toggle, options, report fields, and shell-expansion error type
2. add parser and tokenizer
3. add built-in blacklist and executable lookup
4. add policy discovery and storage

### Phase 2

1. add executor with timeout and capture
2. add `PipelineRuntime`
3. wire shell expansion into `compose/mod.rs`
4. add unit and integration tests for library behavior

### Phase 3

1. add CLI approval handler
2. add non-interactive error guidance
3. add CLI tests
4. update package docs and dependency docs if `which` is added

## Open Questions

1. Should the combined success output preserve exact stdout/stderr interleaving, or is best-effort ordering sufficient for v1? Recommendation: best-effort is sufficient.
2. Should future versions allow document-relative execution roots distinct from policy roots? Recommendation: not in v1.
3. Should policy discovery always prefer source-file ancestry over CWD when a source file exists? Recommendation: yes, because that is more consistent for file-backed compose.

## Recommendation

Ship v1 with:

1. direct process execution through `std::process::Command`
2. strict rejection of shell operators and shell-interpreter wrapper forms
3. append-only `.darkmatter-shell-whitelist` and `.darkmatter-shell-blacklist` files
4. a caller-owned approval callback
5. a shared runtime that survives recursive composition

This gives Darkmatter a usable `::shell` feature without turning the markdown pipeline into a general-purpose shell environment.
