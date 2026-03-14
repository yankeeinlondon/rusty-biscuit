# Stage 1 Design: Shell Expansion

## Skills Used

- `darkmatter`
- `rust`

## Pre-Requisite

It's important to understand that the `biscuit-file` package just added powerful functionality for converting a file reference into a valid file path. We will not only want to use this in our implementation of the "shell expansion" feature but ensure that all areas of Darkmatter which currently are performing some form of file reference resolution use the functionality coming from `biscuit-file`.

## Purpose

Define an implementation-ready technical design for Darkmatter's Stage 1 shell expansion feature, based on the functional specification in `darkmatter/docs/preparation/shell-expansion.md` and aligned with the current transform pipeline in `darkmatter/lib/src/markdown/transform/`.

This design focuses on three things:

1. preserving the functional contract from the spec
2. making the feature safe enough to ship in a library and CLI
3. fitting the existing Stage 1 and recursive Stage 2 transform architecture

## Scope

In scope:

1. `::shell ...` directive parsing
2. stage ordering and pipeline integration
3. command validation, blacklist matching, whitelist lookup, and approvals
4. process execution, timeout handling, and output capture
5. library and CLI integration points
6. policy file format and persistence behavior
7. testing strategy

Out of scope for v1:

1. full shell interpreter semantics (`|`, `&&`, redirects, command substitution, globbing)
2. OS-level sandboxing, chroot, seccomp, or container isolation
3. caching shell command output
4. remote execution or distributed policy storage
5. automatic formatting of shell output beyond literal insertion

## Current Baseline

Darkmatter already has a structured transform pipeline:

1. Stage 1: replacement, interpolation, TOC linking, cleanup, normalization
2. Stage 2: block transclusion and frontmatter transclusion

Relevant implementation pieces already exist:

- `TransformOptions`, `Stage1Stages`, and `TransformReport` are extensible
- Stage scanners already ignore directives inside fenced code blocks via `parse_utils::find_code_regions()`
- recursive processing already shares mutable runtime state for transclusion
- the `compose` CLI already builds `TransformOptions` and runs `transform_with()`

Current gaps:

- there is no shell-expansion stage or module
- there is no approval callback mechanism in the library
- there is no persisted allow/deny policy store
- there is no shared runtime for shell approvals across recursive transforms

## Functional Contract Summary

The shell-expansion stage must implement the following behavior from the spec:

1. `::shell <command> <params>` may appear in markdown content.
2. if the referenced command does not exist on the host, the pipeline exits with an error
3. if the command matches the global blacklist, the pipeline exits with an error
4. if the command is not in the whitelist, the caller must approve it
5. if approved, execute the command and replace the directive with captured output
6. capture both stdout and stderr
7. if the command exceeds the timeout, terminate it and error
8. if exit code is non-zero, error and include stdout/stderr in the message
9. if both stdout and stderr are empty and exit code is zero, remove the directive line
10. support these approval outcomes:
    - allow exact command and persist it
    - allow command with any parameters and persist it
    - allow once for the current transform invocation
    - deny
    - blacklist and stop

## Primary Design Decision

### Execute commands directly, not through a shell

Even though the feature is called "shell expansion", the implementation should not invoke `/bin/sh -c`, `cmd /C`, or any equivalent shell interpreter.

Instead, v1 should:

1. parse the directive into an executable plus argv tokens
2. reject shell metacharacters and unsupported syntax before execution
3. run the executable directly via `std::process::Command`

This is the most important design choice in the document.

Reasons:

1. blacklist enforcement is much more reliable against structured argv than raw shell text
2. direct execution avoids accidental support for redirection, pipes, command substitution, and environment mutation
3. timeout and output capture are simpler and safer
4. approval matching becomes deterministic because it is based on normalized argv, not quoting differences

Implication:

The feature supports "shell-like command lines" for quoting and spacing, but not the full shell language.

That is stricter than the functional spec's wording, but it is fully compatible with the security intent of the spec and makes the feature implementable without creating an obvious escape hatch.

## Stage Placement and Ordering

Proposed Stage 1 order:

1. replacement
2. interpolation
3. TOC linking
4. shell expansion
5. cleanup
6. normalization

This keeps shell expansion in the same Stage 1 slot already documented in `darkmatter/docs/darkmatter-pipeline.md`.

Rationale:

1. interpolation must run before shell expansion so command arguments can include `{{ ... }}` values
2. cleanup and normalization should operate on shell-generated markdown
3. keeping TOC linking ahead of shell expansion preserves the currently documented Stage 1 order

Tradeoff:

Headings emitted by shell expansion will not affect `::toc-linking` generated lists in the same document. That is acceptable for v1 because it matches the published pipeline ordering and avoids surprising TOC side effects from runtime command output.

## Public API Changes

### `Stage1Stages`

Add a dedicated toggle:

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

Convenience builder:

```rust
impl Stage1Stages {
    pub fn only_shell_expansion() -> Self { ... }
}
```

### `TransformOptions`

Add shell-specific configuration:

```rust
pub struct TransformOptions {
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
    pub policy_dir: Option<std::path::PathBuf>,
    pub working_directory: Option<std::path::PathBuf>,
    pub approval_handler: Option<std::sync::Arc<dyn ShellApprovalHandler>>,
}
```

Defaults:

- `timeout = 10s`
- `policy_dir = None`
- `working_directory = None`
- `approval_handler = None`

Meaning:

1. `policy_dir` overrides where repo detection starts for `.shell-whitelist` and `.shell-blacklist`
2. `working_directory` overrides the process cwd used when executing approved commands
3. `approval_handler = None` means the library errors with `ApprovalRequired` instead of prompting

### Approval callback

The library should not own terminal prompting. It should expose an approval callback contract:

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
    pub source: TransformSource,
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

### `TransformReport`

Add shell-expansion reporting:

```rust
pub struct TransformReport {
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
    pub warnings: Vec<TransformWarning>,
}
```

`shell_expansions_applied` counts successful executions, including directives removed because output was empty.

`shell_approvals_used` counts approvals granted during the current transform invocation.

## Internal Runtime Changes

Shell approvals and "allow once" decisions must survive recursive transforms triggered by Stage 2 includes.

The current internal API only threads `TransclusionRuntime` through recursion. That is not enough for shell expansion.

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
    pub approvals_used: usize,
}
```

`run_transform_pipeline()` creates `PipelineRuntime` once at the root and passes `&mut PipelineRuntime` into recursive child transforms.

This ensures:

1. "allow once" really means once per compose run, not once per document
2. persisted whitelist or blacklist updates are visible immediately to later directives
3. recursive includes do not repeatedly re-read policy files

## Module Layout

Recommended module layout:

```txt
darkmatter/lib/src/markdown/transform/shell_expansion/
├── mod.rs         # stage entry point and orchestration
├── parser.rs      # ::shell directive scanning
├── tokenize.rs    # shell-like argv tokenizer
├── policy.rs      # blacklist matching and whitelist checks
├── store.rs       # policy file load/append helpers
├── executor.rs    # process spawning, timeout, output capture
└── types.rs       # directives, rules, runtime, and errors
```

`transform/mod.rs` gets a new `run_shell_expansion_stage(...)` call between TOC linking and cleanup.

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
::shell bash -lc 'printf "hello\n"'
```

Directive rules:

1. the directive must occupy its own logical line
2. leading and trailing whitespace on the directive line are ignored
3. directives inside fenced code blocks or inline code are ignored
4. there are no `key=value` options in v1

### Parsing strategy

Reuse the same line-scanning shape already used for transclusion:

1. build excluded code regions with `parse_utils::find_code_regions()`
2. scan line by line
3. when a trimmed line starts with `::shell`, capture the remainder of the line as raw command text
4. tokenize the raw command text into executable and args

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

### Tokenizer behavior

The tokenizer should support:

1. unquoted tokens
2. single-quoted strings
3. double-quoted strings
4. backslash escaping for spaces, quotes, and backslashes

The tokenizer should reject:

1. unterminated quotes
2. empty command lines
3. unsupported raw shell metacharacters outside quotes:
   - `>`
   - `>>`
   - `<`
   - `|`
   - `;`
   - `&&`
   - `||`
   - `` ` ``
   - `$(`

This is intentionally stricter than the blacklist list in the spec. Because Darkmatter is not invoking a shell interpreter, those tokens do not need to be supported, and rejecting them closes off a large category of risky behavior.

## Validation Model

Validation order for each directive:

1. parse directive and tokenize argv
2. confirm the executable exists on the host
3. reject built-in blacklist matches
4. reject user blacklist matches
5. allow if exact whitelist match exists
6. allow if "command with any parameters" whitelist match exists
7. allow if current runtime contains an `AllowOnce` approval for this exact signature
8. otherwise request approval or return `ApprovalRequired`

This order matters:

1. blacklisted commands never become approvable
2. approval is only requested for commands that are syntactically valid and not globally denied
3. runtime approvals do not bypass built-in or persisted blacklists

## Command Existence Resolution

Use the `which` crate to resolve the executable before running it.

Behavior:

1. resolution uses the current process `PATH`
2. only the executable token is resolved
3. arguments are not interpreted or transformed

If resolution fails, return a hard error matching the spec:

`ERROR: the shell command '{command}' does not exist on this host ...`

This should use the executable token, not the full raw line, because the spec error references "the shell command '{command}'".

## Blacklist Design

### Built-in blacklist

The spec's blacklist becomes code-defined rules compiled into the library.

Recommended rule types:

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

1. `rm` -> `Executable("rm")`
2. `mkfs*` -> `ExecutablePrefix("mkfs")`
3. `git reset` -> `SubcommandPrefix { executable: "git", prefix: &["reset"] }`
4. `find*-delete` -> `ArgExact { executable: "find", arg: "-delete" }`
5. `redis-cli FLUSH*` -> `ArgPrefix { executable: "redis-cli", arg_prefix: "FLUSH" }`
6. `* >*` -> `RawToken(">")`

Matching rules should operate on the executable basename and argv tokens, not on the resolved absolute path. That keeps policy stable across hosts.

### User blacklist

The approval flow includes "blacklist", which means the implementation needs persisted user-controlled deny rules in addition to the built-in blacklist.

Add a sibling policy file:

- repo mode: `<repo root>/.shell-blacklist`
- home mode: `${HOME}/.shell-blacklist`

User blacklist entries are checked after the built-in blacklist and before whitelist checks.

## Whitelist and Blacklist Storage

### Location resolution

Resolve policy files this way:

1. start from `options.shell.policy_dir` if set
2. otherwise use `std::env::current_dir()`
3. if that path is inside a git repo, use the repo root
4. otherwise use `${HOME}`

Policy file paths:

- whitelist: `.shell-whitelist`
- blacklist: `.shell-blacklist`

This follows the functional spec for whitelist resolution and adds the minimal extra file needed to support runtime blacklisting.

### File format

Use a simple line-oriented format:

```text
# comments are allowed
exact git status --short
exact rg "TODO|FIXME" darkmatter/lib/src
prefix git
prefix rg
```

Rules:

1. blank lines are ignored
2. `#` starts a comment line
3. `exact <command line>` matches the normalized executable and full argv
4. `prefix <command>` matches the executable with any argv tail
5. the same tokenizer used for `::shell` directives parses policy file entries

Why this format:

1. it is human-editable
2. it matches the approval choices in the spec
3. it avoids inventing a larger config schema for a small feature

### Persistence strategy

Persist approvals and user-blacklist entries by appending a single normalized line.

Append-only is good enough for v1 because:

1. it avoids complex read-modify-write logic
2. duplicate entries are harmless and can be deduplicated in memory after loading
3. it keeps policy updates simple from the CLI approval flow

Normalization rules:

1. store the executable basename, not the resolved absolute path
2. serialize args with stable quoting only when needed
3. always write a trailing newline

## Approval Flow

### Library behavior

If a command is neither blacklisted nor whitelisted:

1. if `approval_handler` is present, call it
2. otherwise return `ShellExpansionError::ApprovalRequired`

The library never prompts directly.

### Decision handling

`AllowExactPersist`

1. append `exact <normalized command line>` to `.shell-whitelist`
2. reload or update in-memory whitelist
3. execute the command

`AllowCommandPersist`

1. append `prefix <executable>` to `.shell-whitelist`
2. reload or update in-memory whitelist
3. execute the command

`AllowOnce`

1. insert the exact normalized signature into `runtime.allow_once`
2. increment `report.shell_approvals_used`
3. execute the command

`Deny`

1. return a hard error
2. do not modify policy files

`BlacklistPersist`

1. append `exact <normalized command line>` to `.shell-blacklist`
2. update in-memory user blacklist
3. return a hard error

### CLI behavior

The `compose` CLI should install an approval handler only when it can safely prompt.

Prompting conditions:

1. input is a file path, not stdin
2. stdin is a terminal
3. stderr is a terminal

If those conditions are not met, unapproved commands should fail with an error that includes:

1. the command
2. the whitelist file path
3. the exact `exact ...` and `prefix ...` entries the user can add manually

This is important for `md compose -` and pipe-driven workflows, where stdin is already consumed by document input.

## Execution Model

### Working directory

Use this resolution order:

1. `options.shell.working_directory`, if set
2. `options.shell.policy_dir`, if set
3. `std::env::current_dir()`

This keeps command execution and policy lookup anchored to the same environment by default.

### Process spawning

Execute approved commands with `std::process::Command`:

```rust
Command::new(resolved_executable)
    .args(&directive.args)
    .current_dir(working_dir)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
```

Notes:

1. stdin is always null to avoid hanging on interactive commands
2. this prevents shell-expansion directives from silently blocking on user input
3. commands that require stdin are therefore unsupported in v1

### Timeout handling

Default timeout is 10 seconds.

Implementation shape:

1. spawn the child
2. drain stdout and stderr concurrently on background threads
3. poll `try_wait()` until completion or timeout
4. on timeout, kill the child and wait for cleanup
5. return a hard timeout error

### Output capture

Capture:

1. stdout text
2. stderr text
3. a combined output buffer in chunk-arrival order

The combined buffer is what replaces the directive in the markdown document. Separate stdout/stderr buffers are kept for error reporting.

All captured bytes should be decoded with `String::from_utf8_lossy()`.

Replacement behavior:

1. exit code `0` and both streams empty -> replace with empty string
2. exit code `0` and any output present -> replace with combined output verbatim
3. non-zero exit code -> error and do not mutate the document

The output is inserted literally. No fenced code block, trimming, or newline normalization is applied.

## Error Model

Introduce a dedicated shell-expansion error type:

```rust
pub enum ShellExpansionError {
    ParseDirective { line: usize, message: String },
    CommandNotFound { command: String, line: usize },
    Blacklisted { command: String, reason: String, line: usize },
    ApprovalRequired { command: String, whitelist_path: PathBuf, blacklist_path: PathBuf, line: usize },
    Denied { command: String, line: usize },
    Timeout { command: String, timeout: Duration, line: usize },
    ExecutionFailed { command: String, code: i32, stdout: String, stderr: String, line: usize },
    PolicyIo { path: PathBuf, source: std::io::Error },
}
```

These errors should be wrapped into `MarkdownError::Transform(...)` the same way other stage failures are surfaced today.

Important behavior:

Shell-expansion errors are always hard errors. They should not degrade to warnings when `fail_fast = false`.

Reason:

1. the functional spec explicitly says the pipeline exits in error
2. partial output after a denied or failed command is difficult to reason about
3. silent fallback would weaken the security model

## Pipeline Algorithm

For each document:

1. parse all shell directives outside code regions
2. if none exist, return early
3. for each directive in source order:
   - validate executable existence
   - validate blacklist rules
   - check user blacklist
   - check whitelist and runtime approvals
   - request approval if needed
   - execute the command
   - record replacement text
4. apply replacements from end to start to preserve byte offsets
5. increment `report.shell_expansions_applied`

Pseudo-flow:

```rust
fn run_shell_expansion_stage(
    &mut self,
    options: &TransformOptions,
    runtime: &mut PipelineRuntime,
    report: &mut TransformReport,
) -> MarkdownResult<()> {
    let directives = shell_expansion::parse_directives(&self.content)?;
    if directives.is_empty() {
        return Ok(());
    }

    let policy_paths = shell_expansion::resolve_policy_paths(&options.shell)?;
    runtime.shell.ensure_loaded(&policy_paths)?;

    let mut replacements = Vec::new();

    for directive in directives {
        let replacement = shell_expansion::execute_directive(
            &directive,
            &options,
            &policy_paths,
            &mut runtime.shell,
        )?;
        replacements.push((directive.span.clone(), replacement));
        report.shell_expansions_applied += 1;
    }

    apply_replacements_in_reverse(&mut self.content, replacements);
    report.shell_approvals_used += runtime.shell.take_recent_approval_count();
    Ok(())
}
```

## CLI Integration

`darkmatter/cli/src/commands.rs::run_compose()` should:

1. continue building `TransformOptions` as it does now
2. attach a `CliShellApprovalHandler` when prompting is possible
3. otherwise leave `approval_handler` unset

Prompt shape:

```text
Unapproved shell command in docs/example.md:12
  git status --short

1. allow exact and save
2. allow command and save
3. allow once
4. deny
5. blacklist and stop
```

The CLI should prompt on stderr and read from stdin only when stdin is not being used as document input.

## Dependency Notes

Implementation will likely add one new dependency:

- `which` for reliable executable lookup

The rest can be implemented with the standard library.

If this dependency is added, update `docs/dependencies.md` in the same change.

## Testing Strategy

### Unit tests

Parser and tokenizer:

1. parses simple commands
2. parses quoted args
3. parses escaped spaces and quotes
4. rejects unterminated quotes
5. rejects directives inside fenced code blocks
6. rejects unsupported metacharacters

Blacklist matching:

1. `rm`
2. `find . -delete`
3. `git reset --hard`
4. `docker system prune -af`
5. `psql -c "DROP TABLE x"`
6. raw `>` and `>>`

Policy store:

1. loads `exact` and `prefix` entries
2. ignores comments and blank lines
3. appends normalized entries
4. deduplicates in memory
5. resolves repo-root versus home policy location

Approval runtime:

1. `AllowOnce` applies only to exact signature
2. `AllowCommandPersist` matches future argv variants
3. blacklisted commands never reach approval

Executor:

1. stdout-only success
2. stderr-only success
3. mixed stdout and stderr success
4. empty success removes directive
5. non-zero exit returns error with captured streams
6. timeout kills child and errors

### Integration tests

Pipeline behavior:

1. interpolation runs before shell expansion
2. cleanup runs after shell expansion
3. directives in transcluded child docs also run shell expansion
4. "allow once" works across recursive includes within one compose run
5. unapproved commands fail in non-interactive CLI mode with manual-whitelist guidance

CLI behavior:

1. `md compose file.md` can prompt
2. `md compose -` cannot prompt and errors cleanly
3. persisted approvals are reused on later runs

## Implementation Phases

### Phase 1

1. add stage toggle, options, report fields, and error type
2. implement parser and tokenizer
3. implement built-in blacklist and executable lookup
4. implement whitelist and user-blacklist storage

### Phase 2

1. implement executor with timeout and output capture
2. wire stage into `transform/mod.rs`
3. add internal `PipelineRuntime`
4. add tests for pipeline integration

### Phase 3

1. add CLI approval handler
2. add CLI integration tests
3. update docs for policy file format and interactive behavior

## Open Questions

1. Should `bash -lc '...'` itself be allowed when approved, or should v1 blacklist `bash -lc` and similar "shell inside shell expansion" patterns? The secure default is to blacklist them, even though direct execution would technically allow them.
2. Should combined replacement output preserve stdout/stderr interleaving exactly, or is stdout-followed-by-stderr sufficient? The design above prefers best-effort interleaving.
3. Should future versions allow document-relative working directories instead of process cwd? The functional spec points at cwd-based behavior today, so v1 keeps cwd semantics.

## Recommendation

Ship v1 with direct command execution, strict tokenizer rejection of shell syntax, append-only whitelist and blacklist files, and an approval callback owned by the caller.

That gives Darkmatter a usable `::shell` feature without turning the markdown pipeline into a general-purpose shell interpreter.
