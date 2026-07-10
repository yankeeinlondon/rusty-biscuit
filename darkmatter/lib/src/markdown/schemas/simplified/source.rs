//! Source-aware projection for SimplifiedSchema suggestion candidates.

use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;

use super::{
    Constraint, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema, TypeExpr,
    parse_yaml_schema,
};

/// Parses a SimplifiedSchema value and projects suggestion spans into its YAML
/// source.
///
/// `source_offset` is added after YAML scalar projection. Pass zero for a
/// standalone YAML buffer, or the frontmatter YAML block's document offset for
/// inline Markdown. The source is never line-ending-normalized.
pub fn parse_yaml_schema_with_source(
    value: &YamlValue,
    yaml_source: &str,
    source_offset: usize,
) -> Result<SimplifiedSchema, SchemaError> {
    let mut schema = parse_yaml_schema(value)?;
    project_suggestion_spans(&mut schema, yaml_source, source_offset)?;
    Ok(schema)
}

/// Projects expression-relative suggestion spans through YAML scalar quoting
/// and escaping into byte ranges in the caller's source document.
///
/// Plain, single-quoted, and double-quoted YAML scalars are supported, including
/// CRLF input and UTF-8 text. A projection mismatch is a grammar error rather
/// than a silently approximate range.
pub fn project_suggestion_spans(
    schema: &mut SimplifiedSchema,
    yaml_source: &str,
    source_offset: usize,
) -> Result<(), SchemaError> {
    let scalars = scan_value_scalars(yaml_source);
    let mut projector = Projector {
        scalars: &scalars,
        next_scalar: 0,
        source_offset,
    };
    match schema {
        SimplifiedSchema::Single(shape) => projector.shape(shape),
        SimplifiedSchema::Union(arms) => {
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    projector.shape(shape)?;
                }
            }
            Ok(())
        }
    }
}

struct Projector<'a> {
    scalars: &'a [DecodedScalar],
    next_scalar: usize,
    source_offset: usize,
}

impl Projector<'_> {
    fn shape(&mut self, shape: &mut SchemaShape) -> Result<(), SchemaError> {
        for def in shape.properties.values_mut() {
            self.property(def)?;
        }
        for pattern in &mut shape.pattern_keys {
            self.property(&mut pattern.def)?;
        }
        Ok(())
    }

    fn property(&mut self, def: &mut PropertyDef) -> Result<(), SchemaError> {
        match def {
            PropertyDef::Single(atom) => self.atom(atom),
            PropertyDef::Union(atoms) => {
                for atom in atoms {
                    self.atom(atom)?;
                }
                Ok(())
            }
        }
    }

    fn atom(&mut self, atom: &mut PropertyAtom) -> Result<(), SchemaError> {
        if let TypeExpr::InlineObject(shape) = &mut atom.ty {
            self.shape(shape)?;
        }
        let Some(candidates) = atom.constraints.iter_mut().find_map(|constraint| {
            if let Constraint::Suggest(candidates) = constraint {
                Some(candidates)
            } else {
                None
            }
        }) else {
            return Ok(());
        };

        let Some((scalar_index, scalar)) = self.scalars[self.next_scalar..]
            .iter()
            .enumerate()
            .find(|(_, scalar)| scalar_matches_candidates(scalar, candidates))
            .map(|(relative, scalar)| (self.next_scalar + relative, scalar))
        else {
            return Err(projection_error());
        };
        self.next_scalar = scalar_index + 1;
        for candidate in candidates {
            let start = *scalar
                .decoded_to_raw
                .get(candidate.span.start)
                .ok_or_else(projection_error)?;
            let end = *scalar
                .decoded_to_raw
                .get(candidate.span.end)
                .ok_or_else(projection_error)?;
            candidate.span = self.source_offset + start..self.source_offset + end;
        }
        Ok(())
    }
}

fn scalar_matches_candidates(
    scalar: &DecodedScalar,
    expected: &[super::SuggestionCandidate],
) -> bool {
    let Ok(atom) = super::grammar::parse_type_expr("<source>", &scalar.decoded) else {
        return false;
    };
    atom.constraints.iter().any(|constraint| {
        let Constraint::Suggest(actual) = constraint else {
            return false;
        };
        actual.len() == expected.len()
            && actual.iter().zip(expected).all(|(actual, expected)| {
                actual.decoded == expected.decoded
                    && actual.interpreted == expected.interpreted
                    && actual.canonical_decimal == expected.canonical_decimal
            })
    })
}

fn projection_error() -> SchemaError {
    SchemaError::Grammar {
        property: "<source>".into(),
        message: "could not project SimplifiedSchema expression spans through YAML source".into(),
        span: 0..0,
    }
}

#[derive(Debug)]
struct DecodedScalar {
    decoded: String,
    /// One raw byte offset per decoded byte boundary.
    decoded_to_raw: Vec<usize>,
}

fn scan_value_scalars(source: &str) -> Vec<DecodedScalar> {
    let mut scalars = Vec::new();
    let mut line_start = 0;
    for line_with_ending in source.split_inclusive('\n') {
        let line = line_with_ending
            .strip_suffix('\n')
            .unwrap_or(line_with_ending)
            .strip_suffix('\r')
            .unwrap_or_else(|| line_with_ending.strip_suffix('\n').unwrap_or(line_with_ending));
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            line_start += line_with_ending.len();
            continue;
        }
        let value_at = if let Some(rest) = trimmed.strip_prefix('-') {
            if rest.starts_with(char::is_whitespace) {
                let leading = rest.len() - rest.trim_start().len();
                let item = rest.trim_start();
                mapping_value_offset(item)
                    .map(|offset| {
                        let value = &item[offset..];
                        indent + 1 + leading + offset + (value.len() - value.trim_start().len())
                    })
                    .or(Some(indent + 1 + leading))
            } else {
                None
            }
        } else {
            mapping_value_offset(line).map(|offset| {
                let rest = &line[offset..];
                offset + (rest.len() - rest.trim_start().len())
            })
        };
        if let Some(value_at) = value_at.filter(|offset| *offset < line.len()) {
            scan_value(&line[value_at..], line_start + value_at, &mut scalars);
        }
        line_start += line_with_ending.len();
    }
    // `split_inclusive` produces no item for an empty trailing remainder, and
    // handles a final line without `\n` in the ordinary case above.
    scalars
}

fn mapping_value_offset(line: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, ch) {
            (Some('"'), '\\') => escaped = true,
            (Some(active), current) if active == current => quote = None,
            (None, '\'' | '"') => quote = Some(ch),
            (None, ':') => return Some(index + 1),
            _ => {}
        }
    }
    None
}

fn scan_value(raw: &str, base: usize, out: &mut Vec<DecodedScalar>) {
    if raw.starts_with('[') {
        let mut offset = 1;
        while offset < raw.len() {
            offset += raw[offset..].len() - raw[offset..].trim_start().len();
            if raw[offset..].starts_with(']') {
                break;
            }
            if let Some((scalar, consumed)) = decode_scalar(&raw[offset..], base + offset) {
                out.push(scalar);
                offset += consumed;
            } else {
                break;
            }
            if let Some(comma) = raw[offset..].find(',') {
                offset += comma + 1;
            } else {
                break;
            }
        }
    } else if let Some((scalar, _)) = decode_scalar(raw, base) {
        out.push(scalar);
    }
}

fn decode_scalar(raw: &str, base: usize) -> Option<(DecodedScalar, usize)> {
    match raw.as_bytes().first().copied()? {
        b'\'' => decode_single_quoted(raw, base),
        b'"' => decode_double_quoted(raw, base),
        _ => {
            let content = raw.trim_end();
            let mut map = Vec::with_capacity(content.len() + 1);
            map.extend(base..=base + content.len());
            Some((DecodedScalar {
                decoded: content.to_string(),
                decoded_to_raw: map,
            }, content.len()))
        }
    }
}

fn decode_single_quoted(raw: &str, base: usize) -> Option<(DecodedScalar, usize)> {
    let mut decoded = String::new();
    let mut map = vec![base + 1];
    let mut cursor = 1;
    while cursor < raw.len() {
        if raw[cursor..].starts_with("''") {
            push_mapped(&mut decoded, &mut map, "'", base + cursor + 2);
            cursor += 2;
        } else if raw.as_bytes()[cursor] == b'\'' {
            return Some((DecodedScalar { decoded, decoded_to_raw: map }, cursor + 1));
        } else {
            let ch = raw[cursor..].chars().next()?;
            cursor += ch.len_utf8();
            push_mapped(&mut decoded, &mut map, &ch.to_string(), base + cursor);
        }
    }
    None
}

fn decode_double_quoted(raw: &str, base: usize) -> Option<(DecodedScalar, usize)> {
    let mut decoded = String::new();
    let mut map = vec![base + 1];
    let mut cursor = 1;
    while cursor < raw.len() {
        match raw.as_bytes()[cursor] {
            b'"' => {
                return Some((DecodedScalar { decoded, decoded_to_raw: map }, cursor + 1));
            }
            b'\\' => {
                let (value, consumed) = decode_yaml_escape(&raw[cursor..])?;
                cursor += consumed;
                push_mapped(&mut decoded, &mut map, &value, base + cursor);
            }
            _ => {
                let ch = raw[cursor..].chars().next()?;
                cursor += ch.len_utf8();
                push_mapped(&mut decoded, &mut map, &ch.to_string(), base + cursor);
            }
        }
    }
    None
}

fn decode_yaml_escape(raw: &str) -> Option<(String, usize)> {
    let escape = *raw.as_bytes().get(1)?;
    let simple = match escape {
        b'0' => Some('\0'),
        b'a' => Some('\u{7}'),
        b'b' => Some('\u{8}'),
        b't' | b'\t' => Some('\t'),
        b'n' => Some('\n'),
        b'v' => Some('\u{b}'),
        b'f' => Some('\u{c}'),
        b'r' => Some('\r'),
        b'e' => Some('\u{1b}'),
        b' ' => Some(' '),
        b'"' => Some('"'),
        b'/' => Some('/'),
        b'\\' => Some('\\'),
        b'N' => Some('\u{85}'),
        b'_' => Some('\u{a0}'),
        b'L' => Some('\u{2028}'),
        b'P' => Some('\u{2029}'),
        _ => None,
    };
    if let Some(ch) = simple {
        return Some((ch.to_string(), 2));
    }
    let digits = match escape {
        b'x' => 2,
        b'u' => 4,
        b'U' => 8,
        _ => return None,
    };
    let end = 2 + digits;
    let value = u32::from_str_radix(raw.get(2..end)?, 16).ok()?;
    Some((char::from_u32(value)?.to_string(), end))
}

fn push_mapped(decoded: &mut String, map: &mut Vec<usize>, value: &str, raw_end: usize) {
    decoded.push_str(value);
    while map.len() <= decoded.len() {
        map.push(raw_end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate_span(yaml: &str, offset: usize) -> std::ops::Range<usize> {
        let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
        let mut schema = parse_yaml_schema_with_source(&value, yaml, offset).unwrap();
        let SimplifiedSchema::Single(shape) = &mut schema else {
            panic!("expected single shape");
        };
        let PropertyDef::Single(atom) = &shape.properties["value"] else {
            panic!("expected single atom");
        };
        let Constraint::Suggest(candidates) = atom
            .constraints
            .iter()
            .find(|constraint| matches!(constraint, Constraint::Suggest(_)))
            .unwrap()
        else {
            unreachable!()
        };
        candidates[1].span.clone()
    }

    #[test]
    fn projects_plain_single_and_double_quoted_scalars() {
        for yaml in [
            "value: string(suggest(alpha, 'café'))\n",
            "value: 'string(suggest(alpha, ''café''))'\n",
            "value: \"string(suggest(alpha, 'caf\\u00e9'))\"\n",
        ] {
            let span = candidate_span(yaml, 0);
            assert!(yaml[span].contains("caf"), "{yaml:?}");
        }
    }

    #[test]
    fn projection_preserves_crlf_and_document_offset() {
        let yaml = "café: string\r\nvalue: string(suggest(alpha, beta))\r\n";
        let span = candidate_span(yaml, 100);
        assert_eq!(&yaml[span.start - 100..span.end - 100], "beta");
    }
}
