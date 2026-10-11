# Audit notes: module-syntax languages (Rust, JavaScript, TypeScript, Swift, Scala)

Scope: Phase 1 audit rows for five languages only. Samples live in
`audit/samples/{rust,js,ts,swift,scala}/` (`pos-*` positive, `neg-*` negative,
`amb-*` ambiguous, `scala2-*` contrast).

## Method and provenance

- Parse evidence: `/tmp/th-probe/target/release/th-probe <lang> <file>`, built
  against the lockfile grammars: tree-sitter-rust 0.24.2, javascript 0.25.0,
  typescript 0.23.2 (LANGUAGE_TYPESCRIPT only; no TSX), swift 0.7.3, scala
  0.24.1; tree-sitter 0.26.9. No repo code or grammar was changed.
- **Probe limitation (important):** the probe prints only *named* nodes.
  Anonymous tokens (`type`, `default`, `*`, `as`, `=`, `struct`, ...) are
  invisible in its output. Where this matters I confirmed the structure from
  the pinned grammar's `src/grammar.json` in the cargo registry and I flag it
  as "grammar.json" below. A query or walker must test anonymous tokens by
  node kind or source text.
- Documentation: fetched on 2026-10-11 via WebFetch. Rows say "source not
  verified" where the fetched page did not actually state the fact. Nothing was
  filled from memory as verified.
- Abbreviations: `err` = `has_error`; "clean" = `has_error=false`, no MISSING.

## Cross-language findings that affect design

1. A node name containing "export" is not enough, as the spec warns, and here
   the opposite also bites: in TypeScript `export = x` and
   `export as namespace X` are both an `export_statement` whose only named child
   is a bare `identifier`; only the anonymous tokens differ.
2. Several discriminating facts are anonymous tokens: JS/TS `default` in
   `export_specifier` name/alias position (JS: anonymous, TS: probe shows
   `identifier "default"`), wildcard `*` in `export * from`, TS `type` on
   statement/specifier, Swift import kind (`struct`, `func`, ...), Scala
   `given`. Extraction must read token kind or text, not just named children.
3. Conditional context is *not* a wrapping node in Swift (`#if` appears as
   sibling `directive` nodes) but is a sibling `attribute_item` in Rust. Neither
   wraps the declaration, so "conditional" needs a preceding-sibling scan.
4. Macro bodies are opaque in Rust (`token_tree`); a `pub use` inside
   `macro_rules!` is not visible as a `use_declaration`.

---

## Rust

Dialect: Rust Reference (edition 2018+ rules; the reference page does not show
a version number). Grammar: tree-sitter-rust 0.24.2. All Rust samples parse
clean.

Sources: use declarations
<https://doc.rust-lang.org/reference/items/use-declarations.html>, visibility
<https://doc.rust-lang.org/reference/visibility-and-privacy.html>, extern crate
<https://doc.rust-lang.org/reference/items/extern-crates.html>.

Common node shape: `use_declaration` with optional `visibility_modifier` child
(unfielded) and `argument:` field.

| Construct | Sample | Class | Node types / fields (probe) | Parse | Expected result |
|---|---|---|---|---|---|
| `pub use a::B;` | `pos-simple.rs` | Dedicated | `use_declaration` > `visibility_modifier "pub"`, `argument: scoped_identifier(path: identifier a, name: identifier B)` | clean | exported `B`, original `B`, path `a::B`, form named, visibility `pub`, no alias |
| alias `pub use a::B as C;` | `pos-alias.rs` | Dedicated | `argument: use_as_clause(path: scoped_identifier, alias: identifier C)` | clean | exported `C`, original `B`, source `a`, alias `C` |
| grouped `pub use a::{B, C as D, self, e::F};` | `pos-grouped.rs` | Dedicated | `argument: scoped_use_list(path: identifier a, list: use_list[identifier, use_as_clause, self, scoped_identifier])` | clean | One forwarding entry per leaf, all sharing the one `use_declaration` span: `B`, `D`(orig `C`), `a` (from `self`), `F` (source `a::e`). Collapse duplicate captures per leaf, not per statement |
| `self` in braces `pub use a::{self};` and `{self as z}` | `pos-self-only.rs` | Dedicated | `use_list[self]`; alias form `use_as_clause(path: self, alias: identifier z)` | clean | exported name = last segment of the list's `path` (`a`), or `z`; form module (whole-module forward). Reference: parent must be module/enum/trait, binds type namespace only (semantic limit, cannot be checked in one file) |
| glob `pub use a::*;` | `pos-glob.rs` | Dedicated | `argument: use_wildcard(identifier a)`; nested `scoped_use_list(... use_wildcard(identifier b), ...)` | clean | one wildcard entry, source `a` (or `a::b`); no fabricated names. The `*` token is anonymous; the node kind `use_wildcard` is the discriminator |
| restricted `pub(crate)`, `pub(super)`, `pub(self)`, `pub(in crate::x)` | `pos-restricted.rs` | Dedicated, restricted | `visibility_modifier` with child `crate`, `super`, `self`, or `scoped_identifier(path: crate, name: x)`; the `in` and parens are anonymous | clean | keep the restriction text verbatim (`crate`, `super`, `self`, `in crate::x`). `pub(self)` equals private per the reference, so it should not count as a re-export in the effective sense; written-visibility vs reachability must stay separate (spec constraint) |
| leading path roots `crate::`, `self::`, `super::`, `::ext::` | `pos-use-leading.rs`, `pos-extern-crate.rs` | Dedicated | `path: crate`/`self`/`super` keyword nodes; leading `::` gives `scoped_identifier` with `name` only and no `path` | clean | source path keeps the root keyword; `::serde_json::Value` has no `path` field on the inner node, so path assembly must tolerate a missing `path` |
| nested modules | `pos-nested-mod.rs` | Dedicated | `mod_item > body: declaration_list > use_declaration` | clean | enclosing scope = `outer::inner` chain of `mod_item` names; `pub use` inside a private `outer` is still written-`pub` but not externally reachable (flag must not imply reachability) |
| `pub use` inside a function body | `pos-in-fn-pub.rs`, `neg-in-fn.rs` | Dedicated syntax in a block | `block > use_declaration` | clean | enclosing scope is a function body; Reference says use may appear in blocks but is silent on `pub` in a block, so reachability is **source not verified**. Recommend: record with scope kind `block`, never treat as module interface |
| conditional `#[cfg(...)]`, `#[cfg_attr(...)]` | `pos-cfg.rs` | Modifier on dedicated | sibling `attribute_item > attribute(identifier cfg, arguments: token_tree ...)` precedes the `use_declaration`; `cfg_attr` tokens are a flat `token_tree` (the arguments are reordered into a token list in the probe view) | clean | `conditional = true`, attribute text kept raw, not evaluated. Attribute is a *preceding sibling*, not a child; walk previous siblings that are `attribute_item` (also inner-doc/line comments may intervene, untested) |
| `pub extern crate serde;` / `as bar` | `pos-extern-crate.rs` | Dedicated (separate item kind) | `extern_crate_declaration(visibility_modifier, crate "crate", name: identifier, alias: identifier)`; the node child `crate "crate"` is the keyword | clean | exported `serde` (or `bar`), original `foo`, form module (whole-crate), source = external crate. Reference page for extern crate does not state `pub` semantics: **source not verified** for visibility; the rule is the generic item-visibility one |
| `pub use` of external crate item | `pos-extern-crate.rs` | Dedicated | `scoped_identifier(path: identifier serde ...)` | clean | source root is an identifier; one file cannot distinguish a crate from a local module (e.g. `serde` vs `mod serde`), so "external" is **not provable in-file**; record source path only, flag `external: unknown` |
| private `use a::B;` | `neg-private-use.rs` | Not a re-export | `use_declaration` with no `visibility_modifier` child | clean | no entry. Same for private `extern crate` |
| `pub struct/fn/mod/type/const` | `neg-pub-local.rs` | Not a re-export | item nodes with `visibility_modifier` | clean | no entry; `pub type T = a::B;` is a new alias item, not a use declaration |
| `pub use` generated by `macro_rules!` | `pos-macro.rs` | Gap (opaque) | `macro_definition > macro_rule > right: token_tree` | clean | not detectable; coverage limit `macro-expansion-not-analyzed` if any `macro_invocation`/`macro_definition` is present in the file |

Rust summary: dedicated syntax, fully parsed. Hazards are not grammar gaps but
the four semantic limits: restricted forms (keep text), block scope, macro
expansion, and external-crate identity. The existing behavior (any
`visibility_modifier` sets `is_reexport`) matches the grammar but is the
"written visibility" flag only; `pub(self)` is the weak case.

---

## JavaScript

Dialect: ECMAScript modules (MDN reference, citing the ECMAScript 2027
draft; no edition per feature given on the page) plus CommonJS (Node.js docs
v26.11.1). Grammar: tree-sitter-javascript 0.25.0.

Sources: <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/export>,
<https://nodejs.org/api/modules.html>.

Common shape: `export_statement` with `source: string` (the module string,
child `string_fragment`) and one of `export_clause`, `namespace_export`, or
neither (wildcard). `export_specifier` has `name:` and optional `alias:`.

| Construct | Sample | Class | Node types / fields | Parse | Expected result |
|---|---|---|---|---|---|
| named `export { a } from "m"` | `pos-named.js` | Dedicated | `export_statement(export_clause(export_specifier name: identifier a), source: string)` | clean | exported `a`, original `a`, source `m`, form named, no local binding (MDN: not brought into scope) |
| renamed `export { a as b, c, d as e } from "m"` | `pos-renamed.js` | Dedicated | `export_specifier(name, alias)`; one specifier per name | clean | one entry per specifier; exported = `alias` if present else `name`; original = `name` |
| `export { x as default } from "m"` | `pos-default-as-default.js` | Dedicated | `export_specifier(name: identifier x)`; the alias is the **anonymous token `default`** in field `alias` (grammar.json: `_module_export_name = identifier / string / 'default'`); probe shows no alias node | clean | exported `default`, original `x`; must read alias token text, `alias:` named-node queries miss it |
| `export { default } from "m"` | `pos-default-bare.js` | Dedicated | `export_specifier` whose `name` is the anonymous `default` token (probe prints `export_specifier "default"`, no children) | clean | exported `default`, original `default` (stays default per MDN) |
| `export { default as x } from "m"` | `pos-default-as-name.js` | Dedicated | `export_specifier(alias: identifier x)`, `name` is anonymous `default`; probe shows only the alias | clean | exported `x`, original `default` |
| string names `export { "a b" as "c d" } from "m"` | `pos-string-name.js` | Dedicated | `name:`/`alias:` are `string(string_fragment)` | clean | names are string literal contents; not valid identifiers (MDN) so keep unquoted text and flag `name_is_string` |
| wildcard `export * from "m"` | `pos-wildcard.js` | Dedicated | `export_statement` with `source` only; no `export_clause`, no `namespace_export`, no `declaration`/`value`. `*` is anonymous | clean | one wildcard entry, source `m`; MDN: re-exports named exports, **not default**; conflicting stars are resolved at link time, so no name enumeration |
| `export * as ns from "m"` | `pos-namespace.js` | Dedicated | `namespace_export(identifier ns)`, `source` | clean | exported `ns`, form namespace-module, source `m` (distinct from wildcard) |
| `export * as default from "m"` | `pos-namespace-default.js` | Dedicated | `namespace_export "* as default"` with **no identifier child** (name is anonymous `default`) | clean | exported `default`, form namespace-module; derive name from node text, not an `identifier` child |
| import attributes `export {a} from "m" with {type:"json"}` | `pos-attributes.js` | Dedicated, **grammar gap** | `ERROR` nodes; trailing `with {...}` parsed as a labeled statement | **err** | MDN says `export from` supports attributes. This pinned grammar cannot parse it: **support gap, not absence**. Expected result if parsed: same entry plus `attributes`. Coverage limit `parse-error` for the file |
| import-then-export `import {a as local} from "m"; export {local as b};` | `pos-import-export.js` | Convention | `import_specifier(name: a, alias: local)`; `export_specifier(name: local, alias: b)` with **no `source`** | clean | exported `b`, original `a`, local binding `local`, source `m`, form named. Needs: one same-file import binding keyed by local name |
| import default/namespace then export `import d from "m"; import * as ns from "n"; export {d, ns};` | `pos-import-default-export.js` | Convention | `import_clause(identifier d)` / `namespace_import(identifier ns)`; `export_specifier(name)` | clean | `d`: original `default`, source `m`; `ns`: form namespace-module, source `n` |
| `export default a;` where `a` is an import | `pos-import-then-export-default-id.js`, `pos-import-default-export.js` | Convention | `export_statement(value: identifier a)`; `value` field, no `export_clause` | clean | exported `default`, local `a`, resolve through import binding; if `a` is not an import it is a local export. `export default <expression>` of any other shape is not a forward |
| CommonJS `module.exports = require("m")` | `pos-cjs-module-exports.js` | Convention | `expression_statement > assignment_expression(left: member_expression(object: identifier module, property: property_identifier exports), right: call_expression(function: identifier require, arguments: arguments(string)))` | clean | whole-module forward (form module), exported name = module itself, source `m`. Valid only if `module` and `require` are unshadowed and the statement is at module top level, unconditional |
| `exports.x = require("m").y`, `module.exports.z = require("m")` | `pos-cjs-exports-prop.js` | Convention | `left: member_expression(object: identifier exports | member_expression(module.exports), property: x)`; `right: member_expression(object: call_expression require, property: y)` or bare `call_expression` | clean | exported `x`, original member `y`, source `m`; `z`: exported `z`, whole module `m` as a member. `exports` identity relies on no reassignment of `exports`/`module.exports` earlier (Node: reassigning `exports` breaks the link) |
| destructure then export `const {a} = require("m"); module.exports = {a};` and `exports.d = a;` | `pos-cjs-destructure.js` | Convention (indirect) | `variable_declarator(name: object_pattern, value: require call)`; `object(shorthand_property_identifier a)`, `pair(key, value: member_expression)` | clean | `a` -> source `m`; `b.c` -> exported `c`, original member `c`, local `b` -> `n`. Requires const binding to a literal `require` call; `b` must be `const` or never reassigned |
| local exports (negative) | `neg-local-export.js`, `neg-cjs-local.js` | Not a re-export | `export_specifier` with no `source` and no import binding for the name; `module.exports = {a: 1}`; `module.exports = require` (the function itself, no call) | clean | no entry |
| shadowed `require`/`module`/`exports` | `amb-cjs-shadowed.js` | Ambiguous | identical node shapes to the positives; shadowing exists only as `formal_parameters identifier` or `variable_declarator` in an enclosing function scope | clean | **do not report**; coverage limit `cjs-shadowed-binding`. Node docs: these are wrapper-function parameters, so a nested-scope shadow is legal; a top-level `let/const` redeclare is a SyntaxError |
| conditional / computed / dynamic CJS | `amb-cjs-conditional.js` | Ambiguous | `if_statement`, `ternary_expression`, `binary_expression` (`|| {}`), `require(name)` with identifier arg, `subscript_expression` target, `Object.assign(module.exports, require(...))`, `object(spread_element(require))` | clean | `if` / ternary / dynamic argument / computed key: not forwarded, coverage limit `cjs-conditional` / `cjs-dynamic`. Candidate extension (not required): `Object.assign` and spread forms are static wildcard-like forwards; report as limit unless the spec adds a "merge" form |
| import rebound or shadowed before export | `amb-import-rebound.js`, `amb-import-shadow-block.js` | Ambiguous | `export {b}` where `let b = a; b = 2`; `export {a}` after a function parameter `a` (the parameter is in a different scope, so the import still applies); `export` inside a `statement_block` after a block-level `const a` | clean | `export {b}` is a local export of a rebound variable: no entry. A `const`/`let` shadow at the same scope as the export clause defeats the forward: coverage limit `import-shadowed`. ES `export` statements are only legal at module top level in valid code, but the grammar accepts them in blocks (see `amb-import-shadow-block.js`), so the extractor must require `export_statement` parent = `program` |

JavaScript summary: ES module forwarding is dedicated syntax with full grammar
support except import attributes (gap). Anonymous `default` tokens are the
main trap. Import-then-export and CommonJS are conventions that need a
same-file binding table (import specifier name -> source/original; `const x =
require("lit")`) and a scope check for shadowing; anything conditional,
computed, or rebound gets a coverage limit rather than a guess.

---

## TypeScript

Dialect: TypeScript with `export type *` from 5.0. Grammar: tree-sitter-typescript
0.23.2, `LANGUAGE_TYPESCRIPT` (the TSX grammar is not used; `.tsx` was not
probed, but only differs for JSX/generic arrow syntax and not for module
statements as far as I can tell: **not verified**).

Sources: TS 5.0 release notes
<https://www.typescriptlang.org/docs/handbook/release-notes/typescript-5-0.html>,
modules reference
<https://www.typescriptlang.org/docs/handbook/modules/reference.html>,
namespaces handbook <https://www.typescriptlang.org/docs/handbook/namespaces.html>.
All JavaScript forms behave identically in the TS grammar (shape confirmed
for named, renamed, default forms, wildcard, namespace, import-then-export:
`pos-*.ts`). Difference: `default` shows as `identifier "default"` in
`name`/`alias` for TS (named node), unlike JS where it is anonymous.

| Construct | Sample | Class | Node types / fields | Parse | Expected result |
|---|---|---|---|---|---|
| all JS named/renamed/default/wildcard/namespace forms | `pos-named.ts`, `pos-renamed.ts`, `pos-default-forms.ts`, `pos-wildcard.ts`, `pos-namespace.ts`, `pos-import-export.ts` | Dedicated | same as JS (see above); `default` as `identifier` | clean | as JS rows; one extractor can serve both, but the `default` token test must accept both an anonymous token and an `identifier` named `default` |
| `export type { T, U as V } from "m"` | `pos-type-statement.ts` | Dedicated, type-only | grammar.json: `export_statement` alt `'export' 'type' export_clause (from)?`; the `type` token is anonymous, so the probe shows a plain `export_clause` and `source` (indistinguishable from the value form in named nodes) | clean | each specifier `type_only = true` from the statement-level qualifier; detect via anonymous `type` child. Handbook: guaranteed elided from output |
| `export { type T, U, type W as X } from "m"` | `pos-type-specifier.ts` | Dedicated, type-only per specifier | grammar.json `export_specifier = ('type'|'typeof')? name (as alias)?`; the qualifier is anonymous | clean | `type_only` per specifier; the probe hides it. The same sample shows no difference between `T` and `U` in named-node output, which proves token-level detection is mandatory |
| `export type * from "m"` | `pos-type-wildcard.ts` | Dedicated, type-only | `export_statement(ERROR "type", source)` | **err** | **Grammar gap.** TS 5.0 documents it; 0.23.2 cannot parse it. Do not treat as absent. Expected if parsed: wildcard entry + `type_only` |
| `export type * as ns from "m"` | `pos-type-namespace.ts` | Dedicated, type-only | `export_statement(ERROR "type", namespace_export(identifier ns), source)` | **err** | **Grammar gap**; the namespace part is recoverable inside the ERROR-adjacent tree but must not be trusted. Expected: namespace-module entry `ns`, `type_only` |
| import type / `{ type U }` then export | `pos-type-local-clause.ts` | Convention | `import_statement` with anonymous `type`; local `export_clause` | clean | `export type { T }` after `import type { T } from "m"`: forward of `T` from `m`, `type_only`. Needs the import's type-only flag; named nodes don't carry it |
| `export import A = B.C;` | `pos-export-import-alias.ts` | Dedicated (namespace alias) | `export_statement(declaration: import_alias(identifier A, nested_identifier(object B, property C)))` | clean for the dotted form | exported `A`, original member path `B.C`, form alias of a namespace/entity; enclosing scope = file module or `internal_module` (confirmed in `pos-ns-export-import.ts`, inside `namespace N` and `declare module "mod"`). Does the target come from another module? Only if `B` is an import binding: needs binding lookup, else it is a same-file entity alias. The handbook documents only non-exported `import q = x.y.z`; `export import` is **source not verified** |
| `export import D = require("m");` | `pos-export-import-alias.ts` | Dedicated | `import_alias(identifier D, identifier require, MISSING ";")` then a stray `expression_statement(parenthesized_expression)` | **err** (MISSING) | **Grammar gap.** The non-exported `import J = require("m")` parses (`import_statement > import_require_clause`), but the exported variant does not. Expected: exported `D`, source `m`, form module |
| local `import c = A.b;` (negative-ish) | `neg-ns-alias-local.ts` | Not a re-export | `import_alias` at top level, not under `export_statement` | clean | no entry (private alias) |
| `export = x` where `x` is an import | `pos-export-equals-import.ts`, `pos-export-equals-import2.ts` | Convention | `export_statement` with one named child `identifier x`; anonymous `=` token; `import x = require("m")` is `import_statement > import_require_clause(identifier, source)` | clean | whole-module forward: exported name is the module (CommonJS `module.exports`), source `m`; needs import binding lookup (`import x from "m"` too) |
| `export = require("m")` | `pos-export-equals-require.ts` | Convention | `export_statement(call_expression require)` | clean | whole-module forward, source `m`; subject to the shadowed-`require` rule |
| `export = x.y` / `export = { x }` | `amb-export-equals-call.ts` | Ambiguous | `member_expression`, `object` | clean | not a whole-module forward; member form is partial. Report limit or a `member` forward; recommend limit |
| `export = C` local | `neg-export-equals-local.ts` | Not a re-export | `export_statement(identifier C)` where `C` is a class | clean | no entry |
| `export as namespace MyLib;` | `pos-export-as-namespace.ts` | **Not a forward** | `export_statement(identifier MyLib)`, same shape as `export = x`; only the anonymous `as namespace` tokens differ | clean | UMD global declaration: exposes the *current module* as a global named `MyLib`. It declares a global name for this module and does not forward a dependency; produce no entry (and never confuse with `export = m` in the same sample). Authoritative source **not verified**: the library-structures page does not contain the syntax (stated by the fetch result); the classification rests on the TS declaration-file convention |
| ambient/declaration-file forms | `pos-ns-export-import.ts`, `pos-dts-reexport.d.ts` | Dedicated | `ambient_declaration > module(name: string, body: statement_block > export_statement ...)`; `.d.ts` re-exports parse identically | clean | `export *`/`export {..} from` inside `declare module "mod"` are re-exports of that ambient module, enclosing scope `module "mod"`. `declare global { }` parsed as a bare `statement_block` (harmless here) |
| decorators | `pos-decorator-export.ts` | Not a re-export | `export_statement(decorator, declaration: class_declaration)` | clean | no entry |
| local exports | `neg-local-export.ts` | Not a re-export | `export_clause` without `source`, bare `export default a`, `internal_module` | clean | `export type { T }` of a local type is not a forward; `export default a` follows the JS import rule |

TypeScript summary: JS forms plus statement/specifier type-only forms are
dedicated syntax and parse. Grammar gaps (0.23.2): `export type *` and
`export type * as ns from`, and `export import X = require("m")`. Type-only
qualifiers, `default`, and `=` are anonymous tokens. `export as namespace` is
a "no applicable construct" lookalike; `export =` of an imported binding is a
convention that needs the same import table as JS.

---

## Swift

Dialect: Swift language, with `@_exported` an underscored (unstable) attribute
and SE-0409 access-level imports (Swift 6.0). Grammar: tree-sitter-swift
0.7.3 (registry copy read for grammar.json).

Sources: SE-0409
<https://github.com/swiftlang/swift-evolution/blob/main/proposals/0409-access-level-on-imports.md>,
underscored attributes
<https://github.com/swiftlang/swift/blob/main/docs/ReferenceGuides/UnderscoredAttributes.md>.

Shape: `import_declaration(modifiers?, [anonymous kind keyword], identifier(simple_identifier+))`.
`grammar.json`: `import_declaration = modifiers? 'import' _import_kind? identifier`
with `_import_kind` one of typealias/struct/class/enum/protocol/let/var/func.
The kind keyword is anonymous and unfielded.

| Construct | Sample | Class | Node types / fields | Parse | Expected result |
|---|---|---|---|---|---|
| `@_exported import Foundation` | `pos-exported-import.swift` | Dedicated (underscored, unstable) | `import_declaration > modifiers > attribute > user_type > type_identifier "_exported"`, `identifier > simple_identifier` | clean | module re-export: form whole-module, source `Foundation`, no alias (Swift has none). Reference: "Re-exports all declarations from an imported module". Attribute text is `_exported`, without `@` |
| submodule `@_exported import Foo.Bar` | `pos-exported-submodule.swift` | Dedicated | `identifier(simple_identifier Foo, simple_identifier Bar)` | clean | source `Foo.Bar`. Indistinguishable in named nodes from `Foo` + member `Bar` of a selective import unless the anonymous kind keyword is checked |
| selective `@_exported import struct Foo.S` | `pos-exported-selective.swift` | Dedicated | the same `identifier(Foo, S)`; the `struct` kind token is anonymous and not visible in the probe | clean | exported name `S`, kind `struct`, source `Foo`. Whether `@_exported` is meaningful on a selective import: the underscored-attributes page is silent; **semantic status not verified**. Record as written |
| selective imports without `@_exported` | `pos-selective-import.swift` | Not a re-export | `import_declaration` with all 8 kinds parse (typealias/struct/class/enum/protocol/let/var/func; confirmed: struct class func typealias enum protocol var let) | clean | no entry |
| `public import Foo` (SE-0409) | `pos-public-import.swift` | **Access level, not re-export** | `modifiers > visibility_modifier "public"` | clean | SE-0409 (implemented Swift 6.0): access level controls *visibility of the dependency in this file's interface*. It says `@_exported` is accepted only on `public import`. Plain `public import` is **not** a forward of names; it makes the dependency visible in public API, so report `access_level` and do not set re-export unless `@_exported` also present. Important: imports without a modifier are `public` up to Swift 6 language modes, so absence of a modifier does not mean private |
| all access levels | `pos-access-imports.swift` | Access level | `visibility_modifier` text: public, package, internal, fileprivate, private; `public import struct F.S` parses | clean | `package import` parses. SE-0409 rejects `open import`; not probed |
| combos `@_exported public import Foo`; other attributes | `pos-access-with-attr.swift` | Dedicated + access | `modifiers` holds `attribute*` and `visibility_modifier` together; `@_spi(Priv)` is `attribute(user_type _spi, simple_identifier Priv)`; `@preconcurrency`, `@testable`, `@_implementationOnly` are plain attributes | clean | only `_exported` signals forwarding. `@_implementationOnly import` is the opposite (hidden dependency) |
| spacing / newline between attribute and import | `pos-exported-attr-spacing.swift` | Dedicated | same shape | clean | whitespace insensitive |
| inside `#if` | `pos-exported-ifconfig.swift` | Dedicated, conditional | **no wrapper node**: `#if canImport(Foo)` is a sibling `directive(simple_identifier Foo)` before the import and `directive "#endif"` after | clean | `conditional = true` requires scanning siblings for an open `#if` directive; condition text partly lost (only the identifier `Foo`, not `canImport(`). Treat as "conditional, condition text unavailable" |
| public typealias (negative) | `neg-typealias-public.swift` | Not a re-export | `typealias_declaration` with `visibility_modifier`; two `name:` fields | clean | `public typealias S = Foo.S` is a new declaration, not a forward: no entry (nearest related construct) |
| other attributes | `neg-other-attr.swift` | Not a re-export | `class_declaration > modifiers > attribute` | clean | no entry |

Swift stability: `@_exported` is documented as an underscored attribute whose
semantics are "subject to change" and discouraged outside the Swift monorepo,
yet SE-0409 treats it as the existing mechanism ("`@_exported` is accepted only
on public import declarations"). SE-0409 does not deprecate it. So: dedicated
syntax, unstable guarantee, report with a `stability: underscored` marker.
`public import` parse support is complete in 0.7.3, including `package`.
Not verified: the `-enable-experimental-feature AccessLevelOnImport` gating
before Swift 6 affects only compile acceptance, not parsing.

Swift summary: one dedicated construct (`@_exported import`, whole-module or
submodule, optionally selective), clean parse. SE-0409 access modifiers are
parsed but are an access control feature, not forwarding. Conditional
compilation is a sibling-scan problem.

---

## Scala

Dialect: Scala 3 export clauses. The Scala 3 reference page does not state the
release that introduced them (the page says only that `modularity` is
experimental): the introduction version is **source not verified**. Scala 2:
no `export` construct (the dropped-features page does not discuss export; the
absence claim for Scala 2 is **source not verified** by the fetched page).
Grammar: tree-sitter-scala 0.24.1 (it parses Scala 3 syntax).

Sources: <https://docs.scala-lang.org/scala3/reference/other-new-features/export.html>,
<https://docs.scala-lang.org/scala3/reference/dropped-features/package-objects.html>.

Shape: `export_declaration` with a flat sequence of `path: identifier` fields
(repeated), then optionally `namespace_selectors` or `namespace_wildcard`.
The exported member is the **last** `path` when no selector follows.
Grammar.json: `export` followed by comma-separated `_namespace_expression`s,
so `export a.b, c.d.{e, f}` is one `export_declaration` with the paths of both
clauses concatenated (`pos-export-multiple.scala`), losing the clause boundary.

| Construct | Sample | Class | Node types / fields | Parse | Expected result |
|---|---|---|---|---|---|
| `export a.b` | `pos-export-path.scala` | Dedicated | `export_declaration(path: a, path: b)` | clean | exported `b`, original `b`, source object path `a`, form named, enclosing scope = template/top-level |
| member of a local object, no import `export impl.run` | `pos-export-member-no-import.scala` | Dedicated | `val_definition` + `export_declaration(path: impl, path: run)` | clean | exported `run`, source path `impl` (a `val` in the same object). The qualifier is a stable value, not a module: this is member forwarding, which the spec warns must not be mistaken for a module-interface export; mark `source_kind: member-of-value` where the qualifier binds to a same-file `val`/`object` else `unknown` |
| selectors `export a.{b as c, given, *}` | `pos-export-selectors.scala` | Dedicated | `namespace_selectors(as_renamed_identifier(name, alias), namespace_wildcard "given", namespace_wildcard "*")` | clean | `c` (orig `b`); given-wildcard entry; wildcard entry. Each selector is its own forward entry; the `given` wildcard and `*` wildcard are separate entries, since `*` excludes givens (reference) |
| single wildcard `export a.*`, `export a.given` | `pos-export-selectors.scala`, `pos-export-givens.scala` | Dedicated | `namespace_wildcard "*"` or `"given"` directly, no `namespace_selectors` | clean | wildcard form; for `given`, a given-wildcard form |
| exclusion `export a.{x as _, *}`, `{x => _, *}` | `pos-export-exclusion.scala` | Dedicated | `as_renamed_identifier(name: x, alias: wildcard "_")` (or `arrow_renamed_identifier`) | clean | `x` is **excluded**, not renamed; produces an exclusion list on the wildcard entry, no entry named `_`. `alias: wildcard` is the discriminator |
| given by type `export a.{given T}`, `{given t}` | `pos-given-vs-plain.scala` | Dedicated | `namespace_selectors(type_identifier T)`, whereas plain `{T}` yields `identifier T`; lowercase `given t` also `type_identifier` | clean | selector is a *given-by-type* selector (not a member named `t`). The keyword `given` is not a node; the discriminator is `type_identifier` vs `identifier` in selector position, a grammar quirk. If a plain type member is ever exported by name this heuristic would be ambiguous: **needs a real-code check** before relying on it |
| `{given, b}` and `{b => c}` | `pos-export-givens.scala` | Dedicated | `namespace_wildcard "given"`, `identifier b`; `arrow_renamed_identifier` for old-style `=>` | clean | as above; `=>` is the older rename syntax, same semantics as `as` |
| multi-clause `export a.b, c.d.{e, f}` | `pos-export-multiple.scala` | Dedicated | single `export_declaration` with paths `a b c d` and `namespace_selectors(e, f)` | clean | **grammar flattening**: clause boundary is lost. The anonymous `,` must be consulted to split; otherwise the result would wrongly be `a.b.c.d.{e,f}`. Treat as a gap requiring token-level splitting |
| backticked name | `pos-given-vs-plain.scala` | Dedicated | `path: identifier "`weird name`"` | clean | keep backticks stripped in the name; text includes backticks |
| class member exports | `pos-export-class-member.scala` | Dedicated | `class_definition > template_body > export_declaration` | clean for plain; **err** with modifiers | enclosing scope = class `Facade` (a type's API, not a file-level module interface: coverage should describe it as member exposure) |
| access modifier `private export x` / `protected export x` | `pos-export-class-member.scala` | Dedicated, restricted | `ERROR(access_modifier private)` precedes a *successfully parsed* `export_declaration` | **err** | **Grammar gap**; the reference page does not discuss access modifiers on an export clause (**source not verified**), but the grammar cannot represent them, so visibility on exports is lost. The export node itself is recovered after the ERROR, so a missing-modifier hazard exists: it would be reported as public |
| `inline export`, `transparent inline export`, annotated `@deprecated export` | `pos-given-vs-plain.scala`, `pos-export-modifiers.scala` | Dedicated | `ERROR(inline_modifier ...)`, annotation swallows tokens | **err** | **Grammar gap**; the surrounding `export` is mis-parsed (one case eats `export a` as an error token). Whether these modifiers are legal on exports is **source not verified** |
| top-level export | `pos-export-top-level.scala` | Dedicated | `compilation_unit > export_declaration` | clean | enclosing scope = package (`package_clause` sibling `p`) |
| export of enum members | `pos-export-case.scala` | Dedicated | `export_declaration(path: E, namespace_wildcard)` | clean | normal wildcard; enum cases are members |
| import-then-export | `pos-import-then-export.scala` | Convention | `import_declaration(path: lib, path: Thing)` then `export_declaration(path: Thing, namespace_selectors)` | clean | the export's qualifier `Thing` resolves via the same-file import (including renames `Other as O`), so the source path is `lib.Thing`. Export works without the import when the qualifier is a local definition; resolving a leading path segment is optional |
| plain imports (negative) | `neg-import-only.scala` | Not a re-export | `import_declaration` with `namespace_wildcard "_"`/`"*"` | clean | no entry; note `import a._` uses `_` while exports in Scala 3 use `*` (both appear as `namespace_wildcard`) |
| local defs (negative) | `neg-local-def.scala` | Not a re-export | `type_definition` / `val_definition` | clean | `type T = a.T`, `val y = a.y` are aliases or values, not export clauses |
| Scala 2 contrast | `scala2-import-contrast.scala`, `scala2-export-ident.scala` | No applicable construct (Scala 2) | package object members `val x = q.x`, `type T = q.T`; Scala 2 `export` as an identifier gives `ERROR` | `scala2-export-ident.scala` **err** | in Scala 2 the nearest idiom is a forwarding `val`/`type`/`def` in a package object. That is a declaration, not a re-export, so classify as no applicable construct. A Scala 2 file using `export` as an identifier is a grammar-induced false-error (the grammar is Scala 3-oriented) |

Scala summary: dedicated syntax (Scala 3), well covered by the grammar except
export-clause modifiers/annotations and clause flattening. The semantic
traps are exclusion selectors, `given` selectors, wildcards that never
enumerate names, and qualifier meaning (object/value member vs. imported
module). Scala 2: no applicable construct.

---

## Open questions / grammar gaps

Grammar gaps (parse failures at the pinned version; record as support gaps):

1. JS 0.25.0: `export {..} from "m" with {type: "json"}` (import attributes) -> ERROR.
2. TS 0.23.2: `export type * from "m"` and `export type * as ns from "m"` -> ERROR (documented since TS 5.0).
3. TS 0.23.2: `export import D = require("m")` -> MISSING `;` plus stray statement.
4. Scala 0.24.1: access modifiers (`private export`, `protected export`), `inline` / `transparent inline`, and annotations before `export` -> ERROR; the export node is still produced afterwards without the modifier text.
5. Scala 0.24.1: multi-clause `export a.b, c.d.{e}` flattens into one node (not an error, a structural loss).
6. Scala 0.24.1: Scala 2 code using `export` as an identifier -> ERROR.
7. Swift: `#if` has no wrapper node (not a gap, a modeling constraint); the condition text is lost.
8. Rust: macro-generated `pub use` is opaque (not a grammar gap).

Design / semantic questions for the author:

- Do restricted Rust forms with `pub(self)` count as re-exports? The reference says it equals private.
- Swift `public import` without `@_exported`: report as access-level fact only (recommended) or as a re-export? The SE-0409 text separates them.
- Swift `@_exported` is unstable: should the support contract carry an `unstable` marker?
- TS `export import A = B.C`: is it a forward of a dependency, or only an entity alias when `B` is not an import? It requires binding lookup.
- CJS: are `Object.assign(module.exports, require(..))` and `{...require(..)}` supported as merge forwards, or limits? (Recommend limits.)
- Scala `given T` heuristic (`type_identifier` vs `identifier` in selector position) is untested on real code.
- Scala: is a member-of-value forward (`export impl.run`) in scope for "re-export"? It is member exposure, not a module interface; the spec leaves this open.
- `pub use` of block-scope items and `pub extern crate` semantics are not covered by the fetched Rust reference pages (source not verified).

Source-not-verified list: Scala 3 introduction version; Scala 2 has no export (absence); Rust `pub use` inside blocks; `pub extern crate` visibility; TS `export import` and `export as namespace` semantics; TSX grammar behavior; Swift `@_exported` on selective imports.
