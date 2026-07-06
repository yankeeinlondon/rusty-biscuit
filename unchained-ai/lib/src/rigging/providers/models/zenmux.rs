//! Auto-generated provider model enum
//!
//! Generated: 2026-07-06T22:44:37.286219+00:00
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
    /// Model: `anthropic/claude-fable-5`
    Anthropic___Claude__Fable__5,
    /// Model: `anthropic/claude-fable-5-free`
    Anthropic___Claude__Fable__5__Free,
    /// Model: `anthropic/claude-haiku-4.5`
    Anthropic___Claude__Haiku__4_5,
    /// Model: `anthropic/claude-opus-4`
    Anthropic___Claude__Opus__4,
    /// Model: `anthropic/claude-opus-4.1`
    Anthropic___Claude__Opus__4_1,
    /// Model: `anthropic/claude-opus-4.5`
    Anthropic___Claude__Opus__4_5,
    /// Model: `anthropic/claude-opus-4.6`
    Anthropic___Claude__Opus__4_6,
    /// Model: `anthropic/claude-opus-4.7`
    Anthropic___Claude__Opus__4_7,
    /// Model: `anthropic/claude-opus-4.8`
    Anthropic___Claude__Opus__4_8,
    /// Model: `anthropic/claude-sonnet-4`
    Anthropic___Claude__Sonnet__4,
    /// Model: `anthropic/claude-sonnet-4.5`
    Anthropic___Claude__Sonnet__4_5,
    /// Model: `anthropic/claude-sonnet-4.6`
    Anthropic___Claude__Sonnet__4_6,
    /// Model: `anthropic/claude-sonnet-5`
    Anthropic___Claude__Sonnet__5,
    /// Model: `anthropic/claude-sonnet-5-free`
    Anthropic___Claude__Sonnet__5__Free,
    /// Model: `baidu/ernie-5.0-thinking-preview`
    Baidu___Ernie__5_0__Thinking__Preview,
    /// Model: `baidu/ernie-5.1`
    Baidu___Ernie__5_1,
    /// Model: `baidu/ernie-x1.1-preview`
    Baidu___Ernie__X1_1__Preview,
    /// Model: `bytedance/doubao-seed-1.8`
    Bytedance___Doubao__Seed__1_8,
    /// Model: `bytedance/doubao-seed-2.0-code`
    Bytedance___Doubao__Seed__2_0__Code,
    /// Model: `bytedance/doubao-seed-2.0-lite`
    Bytedance___Doubao__Seed__2_0__Lite,
    /// Model: `bytedance/doubao-seed-2.0-mini`
    Bytedance___Doubao__Seed__2_0__Mini,
    /// Model: `bytedance/doubao-seed-2.0-pro`
    Bytedance___Doubao__Seed__2_0__Pro,
    /// Model: `bytedance/doubao-seed-2.1-pro`
    Bytedance___Doubao__Seed__2_1__Pro,
    /// Model: `bytedance/doubao-seed-2.1-turbo`
    Bytedance___Doubao__Seed__2_1__Turbo,
    /// Model: `bytedance/doubao-seed-character`
    Bytedance___Doubao__Seed__Character,
    /// Model: `bytedance/doubao-seed-code`
    Bytedance___Doubao__Seed__Code,
    /// Model: `bytedance/doubao-seed-evolving`
    Bytedance___Doubao__Seed__Evolving,
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
    /// Model: `deepseek/deepseek-v4-flash`
    Deepseek___Deepseek__V4__Flash,
    /// Model: `deepseek/deepseek-v4-pro`
    Deepseek___Deepseek__V4__Pro,
    /// Model: `google/gemini-2.5-flash`
    Google___Gemini__2_5__Flash,
    /// Model: `google/gemini-2.5-flash-lite`
    Google___Gemini__2_5__Flash__Lite,
    /// Model: `google/gemini-2.5-pro`
    Google___Gemini__2_5__Pro,
    /// Model: `google/gemini-3-flash-preview`
    Google___Gemini__3__Flash__Preview,
    /// Model: `google/gemini-3.1-flash-lite`
    Google___Gemini__3_1__Flash__Lite,
    /// Model: `google/gemini-3.1-flash-lite-preview`
    Google___Gemini__3_1__Flash__Lite__Preview,
    /// Model: `google/gemini-3.1-pro-preview`
    Google___Gemini__3_1__Pro__Preview,
    /// Model: `google/gemini-3.5-flash`
    Google___Gemini__3_5__Flash,
    /// Model: `google/gemini-embedding-2`
    Google___Gemini__Embedding__2,
    /// Model: `inclusionai/ling-2.6-1t`
    Inclusionai___Ling__2_6__1t,
    /// Model: `inclusionai/ling-2.6-flash`
    Inclusionai___Ling__2_6__Flash,
    /// Model: `inclusionai/llada2.1-flash`
    Inclusionai___Llada2_1__Flash,
    /// Model: `inclusionai/ring-2.6-1t`
    Inclusionai___Ring__2_6__1t,
    /// Model: `kuaishou/kat-coder-pro-v2`
    Kuaishou___Kat__Coder__Pro__V2,
    /// Model: `meituan/longcat-2.0`
    Meituan___Longcat__2_0,
    /// Model: `meta/llama-3.3-70b-instruct`
    Meta___Llama__3_3__70b__Instruct,
    /// Model: `meta/llama-4-scout-17b-16e-instruct`
    Meta___Llama__4__Scout__17b__16e__Instruct,
    /// Model: `minimax/minimax-m2`
    Minimax___Minimax__M2,
    /// Model: `minimax/minimax-m2-her`
    Minimax___Minimax__M2__Her,
    /// Model: `minimax/minimax-m2.1`
    Minimax___Minimax__M2_1,
    /// Model: `minimax/minimax-m2.5`
    Minimax___Minimax__M2_5,
    /// Model: `minimax/minimax-m2.5-lightning`
    Minimax___Minimax__M2_5__Lightning,
    /// Model: `minimax/minimax-m2.7`
    Minimax___Minimax__M2_7,
    /// Model: `minimax/minimax-m2.7-highspeed`
    Minimax___Minimax__M2_7__Highspeed,
    /// Model: `minimax/minimax-m3`
    Minimax___Minimax__M3,
    /// Model: `mistralai/mistral-large-2512`
    Mistralai___Mistral__Large__2512,
    /// Model: `moonshotai/kimi-k2.5`
    Moonshotai___Kimi__K2_5,
    /// Model: `moonshotai/kimi-k2.6`
    Moonshotai___Kimi__K2_6,
    /// Model: `moonshotai/kimi-k2.7-code`
    Moonshotai___Kimi__K2_7__Code,
    /// Model: `moonshotai/kimi-k2.7-code-highspeed`
    Moonshotai___Kimi__K2_7__Code__Highspeed,
    /// Model: `openai/chat-latest`
    Openai___Chat__Latest,
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
    /// Model: `openai/gpt-5.2-codex`
    Openai___Gpt__5_2__Codex,
    /// Model: `openai/gpt-5.2-pro`
    Openai___Gpt__5_2__Pro,
    /// Model: `openai/gpt-5.3-chat`
    Openai___Gpt__5_3__Chat,
    /// Model: `openai/gpt-5.3-codex`
    Openai___Gpt__5_3__Codex,
    /// Model: `openai/gpt-5.4`
    Openai___Gpt__5_4,
    /// Model: `openai/gpt-5.4-mini`
    Openai___Gpt__5_4__Mini,
    /// Model: `openai/gpt-5.4-nano`
    Openai___Gpt__5_4__Nano,
    /// Model: `openai/gpt-5.4-pro`
    Openai___Gpt__5_4__Pro,
    /// Model: `openai/gpt-5.5`
    Openai___Gpt__5_5,
    /// Model: `openai/gpt-5.5-pro`
    Openai___Gpt__5_5__Pro,
    /// Model: `openai/gpt-image-1.5`
    Openai___Gpt__Image__1_5,
    /// Model: `openai/gpt-image-2`
    Openai___Gpt__Image__2,
    /// Model: `openai/o4-mini`
    Openai___O4__Mini,
    /// Model: `openai/text-embedding-3-large`
    Openai___Text__Embedding__3__Large,
    /// Model: `openai/text-embedding-3-small`
    Openai___Text__Embedding__3__Small,
    /// Model: `qwen/qwen3-14b`
    Qwen___Qwen3__14b,
    /// Model: `qwen/qwen3-235b-a22b-2507`
    Qwen___Qwen3__235b__A22b__2507,
    /// Model: `qwen/qwen3-235b-a22b-thinking-2507`
    Qwen___Qwen3__235b__A22b__Thinking__2507,
    /// Model: `qwen/qwen3-asr-flash`
    Qwen___Qwen3__Asr__Flash,
    /// Model: `qwen/qwen3-coder`
    Qwen___Qwen3__Coder,
    /// Model: `qwen/qwen3-coder-plus`
    Qwen___Qwen3__Coder__Plus,
    /// Model: `qwen/qwen3-max`
    Qwen___Qwen3__Max,
    /// Model: `qwen/qwen3-vl-embedding`
    Qwen___Qwen3__Vl__Embedding,
    /// Model: `qwen/qwen3-vl-plus`
    Qwen___Qwen3__Vl__Plus,
    /// Model: `qwen/qwen3.5-flash`
    Qwen___Qwen3_5__Flash,
    /// Model: `qwen/qwen3.5-plus`
    Qwen___Qwen3_5__Plus,
    /// Model: `qwen/qwen3.6-flash`
    Qwen___Qwen3_6__Flash,
    /// Model: `qwen/qwen3.6-max-preview`
    Qwen___Qwen3_6__Max__Preview,
    /// Model: `qwen/qwen3.6-plus`
    Qwen___Qwen3_6__Plus,
    /// Model: `qwen/qwen3.7-max`
    Qwen___Qwen3_7__Max,
    /// Model: `qwen/qwen3.7-plus`
    Qwen___Qwen3_7__Plus,
    /// Model: `sapiens-ai/agnes-2.0-flash`
    Sapiens__Ai___Agnes__2_0__Flash,
    /// Model: `stepfun/step-3`
    Stepfun___Step__3,
    /// Model: `stepfun/step-3.5-flash`
    Stepfun___Step__3_5__Flash,
    /// Model: `stepfun/step-3.7-flash`
    Stepfun___Step__3_7__Flash,
    /// Model: `stepfun/step-3.7-flash-free`
    Stepfun___Step__3_7__Flash__Free,
    /// Model: `tencent/hy3`
    Tencent___Hy3,
    /// Model: `tencent/hy3-preview`
    Tencent___Hy3__Preview,
    /// Model: `x-ai/grok-4.2-fast`
    X__Ai___Grok__4_2__Fast,
    /// Model: `x-ai/grok-4.2-fast-non-reasoning`
    X__Ai___Grok__4_2__Fast__Non__Reasoning,
    /// Model: `x-ai/grok-4.3`
    X__Ai___Grok__4_3,
    /// Model: `x-ai/grok-build-0.1`
    X__Ai___Grok__Build__0_1,
    /// Model: `xiaomi/mimo-v2.5`
    Xiaomi___Mimo__V2_5,
    /// Model: `xiaomi/mimo-v2.5-pro`
    Xiaomi___Mimo__V2_5__Pro,
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
    /// Model: `z-ai/glm-4.7-flash-free`
    Z__Ai___Glm__4_7__Flash__Free,
    /// Model: `z-ai/glm-4.7-flashx`
    Z__Ai___Glm__4_7__Flashx,
    /// Model: `z-ai/glm-5`
    Z__Ai___Glm__5,
    /// Model: `z-ai/glm-5-turbo`
    Z__Ai___Glm__5__Turbo,
    /// Model: `z-ai/glm-5.1`
    Z__Ai___Glm__5_1,
    /// Model: `z-ai/glm-5.2`
    Z__Ai___Glm__5_2,
    /// Model: `z-ai/glm-5v-turbo`
    Z__Ai___Glm__5v__Turbo,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
