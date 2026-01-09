use model_id::ModelId;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum ProviderModelGemini {
    Gemini__3__Pro__Image__Preview,
    Gemini__3__Pro__Preview,
    Gemini__2_5__Flash__Image,
    Gemini__2_5__Flash__Preview__09__2025,
    Gemini__2_5__Flash__Lite__Preview__09__2025,
    Gemini__2_5__Flash__Image__Preview,
    Gemini__2_5__Flash__Lite,
    Gemma__3n__E2b__Itfree,
    Gemini__2_5__Flash,
    Gemini__2_5__Pro,
    Gemini__2_5__Pro__Preview,
    Gemma__3n__E4b__Itfree,
    Gemma__3n__E4b__It,
    Gemini__2__5__Pro__Preview__05__06,
    Gemma__3__4b__Itfree,
    Gemma__3__4b__It,
    Gemma__3__12b__Itfree,
    Gemma__3__12b__It,
    Gemma__3__27b__Itfree,
    Gemma__3__27b__It,
    Gemini__2__0__Flash__Lite__001,
    Gemini__2__0__Flash__001,
    Gemini__2__0__Flash__Expfree,
    Gemma__2__27b__It,
    Gemma__2__9b__It,
    Gemini__2_0__Flash,
    Gemini__3__Flash__Preview__Free,

    Bespoke(String)
}
