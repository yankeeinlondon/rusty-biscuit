use super::{AgenticEvent, EventSupportLevel, Provider};

pub use super::provider::PROVIDERS_DISPLAY_ORDER;

/// A single support cell in the provider/event support matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventSupportCell {
    pub provider: Provider,
    pub level: EventSupportLevel,
}

/// A single row in the provider/event support matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSupportRow {
    pub event: AgenticEvent,
    pub cells: Vec<EventSupportCell>,
}

/// Native mapping shape for an event/provider pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeEventName {
    /// Provider does not support the event.
    Unsupported,
    /// Provider supports the event but has no specific native name.
    NoSpecificName,
    /// Provider supports the event with a concrete native event name.
    Named(&'static str),
}

/// A single mapping cell in the provider/event native-name matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventNativeMappingCell {
    pub provider: Provider,
    pub native: NativeEventName,
}

/// A single row in the provider/event native-name matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventNativeMappingRow {
    pub event: AgenticEvent,
    pub cells: Vec<EventNativeMappingCell>,
}

/// Build a structured support matrix for the requested providers.
pub fn event_support_matrix(providers: &[Provider]) -> Vec<EventSupportRow> {
    AgenticEvent::ALL
        .into_iter()
        .map(|event| EventSupportRow {
            event,
            cells: providers
                .iter()
                .map(|provider| EventSupportCell {
                    provider: *provider,
                    level: provider.event_support_level(&event),
                })
                .collect(),
        })
        .collect()
}

/// Build a structured native-name mapping matrix for the requested providers.
pub fn event_native_mapping_matrix(providers: &[Provider]) -> Vec<EventNativeMappingRow> {
    AgenticEvent::ALL
        .into_iter()
        .map(|event| EventNativeMappingRow {
            event,
            cells: providers
                .iter()
                .map(|provider| {
                    let native = match provider.native_event_name(&event) {
                        None => NativeEventName::Unsupported,
                        Some("") => NativeEventName::NoSpecificName,
                        Some(name) => NativeEventName::Named(name),
                    };
                    EventNativeMappingCell {
                        provider: *provider,
                        native,
                    }
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_matrix_matches_provider_api() {
        let providers = [Provider::Claude, Provider::OpenCode, Provider::QwenCode];
        let matrix = event_support_matrix(&providers);

        assert_eq!(matrix.len(), AgenticEvent::ALL.len());
        for row in matrix {
            assert_eq!(row.cells.len(), providers.len());
            for cell in row.cells {
                assert_eq!(cell.level, cell.provider.event_support_level(&row.event));
            }
        }
    }

    #[test]
    fn native_mapping_matrix_matches_provider_api() {
        let providers = [Provider::Claude, Provider::OpenCode, Provider::RooCode];
        let matrix = event_native_mapping_matrix(&providers);

        assert_eq!(matrix.len(), AgenticEvent::ALL.len());
        for row in matrix {
            assert_eq!(row.cells.len(), providers.len());
            for cell in row.cells {
                let expected = match cell.provider.native_event_name(&row.event) {
                    None => NativeEventName::Unsupported,
                    Some("") => NativeEventName::NoSpecificName,
                    Some(name) => NativeEventName::Named(name),
                };
                assert_eq!(cell.native, expected);
            }
        }
    }
}
