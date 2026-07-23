//! The transport-neutral half of the boot contract: one shared boot per
//! daemon, a data root that is private before anything opens inside it, and an
//! endpoint the handle can name and release.

use std::path::Path;

use rendezvous_core::local_endpoint::LocalEndpoint;
use tempfile::TempDir;

use rendezvous_core::local_endpoint::test_support::private_endpoint;

use crate::local_transport::{shared_boot_count, spawn_local_server};
use crate::server::{DaemonConfig, ServerError, ServerHandle};

fn ephemeral_config(tmp: &TempDir) -> DaemonConfig {
    DaemonConfig::with_data_dir(tmp.path().join("data"))
        .with_in_memory_projection()
        .without_networking()
}

/// `ServerHandle` owns live tasks and is deliberately not `Debug`, so
/// `expect_err` cannot report the success case.
fn expect_refusal(result: Result<ServerHandle, ServerError>, must: &str) -> ServerError {
    match result {
        Ok(_handle) => panic!("{must}"),
        Err(error) => error,
    }
}

fn spawn(tmp: &TempDir) -> (ServerHandle, LocalEndpoint) {
    let endpoint = private_endpoint(tmp.path(), "daemon");
    let handle =
        spawn_local_server(endpoint.clone(), ephemeral_config(tmp)).expect("spawn daemon");
    (handle, endpoint)
}

/// Whether a daemon is still reachable at `endpoint`.
///
/// The transport-neutral spelling of "the endpoint is bound", so the release
/// contract can be asserted once rather than once per target. The evidence
/// differs because the endpoints do: a Unix socket is a filesystem entry whose
/// presence outlives nothing, while a pipe name resolves exactly as long as an
/// instance handle is open.
fn endpoint_is_bound(endpoint: &LocalEndpoint) -> bool {
    #[cfg(unix)]
    {
        endpoint
            .as_unix_path()
            .expect("private_endpoint yields a Unix socket on Unix")
            .exists()
    }
    #[cfg(windows)]
    {
        tokio::net::windows::named_pipe::ClientOptions::new()
            .open(
                endpoint
                    .as_windows_pipe_name()
                    .expect("private_endpoint yields a named pipe on Windows"),
            )
            .is_ok()
    }
}

/// The reason the transports were split from the boot: a second transport must
/// inherit the storage/identity/network stack, not build a second one. Under
/// nextest each test owns its process, so the process-wide counter measures
/// exactly this daemon.
#[tokio::test]
async fn one_daemon_runs_the_shared_boot_exactly_once() {
    let tmp = TempDir::new().expect("tempdir");
    let before = shared_boot_count();

    let (handle, _socket) = spawn(&tmp);

    assert_eq!(
        shared_boot_count() - before,
        1,
        "portable startup must initialise the shared subsystems once per daemon"
    );
    handle.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn the_handle_reports_the_endpoint_it_bound() {
    let tmp = TempDir::new().expect("tempdir");
    let (handle, endpoint) = spawn(&tmp);

    assert_eq!(handle.local_endpoint(), &endpoint);
    assert!(
        handle.local_endpoint().is_native(),
        "the endpoint must stay typed, not collapse to a display string"
    );
    handle.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn shutdown_releases_the_endpoint() {
    let tmp = TempDir::new().expect("tempdir");
    let (handle, endpoint) = spawn(&tmp);
    assert!(endpoint_is_bound(&endpoint), "the daemon must bind its endpoint");

    handle.shutdown().await.expect("shutdown");

    assert!(
        !endpoint_is_bound(&endpoint),
        "a cleanly stopped daemon must leave no endpoint behind"
    );
}

#[tokio::test]
async fn dropping_the_handle_releases_the_endpoint() {
    let tmp = TempDir::new().expect("tempdir");
    let (handle, endpoint) = spawn(&tmp);
    assert!(endpoint_is_bound(&endpoint));

    drop(handle);
    // Unix unlinks synchronously in the cleanup token, but the named-pipe
    // acceptor's abort lands on the runtime rather than in `drop`.
    tokio::task::yield_now().await;

    assert!(
        !endpoint_is_bound(&endpoint),
        "drop must release the endpoint even without a graceful stop"
    );
}

/// A restarted daemon takes its own endpoint back rather than refusing to boot
/// because the previous run died without unlinking. What counts as reclaimable
/// is the transport's call — see `local_transport::unix`'s tests for the full
/// classification.
#[cfg(unix)]
#[tokio::test]
async fn a_socket_left_by_an_unclean_shutdown_is_reclaimed() {
    let tmp = TempDir::new().expect("tempdir");
    let endpoint = private_endpoint(tmp.path(), "daemon");
    let socket = endpoint
        .as_unix_path()
        .expect("private_endpoint yields a Unix socket on Unix")
        .to_path_buf();
    // A dead listener's socket file, which is what SIGKILL leaves behind.
    drop(std::os::unix::net::UnixListener::bind(&socket).expect("plant a stale socket"));

    let handle = spawn_local_server(endpoint, ephemeral_config(&tmp)).expect("spawn daemon");

    assert!(socket.exists());
    handle.shutdown().await.expect("shutdown");
}

/// An endpoint naming the other target's transport is refused before the boot,
/// so a mistyped `RENDEZVOUS_ENDPOINT` cannot leave a redb lock behind for the
/// next attempt to trip on.
#[tokio::test]
async fn an_incompatible_endpoint_is_rejected_without_booting() {
    let tmp = TempDir::new().expect("tempdir");
    let before = shared_boot_count();

    let error = expect_refusal(
        spawn_local_server(
            LocalEndpoint::WindowsNamedPipe(r"\\.\pipe\claudine-rendezvous-sid-S-1-5-21-7".into()),
            ephemeral_config(&tmp),
        ),
        "a named pipe is not bindable on Unix",
    );

    assert!(matches!(error, ServerError::Endpoint(_)), "got: {error:?}");
    assert_eq!(
        shared_boot_count(),
        before,
        "nothing may be opened for an endpoint that cannot be served"
    );
    assert!(!tmp.path().join("data").exists());
}

/// The data root holds the node-identity seed. A root other users can write is
/// a root they can plant a signing key in, so it is refused rather than
/// silently narrowed.
///
/// Unix-only because the refusal is keyed to the mode bits. Windows has none:
/// there "private" is a DACL, and the check an override has to pass is
/// ownership — see `a_data_root_owned_by_another_account_is_rejected` below.
#[cfg(unix)]
#[tokio::test]
async fn a_group_or_world_accessible_data_root_is_rejected() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().expect("tempdir");
    let data = tmp.path().join("data");
    std::fs::create_dir(&data).expect("create data dir");
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o777)).expect("chmod");
    let before = shared_boot_count();

    let error = expect_refusal(
        spawn_local_server(private_endpoint(tmp.path(), "daemon"), ephemeral_config(&tmp)),
        "a world-writable data root must be refused",
    );

    assert!(matches!(error, ServerError::Ownership(_)), "got: {error:?}");
    assert_eq!(
        shared_boot_count(),
        before,
        "the check must run before the shared boot"
    );
    assert!(
        !data.join("node.key").exists(),
        "no identity seed may be written into an unauthorised root"
    );
    assert!(
        !data.join("session.redb").exists(),
        "no database may be opened in an unauthorised root"
    );
}

/// The same policy applies to an explicit `--data-dir`: an override moves the
/// root, it does not waive the check.
#[cfg(unix)]
#[tokio::test]
async fn an_overridden_data_root_keeps_the_ownership_policy() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().expect("tempdir");
    let override_root = tmp.path().join("operator-chosen");
    std::fs::create_dir(&override_root).expect("create dir");
    std::fs::set_permissions(&override_root, std::fs::Permissions::from_mode(0o755))
        .expect("chmod");

    let config = DaemonConfig::with_data_dir(&override_root)
        .with_in_memory_projection()
        .without_networking();
    let error = expect_refusal(
        spawn_local_server(private_endpoint(tmp.path(), "daemon"), config),
        "an override must not weaken the policy",
    );

    assert!(matches!(error, ServerError::Ownership(_)), "got: {error:?}");
}

/// The Windows spelling of the override rule. There are no mode bits here, so
/// the check an override has to pass is ownership: an owner can rewrite a DACL
/// at will, which makes another account's root untrustworthy however it reads
/// at the moment it is inspected.
#[cfg(windows)]
#[tokio::test]
async fn a_data_root_owned_by_another_account_is_rejected() {
    // %WINDIR% is owned by TrustedInstaller/SYSTEM on every supported Windows,
    // and is the one directory guaranteed to exist and not belong to the test's
    // account.
    let windir = std::env::var_os("WINDIR").expect("every Windows host sets %WINDIR%");
    let tmp = TempDir::new().expect("tempdir");
    let config = DaemonConfig::with_data_dir(Path::new(&windir))
        .with_in_memory_projection()
        .without_networking();
    let before = shared_boot_count();

    let error = expect_refusal(
        spawn_local_server(private_endpoint(tmp.path(), "daemon"), config),
        "a root this account does not own must be refused",
    );

    assert!(matches!(error, ServerError::Ownership(_)), "got: {error:?}");
    assert_eq!(
        shared_boot_count(),
        before,
        "the check must run before the shared boot"
    );
}

/// The durable root is created, not merely accepted, and it comes out with the
/// same owner-only access the endpoint's directory has. This is the positive
/// half of the ownership policy: the tests above prove what is refused, this
/// proves the default the daemon builds for itself passes its own rule.
#[cfg(unix)]
#[tokio::test]
async fn the_data_root_is_created_owner_only() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let tmp = TempDir::new().expect("tempdir");
    let data = tmp.path().join("data");
    assert!(!data.exists(), "the daemon must be the one to create it");

    let (handle, _endpoint) = spawn(&tmp);

    let meta = std::fs::symlink_metadata(&data).expect("metadata");
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o700,
        "the root holding the identity seed must grant no group or other access"
    );
    let sniff::os::StableUserId::UnixUid(me) =
        sniff::os::current_user_id().expect("uid discovery")
    else {
        panic!("a Unix host must report a uid");
    };
    assert_eq!(meta.uid(), me, "the data root must belong to this user");
    assert!(
        data.join("node.key").is_file(),
        "the identity seed must live inside the validated root"
    );

    handle.shutdown().await.expect("shutdown");
}

/// The Windows half of the same positive check: the root the daemon creates
/// passes its own ownership rule, which `ensure_private_dir` re-asserts here
/// rather than this test hand-rolling a second `GetNamedSecurityInfoW`.
#[cfg(windows)]
#[tokio::test]
async fn the_data_root_is_created_owned_by_this_account() {
    let tmp = TempDir::new().expect("tempdir");
    let data = tmp.path().join("data");
    assert!(!data.exists(), "the daemon must be the one to create it");

    let (handle, _endpoint) = spawn(&tmp);

    crate::private_dir::ensure_private_dir(&data)
        .expect("the root the daemon created must pass its own ownership check");
    assert!(
        data.join("node.key").is_file(),
        "the identity seed must live inside the validated root"
    );

    handle.shutdown().await.expect("shutdown");
}

/// A private override is accepted, and the identity seed lands inside it.
#[tokio::test]
async fn a_private_overridden_data_root_is_accepted() {
    let tmp = TempDir::new().expect("tempdir");
    let override_root = tmp.path().join("operator-chosen");
    let config = DaemonConfig::with_data_dir(&override_root)
        .with_in_memory_projection()
        .without_networking();

    let handle =
        spawn_local_server(private_endpoint(tmp.path(), "daemon"), config).expect("spawn daemon");

    assert!(
        override_root.join("node.key").is_file(),
        "the identity seed must live under the configured root"
    );
    assert!(override_root.join("session.redb").is_file());
    handle.shutdown().await.expect("shutdown");
}

/// The old default root was `<tempdir>/rendezvous-data`, which every local user
/// can write. A daemon must never adopt an identity found there — an attacker
/// who planted one would choose the node's signing key.
#[tokio::test]
async fn the_legacy_temp_root_is_neither_selected_nor_read() {
    let legacy = std::env::temp_dir().join("rendezvous-data");
    let legacy_key = legacy.join("node.key");
    // A developer host may already hold real data at the old default. Plant a
    // seed only when it does not, and remove only what this test created.
    let planted = !legacy_key.exists();
    if planted {
        std::fs::create_dir_all(&legacy).expect("create legacy root");
        std::fs::write(&legacy_key, b"planted-seed-that-must-never-be-adopted").expect("plant");
    }
    let legacy_seed = std::fs::read(&legacy_key).expect("read legacy seed");

    let tmp = TempDir::new().expect("tempdir");
    let (handle, _socket) = spawn(&tmp);

    let own_key = tmp.path().join("data").join("node.key");
    assert!(own_key.is_file(), "the daemon must seed its own root");
    assert_ne!(
        std::fs::read(&own_key).expect("read own seed"),
        legacy_seed,
        "the daemon must not import the legacy temp identity"
    );
    assert_eq!(
        std::fs::read(&legacy_key).expect("re-read legacy seed"),
        legacy_seed,
        "the legacy root must not be touched at all"
    );

    handle.shutdown().await.expect("shutdown");
    if planted {
        std::fs::remove_file(&legacy_key).ok();
    }
}

/// `with_data_dir` is the only thing that decides where durable files land, so
/// every one of them must sit under the root the policy validated.
#[test]
fn every_durable_path_sits_under_the_validated_root() {
    let root = Path::new("/srv/claudine/rendezvous");
    let config = DaemonConfig::with_data_dir(root);

    assert_eq!(config.data_dir, root);
    for path in [
        &config.storage_path,
        &config.identity_path,
        config.projection_path.as_ref().expect("projection path"),
    ] {
        assert!(
            path.starts_with(root),
            "{} escapes the validated data root",
            path.display()
        );
    }
}
