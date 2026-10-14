use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::*;

/// A source tree with one nested file, the way a checkout has content below
/// its root.
fn source_tree(root: &Path) -> PathBuf {
    let source = root.join("checkout");
    fs::create_dir_all(source.join("sub")).unwrap();
    fs::write(source.join("sub").join("file.txt"), b"payload").unwrap();
    source
}

fn assert_source_intact(source: &Path) {
    assert_eq!(fs::read(source.join("sub").join("file.txt")).unwrap(), b"payload");
}

/// Creates a link to a target that does not exist. `None` where this host may
/// not create links (Windows without Developer Mode).
fn dangling_link(link: &Path) -> Option<()> {
    let target = link.with_file_name("no-such-target");
    #[cfg(unix)]
    let created = std::os::unix::fs::symlink(&target, link);
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_dir(&target, link);
    match created {
        Ok(()) => Some(()),
        Err(error) => {
            eprintln!("skipping the link case: this host cannot create a link ({error})");
            None
        }
    }
}

#[test]
fn an_absent_destination_receives_the_tree_with_its_identity() {
    let root = tempfile::tempdir().unwrap();
    let source = source_tree(root.path());
    let destination = root.path().join(".wt-remove-checkout-token");
    let before = identity(&source).unwrap();

    rename_no_replace(&source, &destination).unwrap();

    assert!(fs::symlink_metadata(&source).is_err(), "the original path is gone");
    assert_source_intact(&destination);
    assert_eq!(identity(&destination).unwrap(), before);
}

#[test]
fn an_existing_destination_is_never_replaced() {
    let root = tempfile::tempdir().unwrap();
    let source = source_tree(root.path());
    let source_identity = identity(&source).unwrap();

    type Make = fn(&Path) -> Option<()>;
    type Check = fn(&Path);
    let cases: [(&str, Make, Check); 4] = [
        (
            "empty directory",
            |at| fs::create_dir(at).ok(),
            |at| assert_eq!(fs::read_dir(at).unwrap().count(), 0),
        ),
        (
            "file",
            |at| fs::write(at, b"theirs").ok(),
            |at| assert_eq!(fs::read(at).unwrap(), b"theirs"),
        ),
        (
            "non-empty directory",
            |at| {
                fs::create_dir(at).unwrap();
                fs::write(at.join("theirs.txt"), b"theirs").ok()
            },
            |at| assert_eq!(fs::read(at.join("theirs.txt")).unwrap(), b"theirs"),
        ),
        (
            "dangling link",
            dangling_link,
            |at| assert_eq!(fs::read_link(at).unwrap(), at.with_file_name("no-such-target")),
        ),
    ];

    for (shape, make, check) in cases {
        let destination = root.path().join(format!("taken {shape}"));
        if make(&destination).is_none() {
            continue;
        }
        let existing = identity(&destination).unwrap();

        let error = rename_no_replace(&source, &destination).unwrap_err();

        assert_eq!(error.kind, RenameFailure::Collision, "{shape}: {error}");
        assert_source_intact(&source);
        assert_eq!(identity(&source).unwrap(), source_identity, "{shape}: the source stayed");
        assert_eq!(identity(&destination).unwrap(), existing, "{shape}: the destination stayed");
        check(&destination);
    }
}

#[test]
fn a_missing_source_fails_as_other_and_creates_nothing() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("destination");

    let error = rename_no_replace(&root.path().join("vanished"), &destination).unwrap_err();

    assert_eq!(error.kind, RenameFailure::Other);
    assert_eq!(error.source.kind(), io::ErrorKind::NotFound);
    assert!(fs::symlink_metadata(&destination).is_err());
}

#[test]
fn identity_names_a_link_itself_not_its_target() {
    let root = tempfile::tempdir().unwrap();
    let source = source_tree(root.path());
    let link = root.path().join("link");
    #[cfg(unix)]
    let created = std::os::unix::fs::symlink(&source, &link);
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_dir(&source, &link);
    if let Err(error) = created {
        eprintln!("skipping: this host cannot create a link ({error})");
        return;
    }

    assert_ne!(identity(&link).unwrap(), identity(&source).unwrap());
    assert_eq!(identity(&root.path().join("missing")).unwrap_err().kind(), io::ErrorKind::NotFound);
}

#[test]
fn a_link_is_moved_as_itself() {
    let root = tempfile::tempdir().unwrap();
    let source = source_tree(root.path());
    let link = root.path().join("link");
    #[cfg(unix)]
    let created = std::os::unix::fs::symlink(&source, &link);
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_dir(&source, &link);
    if let Err(error) = created {
        eprintln!("skipping: this host cannot create a link ({error})");
        return;
    }
    let moved = root.path().join("moved");

    rename_no_replace(&link, &moved).unwrap();

    assert_eq!(fs::read_link(&moved).unwrap(), source);
    assert_source_intact(&source);
}

#[cfg(unix)]
#[test]
fn a_parent_this_user_cannot_write_is_a_permission_failure() {
    use std::os::unix::fs::PermissionsExt;

    if unsafe { libc::geteuid() } == 0 {
        eprintln!("skipping: root may write any directory");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("parent");
    fs::create_dir(&parent).unwrap();
    let source = source_tree(&parent);
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o555)).unwrap();

    let result = rename_no_replace(&source, &parent.join("moved"));

    fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(result.unwrap_err().kind, RenameFailure::Permission);
    assert_source_intact(&source);
    assert!(fs::symlink_metadata(parent.join("moved")).is_err());
}

#[cfg(unix)]
#[test]
fn a_path_with_a_nul_byte_fails_as_other() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let root = tempfile::tempdir().unwrap();
    let source = source_tree(root.path());

    let error = rename_no_replace(&source, &root.path().join(OsStr::from_bytes(b"bad\0name"))).unwrap_err();

    assert_eq!(error.kind, RenameFailure::Other);
    assert_eq!(error.source.kind(), io::ErrorKind::InvalidInput);
    assert_source_intact(&source);
}

/// macOS (APFS) refuses names that are not UTF-8, so this runs on Linux only.
#[cfg(target_os = "linux")]
#[test]
fn a_name_that_is_not_utf8_moves_losslessly() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let root = tempfile::tempdir().unwrap();
    let source = root.path().join(OsStr::from_bytes(b"check\xffout"));
    fs::create_dir_all(source.join("sub")).unwrap();
    fs::write(source.join("sub").join("file.txt"), b"payload").unwrap();
    let destination = root.path().join(OsStr::from_bytes(b".wt-remove-\xfe"));

    rename_no_replace(&source, &destination).unwrap();

    assert_source_intact(&destination);
    assert!(fs::symlink_metadata(&source).is_err());
}

/// An open file anywhere in the tree blocks a Windows directory rename at the
/// rename step, after the `DELETE` open succeeded, whether or not the holder
/// shares delete access.
#[cfg(windows)]
#[test]
fn an_open_file_in_the_tree_is_a_sharing_failure() {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_WRITE: u32 = 2;

    let root = tempfile::tempdir().unwrap();
    let source = source_tree(root.path());
    let destination = root.path().join("moved");
    let file = source.join("sub").join("file.txt");

    let holders = [
        ("std default sharing", fs::File::open(&file).unwrap()),
        (
            "no delete sharing",
            fs::OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                .open(&file)
                .unwrap(),
        ),
    ];
    for (holder, handle) in holders {
        let error = rename_no_replace(&source, &destination).unwrap_err();

        assert_eq!(error.kind, RenameFailure::Sharing, "{holder}: {error}");
        assert_source_intact(&source);
        assert!(fs::symlink_metadata(&destination).is_err(), "{holder}");
        drop(handle);
    }

    rename_no_replace(&source, &destination).unwrap();
    assert_source_intact(&destination);
}

#[test]
fn an_error_without_an_os_code_is_other() {
    let error = io::Error::other("not from the OS");
    assert_eq!(classify(Stage::Rename, &error), RenameFailure::Other);
    assert_eq!(classify(Stage::Open, &error), RenameFailure::Other);
}

/// Codes a test host cannot produce on demand (WSL drvfs's `EINVAL`, another
/// volume, a denied `DELETE` open) are classified here, by stage.
#[cfg(unix)]
#[test]
fn unix_error_codes_classify() {
    let cases = [
        (libc::EEXIST, RenameFailure::Collision),
        (libc::ENOTEMPTY, RenameFailure::Collision),
        (libc::EINVAL, RenameFailure::Unsupported),
        (libc::ENOSYS, RenameFailure::Unsupported),
        (libc::ENOTSUP, RenameFailure::Unsupported),
        (libc::EXDEV, RenameFailure::CrossDevice),
        (libc::EACCES, RenameFailure::Permission),
        (libc::EPERM, RenameFailure::Permission),
        (libc::ENOENT, RenameFailure::Other),
        (libc::EROFS, RenameFailure::Other),
        (libc::EBUSY, RenameFailure::Other),
    ];
    for (code, expected) in cases {
        let error = io::Error::from_raw_os_error(code);
        assert_eq!(classify(Stage::Rename, &error), expected, "{error}");
    }
}

#[cfg(windows)]
#[test]
fn windows_error_codes_classify_by_stage() {
    const ACCESS_DENIED: i32 = 5;
    const SHARING_VIOLATION: i32 = 32;
    let cases = [
        (Stage::Open, ACCESS_DENIED, RenameFailure::Permission),
        (Stage::Open, SHARING_VIOLATION, RenameFailure::Permission),
        (Stage::Rename, ACCESS_DENIED, RenameFailure::Sharing),
        (Stage::Rename, SHARING_VIOLATION, RenameFailure::Sharing),
        (Stage::Rename, 183, RenameFailure::Collision),
        (Stage::Rename, 80, RenameFailure::Collision),
        (Stage::Rename, 17, RenameFailure::CrossDevice),
        (Stage::Rename, 50, RenameFailure::Unsupported),
        (Stage::Rename, 1, RenameFailure::Unsupported),
        (Stage::Open, 2, RenameFailure::Other),
        (Stage::Open, 3, RenameFailure::Other),
        (Stage::Rename, 87, RenameFailure::Other),
    ];
    for (stage, code, expected) in cases {
        let error = io::Error::from_raw_os_error(code);
        assert_eq!(classify(stage, &error), expected, "{stage:?} {error}");
    }
}
