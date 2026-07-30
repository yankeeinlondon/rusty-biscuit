//! Stable identity of the OS user the current process runs as.
//!
//! This is the *security principal* — the identifier the kernel authorizes
//! filesystem and socket access against — not a username, display name, or
//! home-directory basename. Usernames are mutable and `$USER`/`%USERNAME%`
//! are caller-controlled, so neither is safe to key private per-user state on.
//!
//! The value is deliberately **not** part of [`OsInfo`](super::OsInfo),
//! `SniffResult`, default `sniff --json` output, or any host-capability cache.
//! A stable account identifier is sensitive and is only produced on demand, by
//! an explicit call to [`current_user_id`].
//!
//! ## Notes
//!
//! Discovery is cheap, synchronous, and uncached. It never spawns a
//! subprocess (`id`, `whoami`, PowerShell, WMI), reads the registry, or
//! touches the network.

use crate::Result;

/// The stable identity of an OS user.
///
/// Variants stay explicit rather than collapsing two unlike identifiers into
/// an untagged string: a Unix UID and a Windows SID are not interchangeable,
/// and a consumer choosing a transport or an ACL must be able to tell them
/// apart without guessing from the text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StableUserId {
    /// A Unix effective user ID. Also the WSL representation — WSL is its own
    /// Linux user and is never correlated with a native Windows account.
    UnixUid(u32),
    /// A Windows account SID in canonical string form (`S-1-...`).
    WindowsSid(String),
}

impl StableUserId {
    /// A lossless, deterministic projection safe to embed in an endpoint name.
    ///
    /// The variant tag is part of the projection, so the value round-trips
    /// back to its variant by inspection and a UID can never be mistaken for
    /// a SID. Output is restricted to ASCII alphanumerics and `-`, which every
    /// supported transport namespace accepts verbatim.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff::os::StableUserId;
    ///
    /// assert_eq!(StableUserId::UnixUid(501).endpoint_component(), "uid-501");
    /// assert_eq!(
    ///     StableUserId::WindowsSid("S-1-5-21-7-8-9-1001".to_string()).endpoint_component(),
    ///     "sid-S-1-5-21-7-8-9-1001"
    /// );
    /// ```
    ///
    /// ## Notes
    ///
    /// This is not a hash. The identifier is reproduced verbatim so that a
    /// human debugging an endpoint can recognize their own account.
    pub fn endpoint_component(&self) -> String {
        match self {
            StableUserId::UnixUid(uid) => format!("uid-{uid}"),
            StableUserId::WindowsSid(sid) => format!("sid-{sid}"),
        }
    }
}

impl std::fmt::Display for StableUserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.endpoint_component())
    }
}

/// Discover the stable identity of the user the current process runs as.
///
/// On Unix this is the effective UID; on Windows it is the account SID read
/// from the current process token.
///
/// ## Examples
///
/// ```
/// let me = sniff::os::current_user_id().expect("the OS always knows who we are");
/// let other = sniff::os::current_user_id().unwrap();
/// assert_eq!(me, other, "identity is stable within a process");
/// ```
///
/// ## Errors
///
/// Returns [`SniffError::UserIdentity`] when an OS call fails and
/// [`SniffError::InvalidUserIdentity`] when the OS returns something that is
/// not a usable account identifier. Failure is never softened into a username,
/// the text `default`, or a process-random value — a caller that cannot learn
/// who it is must not proceed to create per-user private state.
pub fn current_user_id() -> Result<StableUserId> {
    // WSL has no branch of its own: it compiles as Linux and takes the Unix
    // arm, so it reports its own Linux UID and never reaches for the native
    // Windows token.
    #[cfg(unix)]
    {
        // geteuid() is infallible by POSIX contract: it has no failure mode
        // and sets no errno. The Result is for the Windows arm's benefit.
        Ok(StableUserId::UnixUid(unsafe { libc::geteuid() }))
    }
    #[cfg(windows)]
    {
        windows_impl::current_user_id()
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{Result, StableUserId};
    use crate::SniffError;
    use windows::Win32::Foundation::{
        CloseHandle, ERROR_INSUFFICIENT_BUFFER, HANDLE, HLOCAL, LocalFree,
    };
    use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows::Win32::Security::{
        GetTokenInformation, IsValidSid, TOKEN_QUERY, TOKEN_USER, TokenUser,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows::core::PWSTR;

    /// Owns a process-token handle and closes it on every exit path.
    struct TokenHandle(HANDLE);

    impl Drop for TokenHandle {
        fn drop(&mut self) {
            // Nothing actionable remains if the close fails; the process is
            // leaking a handle it can no longer name.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    /// Owns the `LocalAlloc`-backed string `ConvertSidToStringSidW` hands back.
    struct LocalSidString(PWSTR);

    impl Drop for LocalSidString {
        fn drop(&mut self) {
            unsafe { LocalFree(Some(HLOCAL(self.0.0 as *mut _))) };
        }
    }

    pub(super) fn current_user_id() -> Result<StableUserId> {
        let token = open_process_token()?;
        let buffer = read_token_user(&token)?;

        // TOKEN_USER's SID pointer aliases into `buffer`, so every use below
        // must stay inside this scope.
        let token_user = unsafe { &*(buffer.as_ptr() as *const TOKEN_USER) };
        let sid = token_user.User.Sid;

        if !unsafe { IsValidSid(sid) }.as_bool() {
            return Err(SniffError::InvalidUserIdentity {
                operation: "TokenUser",
                message: "process token returned a malformed account SID".to_string(),
            });
        }

        let mut raw = PWSTR::null();
        unsafe { ConvertSidToStringSidW(sid, &mut raw) }
            .map_err(|e| SniffError::user_identity("ConvertSidToStringSidW", e))?;
        let owned = LocalSidString(raw);

        let text = unsafe { owned.0.to_string() }.map_err(|e| SniffError::UserIdentity {
            operation: "ConvertSidToStringSidW",
            source: Box::new(e),
        })?;

        if !text.starts_with("S-1-") {
            return Err(SniffError::InvalidUserIdentity {
                operation: "ConvertSidToStringSidW",
                message: format!("expected a canonical S-1-... SID, got '{text}'"),
            });
        }

        Ok(StableUserId::WindowsSid(text))
    }

    fn open_process_token() -> Result<TokenHandle> {
        let mut handle = HANDLE::default();
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) }
            .map_err(|e| SniffError::user_identity("OpenProcessToken", e))?;
        Ok(TokenHandle(handle))
    }

    /// Size and read the variable-length `TOKEN_USER` buffer.
    fn read_token_user(token: &TokenHandle) -> Result<Vec<u8>> {
        let mut needed = 0u32;
        // The sizing call is expected to fail with ERROR_INSUFFICIENT_BUFFER;
        // any other failure is a real error.
        let sizing = unsafe { GetTokenInformation(token.0, TokenUser, None, 0, &mut needed) };
        if let Err(e) = sizing
            && e.code() != ERROR_INSUFFICIENT_BUFFER.to_hresult()
        {
            return Err(SniffError::user_identity("GetTokenInformation", e));
        }
        if needed == 0 {
            return Err(SniffError::InvalidUserIdentity {
                operation: "GetTokenInformation",
                message: "process token reported a zero-length TokenUser buffer".to_string(),
            });
        }

        let mut buffer = vec![0u8; needed as usize];
        unsafe {
            GetTokenInformation(
                token.0,
                TokenUser,
                Some(buffer.as_mut_ptr() as *mut _),
                needed,
                &mut needed,
            )
        }
        .map_err(|e| SniffError::user_identity("GetTokenInformation", e))?;

        if (needed as usize) < std::mem::size_of::<TOKEN_USER>() {
            return Err(SniffError::InvalidUserIdentity {
                operation: "GetTokenInformation",
                message: format!("TokenUser buffer is too small ({needed} bytes)"),
            });
        }

        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SniffError;

    #[test]
    fn endpoint_component_tags_the_variant() {
        assert_eq!(StableUserId::UnixUid(0).endpoint_component(), "uid-0");
        assert_eq!(StableUserId::UnixUid(501).endpoint_component(), "uid-501");
        assert_eq!(
            StableUserId::UnixUid(u32::MAX).endpoint_component(),
            format!("uid-{}", u32::MAX)
        );
        assert_eq!(
            StableUserId::WindowsSid("S-1-5-18".to_string()).endpoint_component(),
            "sid-S-1-5-18"
        );
    }

    #[test]
    fn endpoint_component_is_lossless_and_variant_distinguishing() {
        // A UID projection can never collide with a SID projection, and each
        // value survives the projection verbatim.
        let uid = StableUserId::UnixUid(1000);
        let sid = StableUserId::WindowsSid("1000".to_string());
        assert_ne!(uid.endpoint_component(), sid.endpoint_component());

        assert_eq!(
            uid.endpoint_component().strip_prefix("uid-"),
            Some("1000"),
            "the UID must be recoverable from its projection"
        );
        assert_eq!(
            sid.endpoint_component().strip_prefix("sid-"),
            Some("1000"),
            "the SID must be recoverable from its projection"
        );
    }

    #[test]
    fn endpoint_component_is_endpoint_name_safe() {
        for id in [
            StableUserId::UnixUid(4294967295),
            StableUserId::WindowsSid("S-1-5-21-3623811015-3361044348-30300820-1013".to_string()),
        ] {
            let projected = id.endpoint_component();
            assert!(
                projected
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "'{projected}' must be safe to embed in a socket path or pipe name"
            );
        }
    }

    #[test]
    fn display_matches_endpoint_projection() {
        let id = StableUserId::UnixUid(501);
        assert_eq!(id.to_string(), id.endpoint_component());

        let id = StableUserId::WindowsSid("S-1-5-18".to_string());
        assert_eq!(id.to_string(), id.endpoint_component());
    }

    #[test]
    fn equality_is_by_variant_and_value() {
        assert_eq!(StableUserId::UnixUid(501), StableUserId::UnixUid(501));
        assert_ne!(StableUserId::UnixUid(501), StableUserId::UnixUid(502));
        assert_eq!(
            StableUserId::WindowsSid("S-1-5-18".to_string()),
            StableUserId::WindowsSid("S-1-5-18".to_string())
        );
        assert_ne!(
            StableUserId::WindowsSid("S-1-5-18".to_string()),
            StableUserId::WindowsSid("S-1-5-19".to_string())
        );
    }

    #[test]
    fn hash_agrees_with_equality() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        assert!(set.insert(StableUserId::UnixUid(501)));
        assert!(
            !set.insert(StableUserId::UnixUid(501)),
            "equal ids must hash to the same bucket"
        );
        assert!(set.insert(StableUserId::WindowsSid("S-1-5-18".to_string())));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn current_user_id_is_stable_across_calls() {
        let first = current_user_id().expect("the OS knows the current user");
        let second = current_user_id().expect("the OS knows the current user");
        assert_eq!(first, second);
        assert_eq!(first.endpoint_component(), second.endpoint_component());
    }

    #[cfg(unix)]
    #[test]
    fn current_user_id_returns_the_effective_uid() {
        let expected = unsafe { libc::geteuid() };
        assert_eq!(
            current_user_id().expect("uid discovery"),
            StableUserId::UnixUid(expected)
        );
    }

    /// WSL compiles and runs through the Unix branch. Asserting the variant on
    /// every Linux host — WSL or not — pins that: a native-Windows-token or
    /// SID-correlating path would produce `WindowsSid` here.
    #[cfg(unix)]
    #[test]
    fn unix_hosts_including_wsl_report_a_uid() {
        assert!(matches!(
            current_user_id().expect("uid discovery"),
            StableUserId::UnixUid(_)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn current_user_id_returns_a_canonical_token_user_sid() {
        let StableUserId::WindowsSid(sid) = current_user_id().expect("sid discovery") else {
            panic!("Windows must report a SID, not a UID");
        };
        assert!(
            sid.starts_with("S-1-"),
            "'{sid}' is not canonical SID string form"
        );
        // An account SID is authority + at least one sub-authority; the
        // well-known short forms we must never return (logon/integrity SIDs
        // are longer, but a bare "S-1-5" would signal we read the wrong field).
        assert!(
            sid.matches('-').count() >= 3,
            "'{sid}' is too short to be an account SID"
        );
    }

    /// `USER`, `LOGNAME`, and `UID` are caller-controlled. Discovery must read
    /// the kernel, so poisoning all three changes nothing.
    #[test]
    fn identity_ignores_username_environment_variables() {
        let _lock = crate::test_helpers::ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let baseline = current_user_id().expect("baseline discovery");

        let mut env = crate::test_helpers::ScopedEnv::new();
        env.set("USER", "not-the-real-user");
        env.set("LOGNAME", "not-the-real-user");
        env.set("UID", "999999");
        env.set("USERNAME", "not-the-real-user");

        assert_eq!(
            current_user_id().expect("poisoned-env discovery"),
            baseline,
            "identity must come from the OS, not the environment"
        );
    }

    /// Absent username variables must not push discovery onto a fallback.
    #[test]
    fn identity_survives_missing_username_environment_variables() {
        let _lock = crate::test_helpers::ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let baseline = current_user_id().expect("baseline discovery");

        let mut env = crate::test_helpers::ScopedEnv::new();
        env.remove("USER");
        env.remove("LOGNAME");
        env.remove("UID");
        env.remove("USERNAME");

        assert_eq!(
            current_user_id().expect("stripped-env discovery"),
            baseline,
            "identity must not depend on username variables being present"
        );
    }

    #[test]
    fn user_identity_error_preserves_operation_and_source() {
        use std::error::Error;

        let err = SniffError::user_identity(
            "OpenProcessToken",
            std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        );
        let msg = err.to_string();
        assert!(msg.contains("OpenProcessToken"), "got: {msg}");
        assert!(
            err.source().is_some(),
            "the OS error must stay in the cause chain"
        );

        let SniffError::UserIdentity { operation, .. } = err else {
            panic!("expected a UserIdentity variant");
        };
        assert_eq!(operation, "OpenProcessToken");
    }

    #[test]
    fn invalid_user_identity_error_reports_the_failed_validation() {
        let err = SniffError::InvalidUserIdentity {
            operation: "TokenUser",
            message: "malformed account SID".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("TokenUser"), "got: {msg}");
        assert!(msg.contains("malformed account SID"), "got: {msg}");
    }
}
