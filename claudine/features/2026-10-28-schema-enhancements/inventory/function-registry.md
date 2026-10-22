# Function Registry Inventory

Phase 1, Wave 1 "Contract inventory" for `2026-10-28-schema-enhancements`.
Taken 2026-10-21 from the `feat/pipeline` worktree at `e116c6acc`. Counts come
from enumerating the registry source, not from estimates.

## How the registry was enumerated

The runtime registry is the concatenation of `BINDING_GROUPS`
(`darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:194-213`),
iterated by `bindings()` (`functions/mod.rs:215`). Each group is a
`const BINDINGS: &[FunctionBinding]` in one handler module, plus
`LAZY_BINDINGS` (`functions/mod.rs:179-192`) for `and`/`or`. A
`FunctionBinding` (`functions/mod.rs:72-78`) records `canonical`, `aliases`,
`evaluation` (`Pure`, `Context`, or `Lazy`), and `handler`.

`validate_registry()` (`functions/mod.rs:246-285`) joins every binding with
the catalog descriptors whose signature names it, and panics on first use
(`joined_bindings()`, `functions/mod.rs:304-309`) if a catalog entry has no
binding or a binding has no catalog entry. The unit test
`catalog_and_runtime_bindings_have_bidirectional_canonical_parity`
(`functions/mod.rs:118`) asserts the same parity, so parity is enforced today.
The public runtime list is `dispatchable_canonical_names()`
(`functions/mod.rs:2948`).

Commands used (run from the repository root):

```sh
# 113 canonical registrations (one `canonical:` field per FunctionBinding;
# `agentic_cli.rs` uses the constant FUNCTION = "has_agentic_cli")
rg -o 'canonical: ("[a-z0-9_]+"|FUNCTION)' darkmatter/lib/src/markdown/compose/expression/functions/ | wc -l
# 75 registered aliases
rg -U -o 'aliases: &\[[^\]]*\]' darkmatter/lib/src/markdown/compose/expression/functions/ | rg -o '"[^"]+"' | wc -l
# 113 catalog functions, 121 examples (one per overload)
rg -c '^    - name:' darkmatter/docs/schemas/expression-functions.yaml
rg -c 'verification: (executable|display-only)' darkmatter/docs/schemas/expression-functions.yaml
```

The per-row table below was produced by a throwaway parser that walks
`BINDING_GROUPS` in order, reads each group's `FunctionBinding` blocks, and
joins them with the YAML catalog parsed by PyYAML. It was not committed.

Lookup is case-insensitive: `canonical_function_name` and `is_dispatchable`
lowercase the name and then match the canonical name or an alias
(`functions/mod.rs:2918-2942`).

## Totals

| Item | Count |
|---|---:|
| Canonical registered functions | **113** |
| Pure (eager, no context) | 62 |
| Context (eager, needs a `ResolutionContext`) | 49 |
| Lazy (`and`, `or`; arguments evaluated by the evaluator) | 2 |
| Registered aliases | **75** (on 73 functions; `is_date_time` and `is_date_time_utc` have two each) |
| Catalog functions in `expression-functions.yaml` | 113 (the same set as the registry) |
| Catalog overload descriptors | 121 (7 functions are overloaded) |
| Catalog examples | **121**: **74** `executable`, **47** `display-only` |
| Display-only examples whose reason is "verified via example file" (candidates for the new file form) | 4: `as_line_separated`, `as_tsv`, `as_unordered_list`, `as_ordered_list` (all except `as_tsv` have multi-line results) |
| Functions with no example | 0 |

Handler families by registration module: `paths` 22, `dates` 18, `strings` 16,
`collections` 14, `predicates` 13 (2 in `ABSENCE_PREDICATES`, 11 in
`BINDINGS`), `markdown_docs` 5, `git` 4, `network` 4, `shell` 4, `repository`
2, `pull_requests` 2, `cicd` 2, `skills` 2, lazy (`mod.rs`) 2, `terminal` 1,
`agentic_cli` 1, `composition` 1. Handler bodies live in `functions/mod.rs`
unless the registration names a module-local function (`git`, `repository`,
`network`, `pull_requests`, `cicd`, `shell`, `agentic_cli`, `composition`).

### Examples: 121 against the plan's 117

The spec and plan count of 117 examples (72 executable, 45 display-only) is
the catalog at `78540226f` (2026-09-17), which had 110 functions. Three
functions were added later, with four examples:

| Function | Added in | Examples |
|---|---|---|
| `find_files` | `6fafeae6a` (2026-09-28) | 1 executable |
| `try_frontmatter` | `6fafeae6a` (2026-09-28) | 1 executable |
| `code_link` | `5c4c49785` (2026-10-03) | 2 display-only |

No pre-existing example changed between `78540226f` and `HEAD` (a structural
diff of every `(function, overload) → example` pair found only these
additions). So 117 + 2 + 2 = 121, and 72 + 2 = 74 executable. Criterion 8's
"at least 72" is met by the current 74.

### Baseline outcomes: 3,575 against 113 functions

`spikes/coercion-baseline/baseline.json` records `function_count: 110` and
3,575 records (25 samples per parameter position). Its function set is the
registry minus the same three later additions: `code_link`, `find_files`, and
`try_frontmatter` have no recorded outcomes. Their ledger rows come from
reading their handlers (`functions/mod.rs:2378`, `:2580`, `:2537`). The
baseline also predates the `file_exists` changes of 2026-10-17 and 2026-10-20;
the current handler (`functions/mod.rs:1640`) still returns `false` for every
non-string, the same outcome the baseline recorded.

## Registry vs catalog mismatches

- **Registered functions missing from the catalog:** none. Parity is enforced
  by `validate_registry()` and the parity unit test.
- **Catalog entries with no registered implementation:** none in
  `expression-functions.yaml`.
  - **`keys` and `values`** are in neither the registry nor the current
    catalog. They are planned by the spec ("Map Functions") and must be
    registered and catalogued in this feature (criteria 7 and 45).
  - **`trim`** is named by the spec's null table and by criterion 43
    (`trim(null)` returns `null`), but **no `trim` function is registered or
    catalogued**. Decided 2026-10-22: add `trim` as a text transform, so criterion 43
    passes as written.
    `add` in the family table is illustrative only; arithmetic is an operator,
    not a function.
- **Draft catalog (`claudine/docs/schemas/partials/functions.yaml`) vs
  registry:**
  - The draft lacks `find_files`, `code_link`, and `try_frontmatter`. The spec's
    "Completeness" item lists only the 14 functions the draft lacked when it
    was written; these three must be added as well.
  - The draft contains `is_file_type`, which is not registered and, per the
    spec, belongs to `2026-10-31-type-aware-evaluation`.
- **Alias-only documentation entries:** none. The catalog has no alias field
  and no entry whose name is an alias. Aliases are documented only in prose
  (`darkmatter/docs/topics/darkmatter-expressions.md:297`, `:1286`).
  - **`has_binary` is not an alias.** It is a separate canonical registration
    sharing `has_command_fn` (`functions/paths.rs:9-11` says it is "a second
    documented name for the same probe (R16), not an alias"). It gets its own
    ledger row.

## Registered functions

Arity is the minimum and maximum argument count over the function's catalog
overloads, the same descriptors the arity check reads (`accepts_arity`,
`functions/mod.rs:311-322`); `N+` is variadic. Line numbers point at the
`FunctionBinding` in the named file. Every function has a descriptor in
`expression-functions.yaml`.

| # | Canonical | Aliases | Evaluation | Handler family (registration file:line) | Handler fn | In catalog | Overloads | Examples exec / display-only | Arity (min–max, from descriptors) |
|---:|---|---|---|---|---|:-:|---:|:-:|---|
| 1 | `is_null` | `isnull` | pure | predicates (`functions/predicates.rs:12`) | `is_null` | y | 1 | 1 / 0 | 1 |
| 2 | `is_empty` | `isempty` | pure | predicates (`functions/predicates.rs:13`) | `is_empty_fn` | y | 1 | 1 / 0 | 1 |
| 3 | `is_string` | `isstring` | pure | predicates (`functions/predicates.rs:17`) | `is_string` | y | 1 | 1 / 0 | 1 |
| 4 | `is_number` | `isnumber` | pure | predicates (`functions/predicates.rs:18`) | `is_number` | y | 1 | 1 / 0 | 1 |
| 5 | `is_array` | `isarray` | pure | predicates (`functions/predicates.rs:19`) | `is_array` | y | 1 | 1 / 0 | 1 |
| 6 | `is_object` | `isobject` | pure | predicates (`functions/predicates.rs:20`) | `is_object` | y | 1 | 1 / 0 | 1 |
| 7 | `is_positive` | `ispositive` | pure | predicates (`functions/predicates.rs:21`) | `is_positive` | y | 1 | 1 / 0 | 1 |
| 8 | `is_negative` | `isnegative` | pure | predicates (`functions/predicates.rs:22`) | `is_negative` | y | 1 | 1 / 0 | 1 |
| 9 | `is_integer` | `isinteger` | pure | predicates (`functions/predicates.rs:23`) | `is_integer` | y | 1 | 1 / 0 | 1 |
| 10 | `min` | — | pure | predicates (`functions/predicates.rs:24`) | `min_fn` | y | 1 | 1 / 0 | 2 |
| 11 | `max` | — | pure | predicates (`functions/predicates.rs:25`) | `max_fn` | y | 1 | 1 / 0 | 2 |
| 12 | `abs` | — | pure | predicates (`functions/predicates.rs:26`) | `abs_fn` | y | 1 | 1 / 0 | 1 |
| 13 | `round` | — | pure | predicates (`functions/predicates.rs:27`) | `round_fn` | y | 1 | 1 / 0 | 1–2 |
| 14 | `first` | — | pure | collections (`functions/collections.rs:4`) | `first_fn` | y | 1 | 1 / 0 | 1 |
| 15 | `last` | — | pure | collections (`functions/collections.rs:5`) | `last_fn` | y | 1 | 1 / 0 | 1 |
| 16 | `has_key` | `haskey` | pure | collections (`functions/collections.rs:6`) | `has_key_fn` | y | 1 | 1 / 0 | 2 |
| 17 | `contains` | — | pure | collections (`functions/collections.rs:7`) | `contains_fn` | y | 1 | 1 / 0 | 2 |
| 18 | `length` | — | pure | collections (`functions/collections.rs:8`) | `length_fn` | y | 1 | 1 / 0 | 1 |
| 19 | `number` | — | pure | collections (`functions/collections.rs:9`) | `number_fn` | y | 1 | 1 / 0 | 1–2 |
| 20 | `as_line_separated` | `aslineseparated` | pure | collections (`functions/collections.rs:10`) | `as_line_separated` | y | 1 | 0 / 1 | 1 |
| 21 | `as_csv` | `ascsv` | pure | collections (`functions/collections.rs:11`) | `as_csv` | y | 1 | 1 / 0 | 1 |
| 22 | `as_tsv` | `astsv` | pure | collections (`functions/collections.rs:12`) | `as_tsv` | y | 1 | 0 / 1 | 1 |
| 23 | `as_space_separated` | `asspaceseparated` | pure | collections (`functions/collections.rs:13`) | `as_space_separated` | y | 1 | 1 / 0 | 1 |
| 24 | `as_unordered_list` | `asunorderedlist` | pure | collections (`functions/collections.rs:14`) | `as_unordered_list` | y | 1 | 0 / 1 | 1 |
| 25 | `as_ordered_list` | `asorderedlist` | pure | collections (`functions/collections.rs:15`) | `as_ordered_list` | y | 1 | 0 / 1 | 1 |
| 26 | `as_json` | `asjson` | pure | collections (`functions/collections.rs:16`) | `as_json` | y | 1 | 1 / 0 | 1 |
| 27 | `as_json5` | `asjson5` | pure | collections (`functions/collections.rs:17`) | `as_json5` | y | 1 | 1 / 0 | 1 |
| 28 | `starts_with` | `startswith` | pure | strings (`functions/strings.rs:4`) | `starts_with` | y | 1 | 1 / 0 | 2 |
| 29 | `ends_with` | `endswith` | pure | strings (`functions/strings.rs:5`) | `ends_with` | y | 1 | 1 / 0 | 2 |
| 30 | `lower` | — | pure | strings (`functions/strings.rs:6`) | `lower` | y | 1 | 1 / 0 | 1 |
| 31 | `upper` | — | pure | strings (`functions/strings.rs:7`) | `upper` | y | 1 | 1 / 0 | 1 |
| 32 | `capitalize` | — | pure | strings (`functions/strings.rs:8`) | `capitalize` | y | 1 | 1 / 0 | 1 |
| 33 | `kebab_case` | `kebabcase` | pure | strings (`functions/strings.rs:9`) | `kebab_case` | y | 1 | 1 / 0 | 1 |
| 34 | `snake_case` | `snakecase` | pure | strings (`functions/strings.rs:10`) | `snake_case` | y | 1 | 1 / 0 | 1 |
| 35 | `camel_case` | `camelcase` | pure | strings (`functions/strings.rs:11`) | `camel_case` | y | 1 | 1 / 0 | 1 |
| 36 | `pascal_case` | `pascalcase` | pure | strings (`functions/strings.rs:12`) | `pascal_case` | y | 1 | 1 / 0 | 1 |
| 37 | `title_case` | `titlecase` | pure | strings (`functions/strings.rs:13`) | `title_case` | y | 1 | 1 / 0 | 1 |
| 38 | `without_date` | `withoutdate` | pure | strings (`functions/strings.rs:14`) | `without_date` | y | 1 | 1 / 0 | 1 |
| 39 | `ensure_leading` | `ensureleading` | pure | strings (`functions/strings.rs:15`) | `ensure_leading` | y | 1 | 1 / 0 | 2 |
| 40 | `ensure_trailing` | `ensuretrailing` | pure | strings (`functions/strings.rs:16`) | `ensure_trailing` | y | 1 | 1 / 0 | 2 |
| 41 | `replace` | — | pure | strings (`functions/strings.rs:17`) | `replace` | y | 1 | 1 / 0 | 3 |
| 42 | `replace_first` | `replacefirst` | pure | strings (`functions/strings.rs:18`) | `replace_first` | y | 1 | 1 / 0 | 3 |
| 43 | `replace_last` | `replacelast` | pure | strings (`functions/strings.rs:19`) | `replace_last` | y | 1 | 1 / 0 | 3 |
| 44 | `terminal` | — | pure | terminal (`functions/terminal.rs:4`) | `terminal` | y | 1 | 1 / 0 | 1 |
| 45 | `date` | — | pure | dates (`functions/dates.rs:4`) | `date_fn` | y | 1 | 1 / 0 | 2 |
| 46 | `is_date` | `isdate` | pure | dates (`functions/dates.rs:5`) | `is_date` | y | 1 | 1 / 0 | 1 |
| 47 | `is_date_utc` | `isdateutc` | pure | dates (`functions/dates.rs:6`) | `is_date_utc` | y | 1 | 1 / 0 | 1 |
| 48 | `is_date_time` | `isdatetime`, `is_datetime` | pure | dates (`functions/dates.rs:7`) | `is_datetime` | y | 1 | 1 / 0 | 1 |
| 49 | `is_date_time_utc` | `isdatetimeutc`, `is_datetime_utc` | pure | dates (`functions/dates.rs:8`) | `is_datetime_utc` | y | 1 | 1 / 0 | 1 |
| 50 | `is_today` | `istoday` | pure | dates (`functions/dates.rs:9`) | `is_today` | y | 1 | 0 / 1 | 1 |
| 51 | `is_today_utc` | `istodayutc` | pure | dates (`functions/dates.rs:10`) | `is_today_utc` | y | 1 | 0 / 1 | 1 |
| 52 | `is_yesterday` | `isyesterday` | pure | dates (`functions/dates.rs:11`) | `is_yesterday` | y | 1 | 0 / 1 | 1 |
| 53 | `is_yesterday_utc` | `isyesterdayutc` | pure | dates (`functions/dates.rs:12`) | `is_yesterday_utc` | y | 1 | 0 / 1 | 1 |
| 54 | `is_tomorrow` | `istomorrow` | pure | dates (`functions/dates.rs:13`) | `is_tomorrow` | y | 1 | 0 / 1 | 1 |
| 55 | `is_tomorrow_utc` | `istomorrowutc` | pure | dates (`functions/dates.rs:14`) | `is_tomorrow_utc` | y | 1 | 0 / 1 | 1 |
| 56 | `is_this_month` | `isthismonth` | pure | dates (`functions/dates.rs:15`) | `is_this_month` | y | 1 | 0 / 1 | 1 |
| 57 | `is_this_month_utc` | `isthismonthutc` | pure | dates (`functions/dates.rs:16`) | `is_this_month_utc` | y | 1 | 0 / 1 | 1 |
| 58 | `is_this_year` | `isthisyear` | pure | dates (`functions/dates.rs:17`) | `is_this_year` | y | 1 | 0 / 1 | 1 |
| 59 | `is_this_year_utc` | `isthisyearutc` | pure | dates (`functions/dates.rs:18`) | `is_this_year_utc` | y | 1 | 0 / 1 | 1 |
| 60 | `date_delta` | `datedelta` | pure | dates (`functions/dates.rs:19`) | `date_delta` | y | 1 | 1 / 0 | 3 |
| 61 | `older_than` | `olderthan` | pure | dates (`functions/dates.rs:20`) | `older_than` | y | 1 | 1 / 0 | 3 |
| 62 | `newer_than` | `newerthan` | pure | dates (`functions/dates.rs:21`) | `newer_than` | y | 1 | 1 / 0 | 3 |
| 63 | `predict_conflicts` | `predictconflicts` | context | git (`functions/git.rs:9`) | `predict_conflicts_fn` | y | 1 | 0 / 1 | 1 |
| 64 | `branch_exists_on_remote` | `branchexistsonremote` | context | git (`functions/git.rs:15`) | `branch_exists_on_remote_fn` | y | 3 | 0 / 3 | 0–2 |
| 65 | `remote_vendor` | `remotevendor` | context | git (`functions/git.rs:21`) | `remote_vendor_fn` | y | 1 | 0 / 1 | 0–1 |
| 66 | `recent_commits` | — | context | git (`functions/git.rs:27`) | `recent_commits_fn` | y | 1 | 0 / 1 | 1 |
| 67 | `package_area` | — | context | repository (`functions/repository.rs:11`) | `package_area_fn` | y | 1 | 0 / 1 | 1 |
| 68 | `package` | — | context | repository (`functions/repository.rs:17`) | `package_fn` | y | 1 | 0 / 1 | 1 |
| 69 | `ipv4` | — | context | network (`functions/network.rs:21`) | `ipv4_fn` | y | 1 | 0 / 1 | 0–1 |
| 70 | `ipv6` | — | context | network (`functions/network.rs:27`) | `ipv6_fn` | y | 1 | 0 / 1 | 0–1 |
| 71 | `ping` | — | context | network (`functions/network.rs:33`) | `ping_fn` | y | 1 | 0 / 1 | 1–2 |
| 72 | `ping_under` | — | context | network (`functions/network.rs:39`) | `ping_under_fn` | y | 1 | 0 / 1 | 2–3 |
| 73 | `pr` | — | context | pull_requests (`functions/pull_requests.rs:12`) | `pr_fn` | y | 1 | 0 / 1 | 1 |
| 74 | `pr_list` | `prlist` | context | pull_requests (`functions/pull_requests.rs:13`) | `pr_list_fn` | y | 2 | 0 / 2 | 1 |
| 75 | `cicd` | — | context | cicd (`functions/cicd.rs:10`) | `cicd_fn` | y | 1 | 0 / 1 | 1 |
| 76 | `cicd_list` | `cicdlist` | context | cicd (`functions/cicd.rs:11`) | `cicd_list_fn` | y | 2 | 0 / 2 | 1 |
| 77 | `absolute` | — | context | paths (`functions/paths.rs:4`) | `absolute_fn` | y | 1 | 0 / 1 | 1 |
| 78 | `relative` | — | context | paths (`functions/paths.rs:5`) | `relative_fn` | y | 1 | 1 / 0 | 1 |
| 79 | `file_exists` | `fileexists` | context | paths (`functions/paths.rs:6`) | `file_exists_fn` | y | 1 | 1 / 0 | 1 |
| 80 | `find_files` | `findfiles` | context | paths (`functions/paths.rs:7`) | `find_files_fn` | y | 1 | 1 / 0 | 1 |
| 81 | `has_command` | `hascommand` | context | paths (`functions/paths.rs:8`) | `has_command_fn` | y | 1 | 0 / 1 | 1 |
| 82 | `has_binary` | `hasbinary` | context | paths (`functions/paths.rs:11`) | `has_command_fn` | y | 1 | 0 / 1 | 1 |
| 83 | `is_indexed_file` | `isindexedfile` | context | paths (`functions/paths.rs:12`) | `is_indexed_file_fn` | y | 1 | 1 / 0 | 1 |
| 84 | `file_index` | `fileindex` | context | paths (`functions/paths.rs:13`) | `file_index_fn` | y | 1 | 1 / 0 | 1 |
| 85 | `increment_file_index` | `incrementfileindex` | context | paths (`functions/paths.rs:14`) | `increment_file_index_fn` | y | 1 | 1 / 0 | 1 |
| 86 | `decrement_file_index` | `decrementfileindex` | context | paths (`functions/paths.rs:15`) | `decrement_file_index_fn` | y | 1 | 1 / 0 | 1 |
| 87 | `find_first_index` | `findfirstindex` | context | paths (`functions/paths.rs:16`) | `find_first_index_fn` | y | 1 | 1 / 0 | 1 |
| 88 | `find_last_index` | `findlastindex` | context | paths (`functions/paths.rs:17`) | `find_last_index_fn` | y | 1 | 1 / 0 | 1 |
| 89 | `basename` | — | context | paths (`functions/paths.rs:18`) | `basename_fn` | y | 1 | 1 / 0 | 1 |
| 90 | `basename_without_index` | `basenamewithoutindex` | context | paths (`functions/paths.rs:19`) | `basename_without_index_fn` | y | 1 | 1 / 0 | 1 |
| 91 | `dirname` | — | context | paths (`functions/paths.rs:20`) | `dirname_fn` | y | 1 | 1 / 0 | 1 |
| 92 | `ext` | — | context | paths (`functions/paths.rs:21`) | `ext_fn` | y | 1 | 1 / 0 | 1 |
| 93 | `parent_dir` | `parentdir` | context | paths (`functions/paths.rs:22`) | `parent_dir_fn` | y | 1 | 1 / 0 | 1 |
| 94 | `file_trailing` | `filetrailing` | context | paths (`functions/paths.rs:23`) | `file_trailing_fn` | y | 1 | 1 / 0 | 1 |
| 95 | `dir_leading` | `dirleading` | context | paths (`functions/paths.rs:24`) | `dir_leading_fn` | y | 1 | 1 / 0 | 1 |
| 96 | `join` | — | context | paths (`functions/paths.rs:25`) | `join_fn` | y | 1 | 1 / 0 | 2 |
| 97 | `link` | — | context | paths (`functions/paths.rs:26`) | `link_fn` | y | 2 | 0 / 2 | 1–2 |
| 98 | `code_link` | `codelink` | context | paths (`functions/paths.rs:27`) | `code_link_fn` | y | 2 | 0 / 2 | 1–2 |
| 99 | `has_alias` | — | context | shell (`functions/shell.rs:13`) | `has_alias_fn` | y | 1 | 0 / 1 | 1 |
| 100 | `has_builtin_function` | — | context | shell (`functions/shell.rs:19`) | `has_builtin_function_fn` | y | 1 | 0 / 1 | 1 |
| 101 | `has_user_function` | — | context | shell (`functions/shell.rs:25`) | `has_user_function_fn` | y | 1 | 0 / 1 | 1 |
| 102 | `can_execute` | — | context | shell (`functions/shell.rs:31`) | `can_execute_fn` | y | 1 | 0 / 1 | 1 |
| 103 | `has_agentic_cli` | — | context | agentic_cli (`functions/agentic_cli.rs:14`) | `has_agentic_cli_fn` | y | 1 | 0 / 1 | 1 |
| 104 | `has_skill` | `hasskill` | context | skills (`functions/skills.rs:4`) | `has_skill_fn` | y | 1 | 0 / 1 | 1 |
| 105 | `has_local_skill` | `haslocalskill` | context | skills (`functions/skills.rs:5`) | `has_local_skill_fn` | y | 1 | 0 / 1 | 1 |
| 106 | `frontmatter` | — | context | markdown_docs (`functions/markdown_docs.rs:4`) | `frontmatter_fn` | y | 2 | 2 / 0 | 1–2 |
| 107 | `try_frontmatter` | `tryfrontmatter` | context | markdown_docs (`functions/markdown_docs.rs:5`) | `try_frontmatter_fn` | y | 1 | 1 / 0 | 1 |
| 108 | `markdown_body_empty` | `markdownbodyempty` | context | markdown_docs (`functions/markdown_docs.rs:6`) | `markdown_body_empty_fn` | y | 1 | 1 / 0 | 1 |
| 109 | `markdown_title` | `markdowntitle` | context | markdown_docs (`functions/markdown_docs.rs:7`) | `markdown_title_fn` | y | 1 | 1 / 0 | 1 |
| 110 | `validate_schema` | `validateschema` | context | markdown_docs (`functions/markdown_docs.rs:8`) | `validate_schema_fn` | y | 2 | 2 / 0 | 1–2 |
| 111 | `as_markdown` | — | context | composition (`functions/composition.rs:10`) | `as_markdown_fn` | y | 1 | 0 / 1 | 1 |
| 112 | `and` | — | lazy | mod (`functions/mod.rs:180`) | `— (lazy, evaluator-owned)` | y | 1 | 1 / 0 | 0+ |
| 113 | `or` | — | lazy | mod (`functions/mod.rs:186`) | `— (lazy, evaluator-owned)` | y | 1 | 1 / 0 | 0+ |

## Catalog consumers

Production code reaches the catalog in only one way: Darkmatter embeds the YAML
with `include_str!` and parses it once. Everything else goes through
Darkmatter's library API.

| Consumer | Location | Access |
|---|---|---|
| Darkmatter catalog loader (the only production YAML reader) | `darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs:484-492` (`include_str!` at `:486-488`), parsed by `catalog/parser.rs:158` | **Reads the YAML directly** (embedded at build time) |
| Public descriptor list | `catalog/mod.rs:502` `expression_function_descriptors()` | Library API (the API every other consumer uses) |
| Registry join, dispatch, arity checks | `functions/mod.rs:246-309`, `:2918-3060` | Library API (internal) |
| Unknown-function suggestions and arity hints | `darkmatter/lib/src/markdown/compose/expression/mod.rs:755`, `:771`, `:776` | Library API |
| Prepared-expression name check | `expression/prepared.rs:303` (`is_dispatchable`) | Library API (registry) |
| Scope facts (canonical name and arity) | `expression/scope/facts.rs:63` | Library API (registry) |
| Context-capture capability detection | `darkmatter/lib/src/markdown/compose/context/capture/capabilities.rs:129` | Library API |
| Lifecycle verb and signature lookup | `darkmatter/lib/src/lifecycle/signatures.rs:258`, `:279`, `:319` | Library API |
| `md schema about` signature reference | `darkmatter/cli/src/commands/schema/about.rs:498` | Library API |
| Generated function table for `darkmatter-expressions.md` | `darkmatter/lib/examples/expression_doc_generator.rs:22` (`generate_expression_function_table`, `catalog/mod.rs:516`) | Library API |
| DMLS completion and hover | `darkmatter/dmls/src/overlay/expressions.rs:623-633` (`function_descriptors`, `function_descriptor`), used at `:841` (completion), `:1115` and `:1161` (hover) | Library API |
| `claudine context --expressions` | `claudine/cli/src/commands/context/mod.rs:11`, `:76` (`group_expression_descriptors`), rendered by `claudine/cli/src/commands/context/expressions.rs:305` | Library API |

Test and tooling readers:

| Reader | Location | Access |
|---|---|---|
| Catalog parser tests | `catalog/parser.rs:440` | Reads the YAML directly (`include_str!`) |
| `recent_commits` pairing test | `catalog/mod.rs:1328` | Reads the YAML directly (`include_str!`) |
| `predict_conflicts` catalog test | `darkmatter/lib/tests/l1/predict_conflicts.rs:178` (API) and `:200-201` | API plus a direct YAML read |
| Shipped-schema corpus | `darkmatter/lib/tests/l1/shipped_schema_corpus.rs:102`, `meta_schema_phase1.rs:248`, `darkmatter/dmls/src/overlay/mod.rs:1874` | File name only (classifies the file as inactive) |
| DMLS provider tests | `darkmatter/dmls/src/providers/dsl.rs:1632-1841`, `providers/frontmatter.rs:3093` | Library API (`function_descriptor`) |
| Claudine tests | `claudine/cli/tests/l1/context_command.rs:470`, `:558`; `claudine/cli/src/commands/context/tests.rs:380`; `claudine/lib/tests/l1/claudine_schemas.rs:365` | Library API |
| Coercion-baseline spike | `claudine/features/2026-10-28-schema-enhancements/spikes/coercion-baseline/src/main.rs:16`, `:227`, `:340` | Library API (throwaway) |
| Completed lifecycle-grammar spike | `claudine/features/_completed/2026-10-20-lifecycle-grammar/spike/gen-action-schema.py:55` | Reads the YAML directly (historical, not run) |

Documentation and prompts that name the file (they must be updated when it is
deleted, criterion 7): `darkmatter/docs/topics/darkmatter-expressions.md:710`,
`:1741`; `claudine/docs/topics/state-management/expression-engine.md:191`,
`:240`; `claudine/docs/topics/state-management/drift.md:31`;
`.claude/skills/darkmatter/library-surfaces.md:37`;
`prompts/_add/add-expressions.md:23`, `:25`, `:121`.

### Observations for later waves

- **Alias resolution is not uniform across consumers.** Dispatch is
  case-insensitive and alias-aware, but DMLS `function_descriptor`
  (`dmls/src/overlay/expressions.rs:630-633`) and the lifecycle lookups
  (`lifecycle/signatures.rs:256-260`) match the canonical name exactly, so
  hovering an alias such as `isString(...)` finds no descriptor. This is
  relevant to criterion 57.
- **Lazy evaluation is decided in two places.** The registry marks `and`/`or`
  `Lazy`, but the evaluator also matches the literal names `"and"` and `"or"`
  (`expression/mod.rs:806-830`). Necessary Rule 4 puts evaluation mode in
  registration; the binding work should make the evaluator read it from there.
- **`and()` and `or()` accept zero arguments.** `lazy_arity_eligibility(name, 0)`
  is `Eligible` (`functions/mod.rs:6031`), so these two functions have no
  rejected input of any kind (see the ledger).
- **`claudine context --expressions` orders by the `order` field**
  (`claudine/cli/src/commands/context/mod.rs:67-89`). The spec drops `order`
  and sorts alphabetically within each category (criterion 39), so this
  renderer changes with the catalog.
