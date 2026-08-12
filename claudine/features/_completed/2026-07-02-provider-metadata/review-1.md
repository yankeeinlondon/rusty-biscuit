---
ready: false
agent: codex/default
created: 2026-07-09T14:39:14
implemented: true
---

# Review 1: Provider Metadata Automation

## Verdict

Not production ready.

The central provider metadata work is broadly present: generated `data.rs` files, the `claudine-gen` crate, catalog JSON, registry coverage tests, a unified provider-dispatch inventory guard, generated signal tables, and a `claudine signals check` replay command all exist. The main blockers are in the signal runtime contract: some records that pass fixture replay cannot fire in the production wrapper path, and some `detection: bespoke` records are treated as skips instead of verified runtime behavior.

## Findings

### High: Antigravity `source: exit` signal records pass replay but cannot fire at runtime

- Requirement: the signal catalog records must drive runtime signal detection, and replay checks must prove shipped behavior.
- Implementation: `exit_source_payload` synthesizes only `{"exit_code", "stderr_tail"}` ([bespoke.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/signals/bespoke.rs:25)). Both wrapper paths feed that helper into `SignalSource::Exit` ([spawn.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/exec/spawn.rs:625), [spawn.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/exec/spawn.rs:1177)).
- Generated Antigravity records match and extract `stdout_tail`, not `stderr_tail` ([generated.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/signals/generated.rs:56)). Their fixtures also use `stdout_tail`.
- Impact: unauthenticated Antigravity exit diagnostics such as `Please sign in...` and `authentication failed or timed out` are reported as verified by `claudine signals check`, but the real wrapper-synthesized exit payload has no `stdout_tail`, so these records never match in production.
- Verification level: strongest present verification is Level 1 fixture replay, but it replays fixture-only payload shape rather than the runtime wrapper payload shape. This is a high-severity verification mismatch.

Fix direction: make the runtime exit payload include both stdout and stderr tails where both are available, or change the records/fixtures to the actual runtime contract. Add a unit/integration test that constructs the same exit payload the wrapper emits and asserts these compiled records fire.

### High: Antigravity bespoke signal records are compiled but not verified or wired

- Requirement: bespoke signal records are the documented escape hatch, but they still must emit through the same normalized signal sink and be replayed by `signals check`.
- Implementation: `BespokeChain::for_slug` has no Antigravity detector ([bespoke.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/signals/bespoke.rs:59)), and `bespoke_replayer` has no Antigravity record ids ([bespoke.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/signals/bespoke.rs:86)).
- `claudine signals check --json` completed with `failures: 0`, but reported two Antigravity records as `bespoke_skipped`: `app_log-provider_version-language-server` and `app_log-auth_invalid-not-logged-in`.
- Impact: the check can go green while declared provider-version/auth-invalid app-log records are documentation only. That contradicts the spec's runtime-driving signal-catalog claim and can hide missing last-mile implementation.
- Verification level: Level 1 fixture replay exists for the fleet, but these user-observable operational signals have no runtime detector/replayer verification. The strongest test for those requirements is effectively absent.

Fix direction: either implement an Antigravity app-log detector plus replayers, or keep these records out of the compiled runtime table until they are intentionally documentation-only. `signals check` should fail, not skip, for compiled bespoke records without a replayer unless a record is explicitly marked non-runtime.

### Medium: Generated `model_cli_flag` is not used by the default `apply_model`

- Requirement: static wrapper facts such as `model_cli_flag` should migrate to catalog-driven defaults.
- Implementation: `ProviderInfo` carries `model_cli_flag` ([mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/provider/mod.rs:291)), but the default `WrapperProfile::apply_model` still hard-codes `--model` ([profile/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/profile/mod.rs:377)).
- Impact: the field is generated but not authoritative for the default behavior, so a future provider with a different model flag will silently launch incorrectly unless it adds a profile override. This also explains several near-duplicate per-provider overrides whose only extra behavior is flag de-duplication plus `MODEL` env propagation.
- Verification level: Level 1 tests cover several existing providers, but not the catalog-field-as-authority invariant.

Fix direction: make the default apply `provider_info(provider).model_cli_flag` when present, de-duplicate native aliases where needed, and warn when absent. Keep overrides only for genuinely non-flag delivery such as Goose env behavior or OpenCode's separate non-interactive resolver.

## Test Notes

- `cargo run -q -p claudine-cli -- signals check --json` passed with `failures: 0`; the output still showed two Antigravity `bespoke_skipped` records, which is part of the review finding.
- `cargo nextest run -p claudine-gen --color=never` was started, but it was still compiling after the non-interactive 60-second budget and was aborted with Ctrl+C. No generator-suite result is claimed here.
- No Level 2 or Level 3 requirement applies to the provider-metadata generator/catalog itself. User-observable signal behavior is runtime data-plane behavior, so Level 1 fixture replay is appropriate only when it uses the same payload construction and detector code as production.
