//! The process snapshot every `claudine` route resolves file references from.
//!
//! `dispatch` reads the process once ([`RequestSnapshot::from_process`]),
//! anchors the snapshot on the invocation's launch directory, and stores it
//! here before any command runs. Every invocation context, completion scope,
//! and hook dispatch builds its file-resolution context from this snapshot, so
//! no route reads the current directory, home directory, or environment again
//! to resolve a reference or seed `ctx.*`. See
//! `darkmatter/docs/topics/compose-requests.md`.

use std::path::Path;
use std::sync::OnceLock;

use darkmatter::markdown::compose::RequestSnapshot;

static LAUNCH_SNAPSHOT: OnceLock<RequestSnapshot> = OnceLock::new();

/// Captures the process snapshot, anchored at `launch_dir`.
///
/// `launch_dir` is the invocation's launch directory: the entry directory, or
/// for a provider hook the `AGENT_CWD` its wrapper supplied. Repeated calls
/// keep the first snapshot.
///
/// ## Errors
///
/// Returns the I/O error when the current directory cannot be read.
pub(crate) fn initialize(launch_dir: &Path) -> std::io::Result<&'static RequestSnapshot> {
    if let Some(snapshot) = LAUNCH_SNAPSHOT.get() {
        return Ok(snapshot);
    }
    let snapshot = RequestSnapshot::from_process()?.at_request_dir(launch_dir);
    Ok(LAUNCH_SNAPSHOT.get_or_init(|| snapshot))
}

/// Where every document after the launch document captures its environment:
/// this process's environment at the moment the document is entered.
///
/// The launch document reads the snapshot [`initialize`] captured; a sequence
/// task, group leg, or `proxy` target captures again through
/// `InvocationContext::capture_document`, so the library never reads the
/// process itself.
pub(crate) fn document_environment_source() -> claudine::invocation_context::EnvironmentSource {
    #[cfg(feature = "test-fixtures")]
    if let Some(scripted) = scripted_document_environments() {
        return scripted;
    }
    claudine::invocation_context::EnvironmentSource::new(capture_document_environment)
}

/// This process's environment as a newly entered document sees it.
///
/// `PWD` keeps the launch snapshot's value: Claudine rewrites its own `PWD`
/// when it switches into a document's child working directory
/// (`switch_process_cwd`), and that bookkeeping is not a change to the
/// environment a later document should observe.
fn capture_document_environment() -> std::collections::HashMap<String, String> {
    let mut env = biscuit_file::capture_env();
    match snapshot().env().get("PWD") {
        Some(pwd) => {
            env.insert("PWD".to_string(), pwd.clone());
        }
        None => {
            env.remove("PWD");
        }
    }
    env
}

/// Test seam, compiled only with the `test-fixtures` feature (never into a
/// default or installed build): `CLAUDINE_TEST_DOCUMENT_ENVIRONMENTS=<path>`
/// names a JSON array of overlays, and the `n`th document captured after
/// launch reads this process's environment with the `n`th overlay applied
/// (`null` removes a variable; past the end, no overlay). Each capture appends
/// its ordinal to `<path>.captures`, so a test can count them.
///
/// A process cannot change its own environment between documents the way a
/// user's shell can between tasks, so this is how a spawned test supplies a
/// different map to each document.
#[cfg(feature = "test-fixtures")]
fn scripted_document_environments() -> Option<claudine::invocation_context::EnvironmentSource> {
    use std::io::Write as _;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let path = snapshot().env().get("CLAUDINE_TEST_DOCUMENT_ENVIRONMENTS")?.clone();
    let overlays: Vec<serde_json::Map<String, serde_json::Value>> = serde_json::from_str(
        &std::fs::read_to_string(&path).expect("read the scripted document environments"),
    )
    .expect("scripted document environments are a JSON array of objects");
    let captures = format!("{path}.captures");
    let next = AtomicUsize::new(0);
    Some(claudine::invocation_context::EnvironmentSource::new(move || {
        let ordinal = next.fetch_add(1, Ordering::SeqCst);
        let mut env = capture_document_environment();
        for (name, value) in overlays.get(ordinal).into_iter().flatten() {
            match value {
                serde_json::Value::Null => {
                    env.remove(name);
                }
                serde_json::Value::String(text) => {
                    env.insert(name.clone(), text.clone());
                }
                other => {
                    env.insert(name.clone(), other.to_string());
                }
            }
        }
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&captures)
            .expect("open the document capture log");
        writeln!(log, "{ordinal}").expect("record a document capture");
        env
    }))
}

/// The snapshot `dispatch` captured.
///
/// ## Panics
///
/// Panics when called before [`initialize`]: every route runs after
/// `dispatch` has captured the snapshot.
#[cfg(not(test))]
pub(crate) fn snapshot() -> &'static RequestSnapshot {
    LAUNCH_SNAPSHOT
        .get()
        .expect("the request snapshot is captured in `dispatch` before any command runs")
}

/// Darkmatter's user configuration, read once per invocation from the launch
/// snapshot's home with [`ConfigLoadMode::MigrateLegacy`] (Claudine is a
/// writing CLI). `dispatch` reads it before any command runs; every
/// invocation context then projects this one snapshot
/// ([`with_darkmatter_config`]).
///
/// [`ConfigLoadMode::MigrateLegacy`]: darkmatter::config::load::ConfigLoadMode::MigrateLegacy
pub(crate) fn darkmatter_config()
-> Result<&'static std::sync::Arc<darkmatter::config::load::ConfigSnapshot>, &'static darkmatter::config::load::ConfigError> {
    static CONFIG: OnceLock<
        Result<std::sync::Arc<darkmatter::config::load::ConfigSnapshot>, darkmatter::config::load::ConfigError>,
    > = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            darkmatter::config::load::ConfigSnapshot::load_user(
                snapshot(),
                darkmatter::config::load::ConfigLoadMode::MigrateLegacy,
            )
            .map(std::sync::Arc::new)
        })
        .as_ref()
}

/// `invocation` projecting the configuration [`darkmatter_config`] read,
/// instead of reading the user file again.
pub(crate) fn with_darkmatter_config(
    invocation: claudine::invocation_context::InvocationContext,
) -> claudine::invocation_context::InvocationContext {
    match darkmatter_config() {
        Ok(snapshot) => invocation.with_darkmatter_config(std::sync::Arc::clone(snapshot)),
        Err(_) => invocation,
    }
}

/// Unit tests reach routes without `dispatch`, so they capture the test
/// process on first use.
#[cfg(test)]
pub(crate) fn snapshot() -> &'static RequestSnapshot {
    LAUNCH_SNAPSHOT.get_or_init(|| RequestSnapshot::from_process().expect("read the test process"))
}

/// Claudine's launch context for a request launched at `dir`, from the test
/// process.
#[cfg(test)]
pub(crate) fn test_context_at(dir: impl AsRef<Path>) -> biscuit_file::FileResolutionContext {
    claudine::composition::capture_file_resolution_context(&snapshot().at_request_dir(dir.as_ref()))
        .expect("build the test context")
}

/// A launch context outside any repository (the system temporary directory),
/// whose fallback tree admits a test document wherever its fixture lives, as
/// a request launched beside that document would.
#[cfg(test)]
pub(crate) fn test_context() -> biscuit_file::FileResolutionContext {
    test_context_at(std::env::temp_dir())
}

/// [`test_context`] built once, for a test that borrows it.
#[cfg(test)]
pub(crate) fn test_context_ref() -> &'static biscuit_file::FileResolutionContext {
    static CONTEXT: OnceLock<biscuit_file::FileResolutionContext> = OnceLock::new();
    CONTEXT.get_or_init(test_context)
}
