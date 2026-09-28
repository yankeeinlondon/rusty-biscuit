//! Every delivery task in `src/messaging/` starts through the delivery tracker.
//!
//! A task spawned anywhere else in `messaging` is invisible to
//! `drain_deliveries`, so the CLI would exit over it and lose the message
//! (`2026-09-25-lifecycle-message-exit-race`, R1). This guard fails on any
//! `::spawn`, `.spawn`, `spawn_blocking`, or `spawn_local` in the module outside
//! `delivery.rs`, the tracker itself.
//!
//! Source is lexed with `proc-macro2` rather than searched as text, so a spawn
//! named in a comment, doc comment, or string never trips the guard.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// The one file allowed to spawn: the tracker registers every task it starts.
const TRACKER_FILE: &str = "delivery.rs";

/// `(line, column)` of every task spawn in `source`.
fn spawn_sites(source: &str) -> Vec<(usize, usize)> {
    let stream = TokenStream::from_str(source).expect("messaging source lexes");
    let mut flat = Vec::new();
    flatten(stream, &mut flat);

    let mut sites = Vec::new();
    for (index, token) in flat.iter().enumerate() {
        let TokenTree::Ident(ident) = token else {
            continue;
        };
        let name = ident.to_string();
        let is_spawn = match name.as_str() {
            "spawn_blocking" | "spawn_local" => true,
            // A bare `spawn` is a call only as a path segment or a method.
            "spawn" => index
                .checked_sub(1)
                .and_then(|prev| flat.get(prev))
                .is_some_and(|prev| matches!(prev, TokenTree::Punct(p) if p.as_char() == '.' || p.as_char() == ':')),
            _ => false,
        };
        if is_spawn {
            let start = ident.span().start();
            sites.push((start.line, start.column + 1));
        }
    }
    sites
}

/// Flatten groups into one token sequence so adjacency survives nesting.
fn flatten(stream: TokenStream, out: &mut Vec<TokenTree>) {
    for token in stream {
        match token {
            TokenTree::Group(group) => {
                // A doc comment lexes as `#[doc = "…"]`; its text is a literal,
                // so descending into it cannot surface a spawn.
                let inner = group.stream();
                if group.delimiter() == Delimiter::None {
                    flatten(inner, out);
                } else {
                    out.push(TokenTree::Group(group));
                    flatten(inner, out);
                }
            }
            other => out.push(other),
        }
    }
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn messaging_starts_tasks_only_through_the_delivery_tracker() {
    let root = biscuit_test_harness::manifest_dir!().join("src/messaging");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    files.sort();

    let mut violations = Vec::new();
    let mut tracker_sites = 0;
    let mut scanned_send = false;
    for path in &files {
        let relative = path.strip_prefix(&root).expect("under the messaging root");
        let source = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let sites = spawn_sites(&source);
        if relative == Path::new(TRACKER_FILE) {
            tracker_sites = sites.len();
            continue;
        }
        scanned_send |= relative == Path::new("send.rs");
        for (line, column) in sites {
            violations.push(format!("src/messaging/{}:{line}:{column}", relative.display()));
        }
    }

    // Control rows: the scan reached the helpers, and the detector does see
    // the tracker's own spawn, so an empty violation list is not vacuous.
    assert!(scanned_send, "the guard never scanned src/messaging/send.rs");
    assert_eq!(
        tracker_sites, 1,
        "expected exactly one spawn in src/messaging/{TRACKER_FILE}, the tracker's own"
    );
    assert!(
        violations.is_empty(),
        "these sites start a task outside the delivery tracker; start it with \
         `delivery::track` so the CLI can drain it before exit:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn the_detector_finds_every_spawn_form() {
    let planted = r#"
fn planted(handle: tokio::runtime::Handle) {
    tokio::spawn(async {});
    handle.spawn(async {});
    tokio::task::spawn_blocking(|| ());
    tokio::task::spawn_local(async {});
    std::thread::spawn(|| ());
}
"#;
    assert_eq!(
        spawn_sites(planted),
        vec![(3, 12), (4, 12), (5, 18), (6, 18), (7, 18)]
    );
}

#[test]
fn the_detector_ignores_comments_strings_and_unrelated_names() {
    let quiet = r#"
//! Module docs mention `tokio::spawn` and `.spawn(`.
/// Item docs mention `handle.spawn(fut)` and `spawn_blocking`.
fn quiet() {
    // tokio::spawn(async {});
    /* handle.spawn(async {}); spawn_blocking */
    let text = "tokio::spawn(async {}) and .spawn(";
    let spawn = 1;
    let spawned = spawn + 1;
    respawn_later(spawned);
}
"#;
    assert!(spawn_sites(quiet).is_empty());
}
