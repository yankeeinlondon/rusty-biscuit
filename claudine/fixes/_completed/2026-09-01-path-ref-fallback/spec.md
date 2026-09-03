---
status: draft
created: 2026-09-01
area: claudine
packages:
    - claudine
reviewed: true
implemented: true
review_iterations: 1
reviewed_by: codex/default
reviewed_on: 2026-09-01
---

# A failed explicit operation-file reference is laundered through prompt autocomplete instead of reporting file-not-found

## Summary

When `claudine compose|inline-compose|sequence <file>` cannot resolve its
operation-file reference, the CLI currently hands every clean no-match to the
interactive autocomplete picker as a literal substring query. That recovery is
useful for a bare discovery token such as `access`, but it is the wrong contract
for an explicit reference such as `./docs/unifi/access.md`, `docs/unifi/access.md`,
`@access.md`, or `C:\\docs\\access.md`: the user selected reference semantics and
needs to know which concrete candidates those semantics tried.

For the motivating `./docs/unifi/access.md` case, the fallback is nonsensical by
construction:

1. `path_matches_query` performs a literal substring match, while walked
   candidate paths do not contain the authored `./` prefix; and
2. the picker searches its configured high-profile scopes, not the directory
   selected by the explicit-relative reference.

The user therefore sees `CompositionError: no autocomplete matches …` for a
plain wrong-CWD or mistyped path. In a non-interactive session the same failure
reports `autocomplete not available`. Neither diagnostic states the file
reference that failed, the launch directory against which it was resolved, or
the candidates the shared `FileReference` resolver actually probed.

This fix makes autocomplete an explicitly narrow recovery policy: only a
non-recursive, unsigiled, single-component implicit reference may enter the
picker. Every other well-formed reference that cleanly misses produces the
existing `composition.invalid_file_reference` diagnostic, enriched from
`FileReference::resolve_detailed` with the ordered probe record and a bounded,
best-effort basename suggestion search.

> **Reader's note (inline review, 2026-09-01):** The initial draft proposed a
> second file-not-found variant/code, raw prefix-based classification, and
> normalization inside the shared `path_matches_query` predicate. The reviewed
> design instead preserves the established `composition.invalid_file_reference`
> identity, makes `FileReference` the grammar authority, and leaves the shared
> matcher unchanged. This avoids splitting one authoring mistake across two
> diagnostic codes and avoids changing schema-file matching as a side effect of
> an operation-file routing fix.

## Observed behavior (verified 2026-09-01, installed Claudine of 2026-08-28)

Incident: `inline-compose ./docs/unifi/access.md -y --claude --model opus`
failed with "no autocomplete matches" while the byte-similar `protect.md`
composed successfully. The files were identical in every relevant way; the
actual difference was the invoking shell's CWD. From `homelab/` all three
documents resolve and compose; from the repository root the relative path does
not exist and the fallback produces the incident error verbatim:

```console
$ cd <repo root>
$ claudine inline-compose ./docs/unifi/access.md -y --dry-run   # under a PTY
 CompositionError: no autocomplete matches
┃
┃ No files matched autocomplete query `./docs/unifi/access.md`.
┃
┃ Check the query token or run without a query to see all candidates.
```

The same invocation without a TTY reports `autocomplete not available`.
Neither message mentions the candidate path, the launch directory it was
resolved against, or that the file simply was not found there. The user
reasonably concluded that the documents were being treated differently;
diagnosing the real cause required reproducing from multiple directories.

## Root cause

The ENTER-path autocomplete introduced by the 2026-06-14 auto-complete feature
was designed to discover prompt files from bare name queries. Its two operation
entry seams (`commands/compose/prep.rs` and `commands/sequence.rs`) instead key
only on `CompositionError::FileNotFound`, so every clean `FileReference`
no-match is converted into a picker query:

- `autocomplete_operation_file` gates on TTY availability; it receives no
  typed reference classification.
- `path_matches_query` lowercases the provided query and requires it to occur
  literally in a candidate path. An authored anchoring prefix such as `./`
  therefore guarantees zero matches.
- `resolve_compose_scopes` walks configured prompt scopes for all three modes,
  plus repository `docs/` and agent-skill peers for `inline-compose` and
  `sequence`. These are discovery scopes; they do not and must not reinterpret
  the directory selected by an explicit reference.
- `resolve_composition_source_in_context` and the YAML branch of
  `resolve_sequence_source` call the convenience `resolve_in_context` API. Its
  clean `Ok(None)` projection discards the ordered candidates and provenance
  already available from `resolve_detailed`, leaving `FileNotFound(String)` too
  weak to render the actual resolution attempt.

The resulting `AutocompleteNoMatches` or `AutocompleteNotInteractive` describes
the failed recovery mechanism rather than the user's error.

## Design decisions

### D1 — Parse first; autocomplete only a bare discovery name

The fallback must parse the original token with `FileReference::new()` and use
`FileReference::class()` as the reference-kind authority. It must not recreate
the shared grammar with `starts_with` checks.

A failed token is eligible for operation-file autocomplete only when all of
the following are true:

- its parsed kind is `FileReferenceKind::ImplicitRelative`;
- it is not recursive;
- the original implicit payload contains neither `/` nor `\\`; and
- it contains no `{{...}}` environment interpolation.

This admits bare names such as `access` and `access.md`. A multi-component bare
reference (`docs/access.md`) is still an implicit `FileReference`, but it is an
explicit location for recovery-policy purposes and therefore does not enter the
picker. Every sigiled kind (`./`, `../`, absolute, `~`, `@`, `!`, `vault:`, URL)
and every recursive reference is likewise explicit, even when its payload has
only one component. Invalid syntax and missing-context/I/O failures retain their
existing typed errors; only a genuine `ResolutionFailure::NoMatch` is eligible
for either recovery branch.

The `/` and `\\` payload test is deliberately host-independent so a Windows
spelling is classified consistently when unit-tested on macOS or Linux. It is
applied only after `FileReference` has classified the token as implicit; it is
not a competing prefix grammar.

### D2 — Preserve the established diagnostic identity and detailed probe record

Do not add a new diagnostic code. A missing top-level operation file is the same
correctable authoring mistake already represented by
`composition.invalid_file_reference`.

Evolve the existing `CompositionError::FileNotFound` representation (or replace
it with a structured successor that keeps the same semantic identity) so the
no-match retains a typed projection of `FileReference::resolve_detailed` before
the convenience API discards it. Reuse or extract the existing
`harness::ResolutionDetail` projection rather than defining a second candidate
model. The diagnostic must populate the already-declared catalog fields from
typed data:

- `reference`: the exact authored token;
- `kind` and `effective_kind`: the shared resolver's parsed and post-interpolation
  classifications;
- `base_dir`: the captured launch/base directory used for this top-level
  resolution;
- `repository_root`: the captured root, or `null` when unavailable;
- `candidates`: every actually probed candidate, in order, with its existing
  provenance and disposition vocabulary;
- `failure`: `no_match`; and
- `suggestions`: the same ordered list shown to the human, or an empty array.

Compatibility fields declared by the catalog remain present and retain their
established meanings; unavailable values are `null`, never omitted. The change
must not invent a surface-specific code, derive provenance from path text, or
parse `Display` prose.

The terminal `StatusBlock` names the authored reference and the captured base
directory, then renders the ordered candidates with `TerminalRenderable`
components (`Prose` and `UnorderedList` are appropriate). Candidate labels come
from `RootProvenance` (`repository`, `source`, `package`, `home`, `magic`,
`vault`, or `absolute`). At this top-level seam, explain `source` provenance to
the user as the launch directory; do not relabel other roots by guessing from
their path. Render paths with `biscuit_file::to_portable_string`; do not
canonicalize a missing path or reconstruct candidates by joining strings.

Interactive and non-interactive sessions select and render this same semantic
diagnostic for an explicit reference. TTY state may affect styling only, not
the error identity or detail payload.

### D3 — Basename suggestions are repository-local, deterministic, and bounded

For an explicit no-match with a non-empty filename, search for that exact
filename under the captured effective repository root. The motivating case
must be able to suggest `homelab/docs/unifi/access.md` for the missing
`./docs/unifi/access.md` invoked from the repository root.

The suggestion search is diagnostic assistance, not alternate resolution:

- compare the complete filename, not the stem, using exact case-sensitive
  equality for deterministic behavior across filesystems;
- use the existing `.gitignore`, `_`-prefix, and curated directory-skip rules;
- do not follow directory symlinks, so a repository-local diagnostic cannot
  escape its captured root;
- stop after 20,000 visited entries or five matches, whichever comes first;
- deduplicate matches and sort the final portable repository-relative paths
  lexically before rendering and projecting them;
- do not apply compose/inline/sequence frontmatter gates—the suggestion answers
  "where does this filename exist?", and normal source validation remains
  authoritative if the user invokes it; and
- omit the human suggestion section when there is no captured repository root,
  the reference has no filename, no match was found within the budget, or the
  walk cannot continue. Budget exhaustion and walk errors must never replace
  the primary file-not-found diagnostic.

The search must run only after an explicit no-match. It must not broaden
`FileReference` resolution, silently select a different file, or run on a bare
name before the user has had the existing picker opportunity.

### D4 — One recovery policy covers compose, inline-compose, and sequence

The two current fallback seams must share one pure eligibility helper and one
diagnostic-enrichment path so Markdown and YAML sources cannot drift. The
operation mode may still select the existing picker/frontmatter behavior, and
`sequence` may retain its YAML load branch, but all three commands must make the
same recovery decision from the original token and detailed no-match.

`path_matches_query` remains unchanged. Correct routing makes `./` normalization
unnecessary, and modifying that shared predicate would also change the schema
`file(match)` candidate path.

### D5 — Documentation describes recovery, not just the picker UI

Update both the authoritative completion topic
(`claudine/docs/topics/completions/shell-completions.md`) and its portable
Claudine-skill snapshot
(`.claude/skills/claudine/completions/shell-completions.md`). The ENTER-path
section must distinguish:

- an omitted operation-file positional, which is rejected as a missing required
  argument before reference resolution;
- an unresolved bare discovery name, which may open the filtered picker; and
- an unresolved explicit reference, which reports the typed no-match and never
  opens a picker.

The first item corrects existing documentation drift; it does not add missing-
argument autocomplete. Both command paths currently reject the absent positional
before reaching `autocomplete_operation_file`, and changing that behavior is
outside this fix.

Update the composition topic only if it independently states the old fallback
contract. Keep the authoritative topic and skill-local snapshot byte-consistent
for the changed passage.

## Scope

- `claudine/cli/src/commands/compose/` and `claudine/cli/src/commands/sequence.rs`
  — shared recovery routing for direct compose, inline-compose, Markdown
  sequence, and YAML sequence.
- `claudine/cli/src/completion/` — bare-name eligibility and the bounded
  repository-local suggestion walk; no change to `path_matches_query`.
- `claudine/lib/src/composition/resolve.rs` and
  `claudine/lib/src/composition/error/` — retain detailed no-match data, render
  the candidate/base/suggestion report, and project the existing catalog shape.
- `claudine/lib/src/diagnostics/registry.rs` only if comments or tests need to
  record that the previously reserved fields are now populated for top-level
  operation-file misses; no new code or detail keys are expected.
- The authoritative completion topic and its skill-local portable snapshot,
  plus any composition documentation that currently promises unconditional
  fallback.

## Acceptance criteria

- **AC1 (explicit reference → typed no-match).** From a directory where
  `./docs/unifi/access.md` does not exist, each of `compose`, `inline-compose`,
  and `sequence` reports `composition.invalid_file_reference` with the exact
  authored reference, captured base directory, `failure: no_match`, and every
  ordered probe candidate. No autocomplete error text appears.
- **AC2 (all reference forms route intentionally).** L1 table tests cover a
  multi-component implicit reference, POSIX and Windows explicit-relative
  spellings, POSIX absolute, Windows drive-absolute and UNC spellings, home,
  magic, package, vault, recursive, and interpolation-bearing references.
  Every well-formed no-match is ineligible for autocomplete. Invalid,
  missing-context, I/O, URL, and other non-no-match outcomes retain their
  existing typed error rather than being reclassified.
- **AC3 (bare-name discovery remains).** `access` and `access.md` still enter
  the interactive picker after a clean no-match. The existing no-match,
  over-cap, cancellation, non-interactive, confirmation, and chooser behaviors
  remain compatible.
- **AC4 (diagnostic parity).** The human block and `Diagnostic::detail()` expose
  the same candidate order and suggestion order. All declared
  `composition.invalid_file_reference` keys remain present; unavailable values
  are `null` and `suggestions` is an empty array when none were found.
- **AC5 (bounded did-you-mean).** With
  `homelab/docs/unifi/access.md` present, the motivating failure suggests that
  portable repository-relative path. Duplicate hits are removed, results are
  lexically ordered and capped at five, ignored, underscore-prefixed, and
  skip-list trees and symlink escapes are excluded, and scan exhaustion or I/O
  failure leaves the primary diagnostic intact.
- **AC6 (TTY-independent explicit failure).** L2 coverage invokes the same
  explicit miss with and without a PTY and proves the same diagnostic identity
  and substantive body. The PTY case must terminate without waiting for input
  and must use a self-isolating harness that never focuses a terminal window.
- **AC7 (all operation modes).** L1 routing tests cover direct compose,
  inline-compose, Markdown sequence, and YAML sequence; focused L2 coverage
  exercises at least the motivating inline-compose incident and one sequence
  source so the duplicated entry seams cannot drift.
- **AC8 (cross-platform rendering).** Windows separators and absolute forms
  classify correctly on every host. Candidate and suggestion text uses
  `biscuit_file::to_portable_string`; tests do not assume that a foreign-platform
  absolute path can be probed on the current OS.
- **AC9 (shared matcher unchanged).** Existing `path_matches_query` and schema
  file-matching tests remain unchanged and green; no query-prefix normalization
  is introduced.
- **AC10 (documentation parity).** The authoritative completion topic and its
  skill snapshot describe the three outcomes in D5, no longer claim that an
  omitted positional opens the picker, and remain synchronized.

Verification uses the package-area recipes: `just test`, `just test-l2`, and
`just lint` from `claudine/`. Before push, run `just ci-local claudine` from the
repository root. L1 covers pure classification, diagnostic projection,
rendering, walker bounds, and per-mode routing; L2 is required for the real TTY
versus non-TTY contract.

## Non-goals

- **Changing primary `FileReference` resolution.** Candidate construction,
  precedence, and matching remain owned by `biscuit-file`; this fix retains and
  reports the detailed result.
- **Fuzzy path correction.** Suggestions use exact filename equality, not edit
  distance or stem matching.
- **Autocomplete scope expansion.** Existing discovery scopes, ranking,
  frontmatter gates, and picker UI are unchanged.
- **Automatic recovery.** A suggestion is never selected or retried without a
  new user invocation.
- **Changing nested document references.** Proxy, transclusion, schema, and
  lifecycle references already follow their own
  `composition.invalid_file_reference` surfaces and are outside this operation-
  positional routing fix.
- **Adding missing-positional autocomplete.** An omitted operation-file argument
  retains its current required-argument error; only failed resolution of a
  provided token participates in this recovery policy.

## Open Questions

None. The review resolves recovery eligibility, diagnostic identity, suggestion
bounds, and test-tier requirements above.
