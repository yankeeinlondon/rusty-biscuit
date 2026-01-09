/// Options controlling which identifier to use as the "wire id" source.
#[derive(Debug, Clone, Copy)]
pub struct VariantNameOptions {
    /// If true and `canonical_slug` is present, prefer it over `id`.
    pub prefer_canonical_slug: bool,
}

impl Default for VariantNameOptions {
    fn default() -> Self {
        Self {
            prefer_canonical_slug: true,
        }
    }
}

/// Generate the enum variant name for a model definition.
///
/// Encoding rules:
/// - Aggregator ids: "provider/model" -> "Provider___Model"
/// (delimiter: `___`)
/// - Within provider/model text:
///     '-' => `__`
///     '.' => `_`
/// - Alphanumeric runs become PascalCase segments.
/// - Any other non-alphanumeric separator is treated like '-' (encoded as `__`).
///
/// Notes:
/// - If the resulting identifier would start with a digit, we prefix it with `M`.
pub fn enum_variant_name_for_model(def: &ModelDefinition, opts: VariantNameOptions) -> String {
    let wire_id: &str = if opts.prefer_canonical_slug {
        def.canonical_slug.as_deref().unwrap_or(&def.id)
    } else {
        &def.id
    };

    enum_variant_name_from_wire_id(wire_id)
}

/// Generate the enum variant name directly from a wire id.
/// - If `wire_id` contains '/', it's treated as aggregator "provider/model".
/// - Otherwise it's treated as a primary provider model id.
pub fn enum_variant_name_from_wire_id(wire_id: &str) -> String {
    let wire_id = wire_id.trim();
    if wire_id.is_empty() {
        return "Bespoke".to_string(); // or panic/error; choose what fits your pipeline
    }

    if let Some((provider, rest)) = wire_id.split_once('/') {
        let provider_name = encode_pascal_with_separators(provider);
        let model_name = encode_pascal_with_separators(rest);
        format!("{provider_name}___{model_name}")
    } else {
        encode_pascal_with_separators(wire_id)
    }
}

/// Encodes a segment (provider or model) into PascalCase tokens and maps separators:
/// - '-' => `__`
/// - '.' => `_`
/// - other non-alnum => treated as '-' => `__`
///
/// Examples:
/// - "openai" -> "Openai"
/// - "gpt-4.1-mini" -> "Gpt__4_1__Mini"
/// - "kimi-k2-thinking" -> "Kimi__K2__Thinking"
fn encode_pascal_with_separators(input: &str) -> String {
    let mut out = String::new();

    // Current alnum token we are accumulating
    let mut tok = String::new();

    // Flush current token into output in PascalCase form.
    let mut flush_tok = |out: &mut String, tok: &mut String| {
        if tok.is_empty() {
            return;
        }
        out.push_str(&to_pascal_token(tok));
        tok.clear();
    };

    // Append separator encoding to output.
    // '-' => "__"
    // '.' => "_"
    // other => "__" (treated as '-')
    let mut push_sep = |out: &mut String, sep: char| match sep {
        '-' => out.push_str("__"),
        '.' => out.push('_'),
        _ => out.push_str("__"),
    };

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            tok.push(ch);
        } else {
            flush_tok(&mut out, &mut tok);
            push_sep(&mut out, ch);
        }
    }
    flush_tok(&mut out, &mut tok);

    // Make it a valid Rust identifier if it starts with a digit
    if out
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        out.insert(0, 'M');
    }

    // Defensive: collapse any accidental repeated separator sequences.
    // (Mostly relevant for odd upstream ids like "foo--bar" or "foo..bar".)
    out = collapse_runs(out);

    out
}

/// Convert a raw token to PascalCase without introducing separators.
///
/// - If the token is all digits, it's returned unchanged.
/// - Otherwise: first char uppercased, rest lowercased.
///   (This is deterministic and consistent; it does not try to preserve acronyms.)
fn to_pascal_token(token: &str) -> String {
    if token.chars().all(|c| c.is_ascii_digit()) {
        return token.to_string();
    }
    let mut chars = token.chars();
    let first = chars.next().map(|c| c.to_ascii_uppercase()).unwrap_or('X');
    let rest: String = chars.map(|c| c.to_ascii_lowercase()).collect();
    let mut out = String::with_capacity(1 + rest.len());
    out.push(first);
    out.push_str(&rest);
    out
}

/// Collapse runs that can occur due to strange upstream ids.
/// This keeps your encoding stable and readable.
fn collapse_runs(mut s: String) -> String {
    // Collapse more than 2 underscores used for hyphen encoding into exactly 2,
    // unless it's the triple-underscore namespace delimiter which we leave alone.
    //
    // Strategy:
    // - temporarily protect "___"
    // - reduce "____+" => "__"
    // - restore "___"
    const NS_SENTINEL: &str = "\u{E000}"; // private-use sentinel

    s = s.replace("___", NS_SENTINEL);
    while s.contains("____") {
        s = s.replace("____", "__");
    }
    s = s.replace(NS_SENTINEL, "___");

    // Collapse consecutive dots encoding "__" / "_" patterns is inherently unambiguous,
    // but you may wish to reduce multiple "_" to a single "_" for readability in dot encoding.
    while s.contains("__") && s.contains("___") {
        // nothing to do; this loop is just to show we intentionally preserve "___"
        break;
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enum_variant_name_from_wire_id() {
        assert_eq!(
          enum_variant_name_from_wire_id("gpt-4o"),
          "Gpt__4o"
        );
        assert_eq!(
            enum_variant_name_from_wire_id("gpt-4.1-mini"),
            "Gpt__4_1__Mini"
        );
        assert_eq!(
            enum_variant_name_from_wire_id("openai/gpt-4o"),
            "Openai___Gpt__4o"
        );
        assert_eq!(
            enum_variant_name_from_wire_id("moonshotai/kimi-k2-thinking"),
            "Moonshotai___Kimi__K2__Thinking"
        );
    }
}
