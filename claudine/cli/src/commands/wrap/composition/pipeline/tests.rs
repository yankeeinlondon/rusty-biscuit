use super::*;

#[test]
fn phase_result_preserves_all_pipeline_outcomes() {
    let proceed: CompositionPhaseResult<u8> = CompositionPhaseResult::Proceed(1);
    assert!(matches!(proceed, CompositionPhaseResult::Proceed(1)));

    let blocked: CompositionPhaseResult<()> =
        CompositionPhaseResult::Blocked(eyre!("blocked"));
    assert!(matches!(blocked, CompositionPhaseResult::Blocked(_)));

    let failed: CompositionPhaseResult<()> =
        CompositionPhaseResult::Failed(eyre!("failed"));
    assert!(matches!(failed, CompositionPhaseResult::Failed(_)));
}
