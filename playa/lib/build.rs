use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let effects_rs_path = manifest_dir.join("src/effects.rs");
    let effects_dir = manifest_dir.join("../effects");

    println!("cargo:rerun-if-changed={}", effects_rs_path.display());
    if effects_dir.is_dir() {
        for entry in fs::read_dir(&effects_dir)? {
            let path = entry?.path();
            if path.is_file() {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }

    let source = fs::read_to_string(&effects_rs_path)?;
    let effects_rs_dir = effects_rs_path
        .parent()
        .ok_or("effects.rs missing parent directory")?;

    let const_paths = parse_const_paths(&source)?;
    let variant_names = parse_variant_names(&source);
    let variant_consts = parse_variant_consts(&source);

    let mut durations: Vec<(String, u32)> = Vec::with_capacity(variant_names.len());
    for (variant, name) in variant_names {
        let const_name = variant_consts
            .get(&variant)
            .ok_or_else(|| format!("Missing bytes const for {variant}"))?;
        let rel_path = const_paths
            .get(const_name)
            .ok_or_else(|| format!("Missing include path for {const_name}"))?;
        let abs_path = effects_rs_dir.join(rel_path);
        let duration_ms = compute_duration_ms(&abs_path).map_err(|error| {
            format!(
                "Failed to compute duration for {}: {error}",
                abs_path.display()
            )
        })?;
        durations.push((name, duration_ms));
    }

    durations.sort_by(|left, right| left.0.cmp(&right.0));

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let out_path = out_dir.join("effect_durations.rs");
    write_duration_manifest(&out_path, &durations)?;

    Ok(())
}

fn parse_const_paths(source: &str) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let mut map = HashMap::new();
    let mut pending_const: Option<String> = None;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("const ") && trimmed.contains("_BYTES") {
            let name = trimmed
                .trim_start_matches("const ")
                .split(':')
                .next()
                .ok_or("Invalid const line")?
                .trim()
                .to_string();
            pending_const = Some(name.clone());

            if let Some(path) = extract_include_path(trimmed) {
                map.insert(name, path);
                pending_const = None;
            }
            continue;
        }

        if trimmed.contains("include_bytes!") {
            if let Some(name) = pending_const.take() {
                if let Some(path) = extract_include_path(trimmed) {
                    map.insert(name, path);
                }
            }
        }
    }

    Ok(map)
}

fn parse_variant_names(source: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut in_name_fn = false;
    let mut brace_depth: i32 = 0;
    let mut saw_open = false;

    for line in source.lines() {
        let trimmed = line.trim();

        if !in_name_fn {
            if trimmed.starts_with("pub fn name(") {
                in_name_fn = true;
            } else {
                continue;
            }
        }

        if trimmed.contains("Self::") && trimmed.contains("=>") && trimmed.contains('"') {
            if let (Some(variant), Some(name)) =
                (extract_variant(trimmed), extract_string_literal(trimmed))
            {
                map.insert(variant, name);
            }
        }

        let opens = trimmed.chars().filter(|ch| *ch == '{').count() as i32;
        let closes = trimmed.chars().filter(|ch| *ch == '}').count() as i32;
        if opens > 0 {
            saw_open = true;
        }
        if saw_open {
            brace_depth += opens;
            brace_depth -= closes;
            if brace_depth == 0 {
                break;
            }
        }
    }

    map
}

fn parse_variant_consts(source: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.contains("Self::") || !trimmed.contains("=>") || !trimmed.contains("_BYTES") {
            continue;
        }

        let Some(variant) = extract_variant(trimmed) else {
            continue;
        };
        let Some(const_name) = extract_const_name(trimmed) else {
            continue;
        };
        map.insert(variant, const_name);
    }
    map
}

fn extract_include_path(line: &str) -> Option<String> {
    let needle = "include_bytes!(\"";
    let start = line.find(needle)? + needle.len();
    let end = line[start..].find("\")")? + start;
    Some(line[start..end].to_string())
}

fn extract_variant(line: &str) -> Option<String> {
    let start = line.find("Self::")? + "Self::".len();
    let rest = &line[start..];
    let end = rest
        .find(|ch: char| ch == ' ' || ch == '=' || ch == ',')
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

fn extract_string_literal(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let end = line[start..].find('"')? + start;
    Some(line[start..end].to_string())
}

fn extract_const_name(line: &str) -> Option<String> {
    let arrow = line.find("=>")? + 2;
    let rest = line[arrow..].trim();
    let end = rest
        .find(|ch: char| ch == ',' || ch == ' ' || ch == '\t')
        .unwrap_or(rest.len());
    let name = rest[..end].trim();
    if name.ends_with("_BYTES") {
        Some(name.to_string())
    } else {
        None
    }
}

fn compute_duration_ms(path: &Path) -> Result<u32, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|value| value.to_str()) {
        hint.with_extension(ext);
    }

    let probed = get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let mut format = probed.format;
    let (track_id, sample_rate, mut decoder) = {
        let track = format
            .default_track()
            .ok_or("No default track in audio file")?;
        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or("Missing sample rate")? as u64;
        let decoder = get_codecs().make(&track.codec_params, &DecoderOptions::default())?;
        (track.id, sample_rate, decoder)
    };
    let mut total_frames: u64 = 0;

    loop {
        match format.next_packet() {
            Ok(packet) => {
                if packet.track_id() != track_id {
                    continue;
                }
                match decoder.decode(&packet) {
                    Ok(audio) => {
                        total_frames += audio.frames() as u64;
                    }
                    Err(SymphoniaError::DecodeError(_)) => continue,
                    Err(error) => return Err(Box::new(error)),
                }
            }
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                return Err("Decoder reset required".into());
            }
            Err(error) => return Err(Box::new(error)),
        }
    }

    let duration_ms = ((total_frames as f64 / sample_rate as f64) * 1000.0).round() as u64;
    Ok(duration_ms.min(u32::MAX as u64) as u32)
}

fn write_duration_manifest(
    out_path: &Path,
    durations: &[(String, u32)],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = fs::File::create(out_path)?;
    writeln!(file, "// @generated by playa build.rs")?;
    writeln!(file, "pub(crate) struct EffectDuration {{")?;
    writeln!(file, "    pub(crate) name: &'static str,")?;
    writeln!(file, "    pub(crate) duration_ms: u32,")?;
    writeln!(file, "}}")?;
    writeln!(file, "")?;
    writeln!(
        file,
        "pub(crate) const EFFECT_DURATIONS: &[EffectDuration] = &["
    )?;
    for (name, duration_ms) in durations {
        writeln!(
            file,
            "    EffectDuration {{ name: \"{}\", duration_ms: {} }},",
            name, duration_ms
        )?;
    }
    writeln!(file, "];")?;
    writeln!(file, "")?;
    writeln!(
        file,
        "pub(crate) fn duration_ms_for_name(name: &str) -> Option<u32> {{"
    )?;
    writeln!(
        file,
        "    EFFECT_DURATIONS.iter().find(|entry| entry.name == name).map(|entry| entry.duration_ms)"
    )?;
    writeln!(file, "}}")?;
    Ok(())
}
