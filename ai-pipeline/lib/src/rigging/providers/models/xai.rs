use model_id::ModelId;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum ProviderModelXai {
    grok__3,
    grok__3__Mini,
    grok__Code__Fast__1,
    grok__3__Beta,
    grok__3__Mini__Beta,
    grok__4,
    grok__4_1__Fast,
    grok__4__Fast,

    Bespoke(String)
}
