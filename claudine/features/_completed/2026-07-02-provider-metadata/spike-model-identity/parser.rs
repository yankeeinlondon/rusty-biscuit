//! Spike: model identity grammar prototype (2026-07-02).
//!
//! Parses wire model ids from unchained-ai's generated catalog into
//!   offering = [source /] vendor / model-id
//!   model-id = family + version [+ variants] [+ size] [+ date-pin] [+ serving]
//! and reports how well "family is inferable from the name" holds empirically.
//!
//! Standalone: `rustc -O parser.rs && ./parser model-ids.txt`

use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Debug, Clone, Default)]
struct Identity {
    source: String,
    vendor: String,
    family: String,
    version: Vec<u32>,
    version_raw: String,
    date_pin: String,
    size: Vec<String>,
    variants: Vec<String>,
    serving: Vec<String>,
    rolling: bool,
}

/// Tokens that mark a variant axis (speed, delivery, capability surface).
const VARIANT_VOCAB: &[&str] = &[
    "fast",
    "highspeed",
    "turbo",
    "instruct",
    "chat",
    "it",
    "base",
    "preview",
    "exp",
    "experimental",
    "beta",
    "thinking",
    "vl",
    "vision",
    "audio",
    "native",
    "video",
    "image",
    "ocr",
    "distill",
    "distilled",
    "guard",
    "embedding",
    "embed",
    "moderation",
    "saba",
    "online",
    "tts",
];

/// Post-`:` serving tags (aggregator delivery tiers / local size tags).
const SERVING_VOCAB: &[&str] = &["free", "extended", "nitro", "floor", "exacto"];

fn is_size_token(t: &str) -> bool {
    // 70b, 1.5b, 235m, a3b, a22b, 8x7b, 16e, 30b (activated-param and MoE forms)
    let t = t.trim_start_matches('a');
    if let Some(stripped) = t.strip_suffix(|c| c == 'b' || c == 'm' || c == 'e') {
        if stripped.is_empty() {
            return false;
        }
        return stripped
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == 'x');
    }
    false
}

fn is_pure_version(t: &str) -> bool {
    // 4, 4.5, 2.0, 3.5 — pure numeric with optional dots (but not date-like 4-digit)
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return false;
    }
    if t.contains('.') {
        return true;
    }
    t.len() <= 2 // bare "4", "35" — 4-digit runs are dates (YYMM)
}

fn is_yymm(t: &str) -> bool {
    t.len() == 4
        && t.chars().all(|c| c.is_ascii_digit())
        && matches!(&t[..2], "23" | "24" | "25" | "26" | "27")
}

fn is_yyyymmdd(t: &str) -> bool {
    t.len() == 8 && t.chars().all(|c| c.is_ascii_digit()) && (t.starts_with("20"))
}

/// alpha stem fused with trailing version: qwen3, k2.5, m2.5, o3, v3, r1, phi4, gemma3
fn split_fused(t: &str) -> Option<(String, String)> {
    let idx = t.find(|c: char| c.is_ascii_digit())?;
    if idx == 0 {
        return None;
    }
    let (stem, ver) = t.split_at(idx);
    if !ver.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None; // e.g. "4o", "16e" — not stem+version
    }
    Some((stem.to_string(), ver.to_string()))
}

fn parse_version_nums(raw: &str) -> Vec<u32> {
    raw.split(['.', '-'])
        .filter_map(|p| p.parse::<u32>().ok())
        .collect()
}

fn parse(wire: &str) -> Identity {
    let mut id = Identity::default();
    let segs: Vec<&str> = wire.split('/').collect();
    let (source, vendor, model) = match segs.len() {
        3 => (segs[0], segs[1], segs[2]),
        2 => (segs[0], segs[0], segs[1]),
        _ => (segs[0], segs[0], *segs.last().unwrap()),
    };
    id.source = source.to_string();
    // vendor spelling drifts across sources — canonicalize
    id.vendor = match vendor {
        "z-ai" => "zai",
        "x-ai" => "xai",
        "mistralai" => "mistral",
        "meta-llama" => "meta",
        "gemini" => "google", // direct source name is the product, vendor is google
        v => v,
    }
    .to_string();

    // serving / local size tag after ':'
    let mut model = model.to_lowercase();
    if let Some((head, tag)) = model.split_once(':') {
        let tag = tag.to_string();
        if SERVING_VOCAB.contains(&tag.as_str()) {
            id.serving.push(tag);
        } else if is_size_token(&tag) {
            id.size.push(tag);
        } else {
            id.serving.push(tag);
        }
        model = head.to_string();
    }

    let tokens: Vec<String> = model.split('-').map(str::to_string).collect();
    let mut family_tokens: Vec<String> = Vec::new();
    let mut version_raw: Vec<String> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let t = &tokens[i];
        // date pin forms
        if is_yyyymmdd(t) || is_yymm(t) {
            id.date_pin = t.clone();
            i += 1;
            continue;
        }
        // "09-2025" month-year pair
        if t.len() == 2
            && t.chars().all(|c| c.is_ascii_digit())
            && i + 1 < tokens.len()
            && tokens[i + 1].len() == 4
            && tokens[i + 1].starts_with("20")
            && tokens[i + 1].chars().all(|c| c.is_ascii_digit())
        {
            id.date_pin = format!("{}-{}", tokens[i + 1], t);
            i += 2;
            continue;
        }
        // "2024-11-20" year-month-day triple
        if t.len() == 4
            && t.starts_with("20")
            && t.chars().all(|c| c.is_ascii_digit())
            && i + 2 < tokens.len()
            && tokens[i + 1].len() == 2
            && tokens[i + 1].chars().all(|c| c.is_ascii_digit())
            && tokens[i + 2].len() == 2
            && tokens[i + 2].chars().all(|c| c.is_ascii_digit())
        {
            id.date_pin = format!("{}-{}-{}", t, tokens[i + 1], tokens[i + 2]);
            i += 3;
            continue;
        }
        // full date 2025 01 15 style is covered by yyyymmdd; bare year+ver ambiguity skipped
        if is_size_token(t) {
            id.size.push(t.clone());
            i += 1;
            continue;
        }
        if t == "latest" {
            id.rolling = true;
            i += 1;
            continue;
        }
        if VARIANT_VOCAB.contains(&t.as_str()) {
            id.variants.push(t.clone());
            i += 1;
            continue;
        }
        if is_pure_version(t) {
            version_raw.push(t.clone());
            i += 1;
            continue;
        }
        if t == "4o" || t == "o1" || t == "o3" || t == "o4" {
            // openai oddities: treat as family-fused version literals
            family_tokens.push(t.clone());
            i += 1;
            continue;
        }
        if let Some((stem, ver)) = split_fused(t) {
            family_tokens.push(stem);
            version_raw.push(ver);
            i += 1;
            continue;
        }
        family_tokens.push(t.clone());
        i += 1;
    }

    id.family = family_tokens.join("-");
    id.version_raw = version_raw.join(".");
    id.version = parse_version_nums(&id.version_raw);
    id
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: parser <ids.txt>");
    let data = std::fs::read_to_string(&path).expect("read ids");
    let ids: Vec<&str> = data.lines().filter(|l| !l.trim().is_empty()).collect();

    let mut parsed: Vec<(String, Identity)> = Vec::new();
    let (mut with_family, mut with_version, mut date_only, mut bare) = (0u32, 0u32, 0u32, 0u32);
    let mut oddities: Vec<String> = Vec::new();

    for wire in &ids {
        let idy = parse(wire);
        if !idy.family.is_empty() {
            with_family += 1;
        }
        if !idy.version.is_empty() {
            with_version += 1;
        } else if !idy.date_pin.is_empty() {
            date_only += 1;
        } else if !idy.rolling {
            bare += 1;
            if oddities.len() < 40 {
                oddities.push(format!("{wire}  -> family='{}' (no version/date)", idy.family));
            }
        }
        parsed.push((wire.to_string(), idy));
    }

    // family histogram keyed by vendor+family
    let mut fam: BTreeMap<String, Vec<&(String, Identity)>> = BTreeMap::new();
    for p in &parsed {
        fam.entry(format!("{}/{}", p.1.vendor, p.1.family))
            .or_default()
            .push(p);
    }

    // cross-source identity: same vendor+family+version+variants+size from >1 source
    let mut xsource: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (wire, idy) in &parsed {
        let mut v = idy.variants.clone();
        v.sort();
        let key = format!(
            "{}|{}|{}|{}|{}",
            idy.vendor,
            idy.family,
            idy.version
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join("."),
            v.join("+"),
            idy.size.join("+")
        );
        xsource.entry(key).or_default().push(wire.clone());
    }
    let multi: Vec<(&String, &Vec<String>)> = xsource
        .iter()
        .filter(|(_, v)| {
            let mut sources: Vec<&str> = v.iter().map(|w| w.split('/').next().unwrap()).collect();
            sources.sort();
            sources.dedup();
            sources.len() > 1
        })
        .collect();

    let mut out = String::new();
    let total = ids.len();
    let _ = writeln!(out, "total ids: {total}");
    let _ = writeln!(
        out,
        "family inferred: {with_family} ({:.1}%)",
        100.0 * with_family as f64 / total as f64
    );
    let vp = 100.0 * with_version as f64 / total as f64;
    let _ = writeln!(
        out,
        "version parsed: {with_version} ({vp:.1}%) | date-pin only: {date_only} | bare (no version/date/rolling): {bare}"
    );
    let _ = writeln!(out, "distinct vendor/family keys: {}", fam.len());
    let _ = writeln!(out, "cross-source identity groups (same model, >1 source): {}", multi.len());

    let _ = writeln!(out, "\n== latest-of-family demo ==");
    for probe in ["anthropic/claude-sonnet", "anthropic/claude-opus", "moonshotai/kimi-k", "openai/gpt", "google/gemini-flash", "zai/glm"] {
        let mut members: Vec<&(String, Identity)> = fam
            .iter()
            .filter(|(k, _)| k.starts_with(probe))
            .flat_map(|(_, v)| v.iter().copied())
            .filter(|(_, i)| !i.rolling)
            .collect();
        members.sort_by(|a, b| {
            a.1.version
                .cmp(&b.1.version)
                .then(a.1.date_pin.cmp(&b.1.date_pin))
        });
        if let Some((wire, idy)) = members.last() {
            let _ = writeln!(
                out,
                "latest '{probe}*' -> {wire}  (v{:?} pin='{}' [{} members])",
                idy.version, idy.date_pin, members.len()
            );
        }
    }

    let _ = writeln!(out, "\n== top families ==");
    let mut fams: Vec<(&String, usize)> = fam.iter().map(|(k, v)| (k, v.len())).collect();
    fams.sort_by(|a, b| b.1.cmp(&a.1));
    for (k, n) in fams.iter().take(20) {
        let _ = writeln!(out, "{n:4}  {k}");
    }

    let _ = writeln!(out, "\n== sample cross-source groups ==");
    for (k, v) in multi.iter().take(12) {
        let _ = writeln!(out, "{k}\n      {}", v.join("\n      "));
    }

    let _ = writeln!(out, "\n== bare oddities (no version, date, or rolling marker) ==");
    for o in &oddities {
        let _ = writeln!(out, "  {o}");
    }

    println!("{out}");
}
