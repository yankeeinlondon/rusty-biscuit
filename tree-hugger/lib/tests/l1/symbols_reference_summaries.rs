//! What the reference occurrences feed: the `references`, `dependencies`
//! and `referenced_by` summaries, `semantics.is_recursive`, and the
//! `Occurrences` capability (whole-file evidence whatever kinds are
//! selected, one references run and one resolution per analysis). User
//! contract: `tree-hugger/docs/references.md`.

use std::io::Write;

use tree_hugger::{
    Capability, Confidence, FileSymbolIndex, OccurrenceKind, ProgrammingLanguage, SymbolKindV2, SymbolQuery,
    SymbolRecord, SymbolRef, TreeFile, TriState, WorkCounters, work_counters,
};

use crate::symbols_fixtures::{display_name, primary_fixture};
use crate::symbols_occurrences::{EVAL_CALL_FORMS, EvalMode, eval_source};

fn counted<T>(work: impl FnOnce() -> T) -> (T, WorkCounters) {
    let before = work_counters();
    let value = work();
    (value, work_counters().since(before))
}

/// `source` parsed from a temporary `.{extension}` file.
fn parse(source: &str, extension: &str) -> (tempfile::NamedTempFile, TreeFile) {
    let mut file = tempfile::Builder::new()
        .suffix(&format!(".{extension}"))
        .tempfile()
        .expect("create temp source");
    file.write_all(source.as_bytes()).expect("write temp source");
    let tree_file = TreeFile::new(file.path()).expect("parse inline source");
    (file, tree_file)
}

fn complete(source: &str, extension: &str) -> FileSymbolIndex {
    parse(source, extension).1.symbol_index_v2().expect("complete index")
}

/// The only record called `name` of a function-like kind, else the only
/// record called `name`.
fn record<'a>(index: &'a FileSymbolIndex, name: &str) -> &'a SymbolRecord {
    let named: Vec<&SymbolRecord> = index.symbols.iter().filter(|record| record.identity.name == name).collect();
    let callables: Vec<&SymbolRecord> = named
        .iter()
        .copied()
        .filter(|record| matches!(record.kind, SymbolKindV2::Function | SymbolKindV2::Method | SymbolKindV2::Constructor))
        .collect();
    match (callables.as_slice(), named.as_slice()) {
        ([only], _) | ([], [only]) => only,
        _ => panic!("no single record `{name}`: {named:#?}"),
    }
}

/// Each reference as `(name, target ID present)`.
fn names(refs: &[SymbolRef]) -> Vec<(&str, bool)> {
    refs.iter().map(|reference| (reference.name.as_str(), reference.id.is_some())).collect()
}

/// `(label, extension, source, expected is_recursive by callable name)`.
type RecursionRow = (&'static str, &'static str, &'static str, &'static [(&'static str, TriState)]);

use TriState::{No, Unknown, Yes};

/// The direct-recursion rules, one row per rule: `Yes` only for a `Scoped`
/// call to the callable's own ID (it wins over any other uncertainty); `No`
/// only with complete call coverage and no call that might be to itself;
/// otherwise `Unknown`.
const RECURSION_MATRIX: &[RecursionRow] = &[
    (
        "direct self-call",
        "rs",
        "fn countdown(n: u32) -> u32 { if n == 0 { 0 } else { countdown(n - 1) } }\nfn leaf() -> u32 { 1 }\n",
        &[("countdown", Yes), ("leaf", No)],
    ),
    ("self-value reference is not a call", "rs", "fn f() { let _keep = f; }\n", &[("f", No)]),
    ("call through a parameter", "rs", "fn f(callback: fn()) { callback(); }\n", &[("f", Unknown)]),
    (
        "a variable holding the function is not the function",
        "rs",
        "fn f() { let again = f; again(); }\n",
        &[("f", Unknown)],
    ),
    (
        "proven self-call wins over an indirect call",
        "rs",
        "fn f(callback: fn()) { callback(); f(callback); }\n",
        &[("f", Yes)],
    ),
    ("method call on a receiver", "rs", "struct W;\nimpl W { fn m(&self) { self.m(); } }\n", &[("m", Unknown)]),
    (
        "`Self::f` establishes the one inherent method",
        "rs",
        "struct W;\nimpl W { fn build() -> W { Self::build() } }\n",
        &[("build", Yes)],
    ),
    (
        "unresolved call of the callable's own name",
        "rs",
        "struct W;\nimpl W { fn m(&self) { m(); } }\n",
        &[("m", Unknown)],
    ),
    (
        "a call whose target no capture names",
        "rs",
        "fn f() { (pick())(); }\nfn pick() -> fn() { f }\n",
        &[("f", Unknown), ("pick", No)],
    ),
    ("unexpanded macro", "rs", "fn f() { println!(\"x\"); }\n", &[("f", Unknown)]),
    ("parse error", "rs", "fn f() { let x = ; }\nfn g() {}\n", &[("f", Unknown), ("g", No)]),
    (
        "a closure's call is the closure's, not the enclosing function's",
        "rs",
        "fn f() { let _later = || f(); }\n",
        &[("f", No)],
    ),
    (
        "a nested function's call is its own",
        "rs",
        "fn f() { fn inner() { f(); } }\n",
        &[("f", No), ("inner", No)],
    ),
    ("mutual recursion is not direct", "rs", "fn a() { b() }\nfn b() { a() }\n", &[("a", No), ("b", No)]),
    (
        "bodyless declaration",
        "rs",
        "trait T { fn m(&self); fn d(&self) {} }\n",
        &[("m", Unknown), ("d", No)],
    ),
    (
        "an import alias may name the function itself",
        "rs",
        "use crate::f as again;\nfn f() { again(); }\n",
        &[("f", Unknown)],
    ),
    (
        "an import alias of another name is not the function",
        "rs",
        "use crate::other as again;\nfn f() { again(); }\n",
        &[("f", No)],
    ),
    // A comment anywhere in a TS entity alias never hides the original's
    // last segment (review rows `R8-F1.01`/`.02`/`.S1`/`.S2`).
    (
        "a commented TS entity alias may name the function itself",
        "ts",
        "import A /* c */ = N.S;\nimport B = /* c */ N.T;\nimport /* c */ C = N.U;\nimport D = N. /* c */ V;\n\
         function S() { A(); }\nfunction T() { B(); }\nfunction U() { C(); }\nfunction V() { D(); }\n",
        &[("S", Unknown), ("T", Unknown), ("U", Unknown), ("V", Unknown)],
    ),
    (
        "a commented TS entity alias of another name is not the function",
        "ts",
        "import A /* c */ = N.R;\nfunction S() { A(); }\n",
        &[("S", No)],
    ),
    // `::f` is the crate root's `f` (2015) or the crate `f` (2018): the
    // file-local rules prove neither, so a rooted self-call is uncertain,
    // never `No` (rustc warns that the 2015 form recurses).
    ("rooted self-call", "rs", "fn f() { ::f(); }\n", &[("f", Unknown)]),
    ("wrapped rooted self-call", "rs", "fn f() { ((::f))(); }\n", &[("f", Unknown)]),
    (
        "rooted call beside a shadowing local is not the local",
        "rs",
        "fn f() {}\nfn g() { let f = || {}; ::f(); }\n",
        &[("f", No), ("g", No)],
    ),
    (
        "rooted self-call of a method",
        "rs",
        "struct W;\nimpl W { fn m(&self) { ::m(); } }\n",
        &[("m", Unknown)],
    ),
    (
        "rooted module path names another function",
        "rs",
        "mod m { pub fn f() {} }\nfn h() { ::m::f(); }\n",
        &[("f", No), ("h", No)],
    ),
    (
        "typescript direct self-call",
        "ts",
        "function fact(n: number): number { return n <= 1 ? 1 : n * fact(n - 1); }\nfunction leaf(): number { return 1; }\n",
        &[("fact", Yes), ("leaf", No)],
    ),
    (
        "typescript block `let` shadows the function before its declaration",
        "ts",
        "function f(): void { { f(); let f = 1; } }\n",
        &[("f", Unknown)],
    ),
    ("typescript `this` call", "ts", "class C { m(): void { this.m(); } }\n", &[("m", Unknown)]),
    (
        "constructing its own class",
        "ts",
        "class C { constructor() { new C(); } }\n",
        &[("constructor", Unknown)],
    ),
    (
        "constructor calling another function",
        "ts",
        "class D { constructor() { helper(); } }\nfunction helper(): void {}\n",
        &[("constructor", No), ("helper", No)],
    ),
    ("javascript direct self-call", "js", "function f(n) { return n && f(n - 1); }\n", &[("f", Yes)]),
    ("python is limited", "py", "def f(n):\n    return f(n - 1)\n\n\ndef g():\n    pass\n", &[("f", Unknown), ("g", Unknown)]),
    ("go is limited", "go", "package p\n\nfunc f() { f() }\n", &[("f", Unknown)]),
    (
        "a c# conditional self-call is limited",
        "cs",
        "class W { void F(W obj) { obj?.F(); } }\n",
        &[("F", Unknown)],
    ),
    (
        "php callable creation and a variable call are limited",
        "php",
        "<?php\nfunction f() { f(...); }\nfunction g($cb) { $cb(); }\n",
        &[("f", Unknown), ("g", Unknown)],
    ),
    ("a perl code-reference call is limited", "pl", "sub g { $cb->(); }\n", &[("g", Unknown)]),
    (
        "an inline module's call does not reach the parent's function",
        "rs",
        "fn f() {}\nmod m { pub fn g() { f(); } }\n",
        &[("f", No), ("g", No)],
    ),
    (
        "a nested function's own name wins over an enclosing `let`",
        "rs",
        "fn outer() { let g = || {}; fn g() { g(); } }\n",
        &[("g", Yes), ("outer", No)],
    ),
    (
        "a nested `const` cannot call an enclosing `let`",
        "rs",
        "fn f() { let x = || {}; const Y: () = x(); }\n",
        &[("f", No)],
    ),
    (
        "a nested `static` cannot call an enclosing `let`",
        "rs",
        "fn f() { let x = || {}; static Y: () = x(); }\n",
        &[("f", No)],
    ),
    (
        "a closure calling a captured `let` is the closure's (control)",
        "rs",
        "fn f() { let x = || {}; let _g = || x(); }\n",
        &[("f", No)],
    ),
    (
        "the `else` of an `if let` calls the function, not the binding",
        "rs",
        "fn f(x: Option<fn()>) { if let Some(f) = x { f(); } else { f(x); } }\n",
        &[("f", Yes)],
    ),
    (
        "the `else` of a `let` chain calls the function, not the binding",
        "rs",
        "fn f(x: Option<fn()>) { if let Some(f) = x && true { f(); } else { f(x); } }\n",
        &[("f", Yes)],
    ),
    (
        "a `while let` body calls the binding (control)",
        "rs",
        "fn f(x: Option<fn()>) { while let Some(f) = x { f(); } }\n",
        &[("f", Unknown)],
    ),
    (
        "a `for` body calls the binding (control)",
        "rs",
        "fn f(xs: Vec<fn()>) { for f in xs { f(); } }\n",
        &[("f", Unknown)],
    ),
    // A named function or generator expression's self-call binds the
    // expression's own name, which no record carries (the expression is
    // not extracted as a callable), so it proves nothing about the outer
    // declaration of that name: the outer function stays `No`.
    (
        "a typescript named function expression's self-call is its own",
        "ts",
        "function f() {}\nconst g = function f() { f(); };\nfunction h() { const k = function h() { h(); }; }\n",
        &[("f", No), ("h", No)],
    ),
    (
        "a javascript named function expression's self-call is its own",
        "js",
        "function f() {}\nconst g = function f() { f(); };\nfunction h() { const k = function h() { h(); }; }\n",
        &[("f", No), ("h", No)],
    ),
    (
        "a typescript named generator expression's self-call is its own",
        "ts",
        "function f() {}\nconst g = function* f() { f(); };\nfunction h() { const k = function* h() { h(); }; }\n",
        &[("f", No), ("h", No)],
    ),
    (
        "a javascript named generator expression's self-call is its own",
        "js",
        "function f() {}\nconst g = function* f() { f(); };\nfunction h() { const k = function* h() { h(); }; }\n",
        &[("f", No), ("h", No)],
    ),
    (
        "an outer self-call beside a same-named function expression (control)",
        "js",
        "function h() { const k = function h() { h(); }; h(); }\n",
        &[("h", Yes)],
    ),
    (
        "a typescript `with` body may intercept the self-call",
        "ts",
        "function f(obj: any) { with (obj) { f(obj); } }\n",
        &[("f", Unknown)],
    ),
    (
        "a javascript `with` body may intercept the self-call",
        "js",
        "function f(obj) { with (obj) { f(obj); } }\n",
        &[("f", Unknown)],
    ),
    (
        "a direct `eval` may shadow the function in its body",
        "js",
        "function f() { eval(\"1\"); f(); }\n",
        &[("f", Unknown)],
    ),
    (
        "a file-level `eval` adds no nearer binding (control)",
        "js",
        "eval(\"1\");\nfunction f(n) { return n && f(n - 1); }\n",
        &[("f", Yes)],
    ),
    (
        "a default argument calls the outer function (control)",
        "ts",
        "function f() {}\nfunction g(x = f()) { function f() {} }\n",
        &[("g", No)],
    ),
    // Transparent wrappers around the callee keep a proven self-call.
    ("a parenthesized self-call", "rs", "fn f(n: u32) { if n > 0 { ((f))(n - 1) } }\n", &[("f", Yes)]),
    ("a parenthesized turbofish self-call", "rs", "fn f<T>() { (f::<T>)(); }\n", &[("f", Yes)]),
    ("a parenthesized self-call", "ts", "function f(n: number) { if (n) ((f))(n - 1); }\n", &[("f", Yes)]),
    ("an `as` self-call", "ts", "function f() { (f as () => void)(); }\n", &[("f", Yes)]),
    ("a `satisfies` self-call", "ts", "function f() { (f satisfies () => void)(); }\n", &[("f", Yes)]),
    ("a type-assertion self-call", "ts", "function f() { (<() => void>f)(); }\n", &[("f", Yes)]),
    ("a non-null self-call", "ts", "function f() { f!(); }\n", &[("f", Yes)]),
    ("an instantiation-expression self-call", "ts", "function f<T>() { (f<number>)(); }\n", &[("f", Yes)]),
    ("a parenthesized self-call", "js", "function f(n) { if (n) ((f))(n - 1); }\n", &[("f", Yes)]),
    // A wrapped call to another established declaration is as direct as an
    // unwrapped one: coverage reads the callee through the same wrappers as
    // invocation, so the caller is `No`, never `Unknown`.
    ("a direct call to another function (control)", "rs", "fn f() {}\nfn g() { f(); }\n", &[("g", No)]),
    ("a parenthesized call to another function", "rs", "fn f() {}\nfn g() { (f)(); ((f))(); }\n", &[("g", No)]),
    ("a parenthesized turbofish call to another function", "rs", "fn f<T>() {}\nfn g() { (f::<u8>)(); }\n", &[("g", No)]),
    (
        "a parenthesized type-path call to the one inherent function",
        "rs",
        "struct S;\nimpl S { fn f() {} }\nfn g() { (S::f)(); }\n",
        &[("g", No)],
    ),
    (
        "a parenthesized generic type-path call to the one inherent function",
        "rs",
        "struct S;\nimpl S { fn f<T>() {} }\nfn g() { (S::f::<u8>)(); }\n",
        &[("g", No)],
    ),
    ("a direct call to another function (control)", "ts", "function f() {}\nfunction g() { f(); }\n", &[("g", No)]),
    ("a parenthesized call to another function", "ts", "function f() {}\nfunction g() { (f)(); ((f))(); }\n", &[("g", No)]),
    ("an `as` call to another function", "ts", "function f() {}\nfunction g() { (f as (() => void))(); }\n", &[("g", No)]),
    (
        "a doubled `as` call to another function",
        "ts",
        "function f() {}\nfunction g() { (f as any as () => void)(); }\n",
        &[("g", No)],
    ),
    (
        "a `satisfies` call to another function",
        "ts",
        "function f() {}\nfunction g() { (f satisfies (() => void))(); }\n",
        &[("g", No)],
    ),
    (
        "a type-assertion call to another function",
        "ts",
        "function f() {}\nfunction g() { (<(() => void)>f)(); }\n",
        &[("g", No)],
    ),
    ("a non-null call to another function", "ts", "function f() {}\nfunction g() { f!(); (f!)(); }\n", &[("g", No)]),
    ("an optional call to another function", "ts", "function f() {}\nfunction g() { f?.(); (f)?.(); }\n", &[("g", No)]),
    (
        "a parenthesized tag of another function",
        "ts",
        "function f(_: TemplateStringsArray) {}\nfunction g() { (f)`x`; }\n",
        &[("g", No)],
    ),
    // A TypeScript instantiation expression (`f<number>` apart from its
    // call) is as direct as the instantiated name, at any wrapper depth.
    ("a direct generic call to another function (control)", "ts", "function f<T>() {}\nfunction g() { f<number>(); }\n", &[("g", No)]),
    ("an instantiation-expression call to another function", "ts", "function f<T>() {}\nfunction g() { (f<number>)(); }\n", &[("g", No)]),
    ("a doubly parenthesized instantiation call", "ts", "function f<T>() {}\nfunction g() { ((f<number>))(); }\n", &[("g", No)]),
    ("a non-null instantiation call", "ts", "function f<T>() {}\nfunction g() { (f<number>)!(); }\n", &[("g", No)]),
    ("an `as` instantiation call", "ts", "function f<T>() {}\nfunction g() { (f<number> as () => void)(); }\n", &[("g", No)]),
    (
        "a `satisfies` instantiation call",
        "ts",
        "function f<T>() {}\nfunction g() { (f<number> satisfies () => void)(); }\n",
        &[("g", No)],
    ),
    ("a type-assertion instantiation call", "ts", "function f<T>() {}\nfunction g() { (<() => void>f<number>)(); }\n", &[("g", No)]),
    ("constructing an instantiated local class", "ts", "class C<T> {}\nfunction g() { new (C<number>)(); }\n", &[("g", No)]),
    (
        "an instantiated member call stays a receiver call",
        "ts",
        "class W { m<T>() {} }\nfunction g(obj: W) { (obj.m<number>)(); }\n",
        &[("g", Unknown)],
    ),
    ("constructing a local class (control)", "ts", "class F {}\nfunction g() { new F(); }\n", &[("g", No)]),
    ("constructing a parenthesized local class", "ts", "class F {}\nfunction g() { new (F)(); new ((F))(); }\n", &[("g", No)]),
    ("constructing an `as` local class", "ts", "class F {}\nfunction g() { new (F as typeof F)(); }\n", &[("g", No)]),
    ("a direct call to another function (control)", "js", "function f() {}\nfunction g() { f(); }\n", &[("g", No)]),
    ("a parenthesized call to another function", "js", "function f() {}\nfunction g() { (f)(); ((f))(); }\n", &[("g", No)]),
    ("an optional call to another function", "js", "function f() {}\nfunction g() { f?.(); (f)?.(); }\n", &[("g", No)]),
    ("a parenthesized tag of another function", "js", "function f() {}\nfunction g() { (f)`x`; }\n", &[("g", No)]),
    ("constructing a local class (control)", "js", "class F {}\nfunction g() { new F(); }\n", &[("g", No)]),
    ("constructing a parenthesized local class", "js", "class F {}\nfunction g() { new (F)(); new ((F))(); }\n", &[("g", No)]),
    // A parenthesized `eval` is still a direct `eval` (the parentheses
    // pass the reference through), so it may shadow the function; a
    // sequence makes it indirect.
    (
        "a parenthesized direct `eval` may shadow the function",
        "js",
        "function f() { (eval)(\"1\"); f(); }\n",
        &[("f", Unknown)],
    ),
    (
        "an `as` direct `eval` may shadow the function",
        "ts",
        "function f() { (eval as any)(\"1\"); f(); }\n",
        &[("f", Unknown)],
    ),
    ("an indirect `eval` adds no binding (control)", "js", "function f() { (0, eval)(\"1\"); f(); }\n", &[("f", Yes)]),
    // Only a direct call of `eval` can bind: an optional call is an
    // indirect eval and a tagged template evaluates nothing.
    ("an optional `eval` adds no binding", "js", "function f() { eval?.(\"1\"); f(); }\n", &[("f", Yes)]),
    ("a parenthesized optional `eval` adds no binding", "ts", "function f() { (eval)?.(\"1\"); f(); }\n", &[("f", Yes)]),
    ("a tagged `eval` adds no binding", "ts", "function f() { eval`1`; f(); }\n", &[("f", Yes)]),
    ("a parenthesized tagged `eval` adds no binding", "js", "function f() { (eval)`1`; f(); }\n", &[("f", Yes)]),
    (
        "a tagged `eval` limits nothing for another function",
        "js",
        "function f() {}\nfunction g() { eval`1`; f(); }\n",
        &[("f", No), ("g", No)],
    ),
    (
        "an optional `eval` may still call anything",
        "ts",
        "function f() {}\nfunction g() { eval?.(\"1\"); f(); }\n",
        &[("f", No), ("g", Unknown)],
    ),
    // The other callee shapes stay uncertain for another declaration too.
    ("a member call of another function", "ts", "const o = { f() {} };\nfunction g() { o.f(); }\n", &[("g", Unknown)]),
    ("a variable holding another function", "ts", "function f() {}\nfunction g() { const h = f; h(); }\n", &[("g", Unknown)]),
    ("a sequence callee of another function", "js", "function f() {}\nfunction g() { (0, f)(); }\n", &[("g", Unknown)]),
    (
        "a conditional callee of another function",
        "ts",
        "function f() {}\nfunction g(c: boolean) { (c ? f : f)(); }\n",
        &[("g", Unknown)],
    ),
    ("a dereferenced callee of another function", "rs", "fn f() {}\nfn g() { let p = &f; (*p)(); }\n", &[("g", Unknown)]),
    ("a parenthesized local variable", "rs", "fn f() {}\nfn g() { let h = f; (h)(); }\n", &[("g", Unknown)]),
    // Not wrappers: the callee is another value, so nothing is proven.
    ("a self-reference as an argument is no call", "rs", "fn f() { g((f)); }\nfn g(_: fn()) {}\n", &[("f", No)]),
    ("a dereferenced callee is no self-call", "rs", "fn f() { let p = &f; (*p)(); }\n", &[("f", Unknown)]),
    ("a sequence callee is no self-call", "js", "function f() { (0, f)(); }\n", &[("f", Unknown)]),
    ("a conditional callee is no self-call", "ts", "function f(c: boolean) { (c ? f : f)(); }\n", &[("f", Unknown)]),
    // A Rust pattern name binds or reads a constant by lookup, never by
    // spelling (`symbols_occurrences::PATTERN_IDENTITY_MATRIX`): a
    // capitalized pattern named like the function binds a new local.
    ("an `if let` name named like the function", "rs", "fn F(x: fn()) { if let F = x { F(); } }\n", &[("F", Unknown)]),
    (
        "a `while let` name named like the function",
        "rs",
        "fn F(x: fn()) { while let F = x { F(); break; } }\n",
        &[("F", Unknown)],
    ),
    ("a match arm name named like the function", "rs", "fn F(x: fn()) { match x { F => F() } }\n", &[("F", Unknown)]),
    ("a `mut` match binding named like the function", "rs", "fn F(x: fn()) { match x { mut F => F() } }\n", &[("F", Unknown)]),
    ("a `ref` match binding named like the function", "rs", "fn F(x: fn()) { match x { ref F => F() } }\n", &[("F", Unknown)]),
    (
        "a lowercase constant read by patterns is no call",
        "rs",
        "const f: u8 = 1;\nfn g(x: u8) { match x { f => { f; } _ => {} } if let f = x { f; } while let f = x { f; break; } }\n",
        &[("g", No)],
    ),
    (
        "a captured tuple-struct local, then a proven self-call",
        "rs",
        "fn F(x: Option<fn()>) { if let Some(F) = x { F(); } F(None); }\n",
        &[("F", Yes)],
    ),
    (
        "irrefutable `let`, parameter and `for` names are locals",
        "rs",
        "fn F(x: fn()) { let F = x; F(); }\nfn G(G: fn()) { G(); }\nfn H(xs: Vec<fn()>) { for H in xs { H(); } }\n",
        &[("F", Unknown), ("G", Unknown), ("H", Unknown)],
    ),
    (
        "a struct shorthand named like the function",
        "rs",
        "struct S { F: fn() }\nfn F(s: S) { if let S { F } = s { F(); } }\n",
        &[("F", Unknown)],
    ),
    // Calling a value (a `const`/`static` holding a function pointer) is
    // indirect even when the value is established: its initializer is
    // never followed, whether it names the caller or not.
    ("value `Self::C()`", "rs", "struct S;\nimpl S { const C: fn() = Self::f; fn f() { Self::C(); } }\n", &[("f", Unknown)]),
    ("value `S::C()`", "rs", "struct S;\nimpl S { const C: fn() = S::f; fn f() { S::C(); } }\n", &[("f", Unknown)]),
    (
        "wrapped value `(Self::C)()`",
        "rs",
        "struct S;\nimpl S { const C: fn() = Self::f; fn f() { (Self::C)(); ((Self::C))(); } }\n",
        &[("f", Unknown)],
    ),
    ("wrapped value `(S::C)()`", "rs", "struct S;\nimpl S { const C: fn() = Self::f; fn f() { (S::C)(); } }\n", &[("f", Unknown)]),
    (
        "value in a generic impl",
        "rs",
        "struct S<T>(T);\nimpl<T> S<T> { const C: fn() = Self::f; fn f() { Self::C(); } }\n",
        &[("f", Unknown)],
    ),
    (
        "value candidate behind `S::<u8>::C()`",
        "rs",
        "struct S<T>(T);\nimpl<T> S<T> { const C: fn() = Self::f; fn f() { S::<u8>::C(); } }\n",
        &[("f", Unknown)],
    ),
    ("enum value `Self::C()`", "rs", "enum E { A }\nimpl E { const C: fn() = Self::f; fn f() { Self::C(); } }\n", &[("f", Unknown)]),
    (
        "trait value candidates behind `Self::C()`",
        "rs",
        "trait T { const C: fn(); fn f() { Self::C(); } }\nstruct S;\nimpl T for S { const C: fn() = Self::f; }\n",
        &[("f", Unknown)],
    ),
    (
        "trait value candidates behind `S::C()`",
        "rs",
        "trait T { const C: fn(); fn f() { S::C(); } }\nstruct S;\nimpl T for S { const C: fn() = Self::f; }\n",
        &[("f", Unknown)],
    ),
    (
        "a value with an unrelated initializer",
        "rs",
        "fn h() {}\nstruct S;\nimpl S { const C: fn() = h; fn f() { Self::C(); } }\n",
        &[("f", Unknown), ("h", No)],
    ),
    ("top-level `const` value", "rs", "const C: fn() = f;\nfn f() { C(); }\n", &[("f", Unknown)]),
    ("top-level `static` value", "rs", "static C: fn() = f;\nfn f() { C(); }\n", &[("f", Unknown)]),
    (
        "a member function and a member self-call",
        "rs",
        "struct S;\nimpl S { fn h() {} fn f() { Self::h(); } fn g() { Self::g(); } }\n",
        &[("f", No), ("g", Yes), ("h", No)],
    ),
    ("a variant constructor call", "rs", "enum E { V(u8) }\nfn f() { let _ = E::V(1); }\n", &[("f", No)]),
    (
        "reading a value without calling it",
        "rs",
        "struct S;\nimpl S { const C: fn() = Self::f; fn f() { let _p = Self::C; } }\n",
        &[("f", No)],
    ),
    (
        "a refutable name a glob may supply stays uncertain",
        "rs",
        "mod m { pub fn helper() {} }\nuse m::*;\nfn F(x: fn()) { match x { F => F() } }\n",
        &[("F", Unknown)],
    ),
];

#[test]
fn recursion_matrix_holds_on_complete_and_narrowed_queries() {
    for &(label, extension, source, expected) in RECURSION_MATRIX {
        let (_file, tree_file) = parse(source, extension);
        let full = tree_file.symbol_index_v2().expect("complete index");
        let narrowed = tree_file
            .query(
                &SymbolQuery::new()
                    .kinds([SymbolKindV2::Function, SymbolKindV2::Method, SymbolKindV2::Constructor])
                    .capabilities([Capability::Semantics]),
            )
            .expect("narrowed query")
            .index;
        for &(name, value) in expected {
            assert_eq!(record(&full, name).semantics.is_recursive, Some(value), "{label}: `{name}`");
            assert_eq!(
                record(&narrowed, name).semantics.is_recursive,
                Some(value),
                "{label}: `{name}` in a functions-only query"
            );
        }
    }
}

#[test]
fn is_recursive_is_absent_without_semantics_and_only_on_callables() {
    let source = "fn countdown(n: u32) -> u32 { if n == 0 { 0 } else { countdown(n - 1) } }\nstruct Holder;\n";
    let (_file, tree_file) = parse(source, "rs");
    let full = tree_file.symbol_index_v2().expect("complete index");
    assert_eq!(record(&full, "n").semantics.is_recursive, None, "a parameter has no semantics");
    assert_eq!(record(&full, "Holder").semantics.is_recursive, None, "a type has no semantics");

    for capabilities in [&[Capability::References][..], &[Capability::Occurrences]] {
        let without = tree_file
            .query(&SymbolQuery::new().capabilities(capabilities.iter().copied()))
            .expect("query");
        assert!(
            without.index.symbols.iter().all(|record| record.semantics.is_recursive.is_none()),
            "{capabilities:?}: no semantics requested, no value"
        );
    }
}

const SUMMARY_SOURCE: &str = "\
use std::mem::swap as exchange;
fn a() {}
fn b() {}
fn main() { b(); a(); b(); exchange(&mut 1, &mut 2); }
fn shadow() { let a = 1; let _ = a; }
struct A;
struct B;
impl A { fn run(&self) {} }
impl B { fn run(&self) {} }
fn go(x: A) { x.run(); }
fn closure_owner() { let _c = || a(); }
const TOP: () = b();
";

/// `references`/`dependencies`/`referenced_by` hold only established
/// targets and bound imports, in first-occurrence order without repeats;
/// shadowed names, receiver calls and ownerless occurrences make no edge,
/// while every occurrence stays in the evidence.
#[test]
fn summaries_hold_only_established_targets_in_first_occurrence_order() {
    let index = complete(SUMMARY_SOURCE, "rs");
    let (a, b, main) = (record(&index, "a"), record(&index, "b"), record(&index, "main"));

    assert_eq!(names(&main.relations.references), [("b", true), ("a", true), ("exchange", false)]);
    assert_eq!(main.relations.references[0].id.as_ref(), Some(&b.id));
    assert_eq!(main.relations.references[1].id.as_ref(), Some(&a.id));
    assert_eq!(names(&main.relations.dependencies), [("exchange", false)], "only the bound import");
    let import = &main.relations.dependencies[0];
    let recorded = index.imports.iter().find(|import| import.name == "exchange").expect("import record");
    assert_eq!(import.qualified_name, recorded.symbol.qualified_name, "the import record's original name");

    fn owners(record: &SymbolRecord) -> Vec<&str> {
        record.relations.referenced_by.iter().map(|owner| owner.name.as_str()).collect()
    }
    assert_eq!(owners(a), ["main"], "not `shadow` (its own `a`), not the closure");
    assert_eq!(owners(b), ["main"], "not the ownerless `TOP` initializer");
    assert_eq!(main.relations.referenced_by, [], "nothing calls `main`");

    let shadow = record(&index, "shadow");
    assert_eq!(shadow.relations.references.len(), 1, "{:#?}", shadow.relations.references);
    assert_ne!(shadow.relations.references[0].id.as_ref(), Some(&a.id), "the local `a`, not the function");

    let go = record(&index, "go");
    assert_eq!(
        names(&go.relations.references),
        [("A", true), ("x", true)],
        "the parameter's type and the receiver; `x.run()` binds neither `run`"
    );
    for run in index.symbols.iter().filter(|record| record.identity.name == "run") {
        assert!(run.relations.referenced_by.is_empty(), "a receiver call makes no reverse edge");
    }
    assert!(record(&index, "closure_owner").relations.references.is_empty(), "the closure owns `a()`");

    let kept = |name: &str, confidence: Confidence| {
        index.occurrences.iter().any(|occurrence| occurrence.target.name == name && occurrence.confidence == confidence)
    };
    assert!(kept("run", Confidence::NameOnly), "the uncertain receiver call stays an occurrence");
    let ownerless_b = index
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.target.name == "b" && occurrence.owner.is_none())
        .count();
    assert_eq!(ownerless_b, 1, "the `TOP` initializer call stays an occurrence");
    let a_calls = index.occurrences.iter().filter(|occurrence| occurrence.target.id.as_ref() == Some(&a.id)).count();
    assert_eq!(a_calls, 2, "main's call and the closure's call are both kept");
}

/// An aliased TypeScript import is a dependency of every owner that calls
/// it, once per owner, and is never attached to another owner.
#[test]
fn bound_imports_are_dependencies_of_their_owners_only() {
    let index = complete(
        "import { readFile as rf } from \"fs\";\nimport { join } from \"path\";\n\
         function g(): void { rf(\"x\"); rf(\"y\"); join(); }\nfunction h(): void {}\n",
        "ts",
    );
    let g = record(&index, "g");
    let dependencies: Vec<&str> = g.relations.dependencies.iter().map(|dependency| dependency.name.as_str()).collect();
    assert_eq!(dependencies.len(), 2, "{:#?}", g.relations.dependencies);
    assert_eq!(g.relations.references, g.relations.dependencies, "only imports are referenced");
    for dependency in &g.relations.dependencies {
        assert!(dependency.id.is_none(), "an import has no in-file ID");
        assert!(
            index.imports.iter().any(|import| import.name == dependency.name
                && import.symbol.qualified_name == dependency.qualified_name),
            "{dependency:?} is an import record"
        );
    }
    assert!(record(&index, "h").relations.dependencies.is_empty());
}

/// Calling an established value keeps its summary edge to the value and
/// never adds one to the function its initializer names.
#[test]
fn value_calls_reference_the_value_not_its_initializer() {
    let index = complete(
        "struct S;\nimpl S { const C: fn() = Self::f; fn f() { Self::C(); (S::C)(); } }\n",
        "rs",
    );
    let (value, caller, owner) = (record(&index, "C"), record(&index, "f"), record(&index, "S"));
    let ids = |refs: &[SymbolRef]| refs.iter().map(|reference| reference.id.clone()).collect::<Vec<_>>();
    // `S` is the type usage in `(S::C)()`.
    assert_eq!(ids(&caller.relations.references), [Some(value.id.clone()), Some(owner.id.clone())]);
    assert_eq!(ids(&value.relations.referenced_by), [Some(caller.id.clone())]);
    assert!(caller.relations.referenced_by.is_empty(), "`f` is named only outside any callable");
}

/// A rooted Rust name (`::f`) never reaches ordinary lexical lookup: it
/// keeps its written root, binds neither the shadowing `let f` nor the
/// root function, and so makes no summary edge to either.
#[test]
fn rooted_rust_names_never_summarize_a_shadowing_local() {
    let index = complete("fn f() {}\nfn g() { let f = || {}; ::f(); (::f)(); }\nfn h() { ::m::f(); }\n", "rs");
    let (function, caller, other) = (record(&index, "f"), record(&index, "g"), record(&index, "h"));
    let local = index
        .symbols
        .iter()
        .find(|record| record.identity.name == "f" && record.kind == SymbolKindV2::Variable)
        .expect("the `let f` record");
    assert!(caller.relations.references.is_empty(), "{:#?}", caller.relations.references);
    assert!(other.relations.references.is_empty(), "{:#?}", other.relations.references);
    assert!(local.relations.referenced_by.is_empty(), "the local is never the rooted target");
    assert!(function.relations.referenced_by.is_empty(), "the root binding is unproved");
    let rooted: Vec<_> = index
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.target.qualified_name.as_deref().is_some_and(|path| path.starts_with("::")))
        .map(|occurrence| {
            (occurrence.target.name.as_str(), occurrence.target.qualified_name.as_deref(), occurrence.kind)
        })
        .collect();
    assert_eq!(
        rooted,
        [
            ("f", Some("::f"), OccurrenceKind::Call),
            ("f", Some("::f"), OccurrenceKind::Call),
            ("m", Some("::m"), OccurrenceKind::Usage),
            ("f", Some("::m::f"), OccurrenceKind::Call),
        ]
    );
    for occurrence in index.occurrences.iter().filter(|occurrence| occurrence.target.qualified_name.is_some()) {
        assert_ne!(occurrence.confidence, Confidence::Scoped, "{occurrence:#?}");
        assert!(
            occurrence.candidates.iter().all(|candidate| candidate.id.as_ref() != Some(&local.id)),
            "the local is never a candidate: {occurrence:#?}"
        );
    }
}

/// A language without resolver rules establishes nothing, so it has no
/// summary edges; its occurrences remain, none of them `Scoped`.
#[test]
fn limited_languages_bind_no_edges_but_keep_their_occurrences() {
    let index = complete(
        "from os import path\nfrom sys import argv\n\n\ndef helper():\n    pass\n\n\ndef run():\n    path.join(\"a\")\n    helper()\n    return argv\n",
        "py",
    );
    for record in &index.symbols {
        assert!(record.relations.references.is_empty(), "{}: references", record.identity.name);
        assert!(record.relations.dependencies.is_empty(), "{}: dependencies", record.identity.name);
        assert!(record.relations.referenced_by.is_empty(), "{}: referenced_by", record.identity.name);
    }
    let written: Vec<&str> = index.occurrences.iter().map(|occurrence| occurrence.target.name.as_str()).collect();
    for name in ["path", "helper", "argv"] {
        assert!(written.contains(&name), "`{name}` is still an occurrence: {written:?}");
    }
    assert!(index.occurrences.iter().all(|occurrence| occurrence.confidence != Confidence::Scoped));
}

crate::language_cases::language_tests!(occurrences_describe_the_whole_file_whatever_kinds_are_selected => check_occurrences_describe_the_whole_file_whatever_kinds_are_selected);

/// For every primary fixture: a kind-narrowed query's occurrences and
/// coverage equal the complete index's (whole file, possibly naming
/// unselected declarations), its records equal the complete index's records
/// of those kinds, `References` alone keeps the edges but no evidence, and
/// one analysis runs one references query and one resolution.
fn check_occurrences_describe_the_whole_file_whatever_kinds_are_selected(language: ProgrammingLanguage) {
    let mut named_unselected = 0;
    {
        let path = primary_fixture(language).unwrap_or_else(|| panic!("{language:?}: no primary fixture"));
        let label = display_name(&path);
        let tree_file = TreeFile::new(&path).expect("parse fixture");
        let (full, work) = counted(|| tree_file.symbol_index_v2().expect("complete index"));
        assert_eq!((work.reference_captures, work.occurrence_resolutions), (1, 1), "{label}: complete index");
        assert!(!full.reference_coverage.is_empty(), "{label}: coverage");

        let kinds = [SymbolKindV2::Function, SymbolKindV2::Method];
        let query = SymbolQuery::new().kinds(kinds).capabilities([Capability::Semantics]);
        let (narrow, work) = counted(|| tree_file.query(&query).expect("narrow query").index);
        assert_eq!((work.reference_captures, work.occurrence_resolutions), (1, 1), "{label}: narrow query");
        assert_eq!(narrow.occurrences, full.occurrences, "{label}: occurrences");
        assert_eq!(narrow.reference_coverage, full.reference_coverage, "{label}: coverage");
        let wide = tree_file
            .query(&SymbolQuery::new().capabilities([Capability::Semantics]))
            .expect("wide query")
            .index;
        let expected: Vec<&SymbolRecord> = wide.symbols.iter().filter(|record| kinds.contains(&record.kind)).collect();
        let narrow_records: Vec<&SymbolRecord> = narrow.symbols.iter().collect();
        assert_eq!(
            serde_json::to_value(&narrow_records).expect("serialize"),
            serde_json::to_value(&expected).expect("serialize"),
            "{label}: records"
        );
        let selected: Vec<_> = narrow.symbols.iter().map(|record| &record.id).collect();
        named_unselected += narrow
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.target.id.as_ref().is_some_and(|id| !selected.contains(&id)))
            .count();

        let references = tree_file
            .query(&SymbolQuery::new().capabilities([Capability::References]))
            .expect("references query")
            .index;
        assert!(references.occurrences.is_empty() && references.reference_coverage.is_empty(), "{label}");
        for (with_evidence, without) in full.symbols.iter().zip(&references.symbols) {
            assert_eq!(
                serde_json::to_value(&with_evidence.relations).expect("serialize"),
                serde_json::to_value(&without.relations).expect("serialize"),
                "{label}: {}",
                with_evidence.identity.name
            );
        }

        for capabilities in [&[Capability::Declarations][..], &[Capability::Containment, Capability::Flags]] {
            let (index, work) = counted(|| {
                tree_file
                    .query(&SymbolQuery::new().capabilities(capabilities.iter().copied()))
                    .expect("declaration query")
                    .index
            });
            assert_eq!(
                (work.reference_captures, work.occurrence_resolutions),
                (0, 0),
                "{label}: {capabilities:?} skips reference work"
            );
            assert!(index.occurrences.is_empty() && index.reference_coverage.is_empty(), "{label}");
        }
    }
    if language == ProgrammingLanguage::Rust { assert!(named_unselected > 0, "some occurrence names a declaration the query did not select"); }
}

crate::language_cases::language_tests!(diagnostics_are_unchanged_by_the_shared_references_run => check_diagnostics_are_unchanged_by_the_shared_references_run);

/// Sharing the references run with the occurrences changes no diagnostic:
/// the complete index's diagnostics equal those of the standalone
/// diagnostics path for every fixture.
fn check_diagnostics_are_unchanged_by_the_shared_references_run(language: ProgrammingLanguage) {
    for path in crate::symbols_fixtures::fixtures_for(language, false) {
        let tree_file = TreeFile::new(&path).expect("parse fixture");
        let index = tree_file.symbol_index_v2().expect("complete index");
        assert_eq!(
            serde_json::to_value(&index.diagnostics).expect("serialize"),
            serde_json::to_value(tree_file.try_diagnostics().expect("diagnostics")).expect("serialize"),
            "{}",
            display_name(&path)
        );
    }
}

/// Calls and usages both feed the summaries, but only calls decide
/// recursion: a callable whose only self-reference is a usage is `No`.
#[test]
fn usages_feed_summaries_but_not_recursion() {
    let index = complete("fn f() { let _keep = f; }\nfn g() { f(); }\n", "rs");
    let f = record(&index, "f");
    assert_eq!(names(&f.relations.references), [("f", true)], "the self-value reference is an edge");
    assert_eq!(f.semantics.is_recursive, Some(TriState::No));
    let usage = index
        .occurrences
        .iter()
        .find(|occurrence| occurrence.target.id.as_ref() == Some(&f.id) && occurrence.owner.as_ref().and_then(|owner| owner.id.as_ref()) == Some(&f.id))
        .expect("the self-value occurrence");
    assert_eq!(usage.kind, OccurrenceKind::Usage);
    let owners: Vec<&str> = f.relations.referenced_by.iter().map(|owner| owner.name.as_str()).collect();
    assert_eq!(owners, ["f", "g"]);
}

/// A name that crosses a boundary establishing no binding (an inline
/// module, a nested item, an `if let` past its consequence, a `with` body)
/// makes no edge in either direction; the binding that is in view does.
/// A declaration inside a named function or generator expression that calls
/// the expression's name calls the expression, never the outer declaration
/// of that name: no edge to it, and it has no caller there.
#[test]
fn expression_local_names_make_no_summary_edges() {
    for extension in ["ts", "js"] {
        for star in ["", "*"] {
            let source = format!(
                "function f() {{}}\nconst g = function{star} f() {{ function inner() {{ f(); }} }};\nfunction later() {{ f(); }}\n"
            );
            let file = complete(&source, extension);
            let outer = file
                .symbols
                .iter()
                .find(|record| record.identity.name == "f" && record.kind == SymbolKindV2::Function)
                .expect("outer f");
            let inner = record(&file, "inner");
            assert!(
                inner.relations.references.iter().all(|reference| reference.id.as_ref() != Some(&outer.id)),
                "{extension}{star}: {:#?}",
                inner.relations
            );
            assert_eq!(
                outer.relations.referenced_by.iter().map(|owner| owner.name.as_str()).collect::<Vec<_>>(),
                ["later"],
                "{extension}{star}"
            );
        }
    }
}

/// Every `eval` call form of `symbols_occurrences::EVAL_CALL_FORMS`, in a
/// nested function: only a direct `eval` cuts the function's edges to its
/// own declaration and to `Base`; every other form keeps both, in both
/// directions.
#[test]
fn only_a_direct_eval_cuts_summary_edges() {
    for &(label, extensions, statement, mode) in EVAL_CALL_FORMS {
        for &extension in extensions {
            let label = format!("{extension} {label} eval");
            let index = complete(&eval_source(statement), extension);
            let f = record(&index, "f");
            let base = record(&index, "Base");
            let references_to = |id| f.relations.references.iter().any(|reference| reference.id.as_ref() == Some(id));
            let callers = |target: &SymbolRecord| -> Vec<String> {
                target.relations.referenced_by.iter().map(|owner| owner.name.clone()).collect()
            };
            if mode == EvalMode::Direct {
                assert!(!references_to(&f.id) && !references_to(&base.id), "{label}: {:#?}", f.relations);
                assert!(callers(f).is_empty() && callers(base).is_empty(), "{label}");
            } else {
                assert!(references_to(&f.id) && references_to(&base.id), "{label}: {:#?}", f.relations);
                assert_eq!(callers(f), ["f"], "{label}");
                assert_eq!(callers(base), ["f"], "{label}");
            }
        }
    }
}

#[test]
fn boundary_crossing_names_make_no_summary_edges() {
    let module = complete("fn f() {}\nmod m { pub fn g() { f(); } }\n", "rs");
    assert!(record(&module, "g").relations.references.iter().all(|reference| reference.id.is_none()));
    assert!(record(&module, "f").relations.referenced_by.is_empty(), "no caller across the module");

    let nested = complete("fn f() {\n    let x = || {};\n    fn g() { x(); }\n    let _c = || x();\n}\n", "rs");
    assert!(record(&nested, "g").relations.references.is_empty(), "the nested item cannot capture `x`");
    assert!(record(&nested, "x").relations.referenced_by.is_empty(), "only the ownerless closure uses `x`");

    let conditional = complete(
        "fn f() {}\nfn g(x: Option<fn()>) { if let Some(f) = x { f(); } else { f(); } }\n",
        "rs",
    );
    let outer = conditional
        .symbols
        .iter()
        .find(|record| record.identity.name == "f" && record.kind == SymbolKindV2::Function)
        .expect("outer f");
    let g = record(&conditional, "g");
    assert!(
        g.relations.references.iter().any(|reference| reference.id.as_ref() == Some(&outer.id)),
        "the `else` call is the outer function's: {:#?}",
        g.relations.references
    );
    assert_eq!(outer.relations.referenced_by.iter().map(|owner| owner.name.as_str()).collect::<Vec<_>>(), ["g"]);

    for extension in ["ts", "js"] {
        let with = complete("function f(obj) { with (obj) { f(obj); } }\n", extension);
        let f = record(&with, "f");
        assert!(f.relations.referenced_by.is_empty(), "{extension}: no proven caller inside `with`");
        assert!(
            f.relations.references.iter().all(|reference| reference.id.as_ref() != Some(&f.id)),
            "{extension}: no proven self-reference"
        );
    }
}


#[test]
fn type_only_imports_restore_summaries_in_complete_narrowed_and_references_queries() {
    use crate::symbols_occurrences::{TYPE_ONLY_IMPORT_MATRIX, check_type_only_import_index, type_only_import_source};
    for &(_, import, name, mixed) in TYPE_ONLY_IMPORT_MATRIX {
        let source = type_only_import_source(import, name, mixed);
        let (_file, tree_file) = parse(&source, "ts");
        for query in [
            SymbolQuery::complete_index(),
            SymbolQuery::new().kinds([SymbolKindV2::Function]).capabilities([Capability::Semantics]),
            SymbolQuery::new().kinds([SymbolKindV2::TypeAlias]).capabilities([Capability::Occurrences]),
        ] {
            let result = tree_file.query(&query).expect("query");
            check_type_only_import_index(&result.index, name, mixed, true);
        }
        let refs = tree_file.query(&SymbolQuery::new().kinds([SymbolKindV2::Function]).capabilities([Capability::References])).expect("references");
        let function = record(&refs.index, name);
        assert_eq!(names(&function.relations.references), if mixed { vec![(name, true), ("g", false)] } else { vec![(name, true)] });
        assert_eq!(names(&function.relations.referenced_by), [(name, true)]);
        assert_eq!(function.relations.dependencies.len(), usize::from(mixed));
        assert!(refs.index.occurrences.is_empty(), "References consumes shared facts without returning occurrences");
    }
}
