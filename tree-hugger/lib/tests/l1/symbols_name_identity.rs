//! Written-name equivalence through every persistent-cache serving stage.
use tempfile::TempDir;
use tree_hugger::{Capability, Confidence, FileSymbolIndex, SourceFile, SymbolKindV2, SymbolQuery, TriState, work_counters};
use tree_hugger::cache::{CacheLookup, CacheMiss, PersistentCache, SymbolCache};
#[derive(Clone, Copy)]
enum Want {
    Id(&'static str),
    Local,
    Import,
    Candidates(usize),
    Unresolved,
}
struct Row {
    group: &'static str,
    label: &'static str,
    ext: &'static str,
    source: String,
    want: Want,
    recursion: TriState,
}
fn rows() -> Vec<Row> {
    use Want::*;
    use TriState::*;
    let mut rows = Vec::new();
    let mut add = |group, label, ext, source: &str, want, recursion| {
        rows.push(Row { group, label, ext, source: source.into(), want, recursion });
    };
    for (label, source, want, recursion) in [
        (".01 raw definition", "fn r#f(){/*read*/f();}", Id("r#f"), Yes),
        (".01 raw call", "fn f(){/*read*/r#f();}", Id("f"), Yes),
        (".17 raw control", "fn r#f(){/*read*/r#f();}", Id("r#f"), Yes),
        (".08 NFC", "fn café(){/*read*/café();}", Id("café"), Yes),
        (".06 method", "struct C; impl C {fn r#f(){Self::/*read*/f();}}", Id("r#f"), Yes),
        (".06 owner", "struct r#C; impl C {fn f(){Self::/*read*/f();}}", Id("f"), Yes),
    ] {
        add("rust_call", label,"rs", source, want, recursion);
    }
    for (label, source, want, recursion) in [
        (".02 local", "fn f(x:fn()){let r#f=x; /*read*/f();}", Id("r#f"), Unknown),
        (".02 parameter", "fn f(r#f:fn()){/*read*/f();}", Id("r#f"), Unknown),
        (".03 match", "fn f(x:Option<fn()>){match x {Some(r#f)=>{/*read*/f();},_=>{}}}", Id("r#f"), Unknown),
        (".03 if let", "fn f(x:Option<fn()>){if let Some(r#f)=x {/*read*/f();}}", Id("r#f"), Unknown),
        (".03 loop", "fn f(xs:Vec<fn()>){for r#f in xs {/*read*/f();}}", Id("r#f"), Unknown),
        (".03 closure", "fn f(){let x=|r#f:fn()| {/*read*/f();};}", Id("r#f"), No),
    ] {
        add("rust_binding", label,"rs", source, want, recursion);
    }
    for (label, source, want) in [
        (".04 const pattern", "const r#c:()=(); fn f(x:()){match x {/*read*/c=>{}}}", Id("r#c")),
        (".04 const body", "const r#c:()=(); fn f(x:()){match x {c=>{let x=/*read*/c;}}}", Id("r#c")),
        (".04 unit pattern", "struct r#C; fn f(x:C){let /*read*/C=x;}", Id("r#C")),
        (".04 generic pattern", "fn f<const r#C:()>(x:()){match x {/*read*/C=>{}}}", Local),
        (".04 generic body", "fn f<const r#C:()>(x:()){match x {C=>{let x=/*read*/C;}}}", Local),
        (".05 generic", "fn f<r#T>(){let x: /*read*/T;}", Local),
    ] {
        add("rust_pattern", label,"rs", source, want, No);
    }
    for (label, decl, read, target) in [
        (".05 struct", "struct r#C;", "C", "r#C"),(".05 enum", "enum r#E {A}", "E", "r#E"),
        (".05 trait", "trait r#T {}", "T", "r#T"),(".05 alias", "type r#A=();", "A", "r#A"),
        (".05 static", "static r#c:()=();", "c", "r#c"),(".05 constant", "const r#c:()=();", "c", "r#c"),
        (".05 macro", "macro_rules! r#m {()=>{}}", "m", "r#m"),
    ] {
        let body=if read=="m"{format!("/*read*/{read}!();")}else if read=="c"{format!("let x=/*read*/{read};") } else { format!("let x: /*read*/{read};")};
        add("rust_kinds", label,"rs",&format!("{decl} fn f(){{{body}}}"),Id(target),if read=="m"{Unknown } else { No});
    }
    for (label, source, recursion) in [
        (".07 original", "fn f(){use other::r#f as D; /*read*/D();}", Unknown),
        (".07 alias", "use other::f as r#D; fn f(){/*read*/D();}", Unknown),
        (".07 extern", "extern crate core as r#c; fn f(){let x: /*read*/c::option::Option<()>=None;}", No),
    ] {
        add("rust_import", label,"rs", source,Import, recursion);
    }
    for ext in ["ts", "js"] {
        for (label, source, want, recursion) in [
            (".01 escaped def",r"function \u0066(){/*read*/f();}", Id(r"\u0066"), Yes),
            (".01 escaped call",r"function f(){/*read*/\u0066();}", Id("f"), Yes),
            (".01 brace",r"function \u{66}(){/*read*/f();}", Id(r"\u{66}"), Yes),
            (".01 nonascii",r"function café(){/*read*/caf\u00e9();}", Id("café"), Yes),
            (".17 same",r"function \u0066(){/*read*/\u0066();}", Id(r"\u0066"), Yes),
            (".17 ascii", "function f(){/*read*/f();}", Id("f"), Yes),
            (".17 no NFC", "function café(){/*read*/café();}", Unresolved, No),
        ] {
        add(if ext == "ts" { "ts_call" } else { "js_call"}, label, ext, source, want, recursion);
    }
        for (label, source, want) in [
            (".09 var",r"function f(x){var \u0066=x; /*read*/f();}", Id(r"\u0066")),
            (".09 let",r"function f(x){let \u0066=x; /*read*/f();}", Id(r"\u0066")),
            (".09 const",r"function f(x){const \u0066=x; /*read*/f();}", Id(r"\u0066")),
            (".09 destructured",r"function f(x){const {k:\u0066}=x; /*read*/f();}", Local),
            (".09 formal",r"function f(\u0066){/*read*/f();}", Id(r"\u0066")),
            (".10 catch",r"function f(){try{}catch(\u0066){/*read*/f();}}", Local),
            (".10 var loop",r"function f(xs){for(var \u0066 of xs){/*read*/f();}}", Local),
            (".10 let loop",r"function f(xs){for(let \u0066 of xs){/*read*/f();}}", Local),
            (".10 const loop",r"function f(xs){for(const \u0066 of xs){/*read*/f();}}", Local),
        ] {
        add(if ext == "ts" { "ts_binding" } else { "js_binding"}, label, ext, source, want, Unknown);
    }
        for (label, source, want) in [
            (".09 arrow",r"function f(){const g=(\u0066)=>{/*read*/f();};}", Id(r"\u0066")),
            (".09 single arrow",r"function f(){const g=\u0066=>{/*read*/f();};}", Id(r"\u0066")),
            (".11 function",r"function f(){const g=function \u0066(){/*read*/f();};return /*outer*/f;}", Local),
            (".11 generator",r"function f(){const g=function* \u0066(){/*read*/f();};return /*outer*/f;}", Local),
            (".11 class",r"class C{} function f(){const g=class \u0043{m(){return /*read*/C;}};return /*outer*/C;}", Local),
        ] {
        add(if ext == "ts" { "ts_expression" } else { "js_expression"}, label, ext, source, want, No);
    }
        for (label, source) in [(".14 public",r"class C {f(){this./*read*/\u0066();}}"),(".14 private",r"class C {#f(){this./*read*/#\u0066();}}")] {
        add(if ext == "ts" { "ts_member" } else { "js_member"}, label, ext, source,Candidates(1), Unknown);
    }
        for (label, source, recursion) in [
            (".15 named",r"import {f as \u0044} from 'other'; function f(){/*read*/D();}", Unknown),
            (".15 default",r"import \u0044 from 'other'; function f(){/*read*/D();}", No),
            (".15 namespace",r"import * as \u0044 from 'other'; function f(){return /*read*/D;}", No),
        ] {
        add(if ext == "ts" { "ts_alias" } else { "js_alias"}, label, ext, source,Import, recursion);
    }
        for (label, original, recursion) in [
            (".16 quoted",r#""S""#, Unknown),(".16 escape",r#""\u0053""#, Unknown),
            (".16 brace",r#""\u{53}""#, Unknown),(".16 identifier",r"\u0053", Unknown),
            (".16 control", "S", Unknown),(".17 dotted",r#""A.S""#, No),
            (".S1 hex",r#""\x53""#, Unknown),(".S2 continuation", "\"S\\\n\"", Unknown),
        ] {
        add(if ext == "ts" { "ts_original" } else { "js_original"}, label, ext,&format!("import {{{original} as D}} from 'other'; function S(){{/*read*/D();}}"),Import, recursion);
    }
    }
    for (label, source, want, recursion) in [
        (".12 interface",r"interface \u0049{} function f(x:/*read*/I){}", Id(r"\u0049"), No),
        (".12 alias",r"type \u0041=number; function f(x:/*read*/A){}", Id(r"\u0041"), No),
        (".12 enum",r"enum \u0045{A} function f(x:/*read*/E){}", Id(r"\u0045"), No),
        (".12 namespace",r"namespace \u004E{} function f(){return /*read*/N;}", Id(r"\u004E"), No),
        (".12 class",r"class \u0043{} function f(){new /*read*/C();}", Id(r"\u0043"), No),
        (".12 generic",r"function f<\u0054>(x:/*read*/T){}", Local, No),
        (".12 ambient",r"declare function \u0067():void; function f(){/*read*/g();}", Id(r"\u0067"), No),
        (".12 arrow",r"const \u0066=()=>{/*read*/f();};", Id(r"\u0066"), Yes),
    ] {
        add("ts_kinds", label,"ts", source, want, recursion);
    }
    for (label, source, want, recursion) in [
        (".13 overload",r"function \u0066(x:number):number;function f(x:any){return x;}function g(){/*read*/f(1);}",Candidates(2), No),
        (".15 require",r"import \u0044=require('other');function f(){return /*read*/D;}",Import, No),
        (".15 entity",r"namespace A{export class C{}}import \u0044=A.C;function f(){return /*read*/D;}",Import, No),
        (".S3 surrogate", "import {\"\\uD835\\uDC9C\" as D} from 'other';function 𝒜(){/*read*/D();}",Import, Unknown),
    ] {
        add("ts_extra", label,"ts", source, want, recursion);
    }
    add("ts_extra", ".S4 escaped eval", "ts", r"function f(){\u0065val(''); /*read*/f();}", Candidates(1), Unknown);
    add("js_call", ".S4 escaped eval", "js", r"function f(){\u0065val(''); /*read*/f();}", Candidates(1), Unknown);
    rows
}

fn check(row: &Row, index: &FileSymbolIndex, expected: &FileSymbolIndex, semantics: bool) {
    let start = row.source.find("/*read*/").unwrap() + 8;
    let occurrence = index.occurrences.iter().find(|o| o.span.start_byte as usize == start)
        .unwrap_or_else(|| panic!("{} {} missing required read", row.ext, row.label));
    assert_eq!(&row.source[occurrence.span.start_byte as usize..occurrence.span.end_byte as usize], occurrence.target.name);
    let declaration_id = |name: &str| {
        let candidates: Vec<_> = expected.symbols.iter().filter(|s| s.identity.name == name).collect();
        assert_eq!(candidates.len(), 1, "{} unique declaration {name}", row.label);
        candidates[0].id.clone()
    };
    match row.want {
        Want::Id(name) => {
            assert_eq!(occurrence.confidence, Confidence::Scoped, "{}", row.label);
            assert_eq!(occurrence.target.id, Some(declaration_id(name)), "{} target identity", row.label);
        }
        Want::Local => {
            assert_eq!(occurrence.confidence, Confidence::Scoped, "{}", row.label);
            assert!(occurrence.target.id.is_none(), "{}", row.label);
        }
        Want::Import => {
            assert_eq!(occurrence.confidence, Confidence::Scoped, "{}", row.label);
            assert!(occurrence.target.id.is_none());
            assert!(!index.diagnostics.iter().any(|d| d.rule.as_deref() == Some("unused-import")), "{} unused import", row.label);
        }
        Want::Candidates(count) => {
            assert_eq!(occurrence.confidence, Confidence::NameOnly, "{}", row.label);
            let names = if row.label == ".13 overload" { vec![r"\u0066", "f"] }
                else if row.label == ".14 private" { vec!["#f"] } else { vec!["f"] };
            assert_eq!(names.len(), count);
            let ids: Vec<_> = names.iter().map(|name| declaration_id(name)).collect();
            assert_eq!(occurrence.candidates.iter().map(|c| c.id.clone().unwrap()).collect::<Vec<_>>(), ids);
            assert!(occurrence.target.id.is_none());
        }
        Want::Unresolved => {
            assert_eq!(occurrence.confidence, Confidence::Unresolved);
            assert!(occurrence.target.id.is_none());
        }
    }
    if let Some(at) = row.source.find("/*outer*/") {
        let outer = index.occurrences.iter().find(|o| o.span.start_byte as usize == at + 9).expect("outer binding remains visible");
        let name = if row.label == ".11 class" { "C" } else { "f" };
        assert_eq!(outer.confidence, Confidence::Scoped);
        assert_eq!(outer.target.id, Some(declaration_id(name)));
    }
    for declaration in expected.symbols.iter().filter_map(|s| s.source.name_span.as_ref()) {
        assert!(!index.occurrences.iter().any(|o| o.span == *declaration), "{} declaration token leaked", row.label);
    }
    let suffix = &row.source[occurrence.span.end_byte as usize..];
    let call = suffix.starts_with('(');
    assert_eq!(occurrence.kind, if call { tree_hugger::OccurrenceKind::Call } else { tree_hugger::OccurrenceKind::Usage }, "{} invocation", row.label);
    if row.group.ends_with("call") && row.label != ".S4 escaped eval" {
        let region = index.reference_coverage.iter().find(|r| r.span == occurrence.owner_span && r.owner == occurrence.owner).expect("direct coverage");
        assert_eq!(region.state, tree_hugger::CoverageState::Complete, "{} complete self-call coverage", row.label);
        assert!(region.reasons.is_empty());
    }
    if row.group.ends_with("binding") || row.group.ends_with("member") {
        let region = index.reference_coverage.iter().find(|r| r.span == occurrence.owner_span && r.owner == occurrence.owner).expect("coverage");
        assert_eq!(region.state, tree_hugger::CoverageState::Limited, "{} indirect coverage", row.label);
        assert!(region.reasons.contains(&tree_hugger::LimitReason::IndirectDispatch), "{} indirect reason", row.label);
    }
    if semantics {
        let owner = index.symbols.iter().find(|s| {
            matches!(s.kind, SymbolKindV2::Function | SymbolKindV2::Method) && s.source.body_span.is_some()
                && ["f", "r#f", r"\u0066", r"\u{66}", "café", "S", "𝒜", "#f"].contains(&s.identity.name.as_str())
        }).expect("callable");
        assert_eq!(owner.semantics.is_recursive, Some(row.recursion), "{} {} recursion", row.ext, row.label);
        if row.label == ".06 owner" {
            let parent = owner.relations.parent.as_ref().expect("raw owner parent");
            assert_eq!(parent.name, "r#C");
            assert_eq!(parent.id, Some(declaration_id("r#C")));
        }
        if row.recursion == TriState::Yes {
            assert!(owner.relations.references.iter().any(|r| r.id.as_ref() == Some(&owner.id)));
            assert!(owner.relations.referenced_by.iter().any(|r| r.id.as_ref() == Some(&owner.id)));
        } else if row.label == ".11 function" || row.label == ".11 generator" {
            assert!(owner.relations.references.iter().any(|r| r.id.as_ref() == Some(&owner.id)), "{} outer usage edge retained", row.label);
        } else {
            assert!(!owner.relations.references.iter().any(|r| r.id.as_ref() == Some(&owner.id)), "{} false self edge", row.label);
        }
        if row.label == ".12 ambient" {
            assert_eq!(index.symbols.iter().find(|s| s.identity.name == r"\u0067").unwrap().semantics.is_recursive, Some(TriState::Unknown));
        }
    }
}

fn run(group: &str) {
    let narrow = SymbolQuery::new().kinds([SymbolKindV2::Function, SymbolKindV2::Method]).capabilities([Capability::Occurrences]);
    let semantic = SymbolQuery::new().kinds([SymbolKindV2::Function, SymbolKindV2::Method]).capabilities([Capability::Semantics]);
    let complete = SymbolQuery::complete_index();
    let projected = SymbolQuery::new().kinds([SymbolKindV2::Class, SymbolKindV2::Type]).capabilities([Capability::Occurrences]);
    for row in rows().into_iter().filter(|row| row.group == group) {
        let dir = TempDir::new().expect("temp");
        let path = dir.path().join(format!("names.{}", row.ext));
        std::fs::write(&path, &row.source).expect("source");
        let expected = SourceFile::read(&path).unwrap().query(&complete).unwrap().index;
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
        for (query, wanted_lookup, semantics) in [
            (&narrow, CacheLookup::Miss(CacheMiss::Absent), false),
            (&narrow, CacheLookup::Hit, false),
            (&semantic, CacheLookup::WidenedCapabilities { parsed: false }, true),
            (&complete, CacheLookup::WidenedKinds, true),
            (&complete, CacheLookup::Hit, true),
            (&projected, CacheLookup::Hit, false),
        ] {
            let before = work_counters();
            let result = cache.query(SourceFile::read(&path).unwrap(), query).unwrap();
            let work = work_counters().since(before);
            assert_eq!(result.lookup, wanted_lookup, "{} {} cache", row.ext, row.label);
            assert!(result.write_error.is_none());
            if matches!(wanted_lookup, CacheLookup::Hit | CacheLookup::WidenedCapabilities { parsed: false }) {
                assert_eq!(work.parser_constructions, 0);
            }
            check(&row, &result.result.index, &expected, semantics);
            let fresh = SourceFile::read(&path).unwrap().query(query).unwrap();
            check(&row, &fresh.index, &expected, semantics);
            assert_eq!(serde_json::to_value(&fresh.index).unwrap(), serde_json::to_value(&result.result.index).unwrap());
        }
    }
}

macro_rules! groups {
    ($($group:ident),+) => {
        const GROUPS: &[&str] = &[$(stringify!($group)),+];
        $(#[test] fn $group() { run(stringify!($group)); })+
    };
}
groups!(rust_call, rust_binding, rust_pattern, rust_kinds, rust_import, ts_call, js_call, ts_binding, js_binding, ts_expression, js_expression, ts_member, js_member, ts_alias, js_alias, ts_original, js_original, ts_kinds, ts_extra);

#[test]
fn corpus_is_partitioned() {
    let rows = rows();
    for row in &rows { assert_eq!(GROUPS.iter().filter(|&&group| group == row.group).count(), 1); }
    for &group in GROUPS {
        let count = rows.iter().filter(|row| row.group == group).count();
        assert!((1..=9).contains(&count), "{group}: {count}");
    }
}

#[test]
fn unsupported_language_names_keep_uncertain_resolution() {
    use tree_hugger::ProgrammingLanguage as L;
    for language in [L::Go, L::Python, L::Java, L::CSharp, L::C, L::Cpp, L::Swift, L::Scala, L::Php, L::Perl, L::Bash, L::Zsh, L::Lua] {
        let path = crate::symbols_fixtures::primary_fixture(language).expect("fixture");
        let index = SourceFile::read(path).unwrap().query(&SymbolQuery::complete_index()).unwrap().index;
        assert!(!index.occurrences.is_empty(), "{language:?}: genuine reads");
        assert!(index.occurrences.iter().all(|o| o.confidence != Confidence::Scoped && o.target.id.is_none()), "{language:?}: no established targets");
        assert!(index.reference_coverage.iter().all(|r| r.state == tree_hugger::CoverageState::Limited), "{language:?}: limited coverage");
        assert!(index.symbols.iter().filter_map(|s| s.semantics.is_recursive).all(|state| state == TriState::Unknown), "{language:?}: no false Yes/No");
    }
}
