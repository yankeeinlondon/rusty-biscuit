---
prompt: |-
    Research the Rust `rappct` crate to help a developer decide whether
    and how to use it in a native Windows sandbox launcher.

    The starting claim is that `rappct` provides Windows sandboxing. Verify the exact
    spelling and package identity before researching capabilities. Do not silently
    substitute a similarly named crate.

    Focus on the published crate's public API, enforcement guarantees,
    compatibility behavior, and practical integration. Explain native OS
    concepts where needed without turning this into a general sandbox survey.

    Evidence and scope:
    - State the research date and exact stable crate version examined.
    - Verify identity through published package metadata and its linked
      repository. If the named crate cannot be identified, report the search
      and uncertainty; do not invent APIs or silently research a substitute.
    - Use primary sources: version-specific API documentation, release notes,
      published source, repository history, and official Microsoft documentation and
      relevant upstream source.
      Cite claims near the text they support.
    - Use ./app-container.md for context and research leads. Independently verify its
      relevant claims, especially API requirements and security boundaries.
    - Distinguish released functionality from unreleased changes, proposals,
      documentation claims, and your own recommendations. If sources conflict,
      explain the uncertainty. Do not infer security guarantees from API names,
      a successful launch, or the absence of an error.

    Cover the following topics:

    1. Identity, releases, and maintenance
       - Identify the package, repository, maintainers, license, and any parent
         project. Confirm whether the package is published and usable as a
         dependency, and identify relevant yanked or prerelease versions.
       - Verify the latest stable version and release date. Summarize releases
         from the past 24 months and older milestones needed to explain today's
         API. Use dates to describe cadence rather than impressions.
       - Explain API stability, breaking changes, migration considerations,
         minimum supported Rust version, and important native dependencies.
         Do not assume major releases exist or equate pre-1.0 with unreliability.
       - Assess maintenance using observable evidence; label judgments as such.

    2. Public API and enforcement model
       - Explain the workflow from policy configuration to enforcement and
         workload execution, using the crate's actual public types and methods.
       - Explain defaults, allow/deny composition, enforcement scope, inheritance,
         reversibility, privilege requirements, and the trust boundary between
         supervisor, helpers, and workload where applicable.
       - Describe filesystem, networking, process/IPC, and resource controls
         actually exposed. Distinguish supported controls from absent ones.
       - List Cargo feature flags, defaults, and their effects on functionality,
         dependencies, and platform support. Explicitly say if none exist.

    3. Platform integration, compatibility, and failure behavior
       - Identify the native mechanisms actually used: investigate whether
         AppContainer, restricted tokens, integrity levels, capabilities,
         ACLs, or Job Objects participate. Do not attribute mechanisms the
         crate does not implement to its API.
       - Verify supported Windows versions and architectures, token and
         privilege requirements, profile creation, identity lifetime, and
         filesystem or registry permission setup.
       - Explain how process creation applies restrictions before untrusted
         code executes, including suspended-start or assignment steps if used.
       - Investigate inherited handles, descendant processes, breakaway,
         loopback and network capabilities, IPC, and resource limits.
       - Explain persistent security state and cleanup, including whether
         failed setup can leave profiles or modified permissions behind.
         Distinguish native Windows support from execution inside WSL2.
       - Distinguish successful compilation from usable runtime enforcement.
         Explain capability detection and partial or unavailable support using
         the crate's actual errors, status values, or documented limitations.
       - Show how to require specific guarantees and refuse to launch when
         they cannot be enforced. If the API cannot do this, state that gap.

    4. Practical uses, limitations, and pitfalls
       - Describe supported uses and who they protect against. Distinguish
         accidental access reduction from confinement of malicious workloads.
       - Evaluate read-only inputs, a writable workspace, no networking, and
         access only to approved destinations. State exactly which parts the
         crate can enforce; distinguish paths from objects and network ports
         from destination identity where relevant.
       - Examine enforcement ordering, multithreading, inherited descriptors or
         handles, existing sockets, descendants, executable/runtime access,
         filesystem aliases, and IPC paths that affect the claimed guarantees.
       - Classify material gaps as underlying OS limitations, native capabilities
         missing from the crate, integration/deployment constraints, or needs
         for another isolation mechanism. Explain consequences and mitigations.
       - Include documented defects or advisories that materially affect the
         examined version, with their status and applicability. Do not equate
         an absence of published advisories with evidence of security.

    5. Rust examples and verification
       - Include complete, focused examples demonstrating a native Windows child process
         with read-only inputs and a writable workspace,
         supported network restrictions, and capability/enforcement checks.
         If a requested control is unavailable, explain that instead of
         inventing an implementation.
       - Include Cargo configuration pinned to the examined release, required
         features, target configuration, and any setup or helper requirements.
       - Show meaningful error handling and fail-closed behavior for each
         example's stated requirements. Clearly identify any guarantee that
         needs caller-side validation or a separate mechanism.
       - Explain expected allowed and denied operations and remaining gaps.
         For multiple backends, identify what each example demonstrates on
         each platform instead of implying that compilation proves parity.
       - Compile-check when tooling permits. Label examples as compiled,
         executed, or unverified, naming the environment and version tested.
         Compilation and cross-compilation do not establish enforcement.
       - Describe isolated child-process tests for both allowed and denied
         operations, unavailable capabilities, and launch failures without
         restricting the test runner or leaving persistent security state.

    6. Alternatives and complementary tools
       - Compare a small, relevant selection of other Windows Rust wrappers, direct
         Windows API integration, and stronger process or VM isolation.
       - Distinguish competing bindings, higher-level wrappers, complementary
         mechanisms, and stronger isolation boundaries. Compare enforcement
         coverage, platform support, maintenance, and integration cost.
       - Explain when this crate is sufficient, when it needs another layer,
         and when a different approach better meets the stated requirements.

    Presentation:
    - Begin with a concise suitability assessment and the main caveats.
    - Use well-formed CommonMark and capability/comparison tables where helpful.
      Use Mermaid only when a diagram clarifies policy or launch flow.
    - End with authoritative resource links and unresolved questions that
      materially affect adoption.
hash: edf5fb4a0e4b6861-3d73bf874f574f55
last_updated: 2026-11-03
---

# `rappct` for a native Windows sandbox launcher

**Assessment:** `rappct` is the correctly spelled, published Rust AppContainer/LPAC toolkit. Its latest registry stable release, **0.13.3**, is useful for learning and selected integration helpers, but **I would not use its released launch helper as the complete confinement boundary for malicious workloads**. Job assignment happens after process creation, suspended launches have no public resume operation, and some post-creation failures can leave a child running. These are implementation gaps, not limitations inherent to AppContainer. They are reported in [upstream issue #69](https://github.com/cpjet64/rappct/issues/69) and corroborated by the [published launch implementation](https://docs.rs/crate/rappct/0.13.3/source/src/launch/mod.rs).

**Research date: November 3, 2026. Examined dependency: `rappct = "=0.13.3"`, published October 23, 2025, 00:59:26 UTC.** The registry API, downloaded `.crate` archive, version-specific Rustdoc, and linked repository were checked independently. Repository HEAD was `c02f9dd960645522009da3827a79bdc99f50c9c1` dated August 14, 2026. Its `0.14.0` changes are discussed separately; they were not available as a registry release at this check. [Registry metadata](https://crates.io/api/v1/crates/rappct), [0.13.3 documentation](https://docs.rs/rappct/0.13.3/rappct/), [repository history](https://github.com/cpjet64/rappct/commits/c02f9dd960645522009da3827a79bdc99f50c9c1/).

The practical target is a trusted supervisor launching a small, purpose-built native Windows worker. Read-only input objects and writable scratch objects can be provisioned through ACLs. Network capabilities are coarse grants; they cannot express approved hosts or ports. An offline or destination-restricted product contract needs deployment checks and potentially a broker or additional network policy. This document uses [the local AppContainer research](./app-container.md) as context, while verifying native API requirements against Microsoft and actual crate behavior against the published archive.

## Identity, releases, and maintenance

| Property                  | Verified finding                                                                                                              |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Package and Rust import   | `rappct`; this is not a spelling correction to another crate                                                                  |
| Published repository link | `https://github.com/cpjet64/rappct`                                                                                           |
| Registry owner            | One listed owner: GitHub `cpjet64`, display name Curt                                                                         |
| License                   | MIT                                                                                                                           |
| Parent project            | No parent product is identified in the published metadata; the linked GitHub repository reports that it is not a fork         |
| Dependency availability   | Public registry download and Cargo resolution of `=0.13.3` succeeded                                                          |
| Published MSRV            | Rust 1.90; edition 2024                                                                                                       |
| Native dependencies       | `windows` 0.62; Win32 profile/security/process/job/pipe APIs, with handwritten imports including Userenv, Advapi32, and Ntdll |

Sources: [registry metadata](https://crates.io/api/v1/crates/rappct), [registry ownership](https://crates.io/api/v1/crates/rappct/owners), [published manifest](https://docs.rs/crate/rappct/0.13.3/source/Cargo.toml), [published license](https://docs.rs/crate/rappct/0.13.3/source/LICENSE), [repository metadata](https://api.github.com/repos/cpjet64/rappct).

All registry releases in the 24-month window November 3, 2024–November 3, 2026 occurred within October 21–23, 2025:

| Date, UTC                      | Registry versions                                                                     | Relevant evidence                                                                                            |
| ------------------------------ | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| October 21, 2025               | `0.11.1`, `0.12.0`, `0.12.1`, `0.12.2`, `0.12.3`, `0.12.4`; prerelease `0.13.1-dev.0` | `0.12.3` and `0.13.1-dev.0` are yanked; early changelog entries concern publishing and docs.rs configuration |
| October 22, 2025               | `0.13.0`, `0.13.1`, `0.13.2`                                                          | `0.13.1` declares MSRV 1.90 and documents loopback/job guards and environment helpers                        |
| October 23, 2025               | `0.13.3`                                                                              | Internal FFI ownership wrappers and stable storage for the native security-capabilities structure            |
| October 24, 2025–research date | No newer registry version returned                                                    | Repository changes do not establish a new published dependency                                               |

These dates and yank flags come from the [registry version records](https://crates.io/api/v1/crates/rappct). The [packaged changelog](https://docs.rs/crate/rappct/0.13.3/source/CHANGELOG.md) additionally describes `0.9.0`, `0.9.1`, `0.10.0`, and `0.11.0`; those are historical milestones, **not additional versions returned by the registry**. The important older API change is that builder modifiers became infallible `Self` operations, with errors deferred to `build()`. Do not insert `?` after `with_known()` or treat the compatibility builder `unwrap()` as validation. The latter simply returns the builder. [Released builder implementation](https://docs.rs/crate/rappct/0.13.3/source/src/capability.rs).

The current repository manifest says `0.14.0`, Rust 1.88, and points to a self-hosted GitLab repository. Its migration guide describes `0.14.0` as the first publication after `0.13.3`; nevertheless, the registry and GitHub releases queried still stop at `0.13.3`. Treat the migration guide's publication wording and the August 12, 2026 changelog heading as repository intent, not proof of publication. Search-indexed `main` content also showed an intermediate `0.13.10`. The guide explicitly says `0.13.4`–`0.13.10` were never published. [Pinned manifest](https://github.com/cpjet64/rappct/blob/c02f9dd960645522009da3827a79bdc99f50c9c1/Cargo.toml), [migration guide](https://github.com/cpjet64/rappct/blob/c02f9dd960645522009da3827a79bdc99f50c9c1/docs/releases/0.14.0-migration.md), [GitHub release records](https://api.github.com/repos/cpjet64/rappct/releases?per_page=5).

The repository migration includes owned process lifecycle operations, concurrent output capture, changes to enum matching and struct construction, removal of legacy ownership utilities, and revised loopback confirmation. A `0.13` requirement will not automatically accept `0.14`; review migration explicitly when a release becomes available. The published archive's `.cargo_vcs_info.json` identifies commit `3f9d405ef61fbdbf430f63a0e6c3022265692974`, while the migration guide names a different tag commit as its compatibility baseline. Use the archive as the authority for shipped behavior. [Archive provenance](https://docs.rs/crate/rappct/0.13.3/source/.cargo_vcs_info.json), [migration guide](https://github.com/cpjet64/rappct/blob/c02f9dd960645522009da3827a79bdc99f50c9c1/docs/releases/0.14.0-migration.md).

**Maintenance judgment:** there is observable work after the last publication: August 2026 commits address post-spawn cleanup and packaged-consumer testing, and the repository is not archived. However, there has been over a year without a registry update, and issue #69 remains open. That combination supports evaluating the project, but does not support assuming registry users have the fixes. Pre-1.0 alone is not evidence of unreliability; the concrete release gap and launch defects drive the recommendation. [Commit history](https://api.github.com/repos/cpjet64/rappct/commits?per_page=5), [issue #69](https://github.com/cpjet64/rappct/issues/69).

## Public workflow and security boundary

The released workflow is:

1. `AppContainerProfile::ensure(name, display, description)` creates or reuses a per-user identity and returns a profile with public `name` and `sid` fields. `derive_sid_from_name()` derives an identity; derivation alone does not establish that storage was created.
2. `SecurityCapabilitiesBuilder::new(&profile.sid)` starts with an empty capability list and `lpac: false`. `with_known()` and `with_named()` add grants; `lpac(true)` requests LPAC without automatically adding capabilities. `with_lpac_defaults()` requests LPAC **and adds `registryRead` and `lpacCom`**. `build()` derives SIDs.
3. `acl::grant_to_package()` or `grant_to_capability()` changes permissions on files, directories, or supported registry keys before launch.
4. `LaunchOptions` describes the executable, raw command line, working directory, environment, stdio, optional suspended start, and optional job limits. `launch_in_container_with_io()` returns the process-owning `LaunchedIo`; its `wait(self, timeout)` consumes it.
5. Stop all associated processes, close their handles, remove application-added grants or delete disposable objects, and explicitly call `profile.delete()`.

Sources: [profile API](https://docs.rs/rappct/0.13.3/rappct/profile/struct.AppContainerProfile.html), [capability builder API](https://docs.rs/rappct/0.13.3/rappct/capability/struct.SecurityCapabilitiesBuilder.html), [ACL API](https://docs.rs/rappct/0.13.3/rappct/acl/), [launch types](https://docs.rs/rappct/0.13.3/rappct/launch/).

A SID is a Windows security principal identifier. A DACL is the list of object permissions; an ACE is one entry in that list. Windows checks both the ordinary user side and the AppContainer side of the token, taking their intersecting authority. AppContainer runs at low integrity, but low integrity alone is not a path allowlist. A regular AppContainer retains access through `ALL APPLICATION PACKAGES`; LPAC opts out of that baseline and needs more explicit resource access. Neither mode creates a private filesystem root. [Microsoft launch model](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer), [Chromium LPAC design](https://chromium.googlesource.com/chromium/src/+/main/docs/design/sandbox.md#Less-Privileged-App-Container-LPAC).

There is no released operation to remove AppContainer restrictions from a running worker or replace its launch-time capability set. Treat policy changes as a new process launch. ACL edits can change future object access, but removing an ACE does not revoke an already-open handle; cleanup must stop the worker first.

The supervisor and any helpers remain trusted. The crate does not install a sandbox on the caller, implement a broker protocol, or automatically confine services that act on the worker's behalf. Windows enforces the child's token/object restrictions; the crate constructs launch attributes and modifies persistent permissions. Do not hand the child unrestricted host file, process, token, socket, or IPC handles. Do not pass credentials in its environment and expect credential isolation to protect them. [Microsoft isolation categories](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation), [Windows inherited-handle contract](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute).

### Controls actually exposed in 0.13.3

| Area              | Released API and behavior                                                                                               | Boundary or missing control                                                                                                     |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Files             | `ResourcePath::{File, Directory}`, `AccessMask`, `grant_to_package`, `grant_to_capability`                              | Adds allow ACEs to existing DACLs; no deny-policy compiler, object namespace, rollback, or revoke helper                        |
| Registry          | `ResourcePath::RegistryKey(String)`                                                                                     | HKCU/HKLM roots only; caller supplies an appropriate registry access mask                                                       |
| Capabilities      | `KnownCapability::{InternetClient, InternetClientServer, PrivateNetworkClientServer}`; arbitrary names via `with_named` | Grants accumulate; no deny overrides or removal operation; SID derivation is not semantic approval of a name                    |
| AppContainer/LPAC | `SecurityCapabilities { package, caps, lpac }`                                                                          | Per-child token at creation, not restrictions applied to the supervisor                                                         |
| IPC               | Pipes through `StdioConfig::Pipe`; `named_object_path()` helper                                                         | No RPC/COM/named-pipe authorization policy or broker implementation                                                             |
| Resource usage    | `JobLimits { memory_bytes, cpu_rate_percent, kill_on_job_close }`                                                       | Per-process memory ceiling; CPU hard cap clamped to 1–100%; no process-count, total-job memory, disk, or wall-clock quota field |
| Lifecycle         | `LaunchedIo::wait` and `job_guard`; `JobObjectDropGuard`                                                                | No public released resume/terminate operation on the launched process; job timing defects discussed below                       |
| Network helpers   | Optional `net` module                                                                                                   | Enumeration and loopback exemptions, not WFP destination filters                                                                |
| Diagnostics       | `supports_lpac`, current-process token queries, optional configuration warnings                                         | No complete enforcement report or required-guarantees builder                                                                   |

Sources: [ACL source](https://docs.rs/crate/rappct/0.13.3/source/src/acl.rs), [capability types](https://docs.rs/rappct/0.13.3/rappct/capability/), [job fields](https://docs.rs/rappct/0.13.3/rappct/launch/struct.JobLimits.html), [I/O lifecycle](https://docs.rs/rappct/0.13.3/rappct/launch/struct.LaunchedIo.html), [network module](https://docs.rs/rappct/0.13.3/rappct/net/), [diagnostics](https://docs.rs/rappct/0.13.3/rappct/diag/).

ACL grants broaden access; a later read grant does not subtract an earlier write grant or override another applicable principal's grant. `Directory` requests object/container inheritance; `File` requests a non-inheriting ACE. Existing protected child DACLs and other grants still matter. `AccessMask::GENERIC_ALL` is actually the concrete file full-access mask `0x001F01FF`; avoid applying it indiscriminately, especially to registry objects. The registry helper opens keys with `KEY_READ | KEY_WRITE`, rather than explicitly requesting `WRITE_DAC`; do not assume it can update every key whose data is writable. [Released ACL implementation](https://docs.rs/crate/rappct/0.13.3/source/src/acl.rs), [Microsoft registry rights](https://learn.microsoft.com/en-us/windows/win32/sysinfo/registry-key-security-and-access-rights).

`LaunchOptions::default()` selects `C:\Windows\System32\notepad.exe`, a Windows cwd of `C:\Windows\System32`, inherited parent environment (`env: None`), `StdioConfig::Inherit`, a runnable start, no job, and no startup timeout. Set each relevant field explicitly. `cmdline` is a complete Windows command-line string, not an argv vector; include the program token and quote arguments for the target parser. Avoid shell expansion for untrusted arguments. [Released defaults and launch fields](https://docs.rs/crate/rappct/0.13.3/source/src/launch/mod.rs).

An explicit environment replaces the parent environment. `launch::merge_parent_env()` adds selected parent variables, including `PATH`, `TEMP`, and `TMP`; it is a compatibility helper, not secret removal or policy enforcement. Its existing-key comparison is case-sensitive even though Windows environment names are case-insensitive. Prefer an explicitly validated, case-insensitively deduplicated and sorted environment with writable sandbox temp locations. [Environment helper API](https://docs.rs/rappct/0.13.3/rappct/launch/fn.merge_parent_env.html).

### Cargo features

| Feature         | Default? | Effect                                                                                                                       |
| --------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `default = []`  | Yes      | Core profiles, capabilities, ACL, launch, and token API; no optional features                                                |
| `net`           | No       | Enables network module and `windows/Win32_NetworkManagement_WindowsFirewall`; not required to select network capability SIDs |
| `introspection` | No       | Enables configuration warnings and optional `strsim`; suggestion machinery is present but is not a policy validator          |
| `tracing`       | No       | Optional tracing dependency and implementation diagnostics                                                                   |
| `serde`         | No       | Serialization derives for capability/SID data; does not serialize or enforce a whole launcher policy                         |

No other Cargo features are declared. Feature omission can remove an entire public module, so it is inaccurate to promise every unavailable operation returns `Unimplemented`. Non-Windows stubs also vary: an empty capability builder can succeed because deriving zero names is a pure operation. [Published feature manifest](https://docs.rs/crate/rappct/0.13.3/source/Cargo.toml), [module gating](https://docs.rs/crate/rappct/0.13.3/source/src/lib.rs), [SID serialization](https://docs.rs/crate/rappct/0.13.3/source/src/sid.rs).

## Windows compatibility, ordering, and failures

**Native minimum versus crate support:** Microsoft supports the profile and security-capabilities launch APIs from Windows 8 / Server 2012. LPAC requires Windows 10 version 1703, build 15063; `supports_lpac()` checks this using `RtlGetVersion`. This does not establish that all crate imports, dependencies, and workloads run on every OS meeting those minimums. The published documentation target is `x86_64-pc-windows-msvc`; there is no equivalent published runtime-evidence matrix for x86, ARM64, every Server edition, or Windows 8. Adopt a supported Windows 10/11 deployment baseline and test each architecture you ship. [Microsoft profile requirements](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-createappcontainerprofile), [security attribute requirements](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute), [released LPAC check](https://docs.rs/rappct/0.13.3/rappct/fn.supports_lpac.html), [docs target configuration](https://docs.rs/crate/rappct/0.13.3/source/Cargo.toml).

Normal same-user profile creation and AppContainer process creation do not inherently require an administrator or a separately provisioned account. The launcher uses `CreateProcessW` in its own security context; there is no alternate-user-token parameter. ACL editing requires rights on the target objects, and loopback/firewall changes may need elevated provisioning. Keep the supervisor unelevated where possible rather than elevating it to paper over access failures. Profile APIs operate on the current user; impersonation is needed to manage another user's profile. [Microsoft profile API](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-createappcontainerprofile), [launch sample](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer).

A Linux binary inside WSL2 takes the non-Windows paths and gets no AppContainer backend. Invoking a Windows `.exe` from WSL invokes native Windows behavior for that executable; it does not confine Linux processes in the distribution. Likewise, successful macOS compilation proves only that the stubs compile. [Published platform gating](https://docs.rs/crate/rappct/0.13.3/source/src/lib.rs), [launch stubs](https://docs.rs/rappct/0.13.3/rappct/launch/).

### What happens before the child executes

The crate attaches `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` through `STARTUPINFOEXW` and `EXTENDED_STARTUPINFO_PRESENT`. LPAC adds the all-application-packages opt-out attribute. These restrictions are requested **at process creation**, not after arbitrary child code runs. The implementation does not separately expose `CreateRestrictedToken`, configurable integrity levels, or a mitigation-policy interface. AppContainer's low integrity and restricted authority are native effects, not evidence that the crate implements an independent restricted-token sandbox. [Released attribute construction](https://docs.rs/crate/rappct/0.13.3/source/src/ffi/attr_list.rs), [security-capabilities ownership](https://docs.rs/crate/rappct/0.13.3/source/src/ffi/sec_caps.rs), [Microsoft launch attribute semantics](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute).

Jobs have a different sequence in 0.13.3: `CreateProcessW` → create/configure job → assign process. With the default runnable start, a worker can execute or create descendants before assignment. `suspended: true` avoids that execution window, but the crate closes the primary thread handle and exposes no resume method. The public `startup_timeout` field is unused. `Launched` contains only a PID; `launch_in_container()` immediately drops the internal I/O/process/job owner. With kill-on-close enabled this can terminate the child as the helper returns. Use neither helper as proof of complete lifetime containment. [Issue #69](https://github.com/cpjet64/rappct/issues/69), [released `LaunchOptions`](https://docs.rs/rappct/0.13.3/rappct/launch/struct.LaunchOptions.html), [released `Launched`](https://docs.rs/rappct/0.13.3/rappct/launch/struct.Launched.html).

**Required production pattern, recommendation:** own native process/thread/job handles; configure the job before execution through the job-list creation attribute where supported, or create suspended, assign, inspect, then resume. On any failure after creation, terminate and reap the child, close handles, and roll back setup. Refuse to launch if any required step cannot be established. The released convenience API does not supply this transaction; simply adding `?` at the call site cannot repair it. [Microsoft job-list attribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute), [upstream integration report](https://github.com/cpjet64/rappct/issues/69).

### Handles, descendants, and persistence

`Pipe` passes an explicit inherited-handle list containing the three child pipe endpoints. Parent endpoint inheritance-clearing calls ignore errors. `Null` enables inheritance without an explicit handle allowlist, potentially inheriting unrelated inheritable handles. `Inherit` leaves the startup stdio fields untouched and passes `bInheritHandles = false`; its name is not a guarantee of usable redirected stdio. Concurrent supervisor launches must also prevent other broad-inheritance creation paths from picking up temporarily inheritable pipe handles. [Released launch source](https://docs.rs/crate/rappct/0.13.3/source/src/launch/mod.rs), [Microsoft handle-list requirements](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute).

Ordinary descendants retain the parent's AppContainer authority. After job assignment, normal `CreateProcess` descendants join the job by default, subject to native job rules. The crate does not request breakaway flags, but its pre-assignment race can miss early descendants. An external broker launching a process is a separate path. Jobs complement token confinement; they neither replace AppContainer nor impose a destination policy. [Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

Profiles persist until explicitly deleted; dropping `AppContainerProfile` does not delete one. Reusing a name reuses its SID, old grants, and storage, so separate mutually distrustful workloads need separate lifecycle-managed identities. The released `ensure()` also falls back to SID derivation on `ERROR_INVALID_PARAMETER`, not only `ERROR_ALREADY_EXISTS`; `Ok` therefore does not prove correct profile provisioning. [Released profile source](https://docs.rs/crate/rappct/0.13.3/source/src/profile.rs).

A failed second grant leaves a successful first grant in place. Successful profile creation followed by a conversion failure can leave created storage. Deleting a profile does not remove ACEs on unrelated host objects. Maintain a setup journal, use owned disposable staging objects where possible, and implement crash recovery; do not reset whole DACLs on shared objects after concurrent changes. Stop workers before cleanup. Microsoft warns that open storage handles can prevent complete deletion and a failed deletion leaves the profile state undetermined. [Profile deletion contract](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-deleteappcontainerprofile), [ACL mutation implementation](https://docs.rs/crate/rappct/0.13.3/source/src/acl.rs).

### Detection is not attestation

`AcError` exposes `UnsupportedPlatform`, `UnsupportedLpac`, `UnknownCapability`, `AccessDenied`, `LaunchFailed { stage, hint, source }`, `Win32(String)`, and `Unimplemented`. Preserve the source error chain; a displayed stage/hint alone can omit the OS cause. There is no structured report saying that a requested filesystem/network/resource contract is fully enforced. [Error definitions](https://docs.rs/rappct/0.13.3/rappct/enum.AcError.html).

`supports_lpac()` can be overridden by the production-visible environment variable `RAPPCT_TEST_LPAC_STATUS=ok|unsupported`. `RAPPCT_TEST_NO_CWD` can remove the requested working directory; `RAPPCT_TEST_FORCE_ENV` can change environment construction when no explicit environment is supplied. Launch from a controlled environment and reject unexpected overrides before starting concurrent work; avoid mutating process-global environment variables in a multithreaded supervisor. The repository removes these ambient overrides, but the release retains them. [Published root source](https://docs.rs/crate/rappct/0.13.3/source/src/lib.rs), [published launch source](https://docs.rs/crate/rappct/0.13.3/source/src/launch/mod.rs), [repository migration](https://github.com/cpjet64/rappct/blob/c02f9dd960645522009da3827a79bdc99f50c9c1/docs/releases/0.14.0-migration.md).

`token::query_current_process_token()` inspects the **calling process**, not an arbitrary child PID. It returns `TokenInfo` with AppContainer/LPAC booleans, package SID, and capability SIDs. Some unsupported token-information classes map to false/none/empty; treat these values conservatively. To inspect an untrusted executable before execution, the supervisor needs native token queries on a retained suspended child handle. A trusted bootstrap can self-check before loading untrusted input, but a malicious child can fabricate an IPC report. [Token API](https://docs.rs/rappct/0.13.3/rappct/token/struct.TokenInfo.html), [query implementation](https://docs.rs/crate/rappct/0.13.3/source/src/token.rs).

With `introspection`, `diag::validate_configuration()` returns `LpacWithoutCommonCaps` or `NoNetworkCaps` warnings. It ignores launch options and can omit warnings when capability derivation fails. `NoNetworkCaps` is often intentional. Neither warning presence nor absence establishes enforcement. [Diagnostic source](https://docs.rs/crate/rappct/0.13.3/source/src/diag.rs).

## Filesystem and network policies in practice

| Requirement                       | What can be built                                          | What must be checked or added                                                                           |
| --------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Read-only inputs                  | Package-SID read ACEs on isolated input copies             | Existing DACLs, parent delete-child rights, inherited write handles, aliases, protected child ACLs      |
| Writable workspace                | Package-SID read/write rights on dedicated scratch objects | Loader/traversal needs, output size, executable scripts/hooks later consumed by the supervisor          |
| No ordinary external sockets      | Omit all three network capabilities                        | Effective firewall policy, loopback exemptions, existing sockets, brokers, IPv4/IPv6 and protocol tests |
| Approved remote destinations only | Not expressible by the crate                               | Authorized broker/proxy with independently enforced reachability, or WFP policy                         |
| No descendants                    | Not exposed as policy                                      | Native child-process creation policy or job process-count limit, applied before execution               |
| Bounded execution and cleanup     | Partial job helpers                                        | Correct launch ordering, deadline supervision, complete process-tree termination and reaping            |

Object ACLs follow objects, not the spelling used to open them. A lexical allowed-path prefix cannot establish which object a junction, reparse point, hard link, UNC path, or alias reaches. Validate actual objects and ownership before grants; stage copies and prevent hostile replacement between validation and mutation. Test input rename/deletion as well as ordinary writes. Avoid writable parents of read-only objects and overlapping policy roots that silently widen authority. These are integration requirements derived from the [native access-control model](https://learn.microsoft.com/en-us/windows/win32/secauthz/access-control-model), not capabilities implemented by a `ResourcePath` variant.

Provision executable/DLL access before launch: Windows must load the worker before Rust can run. Keep binaries and loader dependencies immutable and separate from outputs. Do not grant a whole user home or toolchain tree full access to repair a failed launch. A workspace writable by hostile code can poison hooks or build scripts later run outside confinement. Brokering publication of validated output is safer than allowing the worker to edit trusted executable inputs.

The three network capabilities mean broadly outbound Internet access, Internet client/server access, and private-network client/server access respectively. They are additive and do not name a hostname, IP, URL, protocol, or port. A TCP port identifies a service endpoint component, not the authenticated destination. Hostname policy must handle redirects, DNS changes, IP literals, proxy tunnels, and shared hosting. [Microsoft capability definitions](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/app-capability-declarations).

Windows Filtering Platform enforces AppContainer network isolation, but effective host rules matter. Project Zero's original August 19, 2021 investigation demonstrated interactions with executable permit rules and same-package loopback. That historical finding is relevant deployment evidence, not proof that every current Windows build is bypassable. A missing capability, failed DNS lookup, or refused connection alone does not prove strict offline behavior. Test known-reachable endpoints and OS-mediated communication separately. [Original network investigation](https://projectzero.google/2021/08/understanding-network-access-windows-app.html).

The optional loopback helpers modify a shared exemption list. `LoopbackExemptionGuard` removes an exemption on drop, but does not restore whether it existed before; drop ignores removal errors. The confirmation latch is a process-global atomic, not authorization tied to a particular SID, and `confirm_debug_only()` is not limited to debug builds. Read/modify/write of shared firewall state also needs coordination with other writers. Avoid these helpers for production destination restriction. [Released network implementation](https://docs.rs/crate/rappct/0.13.3/source/src/net.rs), [Microsoft exemption setter contract](https://learn.microsoft.com/en-us/windows/win32/api/netfw/nf-netfw-networkisolationsetappcontainerconfig).

For approved destinations, prefer structured IPC to a trusted HTTP broker with no worker network capabilities, or an enforced proxy reachable only through independently restricted transport. Environment variables such as `HTTPS_PROXY` are instructions to cooperative software, not confinement. Validate broker operations, IPC ACLs, and client identity; never accept arbitrary host paths or commands merely because the request came from a sandbox.

### Where the gaps belong

| Classification                                                      | Material gap                                                                                                         | Mitigation                                                                                             |
| ------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Underlying OS model                                                 | Shared kernel; ACL-based access rather than a private root; network capabilities describe broad categories           | Minimize worker authority and treat residual OS access as part of the threat model                     |
| Native capability missing or incorrectly integrated in this release | Atomic job association, usable suspended lifecycle, child-process policy, mitigation attributes, WFP filters         | Own the missing native integration or select a corrected implementation                                |
| Integration/deployment constraint                                   | Existing ACLs, aliases, runtime dependencies, inherited sockets/handles, firewall exemptions, persistent profiles    | Stage controlled objects, constrain handle inheritance, validate effective policy, and journal cleanup |
| Need for another isolation mechanism                                | Separate kernel, broad arbitrary-tool compatibility, disk-growth containment, or stronger network topology isolation | Use an appropriately configured VM/Hyper-V boundary and resource/storage policy                        |

No-network socket policy does not itself close named pipes, shared memory, COM/RPC, or other object-mediated IPC. A reachable permissive service can become a broker that widens the worker's authority. Restrict object ACLs and protocol operations independently. The crate's named-object path helper locates a namespace; it does not authorize everything in it. [Microsoft process/object isolation](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation), [released profile API](https://docs.rs/rappct/0.13.3/rappct/profile/struct.AppContainerProfile.html).

### Material defects and their status

| Finding                                                        | Applicability and status at research date                                                                                                   | Consequence                                                                                          |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Job assignment race, missing resume, incomplete teardown       | Reported August 1, 2026 in open #69; confirmed in 0.13.3                                                                                    | A returned error may coexist with a live child; job limits are not guaranteed from first execution   |
| PID-only helper drops ownership                                | Confirmed by released source                                                                                                                | Pipes are lost and a kill-on-close job can kill the child immediately                                |
| `folder_path()` native signature mismatch and guessed fallback | Released source declares binary `PSID`; Microsoft requires a UTF-16 SID string (`PCWSTR`); repository HEAD corrects it and removes fallback | Avoid this method in 0.13.3; `Ok(path)` from a synthesized path is not proof of storage provisioning |
| Ambient testing overrides                                      | Present in 0.13.3; removed in repository changes                                                                                            | A version check or requested cwd can be changed by launcher environment                              |
| Registry DACL update rights                                    | Released helper does not request `WRITE_DAC`; repository changelog records correction                                                       | Permission setup can fail despite apparent registry data access                                      |
| Loopback guard/global confirmation behavior                    | Present in 0.13.3; revised request confirmation in repository                                                                               | Shared state changes and cleanup are not a per-launch security transaction                           |

Sources: [issue #69](https://github.com/cpjet64/rappct/issues/69), [released profile source](https://docs.rs/crate/rappct/0.13.3/source/src/profile.rs), [Microsoft folder-path signature](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-getappcontainerfolderpath), [corrected repository profile code](https://github.com/cpjet64/rappct/blob/c02f9dd960645522009da3827a79bdc99f50c9c1/src/profile.rs), [repository changelog](https://github.com/cpjet64/rappct/blob/c02f9dd960645522009da3827a79bdc99f50c9c1/CHANGELOG.md).

The folder-path issue is a source/API-contract finding here, not a reproduced Windows crash or a published advisory. Do not infer specific exploitability. The GitHub advisory API returned no public advisories. That says nothing about private reports, undiscovered defects, dependency advisories, or an external audit. The [published security policy](https://docs.rs/crate/rappct/0.13.3/source/SECURITY.md) describes reporting/backport intentions, not a security certification. [Public advisory records](https://api.github.com/repos/cpjet64/rappct/security-advisories).

## Complete Rust lab example and its limits

A complete production example satisfying **all** requested guarantees cannot honestly be supplied using only the released launch API. In particular, fail-closed job-before-execution plus post-create failure teardown is unavailable. The example below therefore runs **only its own trusted probe**, never arbitrary commands or untrusted input. `--strict` refuses before creating a profile or process; it demonstrates refusal for requirements the release cannot meet.

The lab demonstrates regular AppContainer creation with **zero capabilities**, read grants on an input fixture, a writable workspace, self-token checks, and one TCP denial test. It does not claim LPAC, universal filesystem exclusivity, universal offline behavior, resource limits, a deadline, or protection against a malicious worker. Error propagation rejects unexpected results; it cannot cure the crate's internal orphan/cleanup defects.

Create a standalone project outside the monorepo workspace with this `Cargo.toml`. No optional `rappct` feature is required. The direct `windows` dependency supplies the execute access-mask constant; it is a binding, not a second sandbox backend.

```toml
[package]
name = "rappct-lab"
version = "0.1.0"
edition = "2024"
rust-version = "1.90"

[target.'cfg(windows)'.dependencies]
rappct = { version = "=0.13.3", default-features = false }
windows = { version = "=0.62.2", features = ["Win32_Storage_FileSystem"] }
```

On native Windows, use the MSVC toolchain and Windows SDK/build tools:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

Prepare a fresh, dedicated staging directory on a local ACL-supporting filesystem, under a parent without broad AppContainer write grants. It must contain only trusted, caller-owned objects, with no reparse points or hard links. Never use an existing checkout or user home as this root. Copy the built binary and any required DLLs into `bin`, create `inputs/input.txt` containing exactly the seven bytes `fixture`, and create empty `work`. These are deployment preconditions, **not validated by this lab**:

```text
C:\rappct-lab\unique-run\
  bin\rappct-lab.exe
  inputs\input.txt
  work\
```

Save the following as `src/main.rs`:

```rust
#[cfg(not(windows))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Err("native Windows is required; no unsandboxed fallback".into())
}

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use rappct::{acl::{self, AccessMask, ResourcePath}, launch::{
        launch_in_container_with_io, LaunchOptions, StdioConfig,
    }, token, AppContainerProfile, SecurityCapabilitiesBuilder};
    use std::{fs, io::{self, Read}, net::{SocketAddr, TcpStream},
        path::PathBuf, time::{Duration, SystemTime, UNIX_EPOCH}};
    use windows::Win32::Storage::FileSystem::FILE_GENERIC_EXECUTE;
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn denied<T>(operation: io::Result<T>, label: &str) -> Result<()> {
        match operation {
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Ok(()),
            Err(e) => Err(format!("{label}: inconclusive failure: {e}").into()),
            Ok(_) => Err(format!("{label}: unexpectedly allowed").into()),
        }
    }
    fn drain(file: Option<fs::File>) -> std::thread::JoinHandle<io::Result<Vec<u8>>> {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut file) = file { file.read_to_end(&mut bytes)?; }
            Ok(bytes)
        })
    }

    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--strict") {
        return Err("0.13.3 cannot guarantee job-before-execution and failure teardown".into());
    }
    if args.get(1).map(String::as_str) == Some("--worker") {
        let expected = args.get(2).ok_or("missing expected SID")?;
        let endpoint: SocketAddr = args.get(3).ok_or("missing endpoint")?.parse()?;
        let info = token::query_current_process_token()?;
        if !info.is_appcontainer || info.is_lpac || !info.capability_sids.is_empty()
            || info.package_sid.as_ref().map(|s| s.as_string()) != Some(expected.as_str())
        {
            return Err("unexpected token: refusing to perform the probe".into());
        }
        if fs::read("../inputs/input.txt")? != b"fixture" {
            return Err("unexpected input fixture".into());
        }
        denied(fs::OpenOptions::new().write(true).open("../inputs/input.txt"),
            "write existing input")?;
        denied(fs::OpenOptions::new().write(true).create_new(true)
            .open("../inputs/new.txt"), "create input")?;
        fs::write("output.txt", b"allowed")?;
        denied(TcpStream::connect_timeout(&endpoint, Duration::from_secs(3)),
            "connect to reachable test endpoint")?;
        println!("token, input, workspace, and one TCP denial probe passed");
        return Ok(());
    }
    if args.len() != 3 {
        return Err("usage: rappct-lab <dedicated-stage-root> <reachable-IP:port>".into());
    }
    for key in ["RAPPCT_TEST_LPAC_STATUS", "RAPPCT_TEST_FORCE_ENV", "RAPPCT_TEST_NO_CWD"] {
        if std::env::var_os(key).is_some() {
            return Err(format!("refusing ambient test override: {key}").into());
        }
    }
    // Only run against the disposable fixture layout described below.
    let root = PathBuf::from(&args[1]).canonicalize()?;
    let endpoint: SocketAddr = args[2].parse()?;
    if endpoint.ip().is_loopback() || endpoint.ip().is_unspecified() {
        return Err("use a controlled, reachable non-loopback endpoint".into());
    }
    TcpStream::connect_timeout(&endpoint, Duration::from_secs(3))?;
    let exe = root.join("bin/rappct-lab.exe");
    let exe_text = exe.to_str().ok_or("this lab requires a Unicode path")?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let name = format!("rappct.lab.{}.{stamp}", std::process::id());
    let profile = AppContainerProfile::ensure(&name, "rappct lab", Some("disposable probe"))?;
    let outcome = (|| -> Result<()> {
        let sec = SecurityCapabilitiesBuilder::new(&profile.sid).build()?;
        if sec.lpac || !sec.caps.is_empty() { return Err("unexpected policy".into()); }
        let rx = AccessMask(AccessMask::FILE_GENERIC_READ.0 | FILE_GENERIC_EXECUTE.0);
        // File selects a non-inheriting ACE, including on this directory object.
        acl::grant_to_package(ResourcePath::File(root.clone()), &profile.sid, rx)?;
        for (directory, mask) in [
            ("bin", rx),
            ("inputs", AccessMask::FILE_GENERIC_READ),
            ("work", AccessMask(AccessMask::FILE_GENERIC_READ.0
                | AccessMask::FILE_GENERIC_WRITE.0 | FILE_GENERIC_EXECUTE.0)),
        ] {
            acl::grant_to_package(ResourcePath::Directory(root.join(directory)),
                &profile.sid, mask)?;
        }
        let opts = LaunchOptions {
            exe: exe.clone(),
            cmdline: Some(format!("\"{exe_text}\" --worker {} {endpoint}", profile.sid.as_string())),
            cwd: Some(root.join("work")),
            env: Some(vec![
                ("SystemRoot".into(), std::env::var_os("SystemRoot").ok_or("missing SystemRoot")?),
                ("TEMP".into(), root.join("work").into_os_string()),
                ("TMP".into(), root.join("work").into_os_string()),
            ]),
            stdio: StdioConfig::Pipe,
            suspended: false,
            join_job: None,
            startup_timeout: None,
        };
        let mut child = launch_in_container_with_io(&sec, &opts)?;
        drop(child.stdin.take());
        let stdout = drain(child.stdout.take());
        let stderr = drain(child.stderr.take());
        let status = child.wait(None);
        let output = stdout.join().map_err(|_| "stdout reader panicked")??;
        let errors = stderr.join().map_err(|_| "stderr reader panicked")??;
        print!("{}", String::from_utf8_lossy(&output));
        eprint!("{}", String::from_utf8_lossy(&errors));
        if status? != 0 { return Err("probe failed; reject this configuration".into()); }
        Ok(())
    })();
    let cleanup = profile.delete();
    match (outcome, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) => Err(e),
        (result, Err(e)) => Err(format!("probe result: {result:?}; profile cleanup failed: {e:?}").into()),
    }
}
```

Run `rappct-lab.exe <stage-root> <reachable-IP:port>` against a caller-controlled, non-loopback test server. The supervisor first requires a successful connection, so an unreachable server is not mistaken for sandbox enforcement. Do not use an arbitrary production service as the test endpoint. Start the lab in an environment free of `RAPPCT_TEST_*` overrides. The timestamp/PID profile name reduces accidental collisions; it is not an atomic proof of a new profile and must not be used as such in a security design.

The expected child results are: read fixture succeeds, opening it for write fails with permission denied, creating an input sibling fails with permission denied, writing workspace output succeeds, and connecting to the control endpoint fails with permission denied. Another network error is inconclusive and fails the probe. A passing TCP probe applies only to that endpoint and connection attempt; it does not test UDP, DNS, loopback, brokers, or every destination. The child also confirms regular AppContainer identity and an empty capability set before attempting the probes. These self-reports are useful because this worker is trusted.

Both output pipes are drained concurrently to avoid blocking on a full stderr pipe while reading stdout. The sample waits without a deadline because this release lacks a reliable per-child termination method; run it only under an external bounded lab supervisor. If it hangs or crashes, terminate/reap the probe before cleanup. The code always attempts profile deletion after ordinary `Result` failures, but it cannot recover a profile created before `ensure()` itself returns an error, nor roll back added ACEs. After the process has exited, delete **only the dedicated stage tree** to dispose of its grants; reconcile any leaked profile by its recorded name. Never delete a real input tree as an ACL rollback strategy.

**Verification performed:** the exact manifest and Rust source above passed `cargo check --target x86_64-pc-windows-gnu` on macOS 27.2, using Rust 1.98.1, `rappct` 0.13.3, and `windows` 0.62.2. The non-Windows branch also passed a native macOS check. These were type checks, **not Windows linking or execution**. A native Windows build host was configured in the session, but no sandbox execution or persistent Windows security changes were made for this research. Native MSVC linkage, runtime behavior, minimum-Rust compilation, and enforcement remain unverified here. In particular, handwritten import-library choices require a real native link test; a cross-target check does not test them.

For an LPAC experiment, require `supports_lpac()?`, choose `.lpac(true)`, and inspect the resulting child's LPAC token before untrusted input. Reject unsupported or unknown support; never silently retry as regular AppContainer. Add named capabilities only when justified by tested loader/runtime needs. `with_lpac_defaults()` broadens registry/COM authority and is not a necessary definition of LPAC. There is no complete LPAC example here because its deployment grants and pre-execution verification require additional native integration.

## Verification plan for adoption

Keep the runner and supervisor outside confinement. Use a dedicated Windows test account or disposable VM, unique profile identities, and fixture-only objects. Use nextest and the repository's [test toolkit](../../../../tools/test-toolkit/README.md) for integration in this monorepo; create no focused terminal/browser windows. The following are acceptance tests to implement, **not tests executed during this research**:

1. **Establish controls outside confinement.** Confirm fixture read/write and controlled-server connectivity. Missing files, an unreachable server, or ordinary user access denial are setup failures.
2. **Verify creation policy.** With an improved native launcher, inspect the suspended child's AppContainer SID, LPAC flag where required, capability list, integrity, inherited handles, and job configuration before resuming. Supply untrusted input only after verification. Refuse mismatches.
3. **Test allowed and denied operations in isolated children.** Cover read, create, overwrite, rename, file deletion, parent deletion/replacement, unrelated sentinel objects, protected child DACLs, reparse points, hard links, alternate path spellings, registry, and broker/IPC endpoints. Distinguish expected access denial from arbitrary failure.
4. **Test networking independently.** Use controlled public/private IPv4 and IPv6 endpoints, TCP/UDP, loopback, and DNS. Test existing host firewall permit rules and exemptions. If destinations are restricted, attempt direct proxy bypass, redirects, IP literals, and unintended ports.
5. **Test lifecycle adversarially.** Spawn a descendant immediately, attempt breakaway, fill stdout and stderr concurrently, exceed memory/CPU policy, and crash the supervisor. Confirm the whole tree is gone. Inject failures at attribute setup, process creation, job configuration, assignment, and resume. A failure must leave no runnable child.
6. **Test unavailable support.** On suitable isolated systems, exercise unsupported LPAC and failed profile/ACL provisioning. Use child-scoped test environments for the release's overrides, never process-global mutations in a parallel runner. An override tests error handling, not a real older OS.
7. **Audit cleanup after success and failure.** Verify profile absence, no added ACEs on surviving objects, no changed loopback list, no live descendants, and no leaked handles. Record setup state before each mutation so interrupted runs can be reconciled.

A strong launcher should refuse policy it cannot enforce rather than downgrade to an unrestricted `std::process::Command`. Passing a finite probe suite is evidence for the tested builds/configuration; it does not prove universal confinement or protect against kernel vulnerabilities.

## Alternatives and complementary layers

| Option                                                          | Role and coverage                                                                                                       | Maintenance/platform evidence and integration cost                                                                                                                                                  |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rappct` 0.13.3 helpers plus a separately owned native launcher | Reuse selected SID/capability helpers; implement atomic job setup, handle policy, verification, and teardown yourself   | Windows enforcement only; known release defects still require review. Moderate-to-high integration effort                                                                                           |
| Microsoft `windows` / `windows-sys`                             | Competing bindings for direct profile, token, ACL, process, job, mitigation, and WFP integration                        | Microsoft's maintained Windows bindings; each API retains its own OS minimum. Highest policy/ownership responsibility, clearest access to missing native controls                                   |
| `firehazard`                                                    | Lower-level Rust wrappers for tokens, jobs, ACLs, handles, process mitigation and related security primitives           | Published docs identify `0.0.0-2022-09-10`, non-semver prereleases, and explicit review/testing limitations. Not evidence of a turnkey AppContainer launcher; investigate current source separately |
| `sandboxrs-windows`                                             | Higher-level wrapper advertising filesystem policy and runtime backend probing; its AppContainer fallback uses `rappct` | Published 0.1.1 docs describe an experimental Windows sandbox API plus fallback. A candidate for separate source/version/runtime audit, not an automatic fix for a dependency's defects             |
| Native Job Objects, child-process policy, WFP, broker           | Complementary controls, not substitutes for token confinement                                                           | OS-provided primitives; caller owns availability, privilege, lifetime, and error handling. A broker can enforce service identity without granting direct worker egress                              |
| Windows Sandbox or Hyper-V-isolated container / dedicated VM    | Stronger separation through a separate kernel; suitable when arbitrary tools or host-kernel exposure dominate           | Greater deployment/startup cost and OS/virtualization requirements. Explicitly configure shared folders, network, clipboard, and other integrations                                                 |

Sources: [Microsoft Rust bindings](https://github.com/microsoft/windows-rs), [firehazard published documentation](https://docs.rs/firehazard/0.0.0-2022-09-10/firehazard/), [sandboxrs-windows 0.1.1 documentation](https://docs.rs/crate/sandboxrs-windows/0.1.1), [Windows process attributes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute), [Windows Sandbox configuration](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/windows-sandbox-configure-using-wsb-file), [Hyper-V isolation](https://learn.microsoft.com/en-us/virtualization/windowscontainers/manage-containers/hyperv-container).

AppContainer is a genuine native boundary suitable for compromised worker code when provisioned correctly; it is not merely cooperative access reduction. The released crate is sufficient for controlled experiments and potentially selected helpers after review. For malicious workers, use a corrected, verified native launch transaction or independently audited implementation. For shared-kernel escape concerns or workloads requiring a broad host-like environment, prefer separate-kernel isolation. A job alone cannot supply file/network isolation, a proxy variable alone cannot confine egress, and a process-isolated Windows container shares the host kernel.

## Authoritative resources and unresolved adoption questions

- [Crates.io identity and version records](https://crates.io/api/v1/crates/rappct), [owners](https://crates.io/api/v1/crates/rappct/owners), and [version-specific API documentation](https://docs.rs/rappct/0.13.3/rappct/).
- [Published 0.13.3 source archive browser](https://docs.rs/crate/rappct/0.13.3/source/), [open launch issue #69](https://github.com/cpjet64/rappct/issues/69), and [pinned repository migration guide](https://github.com/cpjet64/rappct/blob/c02f9dd960645522009da3827a79bdc99f50c9c1/docs/releases/0.14.0-migration.md).
- [Microsoft's AppContainer launch guide](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer), [profile creation](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-createappcontainerprofile), [folder-path signature](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-getappcontainerfolderpath), and [process attributes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute).
- [Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects), [capability categories](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/app-capability-declarations), and [original Project Zero network analysis](https://projectzero.google/2021/08/understanding-network-access-windows-app.html).

Before adopting, resolve:

1. **Publication:** when will the lifecycle/profile fixes reach a registry release, and which archive/commit does it contain? Does the GitLab-led release process produce verifiable artifacts matching the GitHub mirror?
2. **Runtime baseline:** does native MSVC linkage and the required LPAC/ACL/loader behavior work on each Windows build and architecture being shipped? What privileges does provisioning actually need there?
3. **Policy strength:** is the requirement ordinary socket denial, complete offline behavior including brokers, or approved authenticated destinations? Which independent mechanism establishes it?
4. **Transaction and recovery:** who owns child/thread/job handles through every failure, and how are profiles, ACL grants, exemptions, and descendants reconciled after a crash?
5. **Workload fit:** can executable dependencies and IPC authority be enumerated without broad host grants? If not, would a separate-kernel environment meet the requirement more directly?
