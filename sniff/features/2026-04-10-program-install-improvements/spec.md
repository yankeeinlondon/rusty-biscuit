# Program Install Improvements

## Summary

Extend Sniff's program-installation surface so callers can reason about _how_ a
program would be installed on the current host before anything is executed.
Today the library hides selection inside `CategoryDetector::install()` via the
private `select_best_method()` helper, giving callers no way to inspect,
override, or explain the choice. This feature introduces a first-class
"install plan" that evaluates every known method against the host's actual
capabilities, records a reason for every acceptance and rejection, and lets
both library callers and the Sniff CLI present that reasoning to the user.

## Goals

1. Expose the full set of installation methods the library knows about for a
   given program.
2. Expose the subset of methods that are _runnable_ on this host right now.
3. Produce an ordered, reasoned `InstallPlan` that applies the business-logic
   preference rules described below and can be executed.
4. Update the Sniff CLI (`sniff <category> install …`) so that success and
   failure messaging are aware of the plan, including verbose reasoning, sudo
   warnings, and a website fallback when nothing is runnable.

## Non-Goals

- Changing the `InstallationMethod` variant set. The enum in
  `sniff/lib/src/programs/types.rs` is treated as the existing vocabulary. A
  future rename is noted under _Open Questions_ but is out of scope.
- Adding new package manager command-line adapters in
  `sniff/lib/src/programs/installer.rs::build_install_command`.
- Changing the `inquire::MultiSelect` interactive picker flow used by
  `sniff/cli/src/install.rs::interactive_install_*`. That path continues to
  work; it will call through the new plan API internally.

## Library: Three Capability Tiers

The library grows three cooperating capabilities on every program category.
All three should be reachable via `ProgramDetector` / `CategoryDetector<E>`
trait methods so they work uniformly across editors, utilities, AI clients,
etc.

### Tier 1 — Known methods (already present)

Every `ProgramMetadata` already carries
`installation_methods: &'static [InstallationMethod]`. The new API simply
exposes this as a trait method so callers don't have to hand-roll
`program.info().installation_methods`.

```rust
fn known_methods(&self, program: Self::Program) -> &'static [InstallationMethod];
```

### Tier 2 — Available methods (filtered by host)

Returns the subset of the known methods whose required package manager is
actually installed on the host _and_ whose program is permitted on the host's
OS. This is the public, per-method version of the currently-private
`installer::method_available()` helper.

```rust
fn available_methods(&self, program: Self::Program) -> Vec<InstallationMethod>;
```

Filters applied:

- `ProgramMetadata::os_availability` must either be empty (any OS) or contain
  the detected OS type (`crate::os::detect_os_type()`).
- `method.manager_binary()` must correspond to an installed OS or language
  package manager, using `InstalledOsPackageManagers::is_installed()` /
  `InstalledLanguagePackageManagers::is_installed()`.
- `RemoteBash` is _not_ automatically excluded here. Tier 2 answers the
  question "could this run on this host?", not "is it wise to choose it?".
  Remote bash is downgraded in Tier 3 via a policy flag, not by existence.

### Tier 3 — Install plan

A full evaluation of every known method against the host, producing per-method
decisions and a single chosen method when one qualifies.

```rust
fn install_plan(&self, program: Self::Program) -> InstallPlan;
```

And a convenience free function for callers already holding a metadata slice:

```rust
pub fn build_install_plan(
    program: &dyn ProgramMetadata,
    host: &HostCapabilities,
) -> InstallPlan;
```

## Library: New Types

### `InstallPlan`

```rust
#[derive(Debug, Clone, Serialize)]
pub struct InstallPlan {
    /// Display name of the program this plan was built for.
    pub program: String,
    /// Official website URL from ProgramMetadata. Used by the CLI's
    /// "we can't install it but here's where to get it yourself" fallback.
    pub website: &'static str,
    /// True when at least one option was chosen.
    pub successful: bool,
    /// Every method the library considered, in evaluation order.
    /// The chosen option (if any) has `choose: true`.
    pub options: Vec<InstallPlanOption>,
}

impl InstallPlan {
    /// Every declared installation method for this program, ignoring host
    /// constraints. Equivalent to the program's static metadata.
    pub fn known_installations(&self) -> Vec<&InstallationMethod>;

    /// Every option that was evaluated and NOT chosen, with the reason.
    /// Used by the CLI's verbose output and failure reporting.
    pub fn failed_with_reason(&self) -> Vec<&InstallPlanOption>;

    /// The single chosen option, if any.
    pub fn chosen(&self) -> Option<&InstallPlanOption>;

    /// Execute the chosen option. Errors if `successful` is false.
    pub fn execute(
        &self,
        opts: &InstallOptions,
    ) -> Result<InstallResult, SniffInstallationError>;
}
```

### `InstallPlanOption`

```rust
#[derive(Debug, Clone, Serialize)]
pub struct InstallPlanOption {
    /// The underlying installation method.
    pub kind: InstallationMethod,
    /// True if executing this method will shell out through `sudo`.
    pub requires_sudo: bool,
    /// True if this option is the chosen method for the plan.
    pub choose: bool,
    /// Machine-readable reason this option was (or was not) chosen.
    pub reason_type: InstallPlanReason,
    /// Human-readable explanation, suitable for end-user display.
    pub reason: String,
}
```

### `InstallPlanReason`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallPlanReason {
    /// This option was chosen.
    Selected,

    /// A valid alternative existed and was chosen instead.
    LowerPriorityAlternative,

    /// The program's `os_availability` excludes the detected host OS.
    NoOsSupport,

    /// The package manager required by this method is not installed.
    ManagerNotInstalled,

    /// The method requires sudo and the current user cannot sudo
    /// (or --no-sudo was passed).
    RequiresSudoNotAvailable,

    /// A language PM is installed but we have no evidence the user
    /// uses it (e.g. pnpm has no global packages), so we don't trust
    /// it as a default choice.
    RequiresUnverifiedLangManager,

    /// A RemoteBash method was skipped because remote-bash installs
    /// are opt-in (policy gate).
    RemoteBashNotAllowed,

    /// Catch-all for unexpected skip reasons.
    Unknown,
}
```

### `HostCapabilities`

A new struct gathers the host-side signals the plan needs. It lives in a new
module `sniff/lib/src/programs/host_capability.rs` (or extends
`installer.rs`). It is the only place that performs live host probes for
plan-building, and it is cacheable so a caller can build many plans cheaply.

```rust
#[derive(Debug, Clone)]
pub struct HostCapabilities {
    /// Detected OS type — drives OS package manager defaulting.
    pub os_type: OsType,
    /// Installed OS package managers.
    pub os_pkg_mgrs: InstalledOsPackageManagers,
    /// Installed language package managers.
    pub lang_pkg_mgrs: InstalledLanguagePackageManagers,
    /// True if the current user can elevate via sudo without a password
    /// or is a member of a privileged group (wheel/admin/sudo).
    pub can_sudo: bool,
    /// The OS package manager considered default for this OS, if any.
    /// - Debian/Ubuntu → Apt
    /// - Fedora/RHEL → Dnf
    /// - Arch → Pacman
    /// - macOS → Brew
    /// - Windows → Winget
    pub default_os_package_manager: Option<OsPackageManager>,
    /// Language package managers for which we found at least one
    /// globally-installed package — a signal that the user already
    /// uses that manager and is comfortable with it.
    pub verified_lang_pkg_mgrs: BTreeSet<LanguagePackageManager>,
}

impl HostCapabilities {
    /// Detects all fields. Cheap fields are eager; `verified_lang_pkg_mgrs`
    /// is expensive (runs each manager's global-list command) and MAY be
    /// behind a `detect_with_verification()` constructor.
    pub fn detect() -> Self;
    pub fn detect_with_verification() -> Self;
}
```

#### Sudo detection

`can_sudo` is derived without prompting the user:

1. On Unix: user is in `wheel`, `sudo`, or `admin` group (check `getgrouplist`
   or parse `id -Gn`).
2. On Unix: `sudo -n true` returns zero (passwordless sudo cached or
   configured). This is a probe, not an escalation — it does not run anything
   privileged.
3. On Windows: `can_sudo` is `false`. Windows package managers in the current
   `build_install_command` do not use sudo, so this is a non-issue.

The first positive signal wins. If none match, `can_sudo = false`.

#### Default OS package manager

Mapped from `OsType`. The mapping lives alongside `OsType` or on a new
`OsType::default_package_manager()` method. If an OS has no mapping, the
field is `None` and the plan falls through to the alternative-OS-PM tier.

#### Verified language package manager

For each installed language PM, run its "list globals" command with a short
timeout and parse the result for at least one entry. Commands:

| Manager | Probe                         |
|---------|-------------------------------|
| npm     | `npm ls -g --depth=0 --json`  |
| pnpm    | `pnpm ls -g --depth=0 --json` |
| yarn    | `yarn global list --json`     |
| bun     | `bun pm ls -g`                |
| cargo   | `cargo install --list`        |

These probes only run under `detect_with_verification()` so the cheap path
stays cheap. A caller that wants the "comfortable with pnpm" rule must opt in.

## Library: Selection Algorithm

`build_install_plan()` evaluates every `InstallationMethod` in the program's
`installation_methods` slice and, for each, produces an `InstallPlanOption`.
At most one option has `choose: true`.

Evaluation rules, in order. The first rule that yields a _chosen_ option wins;
subsequent matching methods become `LowerPriorityAlternative`.

1. **Default OS package manager with a declared method.** If the program has
   a method whose manager matches `HostCapabilities::default_os_package_manager`
   and that manager is installed, choose it. `requires_sudo` is derived from
   the installer's command table (apt/nala/dnf/pacman → true; brew/winget/
   choco/scoop/nix → false). If the method needs sudo and
   `HostCapabilities::can_sudo` is false, skip with
   `RequiresSudoNotAvailable`.

2. **Verified pnpm global.** Method is `Pnpm(_)`, pnpm is installed, the
   command does not require sudo, and pnpm is in
   `verified_lang_pkg_mgrs` (i.e. user already has at least one pnpm global).
   This is the "comfortable with this approach" gate from the feature
   description. If pnpm is installed but unverified, record
   `RequiresUnverifiedLangManager`.

3. **User-writable npm global.** Method is `Npm(_)`, npm is installed, and
   installing globally does not require sudo. The sudo check for `npm -g` is
   currently implicit — `build_install_command` does not prepend sudo — so
   this tier assumes `requires_sudo = false`. If a future implementation
   detects a system-owned npm prefix and starts requiring sudo, it falls
   through to rule 7.

4. **Alternative installed OS package manager.** An OS package manager
   method whose manager is installed but is _not_
   `default_os_package_manager` (e.g. Brew on Linux, Nix anywhere, Scoop on
   Windows). Same sudo handling as rule 1.

5. **RemoteBash.** Only chosen when the caller has explicitly allowed remote
   bash via `InstallOptions::allow_remote_bash` (new field, default
   `false`). Otherwise recorded as `RemoteBashNotAllowed`.

6. **Cargo.** Method is `Cargo(_)` and cargo is installed. Always
   `requires_sudo = false`.

7. **Sudo-gated npm global fallback.** Method is `Npm(_)`, npm is installed,
   the prefix is not user-writable, and `can_sudo` is true. The plan
   records `requires_sudo = true` and the CLI surfaces a warning.

8. Any method not consumed by the rules above is recorded with the best
   reason available (`NoOsSupport`, `ManagerNotInstalled`, etc.).

The exact rule numbers above will be encoded as a priority list the selector
walks — not as hard-coded `if` chains — so adding a tier later (e.g. "yarn
global if verified") is localized.

## Library: Integration with Existing API

`CategoryDetector::install()` is preserved and becomes a thin wrapper:

```rust
fn install(&self, program: E) -> Result<(), SniffInstallationError> {
    let plan = self.install_plan(program);
    if !plan.successful {
        return Err(/* map plan.options into a helpful error */);
    }
    plan.execute(&InstallOptions::default())?;
    Ok(())
}
```

This keeps existing CLI paths working unchanged while letting new callers use
`install_plan()` directly.

`installer::method_available` and `installer::select_best_method` remain
`pub(crate)` helpers for the selector; they are no longer the primary API.

## CLI: Updated `install` Behavior

The Sniff CLI commands under `sniff <category> install …` and
`sniff programs install …` are updated so that output reflects the plan. The
CLI does _not_ encode selection logic — it only renders what the library
returns. All styled strings below are
[biscuit-terminal](../../../biscuit-terminal/lib) `Prose` markup.

### Normal mode — success, no sudo

```
The <blue>{program}</blue> will be installed using the <b>{installation-method}</b>.
```

Followed by a confirmation prompt (unless `--yes`) and then execution.

### Normal mode — success, requires sudo

```
The <blue>{program}</blue> is installable using <b>{installation-method}</b> but it requires root privileges so we will include the use of <yellow>sudo</yellow> so this installation method will succeed.
```

Then the same confirm-and-execute path.

### Verbose mode — success

In `--verbose`, each entry from `plan.failed_with_reason()` is rendered above
the success line as `Status::from_prose("{reason}").state(INFO)` in the order
the library evaluated them. After the skipped options the normal (or
sudo-warning) success line is printed.

### Normal or verbose — failure (no runnable method)

When `plan.successful == false`:

```
We know how to install the <blue>{program}</blue> program via the following methods but none are available to you for the stated reasons:

    - {installation-method} (reason: <i><dim><red>{reason}</red></dim></i>)
    - {installation-method} (reason: <i><dim><red>{reason}</red></dim></i>)

While we weren't able to do this for you, it's likely that you can install it yourself by going to their website: <a href="{website}">{website}</a>
```

The list is `plan.options` in order (all of them are skipped, since nothing
was chosen). The website comes from `plan.website` and is rendered as a
terminal hyperlink via `biscuit-terminal`.

### New CLI flags

Applies to every `sniff <category> install …` command:

- `--dry-run` — build the plan and print what would happen; do not execute.
- `--yes` / `-y` — skip the confirmation prompt.
- `--via <manager>` — force a specific method by manager name (e.g. `brew`,
  `cargo`, `pnpm`). The chosen method must be present in
  `plan.options`; otherwise the CLI errors with the list of valid manager
  names from `plan.known_installations()`.
- `--allow-remote-bash` — sets `InstallOptions::allow_remote_bash = true` so
  rule 5 can fire.
- `--no-sudo` — forces `HostCapabilities::can_sudo = false` for this
  invocation, even if the user could sudo.

### New CLI command: `install-plan`

A read-only command on every program category that prints the plan without
executing anything:

```
sniff editors install-plan vim
sniff programs install-plan ripgrep
sniff agents install-plan claude
```

Output is the same rendering the `install` command would emit in `--verbose`
mode, minus the confirmation prompt and execution. `--json` serializes the
full `InstallPlan` struct.

## Testing

### Library

- `build_install_plan` unit tests per rule, using a fabricated
  `HostCapabilities` (all fields injectable) and a small set of fixture
  programs that exercise each `InstallationMethod` variant category.
- Reason mapping: every `InstallPlanReason` variant has a dedicated test
  asserting it shows up for the expected shape of inputs.
- `install_plan().execute()` end-to-end with dry-run `InstallOptions` so no
  commands actually run.
- Backwards compatibility: a test verifying that
  `CategoryDetector::install()` still returns `Ok(())` on a host where a
  method is available, using dry-run.
- `HostCapabilities::detect_with_verification` tests are gated so they don't
  require a live host environment; sudo detection is unit-tested by mocking
  the group-membership and `sudo -n` probe points behind an injectable trait.

### CLI

- `sniff <category> install-plan <name> --json` asserts the plan shape via
  `assert_cmd` + `predicates` against a fixture host.
- Snapshot-style tests of the three messaging branches (success, success
  with sudo, failure) using a fake plan injected via a test-only detector.
- `--via <manager>` happy path and the error-when-method-absent case.

## Backwards Compatibility

All existing public API is preserved:

- `InstallationMethod` variant set is unchanged.
- `CategoryDetector::{installed, installable, install, install_version}`
  keep their current signatures.
- `sniff <category> install <name>` still installs. Its output format
  changes (now plan-aware), and new flags are strictly additive.

## Open Questions

1. **`InstallationMethod` rename.** The feature description notes the name
   might not be ideal. Candidates: `InstallMethod`, `InstallChannel`,
   `InstallRoute`. Deferred; the spec uses `InstallationMethod` to match the
   existing enum.

    **ANSWER:** The name is fine but I suspected that an enum might already exist which enumerated the various installation methods we use. If not, we should enumerate that.

2. **Default behavior for `RemoteBash`.** The spec makes it opt-in via
   `--allow-remote-bash`. Alternative: include it in the plan as a
   selectable option but gate only the execution. Needs a decision before
   implementation.

    **ANSWER:** remove the `--allow-remote-bash` but since we're now describing the options we can let the user know and we should we make sure that we handle CTRL+C gracefully so they can choose to exit if that option is chosen for them. Actually, I guess if that option is being provided, we should explicitly confirm with the user that this is ok before proceeding with this option.

3. **Caching `HostCapabilities`.** The cheap constructor is cheap enough to
   call per install. The verified variant is not — it shells out to every
   installed language PM. Should the CLI memoize a single
   `HostCapabilities::detect_with_verification()` per process, or only when
   a rule actually needs the verification bit (rule 2, pnpm)?

    **ANSWER:** we should cache these results into a `~/.sniff-programs.json` file and invalidate the cache every 3 months because the things we're checking against on a host RARELY change. This means we should add `--force`/`-f` flag for forced cache invalidation too.

4. **Windows sudo model.** The current `build_install_command` never
   prepends sudo on Windows, so `requires_sudo` is always false there.
   Should a future elevated-install path (e.g. winget elevation prompts)
   be modeled as `requires_sudo = true` semantically, or as a new
   `requires_elevation` field?

    **ANSWER:**
    - i'm actually not that sure what happens when `sudo` is used in WSL but I'd have expected it to work. If it does then we should allow it in a WSL environment
    - in a non-WSL Windows environment then I think we SHOULD implement a strategy for `winget` elevation. I think this is best done as proxy for `requires_sudo` in this situation rather than a new field

5. **`--via` manager naming.** The match key is `manager_name()`
   (e.g. `brew`, `npm`). If a program has both `Pnpm("foo")` and
   `Yarn("foo")`, `--via` is enough. If a program ever has two methods
   using the same manager, we need a secondary key. Not a current problem
   but worth flagging.

    **ANSWER:** 

    Ignore for now. Forbid.

6. **Error type.** When `install()` delegates to a failed plan, we need a
   new `SniffInstallationError` variant (e.g. `NoViableMethod { pkg,
   options }`) so the CLI can render the full failure message from a
   single error value. The variant is added as part of this feature.

   **ANSWER:** sounds right

## Files Expected to Change

- `sniff/lib/src/programs/types.rs` — new types (`InstallPlan`,
  `InstallPlanOption`, `InstallPlanReason`), new trait methods.
- `sniff/lib/src/programs/installer.rs` — plan builder, rule tables,
  promoted-to-pub helpers, and `InstallOptions::allow_remote_bash`.
- `sniff/lib/src/programs/host_capability.rs` — new module for
  `HostCapabilities`.
- `sniff/lib/src/programs/mod.rs` — re-exports for the new public types.
- `sniff/lib/src/os/` — `OsType::default_package_manager()` helper.
- `sniff/lib/src/error.rs` — new `NoViableMethod` variant.
- `sniff/cli/src/args.rs` — new `install-plan` subcommand and install flags.
- `sniff/cli/src/install.rs` — plan-aware output rendering, verbose mode,
  failure block, website fallback.
- `sniff/cli/src/commands.rs` — dispatch the new subcommand.
- Tests across `sniff/lib/tests/` and `sniff/cli/tests/`.
