//! Endpoint construction for test fixtures.
//!
//! A fixture cannot use [`default_local_endpoint`](super::default_local_endpoint):
//! that resolves the one real per-user endpoint, and two concurrent test
//! processes pointed at it would fight over a single daemon. It also cannot
//! hand-build one, because the daemon's ownership checks are the thing under
//! test — weakening them for tests would test a different daemon.
//!
//! [`private_endpoint`] builds an endpoint that satisfies the production policy
//! by construction, on either transport, so a daemon-spawning fixture needs no
//! `cfg(unix)` / `cfg(windows)` branch.
//!
//! Gated behind the `test-support` feature so it cannot reach a shipped binary.

use std::ffi::OsString;
use std::path::Path;

use super::LocalEndpoint;

/// An endpoint isolated to one fixture that a daemon will accept.
///
/// Isolation is keyed on `(parent, name)` on **both** transports, so a fixture
/// gets the same guarantee whichever one it runs over: the same pair always
/// names the same endpoint — a fixture may rebuild it to reconnect — and two
/// fixtures holding different `parent` directories can never collide.
///
/// On Unix this creates a `0700` directory under `parent` and names a socket
/// inside it; `tempfile` makes `0755` directories, which the daemon refuses,
/// so a fixture that dropped a socket straight into a temp root would fail the
/// ownership check rather than exercise it.
///
/// On Windows the pipe namespace is flat, so `parent` cannot be a directory
/// there — it is folded into the pipe name instead. It used to be dropped
/// entirely, leaving the process id as the only discriminator: two fixtures in
/// one process asking for the same `name` (which every daemon pairing test
/// does — `alpha`, `beta`) would then have shared a pipe. Nextest gives each
/// test its own process and hid that, but the isolation a fixture is entitled
/// to should not depend on which runner invoked it.
///
/// ## Panics
///
/// Panics if the Unix private directory cannot be created — a fixture that
/// cannot build its own endpoint has nothing to test.
#[must_use]
pub fn private_endpoint(parent: &Path, name: &str) -> LocalEndpoint {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;

        let dir = parent.join("rendezvous-runtime");
        if !dir.exists() {
            std::fs::DirBuilder::new()
                .mode(0o700)
                .recursive(true)
                .create(&dir)
                .unwrap_or_else(|error| {
                    panic!("create private endpoint dir {}: {error}", dir.display())
                });
        }
        LocalEndpoint::UnixSocket(dir.join(format!("{name}.sock")))
    }
    #[cfg(windows)]
    {
        LocalEndpoint::WindowsNamedPipe(OsString::from(format!(
            "{}{}-test-{}-{:016x}-{name}",
            super::PIPE_NAME_PREFIX,
            super::ENDPOINT_STEM,
            std::process::id(),
            fixture_discriminator(parent),
        )))
    }
}

/// A stable digest of a fixture's `parent` directory, for the flat Windows pipe
/// namespace.
///
/// `DefaultHasher::new()` is keyed with zeros, so the same path yields the same
/// digest in every process — which is what lets a fixture rebuild an endpoint
/// it already bound.
#[cfg(windows)]
fn fixture_discriminator(parent: &Path) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    parent.hash(&mut hasher);
    hasher.finish()
}

/// The `RENDEZVOUS_ENDPOINT` value that resolves back to `endpoint`.
///
/// The inverse of [`LocalEndpoint::from_override`], for a fixture that has to
/// point production code — which resolves its own endpoint — at the one the
/// fixture just built. Lossless on both transports: the override is an
/// OS-native value, so this never goes through `to_string_lossy`.
#[must_use]
pub fn endpoint_env_value(endpoint: &LocalEndpoint) -> OsString {
    match endpoint {
        LocalEndpoint::UnixSocket(path) => path.clone().into_os_string(),
        LocalEndpoint::WindowsNamedPipe(name) => name.clone(),
    }
}
