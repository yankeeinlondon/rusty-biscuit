---
ready: false
agent: codex
model: ""
---

# Review: Claudine Inference Adapter

## Findings

### Critical: The `claudine-contract` implementation is absent

The spec requires a new `claudine/contract` library crate implementing
`biscuit_contract::inference::InferenceAdapter` ([spec.md:12](spec.md:12),
[spec.md:54](spec.md:54), [spec.md:111](spec.md:111)). The repository does
not contain `claudine/contract`; `ls claudine/contract` reports `No such file
or directory`, and the root workspace members list includes `claudine/cli` and
`claudine/lib` but not `claudine/contract` ([Cargo.toml:20](../../../Cargo.toml:20)).

Because the crate is missing, none of the required behavior exists:

- no `ClaudineInferenceAdapter` construction API or `Send + Sync`
  `Arc<dyn InferenceAdapter>` path ([spec.md:162](spec.md:162));
- no single non-interactive session execution, stream parsing, metadata
  population, or cancellation handling ([spec.md:186](spec.md:186));
- no `InferenceProfile` model/reasoning mapping ([spec.md:221](spec.md:221));
- no structured output prompt-and-parse flow or JSON Schema validation
  ([spec.md:246](spec.md:246));
- no security isolation, tool/MCP denial, environment allowlist, or provider
  gating ([spec.md:273](spec.md:273));
- no stable `InferenceErrorKind` mapping ([spec.md:327](spec.md:327)).

This is a release blocker: the feature described by the spec is not
implemented.

### Critical: The workspace cannot be verified with Cargo in this worktree

`cargo metadata --no-deps --format-version 1` fails before any package graph can
be loaded because the root workspace references `reaper/lib` and `reaper/cli`
([Cargo.toml:38](../../../Cargo.toml:38)), but both directories are empty and
do not contain `Cargo.toml` files. The command fails with:

```text
failed to read .../reaper/reaper/lib/Cargo.toml
No such file or directory (os error 2)
```

The AGENTS instructions identify `cargo metadata --no-deps --format-version 1`
as the source of truth for workspace membership. Until this is fixed, reviewers
and CI cannot reliably verify workspace integration, dependency direction, or
test selection for this feature.

### High: Required tests are missing at every verification level

The spec requires L1 deterministic tests for the fake session seam, session
planning, prose and structured responses, profile mapping, error mapping,
security rejection, and object-safety ([spec.md:350](spec.md:350)). It also
requires opt-in `real_` tests against an installed/authenticated provider to
prove end-to-end prose, structured output, isolation, environment allowlisting,
and JSON Schema validation ([spec.md:376](spec.md:376)).

No `claudine/contract` crate exists, so there are no adapter tests at all.
Verification level by requirement:

| Requirement | Required level | Strongest present | Result |
|---|---:|---:|---|
| Fake session seam and canned provider stdout | L1 | none | gap |
| Session planning from typed provider catalog | L1 | none | gap |
| Prose response from `assistant_text` | L1 | none | gap |
| Structured output parse and JSON Schema validation | L1 | none | gap |
| Profile priority and reasoning mapping | L1 | none | gap |
| Stable error-kind mapping | L1 | none | gap |
| Reject tool calls, permission prompts, and user-input prompts | L1 | none | gap |
| Object-safe `Arc<dyn InferenceAdapter>` usage | L1 | none | gap |
| End-to-end installed provider behavior | `real_` | none | gap |
| Tool-free isolated directory and env allowlist with real provider | `real_` | none | gap |

This feature is non-terminal and has no keyboard/TTY UX requirements, so
Level 2 and Level 3 terminal verification are not the appropriate tiers here.
The mismatch is that even the required L1 and `real_` tiers are absent.

### High: Security and provider support guarantees are undocumented and unverified

The spec makes tool-free, MCP-free, filesystem-isolated execution normative,
including an explicit environment allowlist and a provider support matrix in
the crate README ([spec.md:278](spec.md:278), [spec.md:290](spec.md:290),
[spec.md:301](spec.md:301)). Because the adapter crate and README do not exist,
there is no documented matrix explaining which providers are enabled or rejected
in v1, and no tests proving unsupported providers are blocked instead of run
unsafely.

Given the spec's threat model, this is not a documentation nit. The adapter is
intended to receive untrusted scraped text and invoke agentic CLIs that can
read files, run commands, and call tools in normal operation. Without the
implementation and provider matrix, the feature cannot be assessed as safe.

### Medium: Drift-maintenance deliverables are missing

The spec requires dependency documentation and skill updates for the new adapter
([spec.md:135](spec.md:135), [spec.md:139](spec.md:139)). There is no
`claudine/contract/justfile`, no `claudine/docs/dependencies.md`, no root
dependency-doc update for `claudine-contract`, and no skill update documenting
the adapter. These should land with the implementation so future consumers can
find and use the adapter consistently.

## Ready For Production

No. `ready` is `false` because the requested adapter crate and its tests are
absent, and the workspace cannot currently be loaded by Cargo for verification.
