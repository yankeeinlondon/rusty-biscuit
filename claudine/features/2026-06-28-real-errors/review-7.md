---
ready: false
implemented: true
agent: "codex/default"
created: "2026-06-29T04:45:14"
---

# Review 7

## Findings

### High - The reference stale-directory failure still cannot produce the required suggestion

The spec's motivating failure is a stale dated path (`features/2026-06-21-opencode-log-fix/spec.md`) and the success criteria require likely-file suggestions for that same reference failure in both `md compose` and `claudine compose`. The implementation only ranks sibling *leaf names* against the missing leaf (`spec.md`), so when the parent directory is wrong it walks up to `features/` and then compares directory names such as `2026-06-28-real-errors` to `spec.md`; that necessarily returns no suggestion (`darkmatter/lib/src/markdown/compose/expression/file_suggestions.rs`:108-117). This is not just untested behavior: the unit test locks the opposite of the required outcome by asserting the dated-directory case returns an empty suggestion list (`darkmatter/lib/src/markdown/compose/expression/file_suggestions.rs`:201-218).

That leaves the concrete reference example under-implemented. It also makes `err.detail.suggestions` empty for the same case because Claudine reuses the render-time suggestion function for `composition.invalid_file_reference` detail (`claudine/lib/src/composition/error.rs`:2751-2776), despite the ratified catalog example showing `err.detail.suggestions` populated for the stale dated path.

Verification level present: Level 1 and Level 2 cover near-miss leaf-name suggestions (`spec.md` -> `specs.md`) in both binaries, and Level 1 explicitly asserts no stale-directory suggestion.

Required verification level: Level 2 in both `md compose` and `claudine compose` for the actual stale-directory shape from the spec, plus Level 1 for the suggestion/detail algorithm.

### High - New lifecycle documentation still teaches deprecated `err.kind` / `err.variant`

The spec requires the legacy `err.kind`, `err.variant`, and `err.msg` fields to remain available during migration, but also says new documentation and examples must use the new faceted names. The updated lifecycle docs still present the public `err` field table as only `kind`, `variant`, and `msg`, then use `err.variant` and `err.kind` in new examples (`claudine/docs/topics/lifecycle.md`:267-279, `claudine/docs/topics/lifecycle.md`:353-361). The usage-cap and timeout recovery examples also instruct authors to match raw provider labels in `err.variant` instead of the locked codes/dispositions (`claudine/docs/topics/lifecycle.md`:408-425, `claudine/docs/topics/lifecycle.md`:450-462).

This undercuts the handleability goal: users following the docs will keep writing stringly lifecycle handlers instead of matching `err.code`, `err.category`, `err.disposition`, `err.origin`, and `err.detail.*`.

Verification level present: I found implementation tests for `err.*` projection, but no documentation check preventing new docs from using the deprecated aliases as the primary examples.

Required verification level: Level 1 documentation/contract test or targeted grep guard, plus updated examples using faceted fields.

### Medium - The transport-lint script fails when invoked directly with `CDPATH` set

`scripts/check-error-transport.sh` computes `script_dir` with `script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"` (`scripts/check-error-transport.sh`:32). In this session `CDPATH` is set, so running `scripts/check-error-transport.sh` from the repo root makes Bash's `cd scripts` print the matched `CDPATH` directory to stdout before `pwd`; `script_dir` becomes a two-line string and the script aborts before scanning. I reproduced that direct invocation failure. `env -u CDPATH scripts/check-error-transport.sh` passes, and `just lint-transport` passes because it invokes `../scripts/...` from `claudine/`, which bypasses `CDPATH` lookup.

The guard is part of the real-errors acceptance criteria, so the script should be robust in its documented direct usage. Clear `CDPATH` locally or use `cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null` before capturing `pwd`.

## Notes

The review-6 blocker is addressed: `darkmatter/cli/tests/level2_errors.rs` now has a Level 2 `$schema` focused-excerpt case for `md compose`, mirroring the Claudine tmux coverage.

Checks run:

- `env -u CDPATH scripts/check-error-transport.sh` passed.
- `env -u CDPATH just lint-transport` passed.
- `just lint-transport` from `claudine/` passed.
- Direct `scripts/check-error-transport.sh` from the repo root failed with the `CDPATH` issue above.

Production ready: **no**.
