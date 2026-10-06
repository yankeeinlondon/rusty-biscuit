use tree_sitter::{Node, Parser};
fn dump(n: Node<'_>, source: &str, depth: usize) {
    if n.is_named() {
        println!("{}{} {:?}", " ".repeat(depth), n.kind(), &source[n.byte_range()]);
    }
    let mut c = n.walk();
    for child in n.children(&mut c) {
        if child.is_named() { dump(child, source, depth + 1); }
    }
}
fn main() {
    for (language, source) in [
        (tree_sitter_c_sharp::LANGUAGE.into(), "using Alias = Lib.Base; using Lib; namespace N { interface I : Other {} struct S : I, External {} class C<T> : Base<T>, I, External {} enum E : int { A } record R : Base, I; }"),
        (tree_sitter_swift::LANGUAGE.into(), "import Foundation\nimport class Lib.Base\nprotocol P: Other {}\nclass Base {}\nclass C<T>: Base, P, External {}\nstruct S: P, Codable {}\nenum E: Int, P { case a = 1 }\nenum F: P { case a }\nextension C: Equatable where T: Equatable {}\nextension Lib.C: P {}\n"),
        (tree_sitter_scala::LANGUAGE.into(), "package sample\nimport lib.Base\nimport lib.{Base => Alias, X}\ntrait T extends Base with Other\nclass Base\nclass C[A] extends Base with T with External\nclass D extends Base(1) with T\nobject O extends T\n"),
    ] {
        let mut parser = Parser::new(); parser.set_language(&language).unwrap();
        let tree = parser.parse(source, None).unwrap();
        dump(tree.root_node(), source, 0);
    }
}
