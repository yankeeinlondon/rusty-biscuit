---
prompt: |-
    Research the Rust `rightkit-sandbox` crate to help a developer decide whether
    and how to use it in a sandbox launcher intended to run across operating systems.

    The starting claim is that `rightkit-sandbox` provides sandboxing across multiple
    operating systems. Verify the exact package, its relationship to any larger Rightkit
    project, and which functionality is available in its published release.

    Focus on the published crate's public API, enforcement guarantees,
    compatibility behavior, and practical integration. Explain native OS
    concepts where needed without turning this into a general sandbox survey.

    Evidence and scope:
    - State the research date and exact stable crate version examined.
    - Verify identity through published package metadata and its linked
      repository. If the named crate cannot be identified, report the search
      and uncertainty; do not invent APIs or silently research a substitute.
    - Use primary sources: version-specific API documentation, release notes,
      published source, repository history, and official documentation for each underlying
      operating-system mechanism.
      Cite claims near the text they support.
    - Use neighboring sandboxing research documents for context and research leads.
      Independently verify relevant claims; do not treat those reports as primary
      evidence.
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
       - Identify the released backend and native mechanism for each OS.
         Separate crate functionality from features supplied only by a parent
         application, workspace package, external binary, or unreleased code.
       - Provide a per-OS matrix of controls, minimum OS/runtime requirements,
         privileges, helper binaries, and signing or packaging requirements.
       - Explain whether the crate can be integrated independently, including
         configuration, runtime, process-management, or ecosystem coupling.
       - Trace a shared policy through backend selection, translation, and
         enforcement. Explain defaults, path semantics, unsupported requests,
         and whether any fallback can weaken the requested policy.
       - Determine how a caller inspects effective capabilities and refuses
         execution when required guarantees are unavailable on a target OS.
       - Distinguish implemented, experimental, planned, and unsupported
         targets. Treat native Windows and WSL2 as different environments.
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
       - Include complete, focused examples demonstrating a shared read-only-input and
         writable-workspace policy across the implemented backends,
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
       - Compare a small, relevant selection of other cross-platform Rust sandbox
         wrappers, direct per-OS integration, and external sandbox launchers.
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
---
