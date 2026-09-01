---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-09-01T22:48:45+01:00
spec: /Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md
implemented: false
description: "A **fix** review of `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md`"
fix: 2026-09-01-inline-compose-frontmatter/review-1.md
---

# Review 1: Inline Compose Frontmatter

## Verdict

Ready for production. The implementation preserves authored frontmatter text through inline closure and generic `md hash --save`, provides the authored allowlist channel for response frontmatter, migrates shipped guardrails safely, and restores mid-run source drift with accurate, non-attributing diagnostics. No requirement-level defects remain in the current tree.

## Findings

No findings.

## Requirement Verification Levels

All acceptance criteria concern parsing, deterministic file transformation, filesystem mutation, or process-level CLI behavior. Level 1 is appropriate for each requirement: none depends on a terminal emulator's renderer or input encoder, so Level 2 or Level 3 would not verify an additional part of the specified contract.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| AC1 — byte preservation | Level 1 unit | Exact full-document comparison permits only the managed hash value and requested `last_updated` change while retaining indentation, trailing spaces, blank lines, and literal escapes. |
| AC2 — no escaped one-liner | Level 1 unit | The AC1 fixture asserts the block scalar remains and the escaped scalar form is absent. |
| AC3 — hash consistency and idempotence | Level 1 unit and CLI process | Stored-hash comparison is clean and a second identical closure is byte-idempotent. |
| AC4 — textual Structured-to-Simple downgrade | Level 1 unit | The managed block node is replaced in place while adjacent authored content remains unchanged. |
| AC5 — generic textual hash save | Level 1 unit and CLI process | LF and CRLF matrices cover Simple, Structured, Detailed, custom and quoted keys, trailing-space block scalars, successful `--diff`, and unsupported flow-root no-write behavior. |
| AC6 — authorized response harvest | Level 1 unit and provider-stub CLI process | Insert, refresh, declaration order, scalar/sequence/mapping values, and accurate status output are exercised across two runs. |
| AC7 — authority and immutability | Level 1 unit and CLI process | Undeclared keys warn with source lines; closure-owned keys are ignored; invalid declarations fail during preparation; removing authorization preserves the prior value. |
| AC8 — invalid harvest is non-mutating | Level 1 unit | Malformed, duplicate, non-map, and empty-body response blocks fail before writing; YAML-significant keys round-trip safely. Exact delimiter classification is also covered through the production CLI caller. |
| AC9 — guardrail migration | Level 1 unit | Both shipped defaults migrate atomically, customization remains untouched, and injected write failure returns the new protocol while preserving the old file and emitting a warning. |
| AC10 — end-to-end inline compose | Level 1 provider-stub CLI process | An obedient provider response inserts and then refreshes generated properties while retaining the authored multiline prompt and producing a clean stored hash. |
| AC11 — mid-run drift restoration | Level 1 unit and CLI process | Added, removed, value-changed, malformed, and delimiter-shape drift are distinguished from body drift; canonical value/body drift produces non-attributing notices and restores authored bytes. |
| AC12 — unchanged-body rejection | Level 1 unit | Authorized returned metadata does not convert an unchanged body into a successful write. |

## Validation

- Claudine focused Nextest: 37 library tests passed, covering closure, preparation, and guardrail behavior.
- Claudine CLI Nextest: 2 provider-stub integration tests and the direct exact-delimiter CLI regression passed.
- Darkmatter focused Nextest: 9 textual writer tests and the CLI preservation matrix passed.
- `just lint` in `claudine/`: passed for all five crates and all 18 diagnostic guard tests.
- The implementation log records a package-wide Darkmatter L1 pass and an unrelated Claudine `shipped_prompt_contract` failure caused by separate prompt artifacts. That external artifact failure does not exercise this fix and does not reduce the verification level of any acceptance criterion above.

## Review Notes

- The mutation path uses portable Rust filesystem and text operations. CRLF-specific tests protect Windows document fidelity; LF coverage protects macOS/Linux documents.
- CLI status output uses `TerminalRenderable` status components.
- The map-based Darkmatter writer remains available for map-owning callers and now explicitly documents its reserialization behavior; text-authoritative callers use the fallible textual writer.
