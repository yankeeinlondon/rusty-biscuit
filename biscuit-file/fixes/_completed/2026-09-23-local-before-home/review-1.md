---
$schema: feature-review.yaml
ready: false
findings:
    - title: Repository-local prompt symlinks lose local priority
      priority: high
    - title: "`@/` no-match diagnostics show the wrong payload"
      priority: medium
human_review: true
human_review_items:
    - |-
        After the implementation findings are fixed, confirm these two rules:

        - The user's `.claudine` prompt folders are searched after local files, even when the launch folder is the home folder.
        - An `@` reference inside a prompt loaded from elsewhere searches the launch tree; `./`, bare, `&`, and `^` references retain the prompt's own location.

        Choose **confirm both**, **change the first rule**, or **change the second rule**. Also confirm whether command-line tests that need a replaceable home folder may remain skipped on native Windows; the ordering logic is exercised there by unit tests.
    - |-
        Decide how the standalone `md compose` command should resolve an `@` reference inside a document outside the launch tree. Choose:

        1. The document's tree, which is its current documented behavior.
        2. The launch tree, matching Claudine.
        3. A separate follow-up fix to settle the difference (recommended by the current specification).
reviewed_by: codex/gpt-6-sol
created: "2026-09-24T04:07:16-07:00"
spec: 2026-09-23-local-before-home/spec.md
implemented: true
next: 2026-09-23-local-before-home/review-2.md
implemented_by: claude/opus
log: biscuit-file/fixes/2026-09-23-local-before-home/implementation-log.md
description: "A **fix** review of `2026-09-23-local-before-home/spec.md`"
fix: 2026-09-23-local-before-home/review-1.md
---

# Review 1: Local Before Home

## Verdict

**Not production ready.** The shared biscuit-file root chain, launch-scope propagation, cache identity, and ordinary Claudine cases implement the main design. Two supported layouts still violate the specification: a repository-local symlink loses its local priority, and the `@/` spelling produces an incorrect no-match message. Both have reproducible user-visible effects and lack regression tests.

## Findings

### High — Repository-local prompt symlinks lose local priority

The specification's R2 classifies a configured root by **lexical containment** in the launch local root and explicitly says not to resolve symlinks solely to classify its tier. [`with_prompt_magic_roots`](../../../claudine/lib/src/composition/resolve.rs) instead canonicalizes every local convention row and drops it whenever it points to the same directory as a user `.claudine` row. That duplicate suppression is needed when the launch root and home are the same directory under different spellings, but it also removes a distinct path inside an ordinary repository.

I reproduced this with a repository at `<home>/project`, `<project>/.claudine` symlinked to `<home>/.claudine`, and both `<home>/.claudine/prompts/x.md` and `<home>/prompts/x.md` present. From the repository, `claudine compose --dry-run @prompts/x.md` selected the **home `prompts/x.md`** file. The repository-local path `<project>/.claudine/prompts/x.md` should have won: its lexical path is in the local tier, and its `PathPosition::End` root precedes every home-tier root. The relevant drop occurs at `resolve.rs:506–528`.

Restrict physical-identity duplicate suppression to the home/launch-root overlap it was meant to handle, retaining lexically local symlinked roots in other repositories. Add a Level 1 CLI fixture with these two competing files and assert the selected content, plus a registration/order assertion if useful. The current symlink regression only tests two spellings of the **same home root**, so it cannot catch this case.

### Medium — `@/` no-match diagnostics show the wrong payload

R4 requires the human-readable miss to name the reference payload once. Biscuit-file accepts both `@missing.md` and `@/missing.md` as the same magic payload. [`render_magic_no_match_body`](../../../claudine/lib/src/composition/error/render/provider.rs) strips `%` and `@` with `trim_start_matches` but never removes the optional separator. A direct CLI probe of `claudine compose --dry-run '@/missing.md'` printed **`` `/missing.md` was not found ... ``** although resolution searched for `missing.md` under the listed roots. The same trimming can discard a literal leading `@` in a valid payload such as `@@name.md`.

Use the parser's payload or strip exactly one recursive marker, one `@`, and the optional `/`. Add Level 1 diagnostic assertions for the compact and separated spellings, including a payload whose filename begins with `@`. The existing diagnostic tests exercise only `@prompts/missing.md`, so they pass with the faulty rendering.

## Requirement Verification

| User-visible requirement | Strongest verification present | Assessment |
|---|---|---|
| R1/R2: local roots, including a plain launch directory, win before home; inferred and explicit tiers retain their order for direct and recursive `@` | Level 1: biscuit-file `magic_local_roots` candidate, recursive-root, and first-match tests; Claudine CLI `compose_prompt_tiers` dry runs | Correct level, but incomplete case coverage. The repository-local symlink repro above violates the lexical tier rule. |
| R5: Claudine's local prompt conventions precede its user and home conventions, including when home overlaps the local tree | Level 1: `with_prompt_magic_roots` unit tests and Claudine CLI `compose_prompt_tiers` dry runs | Correct level. The local symlink case in the high finding is missing and selects a home file. |
| R3: completion offers paths in the same order resolution uses | Level 1: `completion_roots_match_the_chain_for_every_entry_form`, no-repository parity, and Claudine completion-to-compose round trips | Correct level for filesystem selection and emitted tokens. The shared registration defect also affects which physical file a duplicate token represents. |
| R4: an `@` miss lists ordered search directories, marks only configured roots, and preserves structured probes; other misses retain their report | Level 1: Claudine library renderer/structured-detail assertions and CLI stderr assertions | Correct level for the specified text and data. The supported `@/` spelling is missing and renders the wrong payload. No requirement specifies terminal styling or glyph appearance that would call for Level 2. |
| Ruling 2: nested `@` follows the launch tree while other reference forms keep source anchors | Level 1: biscuit-file seeded/derived-context tests, Claudine source-context tests, and CLI composition from user and sibling-repository prompts | Correct level and sufficient for the specified file-selection behavior. |
| Darkmatter graph and compose-cache identities change when launch scope, tier override, or package root changes | Level 1: `identities_distinguish_launch_magic_scope`, `identities_distinguish_magic_tier_override`, and `identities_distinguish_context_package_root` | Correct level; each test compares both identity products. |
| R6: public descriptions match the intended ordering | Static review of the topic docs, README, and skills | Updated for the ordinary rule. The implementation finding above means the stated lexical rule is not yet true in the symlink layout. |

These requirements concern filesystem decisions, completion text, and error text. They do not depend on a terminal emulator's rendering or input encoder; no Level 2 or Level 3 test is required by the specified behavior. The two findings need additional Level 1 cases, not a higher test tier.

## Test Reachability and Validation

- `biscuit-file/lib/tests/magic_local_roots.rs` is an auto-discovered integration target with ordinary L1 names. Claudine CLI's changed modules are declared by `tests/l1/main.rs` in the `l1` target; the target has no required feature. The relevant library tests are compiled in their source modules. The CLI's CI test metadata enables `test-fixtures`.
- `just check-tier-coverage biscuit-file` and `just check-tier-coverage claudine` each reported zero stranded tests. None of the relevant test paths uses a marker that moves it out of L1.
- `biscuit-file/just test`: 856 passed, zero skipped; its minimal-feature gate passed 6 tests.
- Focused `cargo nextest run -p claudine-cli --features test-fixtures` for `compose_prompt_tiers` and `completion_resolution_round_trip`: 17 passed.
- Focused `cargo nextest run -p claudine` for `composition::resolve` and the launch-scope derivation case: 31 passed.
- Separate one-shot CLI probes reproduced both findings using temporary directories; the symlink probe selected `Winner=[home-prompts]`, and the `@/missing.md` probe printed the leading slash in the payload.

I found no additional low-effort performance change that would alter the readiness decision. The root chain uses one ordering builder, and Darkmatter includes the new selection inputs in its cache identity.
