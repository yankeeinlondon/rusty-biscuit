---
prompt: |-
    Research the Rust `landlock` crate to help a developer decide whether
    and how to use it in a Linux sandbox launcher.

    Focus on the crate's public API, compatibility behavior, and practical
    integration. Explain kernel concepts where needed to understand the
    crate, without turning the report into a general survey of sandboxing.

    Evidence and scope:
    - State the research date and the exact stable crate version examined.
    - Use primary sources: published crate metadata, version-specific API
      documentation, repository source, release notes, and official Linux
      documentation. Cite claims near the text they support.
    - Use ./landlock.md for context and research leads. Independently verify
      its relevant claims, especially versions and supported capabilities.
    - Distinguish released functionality from unreleased repository changes,
      development-kernel features, proposals, and your own recommendations.
    - If sources conflict or evidence is unavailable, explain the uncertainty.
      Do not infer security guarantees from API names or example success.

    Cover the following topics:

    1. Identity, releases, and maintenance
       - Identify the crate, upstream repository, maintainers, license, and
         relationship to the Linux Landlock project.
       - Verify the latest stable version and release date.
       - Summarize releases from the past 24 months, plus older milestones
         needed to explain the current API. Use dates to describe cadence.
       - Explain API stability, breaking changes, migration considerations,
         and minimum supported Rust version. Do not assume major releases
         exist or equate a pre-1.0 version with poor reliability.
       - Assess maintenance using observable evidence; distinguish facts
         from judgments.

    2. Public API and enforcement model
       - Explain the workflow from defining handled access rights through
         adding rules to applying restrictions.
       - Explain handled versus allowed operations, rule composition,
         enforcement scope, inheritance, irreversibility, and privilege
         requirements.
       - Describe filesystem, networking, and other controls exposed by
         the examined crate version.
       - List Cargo feature flags, their defaults, and their effects.
         Explicitly say if the crate declares none.

    3. Compatibility and failure behavior
       - Distinguish compile-time platform support from runtime availability.
         Address Linux distributions, containers, and WSL2 where relevant.
       - Explain capability detection, ABI selection, and behavior when
         Landlock is absent, disabled, blocked, or only partly supported.
       - Explain strict versus best-effort compatibility behavior using
         the crate's actual APIs and returned enforcement status.
       - Show how a caller can require specific guarantees and refuse to
         launch a workload when those guarantees cannot be enforced.
       - Provide a capability table mapping each important control to its
         public Rust API, minimum Landlock ABI, introducing kernel release,
         and compatibility caveats. Do not treat the kernel version string
         alone as proof that a capability is usable.

    4. Practical uses, limitations, and pitfalls
       - Describe common uses, including self-restriction and launching
         an untrusted child process.
       - Explain policy preparation and enforcement ordering, multithreading
         considerations, inherited file descriptors and sockets, executable
         and runtime dependencies, and filesystem topology pitfalls.
       - Evaluate representative requirements: read-only inputs, a writable
         workspace, no networking, and access only to approved destinations.
         State exactly which parts the crate can enforce.
       - Classify important gaps as:
         a. limitations of the underlying kernel facility;
         b. released kernel capabilities missing from the crate;
         c. integration or deployment constraints; or
         d. requirements that need another isolation mechanism.
       - For each material gap, explain its consequence and any practical
         mitigation. Avoid describing partial restrictions as complete
         isolation.

    5. Rust examples
       - Include complete, focused examples for filesystem confinement,
         supported network restrictions, and capability/enforcement checks.
       - Include Cargo configuration and pin examples to the examined
         crate version. State the required platform and capabilities.
       - Demonstrate meaningful error handling and fail-closed behavior
         for each example's stated requirements.
       - Explain expected allowed and denied operations and remaining gaps.
       - Compile-check examples when tooling permits. Clearly label what
         was compiled, executed, or left unverified; compilation alone
         does not verify enforcement.
       - Explain how enforcement can be tested in isolated child processes
         without restricting the test runner.

    6. Alternatives and complementary tools
       - Compare a small, relevant selection of Rust alternatives and
         complementary crates or tools.
       - Distinguish competing Landlock bindings, higher-level sandbox
         wrappers, and separate mechanisms such as syscall filtering,
         namespaces, and resource limits.
       - Compare enforcement coverage, platform support, maintenance, and
         integration cost. Explain which requirements each option addresses.

    Presentation:
    - Begin with a concise suitability assessment and the main caveats.
    - Use well-formed CommonMark, comparison tables where helpful, and
      Mermaid only when a diagram clarifies a workflow.
    - End with a compact list of authoritative resources and unresolved
      questions that materially affect adoption.
---
