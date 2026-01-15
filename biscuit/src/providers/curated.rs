// Curated LLM Model Registry
//
// This file contains a manually curated list of LLM models from major providers.
// It is based on information from official provider documentation and registries.
//
// SOURCES:
// - Vercel AI SDK Model Catalog: https://vercel.com/ai-gateway/models
// - OpenAI API: https://api.openai.com/v1/models (dynamically fetched)
// - Anthropic Docs: https://docs.anthropic.com/
// - DeepSeek Docs: https://www.deepseek.com/
// - Mistral Docs: https://docs.mistral.ai/
// - Google AI Docs: https://ai.google.dev/
// - Cohere Docs: https://docs.cohere.com/
// - xAI Docs: https://docs.x.ai/
//
// UPDATE FREQUENCY:
// - Check quarterly for new major model releases
// - Update when providers announce significant new models
// - Sources date: 2025-12-29
//
// HOW TO UPDATE:
// 1. Visit Vercel AI Gateway models page
// 2. Check provider official documentation
// 3. Add new models to appropriate provider function below
// 4. Update LAST_UPDATED constant
// 5. Run tests: `cargo test providers::curated`

use super::types::LlmEntry;

/// Last update date for this registry
pub const LAST_UPDATED: &str = "2025-12-29";

/// Total number of providers in the curated registry
pub const PROVIDER_COUNT: usize = 12;

/// Get all curated models from all providers
pub fn get_curated_models() -> Vec<LlmEntry> {
    let mut models = Vec::new();

    models.extend(get_openai_models());
    models.extend(get_anthropic_models());
    models.extend(get_google_models());
    models.extend(get_deepseek_models());
    models.extend(get_mistral_models());
    models.extend(get_cohere_models());
    models.extend(get_xai_models());
    models.extend(get_meta_models());
    models.extend(get_perplexity_models());
    models.extend(get_alibaba_models());
    models.extend(get_together_models());
    models.extend(get_replicate_models());

    models
}

/// OpenAI models (GPT, o-series, embeddings, codex)
///
/// Source: https://platform.openai.com/docs/models
fn get_openai_models() -> Vec<LlmEntry> {
    vec![
        // GPT-5 Series
        LlmEntry::new("openai", "gpt-5.2"),
        LlmEntry::new("openai", "gpt-5.2-pro"),
        LlmEntry::new("openai", "gpt-5.2-chat"),
        LlmEntry::new("openai", "gpt-5"),
        LlmEntry::new("openai", "gpt-5-pro"),
        LlmEntry::new("openai", "gpt-5-codex"),
        LlmEntry::new("openai", "gpt-5.1-instant"),
        LlmEntry::new("openai", "gpt-5.1-thinking"),
        // GPT-4.1 Series
        LlmEntry::new("openai", "gpt-4.1"),
        LlmEntry::new("openai", "gpt-4.1-mini"),
        LlmEntry::new("openai", "gpt-4.1-nano"),
        // GPT-4o Series
        LlmEntry::new("openai", "gpt-4o"),
        LlmEntry::new("openai", "gpt-4o-mini"),
        // GPT-3.5 Series
        LlmEntry::new("openai", "gpt-3.5-turbo"),
        LlmEntry::new("openai", "gpt-3.5-turbo-instruct"),
        // o-series (Reasoning Models)
        LlmEntry::new("openai", "o1"),
        LlmEntry::new("openai", "o3"),
        LlmEntry::new("openai", "o3-mini"),
        LlmEntry::new("openai", "o3-pro"),
        LlmEntry::new("openai", "o3-deep-research"),
        LlmEntry::new("openai", "o4-mini"),
        // Codex
        LlmEntry::new("openai", "codex-mini"),
        // Embeddings
        LlmEntry::new("openai", "text-embedding-3-large"),
        LlmEntry::new("openai", "text-embedding-3-small"),
        // Open-source models (GPT-OSS)
        LlmEntry::new("openai", "gpt-oss-120b"),
        LlmEntry::new("openai", "gpt-oss-20b"),
        LlmEntry::new("openai", "gpt-oss-safeguard-20b"),
    ]
}

/// Anthropic Claude models
///
/// Source: https://docs.anthropic.com/claude/docs/models-overview
fn get_anthropic_models() -> Vec<LlmEntry> {
    vec![
        // Claude 4.5 Series (Latest)
        LlmEntry::new("anthropic", "claude-opus-4.5-20250929"),
        LlmEntry::new("anthropic", "claude-sonnet-4.5-20250929"),
        LlmEntry::new("anthropic", "claude-haiku-4.5-20250929"),
        // Claude 4.1 Series
        LlmEntry::new("anthropic", "claude-opus-4.1"),
        // Claude 4 Series
        LlmEntry::new("anthropic", "claude-opus-4"),
        LlmEntry::new("anthropic", "claude-sonnet-4"),
        // Claude 3.7 Series
        LlmEntry::new("anthropic", "claude-3.7-sonnet"),
        // Claude 3.5 Series
        LlmEntry::new("anthropic", "claude-3.5-sonnet"),
        LlmEntry::new("anthropic", "claude-3.5-haiku"),
        // Claude 3 Series
        LlmEntry::new("anthropic", "claude-3-opus"),
        LlmEntry::new("anthropic", "claude-3-haiku"),
    ]
}

/// Google Gemini models
///
/// Source: https://ai.google.dev/models/gemini
fn get_google_models() -> Vec<LlmEntry> {
    vec![
        // Gemini 3 Series (Latest)
        LlmEntry::new("google", "gemini-3-pro-preview"),
        LlmEntry::new("google", "gemini-3-pro-image"),
        LlmEntry::new("google", "gemini-3-flash"),
        // Gemini 2.5 Series
        LlmEntry::new("google", "gemini-2.5-pro"),
        LlmEntry::new("google", "gemini-2.5-flash"),
        LlmEntry::new("google", "gemini-2.5-flash-lite"),
        LlmEntry::new("google", "gemini-2.5-flash-image"),
        LlmEntry::new("google", "gemini-2.5-flash-preview-09-2025"),
        LlmEntry::new("google", "gemini-2.5-flash-lite-preview-09-2025"),
        // Gemini 2.0 Series
        LlmEntry::new("google", "gemini-2.0-flash"),
        // Embeddings
        LlmEntry::new("google", "gemini-embedding-001"),
        LlmEntry::new("google", "text-embedding-005"),
        LlmEntry::new("google", "text-multilingual-embedding-002"),
        // Image Generation (Imagen)
        LlmEntry::new("google", "imagen-4.0-generate-001"),
        LlmEntry::new("google", "imagen-4.0-ultra-generate-001"),
        LlmEntry::new("google", "imagen-4.0-fast-generate-001"),
    ]
}

/// DeepSeek models
///
/// Source: https://www.deepseek.com/
fn get_deepseek_models() -> Vec<LlmEntry> {
    vec![
        // V3.2 Series (Latest)
        LlmEntry::new("deepseek", "deepseek-v3.2"),
        LlmEntry::new("deepseek", "deepseek-v3.2-thinking"),
        LlmEntry::new("deepseek", "deepseek-v3.2-exp"),
        // V3.1 Series
        LlmEntry::new("deepseek", "deepseek-v3.1"),
        LlmEntry::new("deepseek", "deepseek-v3.1-terminus"),
        // V3 Series
        LlmEntry::new("deepseek", "deepseek-v3"),
        // R1 Series (Reasoning)
        LlmEntry::new("deepseek", "deepseek-r1"),
    ]
}

/// Mistral AI models
///
/// Source: https://docs.mistral.ai/
fn get_mistral_models() -> Vec<LlmEntry> {
    vec![
        // Magistral Series (Latest)
        LlmEntry::new("mistral", "magistral-medium"),
        LlmEntry::new("mistral", "magistral-small"),
        // Devstral Series (Developer-focused)
        LlmEntry::new("mistral", "devstral-2"),
        LlmEntry::new("mistral", "devstral-small-2"),
        LlmEntry::new("mistral", "devstral-small"),
        // Codestral Series (Code generation)
        LlmEntry::new("mistral", "codestral"),
        LlmEntry::new("mistral", "codestral-embed"),
        // Ministral Series (Efficient models)
        LlmEntry::new("mistral", "ministral-3b"),
        LlmEntry::new("mistral", "ministral-8b"),
        LlmEntry::new("mistral", "ministral-14b"),
        // Pixtral Series (Multimodal)
        LlmEntry::new("mistral", "pixtral-12b"),
        LlmEntry::new("mistral", "pixtral-large"),
        // Mixtral Series (Mixture of Experts)
        LlmEntry::new("mistral", "mixtral-8x22b-instruct"),
        // Standard Series
        LlmEntry::new("mistral", "mistral-small"),
        LlmEntry::new("mistral", "mistral-medium"),
        // Embeddings
        LlmEntry::new("mistral", "mistral-embed"),
    ]
}

/// Cohere models
///
/// Source: https://docs.cohere.com/
fn get_cohere_models() -> Vec<LlmEntry> {
    vec![
        // Command Series (Chat/Generation)
        LlmEntry::new("cohere", "command-a"),
        LlmEntry::new("cohere", "command-a-plus"),
        LlmEntry::new("cohere", "command-r"),
        LlmEntry::new("cohere", "command-r-plus"),
        LlmEntry::new("cohere", "command-r7b-12-2024"),
        LlmEntry::new("cohere", "command-r-08-2024"),
        // C4AI Series
        LlmEntry::new("cohere", "c4ai-aya-expanse-8b"),
        LlmEntry::new("cohere", "c4ai-aya-expanse-32b"),
        // Embeddings
        LlmEntry::new("cohere", "embed-v4.0"),
        LlmEntry::new("cohere", "embed-v3.5"),
        LlmEntry::new("cohere", "embed-english-v3.0"),
        LlmEntry::new("cohere", "embed-english-light-v3.0"),
        LlmEntry::new("cohere", "embed-multilingual-v3.0"),
        LlmEntry::new("cohere", "embed-multilingual-light-v3.0"),
        // Rerank
        LlmEntry::new("cohere", "rerank-v3.5"),
        LlmEntry::new("cohere", "rerank-english-v3.0"),
        LlmEntry::new("cohere", "rerank-multilingual-v3.0"),
    ]
}

/// xAI Grok models
///
/// Source: https://docs.x.ai/
fn get_xai_models() -> Vec<LlmEntry> {
    vec![
        // Grok 4.1 Series (Latest)
        LlmEntry::new("xai", "grok-4.1-fast-reasoning"),
        LlmEntry::new("xai", "grok-4.1-fast-non-reasoning"),
        // Grok 4 Series
        LlmEntry::new("xai", "grok-4-fast-reasoning"),
        LlmEntry::new("xai", "grok-4-fast-non-reasoning"),
        // Grok 3 Series
        LlmEntry::new("xai", "grok-3"),
        LlmEntry::new("xai", "grok-3-mini"),
        LlmEntry::new("xai", "grok-3-mini-fast"),
        // Specialized Models
        LlmEntry::new("xai", "grok-code-fast-1"),
    ]
}

/// Meta Llama models
///
/// Source: https://llama.meta.com/
fn get_meta_models() -> Vec<LlmEntry> {
    vec![
        // Llama 4 Series (Latest)
        LlmEntry::new("meta", "llama-4-405b"),
        LlmEntry::new("meta", "llama-4-70b"),
        LlmEntry::new("meta", "llama-4-13b"),
        // Llama 3.3 Series
        LlmEntry::new("meta", "llama-3.3-70b"),
        // Llama 3.1 Series
        LlmEntry::new("meta", "llama-3.1-405b"),
        LlmEntry::new("meta", "llama-3.1-70b"),
        LlmEntry::new("meta", "llama-3.1-8b"),
        // Llama 3 Series
        LlmEntry::new("meta", "llama-3-70b"),
        LlmEntry::new("meta", "llama-3-8b"),
    ]
}

/// Perplexity models
///
/// Source: https://docs.perplexity.ai/
fn get_perplexity_models() -> Vec<LlmEntry> {
    vec![
        // Sonar Series
        LlmEntry::new("perplexity", "sonar-pro"),
        LlmEntry::new("perplexity", "sonar"),
        LlmEntry::new("perplexity", "sonar-reasoning"),
        LlmEntry::new("perplexity", "sonar-medium-online"),
        LlmEntry::new("perplexity", "sonar-medium-chat"),
        LlmEntry::new("perplexity", "sonar-small-online"),
        LlmEntry::new("perplexity", "sonar-small-chat"),
    ]
}

/// Alibaba Qwen models
///
/// Source: https://huggingface.co/Qwen
fn get_alibaba_models() -> Vec<LlmEntry> {
    vec![
        // Qwen 3 Series (Latest)
        LlmEntry::new("alibaba", "qwen-3-235b"),
        LlmEntry::new("alibaba", "qwen-3-70b"),
        LlmEntry::new("alibaba", "qwen-3-14b"),
        LlmEntry::new("alibaba", "qwen-3-7b"),
        // Qwen 2.5 Series
        LlmEntry::new("alibaba", "qwen-2.5-72b"),
        LlmEntry::new("alibaba", "qwen-2.5-32b"),
        LlmEntry::new("alibaba", "qwen-2.5-14b"),
        LlmEntry::new("alibaba", "qwen-2.5-7b"),
        // Qwen Coder Series
        LlmEntry::new("alibaba", "qwen-coder-32b"),
        LlmEntry::new("alibaba", "qwen-coder-7b"),
    ]
}

/// Together.AI models
///
/// Source: https://www.together.ai/
fn get_together_models() -> Vec<LlmEntry> {
    vec![
        // Meta Llama via Together
        LlmEntry::new("together", "meta-llama/Llama-3.3-70B-Instruct-Turbo"),
        LlmEntry::new("together", "meta-llama/Llama-3.1-405B-Instruct-Turbo"),
        LlmEntry::new("together", "meta-llama/Llama-3.1-70B-Instruct-Turbo"),
        LlmEntry::new("together", "meta-llama/Llama-3.1-8B-Instruct-Turbo"),
        // Mistral via Together
        LlmEntry::new("together", "mistralai/Mistral-7B-Instruct-v0.3"),
        LlmEntry::new("together", "mistralai/Mixtral-8x7B-Instruct-v0.1"),
        LlmEntry::new("together", "mistralai/Mixtral-8x22B-Instruct-v0.1"),
        // Qwen via Together
        LlmEntry::new("together", "Qwen/Qwen2.5-72B-Instruct-Turbo"),
        LlmEntry::new("together", "Qwen/Qwen2.5-7B-Instruct-Turbo"),
        LlmEntry::new("together", "Qwen/QwQ-32B-Preview"),
        // DeepSeek via Together
        LlmEntry::new("together", "deepseek-ai/DeepSeek-V3"),
        // Google via Together
        LlmEntry::new("together", "google/gemma-2-27b-it"),
        LlmEntry::new("together", "google/gemma-2-9b-it"),
    ]
}

/// Replicate models
///
/// Source: https://replicate.com/
fn get_replicate_models() -> Vec<LlmEntry> {
    vec![
        // Meta Llama
        LlmEntry::new("replicate", "meta/llama-3.3-70b-instruct"),
        LlmEntry::new("replicate", "meta/llama-3.1-405b-instruct"),
        LlmEntry::new("replicate", "meta/llama-3.1-70b-instruct"),
        // Mistral
        LlmEntry::new("replicate", "mistralai/mistral-7b-instruct-v0.2"),
        LlmEntry::new("replicate", "mistralai/mixtral-8x7b-instruct-v0.1"),
        // Stability AI
        LlmEntry::new("replicate", "stability-ai/stable-diffusion-3"),
        LlmEntry::new("replicate", "stability-ai/sdxl"),
        // Anthropic (via proxy)
        LlmEntry::new("replicate", "anthropic/claude-3-sonnet"),
        LlmEntry::new("replicate", "anthropic/claude-3-haiku"),
        // Community Models
        LlmEntry::new("replicate", "yorickvp/llava-13b"),
        LlmEntry::new("replicate", "cjwbw/seamless_communication"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curated_models_count() {
        let models = get_curated_models();
        println!("Total curated models: {}", models.len());
        assert!(
            models.len() >= 150,
            "Expected at least 150 models, got {}",
            models.len()
        );
    }

    #[test]
    fn curated_models_have_providers() {
        let models = get_curated_models();
        assert!(models.iter().all(|m| !m.provider.is_empty()));
        assert!(models.iter().all(|m| !m.model.is_empty()));
    }

    #[test]
    fn openai_models_present() {
        let models = get_openai_models();
        assert!(models.len() > 20);
        assert!(models.iter().any(|m| m.model.contains("gpt-5")));
        assert!(models.iter().any(|m| m.model.contains("o3")));
    }

    #[test]
    fn anthropic_models_present() {
        let models = get_anthropic_models();
        assert!(models.len() >= 10);
        assert!(models.iter().any(|m| m.model.contains("claude-opus-4.5")));
        assert!(models.iter().any(|m| m.model.contains("claude-sonnet-4.5")));
    }

    #[test]
    fn deepseek_models_present() {
        let models = get_deepseek_models();
        assert!(models.len() >= 5);
        assert!(models.iter().any(|m| m.model.contains("deepseek-v3")));
        assert!(models.iter().any(|m| m.model.contains("deepseek-r1")));
    }

    #[test]
    fn mistral_models_present() {
        let models = get_mistral_models();
        assert!(models.len() >= 15);
        assert!(models.iter().any(|m| m.model.contains("codestral")));
        assert!(models.iter().any(|m| m.model.contains("ministral")));
    }

    #[test]
    fn all_providers_represented() {
        let models = get_curated_models();
        let providers: std::collections::HashSet<_> =
            models.iter().map(|m| m.provider.as_str()).collect();

        assert!(providers.contains("openai"));
        assert!(providers.contains("anthropic"));
        assert!(providers.contains("google"));
        assert!(providers.contains("deepseek"));
        assert!(providers.contains("mistral"));
        assert!(providers.contains("cohere"));
        assert!(providers.contains("xai"));
        assert!(providers.contains("meta"));
        assert!(providers.contains("perplexity"));
        assert!(providers.contains("alibaba"));
        assert!(providers.contains("together"));
        assert!(providers.contains("replicate"));

        assert!(providers.len() >= PROVIDER_COUNT);
    }

    #[test]
    fn no_duplicate_models() {
        let models = get_curated_models();
        let mut seen = std::collections::HashSet::new();

        for model in &models {
            let key = format!("{}/{}", model.provider, model.model);
            assert!(!seen.contains(&key), "Duplicate model: {}", key);
            seen.insert(key);
        }
    }
}
