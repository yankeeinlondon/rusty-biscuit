//! Windows Service Control Manager (SCM) service enumeration.
//!
//! Uses the native Windows API (`EnumServicesStatusExW`) to list Win32
//! services.  All unsafe code and Windows-specific bindings are isolated
//! within this module.

use crate::services::Service;
#[cfg(target_os = "windows")]
use tracing::warn;

/// List Windows SCM services, returning an empty vector on any failure.
#[cfg(target_os = "windows")]
pub(crate) fn list_windows_scm_services() -> Vec<Service> {
    match enumerate_windows_scm_services() {
        Ok(services) => services,
        Err(e) => {
            warn!(error = %e, "Windows SCM service enumeration failed");
            Vec::new()
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn list_windows_scm_services() -> Vec<Service> {
    Vec::new()
}

#[cfg(target_os = "windows")]
fn enumerate_windows_scm_services() -> windows::core::Result<Vec<Service>> {
    use windows::Win32::Foundation::{ERROR_MORE_DATA, HANDLE};
    use windows::Win32::System::Services::{
        CloseServiceHandle, ENUM_SERVICE_STATUS_PROCESSW, EnumServicesStatusExW, OpenSCManagerW,
        SC_ENUM_PROCESS_INFO, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_RUNNING, SERVICE_STATE_ALL,
        SERVICE_STATUS_PROCESS, SERVICE_WIN32,
    };
    use windows::core::PCWSTR;

    let scm: HANDLE =
        unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ENUMERATE_SERVICE)? };
    let _guard = ScopeGuard::new(|| unsafe { CloseServiceHandle(scm).ok() });

    let mut bytes_needed: u32 = 0;
    let mut services_returned: u32 = 0;
    let mut buf: Vec<u8> = Vec::new();

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
        services.push(service_from_raw_status(
            name,
            status.dwCurrentState,
            status.dwProcessId,
        ));
    }

    Ok(services)
}

/// The Windows `SERVICE_RUNNING` state code (`dwCurrentState == 4`).
#[cfg(any(target_os = "windows", test))]
const SERVICE_RUNNING_STATE: u32 = 4;

/// Convert raw SCM status fields into a [`Service`].
///
/// This is a pure function extracted so that state-code-to-`running`
/// behaviour can be unit-tested on any host, not just Windows.
#[cfg(any(target_os = "windows", test))]
fn service_from_raw_status(name: String, current_state: u32, process_id: u32) -> Service {
    Service {
        name,
        pid: if process_id > 0 {
            Some(process_id)
        } else {
            None
        },
        running: current_state == SERVICE_RUNNING_STATE,
        status: Some(current_state as i32),
    }
}

#[cfg(target_os = "windows")]
struct ScopeGuard<F: FnOnce()>(Option<F>);

#[cfg(target_os = "windows")]
impl<F: FnOnce()> ScopeGuard<F> {
    fn new(f: F) -> Self {
        Self(Some(f))
    }
}

#[cfg(target_os = "windows")]
impl<F: FnOnce()> Drop for ScopeGuard<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_from_raw_status_running() {
        let svc = service_from_raw_status("MyService".into(), 4, 1234);
        assert_eq!(svc.name, "MyService");
        assert!(svc.running);
        assert_eq!(svc.pid, Some(1234));
        assert_eq!(svc.status, Some(4));
    }

    #[test]
    fn test_service_from_raw_status_stopped() {
        let svc = service_from_raw_status("StoppedSvc".into(), 1, 0);
        assert!(!svc.running);
        assert_eq!(svc.pid, None);
        assert_eq!(svc.status, Some(1));
    }

    #[test]
    fn test_service_from_raw_status_start_pending() {
        let svc = service_from_raw_status("StartingSvc".into(), 2, 0);
        assert!(!svc.running);
        assert_eq!(svc.pid, None);
    }

    #[test]
    fn test_service_from_raw_status_running_zero_pid() {
        let svc = service_from_raw_status("SvcSharedProcess".into(), 4, 0);
        assert!(svc.running);
        assert_eq!(svc.pid, None);
    }

    #[test]
    fn test_list_windows_scm_services_returns_vec() {
        let services = list_windows_scm_services();
        assert!(services.is_empty());
    }
}
