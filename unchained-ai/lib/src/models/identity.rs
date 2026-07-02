//! Model wire-id identity grammar.
//!
//! Wire ids decompose predictably:
//!
//! ```text
//! offering  = [source /] vendor / model-id
//! model-id  = family + version [+ variants] [+ size] [+ date-pin] [+ ":" serving]
//! ```
//!
//! Parsing instead of storing opaque strings is what makes three things
//! queryable: cross-source identity (the same model offered by an aggregator
//! and a direct API groups together), rolling-alias resolution (`kimi-latest`
//! is a family selector, not a model), and "the latest `claude-sonnet` is …?"
//! via [`latest_in_family`].
//!
//! Design and empirical validation (99.9% family inference over the generated
//! catalog): `claudine/features/2026-07-02-provider-metadata/spec.md` and the
//! `spike-model-identity` findings next to it.

use std::cmp::Ordering;

/// Tokens that mark a variant axis (speed, delivery, capability surface).
///
/// Variants are distinct catalog entries linked by shared family+version —
/// `claude-opus-4.8-fast` is not the same offering as `claude-opus-4.8`.
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
    "live",
];

/// Vendor spelling drifts across sources; canonicalize to one key so family
/// grouping does not fragment (`z-ai` vs `zai`, `x-ai` vs `xai`, …).
const VENDOR_ALIASES: &[(&str, &str)] = &[
    ("z-ai", "zai"),
    ("x-ai", "xai"),
    ("mistralai", "mistral"),
    ("meta-llama", "meta"),
    // The direct source is named for the product; the vendor is Google.
    ("gemini", "google"),
];

/// Parsed identity of one wire model id.
///
/// Produced by [`ModelIdentity::parse`], which is infallible: tokens that
/// match no other axis land in `family`, so the worst case for an exotic id
/// is an over-long family, never an error.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModelIdentity {
    /// First path segment: the offering source (direct provider, aggregator,
    /// subscription plan, or local runner).
    pub source: String,
    /// Canonicalized model vendor (see [`VENDOR_ALIASES`]); equals `source`
    /// for two-segment direct-provider ids.
    pub vendor: String,
    /// Product line joined from the non-version tokens (`claude-sonnet`,
    /// `kimi-k`, `gpt-mini`).
    pub family: String,
    /// Numeric version segments for ordering (`4.5` → `[4, 5]`).
    pub version: Vec<u32>,
    /// Version as written, dot-joined (`"4.5"`).
    pub version_raw: String,
    /// Release date pin as written: `YYYYMMDD`, `YYMM`, `YYYY-MM`, or
    /// `YYYY-MM-DD`.
    pub date_pin: String,
    /// Parameter-size tokens (`70b`, `a3b`, `8x7b`, local `:27b` tags).
    pub size: Vec<String>,
    /// Variant-axis tokens (see [`VARIANT_VOCAB`]).
    pub variants: Vec<String>,
    /// Post-`:` serving tags (`free`, `nitro`, `thinking`).
    pub serving: Vec<String>,
    /// True when the id is a rolling alias (`-latest`) that re-points over
    /// time; excluded from [`latest_in_family`] because it names no release.
    pub rolling: bool,
}

impl ModelIdentity {
    /// Parses a wire id into its identity components.
    ///
    /// ## Examples
    ///
    /// ```
    /// use unchained_ai::models::identity::ModelIdentity;
    ///
    /// let id = ModelIdentity::parse("openrouter/anthropic/claude-sonnet-4.5");
    /// assert_eq!(id.source, "openrouter");
    /// assert_eq!(id.vendor, "anthropic");
    /// assert_eq!(id.family, "claude-sonnet");
    /// assert_eq!(id.version, vec![4, 5]);
    /// ```
    pub fn parse(wire: &str) -> Self {
        let mut id = Self::default();
        let segs: Vec<&str> = wire.split('/').collect();
        let (source, vendor, model) = match segs.len() {
            3 => (segs[0], segs[1], segs[2]),
            2 => (segs[0], segs[0], segs[1]),
            _ => (segs[0], segs[0], *segs.last().unwrap_or(&"")),
        };
        id.source = source.to_string();
        id.vendor = VENDOR_ALIASES
            .iter()
            .find(|(from, _)| *from == vendor)
            .map(|(_, to)| *to)
            .unwrap_or(vendor)
            .to_string();

        let mut model = model.to_lowercase();
        if let Some((head, tag)) = model.split_once(':') {
            let tag = tag.to_string();
            if is_size_token(&tag) {
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
            if is_yyyymmdd(t) || is_yymm(t) {
                id.date_pin = t.clone();
                i += 1;
                continue;
            }
            // `MM-YYYY` pair (Gemini preview pins like `09-2025`)
            if t.len() == 2
                && all_digits(t)
                && i + 1 < tokens.len()
                && tokens[i + 1].len() == 4
                && tokens[i + 1].starts_with("20")
                && all_digits(&tokens[i + 1])
            {
                id.date_pin = format!("{}-{}", tokens[i + 1], t);
                i += 2;
                continue;
            }
            // `YYYY-MM-DD` triple (OpenAI pins like `2024-11-20`)
            if t.len() == 4
                && t.starts_with("20")
                && all_digits(t)
                && i + 2 < tokens.len()
                && tokens[i + 1].len() == 2
                && all_digits(&tokens[i + 1])
                && tokens[i + 2].len() == 2
                && all_digits(&tokens[i + 2])
            {
                id.date_pin = format!("{}-{}-{}", t, tokens[i + 1], tokens[i + 2]);
                i += 3;
                continue;
            }
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
            if is_pure_version(t) || is_version_serial(t) {
                version_raw.push(t.clone());
                i += 1;
                continue;
            }
            // OpenAI's fused generations (`4o`, `o3`) are family literals: the
            // digit is not an orderable version across the o-/gpt- lines.
            if t == "4o" || t == "o1" || t == "o3" || t == "o4" {
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
        id.version = id
            .version_raw
            .split(['.', '-'])
            .filter_map(|p| p.parse::<u32>().ok())
            .collect();
        id
    }

    /// Returns `vendor/family`, the grouping key for cross-source identity.
    pub fn family_key(&self) -> String {
        format!("{}/{}", self.vendor, self.family)
    }

    /// Returns the parsed family, or `None` when nothing survived tokenizing
    /// (meta-routers like `openrouter/openrouter/auto`).
    pub fn family_or_none(&self) -> Option<&str> {
        if self.family.is_empty() {
            None
        } else {
            Some(&self.family)
        }
    }

    /// True when `query` names this identity's family, either bare
    /// (`"claude-sonnet"`) or vendor-scoped (`"anthropic/claude-sonnet"`).
    pub fn matches_family(&self, query: &str) -> bool {
        query == self.family || query == self.family_key()
    }

    /// Release ordering within a family: version segments first, date pin as
    /// tiebreaker. Only meaningful between members of the same family.
    pub fn release_order(&self, other: &Self) -> Ordering {
        self.version
            .cmp(&other.version)
            .then_with(|| self.date_pin.cmp(&other.date_pin))
    }
}

/// Returns the wire id of the newest release in `family` among `ids`.
///
/// Rolling aliases are excluded (they name no release). `family` may be bare
/// or vendor-scoped. Returns `None` when no id matches.
///
/// ## Examples
///
/// ```
/// use unchained_ai::models::identity::latest_in_family;
///
/// let ids = [
///     "anthropic/claude-sonnet-4-5-20250929",
///     "openrouter/anthropic/claude-sonnet-4.6",
///     "anthropic/claude-haiku-4-5-20251001",
/// ];
/// let latest = latest_in_family(ids.iter().copied(), "claude-sonnet");
/// assert_eq!(latest, Some("openrouter/anthropic/claude-sonnet-4.6"));
/// ```
pub fn latest_in_family<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    family: &str,
) -> Option<&'a str> {
    ids.into_iter()
        .map(|wire| (wire, ModelIdentity::parse(wire)))
        .filter(|(_, id)| !id.rolling && id.matches_family(family))
        .max_by(|(_, a), (_, b)| a.release_order(b))
        .map(|(wire, _)| wire)
}

fn all_digits(t: &str) -> bool {
    !t.is_empty() && t.chars().all(|c| c.is_ascii_digit())
}

fn is_size_token(t: &str) -> bool {
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
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return false;
    }
    if t.contains('.') {
        return true;
    }
    // Bare digit runs of 3 are serials, 4 are YYMM dates — neither is a
    // dotted version.
    t.len() <= 2
}

/// Zero-padded serial versions (`gemini-embedding-001`).
fn is_version_serial(t: &str) -> bool {
    t.len() == 3 && t.starts_with('0') && all_digits(t)
}

fn is_yymm(t: &str) -> bool {
    // Year-windowed heuristic: widen as the window ages.
    t.len() == 4 && all_digits(t) && matches!(&t[..2], "23" | "24" | "25" | "26" | "27" | "28")
}

fn is_yyyymmdd(t: &str) -> bool {
    t.len() == 8 && all_digits(t) && t.starts_with("20")
}

/// Alpha stem fused with a trailing version (`qwen3`, `k2.7`, `m2.5`).
fn split_fused(t: &str) -> Option<(String, String)> {
    let idx = t.find(|c: char| c.is_ascii_digit())?;
    if idx == 0 {
        return None;
    }
    let (stem, ver) = t.split_at(idx);
    if !ver.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    Some((stem.to_string(), ver.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_and_aggregator_forms_share_identity() {
        let direct = ModelIdentity::parse("anthropic/claude-sonnet-4-5-20250929");
        let routed = ModelIdentity::parse("openrouter/anthropic/claude-sonnet-4.5");
        assert_eq!(direct.family_key(), "anthropic/claude-sonnet");
        assert_eq!(direct.family_key(), routed.family_key());
        assert_eq!(direct.version, routed.version);
        assert_eq!(direct.date_pin, "20250929");
        assert_eq!(routed.date_pin, "");
    }

    #[test]
    fn era_dependent_token_order_unifies() {
        let old_era = ModelIdentity::parse("openrouter/anthropic/claude-3.5-haiku");
        let new_era = ModelIdentity::parse("anthropic/claude-haiku-4-5-20251001");
        assert_eq!(old_era.family, "claude-haiku");
        assert_eq!(new_era.family, "claude-haiku");
        assert_eq!(old_era.version, vec![3, 5]);
        assert_eq!(new_era.version, vec![4, 5]);
    }

    #[test]
    fn fused_stem_versions_parse() {
        let kimi = ModelIdentity::parse("moonshotai/kimi-k2.7-code-highspeed");
        assert_eq!(kimi.family, "kimi-k-code");
        assert_eq!(kimi.version, vec![2, 7]);
        assert_eq!(kimi.variants, vec!["highspeed"]);

        let qwen = ModelIdentity::parse("openrouter/qwen/qwen3-coder-30b-a3b-instruct");
        assert_eq!(qwen.family, "qwen-coder");
        assert_eq!(qwen.version, vec![3]);
        assert_eq!(qwen.size, vec!["30b", "a3b"]);
        assert_eq!(qwen.variants, vec!["instruct"]);
    }

    #[test]
    fn date_pin_forms() {
        assert_eq!(ModelIdentity::parse("mistral/codestral-2508").date_pin, "2508");
        assert_eq!(
            ModelIdentity::parse("openrouter/openai/gpt-4o-2024-11-20").date_pin,
            "2024-11-20"
        );
        assert_eq!(
            ModelIdentity::parse("gemini/gemini-2.5-flash-native-audio-preview-09-2025").date_pin,
            "2025-09"
        );
    }

    #[test]
    fn vendor_aliases_canonicalize() {
        assert_eq!(ModelIdentity::parse("openrouter/z-ai/glm-4.6").vendor, "zai");
        assert_eq!(ModelIdentity::parse("zai/glm-4.6").vendor, "zai");
        assert_eq!(ModelIdentity::parse("gemini/gemini-2.5-pro").vendor, "google");
    }

    #[test]
    fn serving_and_local_size_tags() {
        let free = ModelIdentity::parse("openrouter/qwen/qwen3-coder:free");
        assert_eq!(free.serving, vec!["free"]);

        let local = ModelIdentity::parse("ollama/gemma3:27b");
        assert_eq!(local.family, "gemma");
        assert_eq!(local.version, vec![3]);
        assert_eq!(local.size, vec!["27b"]);
    }

    #[test]
    fn rolling_alias_flagged_and_excluded_from_latest() {
        let rolling = ModelIdentity::parse("moonshotai/kimi-latest");
        assert!(rolling.rolling);

        let ids = ["moonshotai/kimi-latest", "moonshotai/kimi-k2.6"];
        assert_eq!(
            latest_in_family(ids, "kimi-k"),
            Some("moonshotai/kimi-k2.6")
        );
    }

    #[test]
    fn latest_prefers_version_then_date_pin() {
        let ids = [
            "anthropic/claude-opus-4-20250514",
            "anthropic/claude-opus-4-5-20251101",
            "openrouter/anthropic/claude-opus-4.7",
            "zenmux/anthropic/claude-opus-4.6",
        ];
        assert_eq!(
            latest_in_family(ids, "anthropic/claude-opus"),
            Some("openrouter/anthropic/claude-opus-4.7")
        );
    }

    #[test]
    fn meta_router_has_no_family() {
        let auto = ModelIdentity::parse("openrouter/openrouter/auto");
        assert_eq!(auto.family_or_none(), Some("auto"));
        // Only the vendor-duplicated segment marks a router; the family token
        // itself parses — callers decide what "auto" means.
        assert_eq!(auto.vendor, "openrouter");
    }

    #[test]
    fn version_serials_parse_as_versions() {
        let embed = ModelIdentity::parse("gemini/gemini-embedding-001");
        assert_eq!(embed.family, "gemini");
        assert_eq!(embed.variants, vec!["embedding"]);
        assert_eq!(embed.version, vec![1]);
    }
}
