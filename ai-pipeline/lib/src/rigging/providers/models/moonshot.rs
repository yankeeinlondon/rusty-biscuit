use model_id::ModelId;


#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum ProviderModelZai {
    Kimi__K2__Thinking,
    Kimi__K2__0905,
    Kimi__K2__0905exacto,
    Kimi__K2free,
    Kimi__K2,
    Kimi__Dev__72b,
    Kimi__K2__0711,
    Kimi__K2__Thinking__Turbo,

    Bespoke(String)
}
