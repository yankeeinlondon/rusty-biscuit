    //! Conservative line-range recovery for `pre_checks` / `post_checks`
    //! frontmatter blocks.
    //!
    //! Given the raw YAML frontmatter text and the 1-indexed line number of
    //! its first line, produce per-rule line ranges keyed by declaration
    //! order. The finder is intentionally conservative: when it cannot
    //! unambiguously match a rule's span, it omits the range rather than
    //! guessing.

    use std::ops::RangeInclusive;

    /// Per-rule line ranges keyed by declaration order.
    #[derive(Debug, Default, Clone)]
    pub(crate) struct SpanIndex {
        pre: Vec<Option<RangeInclusive<usize>>>,
        post: Vec<Option<RangeInclusive<usize>>>,
    }

    impl SpanIndex {
        /// Look up the line range of the *i*-th `pre_checks` rule in
        /// declaration order. Returns `None` if no unambiguous span was
        /// recovered or the index is out of bounds.
        pub(crate) fn pre_check(&self, i: usize) -> Option<RangeInclusive<usize>> {
            self.pre.get(i).cloned().flatten()
        }

        /// Look up the line range of the *i*-th `post_checks` rule in
        /// declaration order. Returns `None` if no unambiguous span was
        /// recovered or the index is out of bounds.
        pub(crate) fn post_check(&self, i: usize) -> Option<RangeInclusive<usize>> {
            self.post.get(i).cloned().flatten()
        }
    }

    /// Build a [`SpanIndex`] for the given frontmatter text.
    ///
    /// `base_line` is the 1-indexed line number of the first line of
    /// `frontmatter_text`. The frontmatter slice does NOT include the
    /// opening `---` fence; the caller is expected to have stripped it.
    ///
    /// ## Returns
    ///
    /// A `SpanIndex` with one slot per rule in declaration order. Slots are
    /// `Some(range)` when the finder is confident; `None` otherwise.
    pub(crate) fn find_rule_spans(frontmatter_text: &str, base_line: usize) -> SpanIndex {
        let mut index = SpanIndex::default();

        let lines: Vec<&str> = frontmatter_text.split('\n').collect();
        // Locate the `pre_checks:` / `post_checks:` parent lines. These must
        // appear at indent 0 (top-level frontmatter keys) with no prefix
        // whitespace; anything more nested is rejected as ambiguous.
        for (parent_key, slot) in [
            ("pre_checks", &mut index.pre),
            ("post_checks", &mut index.post),
        ] {
            let Some(parent_idx) = find_top_level_key(&lines, parent_key) else {
                continue;
            };
            *slot = collect_rule_spans(&lines, parent_idx, base_line);
        }

        index
    }

    /// Find the index in `lines` of a top-level frontmatter key matching
    /// `key`. Top-level means zero leading whitespace and `key:` immediately
    /// at column 0. Returns the *first* match. If the key appears more than
    /// once at the top level, returns `None` (ambiguous).
    fn find_top_level_key(lines: &[&str], key: &str) -> Option<usize> {
        let mut found: Option<usize> = None;
        let needle_eq = format!("{key}:");
        let needle_prefix = format!("{key}: ");
        for (i, line) in lines.iter().enumerate() {
            if *line == needle_eq || line.starts_with(&needle_prefix) {
                if found.is_some() {
                    return None;
                }
                found = Some(i);
            }
        }
        found
    }

    /// Walk lines after `parent_idx` and produce per-rule line ranges in
    /// declaration order. Handles list form (entries beginning with `-`) and
    /// map form (indented `name:` keys).
    fn collect_rule_spans(
        lines: &[&str],
        parent_idx: usize,
        base_line: usize,
    ) -> Vec<Option<RangeInclusive<usize>>> {
        // Determine the body indent by looking at the first non-blank,
        // non-comment line after the parent. If none exists, no rules.
        let mut body_indent: Option<usize> = None;
        for line in lines.iter().skip(parent_idx + 1) {
            if is_blank_or_comment(line) {
                continue;
            }
            let indent = leading_spaces(line);
            if indent == 0 {
                // Sibling top-level key — no body.
                break;
            }
            body_indent = Some(indent);
            break;
        }
        let Some(body_indent) = body_indent else {
            return Vec::new();
        };

        // Detect form: list form starts each entry with `- `; map form does not.
        let mut entries: Vec<RangeInclusive<usize>> = Vec::new();
        let mut current_start: Option<usize> = None;
        let mut current_end: usize = 0;
        let mut is_list_form: Option<bool> = None;

        let body_iter = lines.iter().enumerate().skip(parent_idx + 1);
        for (i, line) in body_iter {
            if is_blank_or_comment(line) {
                continue;
            }
            let indent = leading_spaces(line);
            if indent < body_indent {
                // Either back to top level or to a shallower scope; rules end.
                break;
            }
            if indent == body_indent {
                // Possible new rule entry.
                let trimmed = &line[indent..];
                let starts_with_dash = trimmed.starts_with("- ") || trimmed == "-";
                match is_list_form {
                    None => {
                        is_list_form = Some(starts_with_dash);
                    }
                    Some(true) => {
                        if !starts_with_dash {
                            // Inconsistent with list form; abort.
                            return mark_all_ambiguous(entries.len() + 1);
                        }
                    }
                    Some(false) => {
                        if starts_with_dash {
                            // Inconsistent with map form; abort.
                            return mark_all_ambiguous(entries.len() + 1);
                        }
                    }
                }
                // Close out the previous entry.
                if let Some(start) = current_start.take() {
                    entries.push(start..=current_end);
                }
                current_start = Some(i);
                current_end = i;
            } else if indent > body_indent {
                // Continuation of the current rule (multi-line value).
                if current_start.is_some() {
                    current_end = i;
                }
            }
        }
        if let Some(start) = current_start.take() {
            entries.push(start..=current_end);
        }

        // Detect ambiguity: duplicate top-level rule names within this block.
        // Map form: the key on the entry's start line (after indent, before `:`).
        // List form: the first inline key after `- ` on the entry's start line.
        let mut seen: Vec<&str> = Vec::with_capacity(entries.len());
        let mut ambiguous = false;
        let names: Vec<Option<&str>> = entries
            .iter()
            .map(|r| extract_rule_name(lines[*r.start()], body_indent, is_list_form))
            .collect();
        for name in names.iter().flatten() {
            if seen.contains(name) {
                ambiguous = true;
                break;
            }
            seen.push(name);
        }

        if ambiguous {
            return entries.iter().map(|_| None).collect();
        }

        entries
            .into_iter()
            .map(|r| Some((r.start() + base_line)..=(r.end() + base_line)))
            .collect()
    }

    /// Build an all-`None` vector of length `n` to signal ambiguity.
    fn mark_all_ambiguous(n: usize) -> Vec<Option<RangeInclusive<usize>>> {
        vec![None; n]
    }

    /// Return the count of leading space characters in `line`.
    fn leading_spaces(line: &str) -> usize {
        line.bytes().take_while(|b| *b == b' ').count()
    }

    /// Return `true` for blank lines and comment-only lines (lines whose
    /// first non-space character is `#`).
    fn is_blank_or_comment(line: &str) -> bool {
        let trimmed = line.trim_start();
        trimmed.is_empty() || trimmed.starts_with('#')
    }

    /// Extract the rule name from the first line of a rule entry.
    ///
    /// For map form, the line looks like `  rule_name:` or `  rule_name: value`.
    /// For list form, the line looks like `  - rule_name: value` or
    /// `  - rule_name:`.
    fn extract_rule_name(
        line: &str,
        body_indent: usize,
        is_list_form: Option<bool>,
    ) -> Option<&str> {
        if line.len() < body_indent {
            return None;
        }
        let after_indent = &line[body_indent..];
        let key_part = if is_list_form == Some(true) {
            // strip `- ` or `-` prefix
            after_indent
                .strip_prefix("- ")
                .or_else(|| after_indent.strip_prefix('-'))?
        } else {
            after_indent
        };
        let colon = key_part.find(':')?;
        let raw_name = key_part[..colon].trim();
        if raw_name.is_empty() {
            return None;
        }
        // Comments between `- ` and the key are unusual but possible; bail
        // out by rejecting any name containing whitespace or `#`.
        if raw_name
            .bytes()
            .any(|b| b == b' ' || b == b'#' || b == b'\t')
        {
            return None;
        }
        Some(raw_name)
    }
