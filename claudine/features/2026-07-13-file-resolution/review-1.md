---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T15:46:33-07:00
spec: 2026-07-13-file-resolution/spec.md
implemented: true
description: A **feature** review of `2026-07-13-file-resolution/spec.md`
feature: 2026-07-13-file-resolution/review-1.md
previous: /
---

# Review 1: Unified File-Reference Resolution

## Verdict

The feature is **not ready for production**. The shared parser, repository-first
candidate order, fallible probing, proxy-route convergence, and motivating
implicit/explicit cases are materially implemented. However, the implementation
does not carry the detailed resolution record into Claudine diagnostics, the
supposedly explicit context still performs late discovery and ignores its
`package_area`, native-Windows home discovery still depends on `$HOME`, and
several Darkmatter surfaces retain private grammars with observably different
resolution behavior. Required completion and ordered-candidate rendering
verification is also absent.

## Findings

### 1. Critical — Detailed candidates and provenance are discarded before diagnostics

`resolve_harness_path` obtains a `DetailedResolution`, reads only its first
candidate, and immediately projects it back through `into_convenience()`
(`claudine/lib/src/harness/resolve.rs:73-89`). A no-match then becomes
`HarnessError::PathResolutionFailed`, which stores only the raw reference,
failure slug, source path, and one resolved path. Its diagnostic projection
explicitly leaves `kind`, `repository_root`, and `candidates` as `null`
(`claudine/lib/src/harness/error.rs:279-313`). The registry and Darkmatter
projection still say those fields remain null until this feature supplies typed
values (`claudine/lib/src/diagnostics/registry.rs:181-199` and
`claudine/lib/src/composition/error/render/mod.rs:196-213`).

This is the central D8 and Acceptance Criterion 8 contract, not optional
presentation polish. It also means the promoted proxy-parity L2 test does not
compare typed resolution detail; it compares prose fragments while the required
candidate/root fields are absent.

**Required change:** preserve the shared `DetailedResolution` (or a stable owned
diagnostic projection of it) in the Claudine/Darkmatter semantic wrapper. Populate
the authored/effective kind, repository root, ordered candidates, provenance,
probe disposition, and typed failure in `err.detail.*` and machine output. Add
L1 assertions on the complete structured payload and an L2 implicit no-match
case that captures two attempted candidates in repository-then-source order.

### 2. High — `FileResolutionContext` does not enforce the explicit request-scoped contract

The public context contains `package_area`, but `ResolutionContext::from_context`
does not copy it (`biscuit-file/lib/src/file_reference/context.rs:69-77`) and
package resolution instead invokes `cargo metadata` during candidate building
(`biscuit-file/lib/src/file_reference/resolve.rs:643-652`). When an explicit
context has no repository root, `resolve_detailed` also falls through to live
Git discovery from the base on each resolution
(`biscuit-file/lib/src/file_reference/resolve.rs:571-582`). Claudine's external
sequence adapter constructs exactly such a context without a captured repository
root (`claudine/lib/src/composition/sequence.rs:108-112`).

This contradicts D2, D10, and Acceptance Criterion 12: discovery is not captured
once and reused, the package-area field is dead data, and resolution behavior can
still depend on filesystem state observed after context construction. It is also
unnecessarily expensive for documents with many references.

**Required change:** make the explicit context authoritative. Carry and consume
`package_area`; do not run Git or Cargo discovery inside `resolve_detailed` or
`candidate_plan`. Capture repository/package/home/environment once at the
Claudine/Darkmatter request boundary (using `sniff` there), derive nested contexts
from that snapshot, and return typed missing-context outcomes when a required
anchor was not supplied. Keep discovery only in the documented ambient
compatibility methods.

### 3. High — Native-Windows home resolution still uses `$HOME` alone

The feature adds the optional `dirs` dependency and documents it as the
cross-platform provider, but the exported `home_dir()` implementation remains
`std::env::var_os("HOME")` (`biscuit-file/lib/src/file_reference/context.rs:352-355`).
`FileResolutionContext::new`, ambient resolution, magic completion, and shared
`~` resolution all call that function. Native Windows commonly obtains its home
from profile APIs/environment other than `HOME`, so `~` and the HOME leg of
magic search can become a typed missing-context/no-match despite a valid user
profile.

Existing tests override `with_home_dir`; they prove the injected anchor works,
not that default native-Windows discovery satisfies D11 and Acceptance Criterion
11.

**Required change:** use the declared cross-platform provider (for example
`dirs::home_dir()`) at the capture boundary and add a Windows-target test that
does not set `HOME` but still resolves `~/...` from the native profile directory.

### 4. High — The precedence flip breaks nearest schema-root selection

`try_bare_name_in_roots` promises to search configured schema roots nearest
first, but it calls repository-first `resolve_from(root)` for each root
(`darkmatter/lib/src/markdown/schemas/resolve.rs:280-303`). If a schema root is
`<repo>/schemas` and both `<repo>/schema.yaml` and
`<repo>/schemas/schema.yaml` exist, the first call selects the repository-root
file before it probes the configured schema root. The loop's advertised root
order is therefore bypassed by the new implicit policy.

This is precisely the kind of caller the D12 audit marked as requiring an
explicit transition policy. It can load and validate against the wrong schema
without producing an error.

**Required change:** make each schema-root probe explicitly pinned to that root
(or supply a shared policy/candidate plan whose sole source root is the current
schema root). Add a collision test with distinct schemas at the repository root
and nearest configured schema root.

### 5. High — Darkmatter still has private path grammars and source-first fallbacks

Acceptance Criteria 1 and 6 require `FileReference` to be the only syntax
authority across Claudine-executed Darkmatter surfaces. Two important bypasses
remain:

- Missing-path expression functions fall back to `resolve_path_shape`, which
  manually branches on `Path::is_absolute`, `./`, `../`, `@`, `!`, and `vault:`
  and returns `base_dir.join(path)` for a bare implicit value
  (`darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:1610-1680`).
  That is source-first behavior for a missing implicit path, not the shared
  repository-first candidate shape.
- Local Markdown link resolution suppresses resolver errors and, after a miss,
  directly joins the raw value to the source directory
  (`darkmatter/lib/src/markdown/compose/link_resolve.rs:122-165`). This is the
  fallback bypass called out in the specification's Current Drift section.

There are additional prefix classifiers in transclusion and schema resolution,
including local `@/` rewriting. These are not thin typed adapters over one
grammar; they can disagree with parsing, interpolation, Windows spellings, and
candidate provenance.

**Required change:** use `FileReference::class`, `candidate_plan`, and detailed
resolution for both existing and missing targets. If path-shape functions must
return a non-existent path, select the first candidate from the shared plan
rather than re-parsing the raw string. Preserve surface-specific URL fetch
policy after shared classification. Extend the boundary guard across the actual
Claudine-executed Darkmatter modules instead of checking only three proxy routes.

### 6. High — User-observable completion and ordered-error requirements are under-verified

The new tmux coverage correctly proves the motivating implicit success and
explicit failure, but the remainder of the specification's Level-2 strategy is
not present:

- No L2 test renders an implicit no-match with both ordered attempted
  candidates; the no-color proxy test only checks headline/reference/failure
  prose, and Finding 1 shows the structured candidates are null.
- Completion-produced references are only declared "verified" by static
  inspection in the plan. No test takes an emitted completion value and executes
  it unchanged. `FileReference::complete_partial` also has a separate root
  implementation and no `FileResolutionContext`, so configured magic/package
  roots cannot be shared with execution.
- The plan cites a Darkmatter "Phase 5 L2 suite" for schema, expression,
  sequence, transclusion, and nested-base parity, but this feature added no
  Darkmatter L2 test file and the Claudine collision test exercises transclusion
  only. The other surfaces have L1 coverage, not the promised shared-fixture L2
  verification.

Under the review rubric these are production-readiness gaps. Ordered terminal
rendering specifically requires Level 2; interactive completion needs at least a
process/PTY execution round trip and Level 2 where the real terminal chooser is
part of the behavior.

**Required change:** add the missing L2 ordered-candidate capture and
completion-to-execution round trip. Either add the specified shared-fixture L2
matrix for schema/expression/sequence/transclusion/nested contexts or narrow the
spec with a reasoned explanation of why L1 process integration is sufficient
for the non-rendering portions.

### 7. Medium — Public documentation still contradicts the implemented grammar

The completion API documentation says implicit roots are base then Git root
(`biscuit-file/lib/src/file_reference/mod.rs:323-325` and `:705-713`), while the
implementation is repository then base. `FileReferenceKind::Home` and Claudine
harness/sequence docs advertise `~foo`, although parsing deliberately rejects
`~user`-shaped forms and accepts only `~`, `~/...`, and the Windows backslash
form. These comments drift from the code and fail Acceptance Criterion 7's
documentation-alignment requirement.

**Required change:** correct the public rustdoc and add doc tests or parity tests
for the published order and accepted home spellings.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| Pure classification, explicit/implicit distinction, candidate order, deduplication, interpolation anchoring, and fallible probing | Level 1 unit/integration tests in `biscuit-file` | Appropriate for the shared in-process algorithm; the full `biscuit-file` L1 suite passed. It does not cover the downstream losses above. |
| Motivating bare proxy resolves repository-first; paired `./` proxy does not fall back | Level 2 tmux tests in `level2_file_resolution_capture.rs` | Appropriate and well-discriminated. The focused L2 test was not independently completed in this review because the cold build exceeded the command ceiling. |
| All proxy routes share diagnostic identity and typed resolution detail | Level 2 tmux prose assertions | **Gap:** route headlines/hints are exercised, but candidate/root detail is discarded and therefore cannot be compared. |
| Schema, expression, sequence, transclusion, and nested-document resolution share one contract | Primarily Level 1 unit/process integration; one Claudine transclusion collision process test | Level 1 is sufficient for pure resolution semantics, but the specified cross-surface fixture matrix is absent and concrete schema/expression/link divergences remain. |
| Missing-reference terminal report shows ordered candidates | Level 2 status-block/no-color capture | **Gap:** no two-candidate ordered capture; the structured candidate field is null. Level 2 is required for the terminal rendering claim. |
| Completion output resolves unchanged through execution | Static code inspection plus separate completion unit tests | **Gap:** no completion-to-execution test, and completion does not consume the same explicit context/candidate builder. |
| Native macOS/Linux/Windows absolute and home behavior | Level 1 host-independent parser cases plus injected-home tests on macOS | **Gap:** parser coverage is useful, but default native-Windows home discovery is broken and no native-Windows behavior test is present. |
| Package gates pass | Completed `biscuit-file` L1/lint; partial Darkmatter/Claudine attempts; implementation record claims green | **Gap:** this review did not independently complete all three areas or relevant L2 gates, and static acceptance blockers remain regardless. |

Level 3 is not applicable. The feature does not specify physical keyboard,
terminal input encoding, paste, IME, mouse, or hotkey behavior.

## Verification Performed

- Read the specification, execution plan, decisions, migration inventory, shared
  resolver implementation, Claudine adapters/diagnostics, Darkmatter migration
  surfaces, and feature-specific L1/L2 tests.
- `biscuit-file/just test` passed: 337 library tests and 61 CLI tests.
- `biscuit-file/just lint` passed for both library and CLI.
- Started the three full L1 area gates concurrently. The cold builds exceeded
  the non-interactive command ceiling and were stopped; Darkmatter had completed
  1,771 of 5,607 tests with no observed failure, while the other areas were
  still compiling/waiting on shared Cargo locks. These are incomplete runs, not
  product failures.
- Started the focused motivating L2 test through `claudine/just test-l2` as
  required by the harness guidance. Its cold build exceeded the command ceiling
  before test execution and was stopped cleanly, so no independent L2 result is
  claimed here.
- Preserved the pre-existing unrelated `CLAUDE.md` worktree modification.

## Production Readiness Closure

Production readiness requires all high/critical findings to close: carry the
detailed resolver record through diagnostics, make explicit contexts truly
authoritative, fix native-Windows home discovery, restore schema-root isolation,
remove remaining private grammars/fallback joins, and add the missing
verification. Then rerun complete `just test`, `just lint`, and relevant
`just test-l2` gates in all three package areas.
