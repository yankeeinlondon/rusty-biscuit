---
prompt: |-
    Research the Rust `extrasafe` crate to help a developer decide whether
    and how to use it in a Linux sandbox launcher or self-restricting Rust worker.

    The starting point is a higher-level interface to Linux seccomp, with
    Landlock and namespace-based isolation in relevant configurations. Verify
    which mechanisms are present in the published release and enabled by default.

    Focus on the published crate's public API, enforcement guarantees,
    compatibility behavior, and practical integration. Explain native OS
    concepts where needed without turning this into a general sandbox survey.

    Evidence and scope:
    - State the research date and exact stable crate version examined.
    - Verify identity through published package metadata and its linked
      repository. If the named crate cannot be identified, report the search
      and uncertainty; do not invent APIs or silently research a substitute.
    - Use primary sources: version-specific API documentation, published
      source, release notes, repository history, and official documentation
      for the underlying operating-system mechanisms. Cite claims nearby.
    - Treat the starting links and neighboring research as leads. Independently
      verify relevant claims; do not treat neighboring reports as primary evidence.
    - Distinguish released functionality from unreleased changes, proposals,
      documentation claims, and your own recommendations. Explain conflicting
      or unavailable evidence. Do not infer security from a successful launch.

    Starting links:
    - https://docs.rs/extrasafe/
    - ../os-support/landlock.md

    Cover the following topics:

    1. Identity, releases, and maintenance
       - Identify the package, repository, maintainers, license, and any parent
         project. Confirm it is published and usable as a dependency; identify
         relevant yanked or prerelease versions.
       - Verify the latest stable version and release date. Summarize releases
         from the past 24 months and older milestones needed to explain the API.
         Use dates to describe cadence rather than impressions.
       - Explain API stability, breaking changes, migration considerations,
         minimum supported Rust version, and important native dependencies.
         Do not assume major releases exist or equate pre-1.0 with unreliability.
       - Assess maintenance using observable evidence; label judgments as such.

    2. Public API and enforcement model
       - Explain SafetyContext, RuleSet, built-in rule sets, custom syscall and
         argument rules, and their actual composition semantics.
       - Trace how rules become seccomp filters and, where supported, Landlock
         rules or an isolated subprocess. Explain defaults and implicit grants.
       - Identify per-thread versus process-wide application, inheritance,
         irreversibility, privileges, and how enforcement failures are returned.
       - List Cargo features, defaults, architecture restrictions, native
         dependencies, and which isolation mechanisms each feature enables.

    3. Platform integration, compatibility, and failure behavior
       - Verify Linux kernel and architecture requirements for each enabled
         mechanism; distinguish crate support from kernel availability.
       - Explain Landlock compatibility and enforcement status, seccomp action
         availability, thread synchronization, and unsupported rule handling.
       - Investigate namespace/user-namespace availability, mount setup, and any
         restrictions on invoking isolation from a multithreaded or async host.
       - Trace partial setup failures: can seccomp succeed while Landlock fails,
         or can a caller mistake partial restrictions for complete enforcement?
       - Show how to require specific controls and refuse workload execution
         when they are absent, blocked by an outer container, or unsupported.
       - Address containers and WSL2 as Linux environments; distinguish these
         from native macOS or Windows support.

    4. Practical uses, limitations, and pitfalls
       - Describe supported uses and who they protect against. Distinguish
         accidental access reduction from confinement of malicious workloads.
       - Evaluate read-only inputs, a writable workspace, no networking, and
         access only to approved destinations. State exactly which parts the
         crate can enforce and which require another mechanism.
       - Examine enforcement ordering, multithreading, inherited descriptors or
         handles, existing sockets, descendants, executable/runtime access,
         filesystem aliases, and IPC paths where they affect claimed guarantees.
       - Classify material gaps as underlying OS limitations, native capabilities
         missing from the crate, integration/deployment constraints, or needs
         for another isolation mechanism. Explain consequences and mitigations.
       - Include documented defects or advisories material to the examined
         version, with their status and applicability. Absence of an advisory
         does not establish security.
       - Examine rule-set union, broad built-in grants, descriptor-based
         exceptions, async runtime syscalls, and ordering of irreversible steps.
       - Determine which network requirements need another mechanism; do not
         equate blocking selected socket syscalls with destination filtering.

    5. Rust examples and verification
       - Demonstrate a minimal custom syscall policy, a policy combining
         read-only inputs with a writable workspace where supported, and an
         isolated subprocess. Identify the feature needed for each example.
       - Include thread-scope and partial-enforcement checks, and show a
         refusal path for a capability the caller explicitly requires.
       - Include complete, focused Rust examples with Cargo configuration pinned
         to the examined release, required features, platform setup, and helpers.
       - Show meaningful error handling and fail-closed behavior for each
         example's stated requirements. Explain allowed and denied operations
         and identify guarantees requiring caller validation or another mechanism.
       - Compile-check when tooling permits. Label examples as compiled,
         executed, or unverified, naming the environment and toolchain tested.
         Compilation and cross-compilation do not establish enforcement.
       - Describe isolated child-process tests for allowed and denied operations,
         unavailable capabilities, and launch failures. Keep the supervisor/test
         runner unrestricted and avoid persistent host security changes; use a
         disposable environment when a probe cannot be safely cleaned up.

    6. Alternatives and complementary tools
       Compare direct seccompiler or libseccomp integration, the landlock
       crate, and a namespace launcher such as Bubblewrap. Explain what the
       higher-level rules simplify and what remains the caller's responsibility.
       Distinguish competing bindings, higher-level wrappers, complementary
       mechanisms, and stronger boundaries. Compare platform coverage,
       maintenance, integration cost, and when another layer is needed.

    Presentation:
    - Begin with a concise suitability assessment and the main caveats.
    - Use well-formed CommonMark and capability/comparison tables where helpful.
      Use Mermaid only when a diagram clarifies policy or launch flow.
    - End with authoritative resource links and unresolved questions that
      materially affect adoption.
os: [Linux]
hash: c63a5827816df7cd-a63953ea8d6a8dc1
last_updated: 2026-11-04
---

# extrasafe: suitability for Linux workers and launchers

**Research date: November 4, 2026. Examined release: `extrasafe` 0.5.1, published April 16, 2024.**

`extrasafe` is useful for reducing the syscall access of an application you control, particularly a small synchronous worker on **Linux x86_64**. Its built-in rules simplify policy construction, but they are grants, not security profiles certified for a workload. For a launcher executing hostile native code, prefer a separately supervised process and combine syscall filtering with a checked filesystem policy, namespace configuration, descriptor hygiene, and resource limits.

**The principal adoption blocker in 0.5.1 is its Landlock result handling:** it discards the result of `restrict_self()`. Thus a successful `SafetyContext::apply_*()` does **not** prove filesystem enforcement. Its filesystem policy also handles only ABI 2 rights, omitting truncation protection. The optional namespace launcher has additional limitations, including writable bind mounts, retained namespace capabilities, and an unpublished destination-path validation fix. Neither Landlock nor namespaces is enabled by default. These conclusions come from the **published source**, rather than the all-features documentation landing page. [Published filter implementation](https://docs.rs/crate/extrasafe/0.5.1/source/src/lib.rs), [feature manifest](https://docs.rs/crate/extrasafe/0.5.1/source/Cargo.toml.orig), [isolate implementation](https://docs.rs/crate/extrasafe/0.5.1/source/src/isolate/isolate_sys.rs)

## Identity, releases, and maintenance

The crates.io package is **`extrasafe`**, linked to **[`boustrophedon/extrasafe`](https://github.com/boustrophedon/extrasafe)**. The published author and current registry owner are **Harry Stern**, GitHub account `boustrophedon`. It is MIT licensed. The examined metadata identifies a standalone project, not a component of a declared parent product. It is published and usable with `extrasafe = "=0.5.1"`. Registry metadata returned nine versions, none yanked, and no prereleases; the changelog's reference to “0.1.3 and prior” is not evidence that 0.1.3 was published. [Registry metadata and version history](https://crates.io/api/v1/crates/extrasafe), [registry owners](https://crates.io/api/v1/crates/extrasafe/owners), [published package manifest](https://docs.rs/crate/extrasafe/0.5.1/source/Cargo.toml.orig)

There were **no published releases in the preceding 24 months**, November 4, 2024–November 4, 2026. Older milestones explain the current API:

| Published date, UTC | Release             | Integration significance                                                                                 |
| ------------------- | ------------------- | -------------------------------------------------------------------------------------------------------- |
| 2022-03-02          | 0.1.0, 0.1.1, 0.1.2 | Initial syscall-filtering releases.                                                                      |
| 2023-06-17          | 0.1.4               | Dependency updates, reference `RuleSet` implementation.                                                  |
| 2023-09-01          | 0.2.0               | `Rule` renamed `SeccompRule`; conditional/unconditional grant conflicts become errors.                   |
| 2023-10-21          | 0.3.0               | Backend changes from libseccomp to seccompiler; argument-filter macros/types change; `Pipes` added.      |
| 2023-11-09          | 0.4.0               | Optional Landlock, `LandlockRule`, `landlock_only()`, additional errors.                                 |
| 2024-04-04          | 0.5.0               | Optional `Isolate`; `Sysno` implements `RuleSet`; conditional rules get an empty default implementation. |
| 2024-04-16          | **0.5.1**           | Documentation/user-guide updates and IPC example changes; latest stable.                                 |

Dates are registry publication dates; descriptions are from the [published changelog](https://docs.rs/crate/extrasafe/0.5.1/source/CHANGELOG.md) and [registry version records](https://crates.io/api/v1/crates/extrasafe).

The latest default-branch commit returned by GitHub was **April 18, 2024**, `b56d18c8…`, prohibiting `.` and `..` bind-mount destinations. It follows the 0.5.1 release commit and **is not in the published archive**. Repository `pushed_at` was July 13, 2024; that field can include other refs and is not a newer default-branch release. The repository is not archived. Issues received activity in November–December 2024, including a closed report of seccomp installation returning `EINVAL`; it has no demonstrated universal fix. Open built-in expansion/workspace PRs and API proposals are not released APIs. [Commit history](https://api.github.com/repos/boustrophedon/extrasafe/commits?per_page=8), [repository metadata](https://api.github.com/repos/boustrophedon/extrasafe), [unpublished path fix](https://github.com/boustrophedon/extrasafe/pull/43), [installation failure report](https://github.com/boustrophedon/extrasafe/issues/57), [proposed built-ins](https://github.com/boustrophedon/extrasafe/pull/52)

**Assessment:** publication and implementation activity have been sparse since April 2024. That is an adoption/support risk for a security boundary, not proof that the crate is abandoned or insecure merely because it is pre-1.0. No support commitment, security audit, or maintained MSRV policy was established by the reviewed material. Under Cargo's pre-1.0 compatibility rules, a minor version increase can break APIs; pin the examined release and review upgrades. Migration from 0.2 or earlier requires reviewing the backend and argument-filter changes, not just updating a version string. [Cargo compatibility rules](https://doc.rust-lang.org/cargo/reference/semver.html), [release changelog](https://docs.rs/crate/extrasafe/0.5.1/source/CHANGELOG.md)

**Rust/native dependencies:** the package declares edition 2021 but **no `rust-version`**. Do not describe edition 2021's compiler floor as its verified MSRV. The `isolate` source uses `OsStr::as_encoded_bytes`, stabilized in Rust 1.74, so that feature needs at least that compiler API; current dependency resolution can require more. Runtime dependencies are `seccompiler ^0.4` with default features off, `syscalls ^0.6` with default features off, `libc ^0.2`, and optional `landlock ^0.3`. There is **no libseccomp C-library dependency in this release**. Linux syscall/libc facilities remain native requirements; OpenSSL, SQLite, Tokio, and HTTP crates seen in the manifest are development/example dependencies. [Manifest](https://docs.rs/crate/extrasafe/0.5.1/source/Cargo.toml), [isolate source](https://docs.rs/crate/extrasafe/0.5.1/source/src/isolate/isolate_sys.rs), [Rust API stabilization](https://doc.rust-lang.org/std/ffi/struct.OsStr.html#method.as_encoded_bytes)

## Features and deployment requirements

| Cargo configuration           | Present functionality                                                | Runtime requirements and important limits                                                                                                                                                                           |
| ----------------------------- | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Default; no optional features | Seccomp via `SafetyContext` and built-ins                            | Linux **x86_64 only**, enforced by `compile_error!`; other targets are not a no-op fallback.                                                                                                                        |
| `features = ["landlock"]`     | Path rules, access helpers, Landlock reexports and `landlock_only()` | Source requires **ABI 2** rights with `HardRequirement`; upstream Linux 5.19 introduced ABI 2. Kernel must build and enable Landlock, and permit its syscalls. Activation still depends on the configured rules.    |
| `features = ["isolate"]`      | Self-reexecuting namespace subprocess API                            | User, mount, PID, IPC, UTS, and by default network namespaces; procfs during bootstrap; tmpfs, bind mounts and `pivot_root`; executable memfd and descriptor closing. Does not automatically apply `SafetyContext`. |
| Both optional features        | Both APIs available                                                  | No automatic combination or aggregate enforcement report. Caller orders their use.                                                                                                                                  |

There is no `default` feature entry. docs.rs builds **all features**, which explains why its public documentation shows optional modules even though ordinary dependency builds do not include them. The broader architecture support of seccompiler or the Linux kernel does not override extrasafe's explicit x86_64 restriction. [Feature metadata](https://docs.rs/crate/extrasafe/0.5.1/features), [published manifest](https://docs.rs/crate/extrasafe/0.5.1/source/Cargo.toml.orig), [target check](https://docs.rs/crate/extrasafe/0.5.1/source/src/lib.rs)

Seccomp BPF arrived on x86 in Linux 3.5, but 0.5.1's seccompiler backend uses the **`seccomp()` syscall**, introduced in Linux 3.17, including TSYNC for the all-thread path. Treat 3.17 as the relevant upstream syscall baseline, with `CONFIG_SECCOMP`/`CONFIG_SECCOMP_FILTER`, rather than promising operation on every 3.5 kernel. The actual filter installation is the authoritative capability check. `Allow` and `Errno` are original filter actions; extrasafe does not offer configurable kill/log/notification actions through `SafetyContext`, nor expose an action-availability probe. Outer filters can impose stronger denials or termination independently. [Seccompiler 0.4 installation source](https://docs.rs/crate/seccompiler/0.4.0/source/src/lib.rs), [seccomp syscall/version/action contract](https://man7.org/linux/man-pages/man2/seccomp.2.html)

For `Isolate`, old user-namespace availability alone is insufficient: its unconditional use of `close_range` makes **Linux 5.9** a relevant upstream minimum; it also uses `CLONE_PIDFD` and `waitid(P_PIDFD)`, with the latter available since 5.4. Namespace support, user-namespace quotas, distribution security policy, mount propagation, and outer-container filters can still prevent operation on newer kernels. There is no negotiated fallback to an older close loop or a non-namespace execution mode. [Published namespace syscalls](https://docs.rs/crate/extrasafe/0.5.1/source/src/isolate/isolate_sys.rs), [close_range history](https://man7.org/linux/man-pages/man2/close_range.2.html), [waitid history](https://man7.org/linux/man-pages/man2/waitid.2.html), [user namespace restrictions](https://man7.org/linux/man-pages/man7/user_namespaces.7.html)

Containers and **WSL2** are Linux environments, not alternate native backends. Test the exact architecture, kernel configuration, enabled LSMs, and outer sandbox. WSL2's real Linux kernel does not establish that Landlock or user namespaces are enabled in a particular installation. Native macOS and Windows cannot use this crate directly; an x86_64 Linux VM/container backend or separate OS implementation is necessary. ARM64 Linux, including many Apple Silicon Linux guests, is also rejected by 0.5.1. [Target guard](https://docs.rs/crate/extrasafe/0.5.1/source/src/lib.rs), [Microsoft's WSL2 architecture](https://learn.microsoft.com/en-us/windows/wsl/about)

## Policy API and exact composition

`SafetyContext::new()` constructs policy; it does not restrict execution. `enable(RuleSet)` consumes the builder and returns `Result<SafetyContext, ExtraSafeError>`. Installation consumes the context through `apply_to_current_thread()` or `apply_to_all_threads()`, returning `Result<(), ExtraSafeError>`. `with_errno()` changes the default syscall rejection errno from **1 (`EPERM`)**. Use a conventional nonzero errno: configuring zero would synthesize apparent syscall success without performing the syscall. [Versioned API](https://docs.rs/extrasafe/0.5.1/extrasafe/struct.SafetyContext.html), [implementation](https://docs.rs/crate/extrasafe/0.5.1/source/src/lib.rs)

A custom `RuleSet` supplies a static `name()` and `simple_rules() -> Vec<Sysno>`. `conditional_rules()` defaults to an empty `HashMap<Sysno, Vec<SeccompRule>>`; with the feature enabled, `landlock_rules()` defaults to an empty vector. `Sysno` itself and references to rulesets implement the trait. `SeccompRule::new(syscall)` is an unconditional grant; each `.and_condition()` adds a required argument comparison. The macro `seccomp_arg_filter!` supports argument indexes 0–5, comparisons, and masked equality. It defaults to 64-bit comparisons; `SeccompArgumentFilter::new32` is available for 32-bit arguments with possible sign-extension differences. Import its helper macro as well, or use the documented glob import. [RuleSet/types](https://docs.rs/extrasafe/0.5.1/extrasafe/trait.RuleSet.html), [argument-filter source](https://docs.rs/crate/extrasafe/0.5.1/source/src/macros.rs), [comparison types](https://docs.rs/extrasafe/0.5.1/extrasafe/struct.SeccompArgumentFilter.html)

Composition is important:

- **Across enabled rulesets, grants accumulate.** They are not intersected, and there is no deny rule that overrides another grant inside the context.
- **Within one syscall rule, conditions are ANDed.** Multiple conditional rules for that syscall are **ORed**. Two allowed FDs mean either FD is permitted; adding another rule cannot narrow the first.
- Mixing a conditional grant and an unconditional grant for the same syscall returns **`ConditionalNoEffectError`** instead of silently widening it. Duplicate unconditional grants are accepted.
- `SystemIO` merges access flags for the same path inside its own builder. Separate enabled rulesets supplying the same `PathBuf` cause **`DuplicatePath`**, rather than merging. This is a builder key check, not an inode/alias equivalence check. In one Landlock layer, an allowed ancestor grants rights throughout its subtree; a narrower child rule cannot revoke that grant.
- Separately **installed** seccomp filters stack at the kernel and cannot relax existing restrictions. That is different from unioning grants before installation. Landlock layers likewise only restrict further.

[Extrasafe composition implementation](https://docs.rs/crate/extrasafe/0.5.1/source/src/lib.rs), [SystemIO flag merging](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/systemio.rs), [kernel filter stacking](https://docs.kernel.org/userspace-api/seccomp_filter.html), [Landlock policy layers](https://man7.org/linux/man-pages/man7/landlock.7.html)

### Built-ins are conveniences with broad grants

| Built-in                              | What to review before enabling it                                                                                                                                                                                                                                                                                                  |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `BasicCapabilities`                   | **Automatically added at application time.** Allows memory allocation/mapping/protection, signal setup, futexes, randomness, scheduler/affinity operations, process/thread IDs, `uname`, `readlink`, `rseq`, and exit syscalls. A custom policy is therefore not an exact syscall allowlist containing only the rules it declares. |
| `SystemIO::nothing()`                 | Starts empty; explicitly add reads, writes, opening, metadata, close, or FD/path access. `everything()` is much broader but still not literally every Linux I/O syscall.                                                                                                                                                           |
| `SystemIO` Landlock helpers           | `allow_read_path` adds read/open/close/metadata syscalls; `allow_write_file` adds general write/open/close/metadata calls; creation/listing helpers add further groups. `allow_list_dir` adds unfiltered `ioctl` and `fcntl`. The apparent path-level grant also widens seccomp.                                                   |
| `Networking`                          | Starting clients permits new connections; “running” methods still allow substantial socket I/O and ordinary reads/writes on **any eligible descriptor**, not just the intended connection. Some server constructors require `.yes_really()`.                                                                                       |
| `Time`, `pipes::Pipes`                | Time-related and pipe-related convenience sets; not guarantees that all runtime requirements are covered.                                                                                                                                                                                                                          |
| `danger_zone::Threads`, `ForkAndExec` | Thread creation grants both `clone` and `clone3` without namespace/flag restrictions; fork/exec also permits process creation/execution and waits. These can substantially expand authority.                                                                                                                                       |

[BasicCapabilities source](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/basic.rs), [SystemIO source](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/systemio.rs), [network source](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/network.rs), [time source](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/time.rs), [pipes source](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/pipes.rs), [danger-zone source](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/danger_zone.rs)

`YesReally<T>::yes_really()` is an API acknowledgment, not a runtime security check. Automatic `BasicCapabilities` can also conflict with a custom conditional restriction on one of its syscalls, and that conflict may appear only during application. Choose direct seccompiler if the implicit grants are unacceptable. [Built-in wrapper](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/mod.rs), [application logic](https://docs.rs/crate/extrasafe/0.5.1/source/src/lib.rs)

## Enforcement, thread scope, and partial setup

The seccomp path collects syscall-number rules into seccompiler's `BTreeMap`, constructs an **allow-on-match / errno-on-mismatch** filter for the target architecture, compiles classic BPF, and installs it. Seccompiler sets **`no_new_privs`** before installing; ordinary callers need not be root. That bit prevents privilege gains through setuid/file-capability execution, is inherited, and cannot be cleared. Filter installation errors and TSYNC failures are returned through `SeccompError`; compilation/validation errors also propagate. Unknown/unlisted syscall invocations are denied, not silently permitted. Syscalls named in a policy but absent on the running kernel still cannot become available merely because a filter allows their numbers. [Seccompiler 0.4 source](https://docs.rs/crate/seccompiler/0.4.0/source/src/lib.rs), [seccomp contract](https://man7.org/linux/man-pages/man2/seccomp.2.html), [no_new_privs contract](https://docs.kernel.org/userspace-api/no_new_privs.html)

`apply_to_current_thread()` leaves existing sibling threads unrestricted by this new policy. Threads/processes subsequently created by the restricted task inherit it, and exec preserves it. `apply_to_all_threads()` requests **TSYNC** across the thread group; divergent preexisting filter trees can make synchronization fail. It does not retrofit restrictions into already separate child processes. Threads share memory and usually descriptor tables: a restricted thread cannot safely contain arbitrary malicious code while privileged sibling threads remain accessible. Install before untrusted input, or use a separate worker process. [Thread application methods](https://docs.rs/extrasafe/0.5.1/extrasafe/struct.SafetyContext.html), [kernel TSYNC semantics](https://man7.org/linux/man-pages/man2/seccomp.2.html), [crate thread warning](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/danger_zone.rs)

The **released Landlock path** behaves as follows:

1. Nonempty path rules trigger Landlock, unless `landlock_only()` requests it independently. Merely enabling the Cargo feature does not enforce a filesystem policy.
2. It handles `AccessFs::from_all(ABI::V2)` with **`CompatLevel::HardRequirement`**, creates the kernel ruleset, and adds path rules. Missing rights/ABI or a disabled Landlock generally produce a creation/compatibility error rather than a best-effort ABI downgrade.
3. Each path is opened with `PathFd::new`. **Any failure opening that path is ignored**, and the grant is omitted. This is usually more restrictive, but can cause missing required input/workspace access. The `PathDoesNotExist` error variant is not used to report these failures in this loop.
4. It calls `restrict_self()` but stores the entire `Result` in `_status` and returns `Ok(())`. Neither the final error nor enforcement status reaches the caller.
5. When both mechanisms are requested, **Landlock runs before seccomp**, not after it.

These are source findings, including behavior that the user guide's “requires ABI 2” statement alone does not reveal. [Published application code](https://docs.rs/crate/extrasafe/0.5.1/source/src/lib.rs), [error variants](https://docs.rs/crate/extrasafe/0.5.1/source/src/error.rs), [Landlock 0.3.1 final enforcement and status](https://docs.rs/crate/landlock/0.3.1/source/src/ruleset.rs)

| Failure point                                            | Remaining state                                                          | Safe caller response                                                                            |
| -------------------------------------------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| Policy conflict, before installation                     | This context has installed neither mechanism.                            | Report configuration error; do not launch.                                                      |
| Landlock creation/add-rule error                         | New seccomp filter has not been installed. Earlier policies still exist. | Exit the worker.                                                                                |
| Landlock final restriction error                         | Extrasafe can ignore it and proceed to seccomp, returning success.       | Do not treat this API's success as proof of required Landlock. Use checked direct installation. |
| Landlock succeeds, seccomp conversion/installation fails | Landlock remains; `no_new_privs` may also remain.                        | Exit the worker; never continue a partially initialized workload or retry “without sandbox.”    |
| Seccomp installation fails without Landlock              | `no_new_privs` may already be set.                                       | Exit; supervisor stays unrestricted.                                                            |
| Required path cannot be opened                           | Extrasafe may silently omit its grant.                                   | Validate/open required paths explicitly; fail on error.                                         |

This table is an analysis of the control flow, not a runtime reproduction of every failure. A successful seccomp install can coexist with failed Landlock restriction because the latter error is swallowed. A surfaced Landlock creation error, however, prevents reaching the seccomp step. [Published control flow](https://docs.rs/crate/extrasafe/0.5.1/source/src/lib.rs)

Landlock in this dependency generation is **per-thread**, inherited by future children, without TSYNC. Extrasafe rejects `apply_to_all_threads()` when its Landlock path map is nonempty. There is an additional edge case: `landlock_only()` with no path rules bypasses that check, but still restricts only the current thread. Also, the documented empty-policy recipe needs an explicitly enabled rule such as `BasicCapabilities`, because the implementation rejects a completely empty context before applying Landlock. Do not use either edge case as an all-thread confinement recipe. [Released implementation and docstrings](https://docs.rs/crate/extrasafe/0.5.1/source/src/lib.rs)

**Recommendation:** for required filesystem isolation, either fix/audit the wrapper before adoption or use the direct `landlock` API and check both the returned error and `RulesetStatus::FullyEnforced`. “Fully enforced” is relative to **requested rights**, not every feature in the running kernel. Prepare namespaces and mounts first, then Landlock, then the final seccomp filter. Perform these irreversible steps in a single-threaded child that exits on any failure. A support/version probe is useful diagnostics but cannot replace checking installation in the actual child. [Landlock compatibility/status API](https://docs.rs/landlock/0.3.1/landlock/struct.RestrictionStatus.html), [kernel filesystem setup limitations](https://man7.org/linux/man-pages/man7/landlock.7.html)

## Isolate: subprocess and namespace behavior

`Isolate::new(name, fn() -> ())` configures a **function pointer, not a captured closure or arbitrary executable**. Call `Isolate::main_hook(name, builder)` at the very start of ordinary synchronous `main`. Later, `Isolate::run(name, &HashMap<String, String>)` copies `/proc/self/exe` to a memfd and reexecutes it through `/proc/self/fd/N` with the isolate name as `argv[0]`. It clears the environment and adds only the provided variables; this is a new address space, not transfer of the parent's live heap. [Public isolate implementation](https://docs.rs/crate/extrasafe/0.5.1/source/src/isolate/mod.rs)

The hook recognizes the name and clones into new **user, mount, PID, IPC, and UTS namespaces**, plus a **network namespace by default**. Inside, it maps the parent's effective UID/GID to namespace root, writes `setgroups = deny`, mounts a tmpfs, makes that mount private, creates configured bind mounts, pivots to the new root, detaches the old root, closes FDs **3 and above**, and calls the worker function. Root tmpfs defaults to **10 MiB**, configurable with `set_rootfs_size`; this is not a total process-memory limit. There is no automatic procfs or device filesystem mounted in the new root. [Namespace setup source](https://docs.rs/crate/extrasafe/0.5.1/source/src/isolate/isolate_sys.rs), [builder defaults](https://docs.rs/crate/extrasafe/0.5.1/source/src/isolate/mod.rs)

Material constraints:

- **No read-only bind-mount option.** `add_bind_mount` uses recursive bind mounting without read-only remounts or capability dropping. Host writes allowed by underlying DAC remain possible through these mounts. The source's capability-drop step is a TODO; namespace root retains capabilities in its namespace, though it is not host root.
- **Do not accept attacker-selected mount destinations.** Released code joins the supplied destination to the temporary root without rejecting `..`; the post-release fix adds validation. Resolve trusted source paths and validate destination components yourself if retaining this release. Also consider symlink/mount races; string validation alone does not establish safe mount setup.
- **No implicit syscall policy.** Mount, socket, and process syscalls remain available until the callback explicitly installs a policy. New networking is detached from host interfaces by default; `new_network(false)` deliberately shares the host network instead.
- **Stdio remains.** `run().output()` normally supplies captured stdout/stderr and null stdin, but bootstrap code must not replace these with powerful descriptors before restriction. A bind-mounted service socket can still convey host authority. Namespaces are not a protocol authorization layer.
- **Startup is trusted code.** The loader and code before the hook execute before isolation. Put the hook before Tokio, background threads, global services, and workload processing. A multithreaded parent may invoke `run()` as a subprocess operation, but its reexecuted child must reach the hook single-threaded. Starting an async runtime inside the callback is a separate compatibility task; the API does not carry an existing runtime across execution.
- **Failures are split across interfaces.** Memfd/spawn failures may return `IsolateError`; setup operations mostly panic in the child. Check `Output.status`, capture diagnostics, and require completion. The raw clone return is not validated immediately; subsequent wait/setup failure is not a structured capability report. `run()` offers neither timeout nor child handle for cancellation and buffers output in memory. Use an external supervisor with deadlines/output limits for hostile workloads.

[Released mount/clone/FD implementation](https://docs.rs/crate/extrasafe/0.5.1/source/src/isolate/isolate_sys.rs), [launcher error/output behavior](https://docs.rs/crate/extrasafe/0.5.1/source/src/isolate/mod.rs), [merged but unpublished validation](https://github.com/boustrophedon/extrasafe/commit/b56d18c8d7b337aa19fe5b14c8af51b68db00c13)

A memfd copy does not imply executable sealing: this implementation creates the memfd with flags zero, does not add seals, and closes it during child setup. Do not interpret the module's copy-related description as an independent integrity guarantee against hostile bootstrap code. Reexecution also changes `$ORIGIN` dynamic-library lookup; **open issue #38** documents a failure before the hook with origin-relative `DT_RUNPATH`. Review dynamic libraries, late `dlopen`, interpreters, certificate/DNS data, and target executable access when constructing a minimal root. A preloaded in-process callback may work without a full runtime tree, whereas an exec'd tool generally needs its loader/libraries and explicit execution grants. [Memfd implementation](https://docs.rs/crate/extrasafe/0.5.1/source/src/isolate/isolate_sys.rs), [applicable open loader defect](https://github.com/boustrophedon/extrasafe/issues/38)

## Practical guarantees and gaps

A sensible use is hardening a parser, converter, or network service **after trusted initialization**, to limit the effect of a bug. Seccomp can also contribute to malicious-code confinement, but does not by itself create filesystem, network-destination, resource, or process-memory boundaries. A privileged sibling or reachable broker may perform the forbidden operation on the worker's behalf. [Kernel seccomp purpose and limits](https://docs.kernel.org/userspace-api/seccomp_filter.html), [Landlock design](https://docs.kernel.org/security/landlock.html)

| Requirement or pitfall                   | What 0.5.1 can do                                                                                             | Gap category and mitigation                                                                                                                                                                                                                                                 |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Read-only inputs plus writable workspace | Landlock helpers express separate path grants; seccomp can gate related syscalls.                             | **Wrapper defect / missing requested OS right:** final result unchecked and ABI 2 omits `TRUNCATE`. Require checked ABI 3+ Landlock for content integrity, or properly read-only mounts plus a reviewed policy.                                                             |
| No networking                            | Deny all new socket operations in a narrow seccomp policy; `Isolate` defaults to a private network namespace. | **Integration:** close inherited sockets, exclude host socket bind mounts, account for stdio/brokers and any alternate enabled I/O paths. Blocking `socket`/`connect` alone leaves already connected sockets usable.                                                        |
| Only approved IPs/domains/URLs           | No destination allowlist in `Networking`; classic BPF cannot read pointed-to `sockaddr` or DNS/HTTP content.  | **Needs another mechanism:** trusted proxy/broker, firewall in a controlled network namespace, or equivalent network policy. Modern Landlock TCP port rules are not host/domain filtering and are absent from this release.                                                 |
| Descriptor-specific access               | `allow_file_read`, `allow_file_write`, stdin/stdout/stderr helpers filter numeric FDs.                        | **OS/integration:** FD numbers can be reused by close/open/dup/receipt. Retain intended descriptors; close everything else; restrict duplication/acquisition. A numeric comparison does not bind an inode or peer.                                                          |
| Read-only open helper                    | `allow_open_readonly` masks selected flags on `open`/`openat`; `openat2` is not granted.                      | **Released policy gap:** mask omits `O_TRUNC`; it is not a content-integrity guarantee on Linux. Never use this helper alone to promise input immutability.                                                                                                                 |
| Existing files/mappings/sockets          | New accesses are filtered; existing authority may survive.                                                    | **OS design:** Landlock does not retroactively revoke already-open file access or mappings. Inventory/close descriptors before enforcement; start fresh workers. Broad `write` also writes to inherited sockets.                                                            |
| Paths, aliases, shared input objects     | Landlock follows object/path hierarchy rules, not textual prefix checks.                                      | **Integration:** a writable hard-link or bind-mount alias can expose input content through an approved subtree. Use trusted, disjoint directory trees and fresh workspace objects; review external mutation and mount races. Symlink targets still need appropriate access. |
| Metadata, device operations, IPC         | Syscall policy can deny selected operations.                                                                  | **OS/API coverage:** ABI 2 is not a metadata-hiding policy and does not cover newer device-ioctl/IPC-scoping rights. Broad metadata/ioctl grants and exposed Unix sockets need separate review. Abstract Unix sockets are outside pathname policy.                          |
| Async/threaded workloads                 | Explicit runtime syscalls can be enabled.                                                                     | **Integration:** epoll, eventfd, timers, pipe/socketpair, `clone3`, signals, and possibly io_uring vary by runtime/configuration. Do not enable all networking merely to satisfy a poller; avoid broad io_uring access without separate analysis.                           |
| Descendants and resource exhaustion      | Installed filters/LL restrictions are inherited by future descendants.                                        | **Needs another mechanism:** process-tree supervision, time/output/memory/PID limits, usually cgroups/rlimits. PID 1 behavior and descendant cleanup need testing. Tmpfs size is not a CPU or total-memory budget.                                                          |
| Kernel compromise                        | Reduces exposed operations.                                                                                   | **Underlying boundary:** still shares the Linux kernel; a VM or other stronger boundary is appropriate when kernel escape is in scope.                                                                                                                                      |

The I/O, network, and FD findings are grounded in the [published SystemIO implementation](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/systemio.rs) and [Networking implementation](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/network.rs). Linux documents [`O_TRUNC` behavior even with read-only access on many systems](https://man7.org/linux/man-pages/man2/open.2.html), [Landlock's truncation ABI history and limitations](https://docs.kernel.org/userspace-api/landlock.html), [descriptor semantics](https://docs.kernel.org/security/landlock.html), and [the syscall-filter pointer limitation](https://man7.org/linux/man-pages/man2/seccomp.2.html). Mitigations in the table are recommendations, not extrasafe features.

Network built-ins have a further source-level caveat: socket families and types are tested with **bit-mask inclusion**, not equality (for example, testing that the `AF_INET` bits are set). Family identifiers are enums, not independent capability bits, so these tests can match other values. Treat “TCP-only” or “Unix-only” names as conveniences that need review, not exact family confinement. Use explicit equality and a type mask appropriate for `SOCK_CLOEXEC`/`SOCK_NONBLOCK` when writing a custom policy. [Released socket conditions](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/network.rs)

### Defects, advisories, and evidence limits

The unchecked Landlock result, skipped path errors, ABI 2 truncation omission, broad socket masks, and empty/all-thread Landlock edge cases above were identified in the **0.5.1 source**. This report does not assign them advisory identifiers or claim an exploit was reproduced. The `.`/`..` fix is a documented, merged upstream change, **unreleased** as of this research. The `$ORIGIN` issue remains open and affects the same reexecution design. [Published source](https://docs.rs/crate/extrasafe/0.5.1/source/), [path-fix PR](https://github.com/boustrophedon/extrasafe/pull/43), [loader issue](https://github.com/boustrophedon/extrasafe/issues/38)

A research-date query to OSV for crates.io `extrasafe` version `0.5.1` returned no vulnerability records; the RustSec advisory index/search also yielded no extrasafe entry. That is a limited database observation, **not evidence of security or an audit of the resolved dependency graph**. Run dependency auditing against the deployment's actual lockfile. [OSV query API](https://google.github.io/osv.dev/post-v1-query/), [RustSec database](https://rustsec.org/advisories/)

Some docs.rs URLs were unavailable through the browsing fetcher. API behavior was checked against the **downloaded crates.io 0.5.1 archive**, including its manifest, source, examples, tests, and changelog; versioned docs.rs source links identify the corresponding published files. Repository changes are cited separately. The neighboring Landlock report was a lead, not evidence for extrasafe's implementation.

## Complete Rust examples

These are **separate, one-binary Cargo projects**. Put each manifest in `Cargo.toml` and its Rust source in `src/main.rs`. Run/build them on Linux x86_64, outside the monorepo dependency graph. Each uses the exact examined extrasafe release. Commit a generated `Cargo.lock` for deployment; pinning extrasafe alone does not freeze its transitive dependencies.

Start each worker as a new process with a trusted supervisor, null stdin and controlled stdout/stderr pipes, no inherited privileged descriptors, and a deadline. Never install these policies into an IDE, test runner, or long-lived supervisor. The filesystem example explicitly closes FDs above 2; the minimal example demonstrates only its stated syscall policy, not descriptor revocation.

### 1. Minimal custom syscall policy

**Required extrasafe features: none.** Only writes to FD 1 or 2 are explicitly granted; automatic `BasicCapabilities` still applies. The two write rules are alternatives, while additional conditions inside either rule would be conjunctive. It denies socket creation and validates the denial using a raw syscall. Failure to install returns an error before the probe/workload; an unexpected probe result exits unsuccessfully. `--all-threads` selects the TSYNC path and checks its return value; without it only the calling thread is restricted. A ready sibling deliberately tests that distinction before exiting. This is an educational probe, not a recommendation to retain an unrestricted sibling in a production worker; its reference socket operation must be allowed by the outer environment.

```toml
[package]
name = "extrasafe-minimal"
version = "0.1.0"
edition = "2021"

[dependencies]
extrasafe = { version = "=0.5.1", default-features = false }
libc = "=0.2.186"
```

```rust
use extrasafe::{RuleSet, SafetyContext, SeccompRule, seccomp_arg_filter, match_argno};
use extrasafe::syscalls::Sysno;
use std::collections::HashMap;

struct Diagnostics;
impl RuleSet for Diagnostics {
    fn name(&self) -> &'static str { "diagnostics" }
    fn simple_rules(&self) -> Vec<Sysno> { vec![] }
    fn conditional_rules(&self) -> HashMap<Sysno, Vec<SeccompRule>> {
        HashMap::from([(Sysno::write, vec![
            SeccompRule::new(Sysno::write)
                .and_condition(seccomp_arg_filter!(arg0 == 1)),
            SeccompRule::new(Sysno::write)
                .and_condition(seccomp_arg_filter!(arg0 == 2)),
        ])])
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let all = std::env::args().any(|arg| arg == "--all-threads");
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (go_tx, go_rx) = std::sync::mpsc::channel();
    let sibling = std::thread::spawn(move || {
        ready_tx.send(()).expect("ready receiver");
        go_rx.recv().expect("start sender");
        let fd = unsafe { libc::syscall(libc::SYS_socket, libc::AF_INET, libc::SOCK_STREAM, 0) };
        let denied = fd == -1
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
        if fd >= 0 { unsafe { libc::close(fd as i32); } }
        if all { denied } else { fd >= 0 }
    });
    ready_rx.recv()?;
    let policy = SafetyContext::new().enable(Diagnostics)?;
    if all { policy.apply_to_all_threads()?; }
    else { policy.apply_to_current_thread()?; }

    go_tx.send(())?;
    let sibling_ok = sibling.join().map_err(|_| "sibling panicked")?;
    if !sibling_ok { std::process::exit(72); }

    // Raw syscalls avoid a userspace cache or vDSO masking the test.
    let socket = unsafe { libc::syscall(libc::SYS_socket, libc::AF_INET, libc::SOCK_STREAM, 0) };
    let denied = socket == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM);
    if !denied { std::process::exit(70); }
    let message = b"seccomp probe passed\n";
    let n = unsafe { libc::write(1, message.as_ptr().cast(), message.len()) };
    std::process::exit(if n == message.len() as isize { 0 } else { 71 });
}
```

The explicit `match_argno` import is necessary for the released macro's expansion; the crate documentation alternatively recommends `use extrasafe::*`. `write` checks the descriptor number, not what it refers to. A supervisor that attaches stdout to a socket has intentionally granted access to that socket. No pathname I/O or socket creation is granted here. [Macro source](https://docs.rs/crate/extrasafe/0.5.1/source/src/macros.rs), [BasicCapabilities](https://docs.rs/extrasafe/0.5.1/extrasafe/builtins/struct.BasicCapabilities.html)

### 2. Checked read-only inputs and writable workspace

**Required extrasafe feature: `landlock`. Additional dependency: `landlock = "=0.3.1"`.** This intentionally uses extrasafe's Landlock type reexports and the direct checked installation API, followed by extrasafe's seccomp policy. It does **not** rely on the wrapper's unchecked final Landlock call. `landlock::RulesetStatus` is imported directly because extrasafe does not reexport that status enum.

The worker requires existing, disjoint input/workspace directories and `input.txt` inside the inputs. It permits file reads in inputs, and file reads/writes/creation/truncation in the workspace. It grants no directory creation, deletion, linking, rename, execution, or networking. **ABI 3 is mandatory**, including truncation control: an old/disabled/blocked Landlock fails before the workload. This is the explicit refusal path for required controls, not a best-effort downgrade. No existing input or workspace file is opened before enforcement.

```toml
[package]
name = "extrasafe-filesystem"
version = "0.1.0"
edition = "2021"

[dependencies]
extrasafe = { version = "=0.5.1", default-features = false, features = ["landlock"] }
landlock = "=0.3.1"
libc = "=0.2.186"
```

```rust
use extrasafe::{Access, AccessFs, Compatible, CompatLevel, PathBeneath,
    PathFd, Ruleset, RulesetAttr, RulesetCreatedAttr, ABI, SafetyContext};
use extrasafe::builtins::SystemIO;
use landlock::RulesetStatus;
use std::path::PathBuf;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let inputs = PathBuf::from(args.next().ok_or("missing input directory")?).canonicalize()?;
    let workspace = PathBuf::from(args.next().ok_or("missing workspace directory")?).canonicalize()?;
    if args.next().is_some() || !inputs.is_dir() || !workspace.is_dir()
        || inputs.starts_with(&workspace) || workspace.starts_with(&inputs) {
        return Err("require two disjoint, existing directories".into());
    }
    // This binary must start single-threaded, with controlled pipes on 0..=2.
    let rc = unsafe { libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32) };
    if rc != 0 { return Err(std::io::Error::last_os_error().into()); }

    // ABI 3 is required because read-only includes protection against truncation.
    let abi = ABI::V3;
    let ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(abi))?
        .create()?
        .add_rule(PathBeneath::new(PathFd::new(&inputs)?,
            AccessFs::ReadFile | AccessFs::ReadDir))?
        .add_rule(PathBeneath::new(PathFd::new(&workspace)?,
            AccessFs::ReadFile | AccessFs::ReadDir | AccessFs::WriteFile
            | AccessFs::MakeReg | AccessFs::Truncate))?;
    let status = ruleset.restrict_self()?;
    if status.ruleset != RulesetStatus::FullyEnforced || !status.no_new_privs {
        return Err(format!("required Landlock controls absent: {status:?}").into());
    }

    // No Landlock rules here: use extrasafe only for the second, seccomp step.
    SafetyContext::new().enable(SystemIO::nothing()
        .allow_open().yes_really().allow_read().allow_write()
        .allow_close().allow_metadata())?
        .apply_to_current_thread()?;

    // Workload begins only after both irreversible steps have succeeded.
    let data = std::fs::read(inputs.join("input.txt"))?;
    std::fs::write(workspace.join("output.txt"), data)?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("worker refused or failed: {error}");
        std::process::exit(1);
    }
}
```

Run the binary with `INPUT_DIRECTORY WORKSPACE_DIRECTORY`. Both must be trusted trees prepared by the supervisor. `FullyEnforced` plus `no_new_privs` is checked before the second step; if seccomp then fails, the worker exits while Landlock remains active. There is no rollback and no unrestricted retry. This example protects the requested filesystem operations, not metadata confidentiality or all newer Landlock rights. It needs a kernel exposing ABI 3 (upstream Linux 6.2+) and permitting installation; checking release numbers alone is insufficient. [Landlock 0.3.1 access rights](https://docs.rs/landlock/0.3.1/landlock/enum.AccessFs.html), [status contract](https://docs.rs/landlock/0.3.1/landlock/struct.RestrictionStatus.html), [kernel ABI history](https://man7.org/linux/man-pages/man7/landlock.7.html)

Extrasafe's built-in spelling for a similar **ABI 2** policy is `SystemIO::nothing().allow_read_path(&inputs).allow_read_path(&workspace).allow_create_in_dir(&workspace).allow_write_file(&workspace)`, enabled in a `SafetyContext`. That is useful API vocabulary, **not a substitute for the checked example**: it lacks truncation handling and trustworthy final enforcement reporting. Listing directories additionally needs `allow_list_dir`, whose syscall grants deserve review. [Published helper definitions](https://docs.rs/crate/extrasafe/0.5.1/source/src/builtins/systemio.rs)

Closing FDs does not remove already established mappings or authority held by sibling threads. The example assumes a fresh single-threaded worker, controlled stdio, no untrusted constructors, and workspace objects that do not alias input objects. Do not run it inside an already initialized multi-thread Tokio runtime. Neither canonicalization nor the overlap check is an inode/hard-link audit.

### 3. Isolated subprocess with no host mounts

**Required extrasafe feature: `isolate`.** This demonstrates the released API without bind mounts, avoiding the published destination-validation gap. Its callback writes only to its private tmpfs and uses a new network namespace. It does not claim an exact syscall policy, CPU/memory limits, or a complete malicious-code boundary. Setup panics become child launch failures; both returned launch errors and non-success child status stop the caller's success path. The completion marker checks normal callback completion, not security.

```toml
[package]
name = "extrasafe-isolated"
version = "0.1.0"
edition = "2021"

[dependencies]
extrasafe = { version = "=0.5.1", default-features = false, features = ["isolate"] }
```

```rust
use extrasafe::isolate::Isolate;
use std::collections::HashMap;

const NAME: &str = "extrasafe-offline-probe";
fn worker() {
    if let Err(error) = std::fs::write("/result.txt", b"private tmpfs") {
        eprintln!("isolated workload failed: {error}");
        std::process::exit(1);
    }
    println!("finished");
}
fn build(name: &'static str) -> Isolate {
    Isolate::new(name, worker).new_network(true).set_rootfs_size(1)
}
fn main() {
    // Must precede runtime initialization or creation of any threads.
    Isolate::main_hook(NAME, build);
    match Isolate::run(NAME, &HashMap::new()) {
        Ok(output) if output.status.success() && output.stdout == b"finished\n" => {}
        Ok(output) => {
            eprintln!("isolate refused or failed: {}; {}", output.status,
                String::from_utf8_lossy(&output.stderr));
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("isolate launch failed: {error:?}");
            std::process::exit(1);
        }
    }
}
```

An external supervisor must bound the lifetime and captured output of this entire program; `Isolate::run` alone cannot enforce that. Do not catch a namespace failure and call `worker()` directly. To add seccomp, install it inside the callback before admitting workload input, after namespace/mount setup; propagate failure by terminating the callback process. Do not install the minimal filter in the parent before `run`, because it denies the bootstrap syscalls.

### Verification actually performed

All three source examples were **compile-checked and built together** using the published `extrasafe = 0.5.1`, `landlock = 0.3.1`, `seccompiler = 0.4.0`, `syscalls = 0.6.18`, and `libc = 0.2.186`, with both optional extrasafe features enabled. Each example also passed a separate `cargo check --offline` with exactly its documented optional features: none, `landlock`, or `isolate`. The isolate-only check resolved `libc = 0.2.191`; the other two pinned 0.2.186. The build host was **x86_64 GNU/Linux, kernel `7.0.14-25-pve`, rustc 1.97.1 (`8bab26f4f`, 2026-07-14), Cargo 1.97.1**; work was initiated from macOS with rustc 1.98.1. No claim of native macOS/Windows support follows from this.

| Example    | Compile/build | Execution evidence                                                                                                                                                                                                                                                                                                    |
| ---------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Minimal    | Passed        | Both default current-thread mode and `--all-threads` exited 0 and printed `seccomp probe passed`; raw socket creation returned `EPERM`. A preexisting, ready sibling could create a socket in current-thread mode and received `EPERM` in TSYNC mode.                                                                 |
| Filesystem | Passed        | A fresh subprocess with disposable fixture directories exited 1 reporting incompatible required Landlock rights; `output.txt` was absent. **Refusal path verified; successful filesystem enforcement not verified on this host.** The error does not distinguish kernel-disabled Landlock from outer policy blocking. |
| Isolate    | Passed        | **Not executed.** Namespace/mount behavior, setup cleanup and offline guarantees remain unverified in this research environment.                                                                                                                                                                                      |

The supervisor remained unrestricted. Filesystem fixtures were removed after the subprocess exited; no persistent host security settings were changed. `cargo check --bins` and `cargo build --bins` were used in an isolated probe project, not `cargo test`. Compilation establishes API compatibility only, and the seccomp probe establishes only the observed operations on this host.

### Subprocess tests needed before adoption

Use a disposable Linux VM/container configured for the required facilities. The test controller launches workers and observes results; it must never restrict itself. Keep denial targets in disposable fixtures so a policy failure cannot damage real files.

| Test                                        | Required observation                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Policy composition                          | Two conditional FD grants both work; adding an unconditional grant for that syscall returns `ConditionalNoEffectError`; duplicate Landlock paths across rulesets return `DuplicatePath`.                                                                                                                                                                                       |
| Allowed and forbidden filesystem operations | Read input and create/overwrite workspace output. Deny outside reads/writes, input write and `O_RDONLY \| O_TRUNC`, disallowed creation/deletion/rename/exec, and symlink escapes. Re-read fixture contents from the unrestricted controller, so a failed denial cannot masquerade as success.                                                                                 |
| Current-thread scope                        | The minimal example exercises this basic distinction; retain it as a regression test. Start a sibling and wait for its ready acknowledgment **before** installing in the worker thread. After installation, a raw forbidden syscall fails in the restricted thread but succeeds in the preexisting sibling. Keep this educational scope test isolated from production workers. |
| TSYNC and inheritance                       | With preexisting ready siblings sharing the same filter tree, require raw denials in every sibling after `apply_to_all_threads`. Test a divergent sibling filter causing TSYNC failure and verify the workload marker is absent. Separately test inheritance into permitted new threads, children, and exec targets.                                                           |
| Landlock final-call failure                 | In a disposable child, install an outer filter denying `landlock_restrict_self` while allowing its create/add-rule calls. Demonstrate the released wrapper's apparent success and lack of filesystem enforcement. Repeat with the checked example and require refusal. Do not mistake denial from the outer filter for proof that the inner policy works.                      |
| Required capability unavailable             | Use a kernel/VM with absent Landlock, an older ABI, and an outer filter that blocks setup. The checked worker must exit unsuccessfully without producing a workload marker. Do not modify host-wide LSM/sysctl settings to simulate this.                                                                                                                                      |
| Namespace launch/setup failures             | Deny clone/user namespaces, mount, executable memfd, or descriptor closing in disposable children; test quota exhaustion and an invalid trusted mount source. Require non-success or launch error, absent callback marker, bounded completion, and no fallback execution. Check cleanup of temporary host directories.                                                         |
| Networking and inherited authority          | Test new sockets, inherited connected sockets, IPv4/IPv6, UDP, pathname/abstract Unix sockets, and broker access under the **actual** policy. For a destination requirement, show an approved endpoint succeeds and an unapproved endpoint fails at the external enforcement layer.                                                                                            |
| Real workload and process lifetime          | Exercise loaders, DNS/certificates when needed, async operations, fork/exec and grandchildren; verify deadlines, resource/output limits, and descendant termination from the supervisor.                                                                                                                                                                                       |

These tests extend the limited probes recorded above; **the remaining adoption tests have not been performed**. Avoid using success messages, kernel version, `Seccomp: 2`, or one negative operation as proof of complete policy enforcement. For fault injection, use explicit per-child filters or disposable kernel environments, not persistent host configuration changes.

## Alternatives and complementary tools

All the native mechanisms below are Linux-specific. They are not native macOS or Windows sandbox backends; use platform-specific implementations or a Linux virtualized backend there.

| Choice                                         | Relationship and benefit                                                                                                                                                                                                                 | Cost, coverage, and adoption considerations                                                                                                                                                                                                                                                                                                     |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Direct `seccompiler`**                       | Lower-level alternative to extrasafe's syscall facade; extrasafe 0.5.1 already uses its 0.4 backend. Gives explicit default/match actions, conditions, architecture, BPF, and application control without automatic `BasicCapabilities`. | More policy authoring and runtime syscall maintenance; still cannot dereference pointer arguments or supply filesystem/destination policy. The current upstream advertises more architectures than extrasafe. **Repository was archived when checked**; evaluate support/fork strategy rather than assuming the backend is actively maintained. |
| **libseccomp with a Rust binding**             | Competing seccomp construction path, with a mature native library and architecture/syscall-resolution facilities. It can expose facilities beyond extrasafe's small facade, depending on binding and native-library version.             | Requires native libseccomp deployment/build/linking and review of the chosen Rust binding's maintenance/API coverage. `libseccomp` and `libseccomp-sys` crates are bindings, not the C project itself or full sandbox launchers. Does not turn seccomp into pathname or IP/domain inspection.                                                   |
| **Direct `landlock`**                          | Complement to seccomp and alternative to extrasafe's Landlock adapter. Explicit compatibility choices, path-open errors and restriction status; newer crate versions expose newer kernel rights.                                         | Caller must define handled rights, thread coverage, grant topology and descriptor hygiene. No filesystem-view construction or whole-resource sandbox. The project is maintained under `landlock-lsm`; visible repository pushes continued in October 2026. The example pins 0.3.1 for reproducibility, not because it is the latest.            |
| **Bubblewrap**                                 | Namespace launcher for arbitrary executables; curated root with explicit read-only/read-write binds, namespace selection and descriptor/seccomp integration. A practical replacement for `Isolate` when launching existing tools.        | External executable and deployment/version policy; caller still supplies a correct sandbox profile, service-socket restrictions, network policy, and resource supervision. Contemporary upstream uses unprivileged user namespaces; namespace availability remains a deployment constraint. Repository pushes continued in October 2026.        |
| **VM / separately managed container boundary** | Additional boundary when running arbitrary hostile native workloads, especially if kernel exploitation or strong tenant separation is in scope.                                                                                          | More startup/operational cost. A container still shares its host kernel; a VM provides a different isolation boundary. Neither removes policy, secrets, descriptor, or output-validation responsibilities.                                                                                                                                      |

Primary sources: [seccompiler design/platforms](https://github.com/rust-vmm/seccompiler), [seccompiler archive status](https://api.github.com/repos/rust-vmm/seccompiler), [libseccomp upstream](https://github.com/seccomp/libseccomp), [Rust libseccomp bindings](https://docs.rs/libseccomp/latest/libseccomp/), [Rust Landlock project](https://github.com/landlock-lsm/rust-landlock), [Landlock repository activity](https://api.github.com/repos/landlock-lsm/rust-landlock), [Bubblewrap security model and usage](https://github.com/containers/bubblewrap), [Bubblewrap activity](https://api.github.com/repos/containers/bubblewrap).

**Recommended integration:** for a small self-restricting Rust worker, extrasafe can remain a convenient syscall-policy builder after reviewing its implicit grants. Require checked Landlock separately when filesystem controls are mandatory. For an arbitrary-binary launcher, use a maintained namespace launcher with explicit read-only mounts and a dedicated child bootstrap, then add syscall and network policy as needed. Treat all-thread coverage, setup status, network reachability and resource limits as distinct requirements, not one `sandboxed` boolean.

## Authoritative resources and unresolved adoption questions

- [Exact crates.io metadata/version list](https://crates.io/api/v1/crates/extrasafe), [0.5.1 API](https://docs.rs/extrasafe/0.5.1/extrasafe/), [0.5.1 published source](https://docs.rs/crate/extrasafe/0.5.1/source/), [0.5.1 user guide](https://docs.rs/crate/extrasafe/0.5.1/source/user-guide.md), and [0.5.1 changelog](https://docs.rs/crate/extrasafe/0.5.1/source/CHANGELOG.md).
- [Linked upstream repository](https://github.com/boustrophedon/extrasafe), [unpublished bind-mount validation fix](https://github.com/boustrophedon/extrasafe/pull/43), [open loader issue](https://github.com/boustrophedon/extrasafe/issues/38), and [open architecture-support investigation](https://github.com/boustrophedon/extrasafe/issues/35).
- [Kernel seccomp documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html), [seccomp(2)](https://man7.org/linux/man-pages/man2/seccomp.2.html), [kernel Landlock documentation](https://docs.kernel.org/userspace-api/landlock.html), [Landlock ABI/limitations manual](https://man7.org/linux/man-pages/man7/landlock.7.html), and [user namespace manual](https://man7.org/linux/man-pages/man7/user_namespaces.7.html).

Before adoption, resolve these material questions:

1. Who will maintain a patched/audited extrasafe adapter or replace its unchecked Landlock path and unpublished mount fix? No release resolving these issues was found.
2. Which deployment kernels and outer containers actually permit the required Landlock ABI, seccomp TSYNC, and namespace setup? Successful filesystem enforcement and isolate execution still need environment-specific evidence.
3. Is a single-threaded child architecture acceptable, and what are the exact inherited descriptors, shared filesystem objects, IPC endpoints, executables and runtime dependencies?
4. Does “network access” mean no host networking, a few TCP ports, specific IPs, or approved URLs/domains? The latter requirements need a separate enforcement layer.
5. Who supplies deadlines, output limits, cgroups/rlimits, descendant cleanup, and dependency/security maintenance, including the archived seccompiler upstream? These responsibilities do not disappear behind the higher-level API.
