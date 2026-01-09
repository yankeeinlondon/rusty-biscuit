use model_id::ModelId;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::AsRefStr, ModelId)]
pub enum ProviderModelZai {
    glm_4_6v__Flash__Free,
    glm_4_5__Air,
    glm_4_5,
    glm_4_6,
    glm_4_6v__Flash,
    glm_4_6v,
    glm_4_7,

    Bespoke(String)
}
