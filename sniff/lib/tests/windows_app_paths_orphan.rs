//! Integration test: an App Paths registry entry that points at a
//! nonexistent target must not appear in the built index.
//!
//! Writes to `HKCU` only (no admin required). Cleans up after itself even
//! on assertion failure.

#![cfg(target_os = "windows")]

use sniff::programs::ExecutableIndex;
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

const TEST_KEY: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\sniff_integration_orphan.exe";

struct OrphanGuard;

impl Drop for OrphanGuard {
    fn drop(&mut self) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _ = hkcu.delete_subkey_all(TEST_KEY);
    }
}

#[test]
fn orphaned_hkcu_entry_is_filtered() {
    let _guard = OrphanGuard;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(TEST_KEY).unwrap();
    key.set_value("", &r"C:\__sniff_nowhere__\never_existed.exe".to_string())
        .unwrap();

    let index = ExecutableIndex::build();
    assert!(
        index.find_with_source("sniff_integration_orphan").is_none(),
        "orphan should not resolve"
    );
    assert!(
        index
            .find_with_source("sniff_integration_orphan.exe")
            .is_none(),
        "orphan should not resolve under .exe form either"
    );
}
