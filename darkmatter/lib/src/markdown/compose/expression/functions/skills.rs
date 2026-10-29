use super::FunctionBinding;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::context("has_skill", &["hasskill"], super::has_skill_fn),
    FunctionBinding::context("has_local_skill", &["haslocalskill"], super::has_local_skill_fn),
];
