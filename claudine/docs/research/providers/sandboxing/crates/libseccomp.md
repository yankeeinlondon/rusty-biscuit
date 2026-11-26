---
prompt: |-
    Research the Rust `libseccomp` crate to help a developer decide whether
    and how to use it in the syscall-filtering layer of a Linux sandbox launcher.

    Research the Rust package named libseccomp, distinguishing its safe
    bindings, the libseccomp-sys package, and the native libseccomp C library.
    Architecture portability within Linux must not be presented as native
    macOS or Windows sandbox support.

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
    - https://docs.rs/libseccomp/
    - https://github.com/seccomp/libseccomp

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
       - Explain filter contexts, default and matching actions, syscall lookup,
         argument comparisons, architecture selection, filter export, and load.
       - Describe rule composition, action precedence, stacked filters, thread
         synchronization, no_new_privs, privilege requirements, and inheritance.
       - Determine whether notification, logging, API/version queries, and filter
         attributes are exposed in the examined Rust release.
       - List Cargo features, defaults, native library/header requirements,
         linking options, and the relationship to libseccomp-sys.

    3. Platform integration, compatibility, and failure behavior
       - Separate the Rust binding version, native libseccomp version/API level,
         and Linux kernel support. Map important actions and attributes to all
         relevant requirements, using runtime queries where available.
       - Explain syscall name/number translation, native and additional ABIs,
         architecture checks, multiplexed syscalls, and unknown syscall handling.
       - Trace filter construction, native-library errors, verifier rejection,
         thread synchronization failures, and unavailable kernel actions.
       - Explain build and deployment constraints for static/dynamic linking,
         cross-compilation, containers, and WSL2; identify runtime requirements
         beyond successful compilation.
       - Show how to refuse launch if a required filter or attribute cannot be
         installed. Identify any Rust API gap needing lower-level bindings.
       - If notification is exposed, explain listener ownership, notification
         validation, broker trust, races, and continue-versus-emulate semantics.

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
       - Explain why numeric syscall-argument comparisons cannot safely inspect
         pointed-to paths, socket addresses, or mutable userspace memory.
       - Investigate alternate syscalls/ABIs, preopened resources, thread races,
         filter size limits, action behavior, and unsafe notification assumptions.
       - Distinguish syscall surface reduction from path-based filesystem policy,
         destination allowlists, resource limits, and a complete sandbox.

    5. Rust examples and verification
       - Demonstrate a focused syscall restriction with explicit default action,
         an argument comparison, and process-wide installation where supported.
       - Include native library setup, explicit ABI handling, meaningful error
         propagation, and allowed/denied child-process probes.
       - If useful, show notification as a separate optional broker example,
         stating its trust boundary and limitations.
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
       Compare seccompiler's Rust filter construction with the native-library
       binding, extrasafe's higher-level rule sets, and complementary Landlock
       or namespace isolation. Explain maintenance, deployment, and API tradeoffs.
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
hash: 547d74c4e84d07d5-972855b475e98d2f
last_updated: 2026-11-04
---

# libseccomp for a Linux sandbox launcher

**Assessment:** `libseccomp` is a suitable low-level Rust interface for building and installing Linux syscall filters when a launcher can ship and maintain the native C library. It offers safe filter construction, ABI translation, thread synchronization, exports, and user notification. Use it as one confinement layer: it cannot express path permissions or IP destination allowlists, revoke existing resources, or provide a complete sandbox. Prefer a tested default-deny policy for hostile workloads; the default-allow example below demonstrates one narrow restriction only. [Published API](https://docs.rs/libseccomp/0.4.0/libseccomp/), [kernel scope statement](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html).

**Research date: November 4, 2026. Examined stable Rust crate: `libseccomp` 0.4.0, published April 5, 2025 (UTC).** Its supported native-library floor is 2.5.0; my deployment recommendation is **native libseccomp 2.6.1 or a distribution build carrying all three July 2026 security fixes**. These are independent version requirements. The example deliberately requires upstream 2.6.1 rather than attempting to recognize vendor backports. [Registry metadata](https://crates.io/api/v1/crates/libseccomp), [published requirements](https://docs.rs/crate/libseccomp/0.4.0/source/README.md), [native 2.6.1 release](https://github.com/seccomp/libseccomp/releases/tag/v2.6.1).

## Identity, release history, and maintenance

| Component              | Identity and role                                                                                         | Ownership/license                                                                    |
| ---------------------- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `libseccomp` 0.4.0     | Published safe Rust wrapper; repository is **libseccomp-rs/libseccomp-rs**, not the starting C repository | Published author and registry owner: Manabu Sugimoto (`ManaSugi`); MIT OR Apache-2.0 |
| `libseccomp-sys` 0.3.0 | Raw, unsafe FFI declarations in the same Rust repository; dependency of the wrapper (`^0.3.0`)            | MIT OR Apache-2.0; checked-in bindings, not a bundled native implementation          |
| Native libseccomp      | C library that translates rules into classic BPF and interacts with Linux; **seccomp/libseccomp**         | LGPL-2.1; upstream release-signing identities include Paul Moore and Tom Hromatka    |
| Linux seccomp          | Kernel enforcement mechanism, not a Cargo dependency                                                      | Requires kernel configuration and runtime permissions                                |

The metadata links the Rust packages to their repository; published source and repository releases corroborate that identity. Rust sources carry Sony Group Corporation copyright notices, but this does not imply that the Rust repository and the upstream seccomp organization are one project or offer a support contract. [Rust manifest](https://docs.rs/crate/libseccomp/0.4.0/source/Cargo.toml), [sys manifest](https://docs.rs/crate/libseccomp-sys/0.3.0/source/Cargo.toml), [registry owners](https://crates.io/api/v1/crates/libseccomp/owners), [C project](https://github.com/seccomp/libseccomp).

Registry observations for the last 24 months, November 4, 2024–November 4, 2026:

| Date (UTC) | Release                                            | Adoption significance                                                                                                                         |
| ---------- | -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| 2025-04-05 | Rust `libseccomp` 0.4.0 and `libseccomp-sys` 0.3.0 | Only Rust releases in that window; Rust 2021, declared MSRV 1.67, native library floor raised to 2.5.0; additional architectures and 2.6 APIs |
| 2025-01-24 | Native 2.6.0 and maintenance 2.5.6                 | Separate C release lines; 2.6 adds APIs used by the wrapper's conditional build                                                               |
| 2026-07-01 | Native 2.6.1                                       | Security and correctness fixes described below; latest stable C release observed                                                              |

Older Rust milestones: 0.1.0 on April 18, 2021; 0.2.0 on December 31, 2021; 0.2.3 on February 14, 2022 added bad-architecture and additional attribute accessors; 0.3.0 on October 1, 2022 changed notification types and API querying. All ten `libseccomp` versions returned by the registry were non-yanked, ordinary releases; no prerelease appeared. The sys registry likewise returned no yanked or prerelease version. This is an observation of these registry responses, not a prediction about future releases. [Rust versions](https://crates.io/api/v1/crates/libseccomp), [sys versions](https://crates.io/api/v1/crates/libseccomp-sys), [Rust release notes](https://github.com/libseccomp-rs/libseccomp-rs/releases), [C releases](https://github.com/seccomp/libseccomp/releases).

Migration from 0.3 requires reviewing builder return types, architecture-add/remove behavior, IO-safe export arguments (`AsFd`), the newer bitflags dependency, and Rust/native toolchain requirements. `new_filter` remains a deprecated alias of `new` in **published source**. The release notes describe a rename and say architecture methods return `Result<()>`; the published 0.4.0 signatures actually return `Result<&mut Self>`. Use those signatures as the authority. `check_version` also changed comparison semantics when the actual native major version exceeds the requested major. The README says MSRV increases are not breaking changes; pin dependencies and test an MSRV lockfile rather than assuming every future compatible resolution builds on 1.67. [Released filter source](https://docs.rs/crate/libseccomp/0.4.0/source/src/filter_context.rs), [version comparison](https://docs.rs/crate/libseccomp/0.4.0/source/src/version.rs), [v0.4.0 notes](https://github.com/libseccomp-rs/libseccomp-rs/releases/tag/v0.4.0).

Observable maintenance: the Rust repository's latest five commits include two CI dependency updates dated October 14, 2026, followed by the April 2025 release work. Contributors include ManaSugi and rusty-snake. No new Rust version was published between April 2025 and this research date. **Judgment:** a small, intermittently updated binding project with a usable released API; the dates alone establish neither abandonment nor a security response guarantee. Pre-1.0 numbering alone is not evidence of unreliability. The native project has its own maintainers, releases, advisories, and non-calendar release process. Post-release repository commits are not functionality in the examined crate. [Rust history](https://api.github.com/repos/libseccomp-rs/libseccomp-rs/commits?per_page=5), [contributors](https://api.github.com/repos/libseccomp-rs/libseccomp-rs/contributors?per_page=5), [native release process](https://github.com/seccomp/libseccomp/blob/main/README.md).

## Public API and policy semantics

`ScmpFilterContext::new(default_action)` allocates a native context, initially containing the native ABI. `ScmpAction` includes `Allow`, `Errno(i32)`, `Trap`, `KillThread`, `KillProcess`, `Trace(u16)`, `Log`, and `Notify`. The default applies when no rule matches. `Errno` suppresses the call and fabricates a result; `Log` permits it. `Trap` produces SIGSYS; killing only the calling thread can leave shared process state damaged. Use process termination where that is the intended fatal policy. Validate errno payloads yourself: the Rust conversion casts `i32` to `u16`, despite documentation specifying a 0–65535 range. An invalid or zero payload can have surprising application semantics. [Action source](https://docs.rs/crate/libseccomp/0.4.0/source/src/action.rs), [native initialization](https://man7.org/linux/man-pages/man3/seccomp_init.3.html).

| Task                | Released interface and important semantics                                                                                                                            |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Resolve syscalls    | `ScmpSyscall::from_name`, `from_name_by_arch`, `from_name_by_arch_rewrite`, `get_name`, `get_name_by_arch`; raw conversion and `is_error`/`is_undef` are also exposed |
| Add rules           | `add_rule`, `add_rule_conditional`, and their `_exact` variants; separate rules form alternatives, comparisons within one rule are ANDed                              |
| Compare arguments   | `ScmpArgCompare::new(index, op, datum)` or `scmp_cmp!`; six zero-based argument slots, equality/inequality/order comparisons and `MaskedEqual(mask)`                  |
| Select ABIs         | `ScmpArch::native`, `is_arch_present`, `add_arch`, `remove_arch`; `set_act_badarch` controls unmatched architectures                                                  |
| Export              | `export_pfc` for human inspection; `export_bpf` for kernel-readable bytes; both accept `AsFd`. `export_bpf_mem` needs build-time detection of native 2.6              |
| Install             | `load() -> Result<()>`; enforcement begins on successful return, not when rules are added or exported                                                                 |
| Manage construction | `reset`, `merge`, syscall priority and optimization; 2.6-conditional `precompute`, `start_transaction`, `commit_transaction`, `reject_transaction`                    |
| Query capabilities  | `ScmpVersion::current`, `check_version`, `get_api`, `check_api`; `set_api` forcibly overrides the detected level and should not be used to pretend capabilities exist |
| Configure behavior  | Generic `get_filter_attr`/`set_filter_attr` plus typed convenience methods                                                                                            |
| Notify              | `get_notify_fd`, `ScmpNotifReq::receive`, `ScmpNotifResp::{new_val,new_error,new_continue,respond}`, `notify_id_valid`, response flags                                |

These are released interfaces; the tables are not a transcription of unreleased C headers. The context is neither `Send` nor `Sync`. Build and install it in its owning thread; TSYNC is a kernel operation and does not require moving the Rust context between threads. [0.4.0 API](https://docs.rs/libseccomp/0.4.0/libseccomp/), [argument comparison source](https://docs.rs/crate/libseccomp/0.4.0/source/src/arg_compare.rs), [syscall source](https://docs.rs/crate/libseccomp/0.4.0/source/src/syscall.rs), [context auto traits](https://docs.rs/libseccomp/0.4.0/libseccomp/struct.ScmpFilterContext.html#synthetic-implementations).

Rules are a union, not an ordered firewall list. A broad unconditional allowance can subsume conditional allowances; do not rely on insertion order as an exception mechanism. A single rule may compare each argument only once; express alternatives as separate rules. Exact rules reject transformations that the ordinary rule API may perform for architecture-specific syscall multiplexing. `merge` combines compatible construction contexts with disjoint architectures; it is distinct from loading multiple kernel filters. [Native rule semantics](https://man7.org/linux/man-pages/man3/seccomp_rule_add.3.html), [merge contract](https://man7.org/linux/man-pages/man3/seccomp_merge.3.html).

For stacked kernel filters, action precedence is, strongest first: **KillProcess, KillThread, Trap, Errno, Notify, Trace, Log, Allow**. Equal-precedence results take data from the newest matching filter. A later allowance cannot override an earlier denial. Context reset, transactions, and context destruction cannot remove an installed filter. Loading further filters only adds restrictions under these semantics. TRACE is not an approval service: a ptracer is part of the trust boundary, and pre-4.8 kernel behavior is particularly hazardous. [Kernel action ordering](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html#return-values), [loading and stacking](https://man7.org/linux/man-pages/man3/seccomp_load.3.html), [ptrace caveats](https://man7.org/linux/man-pages/man2/seccomp.2.html#DESCRIPTION).

Ordinary load affects the calling thread. `set_ctl_tsync(true)` requests synchronization across its thread group and must fail if synchronization cannot complete, for example because another thread has diverged to an incompatible filter tree. It does not reach already existing child processes. Newly created children inherit filters; filters survive exec if those operations remain permitted. Prefer a fresh, single-threaded launcher helper that performs isolation before executing untrusted code. Never run untrusted work concurrently while constructing its future filter. [TSYNC attribute](https://man7.org/linux/man-pages/man3/seccomp_attr_set.3.html), [kernel inheritance](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html#usage).

`CtlNnp` defaults to true: libseccomp sets `no_new_privs` before loading. This irreversible, inherited bit prevents exec from granting privileges through set-ID bits or file capabilities; it is not a general capability drop. Installation otherwise requires `CAP_SYS_ADMIN` in the caller's user namespace unless NNP was already set. A failed load can still have set NNP, so do not reuse a partially configured child as an ordinary unsandboxed worker. [NNP semantics](https://man7.org/linux/man-pages/man2/PR_SET_NO_NEW_PRIVS.2const.html), [privilege requirement](https://man7.org/linux/man-pages/man2/seccomp.2.html#ERRORS).

## Three compatibility axes and build/deployment requirements

The Rust release, native shared/static library, and running kernel are independent. A new header or a compiled Rust executable does not grant a kernel feature. Native API levels summarize probed kernel capabilities and are cached; `check_api(level, version)` checks both the level and the C version. Always propagate actual attribute-setting and loading errors too. [Released query implementation](https://docs.rs/crate/libseccomp/0.4.0/source/src/api.rs), [native API levels](https://man7.org/linux/man-pages/man3/seccomp_api_get.3.html).

| Capability                                            | Native library introduction / API level                 | Kernel baseline and Rust 0.4.0 behavior                                                                                                |
| ----------------------------------------------------- | ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Basic Allow, Errno, Trap, KillThread, Trace filtering | Supported by the 2.5.0 binding floor; base API 1        | seccomp filter mode since Linux 3.5, with seccomp filter configuration enabled                                                         |
| NNP                                                   | Supported by the binding floor                          | Linux 3.5; default on; no standalone isolation guarantee                                                                               |
| TSYNC (`CtlTsync`)                                    | Native 2.2.0, API ≥2                                    | Linux 3.17; typed setter checks level/version; required process-wide load errors must abort launch                                     |
| Log action / `CtlLog`                                 | Native 2.4.0, API ≥3                                    | Linux 4.14; audit settings affect actual logs; logging is not denial                                                                   |
| KillProcess                                           | Native 2.4.0                                            | Linux 4.14; query action support when required, not merely enum availability                                                           |
| `CtlSsb`                                              | Native 2.4.0, API ≥4                                    | Linux 4.17 SPEC_ALLOW; disables a speculative-store-bypass mitigation, not extra sandbox protection                                    |
| Native Notify alone                                   | Native 2.4.0, API ≥5                                    | Linux 5.0 listener/USER_NOTIF support                                                                                                  |
| Rust notification helpers and TSYNC + Notify          | Wrapper requires native ≥2.5.0 and API ≥6               | Linux 5.7 TSYNC_ESRCH support; wrapper is stricter than native API 5 notification availability                                         |
| Notification CONTINUE / ADDFD                         | Kernel functionality is independent of listener support | CONTINUE since Linux 5.5, ADDFD since 5.9; Rust exposes CONTINUE but no safe ADDFD helper                                              |
| `CtlWaitkill`                                         | Native ≥2.6.0, API ≥7                                   | Linux 5.19 WAIT_KILLABLE_RECV; typed setter checks both; request applies when possible                                                 |
| `CtlOptimize`, `ApiSysRawRc`, `ApiTskip`              | Library-side controls supported by the binding floor    | No new kernel permission granted; optimize changes generated BPF, raw RC preserves system failures, tskip permits rules for syscall -1 |
| BPF-memory export, precompute, transactions           | Native ≥2.6.0 and build-time `pkg-config` detection     | Construction APIs, not kernel filter replacement; docs.rs may show them even if absent from a local build                              |

Sources for the matrix: [native versioned changelog](https://github.com/seccomp/libseccomp/blob/v2.6.1/CHANGELOG), [API level contract](https://github.com/seccomp/libseccomp/blob/v2.6.1/doc/man/man3/seccomp_api_get.3), [Rust attribute implementation](https://docs.rs/crate/libseccomp/0.4.0/source/src/filter_context.rs), [kernel UAPI definitions](https://github.com/torvalds/linux/blob/v6.12/include/uapi/linux/seccomp.h), [notification history](https://man7.org/linux/man-pages/man2/seccomp_unotify.2.html).

All ten `ScmpFilterAttr` variants are exposed: `ActDefault` (read-only), `ActBadArch`, `CtlNnp`, `CtlTsync`, `ApiTskip`, `CtlLog`, `CtlSsb`, `CtlOptimize`, `ApiSysRawRc`, `CtlWaitkill`. Defaults include native ABI only, bad-architecture KillThread, NNP on, TSYNC/log/SSB/waitkill off, optimization level 1, and raw return codes/tskip off. The published `set_api_tskip` prose incorrectly describes logging; its implementation sets `ApiTskip`. Treat this as documentation drift, not a logging feature. [Attribute enum](https://docs.rs/crate/libseccomp/0.4.0/source/src/filter_attr.rs), [native defaults](https://man7.org/linux/man-pages/man3/seccomp_attr_set.3.html), [released setter](https://docs.rs/crate/libseccomp/0.4.0/source/src/filter_context.rs).

There is one optional Cargo feature, **`const-syscall`**, which enables `cfg-if` and compile-time syscall definitions updated through native 2.6.0. There is no default feature set, vendored-C feature, or static-link Cargo feature. Runtime name lookup uses the linked native table; constants use the crate's compiled table. Neither table proves that a named syscall is implemented by the running kernel. [Feature manifest](https://docs.rs/crate/libseccomp/0.4.0/source/Cargo.toml), [constant implementation](https://docs.rs/crate/libseccomp/0.4.0/source/src/syscall.rs).

For a Debian-based build image, provision `libseccomp-dev`, `pkg-config`, and a Rust/C linker toolchain; Fedora uses `libseccomp-devel`. Native runtime packages must contain a patched library as discussed above. The sys crate has checked-in declarations, so building the Rust wrapper itself does **not** run bindgen or require libclang or parse `seccomp.h`; development packages supply the linkable library and `.pc` file, and headers are needed when building native/C code. The wrapper build script probes `pkg-config` for 2.6 to enable selected methods but does not hard-fail on an unsuccessful probe. A successful check without native metadata is therefore not a compatibility audit. [Wrapper build script](https://docs.rs/crate/libseccomp/0.4.0/source/build.rs), [sys build script](https://docs.rs/crate/libseccomp-sys/0.3.0/source/build.rs), [published setup](https://docs.rs/crate/libseccomp/0.4.0/source/README.md).

Dynamic linking is the sys default: deploy the compatible `libseccomp.so.2` and loader/runtime dependencies with the executable. For static linking set `LIBSECCOMP_LINK_TYPE=static` and `LIBSECCOMP_LIB_PATH` to the directory containing a target-compatible `libseccomp.a`. For musl, use a musl-built native archive, not a glibc archive. Review LGPL redistribution obligations independently of the wrapper's permissive license. `LIBSECCOMP_LIB_PATH` also causes the wrapper build script to use its `pkgconfig` subdirectory. Ensure that `.pc` metadata and archives describe the target: the script enables cross probing when HOST differs from TARGET, which does not prevent accidental host-library selection. Set target linker, sysroot and pkg-config environment explicitly; test the shipped artifact. [Linking implementation](https://docs.rs/crate/libseccomp-sys/0.3.0/source/build.rs), [cross-build implementation](https://docs.rs/crate/libseccomp/0.4.0/source/build.rs).

This is **Linux-only enforcement**, including Linux workloads in containers or WSL2. Architecture portability refers to Linux ABIs. Native macOS and Windows need other backends. Containers use their host's kernel and may already have a filter that blocks `seccomp`, `prctl`, a required action, or a probe; an inner filter cannot relax it. WSL2 runs a Linux kernel, but its actual kernel configuration, runtime policy, Windows interoperability and exposed host resources must be assessed separately. A WSL2 filter is not a native Windows process policy. No WSL2 or macOS enforcement test was performed for this report. [C project scope](https://github.com/seccomp/libseccomp), [Microsoft WSL architecture](https://learn.microsoft.com/en-us/windows/wsl/about), [kernel configuration](https://github.com/torvalds/linux/blob/v6.12/arch/Kconfig).

## ABI translation and failure behavior

Use `ScmpSyscall::from_name` for rule insertion: libseccomp expects identifiers in the **native ABI's numbering** and translates them into every architecture already present in the context. `from_name_by_arch` is useful for inspection, but passing a foreign ABI's number to ordinary `add_rule*` is explicitly warned against. Add all desired architectures **before** adding rules; newly added architectures do not retroactively acquire existing rules. Keep unsupported ABIs excluded and set an explicit bad-architecture action. [Rust warning](https://docs.rs/libseccomp/0.4.0/libseccomp/struct.ScmpSyscall.html#method.from_name_by_arch), [native architecture behavior](https://man7.org/linux/man-pages/man3/seccomp_arch_add.3.html).

On x86, include x86_64, 32-bit x86, and x32 only if the workload requires each. x32 shares an audit architecture identifier with x86_64 and tags syscall numbers with `__X32_SYSCALL_BIT`; checking an audit architecture value alone is insufficient. Let libseccomp generate the ABI guards, inspect the exported filter, and test direct compatibility-ABI calls on supported deployment kernels. On ARM64, deciding whether to allow 32-bit ARM is likewise a policy choice, not an automatic benefit of architecture support. [Kernel ABI warning](https://man7.org/linux/man-pages/man2/seccomp.2.html#CAVEATS), [Rust architecture definitions](https://docs.rs/crate/libseccomp/0.4.0/source/src/arch.rs).

Unknown names fail lookup; architecture-specific resolution can return negative pseudo-syscall numbers for emulated/multiplexed operations, so do not reject every negative value as an error or mistake it for a directly callable kernel syscall. `socketcall` and `ipc` rewrite handling is important on older/compatibility ABIs. Their pointer-packed arguments cannot generally be inspected as ordinary scalar arguments. Exact conditional rules may fail where a portable transformation cannot preserve the desired constraint. Refuse a required policy that cannot be represented; do not silently drop its comparisons. Future unknown syscall numbers fall through to the policy default: default-allow policies automatically expose them. [Resolver contract](https://man7.org/linux/man-pages/man3/seccomp_syscall_resolve_name.3.html), [exact-rule contract](https://man7.org/linux/man-pages/man3/seccomp_rule_add.3.html).

| Failure stage          | What to handle                                                                                                           | Fail-closed launcher response                                                                             |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------- |
| Native loading/linking | Missing library, incompatible symbols or wrong target archive; Rust errors may not run if the dynamic loader fails first | Reject the worker startup; supervisor must not treat absence of readiness as success                      |
| Context/rules          | Allocation failure; unknown syscall; invalid argument index/action; architecture mismatch; impossible exact rule         | Propagate `SeccompError`; do not omit required rules                                                      |
| Attributes/probes      | Unsupported native version/API; invalid/read-only attribute; kernel probe blocked by outer policy                        | Refuse a required feature; never force `set_api` to bypass detection                                      |
| Generation/load        | C generator failure, memory exhaustion, kernel BPF validation or size rejection, permission denial                       | Report the failed stage and error; terminate child without exec                                           |
| TSYNC                  | Divergent filter trees or strict-mode sibling thread; synchronization cannot complete                                    | Reject whole launch; never retry with TSYNC off                                                           |
| Unsupported action     | The enum/BPF can exist without kernel enforcement support                                                                | Probe required actions with `SECCOMP_GET_ACTION_AVAIL` where appropriate; reject unsupported requirements |
| Later use              | Rule blocks loader/runtime shutdown or work takes another syscall path                                                   | Test workload phases and denial paths; do not infer adequacy from initial exec success                    |

`SeccompError::{errno,sysrawrc,raw_ffi_value}` exposes different error information; not every error is an OS errno. `set_api_sysrawrc(true)` helps preserve kernel errors otherwise collapsed to ECANCELED; TSYNC can be mapped to ESRCH by the library. There is no high-level Rust action-availability query or general seccomp syscall wrapper in this release: use an audited `libc::syscall`/lower-level helper for `SECCOMP_GET_ACTION_AVAIL` (Linux ≥4.14), while retaining the load check. Unknown kernel actions terminate rather than degrade into an allowance, but that is not successful installation of the requested behavior. [Error source](https://docs.rs/crate/libseccomp/0.4.0/source/src/error.rs), [native load failures](https://man7.org/linux/man-pages/man3/seccomp_load.3.html), [kernel implementation](https://github.com/torvalds/linux/blob/v6.12/kernel/seccomp.c).

## What a launcher can claim

A scalar filter can reduce accidental access and remove kernel attack surface available to malicious code. It cannot protect the workload from a privileged host, fix kernel vulnerabilities, or recover confidentiality already lost before installation. The following are integration recommendations, not guarantees supplied by calling `load`.

| Requirement                             | What this crate can enforce                                | Gap category and necessary complement                                                                                                                                                                       |
| --------------------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Read-only inputs; writable workspace    | Globally deny selected write-related syscalls or flags     | **Other isolation mechanism:** use Landlock filesystem rules and/or a controlled mount namespace with read-only inputs and writable workspace. Scalar rules cannot bind access to a path                    |
| No networking                           | Deny socket creation and other enumerated syscall paths    | **Integration + OS model:** close inherited sockets, control descriptor transfer and indirect I/O, and isolate the network namespace; blocking `socket` alone does not stop existing connections            |
| Only approved destinations              | Restrict scalar socket families, types or protocol numbers | **OS limitation:** filters cannot read a sockaddr, resolve DNS identity, or authorize TLS peers. Use an enforcing proxy/firewall with no bypass path; port permissions alone are not an IP/domain allowlist |
| Prevent filesystem aliases              | No path/object tracking                                    | **Other mechanism:** kernel object-aware controls, mount topology, and explicit policy for hard links, bind mounts, symlinks, `/proc/*/fd`, cwd and open directory descriptors                              |
| Bound CPU, RAM, tasks, disk             | Deny selected resource-management syscalls                 | **Other mechanism:** cgroups, rlimits, quotas and supervisor timeouts; denying `setrlimit` does not establish a limit                                                                                       |
| Trusted notification broker             | Expose receive/respond/ID validation                       | **Binding gap + integration:** descriptor injection and robust object acquisition need lower-level APIs and a separately audited broker                                                                     |
| Strong isolation from kernel compromise | Reduce reachable syscall surface                           | **Stronger boundary:** VM/microVM or another separate-kernel boundary when required by the threat model                                                                                                     |

Landlock authorizes kernel filesystem objects rather than userspace path strings; available rights and handling of already opened resources depend on its ABI. Kernel documentation includes newer and potentially development-only ABI features: query the deployed ABI and explicitly reject missing rights required by your policy. Do not copy a best-effort example when the product promises mandatory confinement. Namespaces isolate views of resources rather than automatically denying every access. [Landlock design and compatibility](https://docs.kernel.org/userspace-api/landlock.html), [namespace resource table](https://man7.org/linux/man-pages/man7/namespaces.7.html).

Practical ordering is: create an isolated child; establish namespaces/mount layout and resource limits; drop unneeded privileges; close or intentionally transfer descriptors; install other access controls; set NNP and load the required seccomp filter; only then acknowledge readiness and exec the workload. Allow the executable, interpreter, shared libraries, dynamic loader and necessary runtime syscalls in their respective layers. If an exec fails, the child exits. In a multithreaded supervisor, perform allocating/library construction in a fresh helper rather than casually inside `Command::pre_exec`: post-fork code must obey async-signal-safety. Native 2.6 precomputation can help a specialized design, but it does not make arbitrary Rust post-fork code safe. [Native precompute contract](https://man7.org/linux/man-pages/man3/seccomp_precompute.3.html), [Rust pre-exec contract](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#tymethod.pre_exec).

Review at least these bypass routes before asserting broader guarantees:

- **Alternate entry points:** `open`/`openat`/`openat2`, `execve`/`execveat`, `clone`/`clone3`, compatibility ABIs, and `io_uring` operations. A syscall denylist is not a semantic-operation denylist. Deny unused mechanisms rather than assuming all I/O traverses `read` and `write` syscalls. [io_uring submission](https://man7.org/linux/man-pages/man2/io_uring_enter.2.html).
- **Preopened resources:** inherited writable files, shared mappings, connected sockets, directory FDs, and descriptors acquired through SCM_RIGHTS or pidfd operations can preserve authority. FD integers can be duplicated, closed and reused; an argument comparison against an FD number does not identify an immutable object. [Descriptor passing](https://man7.org/linux/man-pages/man7/unix.7.html), [descriptor duplication](https://man7.org/linux/man-pages/man2/dup.2.html).
- **Threads and descendants:** TSYNC closes the installation gap for the current thread group on success, not the period before load or already forked children. Existing asynchronous requests can also outlive installation. Keep setup trusted and descendants supervised. [Thread synchronization implementation](https://github.com/seccomp/libseccomp/blob/v2.6.1/src/system.c).
- **IPC and delegation:** Unix sockets, shared memory, signals, ptrace and host services can let a confined task ask another process to act. A network namespace does not by itself eliminate every Unix-socket or shared-memory route. Restrict reachable peers and permissions as well as syscall entry points. [IPC namespaces](https://man7.org/linux/man-pages/man7/ipc_namespaces.7.html), [network namespaces](https://man7.org/linux/man-pages/man7/network_namespaces.7.html).
- **Argument representation:** registers expose 64-bit values, while a syscall may consume a 32-bit integer. Match the effective width, handle signed values deliberately, and test high-bit variants. The example masks the socket family to its low 32 bits. [Seccomp argument caveats](https://man7.org/linux/man-pages/man2/seccomp.2.html#CAVEATS), [Rust comparison layout](https://docs.rs/crate/libseccomp/0.4.0/source/src/arg_compare.rs).

Numeric comparisons cannot safely inspect pointed-to paths, socket addresses, `open_how`, or other userspace structures. A BPF comparison sees the pointer value, not its referent. Seccomp intentionally disallows dereferencing userspace pointers; adding pointer equality is not string or object authorization. Moving inspection into a notification broker introduces mutable-memory races instead of removing them. [Kernel data-access restriction](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html#introduction), [notification race warnings](https://man7.org/linux/man-pages/man2/seccomp_unotify.2.html#NOTES).

Filter size is finite: Linux permits at most 4,096 classic BPF instructions in one program and 32,768 along a filter path, with accounting overhead for existing filters. Count generated instructions, not just source rules; ABI expansion and argument comparisons matter. Separately cap policy input before asking the C generator to process it, especially in a privileged launcher. [Kernel verifier/accounting constants](https://github.com/torvalds/linux/blob/v6.12/kernel/seccomp.c), [BPF instruction limit](https://github.com/torvalds/linux/blob/v6.12/include/uapi/linux/bpf_common.h).

### Material defects and advisories

All three advisories below were published July 1, 2026 and fixed upstream in **native 2.6.1**. They affect native code used through the safe Rust wrapper; upgrading only the Cargo package does not fix them.

| Advisory                                                                                             | Applicability and consequence                                                                                                                                                           | Status/mitigation                                                                                                                                                 |
| ---------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [GHSA-4q85-33p6-j5g6](https://github.com/seccomp/libseccomp/security/advisories/GHSA-4q85-33p6-j5g6) | Native 2.4.0–2.6.0 includes a 64-bit comparison-tree merge defect: overlapping LT/LE rules on the same argument with common high bits and differing actions can produce a weaker filter | Fixed 2.6.1; patch/backport and regenerate filters by restarting workloads. Existing loaded BPF is not repaired by updating the library                           |
| [GHSA-46fr-jh49-xvhx](https://github.com/seccomp/libseccomp/security/advisories/GHSA-46fr-jh49-xvhx) | Native <2.6.1: extremely large profiles can trigger double-free during generation before kernel load                                                                                    | Fixed 2.6.1; bound untrusted profiles before native compilation                                                                                                   |
| [GHSA-2hqh-5c36-grrm](https://github.com/seccomp/libseccomp/security/advisories/GHSA-2hqh-5c36-grrm) | Native <2.6.1: generated instruction-count overflow can corrupt heap memory; especially relevant to a launcher compiling attacker-controlled profiles                                   | Fixed 2.6.1 with oversized-generation rejection; do not confuse the generator's internal overflow threshold with Linux's much smaller per-filter acceptance limit |

The 2.6.1 release also fixes merging of the notification-use flag (which could prevent listener creation), memory-export copying, and internal aliasing undefined behavior. These are released native fixes, not new Rust APIs. The Rust repository's public advisory API returned an empty list on the research date; the RustSec directory query for this crate returned 404. Those observations do not establish security, exclude unreported flaws, or cover all kernel/distro advisories. Review the deployed native package's patch inventory and kernel security updates. [Native release details](https://github.com/seccomp/libseccomp/releases/tag/v2.6.1), [Rust repository advisory API](https://api.github.com/repos/libseccomp-rs/libseccomp-rs/security-advisories), [RustSec lookup](https://github.com/RustSec/advisory-db/tree/main/crates/libseccomp).

## Notification: exposed, but a separate trust boundary

The release exposes notification without a Cargo feature gate. The wrapper's helpers check native ≥2.5.0 and API ≥6, even though C notification alone can work at API 5. This is an important portability restriction for older kernels. The C allocation helpers query kernel structure sizes; the Rust wrapper uses those helpers rather than assuming a fixed kernel allocation size. [Published notification source](https://docs.rs/crate/libseccomp/0.4.0/source/src/notify.rs), [native allocation contract](https://man7.org/linux/man-pages/man3/seccomp_notify_alloc.3.html).

`get_notify_fd` returns a **borrowed raw FD**, not `OwnedFd`. Published docs say ownership does not transfer and the caller must not close it. Keep the context alive while using that borrowed number; do not wrap it directly with `OwnedFd::from_raw_fd`. If a broker needs ownership, duplicate it or transfer a duplicate with SCM_RIGHTS, then manage that copy's lifetime. Protect the broker and its control socket from the workload; giving the target the responding endpoint lets it influence its own mediation. [FD contract](https://docs.rs/libseccomp/0.4.0/libseccomp/struct.ScmpFilterContext.html#method.get_notify_fd), [listener management](https://man7.org/linux/man-pages/man3/seccomp_notify_fd.3.html).

A broker should validate notification IDs before using target-process resources and handle invalidation, EINTR/ENOENT and target exit. `notify_id_valid` is a liveness check, not a lock on memory, paths, cwd or FD tables. Other threads and processes sharing memory can mutate pointed-to arguments while the calling thread is blocked. Copy and validate data under a threat-aware design; perform authorized operations on broker-controlled objects rather than validating a path and then resuming the original syscall. Broker privileges must be minimal and its filesystem/namespace perspective explicit. The kernel manual explicitly warns against treating notification as a security-policy mechanism. [Kernel notification model](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html#userspace-notification), [unotify security notes](https://man7.org/linux/man-pages/man2/seccomp_unotify.2.html#NOTES).

`new_val` and `new_error` return an emulated result **without executing the original call**. `new_continue` lets the kernel execute it with the target's then-current state; it must not be used to authorize inspected mutable pointer data. `new_error` takes a negative errno for a failing notification response, unlike `ScmpAction::Errno`'s positive payload. Returning an integer that is a broker FD does not install it in the target. This release has no safe ADDFD operation; use an audited ioctl integration (kernel ≥5.9) for descriptor injection. Do not assume the native main branch's newer `seccomp_notify_addfd` helper is in sys 0.3.0: its published declarations lack that symbol too. [Response constructors](https://docs.rs/libseccomp/0.4.0/libseccomp/struct.ScmpNotifResp.html), [published sys declarations](https://docs.rs/crate/libseccomp-sys/0.3.0/source/src/lib.rs), [ADDFD ABI](https://github.com/torvalds/linux/blob/v6.12/include/uapi/linux/seccomp.h).

Without a listener, USER_NOTIF results in ENOSYS; a nonresponsive live broker can block the target. Stacked stronger actions may prevent notifications altogether. Validate broker loss, timeouts, cancellation and denial in isolated processes before adopting this feature. No broker example is included: a toy path-inspect-and-CONTINUE implementation would obscure these security obligations. [Notification operational semantics](https://man7.org/linux/man-pages/man2/seccomp_unotify.2.html).

## Complete focused Rust example and validation

This executable's **only restriction** is that `socket(AF_INET, …)` returns EPERM, including on a thread that existed before load. Its default is explicitly Allow; AF_UNIX remains allowed, AF_INET6 is untouched, and existing sockets are unaffected. It is not a no-network sandbox. Only native x86_64 and aarch64 are accepted; other ABIs remain excluded. The low-32-bit comparison matches the kernel's integer socket-family argument. Required version, attributes and load failures prevent the workload marker.

Create an independent Linux Cargo project with the following `Cargo.toml`. No optional feature is needed. Exact pins make the examined dependencies explicit; preserve Cargo.lock for all transitive versions.

```toml
[package]
name = "seccomp-launch-probe"
version = "0.1.0"
edition = "2021"

[dependencies]
libseccomp = { version = "=0.4.0", default-features = false }
libc = "=0.2.177"
```

Provision native libseccomp **2.6.1+** (or adapt the version policy only after verifying vendor fixes) and its development metadata. A Debian build-image setup, run by the image maintainer, is `apt-get update` followed by `apt-get install -y libseccomp-dev pkg-config build-essential`. Check the actual installed version with `pkg-config --modversion libseccomp`: the package name alone does not guarantee 2.6.1. For an upstream source build use the signed release tarball, C build prerequisites including `gperf`, `./configure --prefix=/your/prefix`, `make`, and `make install`; verify release signatures using the upstream published keys. No sudo or system installation was performed in this research session. [Native build instructions](https://github.com/seccomp/libseccomp/blob/v2.6.1/README.md), [binding requirements](https://docs.rs/crate/libseccomp/0.4.0/source/README.md).

Save this as `src/main.rs`:

```rust
use libseccomp::{
    check_api, ScmpAction, ScmpArch, ScmpArgCompare, ScmpCompareOp,
    ScmpFilterContext, ScmpSyscall, ScmpVersion,
};
use std::{error::Error, io, process::Command, sync::mpsc, thread};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn fail(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, message)
}

fn install(min_api: u32) -> Result<()> {
    if !check_api(min_api, ScmpVersion::from((2, 6, 1)))? {
        return Err(fail("required native version or API level unavailable").into());
    }
    match ScmpArch::native() {
        ScmpArch::X8664 | ScmpArch::Aarch64 => {}
        _ => return Err(fail("example supports only native x86_64/aarch64").into()),
    }
    let mut filter = ScmpFilterContext::new(ScmpAction::Allow)?;
    // New contexts contain only the native ABI; do not add x86 or x32.
    filter.set_act_badarch(ScmpAction::KillProcess)?;
    filter.set_ctl_nnp(true)?;
    filter.set_ctl_tsync(true)?;
    filter.set_api_sysrawrc(true)?;
    filter.add_rule_conditional(
        ScmpAction::Errno(libc::EPERM),
        ScmpSyscall::from_name("socket")?,
        &[ScmpArgCompare::new(
            0, ScmpCompareOp::MaskedEqual(u32::MAX as u64), libc::AF_INET as u64,
        )],
    )?;
    filter.load().map_err(|error| fail(&format!("required filter load failed: {error}")))?;
    Ok(())
}

fn socket_probe(denied: bool) -> io::Result<()> {
    let family = if denied { libc::AF_INET } else { libc::AF_UNIX };
    let fd = unsafe { libc::socket(family, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 {
        let error = io::Error::last_os_error();
        return if denied && error.raw_os_error() == Some(libc::EPERM) {
            Ok(())
        } else {
            Err(error)
        };
    }
    let closed = unsafe { libc::close(fd) };
    if closed < 0 { return Err(io::Error::last_os_error()); }
    if denied { return Err(fail("AF_INET socket unexpectedly allowed")); }
    Ok(())
}

fn child(mode: &str) -> Result<()> {
    if mode == "load-failure" {
        let mut outer = ScmpFilterContext::new(ScmpAction::Allow)?;
        outer.add_rule(ScmpAction::Errno(libc::EPERM), ScmpSyscall::from_name("seccomp")?)?;
        outer.load()?;
        println!("outer installed");
    }
    let (tx, rx) = mpsc::channel();
    let denied = mode == "denied";
    // The thread exists before load, so this probes TSYNC rather than inheritance.
    let worker = thread::spawn(move || -> io::Result<()> {
        rx.recv().map_err(|_| fail("installation did not complete"))?;
        socket_probe(denied)
    });
    install(if mode == "unavailable" { 100 } else { 3 })?;
    println!("workload reached");
    tx.send(())?;
    worker.join().map_err(|_| fail("worker panicked"))??;
    Ok(())
}

fn main() -> Result<()> {
    if let Some(mode) = std::env::args().nth(1) {
        if !["allowed", "denied", "unavailable", "load-failure"].contains(&mode.as_str()) {
            return Err(fail("unknown child mode").into());
        }
        return child(&mode);
    }
    for mode in ["allowed", "denied", "unavailable", "load-failure"] {
        let output = Command::new(std::env::current_exe()?).arg(mode).output()?;
        let reached = String::from_utf8_lossy(&output.stdout).contains("workload reached");
        let should_run = mode == "allowed" || mode == "denied";
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let expected_failure = match mode {
            "unavailable" => stderr.contains("required native version or API level unavailable"),
            "load-failure" => stdout.contains("outer installed")
                && stderr.contains("required filter load failed"),
            _ => true,
        };
        if output.status.success() != should_run || reached != should_run || !expected_failure {
            return Err(fail(&format!("{mode}: status={}, stdout={:?}, stderr={:?}",
                output.status, output.stdout, String::from_utf8_lossy(&output.stderr))).into());
        }
        println!("{mode}: passed");
    }
    Ok(())
}
```

Run `cargo check`, then `cargo run`. If using a private static native installation:

```sh
export LIBSECCOMP_LINK_TYPE=static
export LIBSECCOMP_LIB_PATH=/your/prefix/lib
cargo check --locked
cargo run --locked
```

The supervisor never installs a filter; every case is a new child executable. `allowed` creates and closes an AF_UNIX socket. `denied` requires AF_INET to fail with exactly EPERM. Both probes execute on an existing worker thread after TSYNC. `unavailable` asks for impossible API level 100 and checks that the expected capability error occurs before workload readiness. `load-failure` first installs an outer filter denying the `seccomp` syscall, then requires the inner installation to fail; it checks the outer-install marker and the actual load-error message. Each child exit discards its filters. These are ordinary disposable processes, not persistent host security changes. libc calls here return -1 and set errno; do not compare their return values directly to `-EPERM`.

**Validation performed:** the exact code and manifest above passed `cargo check --locked` and `cargo run --locked` on Debian GNU/Linux 13, x86_64, kernel `7.0.14-25-pve`, rustc `1.97.1 (8bab26f4f 2026-07-14)`, cargo `1.97.1`. The native library was upstream **2.6.1**, built into a private temporary prefix and linked statically; the Rust executable still used the host libc. Output:

```text
allowed: passed
denied: passed
unavailable: passed
load-failure: passed
```

The local macOS host orchestrated the Linux run over the configured build connection. The Linux host's system libseccomp was 2.6.0 and lacked development metadata; it was not replaced. The release archive includes generated syscall tables, so the temporary native build used them without regeneration (`GPERF=/bin/false`; make succeeded without invoking it). No global packages or security settings were changed, and temporary remote artifacts were removed. The resolved sys dependency was 0.3.0. This is focused native execution evidence, not an MSRV, musl, ARM64, compatibility-ABI, notification, WSL2, or complete-launcher certification.

For adoption, add separate child-process cases with bounded supervisor timeouts for: prohibited alternate ABIs; high argument bits; inherited descriptors; fork/exec inheritance; incompatible sibling filter trees causing TSYNC failure; kill actions and expected SIGSYS status; unknown syscall/name behavior; notification broker death; and every required policy layer's unavailable-kernel path. Inspect PFC/BPF to complement behavior tests. Use a disposable VM/container for hostile payloads or policy-size fuzzing, particularly with old native libraries. Never install filters in the test runner itself. The example's socket denial does not test any path, destination, IPC or resource-limit guarantee.

## Alternatives and complementary mechanisms

| Choice                 | Relationship and coverage                                                                               | Deployment/API tradeoff and maintenance evidence                                                                                                                                                                                                |
| ---------------------- | ------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `libseccomp` 0.4.0     | Safe C binding; Linux with broad native ABI translation                                                 | Rich native feature coverage, export and notification; C library and patch inventory required. Released 2025-04-05; Rust CI updates observed 2026-10-14                                                                                         |
| `libseccomp-sys` 0.3.0 | Same implementation through unsafe FFI, not a different sandbox                                         | Useful for native APIs missing safe wrappers only when the declarations actually exist; manual ownership/error handling. Same repository/release date                                                                                           |
| `seccompiler` 0.5.0    | Competing Rust filter compiler/installer, not a C binding; Linux little-endian x86_64, aarch64, riscv64 | Rust conditions/rules/filter → BPF, optional JSON frontend, thread/all-thread installation; avoids native libseccomp deployment. Less native ABI breadth and no comparable notification broker API. Latest registry release 2025-03-07          |
| `extrasafe` 0.5.1      | Higher-level Linux rule sets built on seccompiler, not on this binding                                  | `SafetyContext`, built-in capability groups, per-thread/all-thread application; easier starting policy, but audit every group's allowances. Published 2024-04-16; no release in this report's 24-month window; pins seccompiler `^0.4`, not 0.5 |
| Landlock               | Complementary Linux LSM object-access policy                                                            | Fits read-only inputs/workspace rights; runtime ABI negotiation and strict required-right checks needed. Does not replace syscall surface reduction or an IP/domain egress policy                                                               |
| Linux namespaces       | Complementary mount/network/PID/IPC views                                                               | More launch/setup and privilege handling; combine mount topology, descriptor hygiene and access control. Shared host kernel remains the security boundary                                                                                       |
| VM/microVM             | Stronger separate-kernel boundary                                                                       | Greater startup and operational cost; appropriate when sharing the host kernel is outside the threat model                                                                                                                                      |

Sources: [seccompiler 0.5.0 API](https://docs.rs/seccompiler/0.5.0/seccompiler/), [seccompiler registry](https://crates.io/api/v1/crates/seccompiler), [extrasafe manifest](https://docs.rs/crate/extrasafe/0.5.1/source/Cargo.toml), [extrasafe registry](https://crates.io/api/v1/crates/extrasafe), [Landlock](https://docs.kernel.org/userspace-api/landlock.html), [namespaces](https://man7.org/linux/man-pages/man7/namespaces.7.html). All seccomp choices remain Linux-only; none provides a native macOS/Windows sandbox backend.

Do not interpret seccompiler's original repository archive as proof that development ceased: it was archived August 18, 2026 with an explicit move to the **rust-vmm/rust-vmm monorepo**. Separate published 0.5.0 behavior from that newer source tree. [Original repository notice](https://github.com/rust-vmm/seccompiler), [current project repository](https://github.com/rust-vmm/rust-vmm).

One material extrasafe integration concern is visible in **published 0.5.1 source**: its optional Landlock path uses ABI V2, skips paths it cannot open, and discards the result of `restrict_self`. Its all-thread method rejects nonempty Landlock rules because Landlock does not supply seccomp TSYNC. Consequently, do not treat an extrasafe success result as proof that a required filesystem policy was enforced; audit/fix that behavior or use a directly checked Landlock integration. This is a source-based finding, not a claimed upstream advisory or a statement about unreleased fixes. [Released extrasafe implementation](https://docs.rs/crate/extrasafe/0.5.1/source/src/lib.rs).

## Authoritative resources and adoption questions

- [Published Rust package and versions](https://crates.io/crates/libseccomp), [0.4.0 API](https://docs.rs/libseccomp/0.4.0/libseccomp/), [0.4.0 published source](https://docs.rs/crate/libseccomp/0.4.0/source/).
- [Rust repository and releases](https://github.com/libseccomp-rs/libseccomp-rs), [sys 0.3.0 source](https://docs.rs/crate/libseccomp-sys/0.3.0/source/).
- [Native libseccomp 2.6.1](https://github.com/seccomp/libseccomp/releases/tag/v2.6.1), [native advisories](https://github.com/seccomp/libseccomp/security/advisories), [versioned native manual sources](https://github.com/seccomp/libseccomp/tree/v2.6.1/doc/man/man3).
- [Linux seccomp documentation](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html), [seccomp syscall reference](https://man7.org/linux/man-pages/man2/seccomp.2.html), [notification reference](https://man7.org/linux/man-pages/man2/seccomp_unotify.2.html).

Before adoption, resolve which Linux ABIs and runtime phases the product must support; which patched native packages are actually shipped; whether its minimum kernel and outer container policies permit every required attribute/action; whether notification is necessary enough to justify a trusted broker; and which separate mechanisms enforce filesystem, egress, IPC and resource limits. The focused Linux tests above do not answer those product-specific questions. Treat each missing required capability as a launch refusal, not a successful degraded sandbox.
