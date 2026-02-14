use model_id::ModelId;

/// Test enum without Bespoke variant - FromStr should return error for unknown IDs
#[derive(ModelId, Debug, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum StrictProvider {
    ModelA,
    ModelB,
}

fn main() {
    // Test model_id()
    assert_eq!(StrictProvider::ModelA.model_id(), "modela");
    assert_eq!(StrictProvider::ModelB.model_id(), "modelb");

    // Test FromStr with known model
    let parsed: StrictProvider = "modela".parse().unwrap();
    assert_eq!(parsed, StrictProvider::ModelA);

    // Test FromStr with unknown model returns error
    let result: Result<StrictProvider, _> = "unknown".parse();
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.model_id, "unknown");
    assert_eq!(err.enum_name, "StrictProvider");

    // Test ALL
    assert_eq!(StrictProvider::ALL.len(), 2);
}
