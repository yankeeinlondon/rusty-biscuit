use model_id::ModelId;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum ProviderModelZai {
    glm__4_6v__Flash__Free,
    glm__4_5__Air,
    glm__4_5,
    glm__4_6,
    glm__4_6v__Flash,
    glm__4_6v,
    glm__4_7,

    Bespoke(String)
}
