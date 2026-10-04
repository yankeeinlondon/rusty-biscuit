---
$schema: feature-review.yaml
ready: false
findings:
  - title: Path normalization disagrees at filesystem roots
    priority: high
  - title: Relative context directories bypass configuration validation
    priority: high
  - title: Failed search strategies disappear from attempt history
    priority: medium
  - title: Absolute fallback links omit the required consumer warning
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-01T02:06:54-07:00"
spec: "2026-09-30-reusable-path/spec.md"
implemented: true
next: "2026-09-30-reusable-path/review-2.md"
description: "A **feature** review of `2026-09-30-reusable-path/spec.md`"
feature: "2026-09-30-reusable-path/review-1.md"
---

This feature is **not production ready**. The main strategies and consumer migration are implemented, but four reproduced defect classes remain. None requires a new design decision or human-only testing.

The review covers the specification, implementation log, portable-path modules, context selection and derivation, resolver boundaries and completion, Darkmatter normalization, and the Claudine context consumers. Existing working-tree edits were preserved. Temporary reproduction tests used the implementation's isolated filesystem fixtures and public APIs; they were removed after collecting results. This review changes no implementation code.

## Findings

### High — Path normalization disagrees at filesystem roots

**Defect class:** Two lexical normalization implementations disagree about parent components at a filesystem root, so valid absolute inputs can become relative paths and valid anchors can lose their boundary.

In `biscuit-file`, [PathIdentity](../../lib/src/file_reference/portable/path_identity.rs) compares paths and computes portable routes. It correctly clamps `/..` at `/`. However, [normalize_components](../../lib/src/file_reference/resolve.rs:1298), used by resolution and context selection, unconditionally pops the previous component for `..`, including the root component. The specification explicitly requires normalization without walking above a root and one shared path identity implementation.

**Reproduction:** Copy the existing portable-path `Fixture`, create `repo/docs/x.md` and `home/x.md`, and use its canonical temporary root. Change only a path or anchor from `/private/…` to `/../private/…`. Both native spellings name the same file on macOS. Use the captured context throughout; no environment changes are needed.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `PathIdentity` equality | Target prefixed with `/..` | Equal to the original target | Equal; clean control |
| `PortablePath::from_path`, only `AbsolutePath` | Same changed target | `NoStrategyMatched`; attempt says `Shadowed`, with a relative `resolves_to` | A verified absolute reference |
| `FileReference::resolve_in_context` and `resolve_detailed` | Absolute reference prefixed with `/..` | Report a match whose path is `private/…`, without `/` | Match the absolute target |
| `PortablePath::from_reference`, default strategy | Same absolute reference | `NoStrategyMatched` after accepting that relative resolved target | Same result as the unedited reference, `./x.md` |
| `FileReference::candidate_plan` | Same absolute reference | Retains the authored absolute candidate | Faithful candidate; clean, but execution disagrees with its plan |
| Context containment, changed CWD | `/../…/repo/docs`, ordinary explicit tree root | `CwdOutsideBaseDir` | Accept the contained absolute CWD |
| Context containment, explicit tree root | Ordinary CWD, `/../…/repo` root | `CwdOutsideBaseDir` | Accept the containing root |
| Context containment, repository root | Ordinary CWD, `/../…/repo` repository | `RepositoryRootNotContainingSource` | Accept the containing repository |
| Containing-vault selection | Vault root prefixed with `/..` | Selects `Fallback`, losing the established boundary | Select `Vault` |
| `HomeDir` generation | Home anchor prefixed with `/..` | `NoStrategyMatched`, with a relative shadow target | `~/x.md` |
| `EnvRootedPath` generation | Declared `ROOT` value prefixed with `/..` | `NoStrategyMatched`, with a relative shadow target | `{{ROOT}}/x.md` |
| Opening-reference tree selection, home | Same changed home anchor, accepted absolute source | Selects `Fallback` | Select `Home` |
| Opening-reference tree selection, environment | Same changed `ROOT`, accepted absolute source | Selects `Fallback` | Select `Environment` |
| `MagicPath` generation, configured root isolated | Search root prefixed with `/..`, home removed | Root chain contains a relative root; writer reports `NotUnderMagicRoot` | Absolute search root and verified `@x.md` |
| `MagicPath` generation, alternative home root retained | Same changed configured root plus unchanged home | Successfully returns `@x.md` through the remaining home root | Same target; clean result masks the bad configured root |
| Repository catalog scope selection | Valid catalog, source directory prefixed with `/..` | No repository scope | Same containing repository as the original source |
| Repository catalog construction | Catalog root prefixed with `/..` | `RootNotNormalized` | Same; clean because catalog inputs explicitly require normalized roots |
| `resolve_relative` projection | Valid absolute reference, output base prefixed with `/..` | `RelativePath` error | `x.md` relative to the equivalent base |
| Recursive resolution | Existing absolute target prefixed with `/..`, `%` modifier | Matched path loses its leading `/` | Absolute matched target |
| Context-aware bare completion | Fallback CWD prefixed with `/..`, token `x` | Retains the absolute authored completion root | Equivalent directory; clean |
| Completion classification | Absolute token prefixed with `/..` | `Ok(None)` | Absolute tokens are unsupported; clean |

The unedited target resolves absolutely and generates `./x.md`; the unedited anchors are also covered by the existing tree-root tests. The shared helper also serves recursive filters and candidate deduplication, so those operations must retain their documented contracts when normalization is consolidated. Do not blanket-collapse Windows verbatim components: those dot segments are literal names.

**Required change:** Make resolver and context normalization agree with the shared lexical identity rules while retaining native path spelling. Add public-result regression tests for the rows above, including an excess-parent path at a drive root on Windows. Existing root-clamping identity tests alone cannot catch disagreement with resolution.

The faulty resolver helper predates this feature; the new writer and tree-selection logic now rely on it despite introducing a different, correct normalization definition. This is a gap in the feature's shared-identity and round-trip requirements.

### High — Relative context directories bypass configuration validation

**Defect class:** Context validation checks containment but not absolute-directory invariants, allowing malformed captured contexts to succeed or to depend on the live process directory.

In `biscuit-file`, [FileResolutionContext::validate](../../lib/src/file_reference/context.rs:1246) validates the request and document trees. [PortablePath::prepare_context](../../lib/src/file_reference/portable/evaluate.rs:177) relies on that validation for `with_ctx`, while its separate directory builders use an absolute-path check. The specification requires invalid contexts to fail before any preference runs, even when `AbsolutePath` could succeed, and says the fallback exception does not remove basic absolute-path validation.

**Reproduction:** Keep the existing fixture's absolute target unchanged, replace a context directory with `relative`, and evaluate using only `AbsolutePath`. For paired directories, use `relative/docs` inside `relative` so the containment check has a positive control.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `from_snapshot` supplied through `with_ctx` | Relative CWD, fallback tree | `validate()` succeeds; writer returns the absolute target | Configuration error before evaluation |
| `new` supplied through `with_ctx` | Relative CWD | Writer succeeds | Configuration error |
| Explicit non-repository tree | Relative CWD and relative containing `base_dir` | Validation and writer succeed | Configuration error |
| Repository tree | Relative CWD and relative repository root | Writer succeeds | Configuration error |
| `for_cwd` | Relative destination from a relative fallback context | Writer succeeds | Invalid context rejected |
| `for_source` | Relative source parent from that fallback context | Writer succeeds | Invalid context rejected |
| `for_trusted_external_cwd` | Relative destination from a valid repository context | Writer succeeds | Invalid destination rejected; trust permits changing trees, not relative directories |
| `for_trusted_external_source` | Relative source parent from that valid context | Writer succeeds | Invalid destination rejected |
| Repository root with an absolute CWD | Relative repository root | Configuration error through failed containment | Rejected; clean for this shape |
| Explicit tree root with an absolute CWD | Relative `base_dir` | Configuration error through failed containment | Rejected; clean for this shape |
| `PortablePath::with_cwd` | Relative CWD | `InvalidConfiguration(RelativeDirectory)` | Same; clean |
| `PortablePath::with_base_dir` | Relative root with valid absolute CWD | `InvalidConfiguration(RelativeDirectory)` | Same; clean |

A relative captured home also passes `validate()` and absolute evaluation; validate the absolute-anchor contract consistently rather than only when a strategy happens to use an anchor. Relative environment values and relative configured magic roots have explicitly different contracts and must remain supported under their existing rules.

This matters beyond an incorrect error variant: filesystem probes of a relative candidate use the current process directory. A supposedly captured snapshot can therefore resolve differently after a process-directory change. The existing [configuration tests](../../lib/tests/l1/portable_path/configuration.rs) test the direct writer builders and containment, but omit relative directories supplied through a context.

**Required change:** Validate required request/document directories and absolute tree anchors centrally, including trusted derivations, before strategy execution. Cover both writer constructors, both normal derivations, and both trusted derivations through public errors. Ensure the reference-aware derivation wrappers, which share the same derivation implementation, inherit the validation.

### Medium — Failed search strategies disappear from attempt history

**Defect class:** A strategy-level I/O failure exits evaluation before recording the attempted preference, losing the structured explanation of what was tried.

In `biscuit-file`, [Evaluation::run](../../lib/src/file_reference/portable/evaluate.rs:395) returns `ProbeFailed` before constructing the current attempt. The error retains the failing filesystem path and error kind, but not the preference that tried it. The specification requires one attempt per preference tried on both success and error.

**Reproduction:** Copy the portable-path fixture, keep `repo/docs/x.md` as an existing file, and create a regular file at `repo/blocker`. For magic search, register that file as a local prepended root. For repository search, set it as the package root. Evaluate each search preference alone. The first candidate becomes `repo/blocker/docs/x.md` and fails with `NotADirectory`.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `MagicPath(None)` | Prepended root is a regular file | `ProbeFailed(NotADirectory)`, `attempts() == []` | One failed `MagicPath` attempt with typed probe information |
| `RepoMultiPath(None)` | Package root is a regular file | Same error and empty attempt history | One failed `RepoMultiPath` attempt |
| `SameDirRelative`, `RepoRoot`, `AbsolutePath` | Same target and unrelated bad search-root configuration | Successful result with one attempt | Same; clean |
| `ChildDir`, `PeerDir`, `ImmediateParentDir`, `ParentDir`, `ExternalRelativePath` | Same target and configuration | Typed route/tree non-applicability, one attempt | Same; clean |
| `HomeDir`, `EnvRootedPath` | Same target without an eligible anchor | Typed non-applicability, one attempt | Same; clean |
| Authored-intent lookup | Existing test's `&docs/blocker/x.md` input | Kept reference with a typed resolution finding and matched intent attempt | Same; clean |
| Initial target probe | Existing test's path input ending in `blocker/x.md` | Typed `ProbeFailed` before candidate generation | Retain preparation failure; distinguish it from a failed strategy |

Both searched preferences use the same `searched`/`verify` path. Every target preference also shares the early-return branch, so fixing only one search method would leave the structural defect intact. Multiple candidate rejections collected before the failing candidate are likewise lost by that return.

**Required change:** Record the failing preference and any earlier candidate rejections before returning the error. Add public-error assertions for both search forms and a search that first encounters a shadowed spelling, then an I/O failure. Preserve the existing typed path, error kind, and OS code.

### Medium — Absolute fallback links omit the required consumer warning

**Defect class:** The consumer suppresses the diagnostic for a non-portable absolute fallback instead of warning by default as specified.

In `darkmatter`, [normalize_links](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs:210) applies the portable writer to document destinations. It warns for evaluation errors and some unsupported Windows spellings, but silently continues for an ordinary `AbsolutePath` result. The specification's “Handling Non-Portable Paths” contract requires consumers to warn by default; callers can suppress that warning or disallow the fallback. The implementation's [silent-fallback test](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs:676) asserts the opposite behavior.

**Reproduction:** Copy the existing `detached_options` fixture, create `other/x.md` outside its fallback tree, remove home and portable environment anchors, and change only the destination syntax around that same absolute path. Call the public normalization operation and inspect `ComposeReport`.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Markdown hyperlink | `[x](ABSOLUTE)` | Unchanged, no warning | Keep destination and report absolute fallback |
| Markdown image | `![x](ABSOLUTE)` | Unchanged, no warning | Same warning |
| HTML hyperlink | `<a href="ABSOLUTE">` | Unchanged, no warning | Same warning |
| HTML image | `<img src="ABSOLUTE">` | Unchanged, no warning | Same warning |
| Video | `<video src="ABSOLUTE">` | Unchanged, no warning | Same warning |
| Audio | `<audio src="ABSOLUTE">` | Unchanged, no warning | Same warning |
| Media source | `<source src="ABSOLUTE">` | Unchanged, no warning | Same warning |
| Iframe | `<iframe src="ABSOLUTE">` | Unchanged, no warning | Same warning |
| Script import | `<script src="ABSOLUTE">` | Unchanged, no warning | Same warning |
| Stylesheet import | `<link rel="stylesheet" href="ABSOLUTE">` | Unchanged, no warning | Same warning |
| Font import | `<link rel="preload" as="font" href="ABSOLUTE">` | Unchanged, no warning | Same warning |
| Library result | Same target through `PortablePath` | Exposes `AbsolutePath` and attempts; prints nothing | Correct; the library supplies the diagnostic |
| Evaluation failure | Existing normalization test's `NotADirectory` input | Preserved destination plus warning | Correct control |

All eleven destination forms converge on the same fallback branch. Claudine consumes Darkmatter composition; it has no separate document-link normalizer. Its shell-completion text generators have a different contract and are clean exclusions from this class.

**Required change:** Warn through the composition report for ordinary absolute fallbacks, with an explicit suppression option if needed. Update the silent-fallback test and the current normalization documentation together. This warning explains that the resulting document still contains a link tied to this host.

## Verification and coverage

Executed on macOS:

- `just test` in `biscuit-file`: 973 Level 1 tests passed. Its recipe also passed all six no-default-features path-text checks.
- `just lint` in `biscuit-file`: passed.
- `just check-tier-coverage biscuit-file`: passed, zero stranded tests. The portable-path and file-tree modules are declared in the consolidated `l1` target; CI enables `fetch` for the separate fetch target.
- `just test link_normalization` in `darkmatter`: 20 existing tests passed.
- The Darkmatter area Level 1 suite also ran: 8,744 passed, 12 skipped, including one temporary review probe. The temporary probe was subsequently rerun with captured output, and removed.
- Temporary public-API probes reproduced the instance tables above through nextest, using the existing isolated fixture helpers.

| User-facing requirement | Strongest relevant verification present | Assessment |
| --- | --- | --- |
| Tree-root precedence, origins, fallback exception, normal/trusted derivation, unchanged launch search scope | Level 1 filesystem integration tests in `file_tree` and consumer tree-root tests | Appropriate level; root normalization and invalid-directory cases are missing |
| Relative boundaries, environment expansion, symlink landing, reader opt-in, repository-only sigils, recursive-root restriction, completion containment | Level 1 public resolver tests in `file_tree` | Appropriate level; the findings above expose missing normalization/validation cases |
| Strategy order, all relative route shapes, filters, shadowing, missing/non-file targets | Level 1 public-result tests in `portable_path` | Appropriate level; failed-attempt history lacks assertions |
| Intent preservation, unsupported URL/recursive normalization, minimal churn, idempotence, candidate equality | Level 1 input and property tests | Appropriate level; root-parent spellings are absent from the corpus |
| Portable environment declarations, eligibility, ties, deduplication, captured state | Level 1 environment matrix and property tests | Appropriate level; root-parent anchor spelling is missing |
| Non-Unicode names, Unix backslashes, interpolation in filenames, Windows drive/UNC/verbatim identity and spelling | Level 1 platform and internal text/identity tests, with Windows-gated cases | Appropriate level; add resolver checks at filesystem roots |
| Darkmatter destination rewriting, suffix retention, transcluded child links, environment-reference recomposition | Level 1 library and CLI integration tests | Appropriate level; fallback-warning expectation contradicts the spec |

No new behavior depends on keyboard injection, terminal input encoding, scrolling, or styled pane rendering. Level 2/3 tests are therefore not required for these path and document-output contracts. This readiness decision does not depend on obtaining cross-OS execution evidence.

The file-format input robustness matrix is not applicable: the feature adds no manifest, lockfile, or serialized configuration reader. Its environment declaration/value matrix already tests the supported string inputs through public results; relative environment values remain valid reader inputs and merely ineligible writer anchors.

The documented departures concerning richer attempt records, boxed errors, spelling from a selected magic root, and passing absolute resolved destinations into Darkmatter normalization are understood. Authored intent is preserved by the library but is already lost before Darkmatter finalization; maintaining that intent through the entire compose pipeline would require additional provenance and is not reported as an unimplemented requirement here. No new dependency, unnecessary feature flag, or separate performance blocker was found.
