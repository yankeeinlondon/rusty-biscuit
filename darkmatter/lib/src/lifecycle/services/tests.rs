//! A missing service is a typed error naming the service.

use super::*;
use crate::lifecycle::envelope::DiagnosticSnapshot;

struct NoCodes;

impl DiagnosticCatalog for NoCodes {
    fn select(&self, _: &(dyn std::error::Error + 'static)) -> Option<DiagnosticSnapshot> {
        None
    }
    fn snapshot_for_code(&self, _: &str, _: String) -> Option<DiagnosticSnapshot> {
        None
    }
    fn snapshot_for_error_kind(&self, _: &str, _: String) -> Option<DiagnosticSnapshot> {
        None
    }
}

#[test]
fn an_absent_service_is_a_missing_service_error() {
    let services = LifecycleServices::default();

    assert_eq!(
        services.effects().err(),
        Some(MissingServiceError { service: LifecycleService::Effects })
    );
    assert_eq!(
        services.diagnostics().err(),
        Some(MissingServiceError { service: LifecycleService::Diagnostics })
    );
}

#[test]
fn the_missing_service_error_names_the_service() {
    let error = MissingServiceError { service: LifecycleService::Effects };

    assert_eq!(error.to_string(), "the lifecycle `effects` service is not configured");
}

#[test]
fn a_supplied_service_is_returned() {
    let services = LifecycleServices {
        diagnostics: Some(&NoCodes),
        ..LifecycleServices::default()
    };

    assert!(services.diagnostics().is_ok());
    assert!(services.effects().is_err());
}
