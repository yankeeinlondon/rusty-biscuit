//! Level-3 keyboard injection on a private X display (Linux), for tests that
//! must press a real key without taking focus from the host user's desktop.
//!
//! [`XvfbKitty`] starts its own `Xvfb` server, opens one kitty window on it,
//! gives that window input focus *on the private display*, and presses keys
//! through the X server's XTEST extension. XTEST events enter the server's
//! input path where a physical keyboard's events enter, so kitty receives an
//! ordinary key event and runs its own key encoder, and the pane's line
//! discipline turns Ctrl+C into `SIGINT`. `kitty @ send-key` and
//! `tmux send-keys` start after the OS input layer; this starts before it.
//!
//! The host's display, window manager, and focus are never touched: the server
//! is a separate process with its own screen that nothing shows, reached only
//! by the clients this module starts. That is why these tests may run
//! unattended, unlike [`crate::xdotool`], which injects into the user's
//! `DISPLAY`. No window manager runs on the private display; `SetInputFocus`
//! alone moves focus there, and it is confirmed with `GetInputFocus` before a
//! press.
//!
//! Requires `Xvfb` and `kitty` on `$PATH` ([`XvfbKitty::can_launch`]). kitty
//! renders with Mesa's software OpenGL, since Xvfb has no GPU.

use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use x11rb::CURRENT_TIME;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    ConnectionExt as _, InputFocus, KEY_PRESS_EVENT, KEY_RELEASE_EVENT, Keycode, Keysym, Window,
};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::rust_connection::RustConnection;

use super::kitty::{KittyHarness, attach_at, first_window, quit_instance};
use super::{CLEANUP_TIMEOUT, SPAWN_TIMEOUT, wait_for_prompt};

/// The `Control_L` keysym.
const CONTROL_L: Keysym = 0xffe3;

/// A private `Xvfb` server. [`Drop`] stops it.
pub struct XvfbDisplay {
    server: Child,
    name: String,
}

impl XvfbDisplay {
    /// `true` when `Xvfb` is on `$PATH`.
    pub fn available() -> bool {
        super::which("Xvfb")
    }

    /// Starts `Xvfb` on the first free display number and waits until it
    /// accepts clients. It listens on a local socket only.
    ///
    /// ## Errors
    ///
    /// Returns an error when `Xvfb` cannot be spawned, exits, or does not
    /// report its display within [`SPAWN_TIMEOUT`].
    pub fn start() -> io::Result<Self> {
        let (read_end, write_end) = cloexec_pipe()?;
        let write_fd = write_end.as_raw_fd();
        let mut cmd = Command::new("Xvfb");
        // `-displayfd` picks a free display number and writes it once the
        // server accepts connections, so concurrent tests cannot collide.
        cmd.args(["-displayfd", &write_fd.to_string()])
            .args([
                "-screen",
                "0",
                "1920x1200x24",
                "-nolisten",
                "tcp",
                "-noreset",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // SAFETY: only async-signal-safe calls (`fcntl`, `prctl`) run between
        // fork and exec.
        unsafe {
            cmd.pre_exec(move || {
                inherit_fd(write_fd)?;
                die_with_parent()
            });
        }
        let mut server = cmd.spawn()?;
        drop(write_end);
        match read_display_number(read_end, SPAWN_TIMEOUT) {
            Ok(number) => Ok(Self {
                server,
                name: format!(":{number}"),
            }),
            Err(error) => {
                let _ = server.kill();
                let _ = server.wait();
                Err(error)
            }
        }
    }

    /// The display name, such as `:3`, for `DISPLAY`.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for XvfbDisplay {
    fn drop(&mut self) {
        let _ = self.server.kill();
        let _ = self.server.wait();
    }
}

/// One kitty window on its own [`XvfbDisplay`], with a remote-control socket
/// for typing and capture and an XTEST connection for key presses.
///
/// [`Drop`] quits kitty and then stops the display. Both are also started with
/// `PR_SET_PDEATHSIG`, so a test killed before `Drop` does not leave them
/// running; that signal follows the spawning thread, so keep the instance on
/// the thread that launched it.
pub struct XvfbKitty {
    kitty: Child,
    to: String,
    window_id: String,
    window: Window,
    connection: RustConnection,
    /// Holds the socket and kitty's log.
    dir: tempfile::TempDir,
    /// Declared last so it is dropped after every client of the display.
    display: XvfbDisplay,
}

impl XvfbKitty {
    /// `true` when `Xvfb` and `kitty` are on `$PATH`.
    pub fn can_launch() -> bool {
        XvfbDisplay::available() && super::which("kitty")
    }

    /// Starts a private display, then kitty on it with one `columns` × `lines`
    /// cell window running the harness's rc-suppressed login shell, and waits
    /// for its prompt.
    ///
    /// kitty runs with `--config NONE`, so the host's `kitty.conf` does not
    /// apply.
    ///
    /// ## Errors
    ///
    /// Returns an error when the display or kitty does not start within
    /// [`SPAWN_TIMEOUT`], or when the display lacks the XTEST extension.
    pub fn launch(columns: u32, lines: u32) -> io::Result<Self> {
        let display = XvfbDisplay::start()?;
        // `/tmp`, not `$TMPDIR`: a unix socket path is limited to 108 bytes.
        let dir = tempfile::Builder::new()
            .prefix("biscuit-xvfb-kitty-")
            .tempdir_in("/tmp")?;
        let to = format!("unix:{}", dir.path().join("kitty.sock").display());
        let log = File::create(dir.path().join("kitty.log"))?;

        let mut cmd = Command::new("kitty");
        cmd.args(["--config", "NONE"]);
        for option in [
            "allow_remote_control=socket-only".to_string(),
            "linux_display_server=x11".to_string(),
            "remember_window_size=no".to_string(),
            format!("initial_window_width={columns}c"),
            format!("initial_window_height={lines}c"),
            "confirm_os_window_close=0".to_string(),
        ] {
            cmd.args(["-o", &option]);
        }
        cmd.args(["--listen-on", &to]);
        cmd.args(super::login_shell_argv(&super::detect_shell(), false));
        cmd.env("DISPLAY", display.name())
            .env("LIBGL_ALWAYS_SOFTWARE", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log);
        super::apply_color_forcing_env(&mut cmd);
        // SAFETY: `prctl` is async-signal-safe.
        unsafe {
            cmd.pre_exec(die_with_parent);
        }
        let mut kitty = cmd.spawn()?;

        let deadline = Instant::now() + SPAWN_TIMEOUT;
        let (window_id, platform_window_id) = loop {
            if let Some(ids) = first_window(&to) {
                break ids;
            }
            let exited = kitty.try_wait()?.is_some();
            if exited || Instant::now() > deadline {
                let _ = kitty.kill();
                let _ = kitty.wait();
                return Err(io::Error::other(format!(
                    "kitty on {} never answered on its socket; kitty log:\n{}",
                    display.name(),
                    read_log(&dir.path().join("kitty.log"))
                )));
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        let window = Window::try_from(platform_window_id).map_err(|_| {
            io::Error::other(format!("{platform_window_id} is not an X11 window id"))
        })?;

        let (connection, _) =
            RustConnection::connect(Some(display.name())).map_err(io::Error::other)?;
        connection
            .xtest_get_version(2, 2)
            .map_err(io::Error::other)?
            .reply()
            .map_err(|error| {
                io::Error::other(format!("the private display has no XTEST: {error}"))
            })?;

        let instance = Self {
            kitty,
            to,
            window_id,
            window,
            connection,
            dir,
            display,
        };
        wait_for_prompt(&mut instance.harness())?;
        Ok(instance)
    }

    /// A harness attached to the kitty window, for typing commands and
    /// capturing the screen through `kitty @`. It does not own the window.
    pub fn harness(&self) -> KittyHarness {
        attach_at(&self.window_id, &self.to)
    }

    /// Presses Ctrl+`key` (a lowercase ASCII letter) as XTEST key events
    /// after giving the kitty window input focus on the private display.
    ///
    /// ## Errors
    ///
    /// Returns an error when `key` has no key on the display's keyboard map,
    /// focus does not move to the window, or the X connection fails.
    pub fn press_ctrl(&self, key: char) -> io::Result<()> {
        let control = self.keycode(CONTROL_L)?;
        let letter = self.keycode(Keysym::from(key))?;
        self.focus()?;
        for (kind, code) in [
            (KEY_PRESS_EVENT, control),
            (KEY_PRESS_EVENT, letter),
            (KEY_RELEASE_EVENT, letter),
            (KEY_RELEASE_EVENT, control),
        ] {
            self.connection
                .xtest_fake_input(kind, code, CURRENT_TIME, x11rb::NONE, 0, 0, 0)
                .map_err(io::Error::other)?;
        }
        // A round trip: the server has processed every event when it answers.
        self.connection
            .get_input_focus()
            .map_err(io::Error::other)?
            .reply()
            .map_err(io::Error::other)?;
        Ok(())
    }

    fn focus(&self) -> io::Result<()> {
        self.connection
            .set_input_focus(InputFocus::PARENT, self.window, CURRENT_TIME)
            .map_err(io::Error::other)?
            .check()
            .map_err(io::Error::other)?;
        let focus = self
            .connection
            .get_input_focus()
            .map_err(io::Error::other)?
            .reply()
            .map_err(io::Error::other)?
            .focus;
        if focus != self.window {
            return Err(io::Error::other(format!(
                "input focus on {} is window {focus}, not kitty's {}",
                self.display.name(),
                self.window
            )));
        }
        Ok(())
    }

    /// The keycode whose unshifted keysym is `keysym`.
    fn keycode(&self, keysym: Keysym) -> io::Result<Keycode> {
        let setup = self.connection.setup();
        let (first, last) = (setup.min_keycode, setup.max_keycode);
        let mapping = self
            .connection
            .get_keyboard_mapping(first, last - first + 1)
            .map_err(io::Error::other)?
            .reply()
            .map_err(io::Error::other)?;
        let per_keycode = usize::from(mapping.keysyms_per_keycode).max(1);
        mapping
            .keysyms
            .chunks(per_keycode)
            .position(|keysyms| keysyms.first() == Some(&keysym))
            .and_then(|offset| u8::try_from(offset).ok())
            .map(|offset| first + offset)
            .ok_or_else(|| {
                io::Error::other(format!(
                    "no key on {} maps to keysym {keysym:#x}",
                    self.display.name()
                ))
            })
    }
}

impl Drop for XvfbKitty {
    fn drop(&mut self) {
        quit_instance(&self.to);
        let deadline = Instant::now() + CLEANUP_TIMEOUT;
        while matches!(self.kitty.try_wait(), Ok(None)) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.kitty.kill();
        let _ = self.kitty.wait();
    }
}

fn read_log(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// A pipe whose ends are closed on exec; see [`inherit_fd`].
fn cloexec_pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0 as RawFd; 2];
    // SAFETY: `fds` has room for the two descriptors `pipe2` writes.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `pipe2` succeeded, so both descriptors are open and owned here.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

/// Clears close-on-exec on `fd` in a forked child, so only that child inherits
/// it: a pipe opened without the flag would leak into every process another
/// thread spawns meanwhile.
fn inherit_fd(fd: RawFd) -> io::Result<()> {
    // SAFETY: `fcntl` on a descriptor this process owns.
    if unsafe { libc::fcntl(fd, libc::F_SETFD, 0) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Asks the kernel to kill this child when the thread that spawned it exits.
fn die_with_parent() -> io::Result<()> {
    // SAFETY: `prctl(PR_SET_PDEATHSIG)` takes a signal number only.
    if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Reads the display number `Xvfb -displayfd` writes once it is ready.
fn read_display_number(read_end: OwnedFd, timeout: Duration) -> io::Result<u32> {
    let deadline = Instant::now() + timeout;
    let mut file = File::from(read_end);
    let mut text = Vec::new();
    while !text.contains(&b'\n') {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Xvfb did not report a display",
            ));
        }
        let mut poll = libc::pollfd {
            fd: file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let wait_ms = i32::try_from(remaining.as_millis()).unwrap_or(i32::MAX);
        // SAFETY: one valid `pollfd`.
        if unsafe { libc::poll(&mut poll, 1, wait_ms) } < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if poll.revents == 0 {
            continue;
        }
        let mut chunk = [0u8; 16];
        let read = file.read(&mut chunk)?;
        if read == 0 {
            return Err(io::Error::other("Xvfb exited before reporting a display"));
        }
        text.extend_from_slice(&chunk[..read]);
    }
    String::from_utf8_lossy(&text).trim().parse().map_err(|_| {
        io::Error::other(format!(
            "Xvfb reported {:?}",
            String::from_utf8_lossy(&text)
        ))
    })
}
