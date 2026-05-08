use std::rc::Rc;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};
use serde_json::{Map, Value, json};
use unchained_ai::models::model_default_parameters::ModelDefaultParameters;
use unchained_ai::models::model_metadata::{ModelModalities, ProviderModelMetadata};
use unchained_ai::models::model_pricing::ModelPricing;
use unchained_ai::rigging::providers::models::ProviderModel;

/// Run the `model` subcommand.
pub async fn run(model_str: String, json: bool) -> Result<()> {
    let model = ProviderModel::parse_wire_id(&model_str).map_err(|e| eyre!("{e}"))?;

    if json {
        render_json(&model)?;
    } else {
        render_terminal(&model);
    }

    Ok(())
}

fn render_json(model: &ProviderModel) -> Result<()> {
    let mut map = Map::new();

    map.insert("model".into(), Value::String(model.wire_id()));
    map.insert(
        "provider".into(),
        Value::String(model.provider().display_name().to_string()),
    );
    map.insert(
        "model_id".into(),
        Value::String(model.model_id().to_string()),
    );

    if let Some(meta) = model.metadata() {
        map.insert("metadata".into(), metadata_to_json(meta));
    } else {
        map.insert("metadata".into(), Value::Null);
    }

    println!("{}", serde_json::to_string_pretty(&Value::Object(map))?);
    Ok(())
}

fn metadata_to_json(meta: &ProviderModelMetadata) -> Value {
    let mut map = Map::new();

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

fn render_terminal(model: &ProviderModel) {
    let term = Terminal::default();

    // Header
    let header = Prose::new(format!(
        "<b>{}</b> <dim>({})</dim>",
        model.wire_id(),
        model.provider().display_name(),
    ));
    println!("{}", header.render(&term));
    println!();

    let meta = match model.metadata() {
        Some(m) => m,
        None => {
            let no_data = Prose::new("<dim>No metadata available for this model.</dim>");
            println!("{}", no_data.render(&term));
            return;
        }
    };

    // Build sections as nested lists
    let mut sections: Vec<RenderableContent> = Vec::new();

    // Identity section
    let mut identity_items = Vec::new();
    if let Some(name) = &meta.display_name {
        identity_items.push(format!("display_name: {name}"));
    }
    if let Some(family) = &meta.family {
        identity_items.push(format!("family: {family}"));
    }
    if let Some(created) = meta.created {
        identity_items.push(format!("created: {created} (unix timestamp)"));
    }
    if let Some(cutoff) = &meta.knowledge_cutoff {
        identity_items.push(format!("knowledge_cutoff: {cutoff}"));
    }
    if !identity_items.is_empty() {
        sections.push(section_header("Identity"));
        sections.push(RenderableContent::Component(Rc::new(UnorderedList::new(
            identity_items,
        ))));
    }

    // Capacity section
    let mut capacity_items = Vec::new();
    if let Some(ctx) = meta.context_window {
        capacity_items.push(format!("context_window: {ctx} tokens"));
    }
    if let Some(max_out) = meta.max_output_tokens {
        capacity_items.push(format!("max_output_tokens: {max_out} tokens"));
    }
    if !capacity_items.is_empty() {
        sections.push(section_header("Capacity"));
        sections.push(RenderableContent::Component(Rc::new(UnorderedList::new(
            capacity_items,
        ))));
    }

    // Modalities section
    if let Some(modalities) = &meta.modalities {
        let mut modality_items = Vec::new();
        if !modalities.input.is_empty() {
            let inputs: Vec<String> = modalities.input.iter().map(|m| m.to_string()).collect();
            modality_items.push(format!("input: {}", inputs.join(", ")));
        }
        if !modalities.output.is_empty() {
            let outputs: Vec<String> = modalities.output.iter().map(|m| m.to_string()).collect();
            modality_items.push(format!("output: {}", outputs.join(", ")));
        }
        if !modality_items.is_empty() {
            sections.push(section_header("Modalities"));
            sections.push(RenderableContent::Component(Rc::new(UnorderedList::new(
                modality_items,
            ))));
        }
    }

    // Capabilities section
    if !meta.capabilities.is_empty() {
        sections.push(section_header("Capabilities"));
        sections.push(RenderableContent::Component(Rc::new(UnorderedList::new(
            meta.capabilities.clone(),
        ))));
    }

    // Pricing section
    if let Some(pricing) = &meta.pricing {
        let mut pricing_items = Vec::new();
        if let Some(v) = pricing.prompt_per_token {
            pricing_items.push(format!("prompt: ${v:.8}/token"));
        }
        if let Some(v) = pricing.completion_per_token {
            pricing_items.push(format!("completion: ${v:.8}/token"));
        }
        if let Some(v) = pricing.web_search_per_request {
            pricing_items.push(format!("web_search: ${v:.4}/request"));
        }
        if let Some(v) = pricing.input_cache_read_per_token {
            pricing_items.push(format!("input_cache_read: ${v:.8}/token"));
        }
        if !pricing_items.is_empty() {
            sections.push(section_header("Pricing"));
            sections.push(RenderableContent::Component(Rc::new(UnorderedList::new(
                pricing_items,
            ))));
        }
    }

    // Parameters section
    if let Some(params) = &meta.supported_parameters {
        let mut param_items = Vec::new();
        param_items.push(format!("supported: {}", params.join(", ")));
        if let Some(defaults) = &meta.default_parameters {
            let mut default_parts = Vec::new();
            if let Some(v) = defaults.temperature {
                default_parts.push(format!("temperature={v}"));
            }
            if let Some(v) = defaults.top_p {
                default_parts.push(format!("top_p={v}"));
            }
            if let Some(v) = defaults.top_k {
                default_parts.push(format!("top_k={v}"));
            }
            if let Some(v) = defaults.frequency_penalty {
                default_parts.push(format!("frequency_penalty={v}"));
            }
            if let Some(v) = defaults.presence_penalty {
                default_parts.push(format!("presence_penalty={v}"));
            }
            if !default_parts.is_empty() {
                param_items.push(format!("defaults: {}", default_parts.join(", ")));
            }
        }
        if !param_items.is_empty() {
            sections.push(section_header("Parameters"));
            sections.push(RenderableContent::Component(Rc::new(UnorderedList::new(
                param_items,
            ))));
        }
    }

    // Description section
    if let Some(desc) = &meta.description {
        sections.push(section_header("Description"));
        sections.push(RenderableContent::Component(Rc::new(Prose::new(
            desc.clone(),
        ))));
    }

    // Render all sections
    let list = UnorderedList::from(sections);
    print!("{}", list.render(&term));
}

fn section_header(title: &str) -> RenderableContent {
    RenderableContent::Component(Rc::new(Prose::new(format!("<b>{title}</b>"))))
}
