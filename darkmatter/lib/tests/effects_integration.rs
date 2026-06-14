use darkmatter::effects::EffectEngine;
use serde_json::json;
use std::io::Write;
use std::net::TcpListener;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;

#[test]
fn file_and_dir_verbs() {
    let dir = tempfile::TempDir::new().unwrap();
    let eng = EffectEngine::builder().mutation_root(dir.path()).build();

    // ensure_dir
    let made = eng.ensure_dir("out/logs").unwrap();
    assert!(std::path::Path::new(&made).is_dir());

    // ensure_file idempotent: returns absolute path, leaves existing untouched
    let p = eng.ensure_file("out/state.md").unwrap();
    std::fs::write(&p, "preexisting").unwrap();
    let p2 = eng.ensure_file("out/state.md").unwrap();
    assert_eq!(p, p2);
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "preexisting");

    // ensure_file with content only writes when creating
    let c = eng.ensure_file_with_content("out/seed.md", "seed").unwrap();
    assert_eq!(std::fs::read_to_string(&c).unwrap(), "seed");

    // append_line
    eng.append_line("out/logs/run.log", "first").unwrap();
    eng.append_line("out/logs/run.log", "second").unwrap();
    let log = std::fs::read_to_string(dir.path().join("out/logs/run.log")).unwrap();
    assert_eq!(log, "first\nsecond\n");

    // append_jsonl
    eng.append_jsonl("out/logs/events.jsonl", json!({"ok": true}))
        .unwrap();
    let jsonl = std::fs::read_to_string(dir.path().join("out/logs/events.jsonl")).unwrap();
    assert_eq!(jsonl.trim(), r#"{"ok":true}"#);

    // mutation-root escape is refused
    assert!(eng.ensure_file("../escape.md").is_err());
}

#[test]
fn http_post_uses_allowed_host_policy() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let request_count = Arc::clone(&requests);

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        request_count.fetch_add(1, Ordering::SeqCst);
        let mut buf = [0_u8; 4096];
        let _ = std::io::Read::read(&mut stream, &mut buf);
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
        stream.write_all(response.as_bytes()).unwrap();
    });

    let eng = EffectEngine::builder().allowed_hosts(["127.0.0.1"]).build();

    let response = eng
        .http_post(&format!("http://{addr}/hook"), b"{\"ok\":true}")
        .unwrap();

    assert_eq!(response["status"], json!(200));
    assert_eq!(response["body"], json!("ok"));
    assert_eq!(requests.load(Ordering::SeqCst), 1);
}
