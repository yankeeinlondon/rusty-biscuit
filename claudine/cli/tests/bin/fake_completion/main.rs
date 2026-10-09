//! Portable literal Claude/Codex stream emitter for completion-cutoff callers.
use std::io::{Read, Write};
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--version") { println!("0.157.1"); return; }
    if args.first().is_some_and(|arg| arg == "app-server") { std::process::exit(3); }
    let dir = std::path::PathBuf::from(std::env::var_os("FAKE_COMPLETION_DIR").expect("fixture dir"));
    let provider = std::env::var("FAKE_COMPLETION_PROVIDER").expect("fixture provider");
    let mut prompt = String::new();
    std::io::stdin().read_to_string(&mut prompt).unwrap();
    std::fs::write(dir.join("prompt.txt"), prompt).unwrap();
    std::fs::write(dir.join("argv.json"), serde_json::to_vec(&args).unwrap()).unwrap();
    let records = if provider == "claude" {
        [r#"{"type":"system","subtype":"init","session_id":"session-1","model":"claude-opus"}"#,
         r#"{"type":"assistant","message":{"content":[{"type":"text","text":"review-answer"}]}}"#,
         r#"{"type":"result","subtype":"success","result":"review-answer","session_id":"session-1","is_error":false,"duration_ms":1200,"num_turns":1,"usage":{"input_tokens":100,"output_tokens":50}}"#]
    } else {
        [r#"{"type":"thread.started","thread_id":"th-1"}"#,
         r#"{"type":"item.completed","item":{"id":"a1","type":"agent_message","text":"review-answer"}}"#,
         r#"{"type":"turn.completed","usage":{"input_tokens":100,"output_tokens":50},"duration_ms":800,"status":"completed"}"#]
    };
    let mut out = std::io::stdout().lock();
    for line in records { writeln!(out, "{line}").unwrap(); out.flush().unwrap(); }
    if let Some(index) = args.iter().position(|arg| arg == "--output-last-message") {
        std::fs::write(&args[index + 1], "review-answer").unwrap();
    }
    std::fs::write(dir.join("native-exit.txt"), "0").unwrap();
}
