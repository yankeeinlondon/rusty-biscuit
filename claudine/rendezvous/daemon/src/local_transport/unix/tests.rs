//! The Unix endpoint contract: a private directory, an owner-only socket, a
//! reclaimable stale socket, and a teardown that removes only its own.
//!
//! Every test drives a real daemon through `spawn_local_server` and inspects
//! the filesystem it left, rather than calling the private helpers: the
//! observable result is what the next daemon and every client actually see.

use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::UnixListener as StdUnixListener;
use std::path::{Path, PathBuf};

use rendezvous_core::local_endpoint::LocalEndpoint;
use tempfile::TempDir;

use super::SocketIdentity;
use crate::local_transport::spawn_local_server;
use crate::private_dir::PrivateDirError;
use crate::server::{DaemonConfig, ServerError, ServerHandle};

fn ephemeral_config(tmp: &TempDir) -> DaemonConfig {
    DaemonConfig::with_data_dir(tmp.path().join("data"))
        .with_in_memory_projection()
        .without_networking()
}

/// A private runtime directory to hold the endpoint.
///
/// `tempfile` creates 0755 directories, which the daemon rightly refuses, so a
/// fixture has to build the private directory the real default resolves into
/// rather than dropping the socket straight into the temp root.
fn runtime_dir(tmp: &TempDir) -> PathBuf {
    let dir = tmp.path().join("runtime");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&dir)
        .expect("create runtime dir");
    dir
}

/// The endpoint inside a ready-made private runtime directory.
fn runtime_socket(tmp: &TempDir) -> PathBuf {
    runtime_dir(tmp).join("daemon.sock")
}

fn spawn_at(tmp: &TempDir, socket: &Path) -> Result<ServerHandle, ServerError> {
    spawn_local_server(
        LocalEndpoint::UnixSocket(socket.to_path_buf()),
        ephemeral_config(tmp),
    )
}

/// `ServerHandle` owns live tasks and is deliberately not `Debug`, so
/// `expect_err` cannot report the success case.
fn expect_refusal(result: Result<ServerHandle, ServerError>, must: &str) -> ServerError {
    match result {
        Ok(_handle) => panic!("{must}"),
        Err(error) => error,
    }
}

fn mode_of(path: &Path) -> u32 {
    std::fs::symlink_metadata(path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777
}

fn my_uid() -> u32 {
    let sniff::os::StableUserId::UnixUid(uid) =
        sniff::os::current_user_id().expect("uid discovery")
    else {
        panic!("a Unix host must report a uid");
    };
    uid
}

/// Leave a socket file with nothing listening on it — exactly what a daemon
/// killed with SIGKILL leaves behind.
fn plant_stale_socket(path: &Path) {
    let listener = StdUnixListener::bind(path).expect("bind stale socket");
    drop(listener);
    assert!(
        std::fs::symlink_metadata(path)
            .expect("the stale socket must survive its listener")
            .file_type()
            .is_socket()
    );
}

/// The runtime directory is the security boundary, so the daemon builds it
/// rather than requiring an operator to have done so.
#[tokio::test]
async fn a_missing_runtime_directory_is_created_owner_only() {
    let tmp = TempDir::new().expect("tempdir");
    let runtime = tmp.path().join("claudine").join("rendezvous");
    let socket = runtime.join("daemon.sock");

    let handle = spawn_at(&tmp, &socket).expect("spawn daemon");

    for dir in [runtime.parent().expect("parent"), runtime.as_path()] {
        assert_eq!(mode_of(dir), 0o700, "{} must be owner-only", dir.display());
        assert_eq!(
            std::fs::symlink_metadata(dir).expect("metadata").uid(),
            my_uid(),
            "{} must belong to this process's user",
            dir.display()
        );
    }
    handle.shutdown().await.expect("shutdown");
}

/// `bind(2)` creates a socket with `0777 & ~umask` and offers no way to ask for
/// less, so an inherited umask of 0 would otherwise leave a world-writable
/// endpoint. The mode must come from the daemon, not from the environment.
#[tokio::test]
async fn the_socket_is_owner_only_whatever_the_umask() {
    let tmp = TempDir::new().expect("tempdir");
    let socket = runtime_socket(&tmp);

    // nextest gives each test its own process, so this cannot leak into another
    // test. Restored anyway, because the assertion should be the thing that
    // fails, not everything after it.
    let previous = unsafe { libc::umask(0) };
    let handle = spawn_at(&tmp, &socket);
    unsafe { libc::umask(previous) };
    let handle = handle.expect("spawn daemon");

    let meta = std::fs::symlink_metadata(&socket).expect("metadata");
    assert!(meta.file_type().is_socket(), "the endpoint must be a socket");
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o600,
        "the socket must grant no group or other access"
    );
    assert_eq!(meta.uid(), my_uid());

    handle.shutdown().await.expect("shutdown");
}

/// A directory the daemon already made on a previous boot is the common case;
/// it must be accepted, not re-created or refused.
#[tokio::test]
async fn an_existing_private_runtime_directory_is_reused() {
    let tmp = TempDir::new().expect("tempdir");
    let runtime = tmp.path().join("claudine").join("rendezvous");
    let socket = runtime.join("daemon.sock");

    let first = spawn_at(&tmp, &socket).expect("first boot");
    first.shutdown().await.expect("shutdown");
    assert!(runtime.is_dir(), "the directory outlives the daemon");

    let second = spawn_at(&tmp, &socket).expect("a second boot must reuse the directory");
    assert_eq!(mode_of(&runtime), 0o700);
    assert_eq!(mode_of(&socket), 0o600);
    second.shutdown().await.expect("shutdown");
}

/// The link's target says nothing about the link: whoever owns the link can
/// re-point it after the check and before the bind.
#[tokio::test]
async fn a_symlinked_parent_is_rejected() {
    let tmp = TempDir::new().expect("tempdir");
    let real = tmp.path().join("real");
    std::fs::create_dir(&real).expect("create dir");
    let link = tmp.path().join("linked");
    std::os::unix::fs::symlink(&real, &link).expect("symlink");

    let error = expect_refusal(
        spawn_at(&tmp, &link.join("daemon.sock")),
        "a symlinked parent must not be traversed",
    );

    assert!(
        matches!(
            error,
            ServerError::Ownership(PrivateDirError::SymlinkComponent { .. })
        ),
        "got: {error:?}"
    );
    assert!(
        !real.join("daemon.sock").exists(),
        "nothing may be bound through the link"
    );
}

/// A world-writable parent is a parent any local user can plant a socket in.
#[tokio::test]
async fn a_shared_parent_directory_is_rejected() {
    let tmp = TempDir::new().expect("tempdir");
    let shared = tmp.path().join("shared");
    std::fs::create_dir(&shared).expect("create dir");
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o777)).expect("chmod");

    let error = expect_refusal(
        spawn_at(&tmp, &shared.join("daemon.sock")),
        "a world-writable endpoint directory must be refused",
    );

    assert!(
        matches!(
            error,
            ServerError::Ownership(PrivateDirError::NotPrivate {
                mode: 0o777,
                ..
            })
        ),
        "got: {error:?}"
    );
}

/// The three wrong types at the endpoint. None is a socket, so none is
/// something an unclean shutdown could have left, so none may be removed.
#[tokio::test]
async fn a_regular_file_at_the_endpoint_is_rejected() {
    let tmp = TempDir::new().expect("tempdir");
    let socket = runtime_socket(&tmp);
    std::fs::write(&socket, b"somebody's data").expect("write file");

    let error = expect_refusal(
        spawn_at(&tmp, &socket),
        "a regular file is not a stale socket",
    );

    let ServerError::EndpointOccupied { found, .. } = &error else {
        panic!("expected EndpointOccupied, got: {error:?}");
    };
    assert_eq!(found, "a regular file");
    assert_eq!(
        std::fs::read(&socket).expect("re-read"),
        b"somebody's data",
        "the file must be left exactly as it was"
    );
}

#[tokio::test]
async fn a_directory_at_the_endpoint_is_rejected() {
    let tmp = TempDir::new().expect("tempdir");
    let socket = runtime_socket(&tmp);
    std::fs::create_dir(&socket).expect("create dir");
    std::fs::write(socket.join("inside"), b"contents").expect("write");

    let error = expect_refusal(spawn_at(&tmp, &socket), "a directory is not a stale socket");

    let ServerError::EndpointOccupied { found, .. } = &error else {
        panic!("expected EndpointOccupied, got: {error:?}");
    };
    assert_eq!(found, "a directory");
    assert!(socket.join("inside").is_file(), "the directory must survive");
}

/// A symlink at the endpoint is the classic redirection attack: unlinking it is
/// harmless, but binding *through* it writes wherever it points.
#[tokio::test]
async fn a_symlink_at_the_endpoint_is_rejected() {
    let tmp = TempDir::new().expect("tempdir");
    let target = tmp.path().join("elsewhere");
    std::fs::write(&target, b"target").expect("write");
    let socket = runtime_socket(&tmp);
    std::os::unix::fs::symlink(&target, &socket).expect("symlink");

    let error = expect_refusal(spawn_at(&tmp, &socket), "a symlink must never be followed");

    let ServerError::EndpointOccupied { found, .. } = &error else {
        panic!("expected EndpointOccupied, got: {error:?}");
    };
    assert_eq!(found, "a symlink");
    assert_eq!(
        std::fs::read(&target).expect("re-read"),
        b"target",
        "the link's target must be untouched"
    );
    assert!(
        std::fs::symlink_metadata(&socket)
            .expect("metadata")
            .file_type()
            .is_symlink(),
        "the link itself must be left for its owner to remove"
    );
}

/// A restarted daemon takes its own endpoint back rather than refusing to boot
/// because the previous run died without unlinking.
#[tokio::test]
async fn a_stale_socket_this_user_owns_is_reclaimed() {
    let tmp = TempDir::new().expect("tempdir");
    let socket = runtime_socket(&tmp);
    plant_stale_socket(&socket);

    let handle = spawn_at(&tmp, &socket).expect("a stale socket must be reclaimed");

    assert_eq!(mode_of(&socket), 0o600, "the reclaimed socket is ours now");
    handle.shutdown().await.expect("shutdown");
}

/// Two daemons on one endpoint would fight over the same databases. The second
/// must lose, and must not evict the first.
#[tokio::test]
async fn a_socket_with_a_live_listener_is_refused() {
    let tmp = TempDir::new().expect("tempdir");
    let socket = runtime_socket(&tmp);
    let listener = StdUnixListener::bind(&socket).expect("bind live socket");
    let before = SocketIdentity::read(&socket).expect("read").expect("socket");

    let error = expect_refusal(spawn_at(&tmp, &socket), "a live endpoint must not be stolen");

    assert!(
        matches!(error, ServerError::EndpointInUse { .. }),
        "got: {error:?}"
    );
    assert_eq!(
        SocketIdentity::read(&socket).expect("read"),
        Some(before),
        "the live daemon's socket must be exactly the one still there"
    );
    drop(listener);
}

/// The daemon-versus-daemon spelling of the same rule, through the real boot
/// path on both sides.
#[tokio::test]
async fn a_second_daemon_cannot_take_a_running_daemon_s_endpoint() {
    let tmp = TempDir::new().expect("tempdir");
    let socket = runtime_socket(&tmp);
    let first = spawn_at(&tmp, &socket).expect("first daemon");

    let second = TempDir::new().expect("tempdir");
    let error = expect_refusal(
        spawn_local_server(
            LocalEndpoint::UnixSocket(socket.clone()),
            ephemeral_config(&second),
        ),
        "the endpoint is taken",
    );

    assert!(
        matches!(error, ServerError::EndpointInUse { .. }),
        "got: {error:?}"
    );
    assert!(socket.exists(), "the first daemon must still be reachable");
    first.shutdown().await.expect("shutdown");
}

/// Teardown is keyed to the socket instance, not the path. If a restarted
/// daemon has already taken the name, unlinking it here would silently take
/// down the daemon that now owns it.
#[tokio::test]
async fn teardown_leaves_a_replacement_socket_alone() {
    let tmp = TempDir::new().expect("tempdir");
    let socket = runtime_socket(&tmp);
    let handle = spawn_at(&tmp, &socket).expect("spawn daemon");

    // Stand in for a restarted daemon that reclaimed the endpoint while this
    // one was still shutting down.
    std::fs::remove_file(&socket).expect("unlink ours");
    let replacement = StdUnixListener::bind(&socket).expect("bind replacement");
    let replacement_id = SocketIdentity::read(&socket).expect("read").expect("socket");

    handle.shutdown().await.expect("shutdown must still succeed");

    assert_eq!(
        SocketIdentity::read(&socket).expect("read"),
        Some(replacement_id),
        "the replacement endpoint must survive our teardown untouched"
    );
    drop(replacement);
}

/// Same rule, drop path: `Drop` cannot report an error, so the check has to be
/// in the cleanup token rather than in `shutdown`.
#[tokio::test]
async fn dropping_the_handle_leaves_a_replacement_socket_alone() {
    let tmp = TempDir::new().expect("tempdir");
    let socket = runtime_socket(&tmp);
    let handle = spawn_at(&tmp, &socket).expect("spawn daemon");

    std::fs::remove_file(&socket).expect("unlink ours");
    let replacement = StdUnixListener::bind(&socket).expect("bind replacement");
    let replacement_id = SocketIdentity::read(&socket).expect("read").expect("socket");

    drop(handle);

    assert_eq!(
        SocketIdentity::read(&socket).expect("read"),
        Some(replacement_id),
    );
    drop(replacement);
}

/// A non-socket that appeared at the path is equally not ours to delete.
#[tokio::test]
async fn teardown_leaves_a_foreign_file_at_the_endpoint_alone() {
    let tmp = TempDir::new().expect("tempdir");
    let socket = runtime_socket(&tmp);
    let handle = spawn_at(&tmp, &socket).expect("spawn daemon");

    std::fs::remove_file(&socket).expect("unlink ours");
    std::fs::write(&socket, b"somebody else's data").expect("write file");

    handle.shutdown().await.expect("shutdown");

    assert_eq!(
        std::fs::read(&socket).expect("re-read"),
        b"somebody else's data",
        "cleanup must not delete data it did not create"
    );
}

/// The endpoint the daemon did bind is removed, so the next boot sees a clean
/// name rather than having to reclaim a stale one.
#[tokio::test]
async fn teardown_removes_the_socket_it_bound() {
    let tmp = TempDir::new().expect("tempdir");
    let socket = runtime_socket(&tmp);
    let handle = spawn_at(&tmp, &socket).expect("spawn daemon");
    assert!(socket.exists());

    handle.shutdown().await.expect("shutdown");

    assert!(!socket.exists());
    assert!(
        socket.parent().expect("parent").is_dir(),
        "the runtime directory is not the daemon's to remove; only the endpoint is"
    );
}

/// A bare relative endpoint has no directory to authorize. It is refused where
/// every other unusable path is refused, rather than resolving against whatever
/// the daemon's working directory happens to be.
#[tokio::test]
async fn a_relative_endpoint_is_rejected() {
    let tmp = TempDir::new().expect("tempdir");
    let error = expect_refusal(
        spawn_at(&tmp, &PathBuf::from("daemon.sock")),
        "a relative endpoint cannot be authorized",
    );

    assert!(
        matches!(
            error,
            ServerError::Ownership(PrivateDirError::NotAbsolute { .. })
        ),
        "got: {error:?}"
    );
}
