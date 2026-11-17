# Policy identifier naming

Sidecar to [spec.md](./spec.md). It records what the policy identifiers and
diagnostic codes are called today, where those names travel, and options for
renaming them. Decisions belong in the spec.

## What a user can type today

`md compose --deny` and the shell completions offer two kinds of name for the
same thing (see [inventory.md](./inventory.md) for the full list):

| Kind | Example | Shape | Accepted by |
|---|---|---|---|
| registry identifier | `missing-hyperlinks` | kebab-case, no namespace | `--allow`, `--deny`, `--disable` (per kind) |
| diagnostic code | `dm.reference.missing_hyperlink` | `dm.` + dotted namespace + snake_case | `--deny` only |

Each of the nine recoveries has both: an identifier and the code its warning
carries. `--deny missing-hyperlinks` and `--deny dm.reference.missing_hyperlink`
select the same recovery (the conflict check in `ResolvedPolicy::resolve`
treats them as one). So a user sees every recovery twice, under two naming
schemes.

## Where the `dm.` codes travel

The codes are not only `--deny` arguments. They are the stable
machine-readable identity of a diagnostic, and they leave the CLI:

| Consumer | How it sees the code |
|---|---|
| `md … --output json/yaml/…` | `diagnostics[].code` in every data envelope; `darkmatter/cli/schemas/output/*.yaml` (19 references) |
| DMLS (editors) | the LSP `Diagnostic.code` (`darkmatter/dmls/src/diagnostics/frontmatter.rs:1748`, `router.rs:1464`) |
| `simplified-schema` | emits `dm.schema.*` itself (`simplified-schema/lib/src/validation.rs:145-157`, `triggers/mod.rs:67`, `advisory.rs:42`, `slots.rs:50`) and `dm.expression.invalid_callback` (`expression/parser.rs:797`) |
| Claudine | its tests check for `dm.config.migration` on stderr (`claudine/cli/tests/l1/config_yaml_wiring.rs:63`); Claudine composes through Darkmatter and passes its diagnostics on |
| Docs | `darkmatter/docs/cli/*`, `docs/topics/*`, `docs/inline/*`, `docs/lsp/*`, the `use-darkmatter` caller skill |

There are 132 distinct `dm.*.*` code strings in Rust sources, referenced from
about 190 files (most in Darkmatter's lib, DMLS, and CLI tests).

Observations:

- **The prefix is already inaccurate.** `simplified-schema` is its own crate
  with no Darkmatter dependency, yet its codes say `dm.`. Dropping the prefix
  fixes that misattribution.
- **The prefix is redundant where it is read.** On the `md` command line every
  code is Darkmatter's. A data envelope is produced by `md`. An LSP diagnostic
  has its own `source` field (the server's name), so an editor does not need
  the code to say where it came from.
- **The rest of the namespace earns its place.** `reference.`, `expression.`,
  `schema.`, `collection.`, `link.`, `style.` group a long list (37 deny-able
  codes) so completion and help can be scanned by area.
- Claudine's own names use a `claudine.` prefix (`claudine.lifecycle.*`), but
  those are event names, not diagnostic codes, so no collision arises if
  Darkmatter's codes lose `dm.`.

## Options

### Drop `dm.` from the codes

Recommended. One spelling everywhere: `reference.missing_hyperlink` on the
command line, in data envelopes, and in DMLS. Do not accept both spellings;
the repository has no installed base to keep compatible, and a second
spelling would be one more alias in completions.

Cost: a mechanical rename across the constants, the output schemas, DMLS
fixtures, tests, docs, and the caller skill, plus Claudine's one assertion.
Search for `"dm\.` and `dm\.[a-z_]+\.` to find them.

### Namespace the un-namespaced identifiers?

The un-namespaced identifiers fall into three groups, and the answer differs:

1. **Recoveries (`missing-hyperlinks`, `shell-timeout`, …).** Each is a
   second name for a code. Once each problem lives on exactly one verb list
   (the spec's D2), keeping both the identifier and the code is pure
   duplication. Options:
    - **(a) One name per problem, the code.** `--deny reference.missing_hyperlink`,
      `--allow shell.timeout_recovered`. Every allow/deny argument is
      namespaced and matches the code a report prints, so a user can copy a
      code from a warning straight into `--deny`. The recovery identifiers go
      away.
    - **(b) One name per problem, the short identifier.** Keep
      `missing-hyperlinks` and stop accepting its code on the command line.
      Short to type, but the name on the command line no longer matches the
      `code` in the report.
    - **(c) Keep both** (today). Rejected by the goal of this fix.

   Recommended: **(a)**. If adopted, decide one case style for the last
   segment. The codes use `snake_case` today; the identifiers use
   kebab-case. Either works; switching codes to kebab-case
   (`reference.missing-hyperlink`) touches every code, while keeping
   `snake_case` touches only the identifiers being removed.

2. **Families (`style`).** `style` selects two codes (`style.unknown_key`,
   `style.deprecated`). Under (a) it can become the namespace itself, with
   `--deny style` meaning every code in `style.`. If that is adopted,
   generalize it: `--deny <namespace>` denies every code in that namespace
   (`--deny expression`, `--deny collection`). That replaces a hand-kept family
   list with a rule.

3. **Features and access (`baseline-schema`, `schema-triggers`,
   `collection-blocks`, `host=HOST`).** These are not diagnostics. `--disable`
   takes only features and `host=` is its own form, so neither collides with a
   code, and namespacing them adds typing without removing ambiguity.
   Recommended: leave them un-namespaced.

## Questions for the author

1. Drop `dm.` everywhere (codes, data envelopes, DMLS), with no
   compatibility alias?
2. Recoveries: option (a), the code as the only name?
3. Case of the final segment: keep `snake_case`, or move codes to kebab-case?
4. `--deny <namespace>` as a general rule, replacing the `style` family?
