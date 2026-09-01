---
status: draft
created: 2026-09-01
area: claudine
packages:
    - claudine
---

# A failed path-shaped operation-file reference is laundered through prompt autocomplete instead of reporting file-not-found

## Summary

When `claudine compose|inline-compose|sequence <file>` fails to resolve its
operation-file reference, the CLI unconditionally hands the failed token to
the interactive autocomplete picker as a literal substring query. For a
path-shaped token — the way users most often spell an explicit target, e.g.
`./docs/unifi/access.md` — that fallback is nonsensical by construction:

1. the query can never match (`path_matches_query` does a literal substring
   `contains`, and no walked path contains the text `./`); and
2. the picker never looks where the user pointed (its scope set walks only
   `prompts/` directories — repo, area, package, `.claudine/`, user — never
   the directory named in the path).

The user therefore sees `CompositionError: no autocomplete matches … Check
the query token or run without a query to see all candidates` for what is a
plain wrong-CWD or typo'd path. The one fact that would resolve the incident
instantly — *"no such file: `<cwd>/docs/unifi/access.md`"* — is never
stated. In a non-interactive session the same failure reports
`autocomplete not available`, which is equally unrelated to the actual
problem.

This fix classifies the failed token: path-shaped references get a typed
file-not-found diagnostic naming the absolute candidates tried, the CWD, and
a bounded did-you-mean; only bare name-shaped tokens enter the autocomplete
picker, whose behavior is unchanged.

## Observed behavior (verified 2026-09-01, installed claudine of 2026-08-28)

Incident: `inline-compose ./docs/unifi/access.md -y --claude --model opus`
failed with "no autocomplete matches" while the byte-similar `protect.md`
composed fine. The files were identical in every relevant way; the actual
difference was the invoking shell's CWD. From `homelab/` all three documents
resolve and compose; from the repository root (a different terminal tab) the
relative path does not exist and the fallback produces the incident error
verbatim:

```console
$ cd <repo root>
$ claudine inline-compose ./docs/unifi/access.md -y --dry-run   # under a pty
 CompositionError: no autocomplete matches
┃
┃ No files matched autocomplete query `./docs/unifi/access.md`.
┃
┃ Check the query token or run without a query to see all candidates.
```

The same invocation without a TTY reports `autocomplete not available`
instead. Neither message mentions the path that was tried, the CWD it was
resolved against, or that the file simply was not found there. The user
reasonably concluded the two documents were being treated differently;
diagnosing the real cause required reproducing from multiple directories.

## Root cause

The ENTER-path autocomplete (2026-06-14 auto-complete feature, Phase 3;
`claudine/cli/src/completion/operation_file.rs`) was designed to help
discover *prompt* files from bare, name-shaped tokens. The entry seam,
however, engages it for **every** failed `FileReference` lookup, with the
raw failed token as the query:

- `autocomplete_operation_file` gates only on TTY-ness
  (`operation_file.rs:46-57`); there is no token classification.
- `path_matches_query` (`completion/scopes.rs:115`) lowercases the query and
  requires it to appear as a substring of a candidate path. Walked paths
  never contain `./`, so any `./`-prefixed reference is guaranteed zero
  matches regardless of what exists.
- `resolve_compose_scopes` (`completion/scopes.rs:306`) walks
  `<repo>/prompts/`, `<area>/prompts/`, `<pkg>/prompts/`,
  `<repo>/.claudine/prompts/`, and `~/.claudine/prompts/` — never the
  directory a path-shaped reference names, so even a `./`-stripped
  `docs/unifi/access.md` could not be found.

Two structurally-blind layers stack, and the resulting errors
(`AutocompleteNoMatches`, `AutocompleteNotInteractive`) describe the
fallback's internals rather than the user's failure.

## Design decisions

### D1 — Classify the failed token before choosing a recovery path

A failed operation-file token is **path-shaped** when it contains a path
separator (`/`, or `\` on Windows), starts with `./`, `../`, or `~`, or is
absolute. Everything else — a bare stem such as `access` or `plan` — is
**name-shaped**. Classification happens once at the seam that currently
invokes `autocomplete_operation_file`, on the original token as typed (not a
partially-resolved form).

### D2 — Path-shaped failures report file-not-found, never autocomplete

A path-shaped failure produces a typed composition error (new variant; follow
the Error Architecture reference before adding it — one discovery seam, the
`Semantic`/`Transparent` role contract, and a catalog code) that renders:

- the reference exactly as typed;
- the absolute candidate path(s) the `FileReference` resolution actually
  tried, in order, each labeled with its anchor (CWD, repo root, magic
  root) so the resolution order is visible rather than implied;
- the invoking CWD; and
- a bounded did-you-mean (D3) when it finds anything.

This variant applies identically in interactive and non-interactive
sessions: TTY-ness no longer changes which error a path typo yields. The
picker is never launched for a path-shaped token — a user who typed a path
asked for that file, not a browse session over prompt directories.

### D3 — Did-you-mean searches for the basename, bounded

When the path-shaped reference's basename (e.g. `access.md`) exists
elsewhere under the effective repo root, list up to a small fixed number of
matches (reusing the existing capped scope walker mechanics, widened to the
repo root for this search only) as `did you mean:` lines rendered as
portable relative paths. The incident case must self-diagnose: running from
the repo root with `./docs/unifi/access.md` names
`homelab/docs/unifi/access.md`. The search is best-effort — cap, skip
ignored/hidden trees per the existing walker rules, and omit the section
entirely when nothing is found or the walk overruns its cap.

### D4 — Name-shaped tokens keep the picker, with the query normalized

Bare tokens retain the existing interactive picker flow and its
`AutocompleteNoMatches`/`AutocompleteOverCap`/`AutocompleteCancelled`/
`AutocompleteNotInteractive` behavior. As defense in depth,
`path_matches_query` receives a normalized query (leading `./` stripped)
so a stray prefix can never again zero out matching — but after D1/D2 no
path-shaped token reaches it.

## Scope

- `claudine/cli/src/completion/` — token classification at the fallback
  seam; did-you-mean basename walk; query normalization; tests.
- `claudine/lib/src/composition/error/` — the new file-not-found variant and
  its render (candidate list, CWD, did-you-mean), per the error-architecture
  contract; catalog code; tests.
- Portable docs: update the autocomplete section of
  `.claude/skills/claudine/completions/shell-completions.md` and the
  composition docs where the fallback is described.

## Acceptance criteria

- **AC1 (path-shaped → not-found).** From a directory where
  `./docs/unifi/access.md` does not exist, `inline-compose` reports the new
  file-not-found error naming the reference as typed, each absolute
  candidate tried with its anchor, and the CWD. No autocomplete error text
  appears.
- **AC2 (TTY-independent).** The same invocation with and without an
  interactive terminal yields the same file-not-found variant —
  `autocomplete not available` no longer surfaces for path-shaped tokens.
- **AC3 (did-you-mean resolves the incident).** With
  `homelab/docs/unifi/access.md` present, the AC1 error includes a
  did-you-mean line naming it as a portable relative path; when no basename
  match exists the section is absent; an over-cap walk omits the section
  rather than erroring.
- **AC4 (name-shaped unchanged).** A bare token (`access`) still enters the
  interactive picker; the existing picker tests — no-matches, over-cap,
  cancelled, not-interactive — remain green unmodified.
- **AC5 (normalized query).** `path_matches_query` matching is proven
  insensitive to a leading `./` on the query.
- **AC6 (Windows spelling).** Classification treats `\`-separated and
  drive-absolute references as path-shaped; candidate and did-you-mean
  rendering uses portable path spelling per the existing Windows
  path-spelling contracts.

Verification through the package-area recipes (`just test`, `just lint`,
`just ci-local` before push); all new tests are L1.

## Non-goals

- **Changing primary `FileReference` resolution.** Which candidates are
  tried, and in what order, is untouched — this fix only reports that order
  honestly on failure.
- **Fuzzy path correction.** Did-you-mean is an exact-basename search, not
  edit-distance matching over the tree.
- **Autocomplete scope expansion.** The picker's prompts-directory scope set
  is intentional for name-shaped discovery and is not widened.
