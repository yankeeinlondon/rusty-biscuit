---
ready: false
agent: codex/default
created: 2026-06-27T11:16:06
implemented: true
---

# Review 1 - OpenCode YOLO

## Verdict

Not ready for production.

The previous composition-path gaps have been addressed: the composition MCP fold now calls the shared merge helper, and the OpenCode YOLO permission overlay is applied after effective YOLO is known. The implementation also keeps `--dangerously-skip-permissions`, adds the config-level permission block, and covers the direct, composition, and policy assembly paths with Level 1 tests.

Two issues remain.

## Findings

### High - Primary subagent hang fix is not verified against OpenCode

The spec's primary acceptance criterion is that a non-interactive OpenCode YOLO compose where a subagent touches an external path completes instead of hanging. The current tests verify Claudine's assembled argv/env shape, for example `claudine/cli/src/commands/wrap/composition/tests.rs:1237` and `claudine/cli/src/commands/wrap/composition/tests.rs:1260`, but they do not run OpenCode or a Task/subagent that triggers `external_directory`.

That means the key behavioral assumption remains unverified: OpenCode must apply this `OPENCODE_CONFIG_CONTENT.permission` block to child Task sessions, and `permission["*"]` plus explicit `external_directory` / `doom_loop` must be enough for the installed OpenCode version. The spec even calls out that this should be checked against installed OpenCode behavior and adjusted if `"*"` is not honored for child sessions.

Verification level: strongest current verification is Level 1 assembly/unit coverage. This requirement is not a terminal encoder/rendering requirement, so Level 2/3 terminal testing is not the right tool; it needs a gated real-provider integration/regression test. Without that, the original user-visible hang is only inferred fixed.

Suggested coverage: add a gated `real_` or equivalent integration test that runs `claudine compose --opencode --yolo` with a prompt/fixture that forces a Task/subagent to read or write outside the worktree, sets a short `--step-timeout`, and asserts completion without an `external_directory` prompt/hang.

### High - Non-UTF-8 existing OPENCODE_CONFIG_CONTENT is silently replaced

Acceptance criterion #8 says an existing user-supplied `OPENCODE_CONFIG_CONTENT` is either merged if it is a JSON object or rejected if invalid; it is never silently replaced. The current call sites extract the existing env value with `OsStr::to_str()`:

- `claudine/cli/src/commands/wrap/env/mod.rs:290`
- `claudine/cli/src/commands/wrap/wrapper_mcp.rs:40`
- `claudine/cli/src/commands/wrap/wrapper_stages.rs:134`

If the variable is present but not valid Unicode on Unix-like systems, `to_str()` returns `None`, and `claudine::opencode_config::merge_overlay` treats `None` as an absent value at `claudine/lib/src/opencode_config.rs:70`. The result is a fresh `{}` base plus Claudine's overlay, silently discarding the user's existing value.

That is exactly the class of silent replacement the spec forbids. Since `OPENCODE_CONFIG_CONTENT` is JSON, a non-UTF-8 value should be rejected with the same redacted, actionable error as malformed JSON. Add a test using `std::os::unix::ffi::OsStringExt` behind `#[cfg(unix)]` so this path cannot regress. Windows env vars are Unicode, so this specific case is Unix-only.

## Coverage Notes

Current Level 1 coverage is materially better than the first implementation: it covers the permission block, deep merge, malformed/non-object JSON strings, composition assembly preserving `instructions` + `mcp` + `permission`, non-YOLO gating, direct wrapper argv/config shape, and policy one-shot behavior.

I attempted a targeted nextest run:

```bash
cargo nextest run --color=never -p claudine -p claudine-cli -E 'test(opencode_yolo) or test(merge_overlay) or test(merge_injected_env) or test(opencode_one_shot)'
```

It was stopped after about 60 seconds with exit code 130 because it was still compiling dependencies in this non-interactive session. No pass/fail result should be inferred from that run.
