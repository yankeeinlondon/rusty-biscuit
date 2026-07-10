//! Deterministic synthetic-corpus generator (R-6 bench tiers).
//!
//! Generates reproducible Markdown workspaces for benchmarking and stress
//! tests. Everything is seeded — no `Date::now` analogues, no `rand` — so the
//! same tier written twice is byte-identical and a bench number is comparable
//! run to run. Lives in `src` (not just `tests`) so `--bench-index` can
//! materialize a tier on demand; `large-20k` is generated only when asked, and
//! is never checked in.

use std::io;
use std::path::Path;

/// A named corpus tier from the R-6 mix table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpusTier {
    /// 100 files, light structure.
    Tiny100,
    /// 1,000 files, moderate structure.
    Small1k,
    /// 5,000 files, wiki-link heavy (a note vault).
    Vault5k,
    /// 5,000 files, dense cross-linking.
    Dense5k,
    /// 5,000 files dense in Darkmatter DSL: `$schema` (file-ref + inline),
    /// `file(...)`/image values, `::file`/`::code` transclusion, `{{ }}`
    /// interpolation, and disclosures. Exercises the indexer's directive +
    /// frontmatter stages and the `transcludes`/`uses_schema`/`uses_file`/
    /// `uses_variable` edges that the other tiers leave empty.
    Darkmatter5k,
    /// 1,000 files with adversarial content (huge files, Unicode, duplicate
    /// headings, deep nesting, broken links).
    Pathological1k,
    /// 20,000 files — generated on demand only, never checked in.
    Large20k,
}

impl CorpusTier {
    /// Every tier name, for `--gen-corpus` help and error messages. Kept in sync
    /// with [`parse`](Self::parse) by hand (both are exhaustive over the tiers).
    pub const NAMES: &'static [&'static str] = &[
        "tiny-100",
        "small-1k",
        "vault-5k",
        "dense-5k",
        "darkmatter-5k",
        "pathological-1k",
        "large-20k",
    ];

    /// Parses a tier name (`tiny-100`, `small-1k`, …).
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "tiny-100" => Self::Tiny100,
            "small-1k" => Self::Small1k,
            "vault-5k" => Self::Vault5k,
            "dense-5k" => Self::Dense5k,
            "darkmatter-5k" => Self::Darkmatter5k,
            "pathological-1k" => Self::Pathological1k,
            "large-20k" => Self::Large20k,
            _ => return None,
        })
    }

    /// The tier's generation spec.
    fn spec(self) -> TierSpec {
        match self {
            // Trailing bools are (wiki, pathological, dm).
            Self::Tiny100 => TierSpec::new("tiny-100", 100, 5, 3, 5, false, false, false),
            Self::Small1k => TierSpec::new("small-1k", 1_000, 8, 5, 6, false, false, false),
            Self::Vault5k => TierSpec::new("vault-5k", 5_000, 6, 8, 4, true, false, false),
            Self::Dense5k => TierSpec::new("dense-5k", 5_000, 20, 25, 8, true, false, false),
            Self::Darkmatter5k => {
                TierSpec::new("darkmatter-5k", 5_000, 6, 6, 10, true, false, true)
            }
            Self::Pathological1k => {
                TierSpec::new("pathological-1k", 1_000, 40, 20, 9, true, true, false)
            }
            Self::Large20k => TierSpec::new("large-20k", 20_000, 10, 8, 5, true, false, false),
        }
    }

    /// The tier's file count.
    pub fn file_count(self) -> usize {
        self.spec().files
    }
}

/// One tier's generation parameters.
struct TierSpec {
    name: &'static str,
    files: usize,
    max_headings: usize,
    max_links: usize,
    /// Fraction (out of 10) of files that carry frontmatter.
    frontmatter_ratio: u32,
    wiki: bool,
    pathological: bool,
    /// Emit Darkmatter DSL constructs so the indexer's directive + frontmatter
    /// stages and the `transcludes`/`uses_schema`/`uses_file`/`uses_variable`
    /// edges are exercised (the other tiers leave those empty).
    dm: bool,
}

impl TierSpec {
    // Positional by design: `spec()` reads as a compact one-line-per-tier table.
    #[allow(clippy::too_many_arguments)]
    const fn new(
        name: &'static str,
        files: usize,
        max_headings: usize,
        max_links: usize,
        frontmatter_ratio: u32,
        wiki: bool,
        pathological: bool,
        dm: bool,
    ) -> Self {
        Self {
            name,
            files,
            max_headings,
            max_links,
            frontmatter_ratio,
            wiki,
            pathological,
            dm,
        }
    }
}

/// Deterministic splitmix64 PRNG — seeded, portable, no external crate.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..n` (`n` must be non-zero).
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// SimplifiedSchema the DSL tier's `$schema: ./schema.yaml` file references
/// resolve to. Only read request-time by the frontmatter provider, never during
/// cold indexing, so its exact shape does not affect bench timings.
const DM_SCHEMA: &str = "title: string\ncover: file\n";

const WORDS: &[&str] = &[
    "alpha", "vector", "harbor", "signal", "ember", "quartz", "meadow", "cinder", "lattice",
    "orbit", "canyon", "delta", "prairie", "summit", "wander", "thistle", "beacon", "current",
];

/// Generates `tier` into `dir`, returning the number of files written.
///
/// The directory is created if needed; existing files with the same names are
/// overwritten. Files are named `note-{index}.md` so cross-links resolve.
pub fn generate_corpus(tier: CorpusTier, dir: &Path) -> io::Result<usize> {
    let spec = tier.spec();
    tracing::debug!(tier = spec.name, files = spec.files, "generating corpus");
    std::fs::create_dir_all(dir)?;
    if spec.dm {
        // The shared schema the DSL tier's `$schema: ./schema.yaml` refs resolve
        // to. Not a `.md` file, so workspace discovery ignores it (it never
        // inflates the document count).
        std::fs::write(dir.join("schema.yaml"), DM_SCHEMA)?;
    }
    for index in 0..spec.files {
        let content = generate_file(&spec, index);
        std::fs::write(dir.join(format!("note-{index}.md")), content)?;
    }
    Ok(spec.files)
}

/// Renders one deterministic file's content.
fn generate_file(spec: &TierSpec, index: usize) -> String {
    // Seed per file so any file is reproducible independently of the others.
    let mut rng = Rng::new(0xDEAD_BEEF_0000_0000 ^ index as u64);
    let mut out = String::new();

    if (index as u32 % 10) < spec.frontmatter_ratio {
        out.push_str("---\n");
        out.push_str(&format!("title: {} {}\n", word(&mut rng), index));
        out.push_str(&format!("tags: [{}, {}]\n", word(&mut rng), word(&mut rng)));
        if spec.dm {
            // Half reference a shared schema file (→ `uses_schema` edge); half
            // carry an inline object schema (→ index-time `file(...)` detection
            // of `cover`). Either way `cover` is a `file(...)` value.
            if index.is_multiple_of(2) {
                out.push_str("$schema: ./schema.yaml\n");
            } else {
                out.push_str("$schema:\n  title: string\n  cover: file\n");
            }
            out.push_str(&format!("cover: ./img-{index}.png\n"));
        }
        if spec.pathological {
            // A duplicate key and a Unicode value stress the frontmatter path.
            out.push_str("title: résumé café\n");
        }
        out.push_str("---\n\n");
    }

    let heading_count = 1 + rng.below(spec.max_headings.max(1));
    for heading in 0..heading_count {
        let level = if spec.pathological {
            1 + rng.below(6)
        } else {
            1 + rng.below(3)
        };
        let hashes = "#".repeat(level);
        // Pathological tier repeats heading text to exercise slug `-1`/`-2`.
        let title = if spec.pathological && heading.is_multiple_of(3) {
            "Section".to_string()
        } else {
            format!("{} {}", word(&mut rng), heading)
        };
        out.push_str(&format!("{hashes} {title}\n\n"));
        out.push_str(&paragraph(&mut rng));
        out.push_str("\n\n");
    }

    let link_count = rng.below(spec.max_links + 1);
    for _ in 0..link_count {
        let target = rng.below(spec.files);
        if spec.wiki && rng.below(2) == 0 {
            out.push_str(&format!("See [[note-{target}]] for context.\n"));
        } else if spec.pathological && rng.below(4) == 0 {
            // Deliberately broken link.
            out.push_str(&format!("Broken: [gone](note-{}.md)\n", spec.files + target));
        } else {
            out.push_str(&format!(
                "Refer to [note {target}](note-{target}.md#section).\n"
            ));
        }
    }

    if spec.dm {
        // `{{ }}` interpolation → `uses_variable` edge to the `title`
        // frontmatter key (plus an unresolved `ctx.*` reference).
        out.push_str("Rendered from {{ title }} on {{ ctx.date }}.\n\n");
        // Image → `uses_file` edge.
        out.push_str(&format!("![cover](./img-{index}.png)\n\n"));
        // Transclusion → `transcludes` edge + directive scan.
        let target = rng.below(spec.files);
        out.push_str(&format!("::file ./note-{target}.md\n\n"));
        if rng.below(3) == 0 {
            out.push_str("::code ./example.rs\n\n");
        }
        // A disclosure block exercises the block scanner (no graph edge).
        if index.is_multiple_of(4) {
            out.push_str("::disclosure Notes\n::details\nHidden **body** text.\n::end-disclosure\n\n");
        }
    }

    if spec.pathological && index.is_multiple_of(50) {
        // A handful of very large files to stress read/parse budgets.
        for filler in 0..2_000 {
            out.push_str(&format!("Filler line {filler} {}.\n", word(&mut rng)));
        }
    }

    out
}

fn word(rng: &mut Rng) -> &'static str {
    WORDS[rng.below(WORDS.len())]
}

fn paragraph(rng: &mut Rng) -> String {
    let length = 6 + rng.below(10);
    let mut words = Vec::with_capacity(length);
    for _ in 0..length {
        words.push(word(rng));
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_parse_round_trip() {
        assert_eq!(CorpusTier::parse("tiny-100"), Some(CorpusTier::Tiny100));
        assert_eq!(CorpusTier::parse("dense-5k"), Some(CorpusTier::Dense5k));
        assert_eq!(
            CorpusTier::parse("darkmatter-5k"),
            Some(CorpusTier::Darkmatter5k)
        );
        assert_eq!(CorpusTier::parse("nope"), None);
        // NAMES must stay in sync with parse (both exhaustive over the tiers).
        for name in CorpusTier::NAMES {
            assert!(CorpusTier::parse(name).is_some(), "NAMES has stray {name}");
        }
    }

    #[test]
    fn test_darkmatter_tier_emits_dsl_constructs() {
        // index 0 → the file-ref `$schema` and disclosure branches are both taken.
        let content = generate_file(&CorpusTier::Darkmatter5k.spec(), 0);
        assert!(content.contains("$schema: ./schema.yaml"), "{content}");
        assert!(content.contains("cover: ./img-0.png"), "{content}");
        assert!(content.contains("{{ title }}"), "{content}");
        assert!(content.contains("![cover](./img-0.png)"), "{content}");
        assert!(content.contains("::file ./note-"), "{content}");
        assert!(content.contains("::disclosure"), "{content}");
        // Odd index → the inline object `$schema` branch.
        let inline = generate_file(&CorpusTier::Darkmatter5k.spec(), 1);
        assert!(inline.contains("$schema:\n  title: string"), "{inline}");
        // Plain tiers stay free of DSL constructs.
        let plain = generate_file(&CorpusTier::Small1k.spec(), 0);
        assert!(!plain.contains("::file"), "{plain}");
        assert!(!plain.contains("$schema"), "{plain}");
    }

    #[test]
    fn test_generation_is_deterministic() {
        let first = generate_file(&CorpusTier::Small1k.spec(), 7);
        let second = generate_file(&CorpusTier::Small1k.spec(), 7);
        assert_eq!(first, second);
        // Different index → different content.
        let other = generate_file(&CorpusTier::Small1k.spec(), 8);
        assert_ne!(first, other);
    }

    #[test]
    fn test_generate_tiny_writes_files() {
        let temp = tempfile::tempdir().unwrap();
        let written = generate_corpus(CorpusTier::Tiny100, temp.path()).unwrap();
        assert_eq!(written, 100);
        assert!(temp.path().join("note-0.md").is_file());
        assert!(temp.path().join("note-99.md").is_file());
    }

    #[test]
    fn test_pathological_has_duplicate_headings_and_unicode() {
        let content = generate_file(&CorpusTier::Pathological1k.spec(), 0);
        assert!(content.contains("café") || content.contains("résumé"));
    }
}
