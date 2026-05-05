use std::rc::Rc;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};
use serde_json::{Map, Value, json};
use unchained_ai::models::model_metadata::{ModelModalities, ProviderModelMetadata};
use unchained_ai::rigging::providers::Provider;
use unchained_ai::rigging::providers::models::{
    ProviderModel, anthropic::ProviderModelAnthropic, deepseek::ProviderModelDeepseek,
    gemini::ProviderModelGemini, groq::ProviderModelGroq, mistral::ProviderModelMistral,
    moonshotai::ProviderModelMoonshotAi, openai::ProviderModelOpenAi,
    openrouter::ProviderModelOpenRouter, xai::ProviderModelXai, zai::ProviderModelZai,
    zenmux::ProviderModelZenMux,
};

/// Run the `models` subcommand.
pub async fn run(provider_filter: Option<String>, json: bool, verbose: bool) -> Result<()> {
    let filter = match provider_filter.as_deref() {
        Some(name) => Some(parse_provider(name)?),
        None => None,
    };

    let groups = collect_models(filter);

    if json {
        render_json(&groups)?;
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

fn provider_slug(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => "anthropic",
        Provider::Deepseek => "deepseek",
        Provider::Gemini => "gemini",
        Provider::Groq => "groq",
        Provider::HuggingFace => "huggingface",
        Provider::Mistral => "mistral",
        Provider::MoonshotAi => "moonshotai",
        Provider::Ollama => "ollama",
        Provider::OpenAi => "openai",
        Provider::OpenRouter => "openrouter",
        Provider::Xai => "xai",
        Provider::Zai => "zai",
        Provider::ZenMux => "zenmux",
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

    for (provider, models) in groups {
        let slug = provider_slug(*provider);
        for model in models {
            let wire_id = format!("{}/{}", slug, model.model_id());
            let attributes = metadata_to_json(model.metadata());
            entries.push(json!({
                "model": wire_id,
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

    Value::Object(map)
}

fn modalities_to_json(modalities: &ModelModalities) -> Value {
    json!({
        "input": modalities.input.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
        "output": modalities.output.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
    })
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

fn build_provider_list(models: &[ProviderModel], verbose: bool) -> UnorderedList {
    let mut items: Vec<RenderableContent> = Vec::with_capacity(models.len() * 2);

    for model in models {
        items.push(RenderableContent::String(model.model_id().to_string()));

        if verbose
            && let Some(meta) = model.metadata()
            && let Some(children) = build_metadata_list(meta)
        {
            items.push(RenderableContent::Component(Rc::new(children)));
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

    if entries.is_empty() {
        None
    } else {
        Some(UnorderedList::new(entries))
    }
}
