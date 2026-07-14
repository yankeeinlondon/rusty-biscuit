//! Source-aware projection for SimplifiedSchema suggestion candidates.

use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;

use super::yaml_scalar::{self, DecodedScalar};
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
        if atom_has_suggestions(atom)
            && let Some((scalar_index, scalar)) = self.scalars[self.next_scalar..]
                .iter()
                .enumerate()
                .find(|(_, scalar)| scalar_matches_atom(scalar, atom))
                .map(|(relative, scalar)| (self.next_scalar + relative, scalar))
        {
            self.next_scalar = scalar_index + 1;
            return project_atom_suggestions(atom, scalar, self.source_offset);
        }

        if let TypeExpr::InlineObject(shape) = &mut atom.ty {
            self.shape(shape)?;
        }
        if atom.constraints.iter().any(|constraint| matches!(constraint, Constraint::Suggest(_))) {
            return Err(projection_error());
        }
        Ok(())
    }
}

fn atom_has_suggestions(atom: &PropertyAtom) -> bool {
    atom.constraints.iter().any(|constraint| matches!(constraint, Constraint::Suggest(_)))
        || match &atom.ty {
            TypeExpr::InlineObject(shape) => shape.properties.values().any(property_has_suggestions)
                || shape.pattern_keys.iter().any(|pattern| property_has_suggestions(&pattern.def)),
            TypeExpr::Primitive(_) | TypeExpr::Imported { .. } => false,
        }
}

fn property_has_suggestions(def: &PropertyDef) -> bool {
    match def {
        PropertyDef::Single(atom) => atom_has_suggestions(atom),
        PropertyDef::Union(atoms) => atoms.iter().any(atom_has_suggestions),
    }
}

fn scalar_matches_atom(scalar: &DecodedScalar, expected: &PropertyAtom) -> bool {
    super::grammar::parse_type_expr("<source>", scalar.decoded())
        .is_ok_and(|actual| actual == *expected)
}

fn project_atom_suggestions(
    atom: &mut PropertyAtom,
    scalar: &DecodedScalar,
    source_offset: usize,
) -> Result<(), SchemaError> {
    if let TypeExpr::InlineObject(shape) = &mut atom.ty {
        for def in shape.properties.values_mut() {
            project_property_suggestions(def, scalar, source_offset)?;
        }
        for pattern in &mut shape.pattern_keys {
            project_property_suggestions(&mut pattern.def, scalar, source_offset)?;
        }
    }
    if let Some(candidates) = atom.constraints.iter_mut().find_map(|constraint| {
        if let Constraint::Suggest(candidates) = constraint {
            Some(candidates)
        } else {
            None
        }
    }) {
        for candidate in candidates {
            let start = scalar.raw_offset(candidate.span.start).ok_or_else(projection_error)?;
            let end = scalar.raw_offset(candidate.span.end).ok_or_else(projection_error)?;
            candidate.span = source_offset + start..source_offset + end;
        }
    }
    Ok(())
}

fn project_property_suggestions(
    def: &mut PropertyDef,
    scalar: &DecodedScalar,
    source_offset: usize,
) -> Result<(), SchemaError> {
    match def {
        PropertyDef::Single(atom) => project_atom_suggestions(atom, scalar, source_offset),
        PropertyDef::Union(atoms) => {
            for atom in atoms {
                project_atom_suggestions(atom, scalar, source_offset)?;
            }
            Ok(())
        }
    }
}

fn projection_error() -> SchemaError {
    SchemaError::Grammar {
        property: "<source>".into(),
        message: "could not project SimplifiedSchema expression spans through YAML source".into(),
        span: 0..0,
    }
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
            if let Some((scalar, consumed)) = yaml_scalar::decode_scalar_at(&raw[offset..], base + offset) {
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
    } else if let Some((scalar, _)) = yaml_scalar::decode_scalar_at(raw, base) {
        out.push(scalar);
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

    #[test]
    fn projects_nested_inline_object_candidates_through_containing_scalar() {
        let yaml = "value: \"{ mode: string(min(5); suggest(no, valid)) }\"\n";
        let value: YamlValue = serde_yaml_ng::from_str(yaml).unwrap();
        let schema = parse_yaml_schema_with_source(&value, yaml, 0).unwrap();
        let SimplifiedSchema::Single(shape) = schema else {
            panic!("expected single shape");
        };
        let PropertyDef::Single(value) = &shape.properties["value"] else {
            panic!("expected single value atom");
        };
        let TypeExpr::InlineObject(nested) = &value.ty else {
            panic!("expected inline object");
        };
        let PropertyDef::Single(mode) = &nested.properties["mode"] else {
            panic!("expected single mode atom");
        };
        let Constraint::Suggest(candidates) = mode
            .constraints
            .iter()
            .find(|constraint| matches!(constraint, Constraint::Suggest(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let expected = yaml.find("no").unwrap();
        assert_eq!(candidates[0].span, expected..expected + 2);
    }
}
