//! Request inputs for unit tests.
//!
//! Production code receives its [`RequestSnapshot`] from the `claudine`
//! binary. A unit test stands in for that binary: it reads the test process
//! once, the way the binary would, so a test that never names its inputs
//! still runs against the process it was launched from.

use std::path::Path;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::RequestSnapshot;

/// The test process's directory, home, and environment.
pub(crate) fn snapshot() -> RequestSnapshot {
    RequestSnapshot::from_process().expect("read the test process")
}

/// [`snapshot`] anchored at `dir`.
pub(crate) fn snapshot_at(dir: impl AsRef<Path>) -> RequestSnapshot {
    snapshot().at_request_dir(dir.as_ref())
}

/// A launch context outside any repository (the system temporary directory).
///
/// Its fallback tree admits a test document wherever its fixture lives, as a
/// request launched beside that document would, and building it never walks
/// the checkout the tests run from.
pub(crate) fn context() -> FileResolutionContext {
    context_at(std::env::temp_dir())
}

/// Claudine's launch context for a request launched at `dir`.
pub(crate) fn context_at(dir: impl AsRef<Path>) -> FileResolutionContext {
    crate::composition::capture_file_resolution_context(&snapshot_at(dir))
        .expect("build the test context")
}

/// Claudine's launch context for a request launched in the directory of the
/// document at `path`, as a request with no caller-supplied context resolves
/// that document.
pub(crate) fn context_for(path: impl AsRef<Path>) -> FileResolutionContext {
    let path = path.as_ref();
    context_at(path.parent().unwrap_or(path))
}

/// [`snapshot`] read once, for a test that borrows it for the rest of the
/// test (a lookup that holds a reference).
pub(crate) fn process_snapshot() -> &'static RequestSnapshot {
    static SNAPSHOT: std::sync::OnceLock<RequestSnapshot> = std::sync::OnceLock::new();
    SNAPSHOT.get_or_init(snapshot)
}

/// [`context`] built once, for a test that borrows it.
pub(crate) fn process_context() -> &'static FileResolutionContext {
    static CONTEXT: std::sync::OnceLock<FileResolutionContext> = std::sync::OnceLock::new();
    CONTEXT.get_or_init(context)
}

/// The request context a document at `source_path` resolves its own
/// references in when its request names only `repo_root`: built at the
/// document's directory with that repository's topology. A root that does not
/// contain the document's directory is dropped, as a discovered one would be.
pub(crate) fn document_context(source_path: &Path, repo_root: Option<&Path>) -> FileResolutionContext {
    let base_dir = source_path.parent().unwrap_or(source_path);
    let repo_root = repo_root.filter(|root| base_dir.starts_with(root));
    let repo_info = repo_root
        .and_then(|root| sniff::filesystem::repo::detect_repo_structure(root).ok().flatten());
    crate::composition::build_prompt_resolution_context(&snapshot_at(base_dir), repo_root, repo_info.as_ref())
        .expect("build the document context")
}

const CWD_PROBE_SOURCE: &str = "fn main() {\n    let cwd = std::env::current_dir().unwrap();\n    \
     println!(\"{}\", cwd.display().to_string().replace('\\\\', \"/\"));\n}\n";

/// A program that prints its working directory, `/`-separated; `None` when
/// `rustc` is unavailable. A test uses it where a shell builtin would do,
/// because the shell blacklist rejects every Windows shell that could.
///
/// It prints the portable spelling because its output lands in a Markdown
/// document: a native Windows path would lose the separator of any segment
/// beginning with punctuation to CommonMark backslash escaping.
///
/// Compiling it costs seconds of CPU, so it is compiled once per probe source
/// into the temporary directory and shared by every test and later run.
pub(crate) fn cwd_probe() -> Option<std::path::PathBuf> {
    let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let available = std::process::Command::new(&compiler)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !available {
        return None;
    }
    let key = format!("{:016x}", biscuit_hash::xx_hash_bytes(CWD_PROBE_SOURCE.as_bytes()));
    let dir = std::env::temp_dir().join(format!("claudine-cwd-probe-{key}"));
    std::fs::create_dir_all(&dir).expect("create the probe directory");
    let executable = dir.join(format!("cwd-probe{}", std::env::consts::EXE_SUFFIX));
    let lock = std::fs::File::create(dir.join("lock")).expect("create the probe lock");
    lock.lock().expect("lock the probe");
    if !executable.is_file() {
        let source = dir.join("cwd_probe.rs");
        std::fs::write(&source, CWD_PROBE_SOURCE).expect("write the probe source");
        let compilation = std::process::Command::new(compiler)
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("run rustc");
        assert!(
            compilation.status.success(),
            "failed to compile cwd probe: {}",
            String::from_utf8_lossy(&compilation.stderr)
        );
    }
    Some(executable)
}
