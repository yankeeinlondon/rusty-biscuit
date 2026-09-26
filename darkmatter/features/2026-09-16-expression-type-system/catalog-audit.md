# Function catalog consolidation audit

This is a specification and source review, not runtime validation. The reviewed
draft is `claudine/docs/schemas/partials/functions.yaml`. Implementation moves
it to `darkmatter/schemas/partials/functions.yaml` with its consumers, after
the new grammar exists. The current embedded runtime catalog remains unchanged.

## Inventory and evidence

The inherited draft contained 96 names and 103 signatures. Comparison with the
live `darkmatter/docs/schemas/expression-functions.yaml` found 14 missing names:
`has_binary`, `has_alias`, `has_builtin_function`, `has_user_function`,
`can_execute`, `has_agentic_cli`, `package_area`, `package`, `recent_commits`,
`ipv4`, `ipv6`, `ping`, `ping_under`, and `as_markdown`.

The reviewed draft includes those functions and the already authorized new
`is_file_type`: **111 names, 118 signatures**. All 110 live names are accounted
for. Existing registered aliases remain part of migration; `has_binary` is a
separately documented name sharing the `has_command` implementation.

Source inspection covered the following families under
`darkmatter/lib/src/markdown/compose/expression`. The table identifies the
implementation evidence, not a claim that the draft was executed.

| Family | Source | Review outcome |
|---|---|---|
| Original-type and sign predicates | `functions/predicates.rs`, `functions/mod.rs` | Keep `any` for original-type/emptiness/integrality inspection; sign predicates require `number`. Negative whole values remain integers. |
| Math and explicit conversion | `functions/mod.rs`, expression `mod.rs` | Shared numeric conversion replaces strict/boolean/zero-default inconsistencies. Retain both optional fallbacks; use `f64` under M16. |
| Collection inspection | `functions/collections.rs`, `functions/mod.rs` | `first`/`last` preserve element types and return null on empty arrays. `contains` keeps rendered-text comparison. `length` rejects boolean/null sources. |
| String operations | `functions/strings.rs`, `functions/mod.rs` | Shared scalar-to-string binding; preserve Unicode case operations, word splitting, literal replacement, empty-find no-op, and affix result behavior. |
| List rendering | `functions/collections.rs`, `functions/mod.rs` | Keep heterogeneous `any[]`; null elements render as empty text, nested values use compact JSON. CSV/TSV names are text joiners, without escaping. |
| Date formatting, inspection, and arithmetic | `functions/dates.rs`, `functions/mod.rs` | Date predicates retain `any` and false for invalid inputs. Formatting/arithmetic retain domain parsing failures; local/UTC distinctions stay explicit. |
| Terminal rendering | `functions/terminal.rs`, `functions/mod.rs` | Returns rendered Prose text; no unsupported claim about terminal settings or Markdown composition. |
| Lazy logic | expression `mod.rs`, `functions/mod.rs` | Preserve short-circuiting; registry arity excludes rest parameters from the required count, so `and()` is true and `or()` false. |
| File/path operations | `functions/paths.rs`, `functions/mod.rs` | Retain access-sensitive domain failures. One-argument `link` rejects HTTP(S); two-argument form supports it. `join` rejects HTTP(S). |
| File inspection and schema validation | `functions/markdown_docs.rs`, `functions/mod.rs` | Preserve nullable title/property results and warnings; M9 changes the ignored `obj` into a frontmatter override before validation. |
| Skills | `functions/skills.rs`, `functions/mod.rs` | Add `fallible`: invalid basename names can fail after string binding. Ordinary null propagation is removed under M2. |
| Command and shell inspection | `functions/paths.rs`, `functions/shell.rs`, `functions/mod.rs` | Keep `any`: nonstrings return false without probing, and strings name what is inspected. Do not stringify booleans or null into command probes. |
| Agentic CLI inspection | `functions/agentic_cli.rs`, `agentic_cli_generated.rs` | Preserve supported names/aliases as `agentic-cli`; unknown names fail passive binding. Missing captured context is a setup defect, not ordinary function fallibility. |
| Repository lookup and Git | `functions/repository.rs`, `functions/git.rs` | Preserve captured topology, live branch/provider behavior, empty lookup results, and call-time recent history. Returns from recent history are `string[]`. |
| Provider functions | `functions/pull_requests.rs`, `functions/cicd.rs` | Preserve identifier alternatives, closed query vocabulary, and remote policy. Correct both count declarations to 1–100 and CI/CD categories. |
| Network | `functions/network.rs`, `sniff/lib/src/network/icmp/mod.rs` | Preserve captured-address filtering, passive IP syntax, ICMP grants, nullable denial outcomes, unstable result, and probe failures. |
| Markdown composition | `functions/composition.rs` | Preserve root-request context, authorization, and recursion budget; this remains a fallible runtime operation. |

## Deliberate changes and corrected declarations

M1 replaces local argument conversions with the shared binder. M2 removes
ordinary null propagation; it does not eliminate legitimate null results such
as `first([])`, absent Markdown titles, or denied ICMP probes. Optional omission
is distinct from null, including network filters/timeouts and optional Git
arguments. Every supplied argument is validated even if another is null.

`any` remains appropriate for type/date/command inspection, arbitrary needles,
heterogeneous array elements, and arbitrary returned frontmatter properties.
`has_key` declares `object` and `string`: its old false result for nonobjects
and arbitrary-key rendering are replaced by shared argument validation and
permitted string conversion. These are documented migration changes, not an
assertion of old behavior parity.

`number` and `round` both declare `x: numberlike, fallback?: number`. Recovery
is compiled metadata, not an invented function-body failure. They need no
`fallible` marker solely because unrecovered coercion can fail. Missing subjects
and invalid supplied fallbacks remain errors. Ordinary math/sign/string
operations similarly do not acquire fallibility from argument errors alone.
The numeric annex supplies the changed zero-default, boolean, precision,
integer-boundary, and display acceptance cases.

`contains` searches rendered array elements or object values, not keys, and
retains substring search for strings. `length` has an explicit boolean-source
exclusion before ordinary string conversion. `file_exists` retains its narrow
useful-input signature plus advisory metadata for unsupported or malformed
inputs; it must not turn a number into a filename through ordinary conversion.
Its runtime remote policy is unchanged.

`validate_schema(file, obj)` merges the object into an in-memory copy of the
page frontmatter using existing explicit override semantics, then validates.
The implementation currently ignores `obj`; the draft describes the confirmed
replacement, not the current function. Neither input nor file is mutated.

Affix functions retain string/number unions because they preserve already
matching values and may return either type. Their numeric-text recognition must
use the canonical helper instead of a private permissive parser. Numeric-text
affixes follow that spelling policy; a numeric subject produces a number only
when the concatenated text is representable, otherwise text. Boolean sources
can bind to the string arm under M1; arrays/objects/null cannot.

Provider query objects remain domain-validated rather than pretending that
`object` describes their complete accepted key/value grammar. Count overloads
express the existing 1–100 bound. ICMP timeouts must remain positive and at most
60,000 ms, with fractional milliseconds allowed subject to duration resolution;
attempts are 1–100. Timeout budget failures remain domain errors after numeric
binding. `recent_commits` applies its existing floating-input maximum
`9007199254740991`, with platform count conversion checked before history work.
No source value is silently truncated to fit a machine count.

Descriptions explain the operation concisely; shared conversion explanations
belong in the grammar/numeric annexes. The new `behavior` mapping is an authored
representation of the settled special contracts, supplied equally by hosts.
It is not an implemented function-schema grammar or permission for custom host
analysis callbacks. Registration must retain effects, aliases, context pairs,
and existing executable/display-only example verification records.

## Verification and implementation limits

This review checks YAML structure, duplicate keys, names/signature counts,
category membership, declaration delimiters, optional/rest structure, behavior
references, source inventory coverage, Markdown references, and whitespace.
These checks do **not** replace an implemented function-schema parser. The
historical representation spike checked another metadata envelope with an
installed binary of unproven worktree provenance; its success does not validate
this grammar. No runtime code, compiled grammar, or dispatch behavior was changed.

Implementation acceptance still requires the shared parser/binder, passive
shipped-artifact corpus, normal dispatch tests for every signature and declared
exception, source-location diagnostics, host predicate integration, example
verification, and performance evidence. Keep passive DMLS analysis free of
function execution and external I/O. Preserve request-scoped file resolution,
remote deny-by-default behavior, exact-host consent, and all existing effect
boundaries. Do not migrate the live catalog before its new consumers exist.
