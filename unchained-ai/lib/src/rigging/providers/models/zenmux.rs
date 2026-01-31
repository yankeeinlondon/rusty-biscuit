//! Auto-generated provider model enum
//!
//! Generated: 2026-01-11T20:35:21.745420+00:00
//! Generator: gen-models v0.1.0
//! Provider: ZenMux
//!
//! Do not edit manually.

use model_id::ModelId;

/// Models provided by [ZenMux](<https://zenmux.ai>).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
#[model_id_metadata(
    lookup = "super::metadata_generated::MODEL_METADATA",
    returns = "crate::models::model_metadata::ModelMetadata"
)]
pub enum ProviderModelZenMux {
    /// Model: `anthropic/claude-3.5-haiku`
    Anthropic___Claude__3_5__Haiku,
    /// Model: `anthropic/claude-3.5-sonnet`
    Anthropic___Claude__3_5__Sonnet,
    /// Model: `anthropic/claude-3.7-sonnet`
    Anthropic___Claude__3_7__Sonnet,
    /// Model: `anthropic/claude-haiku-4.5`
    Anthropic___Claude__Haiku__4_5,
    /// Model: `anthropic/claude-opus-4`
    Anthropic___Claude__Opus__4,
    /// Model: `anthropic/claude-opus-4.1`
    Anthropic___Claude__Opus__4_1,
    /// Model: `anthropic/claude-opus-4.5`
    Anthropic___Claude__Opus__4_5,
    /// Model: `anthropic/claude-sonnet-4`
    Anthropic___Claude__Sonnet__4,
    /// Model: `anthropic/claude-sonnet-4.5`
    Anthropic___Claude__Sonnet__4_5,
    /// Model: `baidu/ernie-5.0-thinking-preview`
    Baidu___Ernie__5_0__Thinking__Preview,
    /// Model: `baidu/ernie-x1.1-preview`
    Baidu___Ernie__X1_1__Preview,
    /// Model: `deepseek/deepseek-chat`
    Deepseek___Deepseek__Chat,
    /// Model: `deepseek/deepseek-chat-v3.1`
    Deepseek___Deepseek__Chat__V3_1,
    /// Model: `deepseek/deepseek-r1-0528`
    Deepseek___Deepseek__R1__0528,
    /// Model: `deepseek/deepseek-reasoner`
    Deepseek___Deepseek__Reasoner,
    /// Model: `deepseek/deepseek-v3.2`
    Deepseek___Deepseek__V3_2,
    /// Model: `deepseek/deepseek-v3.2-exp`
    Deepseek___Deepseek__V3_2__Exp,
    /// Model: `google/gemini-2.0-flash`
    Google___Gemini__2_0__Flash,
    /// Model: `google/gemini-2.0-flash-lite-001`
    Google___Gemini__2_0__Flash__Lite__001,
    /// Model: `google/gemini-2.5-flash`
    Google___Gemini__2_5__Flash,
    /// Model: `google/gemini-2.5-flash-lite`
    Google___Gemini__2_5__Flash__Lite,
    /// Model: `google/gemini-2.5-pro`
    Google___Gemini__2_5__Pro,
    /// Model: `google/gemini-3-flash-preview`
    Google___Gemini__3__Flash__Preview,
    /// Model: `google/gemini-3-pro-preview`
    Google___Gemini__3__Pro__Preview,
    /// Model: `google/gemma-3-12b-it`
    Google___Gemma__3__12b__It,
    /// Model: `inclusionai/ling-1t`
    Inclusionai___Ling__1t,
    /// Model: `inclusionai/ling-flash-2.0`
    Inclusionai___Ling__Flash__2_0,
    /// Model: `inclusionai/ling-mini-2.0`
    Inclusionai___Ling__Mini__2_0,
    /// Model: `inclusionai/llada2.0-flash-cap`
    Inclusionai___Llada2_0__Flash__Cap,
    /// Model: `inclusionai/ming-flash-omni-preview`
    Inclusionai___Ming__Flash__Omni__Preview,
    /// Model: `inclusionai/ring-1t`
    Inclusionai___Ring__1t,
    /// Model: `inclusionai/ring-flash-2.0`
    Inclusionai___Ring__Flash__2_0,
    /// Model: `inclusionai/ring-mini-2.0`
    Inclusionai___Ring__Mini__2_0,
    /// Model: `kuaishou/kat-coder-pro-v1`
    Kuaishou___Kat__Coder__Pro__V1,
    /// Model: `kuaishou/kat-coder-pro-v1-free`
    Kuaishou___Kat__Coder__Pro__V1__Free,
    /// Model: `meta/llama-3.3-70b-instruct`
    Meta___Llama__3_3__70b__Instruct,
    /// Model: `meta/llama-4-scout-17b-16e-instruct`
    Meta___Llama__4__Scout__17b__16e__Instruct,
    /// Model: `minimax/minimax-m2`
    Minimax___Minimax__M2,
    /// Model: `minimax/minimax-m2.1`
    Minimax___Minimax__M2_1,
    /// Model: `mistralai/mistral-large-2512`
    Mistralai___Mistral__Large__2512,
    /// Model: `moonshotai/kimi-k2-0711`
    Moonshotai___Kimi__K2__0711,
    /// Model: `moonshotai/kimi-k2-0905`
    Moonshotai___Kimi__K2__0905,
    /// Model: `moonshotai/kimi-k2-thinking`
    Moonshotai___Kimi__K2__Thinking,
    /// Model: `moonshotai/kimi-k2-thinking-turbo`
    Moonshotai___Kimi__K2__Thinking__Turbo,
    /// Model: `openai/gpt-4.1`
    Openai___Gpt__4_1,
    /// Model: `openai/gpt-4.1-mini`
    Openai___Gpt__4_1__Mini,
    /// Model: `openai/gpt-4.1-nano`
    Openai___Gpt__4_1__Nano,
    /// Model: `openai/gpt-4o`
    Openai___Gpt__4o,
    /// Model: `openai/gpt-4o-mini`
    Openai___Gpt__4o__Mini,
    /// Model: `openai/gpt-5`
    Openai___Gpt__5,
    /// Model: `openai/gpt-5-chat`
    Openai___Gpt__5__Chat,
    /// Model: `openai/gpt-5-codex`
    Openai___Gpt__5__Codex,
    /// Model: `openai/gpt-5-mini`
    Openai___Gpt__5__Mini,
    /// Model: `openai/gpt-5-nano`
    Openai___Gpt__5__Nano,
    /// Model: `openai/gpt-5-pro`
    Openai___Gpt__5__Pro,
    /// Model: `openai/gpt-5.1`
    Openai___Gpt__5_1,
    /// Model: `openai/gpt-5.1-chat`
    Openai___Gpt__5_1__Chat,
    /// Model: `openai/gpt-5.1-codex`
    Openai___Gpt__5_1__Codex,
    /// Model: `openai/gpt-5.1-codex-mini`
    Openai___Gpt__5_1__Codex__Mini,
    /// Model: `openai/gpt-5.2`
    Openai___Gpt__5_2,
    /// Model: `openai/gpt-5.2-chat`
    Openai___Gpt__5_2__Chat,
    /// Model: `openai/gpt-5.2-pro`
    Openai___Gpt__5_2__Pro,
    /// Model: `openai/o4-mini`
    Openai___O4__Mini,
    /// Model: `qwen/qwen3-14b`
    Qwen___Qwen3__14b,
    /// Model: `qwen/qwen3-235b-a22b-2507`
    Qwen___Qwen3__235b__A22b__2507,
    /// Model: `qwen/qwen3-235b-a22b-thinking-2507`
    Qwen___Qwen3__235b__A22b__Thinking__2507,
    /// Model: `qwen/qwen3-coder`
    Qwen___Qwen3__Coder,
    /// Model: `qwen/qwen3-coder-plus`
    Qwen___Qwen3__Coder__Plus,
    /// Model: `qwen/qwen3-max`
    Qwen___Qwen3__Max,
    /// Model: `qwen/qwen3-max-preview`
    Qwen___Qwen3__Max__Preview,
    /// Model: `qwen/qwen3-vl-plus`
    Qwen___Qwen3__Vl__Plus,
    /// Model: `stepfun/step-3`
    Stepfun___Step__3,
    /// Model: `volcengine/doubao-seed-1-6-vision`
    Volcengine___Doubao__Seed__1__6__Vision,
    /// Model: `volcengine/doubao-seed-1.8`
    Volcengine___Doubao__Seed__1_8,
    /// Model: `volcengine/doubao-seed-code`
    Volcengine___Doubao__Seed__Code,
    /// Model: `x-ai/grok-4`
    X__Ai___Grok__4,
    /// Model: `x-ai/grok-4-fast`
    X__Ai___Grok__4__Fast,
    /// Model: `x-ai/grok-4-fast-non-reasoning`
    X__Ai___Grok__4__Fast__Non__Reasoning,
    /// Model: `x-ai/grok-4.1-fast`
    X__Ai___Grok__4_1__Fast,
    /// Model: `x-ai/grok-4.1-fast-non-reasoning`
    X__Ai___Grok__4_1__Fast__Non__Reasoning,
    /// Model: `x-ai/grok-code-fast-1`
    X__Ai___Grok__Code__Fast__1,
    /// Model: `xiaomi/mimo-v2-flash`
    Xiaomi___Mimo__V2__Flash,
    /// Model: `xiaomi/mimo-v2-flash-free`
    Xiaomi___Mimo__V2__Flash__Free,
    /// Model: `z-ai/glm-4.5`
    Z__Ai___Glm__4_5,
    /// Model: `z-ai/glm-4.5-air`
    Z__Ai___Glm__4_5__Air,
    /// Model: `z-ai/glm-4.6`
    Z__Ai___Glm__4_6,
    /// Model: `z-ai/glm-4.6v`
    Z__Ai___Glm__4_6v,
    /// Model: `z-ai/glm-4.6v-flash`
    Z__Ai___Glm__4_6v__Flash,
    /// Model: `z-ai/glm-4.6v-flash-free`
    Z__Ai___Glm__4_6v__Flash__Free,
    /// Model: `z-ai/glm-4.7`
    Z__Ai___Glm__4_7,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
