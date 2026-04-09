//! Windows Service Control Manager (SCM) service enumeration.
//!
//! Uses the native Windows API (`EnumServicesStatusExW`) to list Win32
//! services.  All unsafe code and Windows-specific bindings are isolated
//! within this module.

use crate::services::Service;
use tracing::warn;

/// List Windows SCM services, returning an empty vector on any failure.
pub(crate) fn list_windows_scm_services() -> Vec<Service> {
    match enumerate_windows_scm_services() {
        Ok(services) => services,
        Err(e) => {
            warn!(error = %e, "Windows SCM service enumeration failed");
            Vec::new()
        }
    }
}

#[cfg(target_os = "windows")]
fn enumerate_windows_scm_services() -> windows::core::Result<Vec<Service>> {
    use windows::Win32::Foundation::{ERROR_MORE_DATA, HANDLE};
    use windows::Win32::System::Services::{
        CloseServiceHandle, ENUM_SERVICE_STATUS_PROCESSW, EnumServicesStatusExW, OpenSCManagerW,
        SC_ENUM_PROCESS_INFO, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_STATE_ALL,
        SERVICE_STATUS_PROCESS, SERVICE_WIN32,
    };
    use windows::core::PCWSTR;

    let scm: HANDLE =
        unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ENUMERATE_SERVICE)? };
    let _guard = ScopeGuard(|| unsafe { CloseServiceHandle(scm).ok() });

    let mut bytes_needed: u32 = 0;
    let mut services_returned: u32 = 0;
    let mut buf: Vec<u8> = Vec::new();

    // First call to determine required buffer size.
    let _ = unsafe {
        EnumServicesStatusExW(
            scm,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            Some(buf.as_mut_ptr()),
            0,
            &mut bytes_needed,
            &mut services_returned,
            None,
            PCWSTR::null(),
        )
    };

    loop {
        buf.resize(bytes_needed as usize, 0u8);

        let result = unsafe {
            EnumServicesStatusExW(
                scm,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
                SERVICE_STATE_ALL,
                Some(buf.as_mut_ptr()),
                bytes_needed,
                &mut bytes_needed,
                &mut services_returned,
                None,
                PCWSTR::null(),
            )
        };

        match result {
            Ok(()) => break,
            Err(e) if e.code() == ERROR_MORE_DATA.to_hresult() => continue,
            Err(e) => return Err(e),
        }
    }

    let entries = unsafe {
        std::slice::from_raw_parts(
            buf.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW,
            services_returned as usize,
        )
    };

    let mut services = Vec::with_capacity(services_returned as usize);

    for entry in entries {
        let name = unsafe { entry.lpServiceName.to_string() }.unwrap_or_default();
        if name.is_empty() {
            continue;
        }

        let status: SERVICE_STATUS_PROCESS = unsafe { entry.ServiceStatusProcess };

        let running = status.dwCurrentState == 4; // SERVICE_RUNNING
        let pid = if status.dwProcessId > 0 {
            Some(status.dwProcessId)
        } else {
            None
        };

        services.push(Service {
            name,
            pid,
            running,
            status: Some(status.dwCurrentState as i32),
        });
    }

    Ok(services)
}

#[cfg(not(target_os = "windows"))]
fn enumerate_windows_scm_services() -> windows::core::Result<Vec<Service>> {
    Ok(Vec::new())
}

struct ScopeGuard<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> ScopeGuard<F> {
    fn new(f: F) -> Self {
        Self(Some(f))
    }
}

impl<F: FnOnce()> Drop for ScopeGuard<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn test_list_windows_scm_services_does_not_panic() {
        let services = list_windows_scm_services();
        for svc in &services {
            assert!(!svc.name.is_empty());
        }
    }
}
