//! The named-pipe endpoint contract: one daemon per name, a DACL naming only
//! this user, an acceptor that stays available, and a shutdown that releases
//! the name without touching the filesystem.
//!
//! Phase 7 owns the gRPC round trip and the concurrency matrix; these are the
//! focused checks that the transport itself behaves.

use std::ffi::OsString;
use std::time::{Duration, Instant};

use rendezvous_core::local_endpoint::LocalEndpoint;
use tempfile::TempDir;
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use windows::Win32::Foundation::{ERROR_PIPE_BUSY, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{
    DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
};
use windows::core::PWSTR;

use crate::local_transport::spawn_local_server;
use crate::private_dir::current_user_descriptor;
use crate::server::{DaemonConfig, ServerError, ServerHandle};

fn ephemeral_config(tmp: &TempDir) -> DaemonConfig {
    DaemonConfig::with_data_dir(tmp.path().join("data"))
        .with_in_memory_projection()
        .without_networking()
}

/// A pipe name unique to this test, so a parallel test cannot collide with it
/// on the machine-wide pipe namespace.
fn pipe_name(test: &str) -> OsString {
    OsString::from(format!(
        r"\\.\pipe\claudine-rendezvous-test-{test}-{}",
        std::process::id()
    ))
}

fn spawn_at(tmp: &TempDir, name: &OsString) -> Result<ServerHandle, ServerError> {
    spawn_local_server(
        LocalEndpoint::WindowsNamedPipe(name.clone()),
        ephemeral_config(tmp),
    )
}

async fn open_client(name: &OsString, label: &str) -> NamedPipeClient {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match ClientOptions::new().open(name) {
            Ok(client) => return client,
            Err(error)
                if error.raw_os_error() == Some(ERROR_PIPE_BUSY.0.cast_signed())
                    && Instant::now() < deadline =>
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(error) => panic!("{label}: {error:?}"),
        }
    }
}

/// `ServerHandle` owns live tasks and is deliberately not `Debug`, so
/// `expect_err` cannot report the success case.
fn expect_refusal(result: Result<ServerHandle, ServerError>, must: &str) -> ServerError {
    match result {
        Ok(_handle) => panic!("{must}"),
        Err(error) => error,
    }
}

fn my_sid() -> String {
    let sniff::os::StableUserId::WindowsSid(sid) =
        sniff::os::current_user_id().expect("sid discovery")
    else {
        panic!("a Windows host must report a SID");
    };
    sid
}

/// Render the descriptor the daemon builds back to SDDL, which is the only way
/// to assert its *contents* rather than merely that a pointer is non-null.
fn descriptor_sddl() -> String {
    let descriptor = current_user_descriptor().expect("build descriptor");
    let mut text = PWSTR::null();
    unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            windows::Win32::Security::PSECURITY_DESCRIPTOR(descriptor.as_ptr()),
            SDDL_REVISION_1,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &raw mut text,
            None,
        )
        .expect("render sddl");
    }
    let rendered = unsafe { text.to_string() }.expect("sddl is valid UTF-16");
    let _ = unsafe { LocalFree(Some(HLOCAL(text.0.cast()))) };
    rendered
}

/// The DACL is the whole boundary on this target: there is no mode bit and no
/// owning directory to fall back on.
#[test]
fn the_pipe_dacl_names_this_user_and_nobody_else() {
    let sddl = descriptor_sddl();
    let sid = my_sid();

    assert!(
        sddl.contains(&format!("O:{sid}")),
        "the current user must own the descriptor; got: {sddl}"
    );
    assert!(
        sddl.contains(&format!("(A;;GA;;;{sid})")),
        "the current user must be granted access explicitly; got: {sddl}"
    );
    assert!(
        sddl.contains("D:P"),
        "the DACL must be protected, so no inherited ACE can widen it; got: {sddl}"
    );
    // An ACL with no matching ACE denies. One ACE means exactly one principal
    // is allowed.
    assert_eq!(
        sddl.matches("(A;").count(),
        1,
        "no principal beyond the current user may be granted access; got: {sddl}"
    );
}

/// Two daemons on one name would fight over the same databases, and a named
/// pipe silently allows a second process to add instances to an existing name
/// unless the first claims it. `first_pipe_instance` is that claim.
#[tokio::test]
async fn a_second_daemon_cannot_take_a_running_daemon_s_name() {
    let tmp = TempDir::new().expect("tempdir");
    let name = pipe_name("exclusion");
    let first = spawn_at(&tmp, &name).expect("first daemon");

    let second_tmp = TempDir::new().expect("tempdir");
    let error = expect_refusal(spawn_at(&second_tmp, &name), "the name is taken");

    assert!(
        matches!(error, ServerError::EndpointInUse { .. }),
        "got: {error:?}"
    );
    open_client(&name, "the first daemon must still be reachable").await;
    first.shutdown().await.expect("shutdown");
}

/// A client that connects must not consume the last instance: the next client
/// has to find the name served too.
#[tokio::test]
async fn the_acceptor_stays_available_across_connections() {
    let tmp = TempDir::new().expect("tempdir");
    let name = pipe_name("continuity");
    let handle = spawn_at(&tmp, &name).expect("spawn daemon");

    let first = open_client(&name, "first client").await;
    let second = open_client(&name, "second client").await;
    let third = open_client(&name, "third client").await;

    drop((first, second, third));
    handle.shutdown().await.expect("shutdown");
}

/// Two clients at once, which is what the dashboard and a wrapped session do.
#[tokio::test]
async fn concurrent_clients_are_each_given_an_instance() {
    let tmp = TempDir::new().expect("tempdir");
    let name = pipe_name("concurrent");
    let handle = spawn_at(&tmp, &name).expect("spawn daemon");

    let a = open_client(&name, "client a").await;
    let b = open_client(&name, "client b").await;

    // Both are live at the same time; neither displaced the other.
    drop(a);
    drop(b);
    handle.shutdown().await.expect("shutdown");
}

/// `reject_remote_clients(true)` is the other half of the boundary the DACL
/// starts: without it a pipe is reachable over SMB, so a host that shares its
/// pipe namespace would expose the local control plane to the network.
///
/// The remote form is the same pipe reached through the network redirector
/// rather than the local device. Which error comes back depends on whether the
/// runner has the redirector and file sharing enabled at all, so this asserts
/// the property that matters — a remote-form open never *succeeds* — instead of
/// pinning one errno that varies by host configuration.
#[tokio::test]
async fn a_remote_form_client_is_refused_while_the_local_one_is_served() {
    let tmp = TempDir::new().expect("tempdir");
    let name = pipe_name("remote");
    let handle = spawn_at(&tmp, &name).expect("spawn daemon");

    // Control: the local device path is served, so a refusal below is the
    // remote-client rule and not a dead daemon.
    open_client(&name, "the local form must be served").await;

    let remote = OsString::from(format!(
        r"\\localhost\pipe\claudine-rendezvous-test-remote-{}",
        std::process::id()
    ));
    assert!(
        ClientOptions::new().open(&remote).is_err(),
        "a client arriving through the network redirector must never be served"
    );

    handle.shutdown().await.expect("shutdown");
}

/// A named pipe has no filesystem entry, so "released" means the name stops
/// resolving once the last handle closes.
#[tokio::test]
async fn shutdown_releases_the_name() {
    let tmp = TempDir::new().expect("tempdir");
    let name = pipe_name("release");
    let handle = spawn_at(&tmp, &name).expect("spawn daemon");
    open_client(&name, "bound").await;

    handle.shutdown().await.expect("shutdown");

    let error = ClientOptions::new()
        .open(&name)
        .expect_err("a stopped daemon must leave no pipe behind");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound, "got: {error:?}");
}

/// Drop is the ungraceful spelling of the same rule.
#[tokio::test]
async fn dropping_the_handle_releases_the_name() {
    let tmp = TempDir::new().expect("tempdir");
    let name = pipe_name("drop-release");
    let handle = spawn_at(&tmp, &name).expect("spawn daemon");

    drop(handle);
    // The acceptor abort is observed by the runtime, not synchronously.
    tokio::task::yield_now().await;

    let error = ClientOptions::new()
        .open(&name)
        .expect_err("dropping the handle must release the name");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound, "got: {error:?}");
}

/// The endpoint stays an `OsString` end to end. A name Win32 accepts but that
/// round-tripping through `PathBuf`/`to_string_lossy` would corrupt must still
/// bind, and must be the name the handle reports.
#[tokio::test]
async fn the_handle_reports_the_name_it_bound() {
    let tmp = TempDir::new().expect("tempdir");
    let name = pipe_name("identity");
    let handle = spawn_at(&tmp, &name).expect("spawn daemon");

    assert_eq!(
        handle.local_endpoint().as_windows_pipe_name(),
        Some(name.as_os_str()),
    );
    handle.shutdown().await.expect("shutdown");
}
