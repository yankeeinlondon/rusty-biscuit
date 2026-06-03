use tree_hugger::{TreeFile, ProgrammingLanguage};
fn main() {
    let dir = std::env::temp_dir().join("tsxprobe");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("component.tsx");
    std::fs::write(&p, "export function App() {\n  return <div className=\"x\">hi</div>;\n}\n").unwrap();
    let tf = TreeFile::with_language(&p, Some(ProgrammingLanguage::TypeScript)).unwrap();
    println!("symbols: {:?}", tf.symbols().unwrap().iter().map(|s| (&s.name, s.kind)).collect::<Vec<_>>());
    println!("syntax diags: {:?}", tf.syntax_diagnostics().len());
    // Compare to plain .ts (no JSX)
    let p2 = dir.join("plain.ts");
    std::fs::write(&p2, "export function App() { return 1; }\n").unwrap();
    let tf2 = TreeFile::new(&p2).unwrap();
    println!("plain symbols: {:?}", tf2.symbols().unwrap().iter().map(|s| (&s.name, s.kind)).collect::<Vec<_>>());
}
