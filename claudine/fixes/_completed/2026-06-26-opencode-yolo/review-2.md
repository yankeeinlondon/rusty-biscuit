---
ready: true
agent: codex/default
created: 2026-06-27T12:10:26
---

# Review 2 - OpenCode YOLO

## Verdict

Ready for production.

The issues from review 1 have been addressed. The implementation now rejects
present-but-non-UTF-8 `OPENCODE_CONFIG_CONTENT` values instead of treating them
as absent, and it adds a gated real-provider regression test for the OpenCode
subagent external-directory hang.

## Findings

No blocking findings.

## Verification Map

- Spec AC #1, non-interactive OpenCode YOLO compose with a subagent touching an
  external path completes: covered by
  `claudine/cli/tests/real_opencode_yolo_subagent.rs`. This is a gated
  real-provider integration test, not L1/L2/L3 terminal coverage. That is the
  right verification class for this requirement because the load-bearing
  behavior is OpenCode's child-session permission resolver, not terminal
  rendering or keyboard encoding.
- AC #2, YOLO non-interactive config contains `permission["*"]`,
  `external_directory`, and `doom_loop` as `"allow"`: covered by L1 tests in
  `claudine/lib/src/opencode_config.rs`,
  `claudine/cli/src/commands/wrap/wrapper_stages.rs`, and
  `claudine/cli/src/commands/wrap/composition/tests.rs`.
- AC #3, non-YOLO runs do not receive a Claudine `permission` overlay: covered
  by L1 gating tests in `wrapper_stages.rs` and composition assembly tests.
- AC #4, `instructions`, `mcp`, and `permission` coexist in
  `OPENCODE_CONFIG_CONTENT`: covered by L1 merge/assembly tests for the direct
  wrapper, MCP fold, and composition path, including the second-iteration
  regression where system-prompt instructions are folded after YOLO.
- AC #5, `--dangerously-skip-permissions` remains on argv: covered by L1 wrapper
  profile and spawn-spec tests.
- AC #6, `doom_loop` is explicitly auto-allowed: covered wherever the permission
  block is asserted.
- AC #7, the permission block is path-free and serializes consistently across
  platforms: covered by the L1 serializer test in `opencode_config.rs`. The
  implementation does not include host paths in the overlay.
- AC #8, existing user `OPENCODE_CONFIG_CONTENT` is merged or redacted-rejected:
  covered by L1 tests for object merge, malformed JSON, non-object JSON, and
  Unix non-UTF-8 env values. Diagnostics name the variable without echoing the
  raw value.
- AC #9, OpenCode policy one-shot planning uses
  `--dangerously-skip-permissions` plus the merged config overlay rather than
  emitting native `opencode run --yolo`: covered by L1 policy backend tests.

## Test Notes

I attempted a targeted nextest run:

```bash
cargo nextest run --color=never -p claudine -p claudine-cli -E 'test(opencode_yolo) or test(merge_overlay) or test(merge_injected_env) or test(opencode_one_shot)'
```

It was stopped after about 60 seconds with exit code 130 because it was still
compiling dependencies in this non-interactive session. No pass/fail result
should be inferred from that run.

The real OpenCode regression test is intentionally opt-in because it requires an
installed, authenticated provider and consumes provider resources. It is wired
into `claudine/justfile` under `just test-real`.
