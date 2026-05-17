//! Token grammar parser for atomic (`{{token}}`) and block (`<tag>…</tag>`) prose styling.
//!
//! Drives recursion over the input content and consults [`super::styles`]
//! for SGR escape resolution and per-layer state tracking.

use crate::terminal::Terminal;

use super::styles::BlockTagAction;
use super::styles::{
    StyleState, atomic_token_layer, atomic_token_to_escape, atomic_token_to_escape_with_term,
    block_tag_layer, block_tag_to_escape,
};

/// Parse an opening tag into its name and attributes.
pub(super) fn parse_opening_tag(tag_content: &str) -> Option<(String, Vec<(String, String)>)> {
    let tag_content = tag_content.trim();
    if tag_content.is_empty() {
        return None;
    }

    let parts: Vec<&str> = tag_content.splitn(2, |c: char| c.is_whitespace()).collect();
    let tag_name = parts[0].to_lowercase();

    let mut attrs = Vec::new();
    if parts.len() > 1 {
        // Parse attributes
        let attr_str = parts[1];

        // Check if there's any '=' in the attribute string
        // If not, treat the entire remainder as a positional value (empty key)
        if !attr_str.contains('=') {
            let value = attr_str.trim();
            if !value.is_empty() {
                attrs.push((value.to_string(), String::new()));
            }
        } else {
            // Original parsing for key=value attributes
            let mut current_attr = String::new();
            let mut current_value = String::new();
            let mut in_value = false;
            let mut quote_char: Option<char> = None;

            for c in attr_str.chars() {
                if in_value {
                    if let Some(qc) = quote_char {
                        if c == qc {
                            attrs.push((current_attr.clone(), current_value.clone()));
                            current_attr.clear();
                            current_value.clear();
                            in_value = false;
                            quote_char = None;
                        } else {
                            current_value.push(c);
                        }
                    } else if c == '"' || c == '\'' {
                        quote_char = Some(c);
                    } else if c.is_whitespace() {
                        if !current_value.is_empty() {
                            attrs.push((current_attr.clone(), current_value.clone()));
                            current_attr.clear();
                            current_value.clear();
                        }
                        in_value = false;
                    } else {
                        current_value.push(c);
                    }
                } else if c == '=' {
                    in_value = true;
                } else if !c.is_whitespace() {
                    current_attr.push(c);
                }
            }

            // Handle last attribute without closing quote
            if !current_attr.is_empty() || !current_value.is_empty() {
                attrs.push((current_attr, current_value));
            }
        }
    }

    Some((tag_name, attrs))
}

/// Inner token parser that shares a [`StyleState`] across recursion levels.
///
/// Block tags with a style layer push/pop via [`StyleState::set`] and
/// [`StyleState::restore`] so that closing a tag emits the *parent's*
/// escape code (or the layer's default reset) instead of `\x1b[0m`.
pub(super) fn parse_tokens_inner(
    content: &str,
    term: Option<&Terminal>,
    state: &mut StyleState,
) -> String {
    let mut result = String::new();
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        // ── Backslash escapes: \< \> \{ \\ \* \_ \[ \] \( \) ─────────────
        if ch == '\\' {
            match chars.peek() {
                Some(&'<') | Some(&'>') | Some(&'{') | Some(&'\\') | Some(&'*') | Some(&'_')
                | Some(&'[') | Some(&']') | Some(&'(') | Some(&')') => {
                    result.push(chars.next().unwrap());
                }
                _ => result.push(ch),
            }
            continue;
        }

        // ── Atomic tokens: {{token}} ──────────────────────────────────
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'
            let mut token = String::new();
            let mut found_close = false;

            while let Some(c) = chars.next() {
                if c == '}' && chars.peek() == Some(&'}') {
                    chars.next(); // consume second '}'
                    found_close = true;
                    break;
                }
                token.push(c);
            }

            if found_close {
                let token_lower = token.to_ascii_lowercase();

                // Capability-aware lookup: only `double-underline`
                // consults `term`; other tokens fall through to the
                // static table.
                if let Some(escape) = atomic_token_to_escape_with_term(&token, term) {
                    result.push_str(escape.as_ref());
                    state.used_styles = true;

                    // Update layer tracking. We always pass the *raw*
                    // table escape into `state.set` so layer restoration
                    // works regardless of terminal-specific degradation
                    // (e.g. `\x1b[4m` vs `\x1b[4:2m`).
                    if token_lower == "reset" {
                        state.clear_all();
                    } else if token_lower == "reset-style" {
                        state.clear_all_except_background();
                    } else if let Some((layer, is_set)) = atomic_token_layer(&token_lower) {
                        if is_set {
                            state.set(layer, escape.as_ref());
                        } else {
                            state.restore(layer, None);
                        }
                    }
                } else if atomic_token_to_escape(&token).is_some() {
                    // Token exists in the static table but the
                    // capability-aware helper returned `None`
                    // (currently only `double-underline` on a terminal
                    // that supports neither double nor straight
                    // underline). Emit nothing — the parser drops the
                    // styling sequence entirely. The matching close
                    // sequence is also a no-op so layer tracking stays
                    // balanced.
                } else {
                    // Unknown token, output as-is
                    result.push_str("{{");
                    result.push_str(&token);
                    result.push_str("}}");
                }
            } else {
                // Unclosed token, output as-is
                result.push_str("{{");
                result.push_str(&token);
            }
            continue;
        }

        // ── Block tokens: <tag>content</tag> ──────────────────────────
        if ch == '<' {
            let mut tag_content = String::new();
            let mut found_close = false;

            // Collect until '>'
            for c in chars.by_ref() {
                if c == '>' {
                    found_close = true;
                    break;
                }
                tag_content.push(c);
            }

            if found_close && !tag_content.starts_with('/') {
                // Parse the opening tag
                if let Some((tag_name, attrs)) = parse_opening_tag(&tag_content) {
                    // Probe whether this is a recognized tag *before*
                    // consuming any further input. An unrecognized tag must
                    // not trigger the inner-content scan: that would slurp
                    // the rest of the string looking for a closing tag that
                    // does not exist, and then the legacy fallback would
                    // synthesize a fictitious `</tag>` in the output. For
                    // arbitrary user-supplied text like `<root>` or
                    // `<missing.yaml>` the only correct behaviour is to
                    // pass `<tag_content>` through as literal text.
                    let action = match block_tag_to_escape(&tag_name, &attrs, term) {
                        Some(action) => action,
                        None => {
                            result.push('<');
                            result.push_str(&tag_content);
                            result.push('>');
                            continue;
                        }
                    };

                    // Pre-compute tag patterns once
                    let closing_tag = format!("</{}>", tag_name);
                    let opening_tag_full = format!("<{}>", tag_name);
                    let opening_tag_with_space = format!("<{} ", tag_name);

                    let mut inner_content = String::new();
                    let mut depth = 1;

                    while depth > 0 {
                        if let Some(c) = chars.next() {
                            inner_content.push(c);
                            let len = inner_content.len();

                            if c == '>' {
                                if len >= opening_tag_full.len() {
                                    let start = len - opening_tag_full.len();
                                    if inner_content.get(start..).is_some_and(|slice| {
                                        slice.eq_ignore_ascii_case(&opening_tag_full)
                                    }) {
                                        depth += 1;
                                    }
                                }
                                if len >= closing_tag.len() {
                                    let start = len - closing_tag.len();
                                    if inner_content.get(start..).is_some_and(|slice| {
                                        slice.eq_ignore_ascii_case(&closing_tag)
                                    }) {
                                        depth -= 1;
                                        if depth == 0 {
                                            inner_content.truncate(start);
                                        }
                                    }
                                }
                            } else if c == ' ' && len >= opening_tag_with_space.len() {
                                let start = len - opening_tag_with_space.len();
                                if inner_content.get(start..).is_some_and(|slice| {
                                    slice.eq_ignore_ascii_case(&opening_tag_with_space)
                                }) {
                                    depth += 1;
                                }
                            }
                        } else {
                            break;
                        }
                    }

                    // Apply the block style
                    {
                        let layer = block_tag_layer(&tag_name);

                        match action {
                            BlockTagAction::Suppress => {
                                // Suppressed tag (e.g., `<double-underline>` on a
                                // terminal that supports neither double nor
                                // straight underlines, `<a href="">link</a>`,
                                // or `<clipboard>`): emit only the inner
                                // content with no escapes.
                                result.push_str(&parse_tokens_inner(&inner_content, term, state));
                                continue;
                            }
                            BlockTagAction::CodeBlock => {
                                // Fenced code block: dim + 2-space indent,
                                // no Prose markup parsing inside.
                                state.used_styles = true;
                                for (idx, line) in inner_content.lines().enumerate() {
                                    if idx > 0 {
                                        result.push('\n');
                                    }
                                    result.push_str("\x1b[2m  ");
                                    result.push_str(line);
                                    result.push_str("\x1b[0m");
                                }
                                continue;
                            }
                            BlockTagAction::Wrap { open, close } => {
                                state.used_styles = true;
                                if let Some(layer) = layer {
                                    // Styled tag: layer-aware push/pop
                                    let prev = state.set(layer, open.as_ref());
                                    result.push_str(open.as_ref());
                                    result.push_str(&parse_tokens_inner(
                                        &inner_content,
                                        term,
                                        state,
                                    ));
                                    state.restore(layer, prev);
                                    result.push_str(state.close_code(layer));
                                } else {
                                    // Structural tag (e.g. `<a href>` with OSC8
                                    // or markdown fallback): emit open/close as-is.
                                    let rendered_inner =
                                        parse_tokens_inner(&inner_content, term, state);
                                    let is_markdown_link_fallback = open.as_ref() == "["
                                        && close.as_ref().starts_with("](")
                                        && close.as_ref().ends_with(')');
                                    result.push_str(open.as_ref());
                                    if is_markdown_link_fallback {
                                        result.push_str(&rendered_inner.replace(']', "\\]"));
                                    } else {
                                        result.push_str(&rendered_inner);
                                    }
                                    result.push_str(close.as_ref());
                                }
                                continue;
                            }
                        }
                    }
                }
            }

            // Not a valid block tag, output as-is
            result.push('<');
            result.push_str(&tag_content);
            if found_close {
                result.push('>');
            }
            continue;
        }

        result.push(ch);
    }

    result
}
