---
kind: feature
name: reexports-all-languages
area: tree-hugger
status: implemented
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
review_iterations: 3
completed: true
human_review: false
implemented: true
implemented_by: "claude/sonnet"
message_to_agent: |-
  Phase 6 is done: `just test` (2381 passed) and `just lint` are green on macOS; the feature is implementation complete, ready for review. Nothing further to implement.
---

# Spec: Re-exports across all supported languages

This spec defines the goal, the language audit, the author's rulings and the
finalized design. The author accepted every recommendation on 2026-10-11;
**Author rulings** records them and **Finalized design** pins the API, JSON,
capability, schema version and CLI behavior that implementation follows.

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
removed the placeholder `TreeFile::reexported_symbols()` from the
`tree-hugger` library because it always returned an empty list; the removal
has landed and is recorded in the release notes. The replacement is the new
`TreeFile::reexports()` method (see **Finalized design**). The earlier
decision also rejects delivering a language-neutral API that only works for
Rust.

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
  remains). Its replacement is `TreeFile::reexports()`.

### Author rulings (2026-10-11)

The author accepted every recommendation in the decision package:

1. **Conventions: dedicated syntax plus bounded static conventions.** The
   supported languages are Rust, JavaScript, TypeScript, Swift, Scala, Go
   (aliases and value copies), Python, Perl, Lua, C++ (named and wildcard
   namespace-scope `using`) and C# (`TypeForwardedTo` only). Java, C, Bash,
   Zsh and PHP have no applicable construct. Borderline cases:
   - C# `[assembly: TypeForwardedTo]` is **admitted**, with the
     `source_unresolved` caveat.
   - Java `requires transitive` is **out of scope**. It exposes a module
     dependency, not symbols, and JLS 7.7.1 itself separates it from
     re-export.
   - PHP `class_alias` is **out of scope**. It is a runtime call.
   - Shell files that source other files as a "barrel" are **out of scope**.
   - Python reports both bases: the declared interface (`declared`) and
     runtime exposure (`exposed`). The redundant alias `import X as X` /
     `from m import X as X` counts as **declared**.
2. **Representation: a dedicated typed report**, carried in
   `FileSymbolIndex`, a `TreeFile` method, a `SymbolQuery` capability and a
   stored marker on each import record. The exact spellings are pinned in
   **Finalized design**.
3. **Consumers: exposed separately.** `exported_symbols()` and `--exported`
   stay declaration-only. `hug imports` keeps its flags. `--json-full`
   carries the report. No new command, flag or synthesized alias declaration
   is added.
4. **C++20 modules: option 1.** C++ ships the namespace-scope `using` forms
   only. A file containing module syntax reports `limited` with the
   `grammar_gap` reason. There is no grammar upgrade or fork, and C++ stays in
   the goal.
5. **Grammar gaps in Scala, TypeScript and JavaScript.** Damaged or unparsed
   forwarding syntax limits coverage (`parse_error` or `grammar_gap`). It
   never yields a complete negative.

Rulings on the audit's smaller questions, taken from the same
recommendations:

- Rust `pub(self) use` is effectively private and is **not** a re-export. It
  therefore loses the `unused-import` exemption it has today. This is the one
  intended diagnostic change for Rust; every other visibility-restricted
  `pub use` keeps the exemption. A `pub use` inside a function body is not
  reported, because it is not part of any module interface.
- Swift `public import` without `@_exported` is an access-control fact, not
  forwarding, and is not reported. `@_exported import` carries the
  `unstable_feature` caveat.
- TypeScript `export as namespace X` is not reported.
  `export import A = B.C` is reported only when `B` is an import binding.
- CommonJS merge forms (`Object.assign(module.exports, require(..))`,
  `{...require(..)}`) limit coverage with `dynamic_forwarding`. They are not
  merge forwards.
- Python `__all__ += mod.__all__` limits coverage with
  `cross_file_export_list`.
- Go matches an unaliased import by its explicit alias or else its last path
  segment. A last-segment match carries the `inferred_package_name` caveat.
- Lua's legacy `module()` limits coverage with `dynamic_forwarding`.
- Scala `given` selectors produce a wildcard entry with the `givens_only`
  caveat. Exporting a value's members (`export impl.run`) is in scope, as the
  definition section already requires.
- C++ type aliases (`using T = ns::T`, `typedef`) and class-member `using` are
  outside the definition, as are `#include` and C `typedef`/`#define`.

## Finalized design

### Library API

```rust
impl TreeFile {
    /// The file's re-export report, computed through the analysis planner.
    pub fn reexports(&self) -> Result<ReexportReport, TreeHuggerError>;
}

pub enum Capability { /* existing variants */ Reexports }
```

`Capability::Reexports` serializes as `"reexports"`. It is appended to
`Capability::ALL` after `Metrics`, and its only prerequisite is
`Declarations`, because extraction reads the import and definition captures.
`Capability::Diagnostics` gains `Reexports` as a prerequisite, so the
`unused-import` exemption always has proven facts. Re-exports never require
`References`, `Occurrences`, `Semantics` or `Diagnostics`. Indirect forms
need only a same-file, module-scope binding scan over the definition and
import captures. `TreeFile::imported_symbols()` also requests `Reexports`, so
that `ImportSymbol::is_reexport` comes from the report rather than from
today's Rust-only check.

The types live in the new module `shared/schema_v2/reexports.rs` and are
re-exported from the crate root:

```rust
pub struct ReexportReport {
    pub coverage: ReexportCoverage,
    /// Source order (by `span.start_byte`), one entry per forwarding site.
    pub entries: Vec<ReexportEntry>,
}

pub struct ReexportCoverage {
    pub state: ReexportCoverageState,
    /// Ascending by (`span.start_byte`, `reason`). Empty exactly when `state`
    /// is not `Limited`.
    pub limits: Vec<ReexportLimit>,
}

#[serde(rename_all = "snake_case")]
pub enum ReexportCoverageState { Complete, Limited, NotApplicable }

pub struct ReexportLimit {
    pub reason: ReexportLimitReason,
    /// The region the limit covers; a whole-file limit spans the root node.
    pub span: TextSpan,
}

/// New reasons may be added; existing ones keep their meaning.
#[serde(rename_all = "snake_case")]
pub enum ReexportLimitReason {
    ParseError,           // ERROR/MISSING touching forwarding syntax
    GrammarGap,           // forwarding syntax the pinned grammar cannot represent
    ComputedExportList,   // export list not built from literals
    CrossFileExportList,  // export list extended from another module
    ReboundBinding,       // forwarded name or interface table reassigned
    ShadowedGlobal,       // require/module/exports/import shadowed
    DynamicForwarding,    // runtime forwarding: Object.assign, spread, module(), custom import
    UnexpandedMacro,      // Rust macro body that may contain `pub use`
}

pub struct ReexportEntry {
    pub form: ReexportForm,
    pub evidence: ReexportEvidence,
    /// The name callers import; `None` for `Wildcard` and `Module`.
    pub exported_name: Option<String>,
    /// The name in the source; `Some` exactly for `Named` and `ValueCopy`.
    pub original_name: Option<String>,
    /// The same-file binding the forward goes through, when one does.
    pub local_name: Option<String>,
    /// The module, package or path as written; `None` when the file
    /// does not state one (C# `TypeForwardedTo`).
    pub source: Option<String>,
    /// Names a wildcard excludes (Scala `x as _`); empty otherwise.
    pub excluded_names: Vec<String>,
    /// Qualified name of the enclosing module, namespace or object;
    /// `None` at file level.
    pub scope: Option<String>,
    /// Written visibility restriction (`crate`, `super`, `in crate::a`);
    /// `None` when unrestricted.
    pub restriction: Option<String>,
    pub type_only: bool,
    /// Under a build or runtime condition (Rust `cfg`, Swift `#if`, Python
    /// `if`/`try`, Go build constraint); never evaluated.
    pub conditional: bool,
    /// Ascending.
    pub caveats: Vec<ReexportCaveat>,
    /// `span` of the `ImportRecord` this entry forwards, when it forwards one.
    pub import_span: Option<TextSpan>,
    /// The forwarding specifier or clause.
    pub span: TextSpan,
    /// The whole forwarding statement.
    pub statement_span: TextSpan,
}

#[serde(rename_all = "snake_case")]
pub enum ReexportForm { Named, Wildcard, Namespace, Module, ValueCopy }

#[serde(rename_all = "snake_case")]
pub enum ReexportEvidence { Syntax, Declared, Exposed }

#[serde(rename_all = "snake_case")]
pub enum ReexportCaveat {
    MutableBinding,       // Go `var` copy
    SourceUnresolved,     // C# TypeForwardedTo: target assembly not in the file
    FallbackLookup,       // C++ using-directive: names found only when not shadowed
    UnstableFeature,      // Swift `@_exported`
    InferredPackageName,  // Go import matched by its last path segment
    GivensOnly,           // Scala `given` selector
}
```

Every key of these types is always serialized, including `null`, `[]` and
`false`. No field uses `skip_serializing_if` or `#[serde(default)]`. That
keeps the cache reader strict (see the robustness matrix in the plan).

**Forms.** `named` forwards one name. `wildcard` forwards every public name of
`source` without listing them. `namespace` exposes a whole module under
`exported_name` (`export * as ns`, Python `import X as X` of a module,
`pub extern crate x as y`). `module` makes `source` the file's whole interface
(`module.exports = require("m")`, `export =` of an imported module,
`return require("m")`). `value_copy` is a new binding initialized from an
imported value (Go `var`/`const`); it does not preserve identity.

**Evidence.** `syntax` is dedicated forwarding syntax. `declared` is a
convention that the language's tooling treats as an explicit interface
declaration: Python `__all__`, redundant aliases and star imports; Perl
Exporter lists; Lua's returned table; CommonJS; Go value copies. `exposed` is
runtime exposure with no declaration, which today means only a public
module-level Python import when the module has no `__all__`.

**Coverage.** `not_applicable` is decided by language alone (Java, C, Bash,
Zsh, PHP); `entries` and `limits` are then empty. Otherwise the state is
`limited` when any limit exists and `complete` when none does. An ERROR or
MISSING node limits coverage only when it contains or immediately follows a
forwarding trigger token for the language: `export`, `use`, `import`,
`using`, `__all__`, `@EXPORT*`, `module.exports`/`exports`, or a chunk-final
`return`. Unrelated parse errors leave valid entries and `complete` coverage
intact. The report describes the whole file. Kind filters, predicates and
`scan_root` never change it.

**Not computed.** `FileSymbolIndex::reexports` is `None` exactly when
`completed_capabilities` lacks `reexports`; the key is then omitted from
JSON, as `occurrences` is. When computed, an empty `entries` list is a
`complete` negative only if `coverage.state` says so.

**Duplicates and order.** Captures of the same site (same `span`) collapse to
one entry. Distinct sites that expose the same name stay separate.

### Stored import marker

`ImportRecord` gains a required `reexported: bool`, always serialized. It is
`true` exactly when an entry with evidence `syntax` or `declared` has an
`import_span` equal to that record's `span`. It is meaningful only when
`reexports` is in `completed_capabilities`; otherwise it is `false` for every
record, and the cache reader rejects any `true`. The CLI's index-to-
`ImportSymbol` conversion copies it into `is_reexport` instead of hard-coding
`false`. `exposed` entries never set it, so a plain unused Python import is
still reported.

### Schema and revision

- `SchemaVersion::V5_2` ("shape with `imports[].reexported` and the optional
  `reexports` report") becomes `CURRENT`. `V5_1` is documented as no longer
  emitted.
- `EXTRACTOR_REVISION` goes from `"51"` to `"52"`, with its cache-upgrade
  row.
- `ExportRecord`, `SymbolRecord` and `visibility.is_reexported` keep their
  current shape and meaning. Synthetic prelude records are unchanged.

### JSON examples

Spans are abridged as `{…}`. They use the existing `TextSpan` shape.

Renamed ES-module forward, `export { parse as read } from "./parser";`:

```json
{ "form": "named", "evidence": "syntax", "exported_name": "read",
  "original_name": "parse", "local_name": null, "source": "./parser",
  "excluded_names": [], "scope": null, "restriction": null, "type_only": false,
  "conditional": false, "caveats": [], "import_span": null,
  "span": {…}, "statement_span": {…} }
```

Import, then export, `import { a as local } from "m"; export { local as b };`:

```json
{ "form": "named", "evidence": "syntax", "exported_name": "b",
  "original_name": "a", "local_name": "local", "source": "m",
  "excluded_names": [], "scope": null, "restriction": null, "type_only": false,
  "conditional": false, "caveats": [], "import_span": {…},
  "span": {…}, "statement_span": {…} }
```

Wildcard and namespace, `export * from "m"; export * as ns from "m";`:

```json
[{ "form": "wildcard", "evidence": "syntax", "exported_name": null,
   "original_name": null, "local_name": null, "source": "m", "...": "…" },
 { "form": "namespace", "evidence": "syntax", "exported_name": "ns",
   "original_name": null, "local_name": null, "source": "m", "...": "…" }]
```

Restricted Rust forward in a nested module, `mod a { pub(crate) use std::io::Read as R; }`:

```json
{ "form": "named", "evidence": "syntax", "exported_name": "R",
  "original_name": "Read", "local_name": null, "source": "std::io",
  "excluded_names": [], "scope": "a", "restriction": "crate",
  "type_only": false, "conditional": false, "caveats": [],
  "import_span": {…}, "span": {…}, "statement_span": {…} }
```

Type-only TypeScript forward, `export type { Shape } from "./shapes";`: a
`named` entry with `"type_only": true`.

A limited Python file, where `__all__ = names()` is computed:

```json
{ "coverage": { "state": "limited",
                "limits": [{ "reason": "computed_export_list", "span": {…} }] },
  "entries": [] }
```

Not applicable (any Java file): `{ "coverage": { "state": "not_applicable",
"limits": [] }, "entries": [] }`.

Not computed: the `reexports` key is absent from the file index, and
`"reexports"` is absent from `completed_capabilities`.

### Per-language support contract

The audit row IDs (P1–P17, G1–G13, L1–L11, U1–U7) refer to
`audit/notes-static-conventions.md`. Constructs not listed here are not
reported. The `docs/reexports.md` page carries this table in current-behavior
form.

| Language | Reported (form / evidence) | Limits and caveats | Not reported |
|---|---|---|---|
| Rust | `pub use` and every restricted form except `pub(self)`: `named`, glob → `wildcard`, `pub extern crate x [as y]` → `namespace`; `syntax`; `restriction` holds the written restriction; `cfg` attribute → `conditional` | `unexpanded_macro` when a `macro_rules!` body or item-position macro token tree contains `pub` … `use` | `pub(self) use`, `use` inside fn bodies |
| JavaScript | `export {..} from`, `export *`, `export * as ns`, `default` specifiers, import-then-export (`local_name`), `export default <imported>`: `syntax`. CommonJS `module.exports = require()` → `module`, `module.exports.x = require().x` / `exports.x = …` → `named`: `declared` | `parse_error` (import attributes), `shadowed_global`, `dynamic_forwarding` (merge forms), `rebound_binding` | local exports (they stay `ExportRecord`) |
| TypeScript | the JavaScript forms; statement or specifier `type` → `type_only`; `export =` imported binding → `module`/`declared`; `export import A = B.C` with `B` imported → `named`/`syntax` | `parse_error` / `grammar_gap` for `export type *` and `export import … = require()` | `export as namespace` |
| Python | names in a literal `__all__` bound by a module-level import, redundant aliases, `from m import *`: `declared`. Without `__all__`, every other public module-level import: `exposed`. `import a.b` exposes `a` as `namespace`. `if`/`try` → `conditional` | `computed_export_list`, `cross_file_export_list`, `rebound_binding` | function- or class-local imports, `__future__`, underscore names |
| Go | `type X = pkg.Y` (including generic aliases): `named`/`syntax`. Package-level `var`/`const X = pkg.Y`: `value_copy`/`declared`. Exported (uppercase) names only. A `//go:build` constraint → `conditional` on every entry | `mutable_binding` (`var`), `inferred_package_name` | defined types, embedding, plain/blank/dot imports |
| Perl | names in literal `@EXPORT`/`@EXPORT_OK`/`%EXPORT_TAGS` bound by `use Mod qw(..)` in the same package, with an Exporter marker: `named`/`declared`, `exported == original == local` | `computed_export_list`, `dynamic_forwarding` (custom `sub import`, globs), `rebound_binding` (later `sub NAME`), `grammar_gap` (qualified `@Pkg::EXPORT…`) | locally defined subs |
| Lua | `return require("m")` → `module`; a returned table constructor or `M.x = <import-derived>; return M` (U4–U7 rules) → `named`: `declared` | `shadowed_global` (`local require`), `rebound_binding`, `dynamic_forwarding` (`module()`, metatables) | table assignments with no final `return M` |
| Swift | `@_exported import M` → `wildcard`; `@_exported import struct M.T` → `named`: `syntax`, caveat `unstable_feature`; `#if` → `conditional` | — | `public import`, plain `import` |
| Scala 3 | `export` selectors: `named` (renames), `*`/`_` → `wildcard` with `excluded_names`, `given` → `wildcard` with caveat `givens_only`; `source` is the path; `syntax` | `parse_error` for modified or annotated `export` (never read as public); `grammar_gap` for multi-clause `export a.b, c.d.{e}` | Scala 2 (no construct) |
| C++ | namespace-scope `using ns::x;` → `named`; `using namespace ns;` → `wildcard` with caveat `fallback_lookup`; `scope` is the namespace; `syntax` | `grammar_gap` (whole file) when module syntax appears at translation-unit level, whether parsed as ERROR or misparsed as a declaration whose type is `import`, `module` or `export`; `parse_error` for `using typename` and multi-name `using` | type aliases, class-member `using`, `#include` |
| C# | `[assembly: TypeForwardedTo(typeof(N.T))]` → `named`/`syntax`, `exported_name` `N.T`, `source` `null`, caveat `source_unresolved` | — | all `using` forms |
| Java, C, Bash, Zsh, PHP | nothing: `not_applicable` | — | `requires transitive`, `#include`, `source`/`.`, `export -f`, `autoload`, `use`, `include`/`require`, `class_alias` |

### CLI behavior

- `--json-full` additionally requests `Capability::Reexports`. Each file
  index then carries `reexports` and `imports[].reexported`, and
  `completed_capabilities` lists `"reexports"`.
- `hug imports` text and JSON output do not change. Its import conversion
  reads the stored marker, so `is_reexport` agrees between fresh and cached
  runs.
- `--exported` and `exported_symbols()` stay declaration-only. The CLI guide
  line that says "re-exports included" is corrected to say what is actually
  listed (exported declarations, plus synthetic Rust prelude records under
  `--prelude`).
- No new command, flag or styled view is added, so no L2 terminal tests are
  required.
- Callers find out what is supported per language from
  `docs/reexports.md`, and per file from `coverage.state`.

### Cache invariants

The cache reader refuses an entry, recomputes it and rewrites it when any of
these hold:

- `reexports` is in `completed_capabilities` but the report is missing, or
  the report is present without the capability.
- The state is `limited` with no limits, or the state is not `limited` and
  limits exist.
- The state is `not_applicable` for a supported language, or a supported
  state is recorded for a `not_applicable` language.
- An entry's form does not match its name nullability (see the field docs
  above).
- An `import_span` does not equal the span of an `ImportRecord`.
- A `reexported` marker disagrees with the entries.
- A span lies outside the file's byte length.
- Duplicate keys, unknown enum values, or any key absent or `null` where the
  type does not allow it.

The plan's robustness matrix is expanded to these fields.

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
import conversion and full-index JSON. It adds no command and leaves
declaration listings unchanged (see **CLI behavior**). The Rust
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

## Resolved questions

The three open questions from the draft were decided on 2026-10-11. The
pros and cons that informed them are in git history for this file.

1. **Which conventions count:** dedicated syntax plus bounded static
   conventions, with categorical evidence (`syntax`, `declared`, `exposed`).
   There is no numeric confidence score. The supported patterns are in the
   per-language contract table.
2. **Canonical API and serialized representation:** a dedicated
   `ReexportReport` in the file index, exposed by `TreeFile::reexports()` and
   `Capability::Reexports`, plus `ImportRecord::reexported`. The method is
   named `reexports()` rather than reusing `reexported_symbols()`, because it
   returns a report, not symbols. `ImportSymbol::is_reexport` is kept for
   actual imported bindings.
3. **Declaration queries and CLI exposure:** the report is exposed
   separately and declaration queries stay about declarations. No synthesized
   alias declarations and no combined export command are added.

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
  and CLI behavior before it is marked finalized or planned. (Met 2026-10-11:
  see **Author rulings** and **Finalized design**.)

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
