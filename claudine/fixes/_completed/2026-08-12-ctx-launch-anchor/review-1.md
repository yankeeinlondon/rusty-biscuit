---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-08-26T13:14:14+01:00
spec: 2026-08-12-ctx-launch-anchor/spec.md
log: claudine/fixes/2026-08-12-ctx-launch-anchor/log.md
implemented: true
implemented_by: codex/default
next: 2026-08-12-ctx-launch-anchor/review-2.md
description: "A **fix** review of `2026-08-12-ctx-launch-anchor/spec.md`"
fix: 2026-08-12-ctx-launch-anchor/review-1.md
---

# Review 1: Ctx Launch Anchor

## Verdict

The fix is **not ready for production**. The launch-owned capture seam and
Darkmatter extension API are directionally sound, and the direct compose
regressions pass on macOS, but graph preflight can still accept bracket-indexed
target identity. In addition, the implementation does not provide the route,
surface, snapshot-consumer, or cross-platform evidence required by AC1–AC14.

## Findings

### 1. High: bracket-indexed target identity bypasses sequence graph preflight

`target_identity_path_in_expr` recognizes only a `Variable` whose complete path
is exactly `ctx.agent`, `ctx.model`, `env.AGENT`, or `env.MODEL`
(`claudine/lib/src/composition/sequence/preflight/shape.rs:206-210`). For an
`Expr::Index`, it visits the base and index as independent expressions
(`shape.rs:218-219`). Consequently `ctx["agent"]` is inspected as the legal
base `ctx` plus a string literal, and `env["MODEL"]` as `env` plus a string
literal; neither reconstructs a forbidden identity path. Darkmatter evaluates
such object indexes normally, so these equivalent spellings can expand a
pre-selection value into resolve-once graph shell bytes.

The added regression covers only dotted spellings
(`claudine/lib/src/composition/sequence/preflight/tests.rs:1131-1154`). This
violates AC6 and AC7, which require graph-phase target identity to fail with the
typed preflight rejection rather than expand the wrong value.

**Required change:** recognize statically keyed member/index chains as canonical
paths before applying the target-identity policy. Add rejection tests for all
four bracket spellings in primary, setup, teardown, and nested expression
positions. Define and test a conservative policy for computed indexes rooted at
`ctx` or `env` so a dynamic key cannot select a forbidden identity leaf.

Verification level present: Level 1 for dotted paths; none for equivalent index
expressions. Required level: Level 1.

### 2. High: the required route and interpolation-surface matrix is incomplete

The new real-CLI suite contains five Unix-only direct-compose tests
(`claudine/cli/tests/ctx_launch_anchor.rs:1,64,101,125,178,239`). It establishes
direct body values for root/package/opposing/external source locations, one
direct lifecycle case, and one direct `--perf` count. It does not exercise a
loop route. The opposing-area test asserts only the body marker even though its
fixture authors `my_area` in frontmatter; it does not assert effective
frontmatter, preflight-expanded bytes, or lifecycle. There are no real-route
launch-anchor regressions for proxy/re-entry, retry/resume, system-prompt or
appendix relocation, overlay/harness relocation, or conflicting launch/source
file resolution.

These omissions leave the user-observable behavior in AC1, AC2, AC6, AC8, AC9,
and AC10 without the specified Level 1 proof. The implementation plan records
the same unfinished work at `plan.md:258-263`, `plan.md:350-365`, and
`plan.md:414-440`, despite earlier checked boxes claiming the full matrix was
closed at `plan.md:243-257` and `plan.md:402-411`.

**Required change:** add discriminating real-CLI or canonical-owner tests for
each named route and surface. Each fixture must place launch and source values
in deliberate conflict, assert the prepared launch values, and separately
assert that body/file/schema resolution retains source ownership. Do not close
the acceptance-matrix plan items until the named assertions exist.

Verification level present: partial Level 1. Required level: Level 1; these are
semantic composition requirements and do not depend on terminal rendering or
input encoding.

### 3. High: AC5's accounting cannot prove that every consumer received the snapshot

`InvocationWorkSnapshot` counts capture constructions, extensions, and ambient
fallbacks, but it has no per-consumer prepared-context observations
(`claudine/lib/src/invocation_context.rs:41-69`). The CLI perf projection exposes
only those aggregate counts (`claudine/cli/src/perf/report.rs:355-389`). The sole
CLI assertion therefore proves that one direct dry-run constructed one context
and that the outer route recorded no fallback; it cannot prove that preflight,
body, effective frontmatter, and lifecycle each received that context as AC5
requires.

There is also an unobserved fallback inside the canonical preparation service.
When `prepared_context` is absent, `derive_compose_context` captures directly
with Darkmatter (`claudine/lib/src/composition/prepare.rs:163-177`) even when
`PrepareOptions::invocation_context` is present, but it does not call
`record_ambient_fallback`. A future or compatibility caller can therefore drop
the prepared context without increasing the metric that is intended to detect
exactly that failure. The static capture-owner inventory reduces this risk for
known source sites but does not make the runtime AC5 assertion true.

**Required change:** record named consumer observations against the invocation
owner and assert the exact expected consumer set for every route. Make the
canonical fallback observable whenever an invocation owner is available. Add
an end-to-end stabilized-reread case that proves one construction plus the
required extension, unchanged anchor/target overrides, zero fallbacks, and the
complete consumer set.

Verification level present: partial Level 1 for aggregate direct-route counts;
none for per-consumer reuse or the real stabilized-reread extension path.
Required level: Level 1.

### 4. High: native portability and the mandated validation matrix are not established

The only new end-to-end regression module is disabled wholesale on Windows with
`#![cfg(unix)]` (`claudine/cli/tests/ctx_launch_anchor.rs:1`), even though four
of its five tests use no Unix-specific provider process. This prevents the core
path-sensitive matrix from running in `build-win-native` and leaves AC13
without native Windows evidence. The Unix fake-provider lifecycle case should
be isolated behind a narrow platform gate or supplied with a Windows fixture;
it should not disable the portable dry-run tests.

The implementation plan also leaves the required Darkmatter test/lint,
Claudine L2/lint, macOS/Linux/WSL/native-Windows, and hosted-CI gates unchecked
(`plan.md:423-435`). This review's macOS `just test` run is useful Level 1
evidence but cannot substitute for AC14's explicit validation matrix.

**Required change:** make the semantic CLI fixtures portable, add the necessary
platform-specific provider helper for the lifecycle case, and record clean
results for every AC14 gate and environment. Keep L2 terminal/browser fixtures
in the background as required by the specification.

Verification level present: macOS Level 1 only for the new CLI matrix. Required
level: native Level 1 across the specified OS environments, plus the explicit
AC14 L2 regression gate. No Level 3 test is required because this fix specifies
no keyboard, paste, IME, mouse, or terminal-input encoder behavior.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 direct/loop launch-area and lifecycle parity | Level 1 real CLI for direct body and one root lifecycle case | **Gap:** loop and the complete paired lifecycle matrix are absent. |
| AC2 opposing-area behavior on body, frontmatter, preflight bytes, and lifecycle | Level 1 real CLI for body only | **Gap:** three required surfaces are not asserted. |
| AC3 external/outside repository matrix | Level 1 real CLI on macOS | Appropriate level; native portability remains open under AC13. |
| AC4 real CLI capture owner | Level 1 real CLI | Appropriate level for the direct cases that exist. |
| AC5 exact snapshot reuse and every populated consumer seam | Level 1 aggregate counters on one direct dry-run; Level 1 extension unit tests | **Gap:** no consumer accounting or real stabilized-reread route proof. |
| AC6 target identity on direct, proxy, retry/resume, loop, and sequence tasks | Existing Level 1 route tests plus new sequence units, but no fix-specific complete route matrix | **Gap:** the required route matrix is absent and bracket access bypasses graph rejection. |
| AC7 sequence graph/task/source parity | Level 1 unit tests for dotted graph rejection and an external task context | **Gap:** index expressions bypass rejection; the full distributed-source sequence matrix is absent. |
| AC8 proxy/re-entry, retry/resume, and loop epoch parity | Level 1 wiring/unit coverage | **Gap:** no discriminating route-level launch/source or epoch-count proof. |
| AC9 relocated system prompts, appendices, overlays, and harness prompts | Level 1 wiring/unit coverage | **Gap:** relocation cases are absent. |
| AC10 source-owned file and schema resolution under launch/source conflicts | Existing Level 1 file-resolution tests | **Gap:** the specified conflicting launch/source fixtures across changed routes are absent. |
| AC11 no additional discovery | Level 1 invocation units and one direct CLI perf assertion | Partial: retained evidence is covered, but the full route work-bound matrix is absent. |
| AC12 capture-owner guard | Level 1 source inventory plus a seeded-violation test | Appropriate level. |
| AC13 cross-platform path behavior | Unix-gated Level 1 CLI suite plus path-shape units | **Gap:** no native Windows execution of the core regressions. |
| AC14 complete validation matrix | macOS Level 1 `just test` passed during review | **Gap:** required L2, lint, Darkmatter, Linux, WSL, native Windows, and hosted-CI evidence is absent. |

Level 1 is the appropriate behavioral tier for the `ctx.*` value and ownership
requirements because the terminal emulator neither renders nor encodes these
semantics. AC14 nevertheless makes the broader L2 suite an explicit release
gate. No acceptance criterion requires Level 3 OS keyboard injection.

## Verification Run

- `cd claudine && just test` — passed on the macOS review host: all five package
  suites passed (`claudine-catalog-types`, `claudine`, `claudine-contract`,
  `claudine-cli`, and `claudine-gen`). The largest suite reported 4,040/4,040
  library tests and 2,378/2,378 CLI tests passing; configured skips remained.
- Static implementation and regression diff reviewed against base `7d236f7ab`.

## Production Readiness

Not production ready. Fix the graph-preflight bypass, complete the Level 1
route/surface and AC5 consumer-accounting matrix, make the core CLI regressions
portable, and complete AC14 before requesting another review.
