use std::rc::Rc;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};
use serde_json::{Map, Value, json};
use unchained_ai::models::model_default_parameters::ModelDefaultParameters;
use unchained_ai::models::model_metadata::{ModelModalities, ProviderModelMetadata};
use unchained_ai::models::model_pricing::ModelPricing;
use unchained_ai::rigging::providers::Provider;
use unchained_ai::rigging::providers::models::{
    ProviderModel, anthropic::ProviderModelAnthropic, deepseek::ProviderModelDeepseek,
    gemini::ProviderModelGemini, groq::ProviderModelGroq, mistral::ProviderModelMistral,
    moonshotai::ProviderModelMoonshotAi, openai::ProviderModelOpenAi,
    openrouter::ProviderModelOpenRouter, xai::ProviderModelXai, zai::ProviderModelZai,
    zenmux::ProviderModelZenMux,
};

/// Run the `models` subcommand.
pub async fn run(
    provider_filter: Option<String>,
    json: bool,
    verbose: bool,
    flat: bool,
) -> Result<()> {
    let filter = match provider_filter.as_deref() {
        Some(name) => Some(parse_provider(name)?),
        None => None,
    };

    let groups = collect_models(filter);

    if json {
        render_json(&groups)?;
    } else if flat {
        render_flat_terminal(&groups);
    } else {
        render_terminal(&groups, verbose);
    }

    Ok(())
}

fn parse_provider(name: &str) -> Result<Provider> {
    let key: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    match key.as_str() {
        "anthropic" => Ok(Provider::Anthropic),
        "deepseek" => Ok(Provider::Deepseek),
        "gemini" | "google" => Ok(Provider::Gemini),
        "groq" => Ok(Provider::Groq),
        "mistral" => Ok(Provider::Mistral),
        "moonshot" | "moonshotai" => Ok(Provider::MoonshotAi),
        "openai" => Ok(Provider::OpenAi),
        "openrouter" => Ok(Provider::OpenRouter),
        "xai" => Ok(Provider::Xai),
        "zai" => Ok(Provider::Zai),
        "zenmux" => Ok(Provider::ZenMux),
        _ => Err(eyre!(
            "Unknown provider '{name}'. Valid: anthropic, deepseek, gemini, groq, mistral, \
             moonshotai, openai, openrouter, xai, zai, zenmux"
        )),
    }
}

fn collect_models(filter: Option<Provider>) -> Vec<(Provider, Vec<ProviderModel>)> {
    let providers = [
        Provider::Anthropic,
        Provider::Deepseek,
        Provider::Gemini,
        Provider::Groq,
        Provider::Mistral,
        Provider::MoonshotAi,
        Provider::OpenAi,
        Provider::OpenRouter,
        Provider::Xai,
        Provider::Zai,
        Provider::ZenMux,
    ];

    providers
        .into_iter()
        .filter(|p| filter.is_none_or(|f| f == *p))
        .map(|p| (p, models_for(p)))
        .collect()
}

fn models_for(provider: Provider) -> Vec<ProviderModel> {
    match provider {
        Provider::Anthropic => ProviderModelAnthropic::ALL
            .iter()
            .cloned()
            .map(ProviderModel::Anthropic)
            .collect(),
        Provider::Deepseek => ProviderModelDeepseek::ALL
            .iter()
            .cloned()
            .map(ProviderModel::Deepseek)
            .collect(),
        Provider::Gemini => ProviderModelGemini::ALL
            .iter()
            .cloned()
            .map(ProviderModel::Gemini)
            .collect(),
        Provider::Groq => ProviderModelGroq::ALL
            .iter()
            .cloned()
            .map(ProviderModel::Groq)
            .collect(),
        Provider::Mistral => ProviderModelMistral::ALL
            .iter()
            .cloned()
            .map(ProviderModel::Mistral)
            .collect(),
        Provider::MoonshotAi => ProviderModelMoonshotAi::ALL
            .iter()
            .cloned()
            .map(ProviderModel::MoonshotAi)
            .collect(),
        Provider::OpenAi => ProviderModelOpenAi::ALL
            .iter()
            .cloned()
            .map(ProviderModel::OpenAi)
            .collect(),
        Provider::OpenRouter => ProviderModelOpenRouter::ALL
            .iter()
            .cloned()
            .map(ProviderModel::OpenRouter)
            .collect(),
        Provider::Xai => ProviderModelXai::ALL
            .iter()
            .cloned()
            .map(ProviderModel::Xai)
            .collect(),
        Provider::Zai => ProviderModelZai::ALL
            .iter()
            .cloned()
            .map(ProviderModel::Zai)
            .collect(),
        Provider::ZenMux => ProviderModelZenMux::ALL
            .iter()
            .cloned()
            .map(ProviderModel::ZenMux)
            .collect(),
        Provider::HuggingFace | Provider::Ollama => Vec::new(),
    }
}

fn provider_display(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => "Anthropic",
        Provider::Deepseek => "DeepSeek",
        Provider::Gemini => "Google Gemini",
        Provider::Groq => "Groq",
        Provider::HuggingFace => "Hugging Face",
        Provider::Mistral => "Mistral",
        Provider::MoonshotAi => "Moonshot AI",
        Provider::Ollama => "Ollama",
        Provider::OpenAi => "OpenAI",
        Provider::OpenRouter => "OpenRouter",
        Provider::Xai => "xAI",
        Provider::Zai => "Z.ai",
        Provider::ZenMux => "ZenMux",
    }
}

fn render_json(groups: &[(Provider, Vec<ProviderModel>)]) -> Result<()> {
    let mut entries: Vec<Value> = Vec::new();

    for (_provider, models) in groups {
        for model in models {
            let attributes = metadata_to_json(model.metadata());
            entries.push(json!({
                "model": model.wire_id(),
                "attributes": attributes,
            }));
        }
    }

    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}

fn metadata_to_json(meta: Option<&ProviderModelMetadata>) -> Value {
    let mut map = Map::new();
    let Some(meta) = meta else {
        return Value::Object(map);
    };

    if let Some(name) = &meta.display_name {
        map.insert("display_name".into(), Value::String(name.clone()));
    }
    if let Some(family) = &meta.family {
        map.insert("family".into(), Value::String(family.clone()));
    }
    if let Some(ctx) = meta.context_window {
        map.insert("context_window".into(), Value::Number(ctx.into()));
    }
    if let Some(max_out) = meta.max_output_tokens {
        map.insert("max_output_tokens".into(), Value::Number(max_out.into()));
    }
    if let Some(modalities) = &meta.modalities {
        map.insert("modalities".into(), modalities_to_json(modalities));
    }
    if !meta.capabilities.is_empty() {
        map.insert(
            "capabilities".into(),
            Value::Array(
                meta.capabilities
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
    if let Some(desc) = &meta.description {
        map.insert("description".into(), Value::String(desc.clone()));
    }
    if let Some(pricing) = &meta.pricing {
        map.insert("pricing".into(), pricing_to_json(pricing));
    }
    if let Some(params) = &meta.supported_parameters {
        map.insert(
            "supported_parameters".into(),
            Value::Array(params.iter().cloned().map(Value::String).collect()),
        );
    }
    if let Some(defaults) = &meta.default_parameters {
        map.insert(
            "default_parameters".into(),
            default_parameters_to_json(defaults),
        );
    }
    if let Some(cutoff) = &meta.knowledge_cutoff {
        map.insert("knowledge_cutoff".into(), Value::String(cutoff.clone()));
    }
    if let Some(created) = meta.created {
        map.insert("created".into(), Value::Number(created.into()));
    }

    Value::Object(map)
}

fn modalities_to_json(modalities: &ModelModalities) -> Value {
    json!({
        "input": modalities.input.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
        "output": modalities.output.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
    })
}

fn pricing_to_json(pricing: &ModelPricing) -> Value {
    let mut map = Map::new();
    if let Some(v) = pricing.prompt_per_token {
        map.insert("prompt_per_token".into(), json!(v));
    }
    if let Some(v) = pricing.completion_per_token {
        map.insert("completion_per_token".into(), json!(v));
    }
    if let Some(v) = pricing.web_search_per_request {
        map.insert("web_search_per_request".into(), json!(v));
    }
    if let Some(v) = pricing.input_cache_read_per_token {
        map.insert("input_cache_read_per_token".into(), json!(v));
    }
    Value::Object(map)
}

fn default_parameters_to_json(params: &ModelDefaultParameters) -> Value {
    let mut map = Map::new();
    if let Some(v) = params.temperature {
        map.insert("temperature".into(), json!(v));
    }
    if let Some(v) = params.top_p {
        map.insert("top_p".into(), json!(v));
    }
    if let Some(v) = params.top_k {
        map.insert("top_k".into(), json!(v));
    }
    if let Some(v) = params.frequency_penalty {
        map.insert("frequency_penalty".into(), json!(v));
    }
    if let Some(v) = params.presence_penalty {
        map.insert("presence_penalty".into(), json!(v));
    }
    Value::Object(map)
}

fn render_terminal(groups: &[(Provider, Vec<ProviderModel>)], verbose: bool) {
    let term = Terminal::default();

    for (i, (provider, models)) in groups.iter().enumerate() {
        if i > 0 {
            println!();
        }
        let count = models.len();
        let header = Prose::new(format!(
            "<b>{}</b> <dim>({} models)</dim>",
            provider_display(*provider),
            count,
        ));
        println!("{}", header.render(&term));

        if models.is_empty() {
            println!("  (no models)");
            continue;
        }

        let list = build_provider_list(models, verbose);
        print!("{}", list.render(&term));
    }
}

fn render_flat_terminal(groups: &[(Provider, Vec<ProviderModel>)]) {
    let term = Terminal::default();

    for (_provider, models) in groups {
        for model in models {
            println!("{}", Prose::new(model.wire_id()).render(&term));
        }
    }
}

fn build_provider_list(models: &[ProviderModel], verbose: bool) -> UnorderedList {
    let mut items: Vec<RenderableTerminalContent> = Vec::with_capacity(models.len() * 2);

    for model in models {
        items.push(RenderableTerminalContent::String(
            model.model_id().to_string(),
        ));

        if verbose
            && let Some(meta) = model.metadata()
            && let Some(children) = build_metadata_list(meta)
        {
            items.push(RenderableTerminalContent::Component(Rc::new(children)));
        }
    }

    UnorderedList::from(items)
}

fn build_metadata_list(meta: &ProviderModelMetadata) -> Option<UnorderedList> {
    let mut entries: Vec<String> = Vec::new();

    if let Some(name) = &meta.display_name {
        entries.push(format!("display_name: {name}"));
    }
    if let Some(family) = &meta.family {
        entries.push(format!("family: {family}"));
    }
    if let Some(ctx) = meta.context_window {
        entries.push(format!("context_window: {ctx}"));
    }
    if let Some(max_out) = meta.max_output_tokens {
        entries.push(format!("max_output_tokens: {max_out}"));
    }
    if let Some(modalities) = &meta.modalities {
        let inputs: Vec<String> = modalities.input.iter().map(|m| m.to_string()).collect();
        let outputs: Vec<String> = modalities.output.iter().map(|m| m.to_string()).collect();
        entries.push(format!(
            "modalities: input=[{}] output=[{}]",
            inputs.join(", "),
            outputs.join(", "),
        ));
    }
    if !meta.capabilities.is_empty() {
        entries.push(format!("capabilities: {}", meta.capabilities.join(", ")));
    }
    if let Some(desc) = &meta.description {
        entries.push(format!("description: {desc}"));
    }
    if let Some(pricing) = &meta.pricing {
        entries.push(format_pricing(pricing));
    }
    if let Some(cutoff) = &meta.knowledge_cutoff {
        entries.push(format!("knowledge_cutoff: {cutoff}"));
    }
    if let Some(defaults) = &meta.default_parameters {
        entries.push(format_default_parameters(defaults));
    }
    if let Some(params) = &meta.supported_parameters {
        entries.push(format!("supported_parameters: {}", params.join(", ")));
    }

    if entries.is_empty() {
        None
    } else {
        Some(UnorderedList::new(entries))
    }
}

fn format_pricing(pricing: &ModelPricing) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = pricing.prompt_per_token {
        parts.push(format!("prompt=${v:.8}/tok"));
    }
    if let Some(v) = pricing.completion_per_token {
        parts.push(format!("completion=${v:.8}/tok"));
    }
    if let Some(v) = pricing.web_search_per_request {
        parts.push(format!("web_search=${v:.4}/req"));
    }
    if let Some(v) = pricing.input_cache_read_per_token {
        parts.push(format!("cache_read=${v:.8}/tok"));
    }
    if parts.is_empty() {
        "pricing: (no data)".to_string()
    } else {
        format!("pricing: {}", parts.join(", "))
    }
}

fn format_default_parameters(params: &ModelDefaultParameters) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = params.temperature {
        parts.push(format!("temp={v}"));
    }
    if let Some(v) = params.top_p {
        parts.push(format!("top_p={v}"));
    }
    if let Some(v) = params.top_k {
        parts.push(format!("top_k={v}"));
    }
    if let Some(v) = params.frequency_penalty {
        parts.push(format!("freq_penalty={v}"));
    }
    if let Some(v) = params.presence_penalty {
        parts.push(format!("pres_penalty={v}"));
    }
    if parts.is_empty() {
        "default_parameters: (no data)".to_string()
    } else {
        format!("default_parameters: {}", parts.join(", "))
    }
}
