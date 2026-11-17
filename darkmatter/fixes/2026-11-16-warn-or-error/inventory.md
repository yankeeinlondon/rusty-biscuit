# Policy inventory: what each `--allow`/`--deny` item does

Sidecar to [spec.md](./spec.md). It lists every identifier and diagnostic code
that `--allow` and `--deny` offer, as shown by `md compose --help`,
`md render --help`, and the shell completions. All three come from the same
source:

- `POLICY_REGISTRY` in `darkmatter/cli/src/args/policy.rs` (14 identifiers);
- `COMPOSE_DIAGNOSTIC_CODES` (33 codes) and `RENDER_DIAGNOSTIC_CODES` (4 codes) in
  `darkmatter/lib/src/markdown/compose/policy.rs`.

So the list a user sees is exactly the list the code accepts. This page records
whether each item **does anything**, what its default is, and a proposed
disposition. The evidence, with file and line for each claim and probes run
against the built `md`, is in [audit/registry.md](./audit/registry.md) and
[audit/codes.md](./audit/codes.md). Audited at HEAD `baee016e5`, which includes
`11a909a82`.

**Legend.** *Default* is what happens with no flag. *Works?* answers "does
using this item change the outcome?"

## Summary

Of the 51 entries offered (14 identifiers and 37 codes, two of which are on
both the compose and render lists):

| Group | Count | Proposed |
|---|---|---|
| Real warnings that `--deny` blocks | 11 compose codes and 4 render codes | keep on `--deny` |
| Recoveries that warn by default | 3 identifiers, plus their 3 codes | `--deny` only |
| Recoveries that error by default | 6 identifiers, plus their 6 codes | `--allow` only |
| Always an error, so `--deny` is a no-op | 11 codes | remove from `--deny` |
| Never emitted by `md` | 2 codes | remove from `--deny` |
| Accepted but never consulted | `style` on compose | remove from compose, or wire it |
| Features (`--disable`) | 3 | unchanged |
| Access (`--allow host=HOST`) | 1 | unchanged |

Under the proposal, `md compose --deny` would offer 14 codes (or their renamed
forms, see [naming.md](./naming.md)) instead of 33 codes plus 10 identifiers,
and `--allow` would offer 6 recoveries plus `host=HOST` instead of 9 plus
`host=HOST`.

## Duplicates: on both `--allow` and `--deny`

Exactly the nine `Recovery` identifiers are on both lists, because
`PolicyKind::Recovery` accepts both verbs (`cli/src/args/policy.rs:102-110`).
Each also has a code, which `--deny` accepts as a second spelling of the same
identifier. That makes 18 offered names for 9 problems.

| Identifier | Code | Default today | Default before `11a909a82` | Proposed list |
|---|---|---|---|---|
| `missing-hyperlinks` | `dm.reference.missing_hyperlink` | warn (file input only) | error | `--deny` |
| `missing-image-refs` | `dm.reference.missing_image` | warn (file input only) | error | `--deny` |
| `missing-transclusions` | `dm.reference.missing_transclusion` | warn, directive removed | error | `--deny` |
| `ctx-override` | `dm.context.ctx_override` | error | error | `--allow` |
| `invalid-frontmatter-assignment` | `dm.transclusion.invalid_frontmatter_assignment` | error | error | `--allow` |
| `reassigned-frontmatter-property` | `dm.transclusion.reassigned_frontmatter_property` | error | error | `--allow` |
| `shell-timeout` | `dm.shell.timeout_recovered` | error | error | `--allow` |
| `untyped-map-variables` | `dm.collection.untyped_map_source` | error | error | `--allow` |
| `untyped-filter-variables` | `dm.collection.untyped_filter_source` | error | error | `--allow` |

What the "wrong" verb does today, which shows it adds nothing:

- **`--deny` on a default-error recovery** changes nothing on its own, because
  the problem already fails. It matters only next to `--allow all`, to carve
  that recovery out. Worse, three of them then fail under a *different* code
  than the one denied: `ctx_override` fails as `dm.compose.failure` or
  `dm.schema.type_mismatch`, and the two `set=` recoveries fail as
  `dm.transclusion.failure`. So `--deny X` can fail with a report that never
  mentions `X`.
- **`--allow` on a default-warn recovery** changes nothing on its own (the
  recovery is already on). It matters only next to `--deny all`, to exempt
  that recovery.

So the only work the second verb does is to carve an exception out of `all`.
The spec's open question 1 (`--deny all` exemptions) decides whether that
needs another spelling.

## Identifiers (`POLICY_REGISTRY`)

| Identifier | Kind | Verbs × commands | Default | Works? | Proposed |
|---|---|---|---|---|---|
| `missing-hyperlinks` | recovery | allow, deny × compose | warn | yes, file input only; a no-op on stdin | `--deny` only |
| `missing-image-refs` | recovery | allow, deny × compose | warn | yes, file input only; a no-op on stdin | `--deny` only |
| `missing-transclusions` | recovery | allow, deny × compose | warn | yes | `--deny` only |
| `ctx-override` | recovery | allow, deny × compose | error | `--allow` yes; `--deny` only against `--allow all` | `--allow` only |
| `invalid-frontmatter-assignment` | recovery | allow, deny × compose | error | as above | `--allow` only |
| `reassigned-frontmatter-property` | recovery | allow, deny × compose | error | as above | `--allow` only |
| `shell-timeout` | recovery | allow, deny × compose | error | as above | `--allow` only |
| `untyped-map-variables` | recovery | allow, deny × compose | error | as above | `--allow` only |
| `untyped-filter-variables` | recovery | allow, deny × compose | error | as above | `--allow` only |
| `host=HOST` | access | allow × compose | refused unless `network: allow` | yes | unchanged |
| `baseline-schema` | feature | disable × compose, clean, schema validate | on | yes on all three (documented no-op for a YAML input to `schema validate`) | unchanged |
| `schema-triggers` | feature | disable × compose, clean, schema validate | on | yes on all three (same YAML caveat) | unchanged |
| `collection-blocks` | feature | disable × compose | on | yes | unchanged |
| `style` | diagnostics family | deny × compose, render | warn (render) | render: yes. **Compose: never consulted** (no `denies("style")` call; it has no codes) | remove from compose, or wire it to compose's style warnings |

## Compose diagnostic codes (`COMPOSE_DIAGNOSTIC_CODES`)

| Code | Default | Works with `--deny`? | Proposed |
|---|---|---|---|
| `dm.expression.uncertain_call` | warn | yes | keep |
| `dm.expression.always_false_call` | warn | yes | keep |
| `dm.expression.undeclared_property` | warn | yes | keep |
| `dm.expression.undeclared_member` | warn | yes | keep |
| `dm.expression.unknown_context_variable` | warn | yes | keep |
| `dm.glob.skipped_symlink` | warn | yes; raised only inside a repository boundary; judged only after the pipeline ends | keep |
| `dm.remote_cache.io_failure` | warn | yes, but judged only after the pipeline ends, so earlier effects have run (inferred, not probed) | keep |
| `dm.schema.invalid_trigger` | warn | yes | keep |
| `dm.config.not_saved` | warn | yes (inferred; needs an interactive approval whose save fails) | keep |
| `dm.link.legacy_structured_title` | warn | yes | keep |
| `dm.link.malformed_structured_title` | warn | yes | keep |
| `dm.reference.missing_hyperlink` | warn | yes (turns the recovery off) | keep, as the only name for the problem or beside the identifier (see [naming.md](./naming.md)) |
| `dm.reference.missing_image` | warn | yes (turns the recovery off) | same |
| `dm.reference.missing_transclusion` | warn | yes (turns the recovery off) | same |
| `dm.context.ctx_override` | error | only against `--allow all` | `--allow` side |
| `dm.transclusion.invalid_frontmatter_assignment` | error | only against `--allow all` | `--allow` side |
| `dm.transclusion.reassigned_frontmatter_property` | error | only against `--allow all` | `--allow` side |
| `dm.shell.timeout_recovered` | error | only against `--allow all` | `--allow` side |
| `dm.collection.untyped_map_source` | error | only against `--allow all` | `--allow` side |
| `dm.collection.untyped_filter_source` | error | only against `--allow all` | `--allow` side |
| `dm.expression.incompatible_call` | error | **no-op** (its own help text says "always an error") | remove |
| `dm.expression.parse_failure` | error | **no-op**; its warning form exists only in passes that discard it | remove |
| `dm.expression.evaluation_failure` | error | **no-op**; same | remove |
| `dm.expression.invalid_callback` | error | **no-op** (a warning form is possible in theory, never observed) | remove |
| `dm.collection.malformed` | error | **no-op** | remove |
| `dm.collection.invalid_source` | error | **no-op** | remove |
| `dm.collection.pattern_mismatch` | error | **no-op** | remove |
| `dm.collection.invalid_sort_key` | error | **no-op** | remove |
| `dm.collection.callback_failed` | error | **no-op** | remove |
| `dm.iteration.visit_limit` | error | **no-op** | remove |
| `dm.iteration.expansion_limit` | error | **no-op** | remove |
| `dm.expression.incomplete_analysis` | — | **never emitted by `md`**; only DMLS installs the host functions that raise it | remove from `md` (DMLS keeps it) |
| `dm.schema.missing_simplified_envelope` | — | **never emitted**; no loader constructs it, and tests assert its absence | remove; delete the dead code path too |

## Render diagnostic codes (`RENDER_DIAGNOSTIC_CODES`)

| Code | Default | Works with `--deny`? | Proposed |
|---|---|---|---|
| `dm.style.unknown_key` | warn | yes | keep. `md compose --deny dm.style.unknown_key` is a usage error even though compose accepts `--deny style` |
| `dm.style.deprecated` | warn | yes | keep |
| `dm.link.legacy_structured_title` | warn | yes (also on compose) | keep |
| `dm.link.malformed_structured_title` | warn | yes (also on compose; no render deny test) | keep |

## How the list should be kept honest

Today an entry can sit in a list with nothing behind it, and no test notices.
The fix should add a guard, in the style of the existing
`every_leaf_has_a_schema_file_and_a_conformance_test`:

- every `--deny` code must have a test that reports it as a **warning** and
  shows the denial blocking it;
- every `--allow` recovery must have a test that shows the default failing and
  the allowance recovering it;
- a code that is only ever an error cannot be registered as deny-able.

Missing deny tests today: `dm.glob.skipped_symlink`, `dm.remote_cache.io_failure`,
`dm.config.not_saved`, `dm.reference.missing_image`,
`dm.reference.missing_transclusion`, and `dm.link.malformed_structured_title`
on render.
