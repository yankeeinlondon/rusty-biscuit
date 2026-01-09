use model_id::ModelId;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum ProviderModelAnthropic {
    Claude__Opus__4__5__20251101,
    Claude__Haiku__4__5__20251001,
    Claude__Sonnet__4__5__20250929,
    Claude__Opus__4__1__20250805,
    Claude__Opus__4__20250514,
    Claude__Sonnet__4__20250514,
    Claude__3__7_Sonnet__20250219,
    Claude__3__5_Haiku__20241022,
    Claude__3__Haiku__20240307,
    Claude__3__Opus__20240229,

    Bespoke(String)
}

