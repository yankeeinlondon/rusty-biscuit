//! R4 on Windows: Ctrl+C typed into a real console during the exit-time
//! delivery drain, for every command in `common::drain_interrupt`.
//!
//! `claudine` is the root process of a pseudoconsole (ConPTY, through
//! `xpty`), and each press writes ETX to the console's input, which is what a
//! Windows terminal writes for a Ctrl+C key press. conhost turns it into
//! `CTRL_C_EVENT` for every process attached to the console. The
//! `lifecycle_message_drain_interrupt.rs` tests instead send
//! `CTRL_BREAK_EVENT` to a new process group, which bypasses the console's
//! input handling. A pseudoconsole has no window, so nothing takes focus, and
//! it needs no terminal backend, so this is ordinary L1.
//!
//! The output stream is conhost's repaint of the screen rather than the text
//! in order, so the waits below match whole short phrases that Claudine writes
//! in one line, on a screen wide enough that they do not wrap.

use crate::common;

use common::CliProcessFixture;
use common::drain_interrupt::{DrainCommand, PENDING_REPORT};
use common::strip_ansi;
use common::webhook_listener::{ListenerMode, WebhookListener};
use std::io::{Read as _, Write};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use xpty::{Child, CommandBuilder, MasterPty, PtySize, PtySystem as _, native_pty_system};

/// Ceiling on every wait for a notice, a request, or an exit.
const WAIT: Duration = Duration::from_secs(60);

/// Pause between seeing that the command has finished its work and the first
/// press, so the press lands in the drain rather than in the command's last
/// steps. The drain itself lasts 10 s.
const SETTLE: Duration = Duration::from_millis(500);

/// How long the drain must keep running after the first press.
const STILL_DRAINING: Duration = Duration::from_secs(1);

/// The second press must end the process within this; the drain would
/// otherwise run for most of its 10 s budget.
const FORCE_EXIT_WITHIN: Duration = Duration::from_secs(3);

/// Windows Terminal's answer to the Primary Device Attributes query. A bare
/// `ESC [ ? 1 ; 0 c` does not release ConPTY; this longer attribute list does.
const DEVICE_ATTRIBUTES_REPLY: &[u8] = b"\x1b[?61;4;6;7;14;21;22;23;24;28;32;42c";

const POLL: Duration = Duration::from_millis(20);

type HandlerRoutine = unsafe extern "system" fn(u32) -> i32;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetConsoleCtrlHandler(handler: Option<HandlerRoutine>, add: i32) -> i32;
}

const SCREEN: PtySize = PtySize {
    rows: 60,
    cols: 250,
    pixel_width: 0,
    pixel_height: 0,
};

/// `claudine` in a pseudoconsole; killed and closed on drop.
struct Console {
    child: Box<dyn Child + Send + Sync>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    output: Arc<Mutex<Vec<u8>>>,
    _master: Box<dyn MasterPty + Send>,
}

impl Console {
    /// Start `command`'s program, arguments, environment, and directory in a
    /// new pseudoconsole. Like `command`, the builder starts from this
    /// process's environment, and `command`'s explicit variables and removals
    /// are applied on top.
    fn launch(command: &Command) -> Self {
        let pair = native_pty_system()
            .openpty(SCREEN)
            .expect("open a pseudoconsole");
        let mut builder = CommandBuilder::new(command.get_program());
        builder.args(command.get_args());
        for (key, value) in command.get_envs() {
            match value {
                Some(value) => builder.env(key, value),
                None => builder.env_remove(key),
            }
        }
        if let Some(dir) = command.get_current_dir() {
            builder.cwd(dir);
        }
        // Ctrl+C processing is an inherited process attribute, and a process
        // started in a new console group, as nextest's and sshd's children can
        // be, starts with it disabled. A terminal launches its shell with it
        // enabled, so clear the attribute for the child to inherit.
        // SAFETY: a documented Win32 call with a null handler and a flag.
        let enabled = unsafe { SetConsoleCtrlHandler(None, 0) };
        assert_ne!(
            enabled,
            0,
            "SetConsoleCtrlHandler failed: {}",
            std::io::Error::last_os_error()
        );
        let child = pair.slave.spawn_command(builder).expect("start claudine");
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .expect("pseudoconsole reader");
        let writer: Arc<Mutex<Box<dyn Write + Send>>> =
            Arc::new(Mutex::new(pair.master.take_writer().expect("pseudoconsole writer")));
        let output = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&output);
        let replies = Arc::clone(&writer);
        // Draining continuously keeps ConPTY from blocking the child on a
        // full output pipe.
        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            while let Ok(n) = reader.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                sink.lock().unwrap().extend_from_slice(&chunk[..n]);
                // ConPTY opens with a Primary Device Attributes query (ESC [ c)
                // and holds the child's startup for about 3 s unless the
                // hosting terminal answers, as every real terminal does.
                if chunk[..n].windows(3).any(|window| window == b"\x1b[c") {
                    let mut writer = replies.lock().unwrap();
                    let _ = writer.write_all(DEVICE_ATTRIBUTES_REPLY);
                    let _ = writer.flush();
                }
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

    /// Type Ctrl+C.
    fn press(&mut self) -> Instant {
        let pressed = Instant::now();
        let mut writer = self.writer.lock().unwrap();
        writer.write_all(b"\x03").expect("write ETX to the console");
        writer.flush().expect("flush the console");
        pressed
    }

    fn exit_code(&mut self) -> Option<u32> {
        self.child
            .try_wait()
            .expect("poll claudine")
            .map(|status| status.exit_code())
    }

    fn wait_for(&mut self, needle: &str) -> Instant {
        let deadline = Instant::now() + WAIT;
        loop {
            if self.stream().contains(needle) {
                return Instant::now();
            }
            if let Some(code) = self.exit_code() {
                panic!(
                    "claudine exited ({code}) without writing {needle:?}; console:\n{}",
                    self.stream()
                );
            }
            assert!(
                Instant::now() < deadline,
                "claudine did not write {needle:?} within {WAIT:?}; console:\n{}",
                self.stream()
            );
            std::thread::sleep(POLL);
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

/// Run `command` in a console against a route that never replies, type
/// Ctrl+C twice during the drain, and check each press's effect.
fn second_typed_press_during_the_drain_exits_130(command: DrainCommand, name: &str) {
    let fixture = CliProcessFixture::named(name);
    let listener = WebhookListener::start(ListenerMode::NeverReply);
    let args = command.prepare(fixture.home(), fixture.bin_dir(), fixture.cwd());
    let mut child = fixture.command_std();
    listener.apply_route_env(&mut child);
    child.args(&args);
    let mut console = Console::launch(&child);

    listener
        .wait_for_request(WAIT)
        .unwrap_or_else(|| panic!("the message was never sent; console:\n{}", console.stream()));
    console.wait_for(command.finished_marker());
    std::thread::sleep(SETTLE);

    console.press();
    let noticed = console.wait_for(command.first_press_notice());
    while Instant::now() < noticed + STILL_DRAINING {
        if let Some(code) = console.exit_code() {
            panic!(
                "the first press ended the drain ({code}); console:\n{}",
                console.stream()
            );
        }
        std::thread::sleep(POLL);
    }

    let second = console.press();
    let deadline = second + WAIT;
    let code = loop {
        if let Some(code) = console.exit_code() {
            break code;
        }
        assert!(
            Instant::now() < deadline,
            "claudine did not exit; console:\n{}",
            console.stream()
        );
        std::thread::sleep(POLL);
    };
    let waited = second.elapsed();
    let stream = console.stream();
    assert_eq!(
        code, 130,
        "the second press force-exits; console:\n{stream}"
    );
    assert!(
        waited < FORCE_EXIT_WITHIN,
        "the second press exits at once, not at the end of the drain ({waited:?}); console:\n{stream}"
    );
    assert!(
        !stream.contains(PENDING_REPORT),
        "a forced exit skips the drain's report; console:\n{stream}"
    );
}

#[test]
fn a_second_typed_ctrl_c_during_the_compose_drain_exits_130() {
    second_typed_press_during_the_drain_exits_130(DrainCommand::Compose, "drain-console-compose");
}

#[test]
fn a_second_typed_ctrl_c_during_the_inline_compose_drain_exits_130() {
    second_typed_press_during_the_drain_exits_130(
        DrainCommand::InlineCompose,
        "drain-console-inline",
    );
}

#[test]
fn a_second_typed_ctrl_c_during_the_sequence_drain_exits_130() {
    second_typed_press_during_the_drain_exits_130(DrainCommand::Sequence, "drain-console-sequence");
}

#[test]
fn a_second_typed_ctrl_c_during_the_wrapper_drain_exits_130() {
    second_typed_press_during_the_drain_exits_130(DrainCommand::Wrapper, "drain-console-wrapper");
}
