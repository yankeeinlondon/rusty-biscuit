---
prompt: |-
    Research the Rust `nano` crate to help a developer decide whether
    and how to use it in a macOS sandbox launcher.

    The starting claim is that `nano` provides macOS sandboxing. Verify that this is the
    correct crate and that the claim matches its released functionality.

    Focus on the published crate's public API, enforcement guarantees,
    compatibility behavior, and practical integration. Explain native OS
    concepts where needed without turning this into a general sandbox survey.

    Evidence and scope:
    - State the research date and exact stable crate version examined.
    - Verify identity through published package metadata and its linked
      repository. If the named crate cannot be identified, report the search
      and uncertainty; do not invent APIs or silently research a substitute.
    - Use primary sources: version-specific API documentation, release notes,
      published source, repository history, and official Apple documentation and relevant
      upstream OS source.
      Cite claims near the text they support.
    - Use ./seatbelt.md for context and research leads. Independently verify its relevant
      claims, especially API availability and deprecation status.
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
       - Determine whether the crate uses Seatbelt profiles, App Sandbox,
         another mechanism, or a combination. Distinguish these mechanisms
         and identify the actual native APIs invoked.
       - Verify minimum macOS versions, supported architectures, and any
         signing, entitlement, packaging, or helper-process requirements.
       - Investigate public versus private or deprecated API dependencies,
         profile-language stability, and implications for distribution.
       - Explain whether the crate restricts the caller or launches a child,
         when enforcement begins, and whether a child can run before it.
       - Examine profile generation, parameter substitution, path handling,
         symlinks, executable/runtime access, and access through system
         services or IPC. State which risks the crate itself handles.
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
       - Include complete, focused examples demonstrating a macOS workload with read-only
         inputs and a writable workspace,
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
       - Compare a small, relevant selection of other macOS Rust wrappers, native sandbox
         APIs, and stronger process or VM isolation.
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
hash: a615b6c6867d0361-5023f88802adb365
last_updated: 2026-11-03
---

# `nano`: identity check for a macOS sandbox launcher

**Do not use `nano` as a sandbox backend.** The published package is an empty nanopass-compiler placeholder, not a macOS sandbox library. It cannot restrict even accidental filesystem access, let alone confine malicious workloads. The starting claim does not match its released functionality. A similarly named package may have been intended, but substituting one would conceal a dependency identity error. [Package metadata](https://crates.io/api/v1/crates/nano), [published source](https://docs.rs/crate/nano/0.1.0/source/src/lib.rs).

Research date: **November 3, 2026**. Exact stable release examined: **`nano` 0.1.0**, published **August 18, 2017**, at 21:04:37 UTC. Here, “stable” means a non-prerelease Cargo version; it does not imply an implemented or security-reviewed API. The registry lists exactly one release, not yanked, and no prereleases. [Version record](https://crates.io/api/v1/crates/nano).

## Identity, release history, and maintenance

| Item                      | Verified finding                                                                                                                  |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Registry package          | `nano`, described as a future nanopass compiler framework and explicitly a reserved name                                          |
| Linked repository         | [`sdleffler/nano-rs`](https://github.com/sdleffler/nano-rs), also described as a nanopass compiler framework                      |
| Published author          | Sean Leffler, as recorded in the release manifest                                                                                 |
| Current registry owner    | GitHub account `sdleffler`; the owner endpoint currently displays Shea Leffler                                                    |
| License declaration       | `Apache-2.0/MIT`, the historical dual-license spelling in the manifest; the downloaded archive contains no separate license files |
| Parent project            | No sandbox parent project identified in the manifest or linked repository                                                         |
| Installability            | Published library dependency; it compiles, but exports no functionality                                                           |
| Rust version              | No `rust-version` or stated MSRV; no edition field, so Cargo treats it as edition 2015                                            |
| Dependencies and features | No Rust dependencies, native libraries, build script, binary target, or Cargo feature flags, including no `default` feature       |

Evidence: [published manifest](https://docs.rs/crate/nano/0.1.0/source/Cargo.toml.orig), [registry owner](https://crates.io/api/v1/crates/nano/owners), [dependency record](https://crates.io/api/v1/crates/nano/0.1.0/dependencies), [Cargo edition default](https://doc.rust-lang.org/cargo/reference/manifest.html#the-edition-field). The author/owner display-name difference is reported as metadata, not evidence of an ownership transfer.

The package archive was downloaded directly from the [versioned registry download endpoint](https://crates.io/api/v1/crates/nano/0.1.0/download). Its only Rust file is:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
```

There are **no releases in the preceding 24 months**, November 3, 2024–November 3, 2026. The only release milestone is 0.1.0 in 2017. The linked repository has two commits, both August 18, 2017: initial scaffolding and a license declaration. Its tip, `cb4c73637417490edc7e4bf287cdf1cff24917b4`, contains the same empty test. GitHub lists no release entries or tags. The registry's November 30, 2017 version `updated_at` is not another publication date. [Registry history](https://crates.io/api/v1/crates/nano), [commit history](https://api.github.com/repos/sdleffler/nano-rs/commits), [pinned repository source](https://github.com/sdleffler/nano-rs/blob/cb4c73637417490edc7e4bf287cdf1cff24917b4/src/lib.rs), [release entries](https://api.github.com/repos/sdleffler/nano-rs/releases), [tags](https://api.github.com/repos/sdleffler/nano-rs/tags).

**Maintenance judgment:** this is a dormant placeholder, based on the empty implementation and dated history. Its pre-1.0 version is not the reason to reject it. There is no implemented API whose stability, breaking changes, or migration history can be assessed, and no observed unreleased sandbox implementation or proposal in this repository. Migrating to a real backend would mean adopting a different package and its policy model, not upgrading an existing `nano` sandbox API.

Searches for `Rust nano crate macOS sandbox`, `"nano" "crates.io" rust`, and `"nano" rust sandbox crate`, followed by registry, rustdoc, archive, and repository inspection, identified `nono` and `nanosandbox` as distinct candidates. They did not establish which, if either, the original claim intended. In particular, the name `nano` was identified successfully; its alleged sandbox identity was disproved.

## Public API and enforcement: every requested control is absent

The [version-specific rustdoc](https://docs.rs/nano/0.1.0/nano/) exports no public types, functions, constants, errors, or status values. There is no policy builder, application step, launcher, capability probe, or enforcement result. The following conclusions follow from inspection of the complete published implementation, not from missing documentation alone. [Source](https://docs.rs/crate/nano/0.1.0/source/src/lib.rs).

| Requirement                                | `nano` 0.1.0 behavior                                    | Consequence for a launcher                                                |
| ------------------------------------------ | -------------------------------------------------------- | ------------------------------------------------------------------------- |
| Read-only inputs                           | No filesystem controls                                   | Inputs retain the launching account's ordinary permissions                |
| Writable workspace, deny writes elsewhere  | No path rules or policy composition                      | Setting a working directory would not confine file access                 |
| No networking                              | No socket or network controls                            | Neither new connections nor existing sockets are restricted by this crate |
| Approved destinations only                 | No domain, address, protocol, or port controls           | No destination enforcement or proxy integration                           |
| Process and IPC confinement                | No process, Mach/XPC, Unix-socket, or other IPC controls | Helpers and services remain outside any `nano`-defined boundary           |
| CPU, memory, process count, disk, deadline | No resource controls                                     | Supervisor or another mechanism must enforce limits                       |
| Capability/enforcement checks              | No probe, error type, or enforcement status              | A caller cannot request guarantees and check that `nano` installed them   |
| Policy generation and parameters           | No SBPL or other policy code                             | No escaping, canonicalization, parameter substitution, or race handling   |
| Caller versus child                        | Neither restricted nor launched                          | No enforcement onset; no protected launch sequence                        |

There is no crate-specific default-deny policy, allow/deny precedence, inheritance, reversibility, privilege requirement, signing requirement, entitlement, helper protocol, or packaging requirement. Adding the dependency changes none of those properties. Any restrictions inherited from an already sandboxed parent belong to that external mechanism.

There is likewise no meaningful minimum macOS version or supported sandbox architecture list. The dependency's empty library can compile on a Rust-supported host without establishing runtime isolation. It has no Linux, Windows, or WSL2 enforcement backend either. No feature flag can turn it into one.

## What the Seatbelt context does—and does not—establish

[The local Seatbelt page](./seatbelt.md) is useful background, especially its separate-worker model and warnings about descriptors. Its sandbox examples and reported runtime experiments are not examples or test evidence for `nano`.

Three mechanisms must stay distinct:

- **Seatbelt** is the macOS policy enforcement mechanism. Chromium describes profiles that constrain a process and its children. Apple's open XNU source exposes mandatory-access-control hooks for process operations; those hooks alone do not specify the closed Seatbelt policy engine or promise SBPL compatibility. [Chromium's implementation documentation](https://chromium.googlesource.com/chromium/src/+/main/sandbox/mac/README.md), [Apple XNU process hooks](https://github.com/apple-oss-distributions/xnu/blob/main/security/mac_process.c).
- **`sandbox_init` and `/usr/bin/sandbox-exec`** are entry points used for custom confinement. Neither is called by `nano`. The former restricts the caller; the latter enters a sandbox and executes the requested command. For a launcher, applying a policy before executing the workload is a separate backend responsibility.
- **App Sandbox** is Apple's supported entitlement-based app model. It requires enabling the capability for Mac App Store distribution and offers file-access and network entitlements. It is not an arbitrary per-job SBPL API or a domain allowlist. [Apple App Sandbox documentation](https://developer.apple.com/documentation/security/app-sandbox).

**Independent availability/deprecation check:** on the research host, macOS 27.2, the installed Apple SDK's `usr/include/sandbox.h` declares `sandbox_init` and `sandbox_free_error` with availability from macOS 10.5 and deprecation from 10.8, and marks them unavailable on Mac Catalyst. The header itself warns it may be removed. Apple's installed `/usr/share/man/man1/sandbox-exec.1`, dated March 9, 2017, labels the command deprecated, recommends App Sandbox, and documents `-f`, `-p`, `-n`, and `-D key=value`. These are observations of Apple's distributed header and manual, not proof that every policy works on every supported macOS release. They can be reproduced with:

```sh
sdk_path=$(xcrun --show-sdk-path)
cat "$sdk_path/usr/include/sandbox.h"
cat /usr/share/man/man1/sandbox-exec.1
```

There is a material qualification to the local Seatbelt page's FFI discussion: **the public header says the flags argument must be `SANDBOX_NAMED`, with other values reserved.** Passing custom SBPL with flags zero exceeds that documented header contract, even though real implementations use it. Parameterized initialization and compile/apply entry points should not be presented as supported public Apple APIs. Chromium explicitly describes reliance on private APIs and an undocumented policy language. Deprecation is not removal, but observed availability is not a future distribution guarantee. [Chromium startup and API design](https://github.com/chromium/chromium/blob/main/sandbox/mac/seatbelt_sandbox_design.md).

No native call, entitlement, architecture, or compatibility fallback can be attributed to `nano` from these sources. In particular, its successful compilation cannot be interpreted as successful Seatbelt installation.

## Integration consequences and material gaps

For **this release**, the decisive gap is a missing implementation, not an OS limitation or a subtle policy bypass. It provides neither accidental-access reduction nor malicious-workload confinement. The recommendation is to remove it from a sandbox backend candidate list until the intended dependency is identified.

For a replacement Seatbelt backend, review the following independently; these are integration recommendations, not functionality supplied by `nano`:

| Gap class                                 | Question and practical mitigation                                                                                                                                                                                                                                                                               |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Native capabilities absent from the crate | A different backend must generate and apply filesystem, network, process, and IPC policy. Do not emulate denial by changing `cwd` or proxy environment variables.                                                                                                                                               |
| Integration and deployment                | Keep the supervisor outside confinement. Use a trusted fresh helper or `sandbox-exec`, refuse setup errors, and never retry unsandboxed. Ensure executable, loader, libraries, and necessary runtime resources remain accessible.                                                                               |
| Integration and ordering                  | Install confinement before processing untrusted input. Inventory inherited file descriptors, sockets, standard streams, and IPC endpoints; explicitly retained resources are capabilities. Chromium warns that already-open descriptors remain usable.                                                          |
| Filesystem policy semantics               | A path rule is not a promise about immutable object identity. Validate canonical paths, `/tmp` aliases, symlinks, hard links, directory overlap, rename, and concurrent replacement. Canonicalization alone does not remove races. Use isolated staging or brokered object access when object identity matters. |
| Network policy semantics                  | A permitted port does not authenticate a destination. Domain approval needs an enforcing broker/proxy with redirects, DNS/address changes, and direct bypass considered. “No networking” must also account for existing sockets and reachable services that can act for the worker.                             |
| Shared-kernel boundary                    | Seatbelt does not give the workload its own kernel. For a threat model requiring a separate kernel, evaluate a VM and tightly constrain host shares and services.                                                                                                                                               |
| Resource and lifecycle management         | Add bounded output, deadlines, descendant cleanup, and appropriate native resource limits separately; `nano` implements none.                                                                                                                                                                                   |

The descriptor and runtime-access cautions are grounded in [Chromium's sandbox guidance](https://chromium.googlesource.com/chromium/src/+/main/sandbox/mac/README.md). Avoid profile compilation in a multithreaded parent's `CommandExt::pre_exec`: Rust requires that callback to obey post-fork safety restrictions. A dedicated helper avoids doing allocator-heavy policy setup in that callback. [Rust `pre_exec` contract](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#method.pre_exec).

**Defects and advisories:** the repository's all-state issue listing was empty, and no `crates/nano` directory was found in the RustSec advisory database on this date. That is a scoped negative search, not a security assessment or evidence of an audit. The material defect relative to the requested use is already established: there is no sandbox implementation. [Repository issues](https://api.github.com/repos/sdleffler/nano-rs/issues?state=all), [RustSec database](https://github.com/RustSec/advisory-db/tree/main/crates).

## Rust example: refuse the unavailable guarantees

A working example that uses `nano` to permit input reads, workspace writes, or selected network access **cannot be written**. There are no corresponding APIs. The complete example below demonstrates only a caller-side fail-closed decision: reject this backend before launching any workload. The error is an application error, not a `nano` error or a runtime capability probe.

`Cargo.toml`:

```toml
[package]
name = "nano-backend-refusal"
version = "0.1.0"
edition = "2021"

[target.'cfg(target_os = "macos")'.dependencies]
nano = "=0.1.0"
```

`src/main.rs`:

```rust
use std::{io, process::ExitCode};

#[cfg(target_os = "macos")]
use nano as _;

fn require_nano_enforcement() -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "nano 0.1.0 cannot enforce read-only inputs, workspace writes, \
         network denial, or approved destinations; workload not launched",
    ))
}

fn main() -> ExitCode {
    match require_nano_enforcement() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(78)
        }
    }
}
```

There are no required features or helper setup. The macOS target dependency pins the examined package exactly; Linux and Windows compile the refusal without it. There is deliberately no `Command::spawn` success path. All workloads are refused, including ones requesting only a writable workspace or only network denial. This is useful as a negative integration fixture, not as a sandbox launcher. A real application should omit the unusable dependency entirely after correcting its backend selection.

**Verification:** compiled and executed on macOS 27.2, `aarch64-apple-darwin`, using Rust 1.98.1. The pinned registry dependency compiled, and the example exited 78 with the unsupported-backend diagnostic. No Seatbelt policy was installed and no confinement was tested. Other target environments were not compiled or executed for this example; no platform parity is claimed. Successful compilation does not establish a lower MSRV.

### Isolated verification plan for the eventual backend

For `nano`, the meaningful acceptance test is that selecting it with required guarantees returns unsupported **before a workload marker can be created**. An allowed-operation test cannot honestly pass as a sandbox test for this package.

For a separately selected implementation, run each policy probe in a disposable child, never by restricting the test runner. Use private fixtures, loopback listeners, timeouts, bounded output, and explicit child cleanup; do not open terminal or browser windows. Recommended acceptance cases are:

1. Read an allowed input, reject a write to it, permit workspace creation and atomic replacement, and reject reads/writes outside the declared policy. Ensure ordinary file permissions would otherwise allow the forbidden operations, so the test measures the sandbox.
2. For network denial, keep a listener reachable from an unsandboxed control child, then require the sandboxed connection to fail. Cover IPv4, IPv6, UDP, inherited sockets, and descendants. For approved destinations, test the real enforcing proxy and direct bypass, not just environment-variable configuration.
3. Exercise aliases, symlinks, hard links, parent renames, inherited descriptors, and IPC brokers. Test the actual executable and TLS/runtime stack so missing runtime access is not mistaken for correct policy denial.
4. Inject unsupported capability results, invalid profiles, missing helpers, nonzero setup exits, and executable launch failures. Assert the workload marker is absent and that no unsandboxed fallback occurs. A helper's launch success or policy-load success alone is insufficient.

These are proposed tests for a future backend, not tests executed against `nano` or evidence of its enforcement.

## Alternatives and complementary mechanisms

The alternatives below are intentionally separate from the researched package. Their documentation establishes their advertised scope, not a security endorsement or a complete source audit.

| Option                                                                              | Relationship and coverage                                                                                                                                                       | Platform, maintenance evidence, and integration cost                                                                                                                                                                  |
| ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`nono` 0.81.0](https://docs.rs/nono/0.81.0/nono/)                                  | Higher-level Rust sandbox primitive; documents `CapabilitySet`, `Sandbox::support_info`, and irreversible caller-side application using Seatbelt on macOS and Landlock on Linux | Registry release dated October 29, 2026; substantially more relevant than `nano`. Requires separate review of generated rules, support checks, and helper integration. Other platforms are documented as unsupported. |
| [`nanosandbox`](https://crates.io/crates/nanosandbox)                               | Higher-level Rust workload launcher; a different package whose documentation advertises macOS Seatbelt and multiple OS backends                                                 | Registry stable release is 0.4.0, November 2, 2026. Search-index results still showed 0.3.3; do not treat those as the latest release. Its exact current guarantees need a separate version-specific source review.   |
| Direct native bindings or `/usr/bin/sandbox-exec`                                   | Competing ways to apply Seatbelt policies; FFI is lower-level, whereas the executable wraps command launch                                                                      | macOS only; locally present but deprecated as described above. Avoids a Rust-wrapper dependency but puts policy generation, native compatibility, and verification on the launcher owner.                             |
| [Apple App Sandbox](https://developer.apple.com/documentation/security/app-sandbox) | Supported native app confinement, complementary to a broker/helper design; entitlement-based files and network access                                                           | macOS app signing and packaging integration; appropriate when user-selected file access and app entitlements fit the product. Not a per-job domain-filtering substitute.                                              |
| [Apple Virtualization](https://developer.apple.com/documentation/virtualization)    | Stronger boundary using a guest OS and separate kernel; not a competing Seatbelt binding                                                                                        | macOS host framework for Apple silicon and Intel, with guest-specific constraints. Higher image, boot, storage, entitlement, and lifecycle cost. Shared directories and host services need explicit policy.           |

Release observations for the Rust alternatives come from their own registry records: [`nono`](https://crates.io/api/v1/crates/nono), [`nanosandbox`](https://crates.io/api/v1/crates/nanosandbox). The `nono` registry currently links [`nolabs-ai/nono`](https://github.com/nolabs-ai/nono); the older repository URL in `seatbelt.md` should not be treated as current package metadata. Neither similarly named package is proven to be the intended correction.

**Decision:** `nano` is never sufficient for the stated requirements, and adding another isolation layer would make that other layer the actual sandbox. Choose a separately verified Seatbelt implementation for native tools when a shared-kernel boundary is acceptable; add a broker for destination policy and explicit lifecycle controls. Prefer a VM when the required boundary includes a separate kernel.

## Authoritative resources and unresolved adoption questions

- [`nano` registry identity and complete release list](https://crates.io/api/v1/crates/nano), [owners](https://crates.io/api/v1/crates/nano/owners), and [exact published archive](https://crates.io/api/v1/crates/nano/0.1.0/download).
- [`nano` 0.1.0 API documentation](https://docs.rs/nano/0.1.0/nano/), [manifest](https://docs.rs/crate/nano/0.1.0/source/Cargo.toml.orig), and [library source](https://docs.rs/crate/nano/0.1.0/source/src/lib.rs).
- [Linked repository at the inspected commit](https://github.com/sdleffler/nano-rs/tree/cb4c73637417490edc7e4bf287cdf1cff24917b4).
- [Apple App Sandbox](https://developer.apple.com/documentation/security/app-sandbox), [file access](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox), [Virtualization](https://developer.apple.com/documentation/virtualization), and [upstream XNU MAC hooks](https://github.com/apple-oss-distributions/xnu/blob/main/security/mac_process.c).
- [Chromium macOS sandbox implementation](https://chromium.googlesource.com/chromium/src/+/main/sandbox/mac/README.md) and [startup/API design](https://github.com/chromium/chromium/blob/main/sandbox/mac/seatbelt_sandbox_design.md).

The unresolved identity question is **which package or repository the original sandboxing claim meant**. Before adopting a replacement, resolve the required macOS versions and architectures, distribution channel, malicious-code threat model, runtime dependencies, and whether “approved destination” means domain, IP address, port, or authenticated service identity. Those decisions determine its acceptance tests. No uncertainty about those choices changes the finding that published `nano` 0.1.0 provides no sandbox.
