//! `Link` header parsing (RFC 8288, appendix B).
//!
//! A header field holds comma-separated link values, each a `<target>`
//! followed by `;`-separated parameters whose values may be quoted strings
//! containing commas and semicolons. The `rel` parameter holds one or more
//! space-separated relation types, compared without case. Every field of a
//! repeated `Link` header is parsed.

use http::HeaderMap;
use http::header::LINK;

/// One parsed link value: its target as written and its relation types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkValue {
    pub target: String,
    pub rels: Vec<String>,
}

/// A `Link` field value that does not follow the grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MalformedLink;

/// Every link value of every `Link` header in `headers`, in order.
///
/// ## Errors
///
/// Returns [`MalformedLink`] when any field is not text or does not parse;
/// a link of the wanted relation inside an unparseable field cannot be
/// trusted to be absent.
pub fn parse_headers(headers: &HeaderMap) -> Result<Vec<LinkValue>, MalformedLink> {
    let mut links = Vec::new();
    for field in headers.get_all(LINK) {
        let text = field.to_str().map_err(|_| MalformedLink)?;
        links.extend(parse_field(text)?);
    }
    Ok(links)
}

/// The targets of every link in `headers` whose relation types include
/// `rel`, in order.
///
/// ## Errors
///
/// Returns [`MalformedLink`] as [`parse_headers`] does.
pub fn targets_with_rel(headers: &HeaderMap, rel: &str) -> Result<Vec<String>, MalformedLink> {
    Ok(parse_headers(headers)?
        .into_iter()
        .filter(|link| link.rels.iter().any(|candidate| candidate.eq_ignore_ascii_case(rel)))
        .map(|link| link.target)
        .collect())
}

/// Parses one `Link` field value.
fn parse_field(text: &str) -> Result<Vec<LinkValue>, MalformedLink> {
    let mut cursor = Cursor { rest: text };
    let mut links = Vec::new();
    loop {
        cursor.skip_while(|c| c == ',' || c.is_whitespace());
        if cursor.rest.is_empty() {
            return Ok(links);
        }
        if !cursor.eat('<') {
            return Err(MalformedLink);
        }
        let end = cursor.rest.find('>').ok_or(MalformedLink)?;
        let target = cursor.rest[..end].trim().to_owned();
        cursor.rest = &cursor.rest[end + 1..];
        let mut rels = Vec::new();
        let mut rel_seen = false;
        loop {
            cursor.skip_while(char::is_whitespace);
            if cursor.rest.is_empty() || cursor.eat(',') {
                break;
            }
            if !cursor.eat(';') {
                return Err(MalformedLink);
            }
            cursor.skip_while(char::is_whitespace);
            let name = cursor.token();
            if name.is_empty() {
                // An empty parameter (`<a>;;rel=next`) is tolerated.
                continue;
            }
            cursor.skip_while(char::is_whitespace);
            let value = if cursor.eat('=') {
                cursor.skip_while(char::is_whitespace);
                if cursor.rest.starts_with('"') {
                    cursor.quoted()?
                } else {
                    cursor.token().to_owned()
                }
            } else {
                String::new()
            };
            // Only the first `rel` counts (RFC 8288 section 3.3).
            if name.eq_ignore_ascii_case("rel") && !rel_seen {
                rel_seen = true;
                rels = value.split_whitespace().map(str::to_owned).collect();
            }
        }
        links.push(LinkValue { target, rels });
    }
}

struct Cursor<'t> {
    rest: &'t str,
}

impl<'t> Cursor<'t> {
    fn eat(&mut self, wanted: char) -> bool {
        match self.rest.strip_prefix(wanted) {
            Some(rest) => {
                self.rest = rest;
                true
            }
            None => false,
        }
    }

    fn skip_while(&mut self, skip: impl Fn(char) -> bool) {
        self.rest = self.rest.trim_start_matches(skip);
    }

    /// A run of characters up to whitespace or a delimiter.
    fn token(&mut self) -> &'t str {
        let end = self
            .rest
            .find(|c: char| c.is_whitespace() || matches!(c, ';' | ',' | '=' | '"'))
            .unwrap_or(self.rest.len());
        let (token, rest) = self.rest.split_at(end);
        self.rest = rest;
        token
    }

    /// A quoted string, with backslash escapes removed.
    fn quoted(&mut self) -> Result<String, MalformedLink> {
        let mut chars = self.rest.char_indices().skip(1);
        let mut value = String::new();
        while let Some((index, c)) = chars.next() {
            match c {
                '"' => {
                    self.rest = &self.rest[index + 1..];
                    return Ok(value);
                }
                '\\' => value.push(chars.next().ok_or(MalformedLink)?.1),
                _ => value.push(c),
            }
        }
        Err(MalformedLink)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rels(text: &str) -> Vec<(String, Vec<String>)> {
        parse_field(text)
            .expect("parses")
            .into_iter()
            .map(|link| (link.target, link.rels))
            .collect()
    }

    #[test]
    fn github_style_field_lists_each_relation() {
        let parsed = rels(r#"<https://a.test/x?page=2>; rel="next", <https://a.test/x?page=5>; rel="last""#);
        assert_eq!(
            parsed,
            [
                ("https://a.test/x?page=2".to_owned(), vec!["next".to_owned()]),
                ("https://a.test/x?page=5".to_owned(), vec!["last".to_owned()]),
            ]
        );
    }

    #[test]
    fn quoted_commas_and_multiple_relation_tokens_stay_in_one_link() {
        let parsed = rels(r#"<?page=2>; title="a, b; c"; rel="prefetch NEXT", <../p>;rel=prev"#);
        assert_eq!(parsed[0].0, "?page=2");
        assert_eq!(parsed[0].1, ["prefetch", "NEXT"]);
        assert_eq!(parsed[1].0, "../p");
        assert_eq!(parsed[1].1, ["prev"]);
    }

    #[test]
    fn malformed_fields_are_errors_not_empty() {
        assert_eq!(parse_field("https://a.test; rel=next"), Err(MalformedLink));
        assert_eq!(parse_field("<https://a.test; rel=next"), Err(MalformedLink));
        assert_eq!(parse_field(r#"<a>; rel="next"#), Err(MalformedLink));
        assert_eq!(parse_field("<a> rel=next"), Err(MalformedLink));
        assert_eq!(parse_field("  "), Ok(Vec::new()));
    }
}
