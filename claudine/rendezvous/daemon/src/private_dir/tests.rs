use super::*;

fn temp_root() -> tempfile::TempDir {
    tempfile::tempdir().expect("temp dir")
}

#[test]
fn default_data_dir_is_the_platform_local_data_directory() {
    let expected = dirs::data_local_dir()
        .expect("every supported platform reports a local data dir")
        .join("claudine")
        .join("rendezvous");
    assert_eq!(default_data_dir().expect("default data dir"), expected);
}

/// The old default put the node-identity seed in a directory every local user
/// can write. Nothing may select it, so the check is on the value the daemon
/// actually uses rather than on the absence of a string in the source.
#[test]
fn default_data_dir_is_not_the_legacy_temp_root() {
    let root = default_data_dir().expect("default data dir");
    let legacy = std::env::temp_dir().join("rendezvous-data");

    assert_ne!(root, legacy);
    assert!(
        !root.starts_with(std::env::temp_dir()),
        "{} must not live under the shared temp directory",
        root.display()
    );
}

#[test]
fn ensure_private_dir_creates_missing_components() {
    let root = temp_root();
    let target = root.path().join("claudine").join("rendezvous");

    ensure_private_dir(&target).expect("create private dir");

    assert!(target.is_dir());
    assert!(
        target.parent().expect("parent").is_dir(),
        "intermediate components must be created too"
    );
}

#[cfg(unix)]
#[test]
fn created_directories_are_owner_only() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let root = temp_root();
    let target = root.path().join("claudine").join("rendezvous");

    ensure_private_dir(&target).expect("create private dir");

    let sniff::os::StableUserId::UnixUid(me) = sniff::os::current_user_id().expect("uid discovery")
    else {
        panic!("a Unix host must report a uid");
    };
    for dir in [target.parent().expect("parent"), target.as_path()] {
        let meta = std::fs::metadata(dir).expect("metadata");
        assert_eq!(
            meta.permissions().mode() & 0o777,
            0o700,
            "{} must be created owner-only",
            dir.display()
        );
        assert_eq!(
            meta.uid(),
            me,
            "{} must belong to this process's user",
            dir.display()
        );
    }
}

#[test]
fn ensure_private_dir_is_idempotent() {
    let root = temp_root();
    let target = root.path().join("claudine").join("rendezvous");

    ensure_private_dir(&target).expect("first call");
    ensure_private_dir(&target).expect("second call must accept its own directory");
    ensure_private_dir(&target).expect("third call");

    assert!(target.is_dir());
}

#[test]
fn relative_paths_are_rejected() {
    let error = ensure_private_dir(Path::new("relative/rendezvous")).expect_err("must reject");
    assert!(
        matches!(error, PrivateDirError::NotAbsolute { .. }),
        "got: {error:?}"
    );
}

#[test]
fn a_file_where_the_directory_belongs_is_rejected() {
    let root = temp_root();
    let target = root.path().join("rendezvous");
    std::fs::write(&target, b"not a directory").expect("write file");

    let error = ensure_private_dir(&target).expect_err("must reject");
    assert!(
        matches!(error, PrivateDirError::NotADirectory { .. }),
        "got: {error:?}"
    );
}

#[test]
fn a_file_in_a_parent_component_is_rejected() {
    let root = temp_root();
    let blocker = root.path().join("claudine");
    std::fs::write(&blocker, b"not a directory").expect("write file");

    let error = ensure_private_dir(&blocker.join("rendezvous")).expect_err("must reject");
    let PrivateDirError::NotADirectory { path } = error else {
        panic!("expected NotADirectory, got: {error:?}");
    };
    assert_eq!(path, blocker, "the error must name the offending component");
}

#[cfg(unix)]
#[test]
fn a_symlinked_target_is_rejected() {
    let root = temp_root();
    let real = root.path().join("real");
    std::fs::create_dir(&real).expect("create real dir");
    let link = root.path().join("rendezvous");
    std::os::unix::fs::symlink(&real, &link).expect("symlink");

    let error = ensure_private_dir(&link).expect_err("must reject");
    let PrivateDirError::SymlinkComponent { path } = error else {
        panic!("expected SymlinkComponent, got: {error:?}");
    };
    assert_eq!(path, link);
}

/// A symlinked parent is the interesting case: the link's target can be
/// re-pointed after the check, so its ownership proves nothing about where the
/// daemon would actually write.
#[cfg(unix)]
#[test]
fn a_symlinked_parent_component_is_rejected() {
    let root = temp_root();
    let real = root.path().join("real");
    std::fs::create_dir(&real).expect("create real dir");
    let link = root.path().join("claudine");
    std::os::unix::fs::symlink(&real, &link).expect("symlink");

    let error = ensure_private_dir(&link.join("rendezvous")).expect_err("must reject");
    let PrivateDirError::SymlinkComponent { path } = error else {
        panic!("expected SymlinkComponent, got: {error:?}");
    };
    assert_eq!(path, link, "the error must name the symlinked component");
}

/// A pre-existing directory this user owns but shares is not made private
/// silently: widening or narrowing someone's directory behind their back is
/// worse than refusing to use it.
#[cfg(unix)]
#[test]
fn an_existing_group_or_world_accessible_directory_is_rejected() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_root();
    let target = root.path().join("rendezvous");
    std::fs::create_dir(&target).expect("create dir");
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let error = ensure_private_dir(&target).expect_err("must reject");
    let PrivateDirError::NotPrivate { path, mode } = error else {
        panic!("expected NotPrivate, got: {error:?}");
    };
    assert_eq!(path, target);
    assert_eq!(mode, 0o755, "the error must report the mode it found");
}

/// Group access alone is enough to reject: the boundary is the owner, not
/// "world".
#[cfg(unix)]
#[test]
fn an_existing_group_readable_directory_is_rejected() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_root();
    let target = root.path().join("rendezvous");
    std::fs::create_dir(&target).expect("create dir");
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o740)).expect("chmod");

    assert!(
        matches!(
            ensure_private_dir(&target),
            Err(PrivateDirError::NotPrivate { mode: 0o740, .. })
        ),
        "0740 grants the group read and execute"
    );
}

/// Windows has no mode bits, so "private" is a DACL naming one principal. The
/// descriptor is asserted through its SDDL rendering, which is the only way to
/// see its contents rather than merely that a pointer is non-null.
#[cfg(windows)]
#[test]
fn the_current_user_descriptor_names_this_account_and_nobody_else() {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Authorization::{
        ConvertSecurityDescriptorToStringSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows::Win32::Security::{
        DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    };
    use windows::core::PWSTR;

    let descriptor = current_user_descriptor().expect("build descriptor");
    let mut text = PWSTR::null();
    unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            PSECURITY_DESCRIPTOR(descriptor.as_ptr()),
            SDDL_REVISION_1,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &raw mut text,
            None,
        )
        .expect("render sddl");
    }
    let sddl = unsafe { text.to_string() }.expect("sddl is valid UTF-16");
    let _ = unsafe { LocalFree(Some(HLOCAL(text.0.cast()))) };

    let sniff::os::StableUserId::WindowsSid(sid) =
        sniff::os::current_user_id().expect("sid discovery")
    else {
        panic!("a Windows host must report a SID");
    };

    assert!(sddl.contains(&format!("O:{sid}")), "got: {sddl}");
    assert!(sddl.contains(&format!("(A;;GA;;;{sid})")), "got: {sddl}");
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

/// The directories the daemon creates must come out owned by this account,
/// whatever the parent container's inheritable ACEs say.
#[cfg(windows)]
#[test]
fn created_directories_are_owned_by_this_account() {
    let root = temp_root();
    let target = root.path().join("claudine").join("rendezvous");

    ensure_private_dir(&target).expect("create private dir");

    // `ensure_private_dir` verifies ownership of the target itself, so reaching
    // here already proves it; the intermediate component is the one worth
    // re-checking, since nothing else asserts it.
    assert!(target.is_dir());
    ensure_private_dir(target.parent().expect("parent"))
        .expect("intermediate components must belong to this account too");
}

/// The same policy an override has to pass. A root this account does not own is
/// a root whose DACL its owner can rewrite at any time.
#[cfg(windows)]
#[test]
fn a_directory_owned_by_another_account_is_rejected() {
    // `%WINDIR%` is owned by TrustedInstaller/SYSTEM on every supported
    // Windows, and is the one directory guaranteed to exist and not belong to
    // the test's account.
    let Some(windir) = std::env::var_os("WINDIR").map(PathBuf::from) else {
        panic!("every Windows host sets %WINDIR%");
    };

    let error = ensure_private_dir(&windir).expect_err("must reject");
    assert!(
        matches!(error, PrivateDirError::ForeignAccount { .. }),
        "got: {error:?}"
    );
}

/// A shared ancestor is fine — `/tmp` is one, and so is the user's data
/// directory. Only the components the daemon creates must be private.
#[cfg(unix)]
#[test]
fn a_shared_ancestor_is_accepted() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_root();
    let shared = root.path().join("shared");
    std::fs::create_dir(&shared).expect("create dir");
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o777)).expect("chmod");

    let target = shared.join("claudine").join("rendezvous");
    ensure_private_dir(&target).expect("a world-writable ancestor must not block a private child");

    assert_eq!(
        std::fs::metadata(&target)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777,
        0o700,
    );
}
