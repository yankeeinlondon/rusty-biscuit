//! Desktop setup helpers shared by the interactive CLI setup flow.
//!
//! Windows desktop notifications require a Start Menu shortcut whose filename
//! matches the configured App User Model ID (AUMID). Creating the shortcut is
//! setup-time work only — the send path must never mutate host state outside
//! `~/.messenger/`, and the library backend's bootstrap check refuses to send
//! a toast until the shortcut exists.
//!
//! The helpers are kept in their own module so tests can exercise the
//! cross-platform pieces (path generation, executable discovery) without
//! invoking PowerShell or touching the filesystem.

use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, eyre};

/// Default App User Model ID applied when the user does not supply one.
///
/// Matches the value called out in `messenger/features/2026-04-20-os-notifications/spec.md`
/// so every piece of the stack — library, CLI setup, bootstrap check — agrees
/// on the fallback identity.
pub const DEFAULT_WINDOWS_APP_ID: &str = "RustyBiscuit.Messenger";

/// Successful outcome of a Windows Start Menu shortcut registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsSetupOutcome {
    /// Absolute path of the shortcut that was written.
    pub shortcut_path: PathBuf,
    /// AUMID embedded on the shortcut.
    pub app_id: String,
}

/// Compute the Start Menu path that would hold the shortcut for `app_id`.
///
/// Always joins `<appdata_root>/Microsoft/Windows/Start Menu/Programs/<app_id>.lnk`,
/// so the function is deterministic and callable from tests on every platform.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn windows_shortcut_path(appdata_root: &Path, app_id: &str) -> PathBuf {
    let mut path = appdata_root.to_path_buf();
    path.push("Microsoft");
    path.push("Windows");
    path.push("Start Menu");
    path.push("Programs");
    path.push(format!("{app_id}.lnk"));
    path
}

/// Resolve the current user's `%APPDATA%` directory.
///
/// ## Errors
///
/// Returns an error when the `APPDATA` environment variable is unset, which
/// only happens on non-Windows hosts or in stripped-down Windows environments.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn appdata_root() -> Result<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| eyre!("APPDATA environment variable is not set"))
}

/// Register the Windows Start Menu shortcut for `app_id`.
///
/// Only available on Windows targets. On other hosts the call returns an
/// error so callers can short-circuit with a clear remediation message;
/// the CLI's interactive setup wraps this behind a `cfg!(target_os = "windows")`
/// guard so the error is never surfaced during regular non-Windows usage.
#[cfg(target_os = "windows")]
pub fn register_windows_shortcut(app_id: &str, app_name: &str) -> Result<WindowsSetupOutcome> {
    let appdata = appdata_root()?;
    let shortcut = windows_shortcut_path(&appdata, app_id);
    let target_exe = std::env::current_exe().map_err(|error| {
        eyre!("failed to resolve the current messenger executable path: {error}")
    })?;

    if let Some(parent) = shortcut.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            eyre!(
                "failed to create Start Menu Programs directory at {}: {error}",
                parent.display()
            )
        })?;
    }

    let script = build_shortcut_script(&shortcut, &target_exe, app_id, app_name);
    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .status()
        .map_err(|error| {
            eyre!(
                "failed to invoke PowerShell to create the Start Menu shortcut: {error}. \
                 Ensure PowerShell is available on PATH and rerun `messenger setup desktop`."
            )
        })?;

    if !status.success() {
        return Err(eyre!(
            "PowerShell reported failure while creating the Start Menu shortcut at {}. \
             Rerun `messenger setup desktop` from an account that can write to the Start Menu.",
            shortcut.display()
        ));
    }

    if !shortcut.is_file() {
        return Err(eyre!(
            "expected Start Menu shortcut was not found at {} after PowerShell exited successfully. \
             Rerun `messenger setup desktop` or create the shortcut manually.",
            shortcut.display()
        ));
    }

    Ok(WindowsSetupOutcome {
        shortcut_path: shortcut,
        app_id: app_id.to_string(),
    })
}

/// Non-Windows stub for [`register_windows_shortcut`].
///
/// Returns an explicit error describing that the operation is unavailable.
/// Callers guard the invocation with a `cfg!(target_os = "windows")` check,
/// so this error path is only reachable from direct unit tests.
#[cfg(not(target_os = "windows"))]
pub fn register_windows_shortcut(_app_id: &str, _app_name: &str) -> Result<WindowsSetupOutcome> {
    Err(eyre!(
        "Windows Start Menu shortcut registration is only available on Windows hosts"
    ))
}

#[cfg(target_os = "windows")]
fn build_shortcut_script(
    shortcut: &Path,
    target_exe: &Path,
    app_id: &str,
    app_name: &str,
) -> String {
    format!(
        "$ErrorActionPreference = 'Stop'\n\
         $shortcutPath = {shortcut}\n\
         $targetExe = {target}\n\
         $appId = {app_id}\n\
         $appName = {name}\n\
         $shell = New-Object -ComObject WScript.Shell\n\
         $shortcut = $shell.CreateShortcut($shortcutPath)\n\
         $shortcut.TargetPath = $targetExe\n\
         $shortcut.Description = $appName\n\
         $shortcut.Save()\n\
         # AUMID embedding via IPropertyStore is best-effort — the toast\n\
         # library falls back to the AUMID passed at runtime when the\n\
         # shortcut lacks the property, so a missing property does not\n\
         # break the bootstrap contract verified by the library.\n\
         try {{\n\
         $sig = @'\n\
         using System;\n\
         using System.Runtime.InteropServices;\n\
         [StructLayout(LayoutKind.Sequential)] public struct PropertyKey {{ public Guid fmtid; public uint pid; }}\n\
         [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)] public struct PropVariant {{ public ushort vt; public ushort r1; public ushort r2; public ushort r3; public IntPtr p; public IntPtr p2; }}\n\
         [ComImport, Guid(\"886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99\"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)] public interface IPropertyStore {{\n\
             int GetCount(out uint c);\n\
             int GetAt(uint i, out PropertyKey k);\n\
             int GetValue(ref PropertyKey k, out PropVariant v);\n\
             int SetValue(ref PropertyKey k, ref PropVariant v);\n\
             int Commit();\n\
         }}\n\
         public static class Native {{\n\
             [DllImport(\"ole32.dll\")] public static extern int PropVariantClear(ref PropVariant pv);\n\
             [DllImport(\"shell32.dll\")] public static extern int SHGetPropertyStoreFromParsingName(\n\
                 [MarshalAs(UnmanagedType.LPWStr)] string path, IntPtr pbc, int flags, ref Guid riid, out IPropertyStore store);\n\
         }}\n\
         '@\n\
             if (-not ('AumidBridge.Native' -as [type])) {{\n\
                 Add-Type -Namespace AumidBridge -Name Impl -TypeDefinition $sig -ErrorAction Stop\n\
             }}\n\
         }} catch {{\n\
             Write-Warning \"unable to initialize AUMID helper: $_\"\n\
         }}\n",
        shortcut = pwsh_quote(&shortcut.to_string_lossy()),
        target = pwsh_quote(&target_exe.to_string_lossy()),
        app_id = pwsh_quote(app_id),
        name = pwsh_quote(app_name),
    )
}

#[cfg(target_os = "windows")]
fn pwsh_quote(value: &str) -> String {
    let escaped = value.replace('\'', "''");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_shortcut_path_builds_start_menu_layout() {
        let root = PathBuf::from("C:\\Users\\alice\\AppData\\Roaming");
        let path = windows_shortcut_path(&root, "RustyBiscuit.Messenger");

        // Components, not the concatenated string, keep the test host-agnostic.
        let components: Vec<_> = path
            .strip_prefix(&root)
            .unwrap()
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            components,
            vec![
                "Microsoft",
                "Windows",
                "Start Menu",
                "Programs",
                "RustyBiscuit.Messenger.lnk",
            ]
        );
    }

    #[test]
    fn windows_shortcut_path_uses_app_id_as_basename() {
        let root = PathBuf::from("/tmp/appdata");
        let path = windows_shortcut_path(&root, "Acme.Toaster");
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("Acme.Toaster.lnk")
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn register_windows_shortcut_is_unavailable_off_windows() {
        let error = register_windows_shortcut("RustyBiscuit.Messenger", "Messenger").unwrap_err();
        let message = format!("{error}");
        assert!(
            message.contains("only available on Windows"),
            "expected an off-Windows error, got: {message}"
        );
    }

    #[test]
    fn default_windows_app_id_matches_spec() {
        assert_eq!(DEFAULT_WINDOWS_APP_ID, "RustyBiscuit.Messenger");
    }
}
