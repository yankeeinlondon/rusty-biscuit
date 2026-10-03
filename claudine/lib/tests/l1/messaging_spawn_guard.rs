//! Every delivery task in `src/messaging/` starts through the delivery tracker.
//!
//! A task spawned anywhere else in `messaging` is invisible to
//! `drain_deliveries`, so the CLI would exit over it and lose the message
//! (`2026-09-25-lifecycle-message-exit-race`, R1). This guard fails on any
//! `::spawn`, `.spawn`, `spawn_blocking`, or `spawn_local` in the module except
//! the one inside `delivery.rs`'s `track`, the tracker itself. The tracker
//! site is pinned by its enclosing function, not by a count: a spawn moved
//! into another `delivery.rs` function could be called untracked from
//! `send.rs` while the file still held exactly one spawn.
//!
//! Source is lexed with `proc-macro2` rather than searched as text, so a spawn
//! named in a comment, doc comment, or string never trips the guard.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// The one file allowed to spawn: the tracker registers every task it starts.
const TRACKER_FILE: &str = "delivery.rs";
/// The one function in [`TRACKER_FILE`] allowed to spawn.
const TRACKER_FUNCTION: &str = "track";

/// One task spawn: `(line, column)` and the innermost enclosing `fn`, if any.
#[derive(Debug, PartialEq, Eq)]
struct SpawnSite {
    line: usize,
    column: usize,
    function: Option<String>,
}

/// Every task spawn in `source`.
fn spawn_sites(source: &str) -> Vec<SpawnSite> {
    let stream = TokenStream::from_str(source).expect("messaging source lexes");
    let mut sites = Vec::new();
    walk(stream, None, &mut sites);
    sites
}

/// Visit `stream`, attributing each spawn to `function`, and to the `fn` whose
/// body a brace group is when one opens.
fn walk(stream: TokenStream, function: Option<&str>, sites: &mut Vec<SpawnSite>) {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    // The name of a `fn` whose body brace has not opened yet.
    let mut pending_fn: Option<String> = None;
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Group(group) => {
                // A doc comment lexes as `#[doc = "…"]`; its text is a literal,
                // so descending into it cannot surface a spawn.
                let body_of = if group.delimiter() == Delimiter::Brace {
                    pending_fn.take()
                } else {
                    None
                };
                walk(group.stream(), body_of.as_deref().or(function), sites);
            }
            TokenTree::Punct(punct) if punct.as_char() == ';' => pending_fn = None,
            TokenTree::Ident(ident) => {
                let name = ident.to_string();
                if name == "fn"
                    && let Some(TokenTree::Ident(fn_name)) = tokens.get(index + 1)
                {
                    pending_fn = Some(fn_name.to_string());
                }
                let is_spawn = match name.as_str() {
                    "spawn_blocking" | "spawn_local" => true,
                    // A bare `spawn` is a call only as a path segment or a method.
                    "spawn" => index
                        .checked_sub(1)
                        .and_then(|prev| tokens.get(prev))
                        .is_some_and(|prev| matches!(prev, TokenTree::Punct(p) if p.as_char() == '.' || p.as_char() == ':')),
                    _ => false,
                };
                if is_spawn {
                    let start = ident.span().start();
                    sites.push(SpawnSite {
                        line: start.line,
                        column: start.column + 1,
                        function: function.map(str::to_string),
                    });
                }
            }
            _ => {}
        }
    }
}

/// Why the tracker file's spawns are not exactly the one in `track`, if they
/// are not.
fn tracker_violation(sites: &[SpawnSite]) -> Option<String> {
    let in_track = |site: &SpawnSite| site.function.as_deref() == Some(TRACKER_FUNCTION);
    (sites.len() != 1 || !sites.iter().all(in_track)).then(|| {
        format!(
            "expected exactly one spawn in src/messaging/{TRACKER_FILE}, inside \
             `{TRACKER_FUNCTION}`; found {sites:?}"
        )
    })
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
    let mut tracker_problem = Some(format!("the guard never scanned src/messaging/{TRACKER_FILE}"));
    let mut scanned_send = false;
    for path in &files {
        let relative = path.strip_prefix(&root).expect("under the messaging root");
        let source = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let sites = spawn_sites(&source);
        if relative == Path::new(TRACKER_FILE) {
            tracker_problem = tracker_violation(&sites);
            continue;
        }
        scanned_send |= relative == Path::new("send.rs");
        for SpawnSite { line, column, .. } in sites {
            violations.push(format!("src/messaging/{}:{line}:{column}", relative.display()));
        }
    }

    // Control rows: the scan reached the helpers, and the detector does see
    // the tracker's own spawn, so an empty violation list is not vacuous.
    assert!(scanned_send, "the guard never scanned src/messaging/send.rs");
    if let Some(problem) = tracker_problem {
        panic!("{problem}");
    }
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
    let positions: Vec<_> = spawn_sites(planted)
        .into_iter()
        .map(|site| (site.line, site.column, site.function))
        .collect();
    let planted_fn = Some("planted".to_string());
    assert_eq!(
        positions,
        vec![
            (3, 12, planted_fn.clone()),
            (4, 12, planted_fn.clone()),
            (5, 18, planted_fn.clone()),
            (6, 18, planted_fn.clone()),
            (7, 18, planted_fn),
        ]
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

/// The tracker file keeps its one spawn, but in a helper that `track` calls:
/// anything in `send.rs` could call that helper and skip registration.
#[test]
fn a_tracker_spawn_moved_out_of_track_fails() {
    let original = r#"
pub(crate) fn track<F>(label: DeliveryLabel, future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let Ok(runtime) = tokio::runtime::Handle::try_current() else { return; };
    let handle = runtime.spawn(future);
    register(label, handle);
}
"#;
    let moved = r#"
pub(crate) fn spawn_untracked<F>(future: F) -> JoinHandle<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::runtime::Handle::current().spawn(future)
}
pub(crate) fn track<F>(label: DeliveryLabel, future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    register(label, spawn_untracked(future));
}
"#;
    assert_eq!(tracker_violation(&spawn_sites(original)), None);
    let moved_sites = spawn_sites(moved);
    assert_eq!(moved_sites.len(), 1, "the file still holds exactly one spawn");
    let problem = tracker_violation(&moved_sites).expect("a spawn outside `track` fails");
    assert!(problem.contains("spawn_untracked"), "{problem}");
}
