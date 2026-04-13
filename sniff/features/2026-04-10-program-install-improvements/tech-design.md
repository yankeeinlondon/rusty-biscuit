# Program Install Improvements Technical Design

Date: 2026-04-10

This document is complementary to [`spec.md`](./spec.md). The spec defines the product behavior. This design fixes the implementation shape, the module boundaries, the cache model, and a few spec ambiguities that matter once we touch real code.

## Design Intent

The current install pipeline in `sniff` has three problems:

1. Selection logic is private and lossy. `ProgramDetector::installable()` and `ProgramDetector::install()` both rebuild host state and collapse the result to `bool` or one chosen method.
2. The library does not have a host-capability model. Manager presence, OS support, sudo availability, and language-manager trust are all inferred ad hoc.
3. The CLI has no stable read-only install surface. It can install, but it cannot explain why a method was chosen, why others were rejected, or what the fallback is when nothing can run.

The feature should therefore introduce a first-class planning pipeline:

`ProgramMetadata` -> `HostCapabilities` -> `InstallPlan` -> optional CLI override -> execution

The CLI remains a renderer and executor. The library remains the source of truth for install reasoning.

## Spec Alignment Deltas

The spec is directionally correct, but these details should be treated as the implementation contract:

### 1. Remote bash is selected by the plan, not gated out of the plan

The open-question answer rejects `--allow-remote-bash`. The resulting behavior should be:

- `InstallPlan` may choose `RemoteBash` when it is the best runnable method.
- The CLI must require a second, explicit confirmation before executing that choice.
- `--yes` skips the ordinary install confirmation, but it must not silently approve remote-bash execution.

This means `InstallPlanReason::RemoteBashNotAllowed` should be removed from the plan-reason model. Consent is an execution-time concern, not a planning-time concern.

### 2. Host capabilities are cached on disk

The spec's "cheap constructor" is still too expensive if invoked repeatedly from the CLI, especially once verification probes exist. Host capability detection should be cached at:

`~/.sniff-programs.json`

Rules:

- TTL: 90 days (the implementation can call this "three months")
- corrupt or schema-mismatched cache: ignore and rebuild
- `--force` / `-f`: bypass cache and rewrite it

### 3. WSL and native Windows elevation are distinct

`HostCapabilities` needs to distinguish:

- Linux on native Linux
- Linux on WSL
- native Windows

WSL should behave like Linux for sudo-based methods. Native Windows should treat `winget` elevation as the Windows equivalent of `requires_sudo = true`.

### 4. The free-function signature in the spec should be generic, not `&dyn ProgramMetadata`

`ProgramMetadata` is currently declared as `trait ProgramMetadata: Sized`, so `&dyn ProgramMetadata` is not valid today. The implementation should use:

```rust
pub fn build_install_plan<P: ProgramMetadata>(
    program: &P,
    host: &HostCapabilities,
) -> InstallPlan;
```

If we later want trait-object support, that should be a separate cleanup that removes the `Sized` bound from `ProgramMetadata`.

## Current-State Constraints

The design needs to fit the current codebase:

- `ProgramMetadata` already exposes `installation_methods()` and `os_availability()` in [`sniff/lib/src/programs/schema.rs`](../../lib/src/programs/schema.rs).
- `ProgramDetector::installable()`, `install()`, and `install_version()` currently duplicate OS checks and manager detection in [`sniff/lib/src/programs/types.rs`](../../lib/src/programs/types.rs).
- method selection and command building live in [`sniff/lib/src/programs/installer.rs`](../../lib/src/programs/installer.rs).
- the CLI install path is split between direct and interactive flows in [`sniff/cli/src/install.rs`](../../cli/src/install.rs).
- `sniff programs install <name>` currently resolves by recursively attempting category-specific installs. That is not usable once the install path becomes plan-aware.

This feature should improve behavior without forcing a full rewrite of the programs subsystem.

## Module Layout

The spec suggests putting most new types into `types.rs`. That file is already large. The implementation should instead split plan code into dedicated modules and re-export them from `programs/mod.rs`.

### New library modules

- `sniff/lib/src/programs/install_plan.rs`
  - `InstallPlan`
  - `InstallPlanOption`
  - `InstallPlanReason`
  - `build_install_plan`
  - internal selection-rule helpers
- `sniff/lib/src/programs/host_capability.rs`
  - `HostCapabilities`
  - `HostCapabilityCache`
  - detection and cache-loading functions
- `sniff/lib/src/programs/install_method.rs` or additions inside `installer.rs`
  - internal helpers that normalize `InstallationMethod` into execution and availability facts

### Existing modules to extend

- `sniff/lib/src/programs/types.rs`
  - trait additions on `ProgramDetector`
  - thin wrappers on `CategoryDetector<E>`
- `sniff/lib/src/programs/installer.rs`
  - execution helpers should consume plan options, not just raw methods
  - command builders should accept privilege context
- `sniff/lib/src/programs/mod.rs`
  - public re-exports
- `sniff/lib/src/error.rs`
  - new install-planning and remote-bash-consent errors

This keeps the public API flat while containing the new logic.

## Public API Shape

### ProgramDetector additions

Add these methods to `ProgramDetector`:

```rust
fn known_methods(&self, program: Self::Program) -> &'static [InstallationMethod];

fn available_methods(&self, program: Self::Program) -> Vec<InstallationMethod>;

fn install_plan(&self, program: Self::Program) -> InstallPlan;
```

`CategoryDetector<E>` implements all three using shared helpers.

Behavior:

- `known_methods()` is just the metadata slice.
- `available_methods()` applies OS support plus host-executability checks.
- `install_plan()` builds a full `InstallPlan` from cached host capabilities.

### Existing methods rewritten as wrappers

`installable()` becomes:

```rust
fn installable(&self, program: E) -> bool {
    self.install_plan(program).successful
}
```

`install()` becomes:

```rust
fn install(&self, program: E) -> Result<(), SniffInstallationError> {
    let plan = self.install_plan(program);
    plan.execute(&InstallOptions::default())?;
    Ok(())
}
```

`install_version()` uses the same plan, then calls a versioned executor on the chosen option.

This removes duplicated host probing and guarantees that all public install paths use the same decision engine.

## HostCapabilities

`HostCapabilities` is the shared input to plan building.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostCapabilities {
    pub os_type: OsType,
    pub is_wsl: bool,
    pub has_bash: bool,
    pub os_pkg_mgrs: InstalledOsPackageManagers,
    pub lang_pkg_mgrs: InstalledLanguagePackageManagers,
    pub can_sudo: bool,
    pub default_os_package_manager: Option<OsPackageManager>,
    pub verified_lang_pkg_mgrs: BTreeSet<LanguagePackageManager>,
    pub npm_global_prefix_writable: Option<bool>,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}
```

### Why these extra fields exist

- `is_wsl`: required by the native-Windows vs WSL decision.
- `has_bash`: `RemoteBash` cannot be considered runnable unless a shell exists.
- `npm_global_prefix_writable`: needed to make rule 3 and rule 7 real instead of hypothetical.
- `detected_at`: needed for cache TTL and debugging.

### Detection entry points

```rust
impl HostCapabilities {
    pub fn load_or_detect() -> Self;
    pub fn load_or_detect_with_verification(force_refresh: bool) -> Self;
    pub fn detect() -> Self;
    pub fn detect_with_verification() -> Self;
}
```

Guidelines:

- `load_or_detect*` is what `CategoryDetector::install_plan()` should use.
- `detect*` performs live probes only.
- `with_verification` runs global-package verification probes and npm prefix checks.
- non-verification paths leave `verified_lang_pkg_mgrs` empty and `npm_global_prefix_writable` as `None`.

### Detection details

#### OS and environment

- `os_type`: existing `detect_os_type()`
- `is_wsl`: Linux host where `/proc/version`, `/proc/sys/kernel/osrelease`, or environment data indicates WSL
- `default_os_package_manager`: derived from `(os_type, distro)` rather than `OsType` alone for Linux

Implementation note: the spec maps Linux defaults by distro family, not by `OsType::Linux` itself. This logic belongs either on a helper in `os/` or inside `host_capability.rs`, but it must inspect Linux distro metadata rather than flattening all Linux hosts together.

#### Package-manager presence

- `os_pkg_mgrs`: existing `InstalledOsPackageManagers::new()`
- `lang_pkg_mgrs`: existing `InstalledLanguagePackageManagers::new()`

#### Sudo and elevation

Unix:

1. group membership probe (`wheel`, `sudo`, `admin`)
2. `sudo -n true`

Native Windows:

- `can_sudo` stays `false`
- privilege-sensitive Windows installs use a separate internal execution mode, but still surface as `requires_sudo = true` publicly

WSL:

- treat as Linux

#### Verification probes

Run only in `detect_with_verification()`:

- `npm ls -g --depth=0 --json`
- `pnpm ls -g --depth=0 --json`
- `yarn global list --json`
- `bun pm ls -g`
- `cargo install --list`
- `npm prefix -g` plus a writability check on the resulting directory

Probe rules:

- timeout: 2 seconds per probe
- run probes in parallel for installed managers
- parse failures are non-fatal and should degrade to "unverified" / `None`

## Host Capability Cache

The cache file should store a versioned envelope:

```rust
#[derive(Serialize, Deserialize)]
struct HostCapabilityCacheFile {
    schema_version: u32,
    hostname: String,
    os: OsType,
    is_wsl: bool,
    expires_at: chrono::DateTime<chrono::Utc>,
    capabilities: HostCapabilities,
}
```

Rules:

- schema version starts at `1`
- TTL is `detected_at + 90 days`
- host mismatch invalidates the cache
- write via temp file + atomic rename
- best-effort file mode `0600` on Unix
- no home directory: skip caching and do live detection

The cache belongs in the library because both the CLI and future library callers benefit from it.

## InstallPlan Model

The public structs from the spec are correct with two adjustments:

1. remove `RemoteBashNotAllowed` from `InstallPlanReason`
2. keep any execution-only state out of the serialized shape

Recommended public enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallPlanReason {
    Selected,
    LowerPriorityAlternative,
    NoOsSupport,
    ManagerNotInstalled,
    RequiresSudoNotAvailable,
    RequiresUnverifiedLangManager,
    Unknown,
}
```

Execution-only gates should instead surface through new `SniffInstallationError` variants:

- `NoViableMethod { pkg, plan }`
- `RemoteBashConsentRequired { pkg, url }`
- `InstallMethodNotForced { requested, valid }` or equivalent CLI-level error

## Plan-Building Algorithm

The plan builder should be a two-phase evaluator.

### Phase 1: derive method facts

For each declared `InstallationMethod`, compute:

- `os_supported`
- `method_family`
  - default OS manager candidate
  - alternative OS manager candidate
  - verified pnpm candidate
  - npm candidate
  - cargo candidate
  - remote-bash candidate
  - other
- `manager_installed`
- `requires_sudo`
- `lang_manager_verified`
- `eligible_without_priority`
- best blocking reason if not eligible

This phase must not choose anything. It only turns each method into a fact record.

### Phase 2: choose by ordered rule buckets

After fact derivation, walk the methods in this priority order:

1. default OS package manager
2. verified pnpm
3. npm global without sudo
4. alternative installed OS package manager
5. remote bash
6. cargo
7. npm global with sudo

Within a bucket, preserve metadata order from `installation_methods`.

The first eligible method becomes the chosen option. Any other eligible method becomes `LowerPriorityAlternative`. Ineligible methods keep their computed rejection reason.

This keeps the algorithm extensible. Adding "verified yarn global" later becomes "insert one more bucket", not "rewrite a tower of ifs".

## Availability and Execution Normalization

The existing code spreads install behavior across:

- `InstallationMethod::manager_name()`
- `InstallationMethod::manager_binary()`
- `InstallationMethod::is_os_package_manager()`
- `build_install_command()`

That is already inconsistent for some methods. For example:

- `Poetry` checks for `poetry`, but execution shells out to `pip`
- `Nuget` checks for `nuget`, but execution shells out to `dotnet`

This feature should centralize those facts behind one internal descriptor helper:

```rust
struct InstallExecutionDescriptor {
    display_manager: &'static str,
    availability_binary: &'static str,
    execution_mode: ExecutionMode,
    requires_sudo_by_default: bool,
}
```

`ExecutionMode` should distinguish:

- normal command
- Unix `sudo` prefix
- Windows elevation
- remote bash

The plan builder and executor should both consume this descriptor so that "available", "selected", and "executed" all mean the same thing.

## Remote-Bash Execution Contract

Remote-bash selection is allowed. Remote-bash execution is guarded.

### Library behavior

`InstallPlan::execute()` should:

- error with `RemoteBashConsentRequired` if the chosen option is `RemoteBash` and the options do not explicitly approve it
- allow dry-run rendering without approval

That requires one new field on `InstallOptions`:

```rust
pub struct InstallOptions {
    pub dry_run: bool,
    pub skip_confirm: bool,
    pub timeout_secs: u64,
    pub approve_remote_bash: bool,
}
```

Default is `false`.

### CLI behavior

When the chosen option is `RemoteBash(url)`:

1. print the normal success line
2. print an additional warning that a remote shell script will be downloaded and executed
3. require a dedicated `inquire::Confirm`
4. if declined or interrupted, exit cleanly without execution

This prompt is always required, even with `--yes`.

## CLI Design

### New argument model

The current clap model only supports `install { program? }`. It should grow a shared install-argument struct used by both `install` and `install-plan`.

Recommended shape:

```rust
#[derive(clap::Args, Debug, Clone)]
pub struct InstallCommandArgs {
    program: Option<String>,
    #[arg(long)]
    dry_run: bool,
    #[arg(short = 'y', long)]
    yes: bool,
    #[arg(long, value_name = "MANAGER")]
    via: Option<String>,
    #[arg(long)]
    no_sudo: bool,
    #[arg(short = 'f', long)]
    force: bool,
}
```

Then each category action becomes:

```rust
enum EditorAction {
    Install(InstallCommandArgs),
    InstallPlan { ...same args minus yes/dry_run optionality... },
}
```

Exact clap syntax can vary, but the important part is one shared source of truth for install-related flags.

### Resolver refactor

`sniff programs install <name>` should stop recursively attempting category installs. Replace that with a pure resolver:

```rust
enum ResolvedProgram {
    Editor(Editor),
    Utility(Utility),
    ...
}
```

This avoids side effects and gives both `install` and `install-plan` a deterministic path.

### Rendering contract

The CLI should have one renderer:

```rust
fn render_install_plan(
    plan: &InstallPlan,
    verbose: bool,
    plain: bool,
) -> String
```

Modes:

- `install-plan`: render only
- `install --dry-run`: render only
- `install`: render, confirm, execute, then print result

`biscuit-terminal` `Prose` should be used for all styled output so plain-mode stripping remains correct.

### Forced method override

`--via <manager>` applies after the library builds the full plan.

Flow:

1. build the plan normally
2. find all options whose `kind.manager_name()` matches the requested manager
3. if zero matches: error with valid manager names from `plan.known_installations()`
4. if more than one match: error and instruct the user that ambiguous same-manager methods are not supported
5. rebuild a derived plan with that option marked chosen only if it was otherwise runnable

Important: `--via` must not let the CLI force an option that the library said was unavailable. It can only override preference among runnable choices.

### Interrupt handling

Interactive confirmation should distinguish:

- cancel / escape: no-op exit
- ctrl-c / interrupted prompt: exit code `130`

The current install flow treats `OperationCanceled` and `OperationInterrupted` the same. The new confirmation path should not.

## JSON Shape

`install-plan --json` should serialize the full `InstallPlan`.

To keep the JSON stable, `InstallationMethod` should gain explicit serde tagging:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "manager", content = "target", rename_all = "snake_case")]
pub enum InstallationMethod { ... }
```

That yields a predictable shape for `InstallPlanOption.kind` and avoids relying on serde's default externally-tagged tuple-variant format.

## Backwards Compatibility

Public source compatibility is preserved:

- `ProgramDetector` only gains additive methods
- `CategoryDetector::{installable, install, install_version}` keep signatures
- existing `sniff <category> install <name>` commands remain valid

Behavioral changes are intentional:

- `installable()` may now be `true` when the selected route is remote bash
- install failures become more specific
- CLI output becomes plan-aware

## Testing Strategy

### Library tests

1. `HostCapabilities` unit tests
   - default OS manager mapping by distro family
   - sudo detection probe ordering
   - WSL detection
   - cache hit, cache miss, stale cache, corrupt cache
2. plan-builder rule tests
   - one test per priority bucket
   - one test per rejection reason
   - one test proving metadata order wins within a bucket
3. execution tests
   - dry-run execution of chosen option
   - remote-bash execution rejected without consent
   - remote-bash execution allowed with consent
   - versioned execution shares the same chosen option
4. wrapper tests
   - `installable()` mirrors `plan.successful`
   - `install()` maps failed plans to `NoViableMethod`

These should be table-driven and use fabricated `HostCapabilities` so they do not depend on the local machine.

### CLI tests

1. `install-plan --json`
   - assert serialized shape
2. text rendering snapshots
   - success
   - success requiring sudo
   - success via remote bash
   - failure with website fallback
3. override tests
   - `--via brew` succeeds when brew is runnable
   - `--via brew` fails when brew exists in known methods but is unavailable
   - `--via` fails on ambiguity
4. cache tests
   - `--force` bypasses a seeded cache
5. confirmation tests
   - remote-bash extra confirmation
   - ctrl-c returns exit 130

## Recommended Implementation Order

1. add `HostCapabilities` plus cache
2. add `InstallPlan` types and builder
3. switch `ProgramDetector` wrappers to the plan path
4. add remote-bash consent and richer installation errors
5. refactor CLI resolver and renderer
6. add `install-plan` subcommands and flags
7. fill in snapshots and rule tests

That order keeps each step testable and avoids rewriting the CLI before the library contract is stable.

## Risks

### Manager/execution mismatches

If the implementation keeps separate availability and execution match trees, the plan can claim a method is runnable and then fail because a different executable is actually used. This feature should treat that as a correctness bug, not as acceptable technical debt.

### Cache drift

A 90-day TTL is intentionally long. The implementation must therefore be conservative when probes fail:

- missing or unparsable verification data should reduce confidence, not inflate it
- the cache should be easy to bypass with `--force`

### CLI duplication

Do not implement the plan renderer separately for `install` and `install-plan`. One renderer plus one executor keeps the messaging branches consistent.
