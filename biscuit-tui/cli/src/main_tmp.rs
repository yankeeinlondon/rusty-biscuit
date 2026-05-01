fn main() {
    use clap::Parser;
    use clap_complete::Shell;
    
    #[derive(Parser)]
    #[command(name = "question")]
    struct TestCli {
        /// Option strings. Trailing positional arguments become the list
        /// of options when no explicit source flag is set.
        #[arg(value_name = "OPTIONS")]
        positional: Vec<String>,
    }
    
    let mut buf = Vec::new();
    let mut cmd = TestCli::command();
    clap_complete::generate(Shell::Zsh, &mut cmd, "question", &mut buf);
    let text = String::from_utf8(buf).unwrap();
    println!("=== FULL SCRIPT ===");
    println!("{}", text);
    println!("=== POS LINES ===");
    for line in text.lines() {
        if line.contains("positional") || line.contains("_default") || line.contains("_question_choice") {
            println!("{}", line);
        }
    }
}
