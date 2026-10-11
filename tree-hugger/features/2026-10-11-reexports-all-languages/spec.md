---
kind: feature
name: reexports-all-languages
area: tree-hugger
status: draft-spec
scheduled: true
origin: ruling on `2026-10-10-docs-audit-code-defects` (F3), which removed the placeholder `TreeFile::reexported_symbols()` and moved the real capability here
packages:
  - tree-hugger
  - tree-hugger-cli
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: "2026-10-11"
review_iterations: 0
human_review: true
human_review_items:
  - |-
    **Decide the support contract before any code is written (Phase 1 exit gate).**

    The audit of all 16 languages is finished (see the "Audit results" section of this spec). The plan cannot continue to Phase 2 until you rule on what "re-export" supports, because every later phase builds on that contract. Phase 1 was run non-interactively, so nothing below was approved on your behalf.

    Decisions needed:
    - **Supported conventions.** Options: (a) dedicated syntax only, simple but leaves Python, Perl, Lua and CommonJS unsupported; (b) dedicated syntax plus bounded static conventions, more useful, needs per-language rules; (c) broad inference, rejected by repo rules. Also rule on the borderline cases: C# `TypeForwardedTo`, Java `requires transitive`, PHP `class_alias`, shell sourcing "barrel" files, and Python's `import X as X` marker.
    - **Canonical representation.** Recommended: a dedicated typed report in the file index, a `TreeFile` method, a query capability, and a stored imported-binding marker. You must pin the exact type names, JSON shape, method name and schema version.
    - **Consumer exposure.** Recommended: keep declaration lists unchanged, expose the report separately, keep `hug imports` flags, include the report in full-index JSON, add no new command.
    - **C++20 modules grammar blocker.** The pinned tree-sitter-cpp (newest published) cannot parse `export import` or `export module`, and some module files parse as ordinary declarations with no error. Options: (1) ship C++ with `using` forms only and report modules as "limited" (honest, no scope growth); (2) authorize a grammar upgrade or fork (full coverage, new scope and risk); (3) drop C++ from the goal. I recommend option 1.

    I recommend approving option (b) above with option 1 for C++, then marking this spec finalized so Phase 2 can be refined from it.
message_to_agent: |-
  Phase 4 was also attempted and stopped for the same reason (Phases 2 and 3 are unbuilt; no report model, cache schema or consumer seam exists to persist or consume). Nothing changed in source.
  Phase 3 was also attempted and stopped: Phase 2 is unbuilt and the rulings are still pending. Re-run Phase 2, then Phase 3, only after spec.md is finalized with concrete types and JSON.
  Phase 1 produced audit evidence only; no Rust code changed. Read audit/notes-*.md (per-construct tables) and the "Audit results" section in spec.md. Samples are under audit/samples/<lang>/ and are meant to become regression fixtures (copy into the L1 fixture layout, they parse with the pinned grammars). Probe tool: /tmp/th-probe (disposable, may be gone; rebuild from the lockfile versions). The probe hides anonymous tokens (`default`, `*`, `type`, `static`, `global`, `using namespace`), so inspect tokens, not just named nodes. The placeholder `reexported_symbols()` is already removed. The author's rulings are still pending: do not start Phase 2 until spec.md is finalized/planned with concrete types and JSON. Phase 2 was attempted and stopped for exactly this reason; nothing was implemented, so re-run Phase 2 after the rulings.
---

# Spec: Re-exports across all supported languages

This draft defines the goal, the language audit, and the constraints on an
implementation. The choices in **Open questions** still need the author's
decision before implementation planning. Recommendations in that section are
not approved decisions. This review does not implement the feature.

## Problem and starting point

A *re-export* exposes a name or module obtained elsewhere through another
module's interface. For example, `export { parse as read } from "./parser"`
lets callers import `read` without knowing where `parse` was defined.
Some languages also expose members through a containing type or object;
the audit must distinguish that from a file's module interface.

The `tree-hugger` library currently detects only Rust use declarations with a
visibility modifier. Its [ImportSymbol](../../lib/src/shared/symbol.rs)
import description carries `is_reexport`; both `pub use` and restricted forms
such as `pub(crate) use` set it. Other languages report `false`.
This flag describes written syntax, not proof that a name is accessible
outside the package.

The library's [FileSymbolIndex](../../lib/src/shared/schema_v2/mod.rs),
which holds analyzed declarations, imports, and exports, stores imports as
`ImportRecord`. That record has no re-export marker. The
[CLI conversion](../../cli/src/main.rs) in `tree-hugger-cli` reconstructs
`ImportSymbol` with `is_reexport: false`, so expanding direct extraction
alone would leave CLI import results inconsistent. The declaration field
`visibility.is_reexported` remains false during normal extraction; the CLI's
[prelude reporting](../../cli/src/prelude.rs) sets it on synthetic records
that represent resolved Rust prelude exports.

**Review note — dependency correction.** `2026-10-10-docs-audit-code-defects`
decides to remove the placeholder
[TreeFile::reexported_symbols()](../../lib/src/file/tree_file.rs) from the
`tree-hugger` library because it always returns an empty list. As checked on
2026-10-11, that removal is still pending in this checkout, and the library
docs still mention the method. This feature must coordinate with that removal;
it must not assume it has already happened or preserve the empty behavior.
The earlier decision also rejects delivering a language-neutral API that
only works for Rust.

## Goal

Provide one consistent answer to “what does this file re-export?” across all
16 supported languages: Rust, JavaScript, TypeScript, Go, Python, Java, PHP,
Perl, Bash, Zsh, C, C++, C#, Swift, Scala, and Lua.

Each language must have a verified account of what can be detected from one
file, what cannot, and why. Complete language coverage means every language
has an implemented support contract; it does not mean arbitrary runtime
behavior can be determined from syntax. An empty list with incomplete
coverage must be distinguishable from a verified absence of re-exports.
Do not ship a subset while silently returning “nothing” for the remainder.

The feature reports source evidence. It does not prove that the program
compiles or that all dependencies are installed.

## Phase 1: per-language audit

Audit all 16 languages before choosing the final API. For every candidate
construct, consult authoritative language documentation and parse a real
sample with the grammar version pinned by
[tree-hugger's manifest](../../lib/Cargo.toml) and lockfile. Documentation
establishes meaning; a parse establishes what this extractor can recognize.
A node whose name contains “export” is insufficient evidence by itself.

Classify each construct, then summarize the language:

- **Dedicated syntax:** directly expresses forwarding of names or modules.
- **Convention:** import, assignment, or export-list patterns expose an
  imported binding without a dedicated re-export statement.
- **No applicable construct:** no re-export under the agreed definition.
  Explain the nearest related constructs and why they do not qualify.

Record grammar limitations separately. A language feature unsupported by the
pinned grammar is a support gap, not evidence that the language lacks it.
Languages may have both dedicated syntax and conventions. “Approximation”
is not a fourth category: say which fact is established and which is inferred.

The audit table must give, for each language and construct:

- Classification, language version or dialect, and an authoritative source.
- Minimal sample file, grammar crate/version, exact node types and fields,
  and whether the sample parses without error or missing nodes.
- Expected exported name, original name or member path, source module or
  object path, enclosing scope, alias, wildcard/module form, and any
  visibility restriction or type-only qualifier.
- Positive and nearby negative examples; supported static patterns and
  unsupported dynamic or ambiguous variants.

Keep the audit in this specification until design is settled. During
implementation, put the supported behavior and examples in
`tree-hugger/docs/reexports.md`; that current documentation must not refer
back to this dated specification.

### Candidates to verify

These are audit inputs, not completed grammar verification or promises of
support. No language in this table is preclassified as having no concept.

| Language | Constructs and distinctions to investigate |
|----------|------------------------------------------|
| Rust | `pub use a::B`, aliases, grouped paths, `self`, glob imports, `pub(crate)` / `pub(in ...)`, nested modules, conditional attributes, and public external-crate bindings. Distinguish written visibility from effective public reachability. |
| JavaScript | Named, renamed, default, wildcard, and namespace ES-module re-exports; `import { a } from "m"; export { a as b }`; direct CommonJS forwarding such as `module.exports = require("m")`; shadowed `require`, `module`, and `exports`. |
| TypeScript | JavaScript forms, statement-level and per-specifier type-only exports, `export type *`, `export import`, and `export =` of an imported binding. Distinguish `export as namespace` from forwarding a dependency. |
| Python | Imported bindings in any module, aliases, literal `__all__`, absent or computed `__all__`, star imports, rebinding, and conditional imports. Do not restrict the audit to `__init__.py`. |
| Go | Exported aliases such as `type X = pkg.X`; contrast a new defined type `type X pkg.X`, package imports, and exported variables initialized from imported values. |
| Java | Ordinary/static imports versus module declarations such as `requires transitive`; decide whether dependency exposure belongs in this symbol-oriented feature. |
| PHP | `use` aliases, namespace imports, `include` / `require`, and runtime aliasing. Separate local naming and executing another file from an outward-facing alias. |
| Perl | `Exporter`, `@EXPORT`, `@EXPORT_OK`, `@EXPORT_TAGS`, imported names versus locally defined subs, and computed export lists. |
| Bash | `source` / `.`, imported shell definitions, and exported functions; distinguish loading definitions and environment propagation from module re-export. |
| Zsh | `source` / `.`, autoloading, and shell exports; check its own pinned grammar rather than assuming Bash results apply. |
| C | Includes, include guards, and conditional preprocessing. An include alone does not establish that a header is a public interface. |
| C++ | Namespace-level `using`, using-directives versus declarations, type aliases, includes, and C++20 `export import`; class-member exposure is not automatically a module export. |
| C# | `using`, aliases, `global using`, and assembly type forwarding; identify constructs whose meaning depends on compilation or assembly metadata. |
| Swift | `@_exported import`, selective imports, and any public-import forms supported by the audited Swift version; distinguish an underscored facility from a stable language guarantee. |
| Scala | Scala 3 `export` selectors, aliases, wildcards, exclusions, givens, and exporting an object's members without an import. Keep Scala 2 behavior separate. |
| Lua | Returning `require("m")`, returning a table containing imported values, and `M.foo = require("m").foo` followed by `return M`; a table assignment alone does not prove an exported module interface. |

**Review note — definition correction.** Requiring a preceding import would
exclude Scala exports of object members, while requiring `__init__.py`
would miss ordinary Python modules. Scala's
[export-clause reference](https://docs.scala-lang.org/scala3/reference/other-new-features/export.html)
describes forwarding members of a path. Python's
[import reference](https://docs.python.org/3/reference/simple_stmts.html#the-import-statement)
defines public names for star imports using `__all__`, or non-underscore
names when it is absent. Those rules describe exposure, not an author's
intent to make a stable API. The audit must preserve that distinction.

## Audit results (consolidated 2026-10-11)

Evidence lives beside this spec: per-construct tables with sources are in
`audit/notes-module-syntax.md`, `audit/notes-static-conventions.md` and
`audit/notes-interface-boundaries.md`; every sample parsed is under
`audit/samples/<lang>/` and is intended as a later regression fixture. All
parses used the lockfile-pinned grammars (tree-sitter 0.26.9; rust 0.24.2,
javascript 0.25.0, typescript 0.23.2, python 0.25.0, go 0.25.0, java 0.23.5,
php 0.24.2, perl 1.1.2, bash 0.25.1, zsh 0.56.0, c 0.24.2, cpp 0.23.4,
c-sharp 0.23.5, swift 0.7.3, scala 0.24.1, lua 0.5.0). Items a fetched page did
not confirm are marked "source not verified" in the notes and are not promoted
to verified support.

| Language | Classification | Grammar gaps (separate from semantic negatives) |
|----------|----------------|--------------------------------------------------|
| Rust | Dedicated syntax (`pub use`, restricted forms); `pub(self)` is effectively private; block-scoped and macro-generated uses need limits | none; all samples parse cleanly |
| JavaScript | Dedicated ES-module syntax; import-then-export and CommonJS `require` are conventions needing a same-file binding table | `export {a} from "m" with {type:"json"}` yields ERROR |
| TypeScript | JavaScript forms plus type-only exports; `export =` of an imported binding is a convention; `export as namespace` is not a forward | `export type *`, `export type * as ns from` yield ERROR; `export import D = require("m")` yields MISSING |
| Python | Convention only (imports, aliases, star imports, literal `__all__`); runtime exposure and declared interface differ and need a basis marker | none |
| Go | Dedicated syntax for aliases (`type X = pkg.X`, generic aliases); `var`/`const` copies are a convention with a mutability limit; defined types, wrappers, embedding and imports are negatives | build tags are plain comments |
| Java | No applicable construct for symbols; `requires transitive` is dependency exposure that JLS 7.7.1 itself distinguishes from re-export | none |
| PHP | No applicable construct; `use` is per file, `include`/`require` execute a file; `class_alias` is a dubious runtime convention | none (newer grammar 0.25.1 exists, untested) |
| Perl | Convention only (Exporter lists plus names bound by `use Mod qw(..)` in the same package) | `@Pkg::EXPORT_OK` parses silently to a wrong tree; `%EXPORT_TAGS` keys parse as barewords calls; `use`/`no` share a node |
| Bash | No applicable construct (sourcing, `export -f` are not re-export); a sourcing barrel file is dubious | none |
| Zsh | Same as Bash; `autoload` is lazy loading, not forwarding | none (newer grammar 0.63.4 exists, untested) |
| C | No applicable construct (`#include`, `#define`, `typedef` do not establish an interface) | `#if __has_include` is a parse error that unbalances `#endif` |
| C++ | Dedicated syntax: namespace-level `using ns::x;`, `using namespace` (wildcard, fallback-only lookup); type aliases are a convention; class-member `using` and `#include` are outside the definition; C++20 `export import`/`export using` are dedicated syntax the grammar cannot parse | **blocker:** no C++20 module rules in tree-sitter-cpp 0.23.4 (newest published); `import x;`/`module m;` parse as ordinary declarations with no error; `using typename`, multi-name `using` are errors |
| C# | `using`, aliases, `using static`, `global using` are per file or compilation, outside the definition; `[assembly: TypeForwardedTo]` is the one dubious dedicated construct | none |
| Swift | `@_exported import` is the one dedicated form; `public import` (SE-0409) is access control, not forwarding; `#if` is a sibling node with a lossy condition | none |
| Scala | Scala 3 `export` is dedicated syntax; Scala 2 has no applicable construct | `private`/`inline`/annotated `export` yield ERROR yet the export is recovered (a restricted export would read as public); `export a.b, c.d.{e}` is flattened; `given` vs plain selector differs only by `type_identifier` |
| Lua | Convention only (what the chunk returns); `M` needs top-level assignments, no rebinding, `return M` last; legacy `module()` is limit-only | none |

Cross-cutting findings that shape the design:

- The probe prints named nodes only; anonymous tokens carry load-bearing
  distinctions (`default`, `*`, TS `type`, Java `static`, PHP `use function`,
  C# `global`/`static`, C++ `using namespace`). Extraction must read tokens,
  not only named nodes.
- A clean parse is not proof of meaning: C++ module files parse without errors
  as ordinary declarations, and a Perl qualified variable parses wrongly with
  no error flag. Coverage cannot be derived from `has_error` alone.
- The placeholder `TreeFile::reexported_symbols()` is already removed in this
  checkout (the removal landed with the docs-audit work; only the release note
  remains). The replacement name is still the author's ruling.

### Author decision package (pending; nothing below is approved)

1. **Conventions.** Recommended: dedicated syntax plus bounded static
   conventions. Proposed supported set: Rust, JavaScript, TypeScript, Swift,
   Scala, Go (aliases and value copies), Python, Perl, Lua, and C++ named/
   wildcard `using`. Proposed "no applicable construct": Java (symbols), C,
   Bash, Zsh, PHP, C# `using` forms. To rule on: C# `TypeForwardedTo`
   (lean: admit with an unresolved-source limit), Java `requires transitive`
   (lean: out of scope or a separate kind), `class_alias` (lean: out), shell
   barrel files (lean: out), Python basis marker and `import X as X`.
2. **Representation.** Recommended: typed report in `FileSymbolIndex`, a
   `TreeFile` method and `SymbolQuery` capability, plus a stored imported-
   binding marker; schema version from the changed shape. Exact type names,
   JSON and method name remain to be pinned by the author.
3. **Consumers.** Recommended: separate exposure; `exported_symbols()` and
   `--exported` stay declaration-only; `hug imports` keeps flags; full-index
   JSON carries the requested report; no new command.
4. **Grammar blocker (C++20 modules).** Options: (a) ship C++ with the
   `using` forms only and report modules as limited (honest, no scope
   expansion; recommended); (b) authorize a grammar upgrade or fork; (c) drop
   C++ from the goal. Per the out-of-scope rules this needs the author.
5. **Scala/TS/JS grammar gaps.** Recommended: report damaged or unparsed
   export syntax as a limited region, never a definite complete negative.

## Design constraints

### Report forwarding without inventing definitions

A result must preserve the exported name separately from the original name
and from the local imported binding. For
`import { a as local } from "m"; export { local as b }`, those names are
`b`, `a`, and `local`, respectively. A same-file binding may establish that
relationship; it must not be inferred solely from matching text when a
binding is shadowed or reassigned.

Represent a wildcard as one forwarding entry, not as fabricated symbol
names. `export * from "m"` does not enumerate `m`'s exports; default-export
exclusions and similar language rules belong in its documented semantics.
A namespace or whole-module forward must remain distinguishable from a
wildcard: `export * as ns from "m"` exposes the name `ns`.

Preserve scope and written visibility. Rust's
[use-declaration reference](https://doc.rust-lang.org/reference/items/use-declarations.html)
permits visibility on a use declaration; restricted re-exports must remain
recognized as they are today. A re-export flag must not imply that a private
enclosing module or a restricted declaration is externally accessible.

Source spans refer to the forwarding syntax in the analyzed file, with the
repository's existing byte offsets and 1-based locations. Imported target
IDs, declaration kinds, signatures, and original-definition locations stay
unknown without proof. Sort entries by source position; collapse duplicate
captures of the same construct, but preserve distinct forwarding sites even
when they expose the same name. Report conditional syntax without evaluating
build conditions or presenting the resulting interface as unconditional.

The original declaration does not become “re-exported” merely because an
alias exposes it. Any decision to emit alias declaration records must define
their identity and flags separately. Existing synthetic Rust prelude records
retain their established behavior.

### Preserve analysis and cache contracts

The `tree-hugger` library's
[analysis planner](../../lib/src/analysis/mod.rs) shares extraction across
API calls and builds its file index once per requested analysis. Re-export
extraction must integrate there, not call public extraction methods again
inside the planner. Reuse existing file facts and indexed parent navigation.
Do not add cross-file reads or execute source code.

Specify which existing or new analysis capability computes re-exports and
coverage before implementation. Direct and single-statement forms should
need only syntax work; indirect forms may need same-file binding. A request
must expose “not computed” distinctly from “computed and empty.” A cheap
query must not silently run the full reference or diagnostic analysis just
to populate fields it did not request.

Direct API, complete index, selective query, cached query, and CLI
conversions must agree for equivalent requests. Coverage describes the whole
analyzed file and must survive declaration-kind filtering. If binding is
needed, use all necessary bindings before filtering declarations, as the
existing planner does. Narrowing and widening a cached result must produce
the same answer as a fresh query; unavailable evidence requires computation,
not an empty fallback. Keep stored data independent of the scan root.

The library's [SchemaVersion](../../lib/src/shared/schema_v2/types.rs)
currently emits 5.1, despite the historical `schema_v2` module name. Filling
an existing field does not by itself settle versioning: decide the version
change from the final serialized shape and changed meaning. Update records,
file indexes, CLI envelopes, fingerprints, validation, and schema tests
together. Always bump the library's
[EXTRACTOR_REVISION](../../lib/src/file/mod.rs) when extraction changes,
even if serialized fields stay the same, and add the corresponding
cache-upgrade row. Old or malformed entries must be recomputed rather than
served as complete negative evidence.

### Consumers and documentation

The `tree-hugger` library's `unused-import` diagnostic currently exempts
Rust re-exports. Extend that exemption only to imports actually shown to be
forwarded by the agreed support contract; an unrelated import or a guessed
public surface must not suppress a warning. Preserve existing handling of
local ES-module export occurrences: a local export reads its local binding,
whereas `export ... from "m"` must not bind names to unrelated local symbols.

The `tree-hugger-cli` package must preserve re-export evidence in cached
import conversion and full-index JSON. Whether it also adds a command or
changes declaration listings remains an open design choice below. The Rust
`--prelude` resolver already follows targets across files; this feature must
not expand that resolver to other languages. If its detection is switched
to shared facts, preserve its existing public-only selection and synthetic
alias identities, and cover the change with regression tests.

Update the library guide, symbol-record field and file-index references,
CLI guide for any output changes, diagnostics documentation, cache format
and robustness pages, and a new re-export topic page. Update the tree-hugger
skill's API, extraction, analysis, cache, testing, and diagnostics pages where
the contract changes. Keep READMEs as overviews with links into `docs/`.
Review source comments on every changed behavior; distinguish normal
extraction from CLI synthetic records when documenting flags.

## Open questions

### Which conventions should count?

This determines what “all languages” promises and whether callers can rely
on a negative answer. The audit must settle the exact supported patterns
before these options can be finalized.

1. **Dedicated syntax only.** Pros: a small extractor with strong syntax
   evidence. Cons: excludes useful Python, Perl, Lua, and CommonJS forwarding;
   it must report those as unsupported rather than claiming no concept.
2. **Dedicated syntax plus bounded static conventions — recommended.**
   Recognize explicitly documented patterns whose source and exposure can be
   established within one file; attach evidence kind and coverage reasons.
   Pros: useful language coverage while keeping uncertain cases visible.
   Cons: requires language-specific rules for export lists, mutation,
   shadowing, conditional syntax, and module interfaces. Recommendation:
   this meets the goal without guessing intent or executing programs.
3. **Broad public-interface inference.** Treat public-looking imports,
   header includes, and common filenames as evidence. Pros: finds more
   possible interfaces. Cons: false positives, filesystem-dependent meaning,
   and unstable lint suppression. It conflicts with the repository's rule
   that naming conventions are not reported as visibility facts unless that
   change is explicitly designed and documented.

For the recommended option, use categorical evidence such as “explicit
syntax” or “static convention,” not an invented numeric confidence score.
The final report must distinguish complete supported analysis, limited
analysis with reasons, no applicable language construct, and not computed.
For example, a computed Python `__all__` must not produce a complete empty
answer. Decide which conditions limit a region versus the whole file, and
how callers discover the supported patterns for their language.

### What is the canonical API and serialized representation?

Existing import and declaration records do not represent all wildcard,
module, member-forwarding, or coverage cases. This is a public API and schema
decision, so exact types and migration must be agreed before coding.

1. **Extend imports and filter them.** Pros: close to the current Rust flag;
   little new API. Cons: exports need not create a local import, Scala can
   forward object members, and a boolean cannot express form or coverage.
2. **A dedicated re-export report in the file index — recommended.** A
   `TreeFile` method and queries expose the same report: typed entries plus
   coverage and reasons. Entries distinguish named, wildcard, and
   namespace/module forwarding and retain source, names, scope, qualifiers,
   spans, and evidence. Pros: accurately represents forwarding without
   inventing declarations; makes cached and uncached output consistent.
   Cons: adds schema fields, validation, capability rules, and adapters.
   Recommendation: these costs directly support the goal and avoid forcing
   wildcard or unknown-target evidence into declaration records.
3. **Extend the existing export records to cover every form.** Pros: one
   export collection. Cons: currently these records refer to local exported
   declarations; a tagged redesign changes every consumer and must preserve
   local binding behavior and distinguish unresolved forwards from local
   exports. Evaluate this if the audit shows it is simpler than a separate
   report; merely adding a flag is insufficient.

Whichever is chosen, preserve `ImportSymbol::is_reexport` for actual imported
bindings and define the corresponding stored representation. Decide whether
the removed method name should return the new report or whether a clearer
new name is needed. Do not restore an import-only list under a contract that
promises every forwarding form. Pin Rust API and JSON examples for a renamed
export, wildcard, restricted export, type-only export, and limited result.

### How should declaration queries and the CLI expose re-exports?

The existing `exported_symbols()` method and declaration query `--exported`
filter select declarations, while forwarding may have no local definition.
Changing them requires deciding what a “symbol” means in these interfaces.

1. **Expose the report separately; keep declaration queries about
   declarations — recommended.** Pros: preserves kind filters, IDs, counts,
   and unresolved-target semantics; a full-index report can carry every form.
   Cons: callers must ask for forwarding separately. Recommendation: this
   gives a truthful single-file answer without pretending a wildcard is a
   declaration. Correct docs that currently imply ordinary declaration
   listings already include general re-exports.
2. **Synthesize named alias declarations alongside originals.** Pros:
   existing listing and exported filters can show known alias names. Cons:
   kind and signature may be unknown, counts and IDs change, and wildcard
   forwarding still needs a separate representation. Specify deterministic
   alias identity, unresolved kinds, flag meanings, and interaction with
   Rust prelude records before choosing it.
3. **Add a combined export command while retaining separate library
   collections.** Pros: users can inspect local exports and forwards in one
   view without changing declaration records. Cons: adds CLI design and
   rendering work beyond the minimum library feature.

Specify the exact command/flag/output behavior before implementation.
Regardless of the option, `hug imports` must preserve imported-binding flags,
and full-index JSON must expose requested forwarding coverage. A new styled
view requires the existing real-terminal test harness and terminal-rendering
components; do not make a new CLI command a silent scope expansion.

## Out of scope

- Following forwarding chains across files, enumerating wildcard targets,
  resolving dependencies, or proving an original definition's identity.
- Running source code, a compiler, package manager, preprocessor, macro
  expander, or build configuration to determine the runtime interface.
- New language support or grammar upgrades unless the audit identifies a
  specific blocker and the author explicitly adds that work to scope.
- Inferring public interfaces solely from filenames, directory placement,
  include statements, or a declaration's public visibility.

Same-file binding for indirect exports is within scope. Grammar gaps must be
reported honestly; if they prevent the agreed coverage goal, record the
blocker for the author rather than weakening that goal during implementation.

## Acceptance and validation

### Audit and design ready for planning

- Every supported language has an authoritative semantic explanation, pinned
  grammar evidence, sample, expected result, and negative control. No
  recollection-based candidate is promoted to verified support.
- No applicable construct, unsupported grammar, unsupported dynamic pattern,
  and lack of computed analysis are separately documented.
- The author resolves the three open questions. The spec then names concrete
  API types, JSON examples, capability prerequisites, schema-version impact,
  and CLI behavior before it is marked finalized or planned.

### Implementation complete, ready for review

- Every agreed construct has extraction and focused L1 fixture coverage;
  each language has an explicit coverage contract and negative tests. Include
  aliases, grouped forms, wildcard/module distinctions, type-only qualifiers,
  restricted visibility, enclosing scopes, indirect local bindings, and
  dynamic/ambiguous cases where applicable. A negative test does not prove
  that a language has no re-export concept; the audit supplies that evidence.
- Positive fixtures parse cleanly with pinned grammars. Separately test parse
  recovery: unrelated syntax errors must not erase valid forwarding evidence,
  and damaged forwarding syntax must not become a definite complete answer.
- Direct API and equivalent index/query/CLI output agree. Fresh, cache-hit,
  narrowed, and widened results agree; stale and damaged cache entries are
  rejected. Kind filters do not erase whole-file coverage.
- Diagnostics retain the Rust exemption and exempt only supported forwards
  elsewhere. ES-module reference binding and Rust prelude behavior remain
  covered. No cross-file reads occur during ordinary re-export extraction.
- Existing work counters prove shared extraction and that cheap requests do
  not trigger unrequested reference, semantic, or diagnostic work. No
  performance spike is required for this spec; structural work-count tests
  address the identified repeated-analysis risk.
- Run `just test` and `just lint` from `tree-hugger/`, using the existing
  nextest recipes and test-toolkit fixture conventions. Run relevant
  `just test-l2` checks when styled terminal behavior changes; tests must not
  bring terminal or browser windows into focus. Preserve compatibility with
  macOS, Linux, native Windows, and WSL2 through existing test workflows;
  this feature does not add CI environments or gates.
- Current docs and the tree-hugger skill describe the implemented contract,
  examples, coverage limits, and how to request it. The spec remains in its
  active directory for author review; agents do not move it to `_completed`.
