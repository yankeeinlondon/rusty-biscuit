use crate::markdown::Markdown;
use biscuit_file::serde_yaml_ng;
use biscuit_hash::{HashVariant, xx_hash, xx_hash_variant};

impl Markdown {
    /// Hash a markdown document's frontmatter and/or body.
    pub fn hash(&self, body_only: bool, frontmatter_only: bool, strict: bool) -> String {
        if body_only {
            format!("{:016x}", self.hash_body(strict))
        } else if frontmatter_only {
            format!("{:016x}", self.hash_frontmatter(strict))
        } else {
            format!(
                "{:016x}-{:016x}",
                self.hash_frontmatter(strict),
                self.hash_body(strict)
            )
        }
    }

    /// Hash a markdown document's frontmatter.
    pub fn hash_frontmatter(&self, strict: bool) -> u64 {
        let map = self.frontmatter().as_map();
        if map.is_empty() {
            return xx_hash("");
        }
        if strict {
            let yaml = serde_yaml_ng::to_string(map).unwrap_or_default();
            xx_hash(&yaml)
        } else {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let canonical: String = keys
                .iter()
                .map(|k| {
                    let v = &map[*k];
                    format!("{}:{}", k, serde_json::to_string(v).unwrap_or_default())
                })
                .collect::<Vec<_>>()
                .join("\\n");
            xx_hash(&canonical)
        }
    }

    /// Hash a markdown document's body/prose content.
    pub fn hash_body(&self, strict: bool) -> u64 {
        let content = self.content();
        if strict {
            xx_hash(content)
        } else {
            xx_hash_variant(
                content,
                vec![
                    HashVariant::LeadingWhitespace,
                    HashVariant::TrailingWhitespace,
                    HashVariant::BlankLine,
                ],
            )
        }
    }
}
