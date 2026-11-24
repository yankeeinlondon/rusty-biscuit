//! The prepared request an integration test composes under.
//!
//! `request` keeps the request directory the pipeline chose before requests
//! were prepared explicitly (a file source's directory, else the context's
//! anchor, else a per-process temp directory), takes home from the process, and the
//! environment from the options' own context: empty unless the test put
//! values there, so no test sees the process environment by accident. A test
//! whose subject is the request directory builds its own [`RequestSnapshot`].

use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use darkmatter::config::load::{ConfigLoadMode, ConfigSnapshot};
use darkmatter::markdown::compose::{
    ComposeContext, ComposeOptions, ComposeRequest, ComposeSource, ContextAuthority, ContextGroup, RequestSnapshot,
    build_resolution_context,
};

pub fn request(options: ComposeOptions) -> ComposeRequest {
    let dir = request_dir(&options);
    request_at(&dir, options)
}

/// Like [`request`], but a source-less request is anchored at the process
/// directory, for tests whose subject is the ambient capture of a fixture they
/// entered with `set_current_dir`.
pub fn process_dir_request(options: ComposeOptions) -> ComposeRequest {
    let dir = match options.source() {
        ComposeSource::File(_) => request_dir(&options),
        _ if options.context().anchor().is_absolute() => options.context().anchor().to_path_buf(),
        _ => std::env::current_dir().expect("process directory"),
    };
    request_at(&dir, options)
}

/// A request anchored at `dir`, for options with no source of their own
/// (a reference graph builds one per document it visits).
pub fn request_at(dir: &Path, options: ComposeOptions) -> ComposeRequest {
    let snapshot = RequestSnapshot::new(dir)
        .with_home(biscuit_file::home_dir())
        .with_env(options.context().env().clone());
    ComposeRequest::prepare(options, &snapshot).expect("test request")
}

/// The context a test resolves through when its subject is not the request
/// directory: built at `dir` with the process's home and environment, so a
/// repository containing `dir` supplies the `&`/`^` roots.
pub fn context_at(dir: &Path) -> FileResolutionContext {
    let snapshot = RequestSnapshot::new(dir)
        .with_home(biscuit_file::home_dir())
        .with_env(biscuit_file::capture_env());
    build_resolution_context(&snapshot).expect("test context")
}

/// A source-less, anchor-less request is anchored in a per-process temp
/// directory, not the process directory: that is inside this monorepo, and
/// preparing a request there runs repository discovery over all of it.
fn request_dir(options: &ComposeOptions) -> PathBuf {
    static SCRATCH: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    let scratch = &SCRATCH;
    let process = || std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match options.source() {
        ComposeSource::File(path) => {
            let absolute = if path.is_absolute() { path.clone() } else { process().join(path) };
            absolute.parent().map(Path::to_path_buf).unwrap_or_else(process)
        }
        _ if options.context().anchor().is_absolute() => options.context().anchor().to_path_buf(),
        _ => scratch.get_or_init(|| tempfile::tempdir().expect("scratch request dir")).path().to_path_buf(),
    }
}

/// The context a context-free API used before one was required: anchored at
/// the process directory with the process's home and environment, and no
/// repository discovery. For tests whose documents carry no path and whose
/// assertions do not depend on where they resolve.
pub fn cwd_context() -> FileResolutionContext {
    FileResolutionContext::new(std::env::current_dir().expect("process directory"))
}

/// `options` with the configuration a test supplies for `context`: when the
/// context names a repository, the user-only configuration of its home
/// (`ComposeRequest::with_context` requires a snapshot there and a test's
/// repository may be a synthetic catalog with no committed tree); otherwise
/// `options` unchanged, so the request loads it passively.
pub fn configured(options: ComposeOptions, context: &FileResolutionContext) -> ComposeOptions {
    if options.config_snapshot().is_some() || context.repository_root().is_none() {
        return options;
    }
    options.with_config_snapshot(user_config(context.request_cwd(), context.home_dir(), context.env().clone()))
}

/// The configuration of `home` and no repository, for a test whose
/// repository is a synthetic catalog or caller evidence with no committed
/// tree, which has no committed policy to read.
pub fn user_config(
    dir: &Path,
    home: Option<&Path>,
    env: std::collections::HashMap<String, String>,
) -> Arc<ConfigSnapshot> {
    let snapshot = RequestSnapshot::new(dir).with_home(home.map(Path::to_path_buf)).with_env(env);
    Arc::new(ConfigSnapshot::load_user(&snapshot, ConfigLoadMode::Passive).expect("test configuration"))
}

/// `ComposeRequest::with_context` over [`configured`], [`anchored`] options.
pub fn request_in(options: ComposeOptions, context: FileResolutionContext) -> ComposeRequest {
    let options = configured(anchored(options, &context), &context);
    ComposeRequest::with_context(options, context).expect("test request context")
}

/// `options` anchored on `context`'s request directory when their context is
/// anchored nowhere and holds nothing else ([`ComposeOptions::new`]), as
/// `ComposeRequest::prepare` anchors them. `with_context` discovers the
/// request's repository at that anchor, and an empty one is the process
/// directory, inside this monorepo (~70 ms per request on Linux, seconds on
/// Windows under load).
fn anchored(options: ComposeOptions, context: &FileResolutionContext) -> ComposeOptions {
    let captured = options.context();
    let unanchored = !captured.anchor().is_absolute()
        && matches!(options.context_authority(), ContextAuthority::DarkmatterOwned)
        && captured.capture_requirements().iter().all(|group| group == ContextGroup::DateTime)
        && captured.config_source().is_none();
    if !unanchored {
        return options;
    }
    options
        .with_context(ComposeContext::capture_for_content(context.request_cwd(), ""))
        .with_context_authority(ContextAuthority::DarkmatterOwned)
}

/// The configuration of a user whose home is `home`, with `rules` as the
/// `permissions.shell-commands` of `~/.darkmatter.yaml` (no file is written
/// when there are none), and no repository: what a shell test composes
/// under, so it never reads or writes the process's home.
pub fn shell_config(home: impl AsRef<Path>, rules: &[&str]) -> Arc<ConfigSnapshot> {
    let home = home.as_ref();
    if !rules.is_empty() {
        let mut text = String::from("permissions:\n  shell-commands:\n");
        for rule in rules {
            text.push_str(&format!("    - {}\n", serde_json::to_string(rule).expect("a rule string")));
        }
        std::fs::write(home.join(darkmatter::config::schema::CONFIG_FILE_NAME), text).expect("user configuration");
    }
    user_config(home, Some(home), std::collections::HashMap::new())
}

/// The `permissions.shell-commands` entries, as written (`!` included), of
/// the user configuration under `home`; empty when there is none.
pub fn saved_shell_rules(home: impl AsRef<Path>) -> Vec<String> {
    let snapshot = shell_config(home, &[]);
    let policy = snapshot.policy().expect("valid permissions");
    policy.user().permissions().shell_commands.value.iter().map(|entry| entry.authored.clone()).collect()
}
