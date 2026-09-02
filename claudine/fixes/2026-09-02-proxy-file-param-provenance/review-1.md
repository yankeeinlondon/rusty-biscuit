---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-02T05:05:02+01:00
spec: 2026-09-02-proxy-file-param-provenance/spec.md
implemented: false
description: A **fix** review of `2026-09-02-proxy-file-param-provenance/spec.md`
fix: 2026-09-02-proxy-file-param-provenance/review-1.md
---

# Review 1: Proxy File Parameter Provenance

## Verdict

The fix is **not ready for production**. The shipped
`implement.md` → `implement-suggestions.md` regression now reaches the fake
provider, immutable caller records participate in Darkmatter's graph/cache
identity, and both package areas pass their Level 1 and lint gates. However,
lazy materialization changes `FileReference` candidate precedence, root unions
are flattened before their applicable arm is selected, and later read failures
cannot retain the raw caller provenance required by the specification. Several
mandatory Level 1 re-entry and diagnostic rows are also absent.

## Findings

### 1. High: lazy binding overrides `FileReference`'s ordered candidate plan

The specification requires a non-recursive lazy local value to materialize the
first candidate in `FileReference::candidate_plan`. The implementation instead
searches the complete plan for the first candidate whose provenance is
`Source`, falling back to the first candidate only when no source candidate
exists (`darkmatter/lib/src/markdown/compose/schema_validation.rs:697-725`).

That changes established resolution precedence. For an implicit relative
reference, `FileReference` orders the repository root before the authoring
base. If both candidates exist—or if the repository candidate is intentionally
the first unprobed lazy identity—the implementation silently chooses the later
source candidate. Magic paths and any future multi-root form have the same
architectural risk because this call site is re-ranking a plan owned by
`biscuit-file`.

The current tests do not expose the difference: each lazy fixture has only the
source-side file of interest and asserts the source result. This violates D2
and acceptance criteria 6, 11, 13, and 14.

**Required change:** consume the candidate plan in its authoritative order.
If caller-owned lazy implicit references intentionally need authoring-base-first
semantics to make the shipped area-relative workflow work, model that as an
explicit `FileReference` policy/API and reconcile it with the specification;
do not select a provenance class ad hoc in Darkmatter. Add Level 1 collision
tests for implicit, magic, package, and explicit-relative references.

### 2. High: root-union applicability is discarded before file-arm selection

`collect_property_schema_fragments` recursively collects the named property
from every root `allOf`/`anyOf`/`oneOf` arm without testing which root arm
applies (`schema_validation.rs:614-633`). `resolve_caller_file_overrides` then
requires exactly one collected fragment to classify as a file
(`schema_validation.rs:470-509`).

A valid discriminated root union with `spec: file(...)` in two arms therefore
produces two selections and skips materialization, even when normal validation
selects exactly one root arm from another discriminator. The raw caller value
then reaches frontmatter interpolation and is reinterpreted from the target
document. The existing union test covers only a property union
(`file(eager) | number`); no root-union caller projection test exists.

This violates D2 and acceptance criteria 2, 5, 9, and 15.

**Required change:** select the applicable root/property/array schema path as
one operation using the same effective-instance semantics as normal coercion
and validation, then derive eager/lazy mode from that selected path. Add Level
1 cases for discriminated root unions with the same file property in multiple
arms, file versus non-file arms, ambiguous arms, and zero-match arms.

### 3. High: later lazy-read diagnostics lose the caller's raw reference and origin

After lazy materialization, `CallerProjection::install` replaces the effective
frontmatter value with the absolute candidate. The expression layer later sees
only that string. Its `file_reference_error` builds diagnostics from the
argument it received and the active document's `ResolutionContext`; it has no
access to the caller record or selected-candidate provenance
(`darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:1447-1464`).

Consequently, when `frontmatter()` or another read-side function encounters a
missing lazy file, the diagnostic identifies the projected absolute path as
the reference and the target document as the resolution context. It cannot
report the original caller spelling, captured origin/base, and selected
candidate together as D7 and acceptance criterion 12 require. The new typed
diagnostic tests cover malformed and eager no-match failures only; there is no
later-read missing-lazy assertion or direct/proxy failure-equivalence process
test.

**Required change:** retain a provenance lookup alongside the semantic
projection through expression evaluation/diagnostic construction, without
adding a global launch fallback. Add Level 1 direct and proxied tests for
malformed, eager-missing, and lazy-read-missing inputs that assert the same
diagnostic code plus raw reference, origin/base, and candidate evidence.

### 4. High: mandatory Level 1 re-entry and layer-isolation evidence is missing

The five new Claudine process tests cover the shipped proxy, direct/proxy
success equivalence, a second proxy hop, proxied retry on Unix, and
inline-compose. The sequence JIT test only proves that a record is copied into
`PreparedComposition`; it never resolves a caller file or routes a sequence
task through a proxy target.

No Level 1 test verifies:

- a proxied **resume** rematerializing the original caller record;
- a loop iteration/reuse retaining the same semantic identity;
- a sequence task routing to a target that reads the caller file;
- CLI caller and sequence/task-authored file values in the same preparation
  retaining distinct origins;
- caller file materialization after a post-capture process-CWD change;
- an absent caller property and a schema default remaining document-owned;
- approval-cache reuse remaining unchanged when caller origins differ; or
- native/presentation identity on a Windows execution path.

Nearby retry/resume state-machine and sequence tests do not supply a caller
file record and therefore cannot prove this feature's provenance contract.
These are semantic/process requirements, so Level 1 is the correct tier, but
the specification explicitly requires this evidence. Their absence violates
D1, D4, D6 and acceptance criteria 4, 7, 10-15.

**Required change:** add the missing non-interactive Level 1 process and
focused library cases. The process tests must use fake providers and remain
focus-safe; no terminal emulator or OS input injection is needed.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| Shipped `implement.md` routes an area-relative `spec` to the lazy target and reaches a provider | Level 1 fake-provider process test | Appropriate and present for the successful shipped route. |
| Router, direct target, and proxied target read the same specification and derive sibling paths | Level 1 Darkmatter and fake-provider process tests | Appropriate for the covered success case; the shipped test asserts iteration and log but not every `review`/`design` projection. |
| Lazy caller files retain authoritative candidate ordering | Level 1 source-only fixtures | **Broken:** the implementation re-ranks the candidate plan (Finding 1). |
| Scalar, array, property-union, and root-union shapes select exactly one applicable file arm | Level 1 scalar, homogeneous-array, and property-union tests | **Broken/gap:** root-union applicability is flattened and untested (Finding 2). |
| Missing/malformed direct and proxied inputs retain raw/origin/candidate diagnostic evidence | Level 1 malformed/eager-missing Darkmatter tests | **Broken/gap:** later lazy reads lose provenance and route equivalence is untested (Finding 3). |
| Caller origin survives proxy, retry, resume, loop, inline-compose, and sequence/task routes | Level 1 process tests for proxy, retry, and inline; carrier-copy unit test for sequence | **Gap:** resume, loop behavior, and real sequence/task resolution are not verified (Finding 4). |
| `proxy.with`, defaults, document values, and sequence/task values keep their ownership and precedence | Level 1 proxy precedence/multi-hop tests and general schema tests | Partial; mixed CLI/task file origins and absent/default caller cases are missing. |
| Equal raw values from different origins have distinct prepared/cache identities while shell approval identity is unchanged | Level 1 graph/cache fingerprint test | Origin-sensitive compose identity is present; approval-cache non-regression has no direct test. |
| Native semantic and portable presentation paths preserve identity on macOS, Linux, and Windows | Level 1 host test plus a `#[cfg(windows)]` assertion | Partial; macOS executed here, but the required Windows route evidence was not observed in this review. |

Levels 2 and 3 are not applicable. The fix changes schema-selected semantic
state, filesystem resolution, process routing, and typed errors. It does not
claim terminal-emulator rendering, glyph width/style, scrolling, paste/IME,
mouse behavior, keybindings, or physical-key encoding. Level 1 library and
non-interactive process tests are the appropriate verification tier.

## Verification Performed

- `darkmatter/just test`: **7,565 passed; 50 skipped**.
- `darkmatter/just lint`: **passed** for `darkmatter`, `darkmatter-cli`, and `dmls`.
- `claudine/just test`: **6,672 passed; 11 skipped**.
- `claudine/just lint`: **passed** for all five package-area crates and the diagnostic guards.
- GitNexus change detection: **high-risk** surface, including Darkmatter compose/schema identity and Claudine direct/proxy/sequence preparation flows.

The green gates show that the covered implementation and existing behavior are
stable on this macOS host. They do not override the semantic defects and
required Level 1 gaps above.

## Closure Criteria

Resolve Findings 1-3, add the missing Level 1 rows in Finding 4, and rerun the
Darkmatter and Claudine package-area test/lint gates plus the specified root
gates. Production readiness requires authoritative candidate ordering,
applicable root-union selection, provenance-complete direct/proxy diagnostics,
and verified preservation through every required re-entry route.
