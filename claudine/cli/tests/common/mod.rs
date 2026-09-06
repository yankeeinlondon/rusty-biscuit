#![allow(dead_code)]

pub(crate) mod completion;
#[cfg(unix)]
pub(crate) mod pty;
pub(crate) mod wrap;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEST_NONCE: AtomicU64 = AtomicU64::new(0);

/// Initialize the workspace tracing subscriber for this test binary so
/// `trace_phase!` spans become visible without setting `RUST_LOG` manually.
///
/// Idempotent (`test_toolkit::init_test_tracing` uses an internal `Once`),
/// so all helpers below can call it cheaply on every invocation.
fn ensure_test_tracing_initialized() {
    test_toolkit::init_test_tracing();
}

/// Public re-export so individual tests that bypass the helpers below can
/// still opt-in explicitly.
#[allow(unused_imports)]
pub use test_toolkit::init_test_tracing;

/// Absolute path to the `claudine` binary built for this test binary.
///
/// Resolved at run time rather than with `env!`, so a suite executed from a
/// relocated `cargo nextest archive` (the `wsl2-ubuntu` leg) still finds the
/// binary — see [`biscuit_test_harness::bin_exe`]. Returned as `&str` because
/// most call sites interpolate it into a shell command line.
pub fn claudine_bin() -> &'static str {
    static BIN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    BIN.get_or_init(|| {
        biscuit_test_harness::bin_exe!("claudine")
            .to_string_lossy()
            .into_owned()
    })
}

pub struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    pub fn new() -> Self {
        Self::named("claudine-test")
    }

    pub fn named(prefix: &str) -> Self {
        ensure_test_tracing_initialized();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let unique = TEST_NONCE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("{prefix}-{}-{nonce}-{unique}", process::id()));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Hermetic defaults for tests that execute the shipped `claudine` binary.
///
/// Repository topology, prompts, provider memory, and user configuration are
/// absent until a test writes them into the fixture explicitly.
pub struct CliProcessFixture {
    workspace: TestWorkspace,
    cwd: PathBuf,
    home: PathBuf,
    bin_dir: PathBuf,
}

impl CliProcessFixture {
    pub fn named(prefix: &str) -> Self {
        let workspace = TestWorkspace::named(prefix);
        let cwd = workspace.path().join("cwd");
        let home = workspace.path().join("home");
        let bin_dir = workspace.path().join("bin");
        for path in [&cwd, &home, &bin_dir] {
            fs::create_dir_all(path).unwrap();
        }
        Self {
            workspace,
            cwd,
            home,
            bin_dir,
        }
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }

    pub fn initialize_repository(&self) {
        assert!(
            init_git_repo(&self.cwd),
            "failed to initialize fixture repository at {}",
            self.cwd.display()
        );
    }

    pub fn seed_user_config(&self) {
        wrap::seed_minimal_config(&self.home);
    }

    pub fn write_root_system_prompt(&self, content: &str) -> PathBuf {
        let path = self.cwd.join("system-prompt.md");
        write(&path, content);
        path
    }

    pub fn write_user_system_prompt(&self, content: &str) -> PathBuf {
        let path = self.home.join(".claudine/system-prompt.md");
        write(&path, content);
        path
    }

    pub fn write_repo_appendix(&self, content: &str) -> PathBuf {
        let path = self.cwd.join(".claudine/non-interactive.md");
        write(&path, content);
        path
    }

    pub fn write_provider_memory(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.cwd.join(relative);
        write(&path, content);
        path
    }

    pub fn command(&self) -> assert_cmd::Command {
        let path = std::env::join_paths([self.bin_dir.as_path()])
            .expect("fake-only PATH should contain one valid path");
        let mut command = assert_cmd::Command::cargo_bin("claudine").unwrap();
        command
            .current_dir(&self.cwd)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env_remove("HOMEDRIVE")
            .env_remove("HOMEPATH")
            .env_remove("XDG_CONFIG_HOME")
            .env("APPDATA", &self.home)
            .env("LOCALAPPDATA", &self.home)
            .env("PATH", path)
            .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
            .env("NO_COLOR", "1");
        command
    }
}

pub fn write(path: &Path, content: &str) {
    ensure_test_tracing_initialized();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

pub fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    write(path, &serde_json::to_string_pretty(value).unwrap());
}

pub fn init_git_repo(path: &Path) -> bool {
    ensure_test_tracing_initialized();
    Command::new("git")
        .arg("init")
        .current_dir(path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn augmented_path(fake_bin: &Path) -> std::ffi::OsString {
    ensure_test_tracing_initialized();
    let system_path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths: Vec<PathBuf> = vec![fake_bin.to_path_buf()];
    paths.extend(std::env::split_paths(&system_path));
    std::env::join_paths(paths).expect("join_paths")
}

/// Clear an inherited `NO_COLOR` from a pane's interactive shell before a
/// color fixture runs in it.
///
/// `send_command_with_env` cannot express this: it renders env as `VAR='' cmd`,
/// and claudine treats `NO_COLOR` as *present* rather than as truthy
/// (`cli/src/log.rs` — `colors_disabled()` short-circuits `force_color_enabled()`),
/// so an empty value still disables color and `FORCE_COLOR=1` cannot out-vote
/// it. A host that exports `NO_COLOR=1` — as review environments legitimately
/// do — would otherwise turn every L2 color assertion into a false failure, or
/// worse, a vacuous pass.
///
/// The unset lands in the shell the caller then drives, so it survives any
/// `clear`/`cd` sent afterwards.
pub fn clear_no_color<H: biscuit_test_harness::TerminalHarness>(harness: &mut H) {
    harness
        .send_text(b"unset NO_COLOR\n")
        .expect("unset NO_COLOR");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
}

/// Assert that the captured pane row carrying `needle` is itself styled.
///
/// The weaker `frame.raw.contains('\u{1b}')` this replaces is satisfied by *any*
/// escape anywhere in the pane — a colored shell prompt, or tmux's own
/// re-emission — so it can hold on a run that rendered the diagnostic with no
/// styling at all. Anchoring on the row that carries the diagnostic text makes
/// the assertion fail when the thing under test loses its color.
pub fn assert_row_is_styled(raw: &str, needle: &str, what: &str) {
    let row = raw
        .lines()
        .find(|line| strip_ansi(line).contains(needle))
        .unwrap_or_else(|| panic!("no captured row contains {needle:?}.\nraw:\n{raw}"));
    assert!(
        row.contains('\u{1b}'),
        "{what}: the row carrying {needle:?} reached the pane unstyled.\nrow: {row:?}"
    );
}

pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.peek() {
                Some(&'[') => {
                    chars.next();
                    for code in chars.by_ref() {
                        if ('@'..='~').contains(&code) {
                            break;
                        }
                    }
                }
                Some(&']') => {
                    chars.next();
                    for code in chars.by_ref() {
                        if code == '\u{7}' {
                            break;
                        }
                        if code == '\u{1b}' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        out.push(ch);
    }

    out
}

/// Probe for a usable pseudo-terminal on the host.
///
/// On Unix, attempts to open `/dev/ptmx` (the master multiplexer). Returns
/// `true` when allocation succeeds, which is the precondition every
/// `expectrl::Session::spawn` call in this crate's L2 PTY tests requires.
/// On non-Unix targets returns `false` (the PTY test files are themselves
/// `#[cfg(unix)]`, so this branch is unreachable from those tests).
#[allow(dead_code)]
pub fn pty_available() -> bool {
    #[cfg(unix)]
    {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/ptmx")
            .is_ok()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub fn write_executable(path: &Path, content: &str) {
    ensure_test_tracing_initialized();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        write(path, content);
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
    #[cfg(not(unix))]
    {
        write(path, content);
    }
}

/// Wrap `value` in a POSIX single-quoted word, escaping embedded quotes.
#[cfg(unix)]
pub fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// POSIX-shell fragment that replaces `$CLAUDINE_DOC`'s body with
/// `$CLAUDINE_BODY`, inserting `$CLAUDINE_ADD` before the closing delimiter.
///
/// Builtins only. Inline fixtures routinely set `PATH` to the stub directory
/// alone, so a fragment that reached for `cat`, `awk`, or `mv` would silently
/// do nothing and look like an agent that refused the task.
#[cfg(unix)]
pub const INLINE_BODY_REWRITE: &str = concat!(
    "CLAUDINE_OUT=''\n",
    "CLAUDINE_N=0\n",
    "while IFS= read -r CLAUDINE_LINE || [ -n \"$CLAUDINE_LINE\" ]; do\n",
    "  if [ \"$CLAUDINE_LINE\" = '---' ] && [ \"$CLAUDINE_N\" -lt 2 ]; then\n",
    "    CLAUDINE_N=$((CLAUDINE_N + 1))\n",
    "    if [ \"$CLAUDINE_N\" -eq 2 ]; then CLAUDINE_OUT=\"$CLAUDINE_OUT$CLAUDINE_ADD\"; fi\n",
    "    CLAUDINE_OUT=\"$CLAUDINE_OUT$CLAUDINE_LINE\n\"\n",
    "    continue\n",
    "  fi\n",
    "  if [ \"$CLAUDINE_N\" -lt 2 ]; then CLAUDINE_OUT=\"$CLAUDINE_OUT$CLAUDINE_LINE\n\"; fi\n",
    "done < \"$CLAUDINE_DOC\"\n",
    "printf '%s%s' \"$CLAUDINE_OUT\" \"$CLAUDINE_BODY\" > \"$CLAUDINE_DOC\"\n",
);

/// POSIX-shell fragment that recovers the active document from the delivered
/// prompt header into `$CLAUDINE_DOC`, the same way a real agent learns it.
///
/// For a fixture whose active document is not known when the stub is written —
/// a sequence task, or a proxied target. Scans argv first, then stdin, because
/// prompt delivery is per provider and per session mode.
#[cfg(unix)]
pub const INLINE_DOC_FROM_PROMPT: &str = concat!(
    "CLAUDINE_DOC=''\n",
    "for CLAUDINE_ARG in \"$@\"; do\n",
    "  case \"$CLAUDINE_ARG\" in\n",
    "    *'**Document:** `'*)\n",
    "      CLAUDINE_DOC=\"${CLAUDINE_ARG#*'**Document:** `'}\"\n",
    "      CLAUDINE_DOC=\"${CLAUDINE_DOC%%\\`*}\"\n",
    "      ;;\n",
    "  esac\n",
    "done\n",
    "if [ -z \"$CLAUDINE_DOC\" ]; then\n",
    "  while IFS= read -r CLAUDINE_ARG; do\n",
    "    case \"$CLAUDINE_ARG\" in\n",
    "      *'**Document:** `'*)\n",
    "        CLAUDINE_DOC=\"${CLAUDINE_ARG#*'**Document:** `'}\"\n",
    "        CLAUDINE_DOC=\"${CLAUDINE_DOC%%\\`*}\"\n",
    "        break\n",
    "        ;;\n",
    "    esac\n",
    "  done\n",
    "fi\n",
);

/// A provider stub that behaves like a file-aware inline agent.
///
/// Under the 2026-09-05 inline contract the agent *is* the writer: it edits the
/// active document and returns a short summary, and Claudine reads the file
/// back. A stub that only prints to stdout is a stub that did no work, so every
/// inline fixture builds its script here — the body is rewritten in place, the
/// authored frontmatter is preserved byte-for-byte, and only the declared
/// additions are inserted.
///
/// Every field is embedded as a POSIX single-quoted word, so any content is
/// safe. `prelude` is emitted verbatim and must end with a newline when set.
#[cfg(unix)]
pub struct InlineAgentStub<'a> {
    document: &'a Path,
    prelude: &'a str,
    frontmatter_additions: &'a str,
    body: &'a str,
    body_is_literal: bool,
    summary: &'a str,
    exit_code: i32,
}

#[cfg(unix)]
impl<'a> InlineAgentStub<'a> {
    /// An agent that replaces `document`'s body and reports one line.
    pub fn new(document: &'a Path) -> Self {
        Self {
            document,
            prelude: "",
            frontmatter_additions: "",
            body: "Agent body\n",
            body_is_literal: true,
            summary: "Wrote the requested content.",
            exit_code: 0,
        }
    }

    /// Shell run before the edit — argv capture, run counters, sentinels.
    pub fn prelude(mut self, prelude: &'a str) -> Self {
        self.prelude = prelude;
        self
    }

    /// Frontmatter lines inserted immediately before the closing delimiter.
    pub fn frontmatter_additions(mut self, additions: &'a str) -> Self {
        self.frontmatter_additions = additions;
        self
    }

    /// The body the agent writes.
    pub fn body(mut self, body: &'a str) -> Self {
        self.body = body;
        self.body_is_literal = true;
        self
    }

    /// The body as a shell word the caller quotes itself, so a fixture whose
    /// body must vary per run can interpolate a variable set in the prelude.
    pub fn body_expression(mut self, expression: &'a str) -> Self {
        self.body = expression;
        self.body_is_literal = false;
        self
    }

    /// The final response the agent returns to the caller.
    pub fn summary(mut self, summary: &'a str) -> Self {
        self.summary = summary;
        self
    }

    /// Exit with `code` after the edit.
    pub fn exit_code(mut self, code: i32) -> Self {
        self.exit_code = code;
        self
    }

    /// Render the `/bin/sh` script.
    pub fn script(&self) -> String {
        let document = sh_quote(&self.document.display().to_string());
        let additions = sh_quote(self.frontmatter_additions);
        let body = if self.body_is_literal {
            sh_quote(self.body)
        } else {
            self.body.to_string()
        };
        let summary = sh_quote(self.summary);
        let prelude = &self.prelude;
        let exit_code = self.exit_code;
        format!(
            "#!/bin/sh\n\
             {prelude}\
             CLAUDINE_DOC={document}\n\
             CLAUDINE_ADD={additions}\n\
             CLAUDINE_BODY={body}\n\
             {INLINE_BODY_REWRITE}\
             printf '%s\\n' {summary}\n\
             exit {exit_code}\n"
        )
    }

    /// Write the script to `bin_dir/binary` as an executable.
    pub fn install(&self, bin_dir: &Path, binary: &str) {
        write_executable(&bin_dir.join(binary), &self.script());
    }
}

pub fn write_dry_run_provider_stub(bin_dir: &Path, binary: &str) {
    ensure_test_tracing_initialized();
    #[cfg(unix)]
    {
        write_executable(
            &bin_dir.join(binary),
            r#"#!/bin/sh
echo "SHOULD NOT RUN"
exit 1
"#,
        );
    }
    #[cfg(windows)]
    {
        write(
            &bin_dir.join(format!("{binary}.cmd")),
            "@echo off\r\necho SHOULD NOT RUN\r\nexit /b 1\r\n",
        );
    }
}
