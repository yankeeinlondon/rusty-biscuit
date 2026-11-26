---
prompt: |-
    Research the Rust `seccompiler` crate to help a developer decide whether
    and how to use it in the syscall-filtering layer of a Linux sandbox launcher.

    The starting point is Rust construction, compilation, and installation
    of Linux seccomp-BPF filters, with Rust and optional JSON policy inputs.
    Verify the published crate's supported architectures and API rather than
    assuming it is a general filesystem or network sandbox.

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
    - https://docs.rs/seccompiler/
    - https://github.com/rust-vmm/seccompiler

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
       - Explain conditions, argument widths/comparators, AND/OR rule semantics,
         syscall-number mappings, match/mismatch actions, and BPF compilation.
       - Describe Rust and JSON input equivalence, syscall name resolution,
         architecture selection, and per-thread versus all-thread installation.
       - Explain no_new_privs, action precedence, filter stacking, inheritance,
         irreversibility, privileges, and installation errors.
       - List Cargo features, defaults, serialization dependencies, architecture
         restrictions, and any native-library requirements or their absence.

    3. Platform integration, compatibility, and failure behavior
       - Verify supported Linux architectures/endianness and minimum kernel
         requirements for actions, argument handling, and thread synchronization.
       - Distinguish compiling a filter for a target from safely installing it
         there. Explain architecture guards, alternate syscall ABIs, and mismatch
         between target architecture and supplied syscall numbers.
       - Trace malformed JSON, unsupported operators/actions, invalid argument
         indices, BPF instruction limits, verifier rejection, and TSYNC failures.
       - Explain filter-map labels and who must apply a filter to each intended
         thread. Do not assume a named JSON section is installed automatically.
       - Determine whether user notification or other kernel features are exposed;
         identify actual API gaps rather than inventing support.
       - Address containers, outer filters, and WSL2 as runtime capability checks;
         distinguish them from native macOS or Windows support.
       - Show fail-closed error propagation that prevents untrusted work from
         starting after failed compilation or installation.

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
       - Explain pointer-argument limits, numeric truncation/width semantics,
         alternate syscall paths, architecture confusion, and preopened resources.
       - Examine all-thread synchronization, threads created around installation,
         runtime/allocator syscalls, and accidentally inverted default actions.
       - State why syscall filtering alone cannot implement path-based read-only
         inputs, destination identity restrictions, or resource containment.

    5. Rust examples and verification
       - Demonstrate equivalent focused Rust and JSON policies, explicit target
         architecture, an argument comparison, and meaningful allowed/denied probes.
       - Include a separate all-thread installation example where supported,
         required Cargo features, and errors that refuse workload startup.
       - Keep policy-generation tests separate from runtime enforcement tests.
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
       Compare libseccomp's native-library bindings, extrasafe's higher-level
       rule sets, and direct BPF generation. Explain how Landlock, namespaces,
       and resource limits complement syscall filtering rather than replace it.
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
hash: 1b52ce7398927464-5a8315b584977820
last_updated: 2026-11-04
---

# seccompiler for a Linux sandbox launcher

`seccompiler` is a suitable small policy compiler and installer for a launcher's **Linux syscall-filtering layer** when the launcher owns architecture selection, thread coverage, policy testing, and failure handling. It offers Rust and optional JSON policies without linking libseccomp. It does **not** provide path permissions, destination-aware networking, or resource containment. Treat it as one layer of confinement, including for malicious workloads, rather than treating successful installation as proof of a sandbox. The kernel explicitly describes seccomp as attack-surface reduction, not a complete sandbox. [Published API](https://docs.rs/seccompiler/0.5.0/seccompiler/), [kernel scope](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html#what-it-isn-t).

Research date: **2026-11-04**. Release examined: **seccompiler 0.5.0**, published **2025-03-07, 14:13:44 UTC**, the latest stable registry release on this date. Source inspection used the downloaded crates.io package, whose VCS metadata identifies commit `c3cf77d65815037931ae5bc2fca010713defdc8c`, and the matching `v0.5.0` tag. Moving `main` references below are explicitly maintenance evidence, not the released contract. [Registry metadata](https://crates.io/api/v1/crates/seccompiler), [published source](https://docs.rs/crate/seccompiler/0.5.0/source/).

## Identity, releases, and maintenance

The published package's repository is `rust-vmm/seccompiler`; its author is the **Amazon Firecracker Team**, and its license is **Apache-2.0 OR BSD-3-Clause**. It belongs to the rust-vmm virtualization components ecosystem, but does not require running a VM. It is a published library dependency, not a launcher executable. Registry owners are `andreeaflorescu`, `petreeftime`, `alindima`, and `github:rust-vmm:gatekeepers`; ownership is distinct from active review responsibility. The current monorepo CODEOWNERS still lists `@alindima @petreeftime` for this component. [Release manifest](https://docs.rs/crate/seccompiler/0.5.0/source/Cargo.toml), [registry owners](https://crates.io/api/v1/crates/seccompiler/owners), [current CODEOWNERS](https://github.com/rust-vmm/rust-vmm/blob/main/CODEOWNERS).

| Version | Publication date (UTC) | Relevant change                                                                                                                      |
| ------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| 0.5.0   | 2025-03-07             | Adds riscv64; JSON syscall tables updated from Linux 6.12. The only release in the past 24 months, 2024-11-04 through 2026-11-04.    |
| 0.4.0   | 2023-10-18             | Installs through `seccomp()` instead of `prctl(PR_SET_SECCOMP)`; adds TSYNC installation, `Error::Seccomp`, and `Error::ThreadSync`. |
| 0.3.0   | 2022-09-14             | Rust edition 2021; `From<BackendError> for Error`; equality-related changes.                                                         |
| 0.2.0   | 2021-10-01             | Rust filter compiler and optional JSON frontend described as the first release in the changelog.                                     |
| 0.1.0   | 2021-06-16             | Earlier registry publication; the changelog's “First release” wording for 0.2.0 does not erase it.                                   |

All five registry versions are non-yanked; the registry lists no prereleases. These dates come from package publication records, while API changes come from the release changelog. There is no 1.0 release to migrate from. A roughly 20-month publication gap after 0.5.0 is an observable fact, not proof of abandonment or reliability. [Version records](https://crates.io/api/v1/crates/seccompiler), [0.5.0 changelog](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/CHANGELOG.md).

The old repository was archived on **2026-08-18** and directs development to `rust-vmm/rust-vmm`. Subsequent monorepo changes include public `SyscallTable` exposure behind a new feature on **2026-09-21**, Linux 6.18 tables on **2026-09-25**, and a JSON documentation correction on **2026-11-04**. These are **unreleased**: the registry still serves 0.5.0, whose table module is private. Do not copy the monorepo's `syscall-table` feature into a 0.5.0 dependency. [Archive notice](https://github.com/rust-vmm/seccompiler), [table API commit](https://github.com/rust-vmm/rust-vmm/commit/6230ebc0ec), [table refresh](https://github.com/rust-vmm/rust-vmm/commit/c508188a7c), [current manifest](https://github.com/rust-vmm/rust-vmm/blob/main/seccompiler/Cargo.toml).

**Maintenance judgment:** there is evidence of continued component work after migration, but a long release interval and unclear active maintainer coverage are adoption risks. An open **2026-08-18** issue asks the listed codeowners to reconfirm activity, with a proposed September removal date; the checked CODEOWNERS file still names them. Do not infer their removal, availability, or a response-time commitment from that issue. [Maintainer review](https://github.com/rust-vmm/rust-vmm/issues/100), [component history](https://api.github.com/repos/rust-vmm/rust-vmm/commits?path=seccompiler&per_page=8).

The published manifest has **no `rust-version`**, and the release README declares no MSRV. Edition 2021 establishes a language-edition floor, not a verified package-and-dependencies MSRV. Pin and test the application's actual toolchain and lockfile; this report does not certify an older compiler. Version 0.4 adds error enum variants that affect exhaustive matches; 0.5 adds an architecture variant with similar implications. Its installation syscall change also matters under outer container policies that allow `prctl` but deny `seccomp`. Pre-1.0 versioning alone says nothing about enforcement quality. [Manifest](https://docs.rs/crate/seccompiler/0.5.0/source/Cargo.toml), [release changes](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/CHANGELOG.md).

## Public policy API

A system call is a numbered request to the kernel. Seccomp classic BPF evaluates a snapshot containing that number, an audit architecture, an instruction pointer, and six 64-bit argument slots. It does not inspect files or follow argument pointers. `seccompiler` provides policy objects that compile to this BPF instruction format. [Kernel filter input](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html#usage).

| Public item in 0.5.0                                                    | Meaning and constraints                                                                                                                                                  |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `SeccompCondition::new(index, length, operator, value)`                 | Argument index `u8`, valid 0–5; value `u64`. `Dword` compares the low 32 bits; `Qword` compares 64 bits.                                                                 |
| `SeccompCmpOp`                                                          | `Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`, `MaskedEq(u64)`. Ordered comparisons are unsigned; masked equality compares `(argument & mask) == (value & mask)`.                   |
| `SeccompRule::new(Vec<SeccompCondition>)`                               | ANDs its conditions; an empty condition vector is an error.                                                                                                              |
| `SeccompFilter::new(rules, mismatch_action, match_action, target_arch)` | `rules` is `BTreeMap<i64, Vec<SeccompRule>>`. Rules for one syscall are ORed; an empty rule vector makes that syscall match unconditionally. Equal actions are rejected. |
| `TryFrom<SeccompFilter> for BpfProgram`                                 | Compiles to `Vec<sock_filter>`; does not install. `BpfProgramRef` is a borrowed instruction slice.                                                                       |
| `compile_from_json<R: Read>(reader, TargetArch)`                        | Feature-gated; returns `BpfMap = HashMap<String, BpfProgram>`. It compiles every named policy but installs none.                                                         |
| `apply_filter(&program)`                                                | Installs on the calling Linux thread.                                                                                                                                    |
| `apply_filter_all_threads(&program)`                                    | Uses kernel TSYNC to synchronize the process's thread group; no fallback to single-thread installation.                                                                  |

The constructor order is **mismatch first, match second**. An allowlist typically uses `KillProcess` or `Errno(EPERM)` for mismatch and `Allow` for match. The examples in the crate-level docs are denylists: reversing these actions changes the policy fundamentally. An empty syscall map compiles to architecture validation followed by the mismatch action; it is not an empty BPF program. [Filter constructor and compiler](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/backend/filter.rs), [rule constructor](https://docs.rs/seccompiler/0.5.0/seccompiler/struct.SeccompRule.html), [condition generation](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/backend/condition.rs), [public functions](https://docs.rs/seccompiler/0.5.0/seccompiler/).

`Dword` also discards the high half of the supplied comparison value/mask. It does not reject a value too large for 32 bits. `Qword` equality on a kernel `int` parameter can reject values whose upper register bits the kernel ignores. Conversely, a deny condition that demands particular upper bits can miss an equivalent kernel operation. Choose width from the syscall ABI, not from the Rust variable's type; explicitly test high-bit variants and sign-extended negative arguments. There is no signed comparator. [Released condition implementation](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/backend/condition.rs).

### Rust and JSON inputs

Both frontends reach the same Rust intermediate representation and compiler. Rust callers supply **numbers**: `libc::SYS_*` is correct only for the binary's compile target. Selecting `TargetArch::aarch64` does not translate an x86_64 number. The constructor accepts a numeric map without validating that each number names the intended syscall on that target. [Released filter implementation](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/backend/filter.rs).

JSON supplies **names**, resolved using the bundled table for the explicit `TargetArch` (`x86_64`, `aarch64`, or `riscv64`). Name lookup is case-sensitive and independent of the running kernel: an unknown name fails compilation even if a newer kernel implements it; a known name can still be unavailable at runtime. JSON carries no automatic architecture detection. Comments are optional string-valued `comment` fields, not JSON syntax comments. Operators use `eq`, `ne`, `lt`, `le`, `gt`, `ge`, and `{"masked_eq": mask}`; widths are `dword`/`qword`. Parameterized actions use `{"errno": number}` or `{"trace": number}`. Legacy `default_action`/`filter_action` aliases are accepted for mismatch/match. [JSON frontend](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/frontend/json.rs), [release table update](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/CHANGELOG.md), [JSON format](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/docs/json_format.md).

Filter names such as `worker`, `main_thread`, or `vmm` are application labels. They are not kernel thread IDs, selectors, or callbacks. The launcher must validate required labels, select the correct program, and install it on every intended thread, or explicitly install one common program with TSYNC. Independent per-role policies require coordinated installation by those roles before any untrusted work. [Published compiler and installers](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/lib.rs).

### Dependencies and optional capabilities

0.5.0 declares no default feature set. Its explicit `json` feature enables optional `serde` (derive enabled) and `serde_json`; Cargo also exposes the implicit optional-dependency feature names `serde` and `serde_json`. Use `json` to enable the frontend. Rust-only construction requires only `libc` (`^0.2.153`); JSON adds `serde ^1.0.27` and `serde_json ^1.0.9`. There is no libseccomp linkage, native library build script, JSON runtime daemon, or privileged broker dependency. The program still needs its target's normal C ABI/runtime linkage and a capable Linux kernel. The crate does not serialize BPF or policies to a portable binary artifact for you. [Published dependency manifest](https://docs.rs/crate/seccompiler/0.5.0/source/Cargo.toml).

The action enum is `Allow`, `Errno(u32)`, `KillThread`, `KillProcess`, `Log`, `Trace(u32)`, and `Trap`. Errno/trace payloads are masked to 16 bits without validating a meaningful errno. `Errno(0)` prevents execution while reporting apparent success; validate policy payloads yourself. `Log` **permits** the syscall and is not an enforcing denial. `Trap` generates catchable `SIGSYS`; `KillThread` may leave other threads alive with locks held. Prefer explicit process-wide termination when termination is the intended guarantee. Process-wide killing does not kill a separately forked descendant tree. `Trace` without an eligible tracer yields `ENOSYS`; `Log` does not guarantee an audit record because logging configuration also matters. [Kernel action behavior](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html#return-values), [Action definitions and conversion](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/backend/mod.rs).

There is **no** `Notify` action, listener-FD installation, notification ioctls, public arbitrary-flags installer, action-availability query, or filter introspection API. `Trace` is ptrace mediation, not user notification. A trusted tracer can influence execution; trace-based policies demand separate security analysis. The notification interface itself warns against treating it as a security-policy mechanism: pointer inspection and continuation introduce races unless the broker design avoids them. Raw `sock_filter` instructions let a caller encode other returns, but the stock installers cannot return a `NEW_LISTENER` FD, so that does not supply a notification implementation. Linux user notification is a separate kernel facility available since 5.0. [Released API](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/lib.rs), [notification mechanism](https://man7.org/linux/man-pages/man2/seccomp_unotify.2.html).

## Installation, compatibility, and failure

### Supported machines and kernel capabilities

The released support claim is **Linux, little-endian x86_64, aarch64, and riscv64**. This excludes native macOS, native Windows, big-endian machines, and 32-bit x86/ARM as supported hosts. Compilation for a Linux target on macOS does not enable enforcement on macOS. WSL2 runs a Linux kernel, so assess it as a Linux deployment, not as native Windows support. Containers likewise use their host kernel and may inherit policies that prohibit installation. [0.5.0 platform contract](https://docs.rs/seccompiler/0.5.0/seccompiler/#supported-platforms), [Microsoft WSL architecture](https://learn.microsoft.com/en-us/windows/wsl/compare-versions).

| Requirement                                      | Kernel boundary and adoption consequence                                                                                                                                                                                      |
| ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Filter mode and `no_new_privs`                   | Linux 3.5, with `CONFIG_SECCOMP_FILTER`; this historical baseline alone is insufficient for this release's installer.                                                                                                         |
| `seccomp()` and TSYNC                            | Linux 3.17. 0.5.0 always installs through `seccomp()`, including the single-thread function. No old-`prctl` fallback.                                                                                                         |
| `KillProcess`, `Log`, `SECCOMP_GET_ACTION_AVAIL` | Linux 4.14. **Every normally compiled program includes `KillProcess` in its architecture guard**, even if the policy uses only older actions.                                                                                 |
| Unsupported return action                        | Defaults to `KillProcess` on Linux 4.14+ and `KillThread` earlier; detect required actions before starting work. `/proc/sys/kernel/seccomp/actions_avail` and `SECCOMP_GET_ACTION_AVAIL` are useful on kernels exposing them. |
| Argument inspection                              | Six raw 64-bit slots; the compiler implements 64-bit comparisons with classic BPF's 32-bit operations. This does not require eBPF or a BPF JIT.                                                                               |

[Filter-mode baseline](https://man7.org/linux/man-pages/man2/PR_SET_SECCOMP.2const.html), [kernel interface dates and actions](https://man7.org/linux/man-pages/man2/seccomp.2.html), [generated architecture guard](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/backend/bpf.rs).

These are interface introductions, not a certified minimum kernel for all three architectures. RISC-V seccomp arrived later, in the Linux 5.5 development cycle; a 3.17 kernel is not a RISC-V deployment target. Architecture backports, configuration, outer filters, and action support must be checked on the actual system. For x86_64, a maintained kernel at least 5.4 also avoids the historical x32 confusion described below; these historical version numbers are not recommendations to deploy obsolete kernels. [Linux 5.5 RISC-V configuration](https://github.com/torvalds/linux/blob/v5.5/arch/riscv/Kconfig) (absent from [5.4](https://github.com/torvalds/linux/blob/v5.4/arch/riscv/Kconfig)), [current architecture capability matrix](https://www.kernel.org/doc/html/latest/admin-guide/features.html).

### Architecture guards are necessary, but not ABI translation

The compiler prefixes a check of `seccomp_data.arch`; a mismatch returns `KILL_PROCESS`, independently of policy actions. Installation itself does **not** compare `TargetArch` to the running architecture: the kernel can accept a valid foreign-architecture filter and then kill the installer on its next syscall. The raw BPF installer adds no guard at all. Match program, executable ABI, and numeric syscall table before installation. [Guard generation](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/backend/bpf.rs), [installation source](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/lib.rs).

The x86_64 audit value is shared with x32. 0.5.0's guard has no separate `__X32_SYSCALL_BIT` rejection. A native-number allowlist normally leaves x32 numbers unmatched and denied; an allow-by-default denylist can miss their corresponding operations. Kernels before 5.4 also had problematic x32-number aliases, including 512–547. Explicitly reject unsupported ABI numbers or use a carefully reviewed allowlist and supported kernel floor. A different audit architecture, such as i386 compat, is killed by the generated guard. [ABI caveats](https://man7.org/linux/man-pages/man2/seccomp.2.html), [released guard and syscall comparisons](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/backend/bpf.rs).

### Privileges, stacking, and thread coverage

Both installers first reject a zero-length instruction slice, then set `PR_SET_NO_NEW_PRIVS`, then issue `seccomp(SECCOMP_SET_MODE_FILTER, flags, ...)`. `apply_filter_all_threads` supplies `SECCOMP_FILTER_FLAG_TSYNC`; the single-thread function supplies zero. No administrative privilege is normally required after `no_new_privs`, provided the outer environment allows these calls. Kernel rules also permit privileged installation with `CAP_SYS_ADMIN` in the relevant user namespace, but this crate still attempts `no_new_privs`. [Installer source](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/lib.rs).

`no_new_privs` is inherited and cannot be cleared; it prevents privilege gains through exec, including setuid/setgid and file capabilities. It does not drop already-held capabilities, revoke existing descriptors, or forbid every non-exec privilege operation. If installation fails after setting it, the flag remains set; there is no transactional rollback. [Kernel `no_new_privs` contract](https://docs.kernel.org/userspace-api/no_new_privs.html).

Installed filters are inherited by subsequently created children/threads and preserved across exec. Filters cannot be removed or weakened by installing another filter. Stacking runs the filter tree and uses kernel action precedence, not “last policy wins”: `KillProcess > KillThread > Trap > Errno > UserNotif > Trace > Log > Allow`; equal-action data comes from the newest relevant filter. Thus an inner `Allow` cannot override an outer container's denial, and an existing filter can prevent the new installer from running. [Kernel filter semantics](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html#return-values), [kernel implementation](https://github.com/torvalds/linux/blob/v6.12/kernel/seccomp.c).

TSYNC operates on the existing thread group atomically with respect to kernel synchronization; it does not retrofit separately forked processes. Synchronization can fail when a sibling is in strict mode or has diverged onto an incompatible filter tree. The syscall can return a positive offending TID instead of `-1`; the crate converts that to `Error::ThreadSync(tid)`. Treat it as a launch failure, never as partial success. Success does not undo work threads performed before synchronization. Coordinate thread creation and park workers until the installation verdict: a new thread inherits its creator's filters, so an unfiltered creator is a race before group installation. The simplest launcher architecture is a fresh, single-threaded helper that installs before exec. [TSYNC and filter-tree implementation](https://github.com/torvalds/linux/blob/v6.12/kernel/seccomp.c), [crate return handling](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/lib.rs).

### Error paths and release-specific sharp edges

| Stage                    | Actual behavior in 0.5.0                                                                                                                                                                             | Launcher response                                                                                        |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| JSON parsing             | Syntax/type errors, unknown fields, duplicate top-level labels, and unknown action/operator variants fail through `Error::JsonFrontend`. Unknown syscall names are target-specific errors.           | Reject the entire input; validate the expected label set too.                                            |
| Policy validation        | Index >5, empty `SeccompRule`, or identical actions return backend errors. JSON `args: []` is an empty-rule error; omitting `args` expresses an unconditional syscall entry when used alone.         | Reject before installation.                                                                              |
| Numeric Rust syscall IDs | The compiler converts `i64` to `u32` with `unwrap()`. Negative or >`u32::MAX` IDs can **panic**, not return a backend error.                                                                         | Validate external numbers before constructing policies; do not expose unrestricted numeric policy input. |
| Generated program size   | Compiler rejects length **>=4096**, so its accepted maximum is 4095 instructions; the kernel single-program ceiling is 4096.                                                                         | Surface `FilterTooLarge`; do not truncate or silently broaden.                                           |
| Raw BPF installation     | Checks nonempty but casts `len()` to `u16` without a range check. Very large raw slices can wrap to a shorter length. It also accepts caller-generated instructions without architecture validation. | Accept only compiler-produced trusted programs, or validate instruction count and guards yourself.       |
| Kernel validation        | Invalid instructions/jumps/loads, resource limits, unavailable functionality, or an outer policy can reject installation. `Error::Prctl` and `Error::Seccomp` retain OS errors.                      | Record operation plus errno and terminate the child. Do not use a permissive fallback.                   |
| TSYNC                    | Positive TID becomes `ThreadSync`; negative return becomes `Seccomp(io::Error)`.                                                                                                                     | Refuse to release any workers; terminate the process.                                                    |

[JSON validation](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/frontend/json.rs), [backend validation and size check](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/backend/filter.rs), [backend errors](https://docs.rs/seccompiler/0.5.0/seccompiler/enum.BackendError.html), [raw installer](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/lib.rs), [kernel verifier and limits](https://github.com/torvalds/linux/blob/v6.12/kernel/seccomp.c).

Kernel error diagnosis includes `EINVAL` for invalid programs/flags or unsupported configuration, `ENOSYS` where the syscall is absent, `EACCES` for unmet privilege prerequisites, and `ENOMEM` for allocation or filter-tree path limits. A filter tree has a 32768-instruction path budget with accounting overhead for existing filters, so several individually small programs can still fail. An outer seccomp policy can synthesize an errno or kill the helper instead; do not diagnose solely from kernel version. [Kernel installation validation](https://github.com/torvalds/linux/blob/v6.12/kernel/seccomp.c).

A **JSON combination pitfall observed in the released source**: entries sharing a syscall accumulate conditional rules, but an entry without `args` merely creates/looks up that syscall's vector. If unconditional and conditional entries are mixed for one syscall, the unconditional entry does not override the nonempty vector as a reader might expect from OR semantics. This report does not claim an upstream advisory for that behavior. Avoid the combination; use one unconditional entry or only conditional entries, and test the compiled meaning. [Frontend accumulation loop](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/src/frontend/json.rs).

### Documented defects and security evidence

- **Android build regression:** issue #81, opened 2025-03-14, reports missing `libc::SECCOMP_SET_MODE_FILTER` after upgrading to 0.5. The thread reports a fix merged in libc on March 18 and a semver-compatible libc release available by June 14. This is dependency/target compatibility evidence, not a filter bypass; update the resolved libc if targeting Android, and test that target separately. Android is not an additional host architecture promise. [Issue and resolution comments](https://github.com/rust-vmm/seccompiler/issues/81).
- **Invalid documentation errno:** the released JSON guide contains `{"errno": -1}` although the action expects `u32`; this input fails parsing. Monorepo commit `671e367124` on 2026-11-04 changes it to `1`; this is a documentation fix, not a released parser change. Use a positive valid errno. [Released guide](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/docs/json_format.md), [correction](https://github.com/rust-vmm/rust-vmm/commit/671e367124).
- **Advisory search:** no crate-specific directory was found at `RustSec/advisory-db/crates/seccompiler` (GitHub returned 404), and the old repository's public security-advisory page exposed no published entries in the checked view. This is a bounded search result, not a security audit, proof of absence, or assurance about transitive dependencies or the kernel. Run the application's dependency audit against its resolved lockfile and monitor the migrated repository. [RustSec database lookup](https://api.github.com/repos/RustSec/advisory-db/contents/crates/seccompiler), [repository advisory page](https://github.com/rust-vmm/seccompiler/security/advisories).

The panic, raw-length conversion, and mixed JSON behavior above are source observations applicable to the examined release; they should not be mislabeled as assigned CVEs or upstream-confirmed vulnerabilities.

## What a launcher can promise

| Requirement                          | What this crate can contribute                                                        | Missing boundary and mitigation                                                                                                                                                                                                                   |
| ------------------------------------ | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Read-only input tree                 | Deny whole classes of mutation syscalls, or numeric flags when directly visible.      | **OS filter-data limitation:** cannot read pathname strings, resolve symlinks/hard links, or distinguish an input path from a workspace path. Use a filesystem LSM such as Landlock plus a reviewed mount view.                                   |
| Writable workspace only              | Permit selected write-related syscall families.                                       | **Another mechanism needed:** bind-mount/read-only layout and/or Landlock must constrain objects. Include rename/link/truncate/metadata operations and inherited writable FDs in the design.                                                      |
| No networking                        | Deny socket creation and network syscall families as part of a complete allowlist.    | **Integration constraint:** existing sockets work through `read`/`write`, vectored I/O, `sendfile`, etc.; descriptors can arrive over IPC. Close unwanted FDs, restrict descriptor passing, and use an isolated network namespace where required. |
| Approved destinations only           | Restrict directly visible numeric socket domain/type/protocol or specific FD numbers. | **OS pointer limitation:** `connect` points to a `sockaddr`; seccomp cannot inspect address, port, DNS, TLS identity, or an FD's peer. Use an enforced egress proxy/firewall/network topology; validate application identity separately.          |
| CPU, memory, storage, process limits | Remove dangerous interfaces, not measure their consumption.                           | **Another mechanism needed:** cgroup controllers, rlimits, filesystem quotas, timeouts, and descendant cleanup. Allowed calls and pure userspace loops can still exhaust resources.                                                               |
| Kernel attack-surface reduction      | A small syscall allowlist plus ABI-appropriate argument restrictions.                 | **Underlying shared-kernel boundary:** allowed syscalls can contain vulnerabilities. For hostile code needing stronger separation, consider a VM/microVM or a sandbox runtime with a distinct kernel boundary.                                    |
| Brokered syscall decisions           | No high-level support.                                                                | **Crate API gap:** Linux user notification exists, but requires another binding or reviewed implementation with a broker threat model.                                                                                                            |

These are design conclusions from the kernel's pointer restriction and the crate's public surface, not product features claimed by seccompiler. Landlock mediates filesystem objects; namespaces separate resource views; cgroups control resources. Each needs its own capability checks and enforcement tests. Even Landlock's network rules are ABI-dependent and port-oriented rather than general DNS/TLS destination identity rules. [Kernel seccomp scope](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html#introduction), [Landlock](https://docs.kernel.org/userspace-api/landlock.html), [namespaces](https://man7.org/linux/man-pages/man7/namespaces.7.html), [cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html).

Accidental-access reduction can tolerate a broader application-specific policy if that is its stated goal. Malicious-workload confinement must assume raw syscalls, unusual flags, alternate ABIs, and reuse of every available resource. The recommendation is default-deny, a narrowly privileged helper, and independently enforced filesystem/network/resource layers. Restrict `ptrace`, process-memory interfaces, kernel object creation, namespace changes, and unexpected IPC according to the threat model; do not copy a short sample denylist as a hostile-code boundary.

Preopened descriptors identify kernel objects, not immutable identities associated with FD numbers. `dup`, `close`, subsequent opens, or `SCM_RIGHTS` can change what a number denotes; an FD equality check is safe only with a separately maintained lifecycle invariant. Existing writable mappings are another capability that filtering future `open` calls does not revoke. Filesystem aliases, mount changes, `/proc` access, Unix sockets, pipes, shared memory, and privileged helpers can carry authority or data outside the intended workspace. Audit and close resources before release, not after untrusted work begins. [Descriptor sharing semantics](https://man7.org/linux/man-pages/man7/unix.7.html), [memory mappings](https://man7.org/linux/man-pages/man2/mmap.2.html).

Cover alternative paths such as `open`/`openat`/`openat2`, `execve`/`execveat`, direct sockets versus compat multiplexers, and `io_uring` operations submitted through shared memory. Pointer-based argument structures, including `openat2` and `clone3`, cannot be filtered by reading their fields. Disallow an alternate interface when equivalent mediation cannot be guaranteed. A traced happy path is insufficient: libc versions, dynamic loaders, allocators, panic paths, signals, thread startup, and fallback paths can need additional syscalls. vDSO calls may avoid kernel entry on one machine and require a syscall fallback on another. [Upstream policy guidance](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/README.md#seccomp-best-practices), [io_uring security considerations](https://man7.org/linux/man-pages/man7/io_uring.7.html).

A practical launch sequence is: compile and validate trusted policy; create a restricted helper; establish namespaces/mounts, resource limits, credentials, descriptor inventory, and other required controls; install the final seccomp filter; only then exec or release the untrusted workload. All required setup must complete before success is reported. Permit the actual executable, loader, libraries, and runtime startup operations under the other controls. Installing too early can block setup; installing after workload execution leaves an exploitable gap. In a multithreaded supervisor, avoid allocation-heavy or locking Rust code in a post-fork pre-exec callback; prefer execing a dedicated single-threaded helper. [Rust pre-exec safety contract](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#tymethod.pre_exec).

## Complete focused Rust and JSON examples

These examples **only deny `fcntl(fd, F_GETFL)` with `EPERM`**, while permitting `F_GETFD` and all other native-ABI syscalls. This intentionally small denylist demonstrates argument matching, equivalent generation, and installation; it supplies no filesystem or networking confinement. The comparison uses `Dword` because `fcntl`'s command is an integer. Direct `libc::syscall` probes avoid a library wrapper hiding the actual operation. A valid `/dev/null` FD and pre-install probes prevent `EBADF` or an already-denying outer filter from being mistaken for success.

Create a standalone Linux Cargo project with the following files. It targets only the three documented little-endian Linux architectures; use target-specific dependencies and a different backend in a cross-platform launcher. The functions reject other runtime targets, but this project does not claim to compile seccompiler on native Windows/macOS. The helper uses `std::os::fd`, so its own source requires Rust 1.66 or later independently of the crate's unspecified MSRV. Exact tested toolchains are recorded below.

`Cargo.toml`:

```toml
[package]
name = "seccompiler-probe"
version = "0.1.0"
edition = "2021"

[dependencies]
seccompiler = { version = "=0.5.0", default-features = false, features = ["json"] }
libc = "=0.2.177"
```

`policy.json` (`3` is Linux `F_GETFL`; `1` is `EPERM`, checked by the Rust helper):

```json
{
  "worker": {
    "mismatch_action": "allow",
    "match_action": { "errno": 1 },
    "filter": [{
      "syscall": "fcntl",
      "args": [{ "index": 1, "type": "dword", "op": "eq", "val": 3 }]
    }]
  }
}
```

`src/lib.rs`:

```rust
use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp,
    SeccompCondition, SeccompFilter, SeccompRule, TargetArch,
};
use std::{error::Error, io, os::fd::RawFd};

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub fn target() -> Result<TargetArch> {
    if !cfg!(all(target_os = "linux", target_endian = "little")) {
        return Err("requires little-endian Linux".into());
    }
    match std::env::consts::ARCH {
        "x86_64" => Ok(TargetArch::x86_64),
        "aarch64" => Ok(TargetArch::aarch64),
        "riscv64" => Ok(TargetArch::riscv64),
        _ => Err("unsupported architecture".into()),
    }
}

pub fn rust_policy() -> Result<BpfProgram> {
    let rule = SeccompRule::new(vec![SeccompCondition::new(
        1, SeccompCmpArgLen::Dword, SeccompCmpOp::Eq,
        libc::F_GETFL as u64,
    )?])?;
    Ok(SeccompFilter::new(
        [(libc::SYS_fcntl, vec![rule])].into_iter().collect(),
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        target()?,
    )?.try_into()?)
}

pub fn json_policy() -> Result<BpfProgram> {
    if libc::F_GETFL != 3 || libc::EPERM != 1 {
        return Err("JSON constants do not match target ABI".into());
    }
    let mut map = seccompiler::compile_from_json(
        include_bytes!("../policy.json").as_slice(), target()?,
    )?;
    map.remove("worker").ok_or_else(|| "missing worker filter".into())
}

fn fcntl(fd: RawFd, command: libc::c_int) -> io::Result<libc::c_long> {
    let value = unsafe { libc::syscall(libc::SYS_fcntl, fd, command, 0) };
    if value == -1 { Err(io::Error::last_os_error()) } else { Ok(value) }
}

pub fn preflight(fd: RawFd) -> Result<()> {
    fcntl(fd, libc::F_GETFD)?;
    fcntl(fd, libc::F_GETFL)?;
    Ok(())
}

pub fn probe(fd: RawFd) -> Result<()> {
    fcntl(fd, libc::F_GETFD)?;
    match fcntl(fd, libc::F_GETFL) {
        Err(e) if e.raw_os_error() == Some(libc::EPERM) => Ok(()),
        other => Err(format!("expected EPERM, got {other:?}").into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equivalent_generation() -> Result<()> {
        let fields = |p: BpfProgram| p.into_iter()
            .map(|i| (i.code, i.jt, i.jf, i.k)).collect::<Vec<_>>();
        assert_eq!(fields(rust_policy()?), fields(json_policy()?));
        assert!(SeccompCondition::new(
            6, SeccompCmpArgLen::Dword, SeccompCmpOp::Eq, 0,
        ).is_err());
        assert!(seccompiler::compile_from_json(
            br#"{"worker":{"match_action":"allow","mismatch_action":"wat","filter":[]}}"#.as_slice(),
            target()?,
        ).is_err());
        Ok(())
    }
}
```

`src/main.rs`, for a fresh, single-threaded child:

```rust
use seccompiler_probe::{json_policy, preflight, probe, rust_policy, Result};
use std::{fs::File, os::fd::AsRawFd};

fn run() -> Result<()> {
    let policy = match std::env::args().nth(1).as_deref() {
        Some("rust") => rust_policy()?,
        Some("json") => json_policy()?,
        _ => return Err("usage: seccompiler-probe rust|json".into()),
    };
    let file = File::open("/dev/null")?;
    preflight(file.as_raw_fd())?;
    seccompiler::apply_filter(&policy)?;
    probe(file.as_raw_fd())?;
    println!("allowed F_GETFD; denied F_GETFL with EPERM");
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("refusing workload startup: {e}");
        std::process::exit(1);
    }
}
```

Any compilation, preflight, missing-label, or installation error reaches `main`, reports failure, and exits before the probe/workload entry. The only work after successful installation is the stated verification; replace it with a workload only after every other required isolation layer is also installed. For a deny-by-default production policy, allow whatever error reporting and termination need, or use a minimal raw exit path; `eprintln!` is not universally safe under an arbitrary filter.

### Separate all-thread installation

`src/bin/tsync.rs` reuses the library and Cargo configuration above. An already-existing worker proves it can make both calls, reports readiness, and waits. The main thread requires TSYNC success before releasing it. Both threads must then observe denial. No additional Cargo feature is needed for TSYNC. If synchronization fails, `main` exits the process rather than leaving the waiting worker running or continuing with `apply_filter`.

```rust
use seccompiler_probe::{preflight, probe, rust_policy, Result};
use std::{fs::File, os::fd::AsRawFd, sync::mpsc, thread};

fn run() -> Result<()> {
    let policy = rust_policy()?;
    let file = File::open("/dev/null")?;
    preflight(file.as_raw_fd())?;
    let (ready_tx, ready_rx) = mpsc::channel();
    let (go_tx, go_rx) = mpsc::channel();
    let worker = thread::spawn(move || -> Result<()> {
        preflight(file.as_raw_fd())?;
        ready_tx.send(())?;
        go_rx.recv()?;
        probe(file.as_raw_fd())
    });
    ready_rx.recv()?;
    seccompiler::apply_filter_all_threads(&policy)?;
    go_tx.send(())?;
    let local = File::open("/dev/null")?;
    probe(local.as_raw_fd())?;
    worker.join().map_err(|_| "worker panicked")??;
    println!("both existing threads enforced the filter");
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("refusing workload startup: {e}");
        std::process::exit(1);
    }
}
```

The channel/thread operations remain permitted by this deliberately broad policy. A production allowlist must account for their futex, allocation, teardown, and signal requirements. This example covers two parked existing threads; it is not a demonstration that uncoordinated concurrent thread creation is safe.

### Verification commands and evidence

On a disposable Linux child process, with a mounted `/dev/null`, Cargo, and cargo-nextest installed:

```sh
cargo nextest run --locked
cargo build --locked --bins
```

Generate `Cargo.lock` once with `cargo generate-lockfile` before using `--locked`; retain it for reproducibility. The generation test never installs a filter. It compares instruction fields from both frontends and checks invalid argument indices and an invalid JSON action. It proves a narrow policy-generation property, not runtime enforcement.

Run the following **unrestricted Python supervisor** separately. Each invocation starts a fresh child; the parent never installs seccomp, enforces a deadline, and verifies both exit status and the expected probe result:

```python
import subprocess

cases = [
    (["target/debug/seccompiler-probe", "rust"],
     "allowed F_GETFD; denied F_GETFL with EPERM"),
    (["target/debug/seccompiler-probe", "json"],
     "allowed F_GETFD; denied F_GETFL with EPERM"),
    (["target/debug/tsync"], "both existing threads enforced the filter"),
]
for argv, expected in cases:
    result = subprocess.run(argv, capture_output=True, text=True, timeout=15)
    assert result.returncode == 0, (argv, result.returncode, result.stderr)
    assert result.stdout.strip() == expected, (argv, result.stdout)

bad = subprocess.run(["target/debug/seccompiler-probe", "invalid"],
                     capture_output=True, text=True, timeout=15)
assert bad.returncode == 1
assert "refusing workload startup" in bad.stderr
assert not bad.stdout
```

**Observed 2026-11-04:** all Rust sources above were cross-compile-checked (`cargo check --target x86_64-unknown-linux-gnu --all-targets`) from **macOS 27.2 arm64, rustc 1.98.1 (48a229cea 2026-09-01)**. They were then built and the generation test passed through **cargo-nextest 0.9.140** on a separate **x86_64 Linux 7.0.14-25-pve** host with **rustc 1.97.1 (8bab26f4f 2026-07-14)**. Both per-thread modes and the TSYNC binary executed successfully under 15-second external deadlines. The tested dependency resolution included libc 0.2.177 and seccompiler 0.5.0. The Python supervisor also passed all four child cases, including the invalid-argument startup refusal. These are observed results for that environment; aarch64 Linux, riscv64, old kernels, containers, and WSL2 were not runtime-tested. Cross-compilation alone establishes no enforcement.

Additional adoption tests should keep the parent unrestricted and run one irreversible security change per fresh child:

| Test                                                                             | Required assertion                                                                                                                                             | Status in this research                                                                             |
| -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Focused allow/deny and equivalent generation                                     | `F_GETFD` succeeds, `F_GETFL` changes from success to `EPERM`; generated programs agree.                                                                       | Executed as above.                                                                                  |
| Existing-thread synchronization                                                  | Both already-existing threads observe denial after the shared release gate.                                                                                    | Executed as above.                                                                                  |
| Bad launch arguments                                                             | Nonzero child status, failure diagnostic, no workload output.                                                                                                  | Executed with the Python supervisor above.                                                          |
| Missing label, malformed JSON, unsupported operator, `args: []`, oversize policy | Compilation fails and a separate workload-start marker is never produced.                                                                                      | Only invalid action/index are covered by the executed generation test; other cases are recommended. |
| Empty/raw invalid BPF                                                            | `EmptyFilter` or verifier error; workload marker absent.                                                                                                       | Recommended isolated failure-injection test.                                                        |
| TSYNC incompatibility                                                            | Create divergent sibling filter trees inside the child; assert `ThreadSync` and no workload release.                                                           | Recommended; not executed.                                                                          |
| Unavailable actions/syscall or blocked `prctl`/`seccomp`                         | Installation or capability gate fails; parent classifies an unavailable requirement separately from a policy regression, but deployment still refuses startup. | Recommended on actual container/WSL2 fleet.                                                         |
| Wrong ABI, kill action, inheritance through exec/clone                           | Parent checks child signal/status and descendant results, not just a pre-install “ready” message.                                                              | Recommended in a disposable VM/container with bounded descendant cleanup.                           |

Capability discovery is diagnostic: `/proc/.../status` showing seccomp mode 2 does not identify which policy is installed. A negative syscall probe must observe the intended action after successful installation and compare against a pre-install baseline. For probes that spawn descendants, place the whole test tree in a disposable containment boundary and reap/kill it on timeout; Python's simple timeout above is sufficient only for these no-descendant examples. Do not install filters in the test runner, change host sysctls, or disable outer container security merely to make a test pass.

## Alternatives and complementary layers

| Option                                               | Relationship and coverage                                                                                                                                                                          | Integration and maintenance considerations                                                                                                                                                                                                                                                                                   |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `seccompiler` 0.5.0                                  | Rust policy compiler/installer for three little-endian Linux architectures.                                                                                                                        | Small dependency surface; JSON optional; caller owns syscall inventory, ABI correctness, capability checks, and thread coverage. One registry release in the last 24 months; active work migrated to rust-vmm.                                                                                                               |
| `libseccomp` 0.4.0 / `libseccomp-sys`                | Competing safe/unsafe Rust bindings to the native C libseccomp library, not wrappers over seccompiler. Linux enforcement; broader native architecture/ABI abstraction and syscall-name resolution. | Safe binding exposes notification facilities and richer filter configuration. Requires native libseccomp **2.5.0+**, including development/linking setup; static musl packaging needs a suitable native archive. Latest stable Rust-binding publication is **2025-04-05**. Native and binding versions must both be managed. |
| `extrasafe` 0.5.1                                    | Higher-level Linux rule sets over **seccompiler ^0.4**, not an independent kernel boundary or a drop-in 0.5 frontend.                                                                              | Easier capability-oriented rules; optional Landlock and isolation features. Published **2024-04-16**; latest registry stable on the research date. Audit rule-set breadth and its older backend dependency rather than assuming current seccompiler behavior or notification support.                                        |
| Direct classic BPF plus raw Linux calls              | Competing implementation approach; platform support is whatever ABIs the implementer correctly handles.                                                                                            | No required policy library; maximum flexibility for flags and new actions. Highest review/test cost: guards, jumps, numeric widths, verifier limits, and error conventions become local security code.                                                                                                                       |
| Landlock                                             | Complementary Linux LSM; object-based filesystem restrictions and ABI-dependent networking restrictions.                                                                                           | Query ABI and require all rights material to the promise. A port permission is not a destination identity policy. Does not replace syscall attack-surface reduction.                                                                                                                                                         |
| Namespaces plus mounts, credentials, cgroups/rlimits | Complementary Linux resource views and containment controls.                                                                                                                                       | More setup and privilege/delegation constraints. Use to shape visible objects, remove authority, and bound resources; still a shared kernel.                                                                                                                                                                                 |
| VM/microVM or another isolated-kernel runtime        | Stronger boundary for hostile workloads, with syscall filtering still useful inside or around the VMM.                                                                                             | Greater operational and startup cost; review host exposure, mounts, networking, and control-plane privileges separately.                                                                                                                                                                                                     |

Sources: [libseccomp 0.4 API](https://docs.rs/libseccomp/0.4.0/libseccomp/), [native requirements](https://github.com/libseccomp-rs/libseccomp-rs#requirements), [binding publication records](https://crates.io/api/v1/crates/libseccomp), [extrasafe manifest](https://github.com/boustrophedon/extrasafe/blob/v0.5.1/Cargo.toml), [extrasafe API](https://docs.rs/extrasafe/0.5.1/extrasafe/), [extrasafe publication records](https://crates.io/api/v1/crates/extrasafe), [Landlock](https://docs.kernel.org/userspace-api/landlock.html), [namespace types](https://man7.org/linux/man-pages/man7/namespaces.7.html), [resource controllers](https://docs.kernel.org/admin-guide/cgroup-v2.html).

For a launcher already generating explicit architecture-specific allowlists, seccompiler avoids a native libseccomp packaging dependency and keeps the compiler inspectable. Choose libseccomp bindings when broad ABI handling, native syscall resolution, or notification is a hard requirement. Choose extrasafe when its audited higher-level rules match the workload and its dependency/platform limits are acceptable. None supplies a portable native macOS/Windows sandbox backend merely by being written in Rust.

## Authoritative resources and adoption questions

- [Pinned seccompiler 0.5.0 API](https://docs.rs/seccompiler/0.5.0/seccompiler/) and [published package files](https://docs.rs/crate/seccompiler/0.5.0/source/).
- [Package/version metadata](https://crates.io/api/v1/crates/seccompiler), [release changelog](https://github.com/rust-vmm/seccompiler/blob/v0.5.0/CHANGELOG.md), and [current rust-vmm component](https://github.com/rust-vmm/rust-vmm/tree/main/seccompiler).
- [Linux seccomp interface](https://man7.org/linux/man-pages/man2/seccomp.2.html), [kernel filtering guide](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html), and [user notification](https://man7.org/linux/man-pages/man2/seccomp_unotify.2.html).
- [Kernel no_new_privs](https://docs.kernel.org/userspace-api/no_new_privs.html), [Landlock](https://docs.kernel.org/userspace-api/landlock.html), and [cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html).

Material questions before adoption:

1. Which exact executable ABIs, kernel configurations, container profiles, and WSL2 builds must enforce the policy? Have all required actions and TSYNC behavior been tested there?
2. Can the launcher use a fresh single-threaded helper, or must it synchronize multiple roles without releasing unfiltered workers? How will it handle preexisting descendants?
3. What complete allowlist supports runtime startup, error paths, and teardown while excluding alternate syscall interfaces? What authority is retained in descriptors and mappings?
4. Which independent mechanism enforces filesystem object permissions, approved destination identity, and resource ceilings, and does startup fail when any required capability is unavailable?
5. Is notification or public syscall-table access required? If so, adopt another released API or explicitly review a fork; do not rely on unreleased main behavior as if it were 0.5.0.
6. What toolchain/MSRV and maintainer-response expectations are required? The package publishes no MSRV, and continued commits do not establish a release or security-support schedule.
