#!/usr/bin/env python3
"""One-off script to regenerate metadata_generated.rs with full rich fields.

Fetches OpenRouter and Parsera public APIs, merges data, and generates
metadata_generated.rs using the same format as the updated Rust generator.
Extracts all known model IDs from existing provider enum files.
"""

import json
import re
import urllib.request
from pathlib import Path


def fetch_json(url):
    req = urllib.request.Request(url, headers={"User-Agent": "unchained-ai-gen/0.1.0"})
    with urllib.request.urlopen(req, timeout=60) as resp:
        return json.load(resp)


def parse_openrouter_pricing(p):
    if not p:
        return None
    pt = p.get("prompt")
    ct = p.get("completion")
    ws = p.get("web_search")
    icr = p.get("input_cache_read")
    if pt is None and ct is None and ws is None and icr is None:
        return None
    return {
        "prompt_per_token": float(pt) if pt is not None else None,
        "completion_per_token": float(ct) if ct is not None else None,
        "web_search_per_request": float(ws) if ws is not None else None,
        "input_cache_read_per_token": float(icr) if icr is not None else None,
    }


def parse_openrouter_modalities(arch):
    if not arch:
        return None
    inp = arch.get("input_modalities", [])
    out = arch.get("output_modalities", [])
    if not inp and not out:
        return None
    return {"input": inp, "output": out}


def parse_openrouter_default_parameters(dp):
    if not dp:
        return None
    fields = {
        "temperature": dp.get("temperature"),
        "top_p": dp.get("top_p"),
        "top_k": dp.get("top_k"),
        "frequency_penalty": dp.get("frequency_penalty"),
        "presence_penalty": dp.get("presence_penalty"),
    }
    if all(v is None for v in fields.values()):
        return None
    result = {}
    for k, v in fields.items():
        if v is None:
            result[k] = None
        elif k == "top_k":
            result[k] = int(v)
        else:
            result[k] = float(v)
    return result


def parse_openrouter(raw):
    """Parse OpenRouter /api/v1/models response into dict keyed by model ID."""
    entries = {}
    for m in raw.get("data", []):
        mid = m.get("id")
        if not mid:
            continue
        tp = m.get("top_provider", {})
        entries[mid] = {
            "display_name": m.get("name"),
            "description": m.get("description"),
            "context_window": m.get("context_length"),
            "max_output_tokens": tp.get("max_completion_tokens") if tp else None,
            "modalities": parse_openrouter_modalities(m.get("architecture")),
            "capabilities": [],
            "pricing": parse_openrouter_pricing(m.get("pricing")),
            "supported_parameters": m.get("supported_parameters"),
            "default_parameters": parse_openrouter_default_parameters(m.get("default_parameters")),
            "knowledge_cutoff": m.get("knowledge_cutoff") or None,
            "created": m.get("created"),
        }
    return entries


def parse_parsera(raw):
    """Parse Parsera specs into dict keyed by model ID."""
    entries = {}
    for m in raw:
        mid = m.get("id")
        if not mid:
            continue
        entries[mid] = {
            "display_name": m.get("name"),
            "family": m.get("family"),
            "context_window": m.get("context_window"),
            "max_output_tokens": m.get("max_output_tokens"),
            "modalities": m.get("modalities"),
            "capabilities": m.get("capabilities") or [],
        }
    return entries


def merge_metadata(native, parsera):
    """Merge native (provider) and Parsera data. Native wins for overlapping fields."""
    if native is None and parsera is None:
        return None
    if native is None:
        return {
            **{k: parsera.get(k) for k in [
                "display_name", "family", "context_window", "max_output_tokens",
                "modalities", "capabilities", "description", "pricing",
                "supported_parameters", "default_parameters", "knowledge_cutoff", "created"
            ]},
            "capabilities": parsera.get("capabilities") or [],
        }
    if parsera is None:
        return native

    return {
        "display_name": native.get("display_name") or parsera.get("display_name"),
        "family": native.get("family") or parsera.get("family"),
        "context_window": native.get("context_window") if native.get("context_window") is not None else parsera.get("context_window"),
        "max_output_tokens": native.get("max_output_tokens") if native.get("max_output_tokens") is not None else parsera.get("max_output_tokens"),
        "modalities": native.get("modalities") or parsera.get("modalities"),
        "capabilities": native.get("capabilities") if native.get("capabilities") else (parsera.get("capabilities") or []),
        "description": native.get("description"),
        "pricing": native.get("pricing"),
        "supported_parameters": native.get("supported_parameters"),
        "default_parameters": native.get("default_parameters"),
        "knowledge_cutoff": native.get("knowledge_cutoff"),
        "created": native.get("created"),
    }


def escape_string(s):
    return (
        s.replace("\\", "\\\\")
         .replace('"', '\\"')
         .replace("\n", "\\n")
         .replace("\r", "\\r")
         .replace("\t", "\\t")
    )


def format_option_f64(v):
    if v is None:
        return "None"
    return f"Some({v}_f64)"


def format_option_f32(v):
    if v is None:
        return "None"
    return f"Some({v}_f32)"


def format_option_u32(v):
    if v is None:
        return "None"
    return f"Some({int(v)})"


def format_option_string(v):
    if v is None:
        return "None"
    return f'Some("{escape_string(v)}".to_string())'


def format_modality(m):
    mapping = {
        "text": "Modality::Text",
        "image": "Modality::Image",
        "audio": "Modality::Audio",
        "video": "Modality::Video",
        "embeddings": "Modality::Embeddings",
        "embedding": "Modality::Embeddings",
    }
    return mapping.get(m.lower(), f"Modality::Text /* unknown: {m} */")


def generate_entry(model_id, meta):
    lines = [f'    m.insert("{model_id}", ModelMetadata {{']
    lines.append(f'        display_name: {format_option_string(meta.get("display_name"))},')
    lines.append(f'        family: {format_option_string(meta.get("family"))},')
    lines.append(f'        context_window: {format_option_u32(meta.get("context_window"))},')
    lines.append(f'        max_output_tokens: {format_option_u32(meta.get("max_output_tokens"))},')

    mods = meta.get("modalities")
    if mods:
        inp = ", ".join(format_modality(m) for m in mods.get("input", []))
        out = ", ".join(format_modality(m) for m in mods.get("output", []))
        lines.append("        modalities: Some(ModelModalities {")
        lines.append(f"            input: vec![{inp}],")
        lines.append(f"            output: vec![{out}],")
        lines.append("        }),")
    else:
        lines.append("        modalities: None,")

    caps = meta.get("capabilities") or []
    if caps:
        lines.append("        capabilities: vec![")
        for c in caps:
            lines.append(f'            "{escape_string(c)}".to_string(),')
        lines.append("        ],")
    else:
        lines.append("        capabilities: vec![],")

    lines.append(f'        description: {format_option_string(meta.get("description"))},')

    pricing = meta.get("pricing")
    if pricing:
        lines.append("        pricing: Some(ModelPricing {")
        lines.append(f'            prompt_per_token: {format_option_f64(pricing.get("prompt_per_token"))},')
        lines.append(f'            completion_per_token: {format_option_f64(pricing.get("completion_per_token"))},')
        lines.append(f'            web_search_per_request: {format_option_f64(pricing.get("web_search_per_request"))},')
        lines.append(f'            input_cache_read_per_token: {format_option_f64(pricing.get("input_cache_read_per_token"))},')
        lines.append("        }),")
    else:
        lines.append("        pricing: None,")

    sp = meta.get("supported_parameters")
    if sp is not None:
        if not sp:
            lines.append("        supported_parameters: Some(vec![]),")
        else:
            lines.append("        supported_parameters: Some(vec![")
            for p in sp:
                lines.append(f'            "{escape_string(p)}".to_string(),')
            lines.append("        ]),")
    else:
        lines.append("        supported_parameters: None,")

    dp = meta.get("default_parameters")
    if dp:
        lines.append("        default_parameters: Some(ModelDefaultParameters {")
        lines.append(f'            temperature: {format_option_f32(dp.get("temperature"))},')
        lines.append(f'            top_p: {format_option_f32(dp.get("top_p"))},')
        lines.append(f'            top_k: {format_option_u32(dp.get("top_k"))},')
        lines.append(f'            frequency_penalty: {format_option_f32(dp.get("frequency_penalty"))},')
        lines.append(f'            presence_penalty: {format_option_f32(dp.get("presence_penalty"))},')
        lines.append("        }),")
    else:
        lines.append("        default_parameters: None,")

    lines.append(f'        knowledge_cutoff: {format_option_string(meta.get("knowledge_cutoff"))},')
    lines.append(f'        created: {format_option_u32(meta.get("created"))},')
    lines.append("    });")
    return "\n".join(lines)


def extract_model_ids_from_enums(models_dir):
    """Extract all model IDs from provider enum files."""
    ids = set()
    for f in models_dir.glob("*.rs"):
        if f.name in ("mod.rs", "metadata_generated.rs", "metadata_openrouter_generated.rs", "README.md"):
            continue
        if f.is_dir():
            continue
        text = f.read_text()
        for m in re.finditer(r'/// Model: `([^`]+)`', text):
            ids.add(m.group(1))
    return ids


def main():
    repo_root = Path(__file__).resolve().parents[3]
    models_dir = repo_root / "unchained-ai/lib/src/rigging/providers/models"
    metadata_path = models_dir / "metadata_generated.rs"

    print("Fetching OpenRouter models...")
    or_raw = fetch_json("https://openrouter.ai/api/v1/models")
    or_data = parse_openrouter(or_raw)
    print(f"  -> {len(or_data)} OpenRouter models")

    print("Fetching Parsera specs...")
    parsera_raw = fetch_json("https://api.parsera.org/v1/llm-specs")
    parsera_data = parse_parsera(parsera_raw)
    print(f"  -> {len(parsera_data)} Parsera specs")

    print("Extracting model IDs from provider enum files...")
    model_ids = extract_model_ids_from_enums(models_dir)
    print(f"  -> {len(model_ids)} models from enums")

    # Also include any IDs from existing metadata file
    if metadata_path.exists():
        existing_text = metadata_path.read_text()
        for m in re.finditer(r'm\.insert\("([^"]+)"', existing_text):
            model_ids.add(m.group(1))

    # Build merged metadata for all model IDs
    entries = {}
    for mid in sorted(model_ids):
        native = or_data.get(mid)
        parsera = parsera_data.get(mid)
        merged = merge_metadata(native, parsera)
        if merged:
            entries[mid] = merged

    print(f"Merged metadata for {len(entries)} models")

    # Generate code
    lines = [
        "//! Generated model metadata lookup table.",
        "//!",
        "//! This file is auto-generated by `gen-models`. Do not edit manually.",
        "//! Re-run `gen-models` to regenerate.",
        "",
        "#![allow(deprecated)]",
        "",
        "use std::collections::HashMap;",
        "use std::sync::LazyLock;",
        "",
        "#[allow(unused_imports)]",
        'use crate::models::model_metadata::{ModelMetadata, ModelModalities, Modality};',
        "#[allow(unused_imports)]",
        "use crate::models::model_pricing::ModelPricing;",
        "#[allow(unused_imports)]",
        "use crate::models::model_default_parameters::ModelDefaultParameters;",
        "",
        "/// Static lookup table mapping model IDs to their metadata.",
        f'pub static MODEL_METADATA: LazyLock<HashMap<&\'static str, ModelMetadata>> = LazyLock::new(|| {{',
        f'    let mut m = HashMap::with_capacity({len(entries)});',
    ]

    for mid in sorted(entries.keys()):
        lines.append(generate_entry(mid, entries[mid]))

    lines.append("    m")
    lines.append("});")
    lines.append("")

    code = "\n".join(lines) + "\n"
    metadata_path.write_text(code)
    print(f"Wrote {metadata_path}")


if __name__ == "__main__":
    main()
