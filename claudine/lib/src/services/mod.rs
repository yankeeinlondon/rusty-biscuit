#[allow(deprecated)]
pub mod protect;

#[allow(deprecated)]
pub use protect::{
    CompletionPolicy, CompletionPolicyOverride, GateCapability, McpJsonRedaction, McpPolicy,
    McpPolicyOverride, McpTextRedaction, PrivilegePolicy, PrivilegePolicyOverride,
    ProtectCliContext, ProtectConfig, ProtectDecision, ProtectDecisionRecord, ProtectEvaluation,
    ProtectInput, ProtectObservation, ProtectOutcome, ProtectPayload, ProtectPhase,
    ProtectPolicyMode, ProtectPosture, ProtectRequest, ProtectRules, ProtectRulesOverride,
    ProtectRuntimeMode, ProtectService, ProtectSessionContext, ProtectState, ProtectStateExport,
    ProviderProtectCapabilities, ProviderProtectOverride, ProviderProtectProfiles, RiskLevel,
    RuntimeFacts, SubagentPolicy, SubagentPolicyOverride, VisibilityLevel, YoloPolicy,
    YoloPolicyOverride,
};
