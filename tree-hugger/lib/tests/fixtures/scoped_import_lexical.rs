// Shared source corpus: import expectations are independent of cache output.
#[derive(Clone)]
pub struct Case {
    pub label: String,
    pub extension: &'static str,
    pub source: String,
    pub import: Option<(&'static str, &'static str)>,
}
pub fn cases(extension: &'static str) -> Vec<Case> {
    let mut rows = Vec::new();
    let mut add = |label: &str, source: String, import| rows.push(Case { label: label.into(), extension, source, import });
    if matches!(extension, "ts" | "js") {
        for op in ["+", "-", "*", "/", "%", "**", "<<", ">>", ">>>", "<", ">", "&", "|", "^", "&&", "||", "<=", ">=", "==", "!=", "===", "!==", "??"] {
            add(&format!("binary {op}"), format!("function f(x) {{ return x {op} /import D/; }}"), None);
        }
        for op in ["+", "-", "~", "void", "typeof", "delete", "!"] {
            add(&format!("unary {op}"), format!("function f() {{ return {op} /import D/; }}"), None);
        }
        for (label, source) in [
            ("if", "function f(){if(true) /import D/.test('');}"),
            ("while", "function f(){while(false) /import D/.test('');}"),
            ("for", "function f(){for(;false;) /import D/.test('');}"),
            ("else", "function f(){if(false) {} else /import D/.test('');}"),
            ("do", "function f(){do /import D/.test(''); while(false);}"),
            ("throw", "function f(){throw /import D/;}"),
            ("yield", "function* gen(){yield /import D/;} function f(){}"),
            ("await", "async function f(){await /import D/;}"),
            ("default", "export default /import D/; function f(){}"),
            ("assignment", "const r=/import D/; function f(){}"),
            ("return", "function f(){return /import D/;}"),
            ("block", "function f(){ {} /import D/.test('');}"),
            ("case", "function f(x){switch(x){case 0: /import D/.test('');}}"),
            ("argument", "function f(){g(/import D/);}"),
            ("array", "function f(){return [/import D/];}"),
            ("conditional", "function f(x){return x ? /import D/ : /import D/;}"),
            ("string", "const s='import D'; function f(){}"),
            ("template", "const s=`import D`; function f(){}"),
            ("division", "function f(x){return x / 2 / 3;}"),
        ] { add(label, source.into(), None); }
        add("non-nesting", "/* /* */ import {g as D} from 'a'; function f(){D();}".into(), Some(("D", "a")));
    } else {
        let sources: &[(&str, &str)] = match extension {
            "swift" => &[("raw one", "let s = #\" quote \" import A.B \"#\nfunc f() {}"),
                ("raw two", "let s = ##\" quote \"# import A.B \"##\nfunc f() {}"),
                ("raw multiline", "let s = #\"\"\"\n\"\"\" import A.B\n\"\"\"#\nfunc f() {}"),
                ("ordinary", "let s = \"import A.B\"\nfunc f() {}"),
                ("multiline", "let s = \"\"\"\nimport A.B\n\"\"\"\nfunc f() {}")],
            "cs" => &[("raw three", "class C { string S = \"\"\" quote \" using A.B; \"\"\"; void f(){} }"),
                ("raw four", "class C { string S = \"\"\"\" quote \"\"\" using A.B; \"\"\"\"; void f(){} }"),
                ("verbatim", "class C { string S = @\" quote \"\" using A.B; \"; void f(){} }"),
                ("interpolated", "class C { string S = $\"using A.B;\"; void f(){} }")],
            "rs" => &[("raw", "const S:&str=r#\" quote \" use a::D; \"#; fn f(){}"),
                ("raw two", "const S:&str=r##\" quote \"# use a::D; \"##; fn f(){}"),
                ("unicode char", "const C:char='é'; fn f(){}"),
                ("macro", "macro_rules! m{()=>{use a::D;}} fn f(){}"),
                ("macro call", "m!{use a::D;} fn f(){}"),
                ("lifetime", "fn f<'a>() {}")],
            "java" => &[("text block", "class C { String s = \"\"\"\nimport A.B;\n\"\"\"; void f(){} }")],
            "go" => &[("raw", "package p\nvar s = `import D \"a\"`\nfunc f(){}")],
            "scala" => &[("triple", "val s = \"\"\"import A.B\"\"\"\ndef f()=0")],
            "py" => &[("ordinary", "s='from a import D'\ndef f(): pass"),
                ("triple", "s=\"\"\"from a import D\"\"\"\ndef f(): pass"),
                ("f string", "s=f'from a import D'\ndef f(): pass"),
                ("comment", "# from a import D\ndef f(): pass")],
            "php" => &[("heredoc", "<?php $s=<<<TXT\nuse A\\D;\nTXT;\nfunction f(){}"),
                ("nowdoc", "<?php $s=<<<'TXT'\nuse A\\D;\nTXT;\nfunction f(){}"),
                ("hash", "<?php # use A\\D;\nfunction f(){}")],
            "c" | "cpp" => &[("no collector", "void f(){}")],
            "lua" => &[("no collector", "function f() end")],
            "pl" => &[("no collector", "sub f {}")],
            "sh" | "zsh" => &[("no collector", "f(){ :; }")],
            _ => panic!("unknown language"),
        };
        for (label, source) in sources { add(label, (*source).into(), None); }
        let comment = match extension {
            "java" => Some(("/* /* */ import a.D; class C {void f(){}}", "D", "a")),
            "cs" => Some(("/* /* */ using D = A.B; class C {void f(){}}", "D", "A.B")),
            "go" => Some(("package p\n/* /* */ import D \"a\"\nfunc f(){}", "D", "a")),
            "php" => Some(("<?php /* /* */ use A\\B as D; function f(){}", "D", "A\\B")),
            "rs" => Some(("/* /* */ */ use a::g as D; fn f(){D();}", "D", "a")),
            "scala" => Some(("/* /* */ */ import a.g as D\ndef f()=D()", "D", "a.g")),
            "swift" => Some(("/* /* */ */ import A.B\nfunc f(){}", "A.B", "A.B")),
            _ => None,
        };
        if let Some((source, name, module)) = comment { add("comment boundary", source.into(), Some((name, module))); }
    }
    rows
}
