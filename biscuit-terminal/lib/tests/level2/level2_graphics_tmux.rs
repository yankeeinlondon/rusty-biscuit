//! Level-2 evidence for graphics through a real tmux.
//!
//! The `discovery_probe` example (`PROBE=graphics`, `PROBE_GRAPHICS_IMAGE`)
//! runs inside a detached pane on the harness's private tmux server, so the
//! library reads the live facts: `tmux -V`, the pane's `allow-passthrough`
//! (set by the test before the probe starts), and the attached clients (none:
//! the session is detached, so the outer terminal is unidentified). The L1
//! sibling `graphics_tmux_matrix.rs` covers every route row with fixed facts.
//!
//! `pipe-pane` records the raw bytes the probe writes, which is how the test
//! sees the passthrough envelopes: tmux consumes them, so they never reach
//! the pane's grid. No window is opened and nothing takes focus.
//!
//! The placeholder test attaches one client with `TERM=xterm-kitty` through
//! `script` (a pseudo-terminal, no window), so the library identifies a kitty
//! outer terminal from the live client list; the client's output is what
//! tmux forwards to that terminal. The Sixel test attaches an
//! `xterm-256color` client with `tmux -T sixel`, a feature of that client
//! alone, so tmux draws Sixel itself.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use biscuit_test_harness::strip_ansi;
use biscuit_test_harness::tmux::{TmuxHarness, kill_session_by_name, tmux_command};
use test_toolkit::{Backend, Level, require_level};

/// Variables that would decide the route without the tmux facts.
const UNSET: &[&str] = &[
    "BISCUIT_GRAPHICS",
    "BISCUIT_GRAPHICS_TRANSPORT",
    "BISCUIT_TERMINAL_PROBE",
    "ZELLIJ",
    "SSH_CONNECTION",
    "SSH_CLIENT",
    "SSH_TTY",
];

fn probe_binary() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe unavailable");
    let probe = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("unexpected test exe layout")
        .join("examples")
        .join("discovery_probe");
    assert!(
        probe.exists(),
        "discovery_probe example not found at {}. Run \
         `cargo build -p biscuit-terminal --example discovery_probe`.",
        probe.display()
    );
    probe
}

fn tmux(args: &[&str]) -> String {
    let out = tmux_command()
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("run tmux");
    assert!(
        out.status.success(),
        "tmux {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A detached session whose only pane waits for Enter, then runs the probe
/// with `allow-passthrough` set to `passthrough`. Killed on drop.
struct ProbeSession {
    // Dropped before the session is killed.
    client: Option<TestClient>,
    name: String,
    raw: PathBuf,
    _dir: tempfile::TempDir,
}

/// The terminal a [`TestClient`] claims to be.
#[derive(Clone, Copy)]
struct ClientSpec<'a> {
    term: &'a str,
    /// `tmux -T`: terminal features for this client only.
    features: Option<&'a str>,
    /// A terminfo directory for `term` when the system database lacks it.
    terminfo: Option<&'a std::path::Path>,
}

/// A tmux client attached from inside `script` as `spec.term`, writing what
/// tmux sends it to `output`. Killed on drop.
struct TestClient {
    child: std::process::Child,
    output: PathBuf,
}

impl TestClient {
    fn attach(session: &str, spec: ClientSpec<'_>, output: PathBuf) -> Self {
        let mut tmux_args: Vec<String> = tmux_command()
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        if let Some(features) = spec.features {
            tmux_args.extend(["-T".to_string(), features.to_string()]);
        }
        // `-u`: UTF-8 to this client whatever the locale.
        tmux_args.extend(["-u", "attach", "-t", session].map(String::from));
        let mut script = std::process::Command::new("script");
        if cfg!(target_os = "macos") {
            script.args(["-q", "/dev/null", "tmux"]).args(&tmux_args);
        } else {
            let quoted: Vec<String> = tmux_args.iter().map(|arg| format!("'{arg}'")).collect();
            script.args(["-q", "-c", &format!("tmux {}", quoted.join(" ")), "/dev/null"]);
        }
        script.env("TERM", spec.term);
        if let Some(dir) = spec.terminfo {
            script.env("TERMINFO", dir);
        }
        // A pipe kept open holds the client attached; EOF would detach it.
        let child = script
            .stdin(Stdio::piped())
            .stdout(std::fs::File::create(&output).expect("client output file"))
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn script");
        let client = Self { child, output };
        let deadline = Instant::now() + Duration::from_secs(10);
        while tmux(&["list-clients", "-t", session, "-F", "#{client_termname}"]).trim() != spec.term {
            assert!(Instant::now() < deadline, "the {} client did not attach", spec.term);
            std::thread::sleep(Duration::from_millis(50));
        }
        client
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Where the `xterm-kitty` terminfo entry is: `Some(None)` when the system
/// database has it, `Some(dir)` for an installed kitty's own database, and
/// `None` when neither does (tmux then refuses to attach the client).
fn kitty_terminfo() -> Option<Option<PathBuf>> {
    let system = std::process::Command::new("infocmp")
        .arg("xterm-kitty")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if system {
        return Some(None);
    }
    let bundled = PathBuf::from("/Applications/kitty.app/Contents/Resources/kitty/terminfo");
    bundled.is_dir().then_some(Some(bundled))
}

fn script_available() -> bool {
    std::process::Command::new("sh")
        .args(["-c", "command -v script"])
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

impl ProbeSession {
    fn run(passthrough: &str) -> Self {
        Self::run_with(passthrough, None, 50, (4, 4))
    }

    /// [`run`](Self::run) with `client` attached before the probe starts,
    /// a pane `height` rows tall, and an `image_size` pixel image.
    fn run_with(passthrough: &str, client: Option<ClientSpec<'_>>, height: u32, image_size: (u32, u32)) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let image = dir.path().join("dot.png");
        image::RgbImage::from_pixel(image_size.0, image_size.1, image::Rgb([200, 30, 30]))
            .save(&image)
            .expect("write image");
        let raw = dir.path().join("raw.bin");
        let name = format!(
            "biscuit_test_{}_graphics_{passthrough}_{}",
            std::process::id(),
            client.map_or("none", |spec| spec.term)
        );

        let unset: String = UNSET.iter().map(|name| format!("-u {name} ")).collect();
        let command = format!(
            "read _; env {unset}PROBE=graphics NO_COLOR=1 PROBE_GRAPHICS_IMAGE='{}' '{}'; sleep 60",
            image.display(),
            probe_binary().display()
        );
        tmux(&[
            "new-session", "-d", "-s", &name, "-x", "250", "-y", &height.to_string(), "sh", "-c", &command,
        ]);
        // The window keeps its size when a client with a smaller terminal
        // attaches, so the probe's lines are not wrapped.
        tmux(&["set-option", "-t", &name, "window-size", "manual"]);
        let client = client.map(|spec| TestClient::attach(&name, spec, dir.path().join("client.bin")));
        let session = Self {
            client,
            name,
            raw,
            _dir: dir,
        };
        tmux(&["set-option", "-p", "-t", &session.name, "allow-passthrough", passthrough]);
        tmux(&[
            "pipe-pane",
            "-o",
            "-t",
            &session.name,
            &format!("cat >> '{}'", session.raw.display()),
        ]);
        tmux(&["send-keys", "-t", &session.name, "Enter"]);
        session
    }

    /// The pane text once the probe printed `graphics_done`.
    fn finished_text(&self) -> String {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            // With history: a placeholder grid can scroll the early lines.
            let text = strip_ansi(&tmux(&["capture-pane", "-p", "-S", "-", "-t", &self.name]));
            if text.contains("graphics_done") {
                return text;
            }
            assert!(Instant::now() < deadline, "probe did not finish; pane:\n{text}");
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Everything the probe wrote, as tmux received it.
    fn raw_output(&self) -> Vec<u8> {
        // `pipe-pane` copies asynchronously; wait for the end marker.
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let bytes = std::fs::read(&self.raw).unwrap_or_default();
            if find(&bytes, b"graphics_done").is_some() || Instant::now() > deadline {
                return bytes;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for ProbeSession {
    fn drop(&mut self) {
        kill_session_by_name(&self.name);
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.trim_start().strip_prefix(key))
        .map(str::trim)
}

fn tmux_version_at_least_3_3() -> bool {
    let out = tmux_command().arg("-V").output();
    let Ok(out) = out else { return false };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut parts = text
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u32>().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0)) >= (3, 3)
}

/// `allow-passthrough on`, no client attached: Kitty through tmux,
/// unverified, and every Kitty chunk reaches tmux in its own envelope with
/// its ESCs doubled, none of it left on the pane.
#[test]
fn level2_tmux_passthrough_on_wraps_kitty_chunks() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    require_level!(Level::L2, tmux_version_at_least_3_3(), "tmux >= 3.3");
    let session = ProbeSession::run("on");
    let text = session.finished_text();

    assert_eq!(field(&text, "graphics_format[0]="), Some("Kitty"), "pane:\n{text}");
    assert_eq!(field(&text, "graphics_transport[0]="), Some("Tmux"), "pane:\n{text}");
    assert_eq!(field(&text, "graphics_confidence[0]="), Some("Unknown"), "pane:\n{text}");
    let reason = field(&text, "graphics_reason[0]=").unwrap_or_default();
    assert!(reason.contains("allow-passthrough on"), "reason: {reason}");
    assert_eq!(field(&text, "graphics_image[0]="), Some("pixels"), "pane:\n{text}");
    let after = &text[text.find("graphics_image[0]=").unwrap_or(0)..];
    assert!(
        !after.contains("_G") && !after.contains("tmux;"),
        "graphics bytes reached the pane's grid:\n{after}"
    );

    let raw = session.raw_output();
    let envelope = find(&raw, b"\x1bPtmux;\x1b\x1b_Gq=2,i=").unwrap_or_else(|| {
        panic!("no wrapped Kitty upload in the raw output: {:?}", String::from_utf8_lossy(&raw))
    });
    let upload = &raw[envelope..];
    let end = find(upload, b"\x1b\x1b\\\x1b\\").expect("the chunk's ST is doubled, then the envelope's");
    let chunk = String::from_utf8_lossy(&upload[..end]);
    assert!(chunk.contains(",f=100,a=T,t=d,c="), "{chunk}");
    assert!(
        find(&raw, b"\x1b_G").is_none_or(|at| at > 0 && raw[at - 1] == 0x1b),
        "every Kitty sequence is inside an envelope"
    );
}

/// `allow-passthrough off`: no pixel route, the reason names the setting,
/// and the image prints as alt text with no graphics bytes at all.
#[test]
fn level2_tmux_passthrough_off_falls_back_and_names_the_setting() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    require_level!(Level::L2, tmux_version_at_least_3_3(), "tmux >= 3.3");
    let session = ProbeSession::run("off");
    let text = session.finished_text();

    assert_eq!(field(&text, "graphics_format[0]="), Some("None"), "pane:\n{text}");
    assert_eq!(field(&text, "graphics_reason_kind[0]="), Some("BlockedTransport"), "pane:\n{text}");
    assert_eq!(
        field(&text, "graphics_fix[0]="),
        Some("Some(\"set -g allow-passthrough on\")"),
        "pane:\n{text}"
    );
    assert_eq!(
        field(&text, "graphics_image[0]="),
        Some("fallback:[Image: dot.png]"),
        "pane:\n{text}"
    );
    let raw = session.raw_output();
    assert!(find(&raw, b"\x1b_G").is_none(), "no Kitty bytes are written");
    assert!(find(&raw, b"tmux;").is_none(), "no passthrough envelope is written");
}

/// A kitty client attached, `allow-passthrough on`: Kitty through tmux with
/// Unicode placeholders. tmux receives the `U=1` upload in an envelope and
/// the placeholder grid as ordinary text, and forwards both to the client
/// unwrapped, with the image ID in each cell's 24-bit foreground color.
#[test]
fn level2_tmux_kitty_client_gets_unicode_placeholders() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    require_level!(Level::L2, tmux_version_at_least_3_3(), "tmux >= 3.3");
    require_level!(Level::L2, script_available(), "script(1)");
    let terminfo = kitty_terminfo();
    require_level!(Level::L2, terminfo.is_some(), "xterm-kitty terminfo");
    let kitty = ClientSpec {
        term: "xterm-kitty",
        features: None,
        terminfo: terminfo.as_ref().and_then(Option::as_deref),
    };
    let session = ProbeSession::run_with("on", Some(kitty), 50, (4, 4));
    let text = session.finished_text();

    assert_eq!(field(&text, "graphics_format[0]="), Some("Kitty"), "pane:\n{text}");
    assert_eq!(field(&text, "graphics_transport[0]="), Some("Tmux"), "pane:\n{text}");
    assert_eq!(field(&text, "graphics_placement[0]="), Some("Placeholders"), "pane:\n{text}");
    assert_eq!(field(&text, "graphics_confidence[0]="), Some("Known"), "pane:\n{text}");
    let reason = field(&text, "graphics_reason[0]=").unwrap_or_default();
    assert!(reason.contains("kitty") && reason.contains("Unicode placeholders"), "reason: {reason}");
    assert_eq!(field(&text, "graphics_image[0]="), Some("pixels"), "pane:\n{text}");

    let raw = session.raw_output();
    let upload_at = find(&raw, b"\x1bPtmux;\x1b\x1b_Gq=2,i=").unwrap_or_else(|| {
        panic!("no wrapped Kitty upload in the raw output: {:?}", String::from_utf8_lossy(&raw))
    });
    let upload = String::from_utf8_lossy(&raw[upload_at..]);
    assert!(upload.contains(",a=T,U=1,"), "a virtual placement: {upload:.120}");
    let id: u32 = upload["\x1bPtmux;\x1b\x1b_Gq=2,i=".len()..]
        .split(',')
        .next()
        .and_then(|id| id.parse().ok())
        .expect("the upload carries an image ID");
    let color = format!("\x1b[38;2;{};{};{}m", id >> 16, (id >> 8) & 0xFF, id & 0xFF);
    let first_cell = format!("{color}\u{10EEEE}\u{0305}\u{0305}");
    let raw_text = String::from_utf8_lossy(&raw);
    let grid_at = raw_text.find(&first_cell).unwrap_or_else(|| panic!("no placeholder grid for image {id}"));
    assert!(!raw_text[grid_at..].starts_with("\x1bP"), "the grid is not wrapped");
    assert!(raw_text[grid_at..].contains("\x1b[39m"), "the grid resets the foreground");

    let client = session.client.as_ref().expect("a kitty client is attached");
    let deadline = Instant::now() + Duration::from_secs(5);
    let forwarded = loop {
        let bytes = String::from_utf8_lossy(&std::fs::read(&client.output).unwrap_or_default()).into_owned();
        let done = bytes.contains(&color) && bytes.contains('\u{10EEEE}');
        if done || Instant::now() > deadline {
            break bytes;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(
        forwarded.contains(&format!("\x1b_Gq=2,i={id},f=100,a=T,U=1,")),
        "tmux forwards the upload unwrapped to the client"
    );
    assert!(forwarded.contains(&color), "tmux keeps the 24-bit ID color for the client");
    assert!(forwarded.contains('\u{10EEEE}'), "tmux draws the placeholder cells for the client");
}

/// Whether the harness's tmux was built with Sixel (`#{sixel_support}`,
/// tmux 3.4 and later), read in a short-lived session of its own.
fn tmux_sixel_support() -> bool {
    let name = format!("biscuit_test_{}_sixel_support", std::process::id());
    let started = tmux_command()
        .args(["new-session", "-d", "-s", &name, "sleep", "30"])
        .stdin(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !started {
        return false;
    }
    let supported = tmux_command()
        .args(["display-message", "-p", "-t", &name, "#{sixel_support}"])
        .stdin(Stdio::null())
        .output()
        .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).trim() == "1");
    kill_session_by_name(&name);
    supported
}

/// tmux built with Sixel and a client with the `sixel` feature, with
/// `allow-passthrough off`: tmux draws Sixel itself. The route is verified
/// from tmux's own DA1, asked without the Kitty query (which tmux would turn
/// into the pane title), and the image goes to tmux unwrapped. The pane is
/// short, so the image is drawn at the bottom of the viewport: the text
/// after it lands right below it. tmux does not redraw it for this client,
/// whose `script` pseudo-terminal reports no pixel size; that is tmux's
/// own behavior and is not asserted.
#[test]
fn level2_tmux_draws_sixel_itself_at_the_bottom_of_the_viewport() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    require_level!(Level::L2, script_available(), "script(1)");
    require_level!(Level::L2, tmux_sixel_support(), "tmux built with Sixel");
    let sixel = ClientSpec {
        term: "xterm-256color",
        features: Some("sixel"),
        terminfo: None,
    };
    let session = ProbeSession::run_with("off", Some(sixel), 8, (400, 16));
    let text = session.finished_text();

    assert_eq!(field(&text, "graphics_format[0]="), Some("Sixel"), "pane:\n{text}");
    assert_eq!(field(&text, "graphics_transport[0]="), Some("Direct"), "pane:\n{text}");
    assert_eq!(field(&text, "graphics_confidence[0]="), Some("Verified"), "pane:\n{text}");
    let reason = field(&text, "graphics_reason[0]=").unwrap_or_default();
    assert!(reason.contains("draws Sixel itself"), "reason: {reason}");
    assert_eq!(field(&text, "graphics_image[0]="), Some("pixels"), "pane:\n{text}");

    let raw = session.raw_output();
    let raw_text = String::from_utf8_lossy(&raw);
    assert!(find(&raw, b"\x1b_G").is_none(), "no Kitty query reaches tmux: {raw_text:?}");
    assert!(find(&raw, b"tmux;").is_none(), "nothing is wrapped for passthrough");
    let title = tmux(&["display-message", "-p", "-t", &session.name, "#{pane_title}"]);
    assert!(!title.contains("Gi="), "an unwrapped Kitty query became the title: {title}");

    // The image reserved `rows` lines with IND, drew, and moved below them.
    let dcs = raw_text.find("\x1b[s\x1bP9;1;0q").expect("an unwrapped Sixel image after a cursor save");
    let up = raw_text[..dcs].rfind("\x1b[").expect("the cursor moves back up");
    let rows: usize = raw_text[up + 2..dcs]
        .split('A')
        .next()
        .and_then(|n| n.parse().ok())
        .expect("CUU before the image");
    assert_eq!(raw_text[..up].matches("\x1bD").count(), rows, "IND once per row");
    let lines: Vec<&str> = text.lines().collect();
    let image_line = lines.iter().position(|l| l.contains("graphics_image[0]=pixels")).unwrap();
    let done_line = lines.iter().position(|l| l.contains("graphics_done")).unwrap();
    assert_eq!(done_line - image_line, rows + 1, "the next text is right below the image:\n{text}");
}
