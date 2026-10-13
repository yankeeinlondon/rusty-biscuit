---
kind: spike
fix: 2026-10-03-visibility-and-kind
spike: baseline
created: 2026-10-03
rev: 21debbcc5c98000f5b9a331be59e7ef24cdde66f
fixtures: spike/fixtures/
raw_output: spike/raw/
scanner: spike/summarize.py
---

# Baseline: which committed matrix cells hold today

`kind: spike` is used because `docs/kind-documents.md`'s formal enum
(`schema`, `schema-target`, `sequence`, `task`, `index`,
`canonical-documentation`) has no research or spike value. Spike documents
elsewhere in the repo use `kind: spike`.

## Summary

- **Three `ok` cells are wrong outright:**
  - Swift Method: every member function reports `Function`.
  - Java Namespace: `package` is never reported.
  - C++ union: `union` is not reported, and its fields come out as namespace
    members.
- **Three more `ok` cells hold only partly:**
  - Rust Method: associated functions report `Function`, and bodyless trait
    method declarations are missing.
  - Python Method: `@staticmethod`/`@classmethod` report `Function`.
  - C named tag: a named `union` tag is not reported.
- **Every `LIM` cell holds** (Go and Swift `Type`).
- **The extra assertions match the spec's description.** Rust `const` →
  `Constant` holds. Rust `static`/`static mut` and Perl `my`/`our`/`state` are
  missing (they are already FIX).
- **The defects are confirmed across all 19 output files.** None of them has a
  v1 `container_kind`, a v1 `signature.visibility` or a v1
  `signature.is_static`, even where v2 holds a level or a `Static` modifier.
- **The largest surprise affecting scope is C++.** Methods defined inside a
  class body (with `{}`) are not reported at all; only bodyless member
  declarations are. Members of a class nested in a namespace get the
  *namespace* as their container, not no container.

## Method

1. Wrote one fixture per language under `spike/fixtures/`, plus three
   JavaScript/TypeScript edge files: `typescript_script.ts`,
   `javascript_script.js` and `javascript_commonjs.js`. The JS/TS visibility
   section requires those files.
2. Checked syntax with the native tools:
   - `rustc --emit=metadata`, `tsc --noEmit`, `node --check`, `gofmt -e`
   - `python ast.parse`, `php -l`, `luac -p`, `perl -c`, `bash -n`, `zsh -n`
   - `clang -fsyntax-only`, `clang++ -std=c++20 -fsyntax-only`,
     `swiftc -parse`

   All of them pass. `java.java` parses, but `javac` rejects it because it
   puts several public top-level types in one file; its header comment says
   so. No C# or Scala compiler was available.
3. Built `hug` with `cargo build -p tree-hugger-cli` at the `rev` above.
   `hug --version` reports 0.1.0.
4. Copied the fixtures into a scratch git repository. For each file, ran:
   - `hug --json symbols --no-cache <file>`, saved to `raw/<stem>.json`
   - `hug --plain symbols|classes|functions|types --no-cache`, saved to
     `raw/plain/<stem>.txt`

   The scratch path prefix is stripped from the saved output.
5. `python3 summarize.py raw/<stem>.json` prints one row per symbol with the
   v1 `files[].symbols` record and the v2 `symbol_indexes` record side by side.
   Each row shows the kind, container, whether a signature is present, the
   signature's visibility and static flag, the v2 level, v2 modifiers and v2
   `module_path`. Type-metadata fields are listed under their owner.

The scratch repo is
`$TMPDIR/claude-501/.../scratchpad/baseline` and is disposable.

## Kind cells: committed `ok` / `LIM` → actual

The table quotes v1 kind / v2 kind; they agree unless both are shown. FIX
cells are listed only when the actual output differs from the spec's
description of the defect.

### Rust

| Cell | Fixture symbol | Actual | Verdict |
|---|---|---|---|
| Function `ok` | `public_function` | Function | holds |
| Method `ok` | `instance_method(&self)` | Method `[in impl PublicStruct]` | holds for `self` methods |
| Method `ok` | `new_associated_function()` (no `self`) | **Function** `[in impl PublicStruct]`, v2 `Static` | **partly wrong**: section 6 says direct members are Method |
| Method `ok` | `PublicTrait::required_trait_method(&self) -> u32;` (no body) | **not reported**; only its `self` parameter (78:31) appears | **WRONG**: `function_signature_item` is not captured |
| Method `ok` | `PublicTrait::provided_trait_method` (body) | Method `[in PublicTrait]` | holds |
| Trait `ok` | `PublicTrait`, `PrivateTrait` | Trait | holds |
| Enum `ok` | `PublicEnum` | Enum | holds |
| Type `ok` (struct) | `PublicStruct` | Type | holds |
| Field `ok` | `public_field` | Field `[in PublicStruct]` | holds |
| EnumVariant `FIX` | `UnitVariant` | Field | as described. Also: struct-variant field `variant_field` reports `Field [in PublicEnum]` |
| union `FIX` | `PublicUnion` | not reported; `union_int_field` has no container | as described |
| TypeAlias `FIX` | `PublicAlias` | not reported | as described |
| Module `FIX` | `outer_module` | Namespace, with `container_name: "outer_module"` (**its own name**) | as described. Surprise: a `mod` is its own container |

### TypeScript

| Cell | Fixture symbol | Actual | Verdict |
|---|---|---|---|
| Function `ok` | `exportedFunction`, `moduleFunction` | Function | holds |
| Method `ok` | `publicMethod` | Method `[in ExportedWidget]` | holds |
| Method `ok` | `#hashPrivateMethod()` | **not reported** | **surprise**. The `#hashPrivateProperty` field is reported |
| Class `ok` | `ExportedWidget`, `ModuleWidget` | Class | holds |
| Interface `ok` | `ExportedInterface` | Interface | holds; `interfaceMethod()` signature not reported (not committed) |
| Enum `ok` | `ExportedEnum` | Enum | holds |
| Constructor `FIX` | `constructor` | `unknown` / Constructor | as described |
| abstract class `FIX` | `ExportedAbstractBase` | not reported. Its `concreteProtectedMethod` is a Method with **no container**; `abstract abstractMethod(): void;` is not reported | as described, plus the abstract member signature |
| TypeAlias `FIX` | `ExportedAlias` | `Unknown` / TypeAlias | as described |
| Property `FIX` | `publicProperty` | `Unknown` / Property | as described |

### JavaScript

| Cell | Fixture symbol | Actual | Verdict |
|---|---|---|---|
| Function `ok` | `exportedFunction` | Function | holds |
| Method `ok` | `unmarkedMethod`, `#hashPrivateMethod`, `staticMethod` | Method `[in ExportedWidget]` | holds (JS reports `#` methods; TS does not) |
| Class `ok` | `ExportedWidget` | Class | holds |
| Constructor `FIX` | `constructor` | `Unknown` / Constructor | as described |
| Property `FIX` | `unmarkedProperty = 1` | **Field** / Field, not Unknown | FIX holds, but the starting kind is Field |
| (surprise) | `this.unmarkedProperty = initialValue` in the constructor | **Variable** `[in ExportedWidget]` (23:10) | assignment through `this` is reported |

### Go

| Cell | Fixture symbol | Actual | Verdict |
|---|---|---|---|
| Function `ok` | `ExportedFunction`, `ÉtéFunction` | Function | holds |
| Method `ok` | `ExportedMethod` | Method, **no container** | holds (the cell asks for no container) |
| Type `LIM` (interface) | `ExportedInterface` | Type | holds |
| Type `LIM` (struct) | `ExportedStruct` | Type | holds |
| Namespace `ok` | `package symbols` | Namespace | holds |
| (FEAT, surprise) | `type ExportedAlias = ExportedStruct` | **not reported at all** | wrong kind would be expected; absent instead |

### Python

| Cell | Fixture symbol | Actual | Verdict |
|---|---|---|---|
| Function `ok` | `module_function` | Function | holds |
| Method `ok` | `instance_method`, `__init__` | Method `[in PlainWidget]` | holds |
| Method `ok` | `@staticmethod static_method`, `@classmethod class_method` | **Function** `[in PlainWidget]`, v2 `Static` | **partly wrong**. `hug classes` omits both entirely |
| Class `FIX` | `PlainWidget` | Type | as described |
| Field `FIX` | class body | All five class-body names are `Field`. `self.assigned_through_self` appears twice (29:14, 33:14). `self.unannotated_static_field` duplicates the class attribute (30:14). `target_object.assigned_from_outside` is a container-less Field (49:19) | as described |

### Java

| Cell | Fixture symbol | Actual | Verdict |
|---|---|---|---|
| Method `ok` | `publicMethod` etc. | Method `[in PublicWidget]` | holds |
| Enum `ok` | `PublicColor` | Enum | holds |
| Type `ok` (record) | `PublicPoint` | Type | holds |
| Namespace `ok` | `package com.example.symbols;` | **not reported** | **WRONG**. Also confirmed on the existing `Sample.java`, `types.java` and `imports.java`: no package symbol in any of them |
| Field `ok` | `publicField` | Field `[in PublicWidget]` | holds. The interface constant `int SIDES = 0;` is not reported |
| Class `FIX` | `PublicWidget` | Type | as described |
| Interface `FIX` | `PublicShape` | not reported; members `area`/`describe`/`unit` are reported `[in PublicShape]` | as described |

### C#

| Cell | Fixture symbol | Actual | Verdict |
|---|---|---|---|
| Method `ok` | `PublicMethod` etc. | Method `[in PublicWidget]` | holds |
| Class `ok` | `PublicWidget`, `FileLocalClass`, `NestedHelper` | Class | holds |
| Interface `ok` | `IShape` | Interface | holds |
| Enum `ok` | `PublicColor` | Enum | holds |
| Type `ok` (struct) | `PublicStruct` | Type | holds |
| Field `ok` | `PublicField` | Field `[in PublicWidget]` | holds |
| Namespace `FIX` | `namespace Example.Symbols;` | not reported | as described |

### C

| Cell | Fixture symbol | Actual | Verdict |
|---|---|---|---|
| Function `ok` | `public_function`, `file_static_function` | Function | holds |
| Enum `ok` | `NamedColor` | Enum | holds (enumerators are Variable; FEAT) |
| Type `ok` (named tag) | `struct NamedTag` | Type | holds for struct |
| Type `ok` (named tag) | `union NamedUnion` | **not reported**; `union_int_field` is a container-less Field | **wrong if the cell includes unions** |
| Field `ok` | `tag_field` | Field `[in NamedTag]` | holds. `anonymous_struct_field` has no container |
| TypeAlias `FIX` | `TagAlias`, `CountAlias` | Type | as described |

### C++

| Cell | Fixture symbol | Actual | Verdict |
|---|---|---|---|
| Function `ok` | `namespaceFunction`, `helper` ×3, `api` | Function | holds |
| Enum `ok` | `ScopedColor`, `PlainColor` | Enum | holds |
| Type `ok` (struct) | `PlainStruct` | Type | holds |
| Type `ok` (union) | `union PlainUnion` | **not reported**; fields `unionIntField`/`unionFloatField` report `[in fixture_space]` | **WRONG** |
| Class `FIX` | `AccessWidget` | Type | as described |
| Method with container `FIX` | `firstPublicMethod();` (declaration) | Method, container **`fixture_space`** (the namespace) | **worse than described**: wrong container, not missing |
| Method with container `FIX` | `secondPublicMethod() {}`, `instanceMethod() const {…}`, `protectedMethod() {}`, `explicitPrivateMethod() {}`, `PlainStruct::structPublicMethod() {}` | **not reported at all** | **surprise**: in-class method *definitions* are not captured |
| Field with container `FIX` | `firstPublicField` | Field `[in fixture_space]` | wrong container. `PlainStruct`'s fields do get `[in PlainStruct]` |

### Swift

| Cell | Fixture symbol | Actual | Verdict |
|---|---|---|---|
| Function `ok` | `publicFunction` | Function | holds |
| Method `ok` | `instanceMethod`, `staticMethod`, extension members, etc. | **Function** `[in OpenWidget]` | **WRONG**. Also true on the existing `signatures.swift` and `types.swift` (`function greet(...) [in Greeter]`). `hug classes` lists no Swift methods at all |
| Type `LIM` (class) | `OpenWidget` | Type | holds |
| Interface `ok` | `PublicDrawable` | Interface | holds. Protocol requirements: `func draw()` not reported; `var drawableName` is a Variable |
| Type `LIM` (enum) | `PublicDirection` | Type | holds |
| Type `LIM` (struct) | `PublicApi` | Type | holds |
| Field `ok` | `publicProperty` | Field `[in OpenWidget]` | holds |
| EnumVariant `FIX` | `north`, `east` | Field | as described |
| (surprise) | `let localValue` inside a function | **Field**, no container | a local is reported as a Field |

### Scala

| Cell | Fixture symbol | Actual | Verdict |
|---|---|---|---|
| Function `ok` | `topLevelFunction` | Function | holds |
| Class `ok` | `PlainWidget`, `PrivateClass` | Class | holds |
| Trait `ok` | `PlainTrait` | Trait | holds |
| Enum `ok` | `PlainColor` | Enum | holds |
| Class `ok` (case class) | `CasePoint` | Class | holds |
| Module `ok` (object) | `StandaloneObject`, companion `PlainWidget` | Module | holds |
| Method with container `FIX` | trait/object members | Function, no container | as described |
| Method with container `FIX` | **class** members `publicMethod`, `CasePoint.sum` | **Function** `[in PlainWidget]` | the FIX also covers class bodies: the container is present but the kind is Function |

### PHP

| Cell | Fixture symbol | Actual | Verdict |
|---|---|---|---|
| Function `ok` | `topLevelFunction` | Function | holds |
| Class `ok` | `PublicWidget` | Class | holds |
| Interface `ok` | `Shape` | Interface | holds |
| Trait `ok` | `Greets` | Trait | holds |
| Enum `ok` | `Suit` | Enum | holds |
| Field `ok` | `publicField`, `var $legacyUnmarkedField` | Field `[in PublicWidget]` | holds |
| Method with container `FIX` (trait) | `greet` | Method, no container | as described |

### Lua, Perl, Bash, Zsh

| Cell | Actual | Verdict |
|---|---|---|
| Lua Function `ok` | `globalFunction`, `chunkLocalFunction`, `Widget.tableFunction`: Function | holds. `Widget:methodWithSelf` is already Method (FEAT cell); `Widget.tableField` is a Variable with no container |
| Perl Function `FIX`, Module `FIX` | `(no symbols)` | as described |
| Bash Function `ok` | all three function syntaxes: Function | holds |
| Zsh Function `ok` | all three function syntaxes: Function | holds |

## Extra required assertions

| Assertion | Actual | Verdict |
|---|---|---|
| Rust `const` → Constant | `PUBLIC_CONSTANT`, `PRIVATE_CONSTANT`: Constant | holds |
| Rust `static` / `static mut` → Variable | `PUBLIC_STATIC`, `PRIVATE_STATIC_MUT` and the function-local `LOCAL_STATIC`: **not reported** | FIX (matches the gap table) |
| Perl `our` / `my` / `state` / list `my` → Variable; `local` → none | nothing reported | FIX (matches) |
| C `typedef struct AliasedTag {…} TagAlias;` → `AliasedTag: Type`, `TagAlias: TypeAlias`, once each | `AliasedTag` Type (11:9) once; `TagAlias` **Type** (11:1) once, carrying a duplicate copy of the struct's field metadata | Tag holds; alias is FIX |
| C anonymous typedef struct → alias only, no invented tag | `AnonymousStructAlias` Type only; its field has no container | no invented tag: holds; alias kind is FIX |
| C `typedef struct SameName {…} SameName;` → two distinct symbols | `SameName` Type at 19:1 and Type at 19:9 | holds (distinct spans); alias kind is FIX |

## Visibility, static and container kind today

These hold across all 19 output files (`summarize.py` aggregate):

- v1 `container_kind` is set: **0** symbols
- v1 `signature.visibility` is set: **0**
- v1 `signature.is_static` is true: **0**

The adapter defect is confirmed exactly as described. v2 already holds some of
the data:

| Language | v2 `visibility.level` today | v2 `Static` modifier today | Field-metadata visibility/static |
|---|---|---|---|
| Rust | `pub`, `pub(crate)`, `pub(super)`, `pub(in …)` and **`pub(self)`** all → Public. Unmarked → Unknown. Consts, types, mods, fields, trait items → Unknown | associated fns (`new_associated_function`, `crate_associated_function`) | Rust struct fields have **no** visibility in metadata, even `pub` |
| TypeScript | methods: public/protected/private correct. Unmarked method, `export function`, classes, aliases, properties → Unknown | `staticMethod`, `privateStaticMethod` | correct (public/protected/private, `is_static`); `#hashPrivateProperty` is **missing** from metadata |
| JavaScript | all Unknown, including `#` members and module/script/CommonJS top-level declarations | `staticMethod`, `#hashPrivateStaticMethod`; static fields have no modifier | n/a |
| Go | all Unknown (as described) | none | none |
| Python | all Unknown (correct per spec) | `static_method`, `class_method` (both Function kind) | `ClassVar[...]` fields have `is_static: false`; the unannotated class attribute is absent from metadata (as described) |
| Java | public/protected/private correct on methods; unmarked → Unknown. **Interface members are already Public** (`area`, `describe`, `unit`, and `packageShapeMethod` in a package-private interface). Types/fields → Unknown | `staticFactory`, `packageStaticMethod`, `unit` | correct, including `is_static` |
| C# | public/protected/private/internal correct on methods. `protected internal` → **Protected**, `private protected` → **Private**. Unmarked → Unknown. Unmarked interface members `Area`/`Draw`/`Unit` already Public; `private void Log()` → Private. Explicit implementation `IShape.Area` → Unknown. Types/fields → Unknown | `StaticFactory`, `Unit` | correct, with compound modifiers collapsed the same way. **Interface metadata is garbage**: `IShape` lists `Area`, `Draw`, `IShape` (for `Unit`) and `Log` as *fields*, with the whole declaration as `type_annotation` and all marked Public |
| C | all Unknown | none (no StaticStorage exists yet) | none |
| C++ | all Unknown, including methods in `public:` sections (not just "after the first") | `staticFactory` only. **File-scope `static` functions and variables have no `Static` modifier**, contrary to the spec's note that `extract_cpp_is_static` flags them. The standalone `staticMemberField` also has no modifier | **access sections are correct in metadata** (`secondPublicField` Public, `protectedField` Protected). Implicit class-private and struct-public fields carry no level |
| Swift | `public` → Public, `internal` → Internal, `private` → Private. **`fileprivate` → Private** (should be ModulePrivate). **`open` and `package` → Unknown**. Unmarked → Unknown | `staticMethod` only; `class func classMethod` has none | `staticProperty` and `classProperty` have `is_static: true`; visibility present only when explicit |
| Scala | all Unknown (as described) | object members (`companionStaticMethod`, `objectMethod`, `privateObjectMethod`) | no visibility |
| PHP | public/protected/private correct on methods. Unmarked method → Unknown (should be Public) | `traitStaticMethod`, `staticFactory`, `unmarkedStaticMethod` | correct |
| Lua, Perl, Bash, Zsh | all Unknown | none | n/a |

**Container kind:** absent everywhere, as described. The v1 `container_name`
is present where v2 `module_path` is set. Rust impl members use
`impl PublicStruct` / `impl PublicTrait for PublicStruct`. Consequently
`hug --plain classes` lists **no Rust methods at all**, because the container
name never equals the struct name.

**Signature presence (section 5):** the signature is absent (`-`) for:

- parameterless, return-less declarations in TypeScript, JavaScript, Swift and
  Go (`unmarkedMethod`, `constructor() {}`, `exportedFunction`,
  `instanceMethod`, `ExportedFunction`)
- **every Lua function, even with parameters** (`globalFunction(parameterValue)`)
- every Bash/Zsh function
- **C# expression-bodied methods returning a user type**:
  `public static PublicWidget StaticFactory() => …` and
  `static IShape Unit() => …` have no signature, while
  `public int PublicMethod() => PrivateField` has one. `hug classes` prints a
  bare `StaticFactory`

## `ok` cells to flip to FIX

Definite:

1. **Swift Method.** Every member function is `Function`, so the kind must
   become Method. `hug classes` then gains Swift methods as a side effect.
2. **Java Namespace.** `package` is never captured; `queries/vendor/java/locals.scm`
   needs a `package_declaration` pattern.
3. **C++ Type (union).** `union` is never captured, and its fields are
   attributed to the enclosing namespace.

Partial (the cell holds for its common case but not for constructs in the
fixture; recommend splitting the cell or flipping it):

4. **Rust Method.** Associated functions without `self` report `Function`, and
   bodyless trait method declarations (`function_signature_item`) are not
   reported at all.
5. **Python Method.** `@staticmethod` and `@classmethod` report `Function`, so
   `hug classes` drops them. The static matrix requires them listed under
   *Static Methods*, so this has to be fixed anyway.
6. **C Type (named tag).** A named `union` tag is not reported, though the
   named `struct` tag holds.

## Other surprises affecting scope

- **C++ in-class method definitions are not reported at all.** Only bodyless
  member declarations (`void f();`) become Methods. Every `void f() {}` inside
  a `class`/`struct` body is missing, so the C++ Method FIX is a query gap
  plus a container gap. Class members inside a `namespace` get the namespace
  as container, not none.
- **Scala class-body methods are `Function` too**, with a correct container.
  The Scala Method FIX covers classes, not only traits and objects.
- **TypeScript `#private` methods are not reported** (JS reports them).
  Abstract and interface method signatures are not reported either.
- **Rust**: `pub(self)` reports Public. A `mod`'s container is its own name.
- **Swift**: `let` locals are reported as `Field` with no container.
  Protocol method requirements are missing; protocol properties are Variable.
  The required "protocol requirements inherit their container" assertion needs
  a requirement symbol to exist, and today only `var drawableName` (Variable)
  does.
- **JavaScript**: `this.x = …` inside a method is reported as a `Variable` in
  the class. That is a duplicate of the class field, much like the Python
  `self.x` defect.
- **C# interface type metadata lists methods as fields.** The type text is the
  whole declaration, and `private void Log()` is marked Public. That is outside
  the matrix, but the spec requires `FieldInfo` and standalone visibility to
  agree.
- **C# expression-bodied methods returning a non-primitive type have no
  signature.** Section 5 requires one.
- **Lua functions never carry a signature**, even with parameters. Neither do
  Bash and Zsh functions. Under section 5, every Function needs `Some`.
- **Go `type A = B` is not reported at all.** This is a FEAT cell, but the
  symbol is absent rather than mis-kinded.
- **Java interface constants are not reported.** Java interface members
  already resolve to Public in v2, so part of the Java visibility work exists.
- **C# interface defaults are already Public** for unmarked members, with an
  explicit `private` winning. The explicit-implementation → Private rule is
  missing.
- **C++ access sections already work in field metadata, but not for
  methods.** The section walk exists and can be reused.
- **C++ file-scope `static` currently produces no `Static` modifier.** The
  planned move of file-scope `static` from `Static` to `StaticStorage`
  therefore has nothing to remove in today's output; it only needs to add
  `StaticStorage`.
- **`hug classes` drops Function-kinded members** (Swift, Scala, Python
  decorated methods) and **all Rust impl methods**, because the container
  name is `impl X`. The spec's CLI wiring checks use TypeScript, which is
  unaffected. A Rust or Swift `hug classes` assertion would need the
  container fix.

## Scope impact

- **Kind work grows by about four query or classification items:**
  - Java `package_declaration`
  - C++ `union_specifier` and in-class `function_definition` capture
  - Rust `function_signature_item` capture
  - Swift member functions → Method, matching what Scala, Rust associated
    functions and Python decorated methods already need under section 6's
    "direct member functions are Method" rule

  The Swift, Scala, Python and Rust-associated-function reclassifications
  are one shared rule in `refine_symbol_kind`/`find_symbol_container`, not
  four separate changes.
- **C++ grows the most.** The query gap is larger than a container fix, and
  containment must prefer the class over the enclosing namespace.
- **Visibility is somewhat smaller than feared for Java, C#, PHP and
  TypeScript methods.** Their keyword levels already reach v2; the work there
  is defaults, non-function kinds and the adapter. Rust (`pub(self)`, Crate),
  Swift (`open`/`package`/`fileprivate`), C/C++, Go, Scala, Lua and JS/TS
  module rules are built from scratch, as the spec expects.
- **Section 5 reaches further than TypeScript's `tick`.** Lua (every
  function), Bash/Zsh and C# expression-bodied methods also lack signatures.
- **Out-of-matrix defects found here go in the implementation log, not this
  fix:** Go alias, Swift local-as-Field, JS `this.x`, C# interface metadata,
  and TS interface/abstract signatures.

## Files

- `fixtures/<language>.<ext>`: 16 fixtures (`rust.rs`, `typescript.ts`,
  `javascript.js`, `go.go`, `python.py`, `java.java`, `c_sharp.cs`, `c.c`,
  `cpp.cpp`, `swift.swift`, `scala.scala`, `php.php`, `lua.lua`, `perl.pl`,
  `bash.sh`, `zsh.zsh`), plus `typescript_script.ts`, `javascript_script.js`
  and `javascript_commonjs.js`
- `raw/<stem>.json`: `hug --json symbols` output
- `raw/plain/<stem>.txt`: plain `symbols`, `classes`, `functions` and `types`
  output
- `summarize.py`: the side-by-side v1/v2 summarizer used for every table above
