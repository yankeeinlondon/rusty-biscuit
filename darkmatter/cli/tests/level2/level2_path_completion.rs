//! Real Tab presses in real shells: `style.pa<Tab>` through `ce<Tab>` to
//! `style.page.alignment=center`, then Enter.
//!
//! The Level-1 tests prove the words `md` receives and prints. Only a shell can
//! prove what it does with them: whether readline replaces the right piece, and
//! whether a space lands after a word that was meant to continue. Each test owns a
//! detached tmux session with a fixed environment, so no window takes focus and no
//! developer configuration reaches the shell.
//!
//! A stray space at any step shows up in the finished line, which is asserted in
//! full. A shell this host does not have skips through `require_level!`, as a missing
//! tmux does, so `BISCUIT_TEST_LEVEL_REQUIRED=2` makes it fail. PowerShell runs in a
//! windowless pseudoconsole instead of tmux, on native Windows only.

use crate::common;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use common::CliProcessFixture;
use common::level2::{link_or_copy, md_bin};
use std::process::Command;
use std::time::{Duration, Instant};
use test_toolkit::{Backend, Level, require_level};

const DOCUMENT: &str = "---\nstyle:\n  page:\n    alignment: left\n---\nAlign: {{ style.page.alignment }}!\n";
const WAIT: Duration = Duration::from_secs(20);

/// Waits until a line of the pane ends with `expected`. `capture-pane` trims trailing
/// spaces, so this alone cannot see a stray one; the finished line can.
fn wait_for_line(harness: &mut TmuxHarness, expected: &str) {
    let deadline = Instant::now() + WAIT;
    let mut last = String::new();
    while Instant::now() < deadline {
        last = harness.capture().expect("capture").plain;
        if last.lines().any(|line| line.trim_end().ends_with(expected)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("no line ended with `{expected}` after {WAIT:?}. Pane:\n{last}");
}

fn type_text(harness: &mut TmuxHarness, text: &str) {
    harness.send_text(text.as_bytes()).expect("send text");
}

fn press_tab(harness: &mut TmuxHarness) {
    harness.send_key("Tab").expect("send Tab");
}

fn press_enter(harness: &mut TmuxHarness) {
    harness.send_key("Enter").expect("send Enter");
}

/// Runs one line at the shell's prompt and waits until it has finished.
fn run_line(harness: &mut TmuxHarness, line: &str, ready: &str) {
    type_text(harness, line);
    press_enter(harness);
    wait_for_line(harness, ready);
}

/// A prompt in `program` inside a fresh fixture holding the document, with `md`'s
/// script loaded by `setup`. The fixture owns the shell's home, so it outlives the pane.
fn open_shell(program: &str, flags: &[&str], setup: impl FnOnce(&CliProcessFixture) -> String) -> (CliProcessFixture, TmuxHarness) {
    let fixture = CliProcessFixture::named("level2-path-completion");
    std::fs::write(fixture.cwd().join("doc.md"), DOCUMENT).expect("write document");
    let md = fixture.bin_dir().join("md");
    link_or_copy(md_bin(), &md).expect("link md");

    let path = format!("PATH={}:/usr/bin:/bin", fixture.bin_dir().display());
    let home = format!("HOME={}", fixture.home().display());
    let mut arguments = vec!["-i", &path, &home, "TERM=xterm-256color", "LANG=C.UTF-8", program];
    arguments.extend_from_slice(flags);

    let mut harness = TmuxHarness::new();
    harness.spawn_program("/usr/bin/env", &arguments).expect("spawn shell");
    run_line(&mut harness, &format!("cd '{}'; echo in''place", fixture.cwd().display()), "inplace");
    run_line(&mut harness, &format!("{}; echo set''up", setup(&fixture)), "setup");
    (fixture, harness)
}

/// The Tab sequence, in `program`, loading `md`'s script with `setup`.
fn assert_round_trip(program: &str, flags: &[&str], setup: &str) {
    let (_fixture, mut harness) = open_shell(program, flags, |_| setup.to_string());

    type_text(&mut harness, "md compose doc.md style.pa");
    press_tab(&mut harness);
    wait_for_line(&mut harness, "md compose doc.md style.page");
    type_text(&mut harness, ".");
    press_tab(&mut harness);
    wait_for_line(&mut harness, "md compose doc.md style.page.");
    type_text(&mut harness, "al");
    press_tab(&mut harness);
    wait_for_line(&mut harness, "md compose doc.md style.page.alignment=");
    type_text(&mut harness, "ce");
    press_tab(&mut harness);
    wait_for_line(&mut harness, "md compose doc.md style.page.alignment=center");

    type_text(&mut harness, "zz=1");
    wait_for_line(&mut harness, "md compose doc.md style.page.alignment=center zz=1");
    press_enter(&mut harness);
    wait_for_line(&mut harness, "Align: center!");
}

fn bash_major(program: &str) -> Option<u32> {
    let output = Command::new(program).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let version = text.split("version ").nth(1)?;
    version.split('.').next()?.parse().ok()
}

/// The first bash on the usual paths whose major version `accepts`.
fn find_bash(accepts: impl Fn(u32) -> bool) -> Option<&'static str> {
    ["/opt/homebrew/bin/bash", "/usr/local/bin/bash", "/usr/bin/bash", "/bin/bash"]
        .into_iter()
        .find(|program| bash_major(program).is_some_and(&accepts))
}

const BASH_SETUP: &str = "eval \"$(COMPLETE=bash md)\"";
const BASH_FLAGS: &[&str] = &["--noprofile", "--norc", "-i"];

#[test]
fn a_tab_round_trip_in_modern_bash() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let bash = find_bash(|major| major >= 4);
    require_level!(Level::L2, bash.is_some(), "bash 4 or later");
    assert_round_trip(bash.expect("gated above"), BASH_FLAGS, BASH_SETUP);
}

/// Bash 3.2 keeps `x=ce` as one word and has no `compopt`, which is why it has its
/// own test; in practice it is the `/bin/bash` of every Mac.
#[test]
fn a_tab_round_trip_in_bash_3() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let bash = find_bash(|major| major == 3);
    require_level!(Level::L2, bash.is_some(), "bash 3");
    assert_round_trip(bash.expect("gated above"), BASH_FLAGS, BASH_SETUP);
}

fn installed(program: &str) -> Option<String> {
    ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"]
        .into_iter()
        .map(|directory| format!("{directory}/{program}"))
        .find(|candidate| std::path::Path::new(candidate).exists())
}

#[test]
fn a_tab_round_trip_in_zsh() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let zsh = installed("zsh");
    require_level!(Level::L2, zsh.is_some(), "zsh");
    assert_round_trip(&zsh.expect("gated above"), &["-f", "-i"], "autoload -Uz compinit; compinit -u -D; eval \"$(COMPLETE=zsh md)\"");
}

#[test]
fn a_tab_round_trip_in_fish() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let fish = installed("fish");
    require_level!(Level::L2, fish.is_some(), "fish");
    assert_round_trip(&fish.expect("gated above"), &["--no-config", "-i"], "COMPLETE=fish md | source");
}

/// The line a person is told to add to their startup file.
fn printed_setup(fixture: &CliProcessFixture, shell: &str) -> String {
    let output = fixture.command().args(["--completions", shell]).output().expect("run md --completions");
    assert!(output.status.success(), "{output:?}");
    let text = String::from_utf8(output.stdout).expect("setup text");
    text.lines().last().expect("a setup line").to_string()
}

/// The default prompts show the directory and, at the right edge, the host, either of
/// which would hide the end of the line.
const ELVISH_PROMPTS: &str = "set edit:prompt = { put '> ' }; set edit:rprompt = { }";
const ELVISH_LIST: &str = "COMPLETING argument";

fn open_elvish(elvish: &str) -> (CliProcessFixture, TmuxHarness) {
    open_shell(elvish, &["-norc"], |fixture| format!("{}; {ELVISH_PROMPTS}", printed_setup(fixture, "elvish")))
}

/// Elvish's Tab opens a list and keeps it open even for one candidate, and Enter
/// leaves it with the word still open, so unlike the other shells each Tab here needs
/// an Enter before typing goes on. It writes a candidate holding `=` in quotes, and
/// text typed after the closing quote is still the same word.
#[test]
fn a_tab_round_trip_in_elvish() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let elvish = installed("elvish");
    require_level!(Level::L2, elvish.is_some(), "elvish");
    let (_fixture, mut harness) = open_elvish(&elvish.expect("gated above"));

    type_text(&mut harness, "md compose doc.md style.pa");
    press_tab(&mut harness);
    wait_for_line(&mut harness, ELVISH_LIST);
    wait_for_line(&mut harness, "md compose doc.md style.page.");
    press_enter(&mut harness);
    type_text(&mut harness, "al");
    wait_for_line(&mut harness, "md compose doc.md style.page.al");
    press_tab(&mut harness);
    wait_for_line(&mut harness, "md compose doc.md 'style.page.alignment='");
    press_enter(&mut harness);
    type_text(&mut harness, "ce");
    wait_for_line(&mut harness, "md compose doc.md 'style.page.alignment='ce");
    press_tab(&mut harness);
    wait_for_line(&mut harness, "md compose doc.md 'style.page.alignment=center'");
    press_enter(&mut harness);

    type_text(&mut harness, " zz=1");
    wait_for_line(&mut harness, "md compose doc.md 'style.page.alignment=center' zz=1");
    press_enter(&mut harness);
    wait_for_line(&mut harness, "Align: center!");
}

/// The limit the documentation's fallback answers: while the list is open, typing only
/// narrows it, so the sequence the other shells accept never reaches the line. Escape
/// closes the list and restores what was typed.
#[test]
fn typing_in_an_elvish_list_filters_it_and_leaves_the_line_alone() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let elvish = installed("elvish");
    require_level!(Level::L2, elvish.is_some(), "elvish");
    let (_fixture, mut harness) = open_elvish(&elvish.expect("gated above"));

    type_text(&mut harness, "md compose doc.md style.pa");
    press_tab(&mut harness);
    wait_for_line(&mut harness, ELVISH_LIST);
    type_text(&mut harness, "ge.");
    wait_for_line(&mut harness, "COMPLETING argument  ge.");
    wait_for_line(&mut harness, "md compose doc.md style.page.");

    harness.send_key("Escape").expect("send Escape");
    wait_for_line(&mut harness, "md compose doc.md style.pa");
}

/// PowerShell in a windowless pseudoconsole. A console output stream is a repaint, not
/// the text, so what the line holds is read from PSReadLine itself: a key handler writes
/// its buffer to a file, which also shows a trailing space that a screen would hide.
#[cfg(windows)]
mod powershell {
    use super::*;
    use std::io::{Read as _, Write};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use xpty::{Child, CommandBuilder, MasterPty, PtySize, PtySystem as _, native_pty_system};

    /// Under nextest's termination, so a stall fails with the console stream.
    const STEP: Duration = Duration::from_secs(15);
    /// Without this answer to its Device Attributes query, ConPTY holds the child for
    /// about three seconds.
    const DA1_QUERY: &[u8] = b"\x1b[c";
    const DA1_REPLY: &[u8] = b"\x1b[?61;4;6;7;14;21;22;23;24;28;32;42c";
    const F12: &str = "\x1b[24~";
    fn starts(program: &str) -> bool {
        Command::new(program)
            .args(["-NoProfile", "-NonInteractive", "-Command", "exit 0"])
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn ps_quote(path: &Path) -> String {
        format!("'{}'", path.display().to_string().replace('\'', "''"))
    }

    struct Console {
        child: Box<dyn Child + Send + Sync>,
        writer: Arc<Mutex<Box<dyn Write + Send>>>,
        output: Arc<Mutex<Vec<u8>>>,
        _master: Box<dyn MasterPty + Send>,
        line_file: PathBuf,
        fixture: CliProcessFixture,
    }

    impl Console {
        /// A prompt in `program` whose session ran `setup` before it was offered.
        fn open(program: &str, setup: &[&str]) -> Self {
            let fixture = CliProcessFixture::named("level2-path-completion-powershell");
            std::fs::write(fixture.cwd().join("doc.md"), DOCUMENT).expect("write document");
            link_or_copy(md_bin(), &fixture.bin_dir().join("md.exe")).expect("link md");

            let line_file = fixture.tmp_dir().join("line.txt");
            let ready_file = fixture.tmp_dir().join("ready.txt");
            let script_file = fixture.tmp_dir().join("setup.ps1");
            let pending = line_file.with_extension("pending");
            let script = format!(
                "$ErrorActionPreference = 'Stop'\r\n\
                 Set-PSReadLineOption -HistorySaveStyle SaveNothing\r\n\
                 Set-PSReadLineKeyHandler -Chord F12 -ScriptBlock {{\r\n\
                 \x20   $line = $null; $cursor = $null\r\n\
                 \x20   [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)\r\n\
                 \x20   [IO.File]::WriteAllText({pending}, $line)\r\n\
                 \x20   [IO.File]::Move({pending}, {line})\r\n\
                 }}\r\n\
                 {setup}\r\n\
                 [IO.File]::WriteAllText({ready}, 'ready')\r\n",
                pending = ps_quote(&pending),
                line = ps_quote(&line_file),
                setup = setup.join("\r\n"),
                ready = ps_quote(&ready_file),
            );
            std::fs::write(&script_file, script).expect("write setup script");

            let size = PtySize { rows: 60, cols: 160, pixel_width: 0, pixel_height: 0 };
            let pair = native_pty_system().openpty(size).expect("open a pseudoconsole");
            let mut command = CommandBuilder::new(program);
            command.args(["-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass"]);
            command.cwd(fixture.cwd());
            for (key, value) in fixture.command_builder().host_path().build_std().get_envs() {
                match value {
                    Some(value) => command.env(key, value),
                    None => command.env_remove(key),
                }
            }
            let child = pair.slave.spawn_command(command).expect("start PowerShell");
            drop(pair.slave);

            let mut reader = pair.master.try_clone_reader().expect("pseudoconsole reader");
            let writer = Arc::new(Mutex::new(pair.master.take_writer().expect("pseudoconsole writer")));
            let output = Arc::new(Mutex::new(Vec::new()));
            let (sink, answerer) = (Arc::clone(&output), Arc::clone(&writer));
            // Draining also keeps ConPTY from blocking the child on a full pipe.
            std::thread::spawn(move || {
                let mut chunk = [0u8; 4096];
                while let Ok(read) = reader.read(&mut chunk) {
                    if read == 0 {
                        break;
                    }
                    if chunk[..read].windows(DA1_QUERY.len()).any(|window| window == DA1_QUERY) {
                        let mut writer = answerer.lock().unwrap();
                        let _ = writer.write_all(DA1_REPLY);
                        let _ = writer.flush();
                    }
                    sink.lock().unwrap().extend_from_slice(&chunk[..read]);
                }
            });

            let mut console = Self { child, writer, output, _master: pair.master, line_file, fixture };
            console.send(&format!(". {}\r", ps_quote(&script_file)));
            console.wait_for_file(&ready_file);
            console
        }

        fn send(&mut self, keys: &str) {
            let mut writer = self.writer.lock().unwrap();
            writer.write_all(keys.as_bytes()).expect("write to the console");
            writer.flush().expect("flush the console");
        }

        fn stream(&self) -> String {
            biscuit_test_harness::strip_ansi(&String::from_utf8_lossy(&self.output.lock().unwrap()))
        }

        fn wait_for_file(&self, path: &Path) -> String {
            let deadline = Instant::now() + STEP;
            loop {
                if let Ok(text) = std::fs::read_to_string(path) {
                    return text;
                }
                assert!(Instant::now() < deadline, "{} never appeared; console stream:\n{}", path.display(), self.stream());
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        /// What the line holds once PowerShell has handled `keys`.
        fn line_after(&mut self, keys: &str) -> String {
            let _ = std::fs::remove_file(&self.line_file);
            self.send(keys);
            self.send(F12);
            let line_file = self.line_file.clone();
            self.wait_for_file(&line_file)
        }

        /// Runs what is on the line with its output sent to a file, and waits for `expected`.
        fn run_expecting(&mut self, expected: &str) {
            let result = self.fixture.tmp_dir().join("result.txt");
            self.send(&format!(" | Out-File -Encoding utf8 {}\r", ps_quote(&result)));
            let deadline = Instant::now() + STEP;
            loop {
                let text = std::fs::read_to_string(&result).unwrap_or_default();
                if text.contains(expected) {
                    return;
                }
                assert!(Instant::now() < deadline, "`{expected}` never reached {}: {text}; console stream:\n{}", result.display(), self.stream());
                std::thread::sleep(Duration::from_millis(50));
            }
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

    /// The sequence through the line `md --completions powershell` prints. Tab writes the
    /// first candidate whole, so unlike readline's shells there is no `.` to type after
    /// `style.page`, and it adds no space after a finished value either.
    fn assert_round_trip_in(program: &str) {
        let printed = printed_setup(&CliProcessFixture::named("level2-powershell-setup"), "powershell");
        let record = |name: &str| format!("[IO.File]::WriteAllText((Join-Path $PWD '{name}'), (Get-Command md).CommandType.ToString())");
        let mut console = Console::open(program, &[&record("before.txt"), &printed, &record("after.txt")]);
        let read = |name: &str| std::fs::read_to_string(console.fixture.cwd().join(name)).expect("read the recorded command type");
        assert_eq!(read("before.txt"), "Alias", "`md` starts as `mkdir`");
        assert_eq!(read("after.txt"), "Application", "the setup line leaves `md` itself");

        assert_eq!(console.line_after("md compose doc.md style.pa\t"), "md compose doc.md style.page.");
        assert_eq!(console.line_after("al\t"), "md compose doc.md style.page.alignment=");
        assert_eq!(console.line_after("ce\t"), "md compose doc.md style.page.alignment=center");
        assert_eq!(console.line_after(" zz=1"), "md compose doc.md style.page.alignment=center zz=1");
        console.run_expecting("Align: center!");
    }

    #[test]
    fn a_tab_round_trip_in_windows_powershell() {
        require_level!(Level::L2, starts("powershell.exe"), "ConPTY (Windows PowerShell)");
        assert_round_trip_in("powershell.exe");
    }

    #[test]
    fn a_tab_round_trip_in_powershell_7() {
        require_level!(Level::L2, starts("pwsh.exe"), "ConPTY (PowerShell 7)");
        assert_round_trip_in("pwsh.exe");
    }
}
