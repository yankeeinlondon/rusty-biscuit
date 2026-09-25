//! Level 2: move-first `wt remove` from a Windows PowerShell console that was
//! launched inside the worktree (spec item 3, acceptance criteria 3 and 7).
//!
//! PowerShell runs in a pseudoconsole (ConPTY, through `xpty`), so it has a
//! real console, and no window is created or focused. It is started with the
//! worktree as its working directory, which is what makes Windows hold the
//! directory: the removal only succeeds if the wrapper moves both the location
//! and `[Environment]::CurrentDirectory` before the handoff call.
//!
//! The scene script records the exit code and both directories to a marker
//! file, then dumps the console's screen buffer (`GetBufferContents`), which is
//! what the assertions on the report and the prompt read. The raw ConPTY stream
//! is used only to see the prompt appear, because ConPTY repaints the screen
//! with cursor movement rather than writing text in order.
//!
//! The held-directory case (acceptance criterion 3, Windows) runs `wt remove`
//! from the base repo while a separate windowless `ping` stands in the
//! worktree. `--force-worktree` approves the dirty and ignored files and the
//! branch is Safe, so only the lock check can stop the removal: the console
//! must show the in-use refusal with exit 4, and every file, the registration,
//! and the branch must survive. Without that check `git worktree remove`
//! deletes the files before it fails.
#![cfg(windows)]

use std::fs;
use std::io::{Read as _, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use biscuit_test_harness::{bin_exe, strip_ansi};
use test_toolkit::{Level, require_level};
use worktree::remove::handoff::canonical;
use xpty::{Child, CommandBuilder, MasterPty, PtySize, PtySystem as _, native_pty_system};

/// Each wait stays well under nextest's 30 s termination, so a stalled step
/// fails with the console stream instead of a bare TIMEOUT.
const STEP: Duration = Duration::from_secs(15);

const SCREEN: PtySize = PtySize {
    rows: 60,
    cols: 120,
    pixel_width: 0,
    pixel_height: 0,
};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("git should be installed");
    assert!(
        out.status.success(),
        "git {args:?} in {dir:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn powershell_available() -> bool {
    Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "exit 0"])
        .output()
        .is_ok_and(|out| out.status.success())
}

/// A PowerShell single-quoted string literal.
fn ps_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "''"))
}

/// A PowerShell session in a pseudoconsole; killed and closed on drop.
struct Console {
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output: Arc<Mutex<Vec<u8>>>,
    _master: Box<dyn MasterPty + Send>,
}

impl Console {
    fn launch(cwd: &Path) -> Self {
        let pair = native_pty_system().openpty(SCREEN).expect("open a pseudoconsole");
        let mut cmd = CommandBuilder::new("powershell.exe");
        cmd.args(["-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass"]);
        cmd.cwd(cwd);
        cmd.env_remove("CI");
        cmd.env_remove("WT_SHELL_WRAPPER");
        cmd.env("NO_COLOR", "1");
        let child = pair.slave.spawn_command(cmd).expect("start powershell.exe");
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().expect("pseudoconsole reader");
        let writer = pair.master.take_writer().expect("pseudoconsole writer");
        let output = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&output);
        // Draining continuously also keeps ConPTY from blocking the child
        // (and `ClosePseudoConsole`) on a full output pipe.
        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            while let Ok(n) = reader.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                sink.lock().unwrap().extend_from_slice(&chunk[..n]);
            }
        });
        Self {
            child,
            writer,
            output,
            _master: pair.master,
        }
    }

    fn stream(&self) -> String {
        strip_ansi(&String::from_utf8_lossy(&self.output.lock().unwrap()))
    }

    fn wait_for(&self, needle: &str) {
        let deadline = Instant::now() + STEP;
        while !self.stream().contains(needle) {
            assert!(
                Instant::now() < deadline,
                "{needle:?} never appeared; console stream:\n{}",
                self.stream()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn type_text(&mut self, text: &str) {
        self.writer.write_all(text.as_bytes()).expect("write to the console");
        self.writer.flush().expect("flush the console");
    }

    /// Types `exit` and waits for PowerShell to end.
    fn exit(&mut self) {
        self.type_text("exit\r");
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("PowerShell did not exit; console stream:\n{}", self.stream());
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// A windowless process whose current directory is the held directory;
/// killed on drop so the lock is released even when an assertion fails.
///
/// `ping` is spawned itself, not through `cmd /C`: killing `cmd` would leave
/// `ping` holding the directory (the `os` skill's current-directory locks).
struct Holder(std::process::Child);

impl Holder {
    /// Returns once the process has written output, so its current directory
    /// is already set up.
    fn start(dir: &Path) -> Self {
        use std::os::windows::process::CommandExt as _;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut child = Command::new("ping")
            .args(["-n", "120", "127.0.0.1"])
            .current_dir(dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("start ping");
        let mut stdout = child.stdout.take().expect("ping's stdout");
        let (started, first_output) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut chunk = [0u8; 512];
            let mut sent = false;
            while let Ok(n) = stdout.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                if !sent {
                    let _ = started.send(());
                    sent = true;
                }
            }
        });
        let holder = Self(child);
        first_output
            .recv_timeout(STEP)
            .expect("the holding process never started");
        holder
    }
}

impl Drop for Holder {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Scene {
    root: tempfile::TempDir,
}

struct Outcome {
    code: i32,
    location: PathBuf,
    current_directory: PathBuf,
    /// The console screen buffer, one line per row, trailing blanks trimmed.
    screen: String,
}

impl Scene {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        fs::create_dir_all(repo.join("docs")).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        for (key, value) in [
            ("user.email", "test@example.com"),
            ("user.name", "Test User"),
            ("commit.gpgsign", "false"),
            ("gc.auto", "0"),
        ] {
            git(&repo, &["config", key, value]);
        }
        fs::write(repo.join("docs/guide.md"), "guide\n").unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-q", "-m", "initial"]);
        Self { root }
    }

    fn repo(&self) -> PathBuf {
        self.root.path().join("repo")
    }

    fn add_worktree(&self, branch: &str, dir_name: &str) -> PathBuf {
        let path = self.root.path().join("wts").join(dir_name);
        git(&self.repo(), &["worktree", "add", "-q", "-b", branch, path.to_str().unwrap(), "main"]);
        path
    }

    fn record_fork(&self, branch: &str, parent: &str) {
        let store = worktree::fork_origin::fork_origin_path(&self.repo()).unwrap();
        worktree::fork_origin::record(
            &store,
            branch,
            worktree::fork_origin::ForkOrigin {
                base_branch: parent.to_string(),
                base_sha: git(&self.repo(), &["rev-parse", parent]),
                created_at: 0,
            },
        )
        .unwrap();
    }

    fn branch_exists(&self, branch: &str) -> bool {
        Command::new("git")
            .current_dir(self.repo())
            .args(["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")])
            .status()
            .unwrap()
            .success()
    }

    fn registered(&self, path: &Path) -> bool {
        let wanted = canonical(path);
        git(&self.repo(), &["worktree", "list", "--porcelain"])
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .any(|listed| canonical(Path::new(listed)) == wanted)
    }

    fn marker(&self) -> PathBuf {
        self.root.path().join("marker.txt")
    }

    fn screen_file(&self) -> PathBuf {
        self.root.path().join("screen.txt")
    }

    /// Writes the wrapper and a scene script that runs `command` through it
    /// and records the outcome; returns the script's path.
    fn script(&self, command: &str) -> PathBuf {
        let wrapper = self.root.path().join("wrapper.ps1");
        let generated = Command::new(bin_exe!("wt"))
            .args(["--completions", "powershell"])
            .output()
            .unwrap();
        assert!(generated.status.success(), "wt --completions powershell failed");
        fs::write(&wrapper, generated.stdout).unwrap();
        let script = self.root.path().join("scene.ps1");
        let body = format!(
            ". {wrapper}\r\n\
            $global:LASTEXITCODE = 0\r\n\
            {command}\r\n\
            $rc = $global:LASTEXITCODE\r\n\
            $raw = $Host.UI.RawUI\r\n\
            $size = $raw.BufferSize\r\n\
            $rect = New-Object System.Management.Automation.Host.Rectangle 0, 0, ($size.Width - 1), ($size.Height - 1)\r\n\
            $cells = $raw.GetBufferContents($rect)\r\n\
            $rows = for ($y = 0; $y -lt $size.Height; $y++) {{\r\n\
            \x20   $line = New-Object System.Text.StringBuilder\r\n\
            \x20   for ($x = 0; $x -lt $size.Width; $x++) {{ [void]$line.Append($cells.GetValue($y, $x).Character) }}\r\n\
            \x20   $line.ToString().TrimEnd()\r\n\
            }}\r\n\
            [IO.File]::WriteAllLines({screen}, [string[]]$rows, [Text.UTF8Encoding]::new($false))\r\n\
            $record = \"rc=$rc`nlocation=$((Get-Location).ProviderPath)`ncwd=$([Environment]::CurrentDirectory)`n\"\r\n\
            [IO.File]::WriteAllText({pending}, $record, [Text.UTF8Encoding]::new($false))\r\n\
            Move-Item -LiteralPath {pending} -Destination {marker}\r\n",
            wrapper = ps_quote(&wrapper),
            screen = ps_quote(&self.screen_file()),
            pending = ps_quote(&self.root.path().join("marker.pending")),
            marker = ps_quote(&self.marker()),
        );
        fs::write(&script, body).unwrap();
        script
    }

    fn wait_outcome(&self, console: &Console) -> Outcome {
        let deadline = Instant::now() + STEP;
        let text = loop {
            if let Ok(text) = fs::read_to_string(self.marker()) {
                break text;
            }
            assert!(
                Instant::now() < deadline,
                "the scene never finished; console stream:\n{}",
                console.stream()
            );
            std::thread::sleep(Duration::from_millis(50));
        };
        let value = |key: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(key))
                .unwrap_or_default()
                .to_string()
        };
        Outcome {
            code: value("rc=").parse().unwrap_or(-1),
            location: PathBuf::from(value("location=")),
            current_directory: PathBuf::from(value("cwd=")),
            screen: fs::read_to_string(self.screen_file()).expect("the screen dump"),
        }
    }

    /// Launches PowerShell inside `cwd`, runs `wt remove <name>` through the
    /// wrapper, waits for the files question, and answers `answer`.
    fn remove_from_inside(&self, cwd: &Path, name: &str, answer: &str) -> Outcome {
        let script = self.script(&format!("wt remove {name}"));
        let mut console = Console::launch(cwd);
        console.wait_for("PS ");
        console.type_text(&format!(". {}\r", ps_quote(&script)));
        console.wait_for("Discard");
        console.type_text(answer);
        let outcome = self.wait_outcome(&console);
        console.exit();
        outcome
    }

    /// Launches PowerShell in the base repo and runs `command` through the
    /// wrapper; no prompt is expected.
    fn run_from_base(&self, command: &str) -> Outcome {
        let script = self.script(command);
        let mut console = Console::launch(&self.repo());
        console.wait_for("PS ");
        console.type_text(&format!(". {}\r", ps_quote(&script)));
        let outcome = self.wait_outcome(&console);
        console.exit();
        outcome
    }
}

/// The screen as one line: a full-width row continues on the next row
/// without a break, and runs of whitespace become one space.
fn unwrapped(screen: &str) -> String {
    let mut joined = String::new();
    for row in screen.lines() {
        joined.push_str(row);
        if row.chars().count() < usize::from(SCREEN.cols) {
            joined.push(' ');
        }
    }
    joined.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Drop for Scene {
    fn drop(&mut self) {
        if let Ok(path) = worktree::fork_origin::fork_origin_path(&self.repo()) {
            let _ = fs::remove_file(path);
        }
    }
}

/// Asserts the line holding `needle` follows exactly one blank line, which
/// itself follows text.
fn assert_one_blank_line_before(screen: &str, needle: &str) {
    let lines: Vec<&str> = screen.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle:?} not on screen:\n{screen}"));
    assert!(at >= 2, "{needle:?} too close to the top:\n{screen}");
    assert!(lines[at - 1].trim().is_empty(), "no blank line before {needle:?}:\n{screen}");
    assert!(!lines[at - 2].trim().is_empty(), "more than one blank line before {needle:?}:\n{screen}");
}

fn assert_removed(scene: &Scene, wt: &Path, branch: &str, screen: &str) {
    assert!(!wt.exists(), "the worktree directory is gone:\n{screen}");
    assert!(!scene.registered(wt), "the worktree is deregistered:\n{screen}");
    assert!(!scene.branch_exists(branch), "the branch is on main, so Safe, and deleted:\n{screen}");
}

#[test]
fn level2_powershell_launched_inside_the_worktree_moves_to_the_base_repo_and_removes_it() {
    require_level!(Level::L2, powershell_available(), "ConPTY (Windows PowerShell)");
    let scene = Scene::new();
    let wt = scene.add_worktree("feat/x", "feat-x");
    fs::write(wt.join("notes.txt"), "draft\n").unwrap();

    let outcome = scene.remove_from_inside(&wt, "feat-x", "y\r");
    let screen = &outcome.screen;

    assert_eq!(outcome.code, 0, "{screen}");
    assert!(screen.contains("Uncommitted files (1):"), "{screen}");
    assert!(screen.contains("notes.txt"), "{screen}");
    assert_one_blank_line_before(screen, "Discard the files listed above");
    assert!(screen.contains("Moving you to the base repo"), "{screen}");
    assert!(screen.contains("Removed worktree"), "{screen}");
    assert!(screen.contains("You are now in"), "{screen}");
    assert_eq!(canonical(&outcome.location), canonical(&scene.repo()), "{screen}");
    assert_eq!(canonical(&outcome.current_directory), canonical(&scene.repo()), "{screen}");
    assert_removed(&scene, &wt, "feat/x", screen);
}

#[test]
fn level2_powershell_launched_inside_a_subdirectory_lands_in_the_fork_parent() {
    require_level!(Level::L2, powershell_available(), "ConPTY (Windows PowerShell)");
    let scene = Scene::new();
    let parent = scene.add_worktree("feat/theme", "feat-theme");
    let wt = scene.add_worktree("feat/x", "feat-x");
    scene.record_fork("feat/x", "feat/theme");
    fs::write(wt.join("notes.txt"), "draft\n").unwrap();

    let outcome = scene.remove_from_inside(&wt.join("docs"), "feat-x", "y\r");
    let screen = &outcome.screen;

    assert_eq!(outcome.code, 0, "{screen}");
    assert_one_blank_line_before(screen, "Discard the files listed above");
    assert!(screen.contains("Moving you to the feat/theme worktree"), "{screen}");
    assert!(screen.contains("Removed worktree"), "{screen}");
    assert!(screen.contains("You are now in"), "{screen}");
    let landing = canonical(&parent.join("docs"));
    assert_eq!(canonical(&outcome.location), landing, "{screen}");
    assert_eq!(canonical(&outcome.current_directory), landing, "{screen}");
    assert_removed(&scene, &wt, "feat/x", screen);
    assert!(parent.exists(), "the fork parent is untouched");
}

#[test]
fn level2_powershell_refuses_a_worktree_another_program_holds_with_exit_4_and_nothing_removed() {
    require_level!(Level::L2, powershell_available(), "ConPTY (Windows PowerShell)");
    let scene = Scene::new();
    let wt = scene.add_worktree("feat/x", "feat-x");
    fs::create_dir_all(scene.repo().join(".git/info")).unwrap();
    fs::write(scene.repo().join(".git/info/exclude"), ".env\n").unwrap();
    fs::write(wt.join("notes.txt"), "draft\n").unwrap();
    fs::write(wt.join(".env"), "SECRET=1\n").unwrap();
    let status = git(&wt, &["status", "--porcelain", "--ignored"]);
    assert!(status.contains("?? notes.txt"), "{status}");
    assert!(status.contains("!! .env"), "{status}");
    // Dropped before `scene`, so the lock is gone before the tempdir cleanup.
    let _holder = Holder::start(&wt);

    let outcome = scene.run_from_base("wt remove feat-x --force-worktree");
    let screen = &outcome.screen;
    let text = unwrapped(screen);

    assert_eq!(outcome.code, 4, "{screen}");
    assert!(text.contains("Error: the folder"), "{screen}");
    assert!(text.contains("feat-x is in use by another program"), "{screen}");
    assert!(!text.contains("Discard"), "--force-worktree skips the question:\n{screen}");
    assert!(!text.contains("Removed"), "{screen}");
    // The checkout follows the host's `core.autocrlf`.
    assert_eq!(
        fs::read_to_string(wt.join("docs/guide.md")).unwrap().replace("\r\n", "\n"),
        "guide\n",
        "tracked file kept"
    );
    assert_eq!(fs::read_to_string(wt.join("notes.txt")).unwrap(), "draft\n", "dirty file kept");
    assert_eq!(fs::read_to_string(wt.join(".env")).unwrap(), "SECRET=1\n", "ignored file kept");
    assert!(scene.registered(&wt), "still in `git worktree list`:\n{screen}");
    assert!(scene.branch_exists("feat/x"), "the branch is kept:\n{screen}");
}
