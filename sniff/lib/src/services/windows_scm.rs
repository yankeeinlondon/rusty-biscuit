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
    use windows::Win32::Foundation::ERROR_MORE_DATA;
    use windows::Win32::System::Services::{
        CloseServiceHandle, ENUM_SERVICE_STATUS_PROCESSW, EnumServicesStatusExW, OpenSCManagerW,
        SC_ENUM_PROCESS_INFO, SC_HANDLE, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_RUNNING,
        SERVICE_STATE_ALL, SERVICE_STATUS_PROCESS, SERVICE_WIN32,
    };
    use windows::core::PCWSTR;

    let scm: SC_HANDLE =
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

/// The Windows `SERVICE_STOPPED` state code (`dwCurrentState == 1`).
#[cfg(any(target_os = "windows", test))]
const SERVICE_STOPPED_STATE: u32 = 1;

/// The Windows `SERVICE_START_PENDING` state code (`dwCurrentState == 2`).
#[cfg(any(target_os = "windows", test))]
const SERVICE_START_PENDING_STATE: u32 = 2;

/// The Windows `SERVICE_STOP_PENDING` state code (`dwCurrentState == 3`).
#[cfg(any(target_os = "windows", test))]
const SERVICE_STOP_PENDING_STATE: u32 = 3;

/// The Windows `SERVICE_CONTINUE_PENDING` state code (`dwCurrentState == 5`).
#[cfg(any(target_os = "windows", test))]
const SERVICE_CONTINUE_PENDING_STATE: u32 = 5;

/// The Windows `SERVICE_PAUSE_PENDING` state code (`dwCurrentState == 6`).
#[cfg(any(target_os = "windows", test))]
const SERVICE_PAUSE_PENDING_STATE: u32 = 6;

/// The Windows `SERVICE_PAUSED` state code (`dwCurrentState == 7`).
#[cfg(any(target_os = "windows", test))]
const SERVICE_PAUSED_STATE: u32 = 7;

/// Classifies a raw Windows SCM `dwCurrentState` code into a filterable
/// service-state category.
///
/// This is a pure function so it can be unit-tested on any platform.
///
/// ## Returns
///
/// - `Some(true)` for running states (`SERVICE_RUNNING`).
/// - `Some(false)` for stopped states (`SERVICE_STOPPED`).
/// - `None` for pending / transitional states (start-pending, stop-pending,
///   continue-pending, pause-pending, paused).
#[cfg(any(target_os = "windows", test))]
fn classify_scm_state(current_state: u32) -> Option<bool> {
    match current_state {
        SERVICE_RUNNING_STATE => Some(true),
        SERVICE_STOPPED_STATE => Some(false),
        _ => None,
    }
}

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
        running: classify_scm_state(current_state).unwrap_or(false),
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

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_list_windows_scm_services_stub_returns_empty() {
        let services = list_windows_scm_services();
        assert!(services.is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_list_windows_scm_services_returns_real_services() {
        let services = list_windows_scm_services();
        assert!(
            !services.is_empty(),
            "SCM enumeration should return at least one service on Windows"
        );
        for svc in &services {
            assert!(!svc.name.is_empty(), "service name should not be empty");
        }
    }

    #[test]
    fn test_classify_scm_state_running() {
        assert_eq!(classify_scm_state(SERVICE_RUNNING_STATE), Some(true));
    }

    #[test]
    fn test_classify_scm_state_stopped() {
        assert_eq!(classify_scm_state(SERVICE_STOPPED_STATE), Some(false));
    }

    #[test]
    fn test_classify_scm_state_start_pending() {
        assert_eq!(classify_scm_state(SERVICE_START_PENDING_STATE), None);
    }

    #[test]
    fn test_classify_scm_state_stop_pending() {
        assert_eq!(classify_scm_state(SERVICE_STOP_PENDING_STATE), None);
    }

    #[test]
    fn test_classify_scm_state_continue_pending() {
        assert_eq!(classify_scm_state(SERVICE_CONTINUE_PENDING_STATE), None);
    }

    #[test]
    fn test_classify_scm_state_pause_pending() {
        assert_eq!(classify_scm_state(SERVICE_PAUSE_PENDING_STATE), None);
    }

    #[test]
    fn test_classify_scm_state_paused() {
        assert_eq!(classify_scm_state(SERVICE_PAUSED_STATE), None);
    }

    #[test]
    fn test_service_from_raw_status_pending_states_not_running() {
        let pending_states = [
            SERVICE_START_PENDING_STATE,
            SERVICE_STOP_PENDING_STATE,
            SERVICE_CONTINUE_PENDING_STATE,
            SERVICE_PAUSE_PENDING_STATE,
            SERVICE_PAUSED_STATE,
        ];
        for state in pending_states {
            let svc = service_from_raw_status(format!("Svc-{state}").into(), state, 0);
            assert!(
                !svc.running,
                "service with SCM state {state} should not be marked running"
            );
        }
    }

    #[test]
    fn test_service_state_filter_running() {
        use crate::services::ServiceState;

        let services = vec![
            service_from_raw_status("RunningSvc".into(), SERVICE_RUNNING_STATE, 100),
            service_from_raw_status("StoppedSvc".into(), SERVICE_STOPPED_STATE, 0),
            service_from_raw_status("PendingSvc".into(), SERVICE_START_PENDING_STATE, 0),
        ];

        let running: Vec<_> = services
            .iter()
            .filter(|s| ServiceState::Running.matches(Some(s.running)))
            .collect();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].name, "RunningSvc");
    }

    #[test]
    fn test_service_state_filter_stopped() {
        use crate::services::ServiceState;

        let services = vec![
            service_from_raw_status("RunningSvc".into(), SERVICE_RUNNING_STATE, 100),
            service_from_raw_status("StoppedSvc".into(), SERVICE_STOPPED_STATE, 0),
            service_from_raw_status("AnotherStopped".into(), SERVICE_STOPPED_STATE, 0),
        ];

        let stopped: Vec<_> = services
            .iter()
            .filter(|s| ServiceState::Stopped.matches(Some(s.running)))
            .collect();
        assert_eq!(stopped.len(), 2);
    }

    #[test]
    fn test_service_state_filter_all() {
        use crate::services::ServiceState;

        let services = vec![
            service_from_raw_status("RunningSvc".into(), SERVICE_RUNNING_STATE, 100),
            service_from_raw_status("StoppedSvc".into(), SERVICE_STOPPED_STATE, 0),
            service_from_raw_status("PendingSvc".into(), SERVICE_START_PENDING_STATE, 0),
        ];

        let all: Vec<_> = services
            .iter()
            .filter(|s| ServiceState::All.matches(Some(s.running)))
            .collect();
        assert_eq!(all.len(), 3);
    }
}
