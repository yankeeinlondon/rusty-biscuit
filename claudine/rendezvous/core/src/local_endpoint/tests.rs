//! Unit tests for the local endpoint contract.
//!
//! Environment-mutating tests share one process, so every test that touches
//! `RENDEZVOUS_ENDPOINT`, `XDG_RUNTIME_DIR`, `TMPDIR`, or the username
//! variables holds [`EnvSnapshot`], which serializes them and restores the
//! prior values on drop.

use super::*;

/// Serializes the env-mutating tests in this binary.
fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Every variable that can steer endpoint resolution, captured and cleared on
/// construction and restored on drop. `USER`/`LOGNAME`/`USERNAME` are in the
/// list not because resolution reads them — it must not — but so a test can
/// poison them and prove that.
const TRACKED_ENV: [&str; 6] = [
    ENDPOINT_ENV_VAR,
    "XDG_RUNTIME_DIR",
    "TMPDIR",
    "USER",
    "LOGNAME",
    "USERNAME",
];

struct EnvSnapshot {
    saved: Vec<(&'static str, Option<OsString>)>,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl EnvSnapshot {
    fn new() -> Self {
        let guard = env_guard();
        let saved = TRACKED_ENV
            .iter()
            .map(|key| (*key, env::var_os(key)))
            .collect();
        for key in TRACKED_ENV {
            // SAFETY: serialized by `env_guard`.
            unsafe { env::remove_var(key) };
        }
        Self {
            saved,
            _guard: guard,
        }
    }

    fn set(&self, key: &str, value: impl AsRef<OsStr>) {
        // SAFETY: serialized by `env_guard`.
        unsafe { env::set_var(key, value) };
    }
}

impl Drop for EnvSnapshot {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            // SAFETY: serialized by `env_guard`.
            unsafe {
                match value {
                    Some(v) => env::set_var(key, v),
                    None => env::remove_var(key),
                }
            }
        }
    }
}

fn unix_endpoint(path: &str) -> LocalEndpoint {
    LocalEndpoint::UnixSocket(PathBuf::from(path))
}

fn pipe_endpoint(name: &str) -> LocalEndpoint {
    LocalEndpoint::WindowsNamedPipe(OsString::from(name))
}

// ---------------------------------------------------------------------------
// Enum and accessor behavior
// ---------------------------------------------------------------------------

#[test]
fn accessors_answer_only_for_their_own_variant() {
    let socket = unix_endpoint("/run/user/1000/claudine/rendezvous/daemon.sock");
    assert_eq!(
        socket.as_unix_path(),
        Some(Path::new("/run/user/1000/claudine/rendezvous/daemon.sock"))
    );
    assert_eq!(
        socket.as_windows_pipe_name(),
        None,
        "a Unix socket must never present itself as a pipe name"
    );

    let pipe = pipe_endpoint(r"\\.\pipe\claudine-rendezvous-sid-S-1-5-21-7-8-9-1001");
    assert_eq!(
        pipe.as_windows_pipe_name(),
        Some(OsStr::new(
            r"\\.\pipe\claudine-rendezvous-sid-S-1-5-21-7-8-9-1001"
        ))
    );
    assert_eq!(
        pipe.as_unix_path(),
        None,
        "a pipe name must never present itself as a filesystem path"
    );
}

/// The pipe name is held as an `OsString` end to end. Routing it through
/// `PathBuf`/`to_string_lossy` would replace un-representable bytes with
/// U+FFFD; the accessor must hand back exactly what went in.
#[test]
fn pipe_name_survives_the_round_trip_byte_for_byte() {
    let raw = OsString::from(r"\\.\pipe\claudine-rendezvous-sid-S-1-5-21-1013");
    let endpoint = LocalEndpoint::WindowsNamedPipe(raw.clone());
    assert_eq!(endpoint.as_windows_pipe_name(), Some(raw.as_os_str()));

    let recovered = LocalEndpoint::WindowsNamedPipe(
        endpoint
            .as_windows_pipe_name()
            .expect("pipe variant")
            .to_os_string(),
    );
    assert_eq!(recovered, endpoint);
}

#[test]
fn display_renders_each_variant_without_a_transport_prefix() {
    assert_eq!(unix_endpoint("/tmp/rs/daemon.sock").to_string(), "/tmp/rs/daemon.sock");
    assert_eq!(
        pipe_endpoint(r"\\.\pipe\claudine-rendezvous-sid-S-1-5-18").to_string(),
        r"\\.\pipe\claudine-rendezvous-sid-S-1-5-18"
    );
}

#[test]
fn equality_distinguishes_the_variants() {
    assert_eq!(unix_endpoint("/tmp/a.sock"), unix_endpoint("/tmp/a.sock"));
    assert_ne!(unix_endpoint("/tmp/a.sock"), unix_endpoint("/tmp/b.sock"));
    assert_ne!(
        unix_endpoint(r"\\.\pipe\x"),
        pipe_endpoint(r"\\.\pipe\x"),
        "same text, different transport, must not compare equal"
    );
}

// ---------------------------------------------------------------------------
// Target compatibility
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn unix_accepts_sockets_and_rejects_pipes() {
    let socket = unix_endpoint("/tmp/rs/daemon.sock");
    assert!(socket.is_native());
    assert!(socket.ensure_native().is_ok());

    let pipe = pipe_endpoint(r"\\.\pipe\claudine-rendezvous-sid-S-1-5-18");
    assert!(!pipe.is_native());
    let err = pipe.ensure_native().expect_err("a pipe cannot bind on Unix");
    assert!(
        matches!(err, LocalEndpointError::IncompatibleTransport { .. }),
        "got: {err:?}"
    );
    let msg = err.to_string();
    assert!(msg.contains(r"\\.\pipe\claudine-rendezvous-sid-S-1-5-18"), "got: {msg}");
    assert!(msg.contains("Windows named pipe"), "got: {msg}");
    assert!(msg.contains("Unix domain socket"), "got: {msg}");
}

#[cfg(windows)]
#[test]
fn windows_accepts_pipes_and_rejects_sockets() {
    let pipe = pipe_endpoint(r"\\.\pipe\claudine-rendezvous-sid-S-1-5-18");
    assert!(pipe.is_native());
    assert!(pipe.ensure_native().is_ok());

    let socket = unix_endpoint("/tmp/rs/daemon.sock");
    assert!(!socket.is_native());
    assert!(matches!(
        socket.ensure_native(),
        Err(LocalEndpointError::IncompatibleTransport { .. })
    ));
}

// ---------------------------------------------------------------------------
// RENDEZVOUS_ENDPOINT override parsing
// ---------------------------------------------------------------------------

#[test]
fn empty_override_is_an_error_not_a_request_for_the_default() {
    let err = LocalEndpoint::from_override(OsStr::new("")).expect_err("empty is rejected");
    assert!(matches!(err, LocalEndpointError::EmptyOverride), "got: {err:?}");
    assert!(err.to_string().contains(ENDPOINT_ENV_VAR), "got: {err}");
}

#[test]
fn empty_env_override_does_not_silently_fall_back_to_the_default() {
    let env = EnvSnapshot::new();
    env.set(ENDPOINT_ENV_VAR, "");
    assert!(matches!(
        default_local_endpoint(),
        Err(LocalEndpointError::EmptyOverride)
    ));
}

#[cfg(unix)]
#[test]
fn unix_override_wins_over_the_per_user_default() {
    let env = EnvSnapshot::new();
    env.set(ENDPOINT_ENV_VAR, "/tmp/custom-rs.sock");
    assert_eq!(
        default_local_endpoint().expect("override resolves"),
        unix_endpoint("/tmp/custom-rs.sock")
    );
}

/// A native (unquoted-equivalent) OS value with no Unicode round trip: the
/// override is an `OsStr`, so a non-UTF-8 path must survive verbatim.
#[cfg(unix)]
#[test]
fn unix_override_preserves_non_utf8_bytes() {
    use std::os::unix::ffi::OsStrExt;

    let raw = OsStr::from_bytes(b"/tmp/rs-\xff\xfe/daemon.sock");
    let endpoint = LocalEndpoint::from_override(raw).expect("non-UTF-8 paths are legal on Unix");
    assert_eq!(
        endpoint.as_unix_path().map(|p| p.as_os_str()),
        Some(raw),
        "bytes must not be lossily replaced"
    );
}

#[cfg(unix)]
#[test]
fn unix_override_rejects_a_windows_pipe_name() {
    let err = LocalEndpoint::from_override(OsStr::new(r"\\.\pipe\claudine-rendezvous"))
        .expect_err("a pipe name is not a Unix socket");
    let LocalEndpointError::IncompatibleTransport { actual, expected, .. } = err else {
        panic!("expected IncompatibleTransport, got: {err:?}");
    };
    assert_eq!(actual, "a Windows named pipe");
    assert_eq!(expected, "a Unix domain socket");
}

/// Win32 folds the `pipe` keyword case-insensitively, so a mixed-case pipe
/// name must be recognized as a pipe rather than mistaken for a relative path.
#[cfg(unix)]
#[test]
fn unix_override_rejects_pipe_names_in_every_recognized_form() {
    for candidate in [
        r"\\.\pipe\rs",
        r"\\.\PIPE\rs",
        r"\\?\pipe\rs",
        r"\\.\Pipe\rs",
        r"\\.\pipe\",
    ] {
        let result = LocalEndpoint::from_override(OsStr::new(candidate));
        assert!(
            matches!(result, Err(LocalEndpointError::IncompatibleTransport { .. })),
            "'{candidate}' must be recognized as a pipe name, got: {result:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn unix_override_rejects_a_relative_path() {
    let err = LocalEndpoint::from_override(OsStr::new("relative/daemon.sock"))
        .expect_err("a relative socket path resolves differently per process");
    let LocalEndpointError::MalformedOverride { value, expected, .. } = err else {
        panic!("expected MalformedOverride, got: {err:?}");
    };
    assert_eq!(value, "relative/daemon.sock");
    assert_eq!(expected, "Unix socket path");
}

#[cfg(unix)]
#[test]
fn unix_override_rejects_a_bare_root() {
    let err = LocalEndpoint::from_override(OsStr::new("/"))
        .expect_err("the root directory does not name a socket file");
    assert!(
        matches!(err, LocalEndpointError::MalformedOverride { .. }),
        "got: {err:?}"
    );
}

#[cfg(windows)]
#[test]
fn windows_override_wins_and_stays_a_pipe_name() {
    let env = EnvSnapshot::new();
    env.set(ENDPOINT_ENV_VAR, r"\\.\pipe\custom-rs");
    assert_eq!(
        default_local_endpoint().expect("override resolves"),
        pipe_endpoint(r"\\.\pipe\custom-rs")
    );
}

#[cfg(windows)]
#[test]
fn windows_override_rejects_a_filesystem_path() {
    let err = LocalEndpoint::from_override(OsStr::new(r"C:\Users\me\daemon.sock"))
        .expect_err("a drive path is not a named pipe");
    let LocalEndpointError::IncompatibleTransport { actual, expected, .. } = err else {
        panic!("expected IncompatibleTransport, got: {err:?}");
    };
    assert_eq!(actual, "a filesystem path");
    assert_eq!(expected, "a Windows named pipe");
}

#[cfg(windows)]
#[test]
fn windows_override_rejects_malformed_pipe_names() {
    for candidate in [r"\\.\pipe\", r"\\.\pipe\nested\name"] {
        let err = LocalEndpoint::from_override(OsStr::new(candidate))
            .expect_err("malformed pipe names are rejected");
        assert!(
            matches!(err, LocalEndpointError::MalformedOverride { .. }),
            "'{candidate}' got: {err:?}"
        );
    }

    let too_long = format!(r"\\.\pipe\{}", "x".repeat(256));
    assert!(matches!(
        LocalEndpoint::from_override(OsStr::new(&too_long)),
        Err(LocalEndpointError::MalformedOverride { .. })
    ));
}

// ---------------------------------------------------------------------------
// Per-user defaults
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn unix_default_falls_back_to_a_uid_qualified_private_temp_directory() {
    let env = EnvSnapshot::new();
    let tmp = tempfile::tempdir().expect("tempdir");
    env.set("TMPDIR", tmp.path());

    let uid = unsafe { libc::geteuid() };
    assert_eq!(
        default_local_endpoint().expect("default resolves"),
        LocalEndpoint::UnixSocket(
            tmp.path()
                .join(format!("claudine-rendezvous-uid-{uid}"))
                .join("daemon.sock")
        ),
        "with no runtime directory the endpoint is qualified by the effective UID"
    );
}

#[cfg(unix)]
#[test]
fn unix_default_uses_a_private_xdg_runtime_dir_when_one_is_usable() {
    let env = EnvSnapshot::new();
    let runtime = tempfile::tempdir().expect("tempdir");
    // tempfile creates 0700 directories owned by the current user, which is
    // exactly the shape a real $XDG_RUNTIME_DIR has.
    env.set("XDG_RUNTIME_DIR", runtime.path());

    assert_eq!(
        default_local_endpoint().expect("default resolves"),
        LocalEndpoint::UnixSocket(
            runtime
                .path()
                .join("claudine")
                .join("rendezvous")
                .join("daemon.sock")
        )
    );
}

#[cfg(unix)]
#[test]
fn unix_default_prefers_the_runtime_dir_over_tmpdir() {
    let env = EnvSnapshot::new();
    let runtime = tempfile::tempdir().expect("tempdir");
    let tmp = tempfile::tempdir().expect("tempdir");
    env.set("XDG_RUNTIME_DIR", runtime.path());
    env.set("TMPDIR", tmp.path());

    let endpoint = default_local_endpoint().expect("default resolves");
    let path = endpoint.as_unix_path().expect("unix variant");
    assert!(path.starts_with(runtime.path()), "got: {}", path.display());
}

/// A group/other-writable runtime directory is not a private one. It is not an
/// error — resolution treats it as *unavailable* and falls back — but the
/// endpoint must not be placed inside it.
#[cfg(unix)]
#[test]
fn unix_default_rejects_a_world_writable_runtime_dir_and_falls_back() {
    use std::os::unix::fs::PermissionsExt;

    let env = EnvSnapshot::new();
    let runtime = tempfile::tempdir().expect("tempdir");
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::set_permissions(runtime.path(), std::fs::Permissions::from_mode(0o777))
        .expect("widen runtime dir");
    env.set("XDG_RUNTIME_DIR", runtime.path());
    env.set("TMPDIR", tmp.path());

    let uid = unsafe { libc::geteuid() };
    assert_eq!(
        default_local_endpoint().expect("default resolves"),
        LocalEndpoint::UnixSocket(
            tmp.path()
                .join(format!("claudine-rendezvous-uid-{uid}"))
                .join("daemon.sock")
        )
    );
}

#[cfg(unix)]
#[test]
fn unix_default_rejects_unusable_runtime_dirs_and_falls_back() {
    use std::os::unix::fs::PermissionsExt;

    let holder = tempfile::tempdir().expect("tempdir");
    let real = holder.path().join("real");
    std::fs::create_dir(&real).expect("create dir");
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o700)).expect("tighten");

    let symlink = holder.path().join("link");
    std::os::unix::fs::symlink(&real, &symlink).expect("symlink");

    let regular_file = holder.path().join("file");
    std::fs::write(&regular_file, b"not a directory").expect("write file");

    let cases: [(&str, PathBuf); 4] = [
        ("a symlink can be re-pointed after the check", symlink),
        ("a regular file is not a directory", regular_file),
        ("a missing directory", holder.path().join("absent")),
        ("a relative path", PathBuf::from("relative/runtime")),
    ];

    let uid = unsafe { libc::geteuid() };
    for (why, candidate) in cases {
        let env = EnvSnapshot::new();
        let tmp = tempfile::tempdir().expect("tempdir");
        env.set("XDG_RUNTIME_DIR", &candidate);
        env.set("TMPDIR", tmp.path());

        assert_eq!(
            default_local_endpoint().expect("default resolves"),
            LocalEndpoint::UnixSocket(
                tmp.path()
                    .join(format!("claudine-rendezvous-uid-{uid}"))
                    .join("daemon.sock")
            ),
            "{why}: resolution must fall back, not use {}",
            candidate.display()
        );
    }
}

#[cfg(unix)]
#[test]
fn empty_xdg_runtime_dir_is_treated_as_unset() {
    let env = EnvSnapshot::new();
    let tmp = tempfile::tempdir().expect("tempdir");
    env.set("XDG_RUNTIME_DIR", "");
    env.set("TMPDIR", tmp.path());

    let endpoint = default_local_endpoint().expect("default resolves");
    let path = endpoint.as_unix_path().expect("unix variant");
    assert!(path.starts_with(tmp.path()), "got: {}", path.display());
}

#[cfg(windows)]
#[test]
fn windows_default_is_a_sid_qualified_pipe() {
    let _env = EnvSnapshot::new();
    let sniff::os::StableUserId::WindowsSid(sid) =
        sniff::os::current_user_id().expect("sid discovery")
    else {
        panic!("Windows must report a SID");
    };

    assert_eq!(
        default_local_endpoint().expect("default resolves"),
        pipe_endpoint(&format!(r"\\.\pipe\claudine-rendezvous-sid-{sid}"))
    );
}

/// `$USER`/`$LOGNAME`/`%USERNAME%` are caller-controlled. The default is keyed
/// on the kernel-reported principal, so poisoning them must change nothing —
/// this is the specific defect the old `$USER`-derived path had.
#[test]
fn default_ignores_username_environment_variables() {
    let env = EnvSnapshot::new();
    let tmp = tempfile::tempdir().expect("tempdir");
    env.set("TMPDIR", tmp.path());
    let baseline = default_local_endpoint().expect("baseline resolves");

    for name in ["attacker", "", "root"] {
        env.set("USER", name);
        env.set("LOGNAME", name);
        env.set("USERNAME", name);
        assert_eq!(
            default_local_endpoint().expect("poisoned-env resolves"),
            baseline,
            "USER='{name}' must not steer the endpoint"
        );
    }

    assert!(
        !baseline.to_string().contains("attacker"),
        "no username may appear in the endpoint: {baseline}"
    );
    assert!(
        !baseline.to_string().contains("default"),
        "there is no 'default' fallback identity: {baseline}"
    );
}

#[test]
fn default_is_stable_across_calls() {
    let env = EnvSnapshot::new();
    let tmp = tempfile::tempdir().expect("tempdir");
    env.set("TMPDIR", tmp.path());

    let first = default_local_endpoint().expect("first resolve");
    let second = default_local_endpoint().expect("second resolve");
    assert_eq!(first, second);
}

#[test]
fn default_endpoint_is_native_to_this_target() {
    let env = EnvSnapshot::new();
    let tmp = tempfile::tempdir().expect("tempdir");
    env.set("TMPDIR", tmp.path());

    default_local_endpoint()
        .expect("default resolves")
        .ensure_native()
        .expect("the per-user default must always be bindable here");
}

/// Resolution is inspection-only: the daemon owns every mkdir/chmod/unlink, so
/// nothing may exist on disk merely because an endpoint was named.
#[test]
fn resolution_does_not_touch_the_filesystem() {
    let env = EnvSnapshot::new();
    let tmp = tempfile::tempdir().expect("tempdir");
    env.set("TMPDIR", tmp.path());
    env.set("XDG_RUNTIME_DIR", tmp.path());

    let endpoint = default_local_endpoint().expect("default resolves");
    let Some(path) = endpoint.as_unix_path() else {
        return; // Windows pipe names have no filesystem presence at all.
    };

    assert!(!path.exists(), "the endpoint itself must not be created");
    let parent = path.parent().expect("endpoint has a parent");
    assert!(
        !parent.exists(),
        "the runtime directory must not be created: {}",
        parent.display()
    );
    assert_eq!(
        std::fs::read_dir(tmp.path())
            .expect("read tempdir")
            .count(),
        0,
        "resolution created something under the temp root"
    );
}

// ---------------------------------------------------------------------------
// Identity mismatch
// ---------------------------------------------------------------------------

/// Unreachable through `current_user_id`, but the derivation must fail closed
/// rather than embed a SID in a socket path (or a UID in a pipe name) if a
/// future identity source ever hands back the wrong variant.
#[cfg(unix)]
#[test]
fn unix_derivation_refuses_a_windows_identity() {
    let err = default_unix_endpoint(&StableUserId::WindowsSid("S-1-5-18".to_string()))
        .expect_err("a SID cannot qualify a Unix socket");
    let LocalEndpointError::MismatchedIdentity { actual, expected } = err else {
        panic!("expected MismatchedIdentity, got: {err:?}");
    };
    assert_eq!(actual, "a Windows SID");
    assert_eq!(expected, "a Unix domain socket");
}

#[cfg(windows)]
#[test]
fn windows_derivation_refuses_a_unix_identity() {
    let err = default_windows_endpoint(&StableUserId::UnixUid(501))
        .expect_err("a UID cannot qualify a named pipe");
    assert!(
        matches!(err, LocalEndpointError::MismatchedIdentity { .. }),
        "got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Error surface
// ---------------------------------------------------------------------------

#[test]
fn user_identity_failure_keeps_the_sniff_error_in_the_cause_chain() {
    use std::error::Error;

    let err = LocalEndpointError::from(sniff::SniffError::InvalidUserIdentity {
        operation: "TokenUser",
        message: "malformed account SID".to_string(),
    });
    assert!(
        err.to_string().contains("stable OS user identity"),
        "got: {err}"
    );
    let source = err.source().expect("the sniff error must survive");
    assert!(source.to_string().contains("TokenUser"), "got: {source}");
}
