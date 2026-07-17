//! Difference explanation engine.
//!
//! Turns a like-for-like comparison into the human-facing report printed by
//! `md hash --diff` and `md hash --save`. The amount a report can say is bounded
//! by the *stored* kind: `simple` distinguishes only frontmatter from body,
//! `structured` adds key-vs-value and structure-vs-content resolution, and
//! `detailed` expands the body into a per-section list.
//!
//! This implementation is intentionally separate from the legacy `Markdown`
//! delta engine: the spec declares the legacy change *descriptions* are not
//! authoritative, so the section classifications and rendered text live here and
//! are owned by the hashing feature.

use super::compare::{ComparisonDetail, HashComparison, IgnorePolicyAdvisory, compare_options};
use super::compute::{ComputedHash, SectionTuple};
use super::kind::MdHashKind;
use super::options::MdHashOptions;
use super::stored::{StoredHash, StoredHashValue, malformed};
use crate::markdown::{Markdown, MarkdownResult};

/// A rendered, kind-aware explanation of how a document differs from its stored
/// hash.
///
/// Build one with [`Markdown::explain_hash_diff`], then call [`Self::render`] to
/// produce the report string. The report shape follows the stored kind; an
/// [`IgnorePolicyAdvisory`], when present, is appended as a trailing line and is
/// never counted as a content change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashExplanation {
    body: ExplanationBody,
    ignore_policy: Option<IgnorePolicyAdvisory>,
}

/// The kind-specific core of an explanation, before the ignore-policy advisory.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ExplanationBody {
    /// `simple` — only the two coarse concerns are known.
    Simple {
        frontmatter_changed: bool,
        body_changed: bool,
    },
    /// `fm` — the single frontmatter concern.
    Fm { changed: bool },
    /// `body` — the single body concern.
    Body { changed: bool },
    /// `structured` — key-vs-value frontmatter resolution and
    /// structure-vs-content body resolution.
    Structured {
        frontmatter: FmConcern,
        body: StructuredBody,
    },
    /// `detailed` — the same frontmatter resolution as `structured`, plus a
    /// per-section body breakdown.
    Detailed {
        frontmatter: FmConcern,
        body: DetailedBody,
    },
}

/// Frontmatter concern at `structured`/`detailed` resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FmConcern {
    /// No change to the frontmatter.
    Unchanged,
    /// A value changed but the key set did not.
    ValuesOnly,
    /// The key set changed (and therefore the values too).
    KeysAndValues,
}

impl FmConcern {
    /// Maps the coarse `frontmatter_changed` plus the keys-only signal onto the
    /// three-way concern. A key change implies a value change, so a changed key
    /// set always reads as keys-and-values.
    fn from_signals(frontmatter_changed: bool, fm_keys_changed: bool) -> Self {
        match (frontmatter_changed, fm_keys_changed) {
            (false, _) => FmConcern::Unchanged,
            (true, false) => FmConcern::ValuesOnly,
            (true, true) => FmConcern::KeysAndValues,
        }
    }

    fn line(self) -> &'static str {
        match self {
            FmConcern::Unchanged => "Frontmatter has not changed",
            FmConcern::ValuesOnly => {
                "Frontmatter has changed but no changes to the keys, only to the values"
            }
            FmConcern::KeysAndValues => "Frontmatter keys and values have changed",
        }
    }
}

/// Body concern at `structured` resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StructuredBody {
    /// No semantic change to the body.
    Unchanged,
    /// The heading layout is unchanged; only section content changed.
    ContentOnly,
    /// Both the heading layout and the content changed.
    Structural,
}

impl StructuredBody {
    fn from_signals(body_changed: bool, body_structure_changed: bool) -> Self {
        match (body_changed, body_structure_changed) {
            (false, _) => StructuredBody::Unchanged,
            (true, false) => StructuredBody::ContentOnly,
            (true, true) => StructuredBody::Structural,
        }
    }

    fn line(self) -> &'static str {
        match self {
            StructuredBody::Unchanged => "No semantic changes to the body",
            StructuredBody::ContentOnly => {
                "The body has the same structural layout as before, however, there are semantic changes to the contents within the content"
            }
            StructuredBody::Structural => {
                "The body has changed both structurally as well as the semantic content"
            }
        }
    }
}

/// Body concern at `detailed` resolution: a preamble change plus per-section
/// reports. `sections` holds surviving/added sections in document order;
/// `removed` holds sections present only in the stored hash, in stored order.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DetailedBody {
    preamble: Option<PreambleReport>,
    sections: Vec<SectionReport>,
    removed: Vec<SectionReport>,
}

impl DetailedBody {
    fn has_changes(&self) -> bool {
        self.preamble.is_some() || !self.sections.is_empty() || !self.removed.is_empty()
    }
}

/// How the preamble (content before the first heading) changed. A removed
/// preamble has no name to surface — its stored value was only ever a hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreambleReport {
    Added,
    Removed,
    Changed,
}

impl PreambleReport {
    fn line(self) -> &'static str {
        match self {
            PreambleReport::Added => "A preamble was added before the first heading",
            PreambleReport::Removed => "The preamble was removed",
            PreambleReport::Changed => "The preamble has changed",
        }
    }
}

/// The classification of one section after alignment. Each renders to one nested
/// body child line. Sections that did not change are omitted entirely.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SectionReport {
    /// Heading and level unchanged; content differs.
    ContentChanged { heading: String },
    /// Heading changed, content unchanged.
    Renamed { heading: String },
    /// Both heading and content changed, but the section persisted.
    RenamedAndEdited { heading: String },
    /// Heading unchanged, level decreased (e.g. H3 → H2).
    Promoted {
        heading: String,
        from: u8,
        to: u8,
        content_changed: bool,
    },
    /// Heading unchanged, level increased (e.g. H2 → H3).
    Demoted {
        heading: String,
        from: u8,
        to: u8,
        content_changed: bool,
    },
    /// Reordered among its siblings; now appears before `before`.
    Reordered {
        heading: String,
        before: String,
        content_changed: bool,
    },
    /// Moved to a different parent.
    Moved {
        heading: String,
        new_parent: Option<String>,
        old_parent: Option<String>,
        content_changed: bool,
    },
    /// Present only in the new document.
    Added { heading: String, level: u8 },
    /// Present only in the stored hash, named from its stored heading text.
    Removed {
        heading: String,
        prev: Option<String>,
        next: Option<String>,
    },
}

impl SectionReport {
    fn line(&self) -> String {
        match self {
            SectionReport::ContentChanged { heading } => {
                format!("\"{heading}\" section: content has changed")
            }
            SectionReport::Renamed { heading } => {
                format!("\"{heading}\" section: heading renamed, content unchanged")
            }
            SectionReport::RenamedAndEdited { heading } => {
                format!("\"{heading}\" section: heading renamed and content has changed")
            }
            SectionReport::Promoted {
                heading,
                from,
                to,
                content_changed,
            } => format!(
                "\"{heading}\" section: promoted from H{from} to H{to}{}",
                content_suffix(*content_changed)
            ),
            SectionReport::Demoted {
                heading,
                from,
                to,
                content_changed,
            } => format!(
                "\"{heading}\" section: demoted from H{from} to H{to}{}",
                content_suffix(*content_changed)
            ),
            SectionReport::Reordered {
                heading,
                before,
                content_changed,
            } => format!(
                "\"{heading}\" section: reordered, now appears before \"{before}\"{}",
                placement_content_suffix(*content_changed)
            ),
            SectionReport::Moved {
                heading,
                new_parent,
                old_parent,
                content_changed,
            } => format!(
                "\"{heading}\" section: {}{}",
                moved_phrase(new_parent, old_parent),
                placement_content_suffix(*content_changed)
            ),
            SectionReport::Added { heading, level } => {
                format!("\"{heading}\" section: added (H{level})")
            }
            SectionReport::Removed {
                heading,
                prev,
                next,
            } => format!("\"{heading}\" section was removed{}", removed_position(prev, next)),
        }
    }
}

/// A promotion/demotion notes its content state only when content is unchanged;
/// a content change is implied by the rest of the report so it is left unstated.
fn content_suffix(content_changed: bool) -> &'static str {
    if content_changed {
        ""
    } else {
        ", content unchanged"
    }
}

/// A placement change (move/reorder) does not imply a content edit the way a
/// promotion/demotion does, so the noteworthy case here is the inverse: a
/// content change is stated explicitly, while an unchanged body needs no note.
fn placement_content_suffix(content_changed: bool) -> &'static str {
    if content_changed {
        ", content has changed"
    } else {
        ""
    }
}

/// Renders the `moved beneath … (previously beneath …)` clause, degrading
/// gracefully when either side is a top-level section with no parent.
fn moved_phrase(new_parent: &Option<String>, old_parent: &Option<String>) -> String {
    match (new_parent, old_parent) {
        (Some(np), Some(op)) => format!("moved beneath \"{np}\" (previously beneath \"{op}\")"),
        (Some(np), None) => format!("moved beneath \"{np}\" (previously at the top level)"),
        (None, Some(op)) => format!("moved to the top level (previously beneath \"{op}\")"),
        (None, None) => "moved".to_string(),
    }
}

/// Renders the positional `(previously …)` clause for a removed section, naming
/// whichever stored neighbors exist.
fn removed_position(prev: &Option<String>, next: &Option<String>) -> String {
    match (prev, next) {
        (Some(p), Some(n)) => format!(" (previously between \"{p}\" and \"{n}\")"),
        (Some(p), None) => format!(" (previously after \"{p}\")"),
        (None, Some(n)) => format!(" (previously before \"{n}\")"),
        (None, None) => String::new(),
    }
}

impl HashExplanation {
    /// Renders the explanation to its report string.
    ///
    /// `simple`, `fm`, and `body` produce a single line; `structured` produces
    /// two bullet lines; `detailed` nests changed sections beneath the body
    /// bullet. When an [`IgnorePolicyAdvisory`] is present it is appended on its
    /// own trailing line.
    pub fn render(&self) -> String {
        let mut report = self.body.render();
        if let Some(advisory) = &self.ignore_policy {
            report.push('\n');
            report.push_str(&render_ignore_policy(advisory));
        }
        report
    }
}

impl ExplanationBody {
    fn render(&self) -> String {
        match self {
            ExplanationBody::Simple {
                frontmatter_changed,
                body_changed,
            } => match (frontmatter_changed, body_changed) {
                (false, false) => "No semantic changes detected".to_string(),
                (true, false) => {
                    "Frontmatter has changed, body remains unchanged semantically".to_string()
                }
                (false, true) => {
                    "Frontmatter remains unchanged, but body has changed".to_string()
                }
                (true, true) => "Both the Frontmatter and body have changed".to_string(),
            },
            ExplanationBody::Fm { changed } => if *changed {
                "Frontmatter has changed"
            } else {
                "Frontmatter remains unchanged"
            }
            .to_string(),
            ExplanationBody::Body { changed } => if *changed {
                "Body has changed"
            } else {
                "Body remains unchanged"
            }
            .to_string(),
            ExplanationBody::Structured { frontmatter, body } => {
                if *frontmatter == FmConcern::Unchanged && *body == StructuredBody::Unchanged {
                    "No semantic changes detected".to_string()
                } else {
                    format!("- {}\n- {}", frontmatter.line(), body.line())
                }
            }
            ExplanationBody::Detailed { frontmatter, body } => {
                if *frontmatter == FmConcern::Unchanged && !body.has_changes() {
                    "No semantic changes detected".to_string()
                } else {
                    format!("- {}\n{}", frontmatter.line(), render_detailed_body(body))
                }
            }
        }
    }
}

/// Renders the body bullet of a `detailed` report, nesting one child per change
/// in document order: the preamble first, then surviving/added sections, then
/// removed sections.
fn render_detailed_body(body: &DetailedBody) -> String {
    if !body.has_changes() {
        return "- No semantic changes to the body".to_string();
    }
    let mut out = String::from("- There were changes to the document body:");
    if let Some(preamble) = &body.preamble {
        out.push_str("\n  - ");
        out.push_str(preamble.line());
    }
    for section in &body.sections {
        out.push_str("\n  - ");
        out.push_str(&section.line());
    }
    for section in &body.removed {
        out.push_str("\n  - ");
        out.push_str(&section.line());
    }
    out
}

/// Renders the advisory line, e.g.
/// `Ignore policy changed: now also ignoring [foo]; previously [bar]`.
fn render_ignore_policy(advisory: &IgnorePolicyAdvisory) -> String {
    format!(
        "Ignore policy changed: now also ignoring {}; previously {}",
        render_name_list(&advisory.now_ignoring),
        render_name_list(&advisory.previously)
    )
}

fn render_name_list(names: &[String]) -> String {
    format!("[{}]", names.join(", "))
}

impl Markdown {
    /// Explains how this document differs from a stored hash, at the resolution
    /// the stored kind affords.
    ///
    /// Change is judged like-for-like under the stored hash's ignore-set (see
    /// [`Markdown::compare_hash`]); the current options matter only for surfacing
    /// an [`IgnorePolicyAdvisory`]. For a `detailed` stored hash the body is
    /// aligned section-by-section so each change can be named.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::hash::{MdHashKind, MdHashOptions, StoredHash};
    ///
    /// let original: Markdown = "---\ntitle: T\n---\n# H\n\nBody.".into();
    /// let opts = MdHashOptions::default();
    /// let stored = StoredHash {
    ///     kind: MdHashKind::Simple,
    ///     value: original.compute_hash(MdHashKind::Simple, &opts).to_stored_value(),
    ///     ignored: Vec::new(),
    /// };
    ///
    /// let edited: Markdown = "---\ntitle: T\n---\n# H\n\nNew body.".into();
    /// let explanation = edited.explain_hash_diff(&stored, &opts).unwrap();
    /// assert_eq!(
    ///     explanation.render(),
    ///     "Frontmatter remains unchanged, but body has changed",
    /// );
    /// ```
    ///
    /// ## Errors
    ///
    /// Propagates [`MarkdownError::MalformedStoredHash`] from
    /// [`Markdown::compare_hash`] when a stored value is malformed for its kind.
    ///
    /// [`MarkdownError::MalformedStoredHash`]: crate::markdown::MarkdownError::MalformedStoredHash
    pub fn explain_hash_diff(
        &self,
        stored: &StoredHash,
        options: &MdHashOptions,
    ) -> MarkdownResult<HashExplanation> {
        Ok(self.diff_hash(stored, options)?.1)
    }

    /// The comparison **and** the explanation for one `md hash --diff`, derived
    /// from a single computed artifact.
    ///
    /// [`Self::compare_hash`] and [`Self::explain_hash_diff`] each compute the
    /// like-for-like artifact for themselves, so a caller that needs both — the
    /// explanation to print, the comparison to decide an exit status — hashes
    /// the document twice. This computes it once and returns both products,
    /// which are identical to those of the two-call path.
    ///
    /// Crate-private: compatibility invariant 2 of the 2026-07-15 performance
    /// follow-up bars this feature from adding public API shape, and `md hash`
    /// is the only consumer that needs both products. `darkmatter-cli` reaches
    /// it through the `internal` module, which is `#[doc(hidden)]` and gated
    /// behind the non-default `internal-hash-orchestration` feature, so the
    /// crate's default public surface is unchanged.
    ///
    /// ## Errors
    ///
    /// Propagates [`MarkdownError::MalformedStoredHash`] from
    /// [`Self::compare_hash`] when a stored value is malformed for its kind.
    ///
    /// [`MarkdownError::MalformedStoredHash`]: crate::markdown::MarkdownError::MalformedStoredHash
    pub(crate) fn diff_hash(
        &self,
        stored: &StoredHash,
        options: &MdHashOptions,
    ) -> MarkdownResult<(HashComparison, HashExplanation)> {
        let computed = self.compute_hash(stored.kind, &compare_options(stored, options));
        self.diff_with_computed(stored, options, &computed)
    }

    /// [`Self::diff_hash`] against an already-computed artifact.
    ///
    /// `computed` **must** be `compute_hash(stored.kind, compare_options(stored,
    /// options))`. `--save` holds exactly that artifact from its planning step
    /// and uses this to explain without recomputing it.
    pub(super) fn diff_with_computed(
        &self,
        stored: &StoredHash,
        options: &MdHashOptions,
        computed: &ComputedHash,
    ) -> MarkdownResult<(HashComparison, HashExplanation)> {
        let comparison = self.compare_with_computed(stored, options, computed)?;

        // The detailed per-section alignment needs the same artifact the
        // comparison used: `detailed_body` is only reachable through
        // `ComparisonDetail::Detailed`, which only arises when
        // `stored.kind == Detailed`, so they are provably the same artifact.
        let body = match comparison.detail {
            ComparisonDetail::Coarse => coarse_body(stored.kind, &comparison),
            ComparisonDetail::Structured {
                fm_keys_changed,
                body_structure_changed,
            } => ExplanationBody::Structured {
                frontmatter: FmConcern::from_signals(comparison.frontmatter_changed, fm_keys_changed),
                body: StructuredBody::from_signals(comparison.body_changed, body_structure_changed),
            },
            ComparisonDetail::Detailed {
                fm_keys_changed, ..
            } => ExplanationBody::Detailed {
                frontmatter: FmConcern::from_signals(comparison.frontmatter_changed, fm_keys_changed),
                body: Self::detailed_body(stored, options, computed)?,
            },
        };

        let explanation = HashExplanation {
            body,
            ignore_policy: comparison.ignore_policy.clone(),
        };
        Ok((comparison, explanation))
    }

    /// Aligns the like-for-like detailed artifact against the stored detailed
    /// value to produce per-section reports.
    ///
    /// `computed` is the caller's already-computed
    /// `compute_hash(Detailed, compare_options(..))`; this no longer recomputes
    /// it, since the caller needed the same artifact for the comparison.
    fn detailed_body(
        stored: &StoredHash,
        options: &MdHashOptions,
        computed: &ComputedHash,
    ) -> MarkdownResult<DetailedBody> {
        let StoredHashValue::Detailed(stored_value) = &stored.value else {
            return Err(malformed(
                &options.property,
                "detailed stored hash carries a flat value",
            ));
        };

        let ComputedHash::Detailed(current) = computed else {
            unreachable!(
                "detailed_body is only reachable via ComparisonDetail::Detailed, which requires a Detailed artifact"
            );
        };

        let preamble = preamble_report(&stored_value.preamble, &current.preamble);
        let (sections, removed) = align_sections(&stored_value.sections, &current.sections);

        Ok(DetailedBody {
            preamble,
            sections,
            removed,
        })
    }
}

/// Builds the coarse body for `fm`, `body`, or `simple` stored kinds.
fn coarse_body(kind: MdHashKind, comparison: &HashComparison) -> ExplanationBody {
    match kind {
        MdHashKind::Fm => ExplanationBody::Fm {
            changed: comparison.frontmatter_changed,
        },
        MdHashKind::Body => ExplanationBody::Body {
            changed: comparison.body_changed,
        },
        // `simple` is the only other kind that reports `Coarse` detail.
        _ => ExplanationBody::Simple {
            frontmatter_changed: comparison.frontmatter_changed,
            body_changed: comparison.body_changed,
        },
    }
}

/// Classifies a preamble change. The stored preamble is only ever a hash, so a
/// removed preamble can be reported only as removed (it has no name to surface).
fn preamble_report(old: &Option<String>, new: &Option<String>) -> Option<PreambleReport> {
    match (old, new) {
        (None, None) => None,
        (None, Some(_)) => Some(PreambleReport::Added),
        (Some(_), None) => Some(PreambleReport::Removed),
        (Some(o), Some(n)) => (o != n).then_some(PreambleReport::Changed),
    }
}

/// Aligns stored sections (`old`) against freshly computed sections (`new`) and
/// classifies each change.
///
/// Alignment follows the spec order: heading match, then content-hash match
/// (renames), then same-level positional pairing (rename-plus-edit), then the
/// leftover surplus on each side becomes added/removed. Returns the surviving
/// and added section reports in new-document order, plus the removed reports in
/// stored order.
fn align_sections(
    old: &[SectionTuple],
    new: &[SectionTuple],
) -> (Vec<SectionReport>, Vec<SectionReport>) {
    let mut new_to_old: Vec<Option<usize>> = vec![None; new.len()];
    let mut old_used = vec![false; old.len()];

    // Pass 1 — anchor on identical heading text.
    for (i, ns) in new.iter().enumerate() {
        if let Some(j) = old
            .iter()
            .enumerate()
            .position(|(j, os)| !old_used[j] && os.heading == ns.heading)
        {
            new_to_old[i] = Some(j);
            old_used[j] = true;
        }
    }

    // Pass 2 — pair identical content (heading changed, content unchanged).
    for (i, ns) in new.iter().enumerate() {
        if new_to_old[i].is_some() {
            continue;
        }
        if let Some(j) = old
            .iter()
            .enumerate()
            .position(|(j, os)| !old_used[j] && os.content_hash == ns.content_hash)
        {
            new_to_old[i] = Some(j);
            old_used[j] = true;
        }
    }

    // Pass 3 — pair remaining sections by corresponding position at the same
    // level (rename-plus-edit: both heading and content changed). A candidate is
    // valid only inside the old-document gap delimited by the new section's
    // already-anchored siblings: the nearest preceding and following sections
    // matched in passes 1–2 fix the gap's old-index bounds. Restricting to that
    // gap stops a removal before an anchor from being paired with an addition
    // after it merely because both share a heading level (spec.md:263-268, the
    // "same relative position at the same level" tie-break in spec.md:280-286).
    let anchored = new_to_old.clone();
    for i in 0..new.len() {
        if new_to_old[i].is_some() {
            continue;
        }
        let level = new[i].level;
        let lo = (0..i)
            .rev()
            .find_map(|p| anchored[p])
            .map_or(0, |old_idx| old_idx + 1);
        let hi = ((i + 1)..new.len())
            .find_map(|p| anchored[p])
            .unwrap_or(old.len());
        if let Some(j) = (lo..hi).find(|&j| !old_used[j] && old[j].level == level) {
            new_to_old[i] = Some(j);
            old_used[j] = true;
        }
    }

    let old_parents = parent_headings(old);
    let new_parents = parent_headings(new);

    let surviving = new
        .iter()
        .enumerate()
        .filter_map(|(i, ns)| match new_to_old[i] {
            Some(j) => classify_pair(old, new, j, i, &old_parents, &new_parents, &new_to_old),
            None => Some(SectionReport::Added {
                heading: ns.heading.clone(),
                level: ns.level,
            }),
        })
        .collect();

    let removed = old
        .iter()
        .enumerate()
        .filter(|&(j, _)| !old_used[j])
        .map(|(j, os)| SectionReport::Removed {
            heading: os.heading.clone(),
            prev: j.checked_sub(1).map(|p| old[p].heading.clone()),
            next: old.get(j + 1).map(|n| n.heading.clone()),
        })
        .collect();

    (surviving, removed)
}

/// Classifies a matched (old, new) section pair, or `None` when it is unchanged.
fn classify_pair(
    old: &[SectionTuple],
    new: &[SectionTuple],
    j: usize,
    i: usize,
    old_parents: &[Option<usize>],
    new_parents: &[Option<usize>],
    new_to_old: &[Option<usize>],
) -> Option<SectionReport> {
    let (os, ns) = (&old[j], &new[i]);
    let heading_changed = os.heading != ns.heading;
    let content_changed = os.content_hash != ns.content_hash;

    if heading_changed {
        return Some(if content_changed {
            SectionReport::RenamedAndEdited {
                heading: ns.heading.clone(),
            }
        } else {
            SectionReport::Renamed {
                heading: ns.heading.clone(),
            }
        });
    }

    if os.level != ns.level {
        let report = if ns.level < os.level {
            SectionReport::Promoted {
                heading: ns.heading.clone(),
                from: os.level,
                to: ns.level,
                content_changed,
            }
        } else {
            SectionReport::Demoted {
                heading: ns.heading.clone(),
                from: os.level,
                to: ns.level,
                content_changed,
            }
        };
        return Some(report);
    }

    let old_parent = old_parents[j].map(|p| old[p].heading.clone());
    let new_parent = new_parents[i].map(|p| new[p].heading.clone());
    if old_parent != new_parent {
        return Some(SectionReport::Moved {
            heading: ns.heading.clone(),
            new_parent,
            old_parent,
            content_changed,
        });
    }

    if let Some(before) = reorder_target(new, j, i, new_parents, new_to_old) {
        return Some(SectionReport::Reordered {
            heading: ns.heading.clone(),
            before,
            content_changed,
        });
    }

    content_changed.then(|| SectionReport::ContentChanged {
        heading: ns.heading.clone(),
    })
}

/// Finds the nearest following same-parent sibling that the section now precedes
/// but previously followed, naming the section for a `reordered` report.
fn reorder_target(
    new: &[SectionTuple],
    j: usize,
    i: usize,
    new_parents: &[Option<usize>],
    new_to_old: &[Option<usize>],
) -> Option<String> {
    let parent = new_parents[i];
    ((i + 1)..new.len())
        .filter(|&k| new_parents[k] == parent)
        .find_map(|k| match new_to_old[k] {
            // A sibling that used to sit before this section (smaller stored
            // index) but now sits after it has been overtaken: this section moved
            // forward past it.
            Some(ok) if ok < j => Some(new[k].heading.clone()),
            _ => None,
        })
}

/// For each section, the index of its parent — the nearest preceding section
/// with a smaller heading level — or `None` for a top-level section.
fn parent_headings(sections: &[SectionTuple]) -> Vec<Option<usize>> {
    let mut parents = vec![None; sections.len()];
    for i in 0..sections.len() {
        parents[i] = (0..i).rev().find(|&j| sections[j].level < sections[i].level);
    }
    parents
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::hash::{FmHashPair, StoredHashValue};

    fn md(content: &str) -> Markdown {
        content.into()
    }

    /// Stored hash computed from `doc` at `kind` under `opts`.
    fn stored_at(doc: &Markdown, kind: MdHashKind, opts: &MdHashOptions) -> StoredHash {
        StoredHash {
            kind,
            value: doc.compute_hash(kind, opts).to_stored_value(),
            ignored: super::super::compare::normalize_extras(&opts.extra_ignored),
        }
    }

    fn explain(doc: &Markdown, stored: &StoredHash) -> String {
        doc.explain_hash_diff(stored, &MdHashOptions::default())
            .unwrap()
            .render()
    }

    // --- simple ---------------------------------------------------------

    #[test]
    fn simple_reports_each_combination() {
        let base = "---\ntitle: T\n---\n# H\n\nBody.";
        let opts = MdHashOptions::default();
        let stored = stored_at(&md(base), MdHashKind::Simple, &opts);

        assert_eq!(explain(&md(base), &stored), "No semantic changes detected");
        assert_eq!(
            explain(&md("---\ntitle: Changed\n---\n# H\n\nBody."), &stored),
            "Frontmatter has changed, body remains unchanged semantically",
        );
        assert_eq!(
            explain(&md("---\ntitle: T\n---\n# H\n\nNew body."), &stored),
            "Frontmatter remains unchanged, but body has changed",
        );
        assert_eq!(
            explain(&md("---\ntitle: Changed\n---\n# H\n\nNew body."), &stored),
            "Both the Frontmatter and body have changed",
        );
    }

    // --- fm / body ------------------------------------------------------

    #[test]
    fn fm_reports_only_the_frontmatter_concern() {
        let opts = MdHashOptions::default();
        let stored = stored_at(&md("---\ntitle: T\n---\n# H\n\nBody."), MdHashKind::Fm, &opts);

        assert_eq!(
            explain(&md("---\ntitle: T\n---\n# H\n\nWildly different body."), &stored),
            "Frontmatter remains unchanged",
        );
        assert_eq!(
            explain(&md("---\ntitle: Changed\n---\n# H\n\nBody."), &stored),
            "Frontmatter has changed",
        );
    }

    #[test]
    fn body_reports_only_the_body_concern() {
        let opts = MdHashOptions::default();
        let stored = stored_at(&md("---\ntitle: T\n---\n# H\n\nBody."), MdHashKind::Body, &opts);

        assert_eq!(
            explain(&md("---\ntitle: Wildly different\n---\n# H\n\nBody."), &stored),
            "Body remains unchanged",
        );
        assert_eq!(
            explain(&md("---\ntitle: T\n---\n# H\n\nNew body."), &stored),
            "Body has changed",
        );
    }

    // --- structured -----------------------------------------------------

    #[test]
    fn structured_distinguishes_value_change_from_content_change() {
        let opts = MdHashOptions::default();
        let stored = stored_at(
            &md("---\ntitle: T\n---\n# H\n\nBody."),
            MdHashKind::Structured,
            &opts,
        );

        // Frontmatter value changed (same keys); body untouched.
        assert_eq!(
            explain(&md("---\ntitle: Changed\n---\n# H\n\nBody."), &stored),
            "- Frontmatter has changed but no changes to the keys, only to the values\n- No semantic changes to the body",
        );

        // Same headings, edited content under the heading.
        assert_eq!(
            explain(&md("---\ntitle: T\n---\n# H\n\nDifferent prose."), &stored),
            "- Frontmatter has not changed\n- The body has the same structural layout as before, however, there are semantic changes to the contents within the content",
        );
    }

    #[test]
    fn structured_reports_structural_body_change() {
        let opts = MdHashOptions::default();
        let stored = stored_at(&md("# H\n\nBody."), MdHashKind::Structured, &opts);

        assert_eq!(
            explain(&md("# Renamed Heading\n\nBody changed too."), &stored),
            "- Frontmatter has not changed\n- The body has changed both structurally as well as the semantic content",
        );
    }

    // --- detailed -------------------------------------------------------

    #[test]
    fn detailed_unchanged_collapses_to_single_line() {
        let doc = md("---\ntitle: T\n---\nLead.\n\n# Intro\n\nA.\n\n## Setup\n\nB.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&doc, MdHashKind::Detailed, &opts);
        assert_eq!(explain(&doc, &stored), "No semantic changes detected");
    }

    #[test]
    fn detailed_reports_section_content_change() {
        let original = md("# Installation\n\nRun the installer.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Detailed, &opts);

        let edited = md("# Installation\n\nDownload, then run the installer.");
        assert_eq!(
            explain(&edited, &stored),
            "- Frontmatter has not changed\n- There were changes to the document body:\n  - \"Installation\" section: content has changed",
        );
    }

    #[test]
    fn detailed_reports_rename_and_rename_with_edit() {
        let original = md("# Configuration\n\nSettings live here.\n\n# Usage\n\nRun it.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Detailed, &opts);

        // "Configuration" → "Config" with identical content (rename only);
        // "Usage" content edited (content change, heading kept).
        let edited = md("# Config\n\nSettings live here.\n\n# Usage\n\nRun it differently.");
        assert_eq!(
            explain(&edited, &stored),
            "- Frontmatter has not changed\n- There were changes to the document body:\n  - \"Config\" section: heading renamed, content unchanged\n  - \"Usage\" section: content has changed",
        );
    }

    #[test]
    fn detailed_reports_rename_plus_edit() {
        let original = md("# Quick Start\n\nDo the thing.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Detailed, &opts);

        // Both heading and content changed for the single same-level section.
        let edited = md("# Getting Going\n\nDo a different thing entirely.");
        assert_eq!(
            explain(&edited, &stored),
            "- Frontmatter has not changed\n- There were changes to the document body:\n  - \"Getting Going\" section: heading renamed and content has changed",
        );
    }

    #[test]
    fn detailed_reports_preamble_change() {
        let original = md("Old lead-in.\n\n# Intro\n\nA.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Detailed, &opts);

        let edited = md("A brand new lead-in.\n\n# Intro\n\nA.");
        assert_eq!(
            explain(&edited, &stored),
            "- Frontmatter has not changed\n- There were changes to the document body:\n  - The preamble has changed",
        );
    }

    #[test]
    fn detailed_reports_promotion_and_demotion() {
        let original = md("# Top\n\nT.\n\n## Usage\n\nU.\n\n## Appendix\n\nApp.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Detailed, &opts);

        // "Usage" promoted H2 → H1 with edited content; "Appendix" demoted
        // H2 → H3 with unchanged content. Promoting "Usage" out of "Top" shrinks
        // "Top"'s subtree, so its section content changes too (Fix: detailed
        // section content covers the full subtree, not just the prelude).
        let edited = md("# Top\n\nT.\n\n# Usage\n\nU rewritten.\n\n### Appendix\n\nApp.");
        assert_eq!(
            explain(&edited, &stored),
            "- Frontmatter has not changed\n- There were changes to the document body:\n  - \"Top\" section: content has changed\n  - \"Usage\" section: promoted from H2 to H1\n  - \"Appendix\" section: demoted from H2 to H3, content unchanged",
        );
    }

    #[test]
    fn detailed_reports_added_and_removed_sections() {
        let original = md("# Usage\n\nU.\n\n# Changelog\n\nC.\n\n# Examples\n\nE.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Detailed, &opts);

        // "Changelog" removed; "Troubleshooting" added as a child of "Examples".
        // Nesting "Troubleshooting" under "Examples" grows "Examples"'s subtree,
        // so its section content changes too (Fix: detailed section content
        // covers the full subtree, not just the prelude).
        let edited = md("# Usage\n\nU.\n\n# Examples\n\nE.\n\n## Troubleshooting\n\nT.");
        assert_eq!(
            explain(&edited, &stored),
            "- Frontmatter has not changed\n- There were changes to the document body:\n  - \"Examples\" section: content has changed\n  - \"Troubleshooting\" section: added (H2)\n  - \"Changelog\" section was removed (previously between \"Usage\" and \"Examples\")",
        );
    }

    #[test]
    fn detailed_reports_anchored_sibling_remove_plus_add_not_rename() {
        // Regression (review-4): a removed section before an anchor plus an added
        // section after it must NOT be paired as a rename-plus-edit just because
        // both are the same level. Pass 3 may only pair sections at the same
        // *relative position* among already-anchored siblings.
        let original = md("# A\n\na\n\n# B\n\nb");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Detailed, &opts);

        // "B" is anchored (unchanged). Old "A" sits before the anchor; new "C"
        // sits after it — different gaps, so "A" is removed and "C" is added.
        let edited = md("# B\n\nb\n\n# C\n\nc");
        assert_eq!(
            explain(&edited, &stored),
            "- Frontmatter has not changed\n- There were changes to the document body:\n  - \"C\" section: added (H1)\n  - \"A\" section was removed (previously before \"B\")",
        );
    }

    #[test]
    fn detailed_reports_reorder() {
        let original = md("# Usage\n\nU.\n\n# Examples\n\nE.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Detailed, &opts);

        let edited = md("# Examples\n\nE.\n\n# Usage\n\nU.");
        assert_eq!(
            explain(&edited, &stored),
            "- Frontmatter has not changed\n- There were changes to the document body:\n  - \"Examples\" section: reordered, now appears before \"Usage\"",
        );
    }

    #[test]
    fn detailed_reorder_reports_simultaneous_content_change() {
        // A same-heading sibling reordered AND edited must surface both: the
        // placement note plus an explicit content-changed clause, since a move
        // does not imply a content edit.
        let original = md("# Usage\n\nU.\n\n# Examples\n\nOld examples.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Detailed, &opts);

        let edited = md("# Examples\n\nNew examples.\n\n# Usage\n\nU.");
        assert_eq!(
            explain(&edited, &stored),
            "- Frontmatter has not changed\n- There were changes to the document body:\n  - \"Examples\" section: reordered, now appears before \"Usage\", content has changed",
        );
    }

    #[test]
    fn detailed_move_reports_simultaneous_content_change() {
        // A same-heading section moved to a different parent AND edited must
        // surface both the move and the content change. Relocating "Caveats"
        // also shrinks "Usage"'s subtree and grows "Advanced"'s, so those
        // parents register content changes too.
        let original = md("# Usage\n\nU.\n\n## Caveats\n\nOld caveat.\n\n# Advanced\n\nA.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Detailed, &opts);

        let edited = md("# Usage\n\nU.\n\n# Advanced\n\nA.\n\n## Caveats\n\nNew caveat.");
        assert_eq!(
            explain(&edited, &stored),
            "- Frontmatter has not changed\n- There were changes to the document body:\n  - \"Usage\" section: content has changed\n  - \"Advanced\" section: content has changed\n  - \"Caveats\" section: moved beneath \"Advanced\" (previously beneath \"Usage\"), content has changed",
        );
    }

    #[test]
    fn detailed_keys_and_values_change_renders_nested_body() {
        // Mirrors the spec's "Frontmatter keys and values have changed" example.
        let original = md("---\nauthor: A\n---\n# Top\n\nT.\n\n## Usage\n\nU.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Detailed, &opts);

        // A new frontmatter key (keys changed) and "Usage" promoted to H1.
        // Promoting "Usage" out of "Top" shrinks "Top"'s subtree, so its section
        // content changes too (detailed section content covers the full subtree).
        let edited = md("---\nauthor: A\nstatus: draft\n---\n# Top\n\nT.\n\n# Usage\n\nU.");
        assert_eq!(
            explain(&edited, &stored),
            "- Frontmatter keys and values have changed\n- There were changes to the document body:\n  - \"Top\" section: content has changed\n  - \"Usage\" section: promoted from H2 to H1, content unchanged",
        );
    }

    // --- ignore-policy advisory ----------------------------------------

    #[test]
    fn ignore_policy_advisory_is_appended_without_counting_as_change() {
        // Stored ignored nothing; the document carries an unchanged `draft` key.
        let doc = md("---\ntitle: T\ndraft: true\n---\n# H\n\nBody.");
        let stored = stored_at(&doc, MdHashKind::Simple, &MdHashOptions::default());

        let opts = MdHashOptions {
            extra_ignored: vec!["draft".to_string()],
            ..MdHashOptions::default()
        };
        let rendered = doc.explain_hash_diff(&stored, &opts).unwrap().render();
        assert_eq!(
            rendered,
            "No semantic changes detected\nIgnore policy changed: now also ignoring [draft]; previously []",
        );
    }

    // --- malformed detailed --------------------------------------------

    #[test]
    fn detailed_with_flat_value_errors() {
        let doc = md("# H\n\nBody.");
        let stored = StoredHash {
            kind: MdHashKind::Detailed,
            value: StoredHashValue::Flat("not-detailed".to_string()),
            ignored: Vec::new(),
        };
        let err = doc
            .explain_hash_diff(&stored, &MdHashOptions::default())
            .unwrap_err();
        assert!(matches!(
            err,
            crate::markdown::MarkdownError::MalformedStoredHash { .. }
        ));
    }

    #[test]
    fn preamble_added_and_removed_are_distinguished() {
        assert_eq!(
            preamble_report(&None, &Some("aaaa".to_string())),
            Some(PreambleReport::Added),
        );
        assert_eq!(
            preamble_report(&Some("aaaa".to_string()), &None),
            Some(PreambleReport::Removed),
        );
        assert_eq!(
            preamble_report(&Some("aaaa".to_string()), &Some("aaaa".to_string())),
            None,
        );
    }

    #[test]
    fn fm_hash_pair_is_constructible_in_tests() {
        // Touch the re-exported helper type so the import stays meaningful.
        let pair = FmHashPair {
            fm: "0".repeat(16),
            keys: "1".repeat(16),
        };
        assert_eq!(pair.fm.len(), 16);
    }
}

/// Finding 35.5 (review-1) regression coverage for `md hash --diff`.
///
/// The CLI needs two products per `--diff`: the explanation to print and the
/// comparison to drive the exit status. Reaching for `compare_hash` and
/// `explain_hash_diff` computed the same `(stored kind, stored ignore-set)`
/// artifact twice. [`Markdown::diff_hash`] computes it once and returns both.
#[cfg(test)]
mod finding_35_5_diff {
    use super::super::compute::probe;
    use super::*;

    const ALL_KINDS: [MdHashKind; 5] = [
        MdHashKind::Fm,
        MdHashKind::Body,
        MdHashKind::Simple,
        MdHashKind::Structured,
        MdHashKind::Detailed,
    ];

    fn md(content: &str) -> Markdown {
        content.into()
    }

    /// `(original, edited)` pairs spanning unchanged, frontmatter-only,
    /// body-only, both, and structural edits — so every explanation shape and
    /// both exit statuses are exercised.
    fn edit_pairs() -> Vec<(Markdown, Markdown)> {
        let original = "---\ntitle: T\ndraft: true\n---\nLead.\n\n# Intro\n\nA.\n\n## Setup\n\nB.";
        vec![
            (md(original), md(original)),
            (
                md(original),
                md("---\ntitle: Changed\ndraft: true\n---\nLead.\n\n# Intro\n\nA.\n\n## Setup\n\nB."),
            ),
            (
                md(original),
                md("---\ntitle: T\ndraft: true\n---\nLead.\n\n# Intro\n\nA.\n\n## Setup\n\nB rewritten."),
            ),
            (
                md(original),
                md("---\ntitle: Changed\nextra: k\n---\nNew lead.\n\n# Intro\n\nA.\n\n# Setup\n\nB rewritten."),
            ),
        ]
    }

    fn stored_at(doc: &Markdown, kind: MdHashKind, opts: &MdHashOptions) -> StoredHash {
        StoredHash {
            kind,
            value: doc.compute_hash(kind, opts).to_stored_value(),
            ignored: super::super::compare::normalize_extras(&opts.extra_ignored),
        }
    }

    /// The structural bound the spec sets: one `(kind, effective options)`
    /// artifact per `--diff`, computed at most once. Observed through a call
    /// counter rather than a timer — a duplicate that happens to be fast is
    /// still a duplicate.
    #[test]
    fn diff_hash_computes_the_artifact_exactly_once() {
        let opts = MdHashOptions::default();
        for kind in ALL_KINDS {
            for (original, edited) in edit_pairs() {
                let stored = stored_at(&original, kind, &opts);

                let (result, calls) = probe::count_calls(|| edited.diff_hash(&stored, &opts));
                result.expect("diff succeeds");

                assert_eq!(
                    calls, 1,
                    "{kind:?}: --diff must compute its artifact exactly once, computed {calls}",
                );
            }
        }
    }

    /// Sensitivity check for the counter above: the two-call path `diff_hash`
    /// replaces really does compute the artifact twice. Without this, a counter
    /// that silently stopped counting would let the bound test pass vacuously.
    #[test]
    fn the_replaced_two_call_path_computed_it_twice() {
        let opts = MdHashOptions::default();
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let stored = stored_at(&doc, MdHashKind::Simple, &opts);

        let (_, calls) = probe::count_calls(|| {
            let comparison = doc.compare_hash(&stored, &opts).unwrap();
            let explanation = doc.explain_hash_diff(&stored, &opts).unwrap();
            (comparison, explanation)
        });
        assert_eq!(calls, 2, "the pre-fix CLI path hashed the document twice");
    }

    /// Byte-identical products: `diff_hash` must return exactly what the two
    /// public operations return on their own, for every kind and edit shape.
    /// The rendered explanation text itself is pinned to literal expected
    /// strings by the `tests` module above.
    #[test]
    fn diff_hash_matches_the_separate_operations() {
        let opts = MdHashOptions::default();
        for kind in ALL_KINDS {
            for (original, edited) in edit_pairs() {
                let stored = stored_at(&original, kind, &opts);

                let (comparison, explanation) = edited.diff_hash(&stored, &opts).unwrap();

                assert_eq!(
                    comparison,
                    edited.compare_hash(&stored, &opts).unwrap(),
                    "{kind:?}: comparison drifted from compare_hash",
                );
                assert_eq!(
                    explanation.render(),
                    edited.explain_hash_diff(&stored, &opts).unwrap().render(),
                    "{kind:?}: rendered explanation drifted from explain_hash_diff",
                );
            }
        }
    }

    /// The ignore-policy advisory rides on *both* products, not just whichever
    /// one happened to consume it: the explanation used to take the comparison's
    /// advisory by move.
    #[test]
    fn advisory_survives_on_both_products() {
        let doc = md("---\ntitle: T\ndraft: true\n---\n# H\n\nBody.");
        let stored = stored_at(&doc, MdHashKind::Simple, &MdHashOptions::default());
        let opts = MdHashOptions {
            extra_ignored: vec!["draft".to_string()],
            ..MdHashOptions::default()
        };

        let (comparison, explanation) = doc.diff_hash(&stored, &opts).unwrap();
        assert!(comparison.ignore_policy.is_some());
        assert_eq!(
            explanation.render(),
            "No semantic changes detected\nIgnore policy changed: now also ignoring [draft]; previously []",
        );
        assert!(
            !comparison.frontmatter_changed && !comparison.body_changed,
            "an ignore-policy change is never a content change",
        );
    }

    /// A malformed stored value must still fail, and fail before printing —
    /// `diff_hash` derives the comparison first, exactly as the CLI's
    /// compare-then-explain ordering did.
    #[test]
    fn malformed_stored_value_still_errors() {
        let doc = md("# H\n\nBody.");
        let stored = StoredHash {
            kind: MdHashKind::Structured,
            value: StoredHashValue::Flat("aaaa-bbbb".to_string()),
            ignored: Vec::new(),
        };
        let err = doc.diff_hash(&stored, &MdHashOptions::default()).unwrap_err();
        assert!(matches!(
            err,
            crate::markdown::MarkdownError::MalformedStoredHash { .. }
        ));
    }
}

/// Finding 35.5's retained measurement harness.
///
/// This lived in `benches/phase11_evidence.rs` while [`Markdown::diff_hash`] was
/// public. Review-3 found that public method violated the follow-up's
/// compatibility invariant 2; demoting it to `pub(crate)` put it out of reach of
/// a `benches/*` target, which is a separate compilation unit. That is exactly
/// the case `perf_harness` exists for, so the checkpoint moved in-crate rather
/// than the API staying wide to keep a benchmark compiling.
///
/// Both arms are the real code:
///
/// - baseline  — `compare_hash` + `explain_hash_diff`, verbatim the two-call
///   path `run_hash_diff` used before commit `b8ecb88cb`, computing the
///   like-for-like artifact twice;
/// - candidate — `diff_hash`, computing it once and returning both products.
#[cfg(test)]
mod finding_35_5_perf {
    use super::super::compute::probe;
    use super::*;
    use crate::perf_harness::{Harness, fixture_text};
    use std::hint::black_box;

    /// Builds a stored hash of `kind` from `source`, then edits the body so the
    /// diff has real work to explain (an unchanged document short-circuits parts
    /// of the detailed alignment and would flatter both arms equally).
    fn stored_and_edited(source: &str, kind: MdHashKind) -> (Markdown, StoredHash) {
        let options = MdHashOptions::default();
        let original: Markdown = Markdown::from(source);
        let stored = StoredHash {
            kind,
            value: original.compute_hash(kind, &options).to_stored_value(),
            ignored: Vec::new(),
        };
        let edited: Markdown =
            Markdown::from(format!("{source}\n\nAppended paragraph.\n").as_str());
        (edited, stored)
    }

    #[test]
    #[ignore = "measurement harness; run explicitly with DM_PERF_RAW_DIR set"]
    fn f35_5_diff_hash_raw_samples() {
        let Some(harness) = Harness::from_env(100, 1) else {
            return;
        };
        let options = MdHashOptions::default();
        let large = fixture_text("toc_large");
        let small = fixture_text("hash_basic");

        for (fixture_name, source) in [("toc_large", &large), ("hash_basic", &small)] {
            for kind in [MdHashKind::Simple, MdHashKind::Structured, MdHashKind::Detailed] {
                let (edited, stored) = stored_and_edited(source, kind);

                // Equivalence gate: refuse to report a ratio between two paths
                // that do not agree. The candidate must return exactly the
                // comparison and the explanation the two-call baseline produces.
                let baseline_comparison = edited
                    .compare_hash(&stored, &options)
                    .expect("baseline comparison");
                let baseline_explanation = edited
                    .explain_hash_diff(&stored, &options)
                    .expect("baseline explanation");
                let (candidate_comparison, candidate_explanation) =
                    edited.diff_hash(&stored, &options).expect("candidate diff");
                assert_eq!(
                    baseline_comparison, candidate_comparison,
                    "35.5 baseline/candidate must agree on the comparison ({fixture_name}/{kind:?})"
                );
                assert_eq!(
                    baseline_explanation.render(),
                    candidate_explanation.render(),
                    "35.5 baseline/candidate must render an identical explanation ({fixture_name}/{kind:?})"
                );

                // The mechanism, asserted structurally rather than inferred from
                // the timing below: two artifacts against one.
                let (_, baseline_calls) = probe::count_calls(|| {
                    let comparison = edited.compare_hash(&stored, &options).expect("comparison");
                    let explanation = edited
                        .explain_hash_diff(&stored, &options)
                        .expect("explanation");
                    (comparison, explanation)
                });
                let (_, candidate_calls) =
                    probe::count_calls(|| edited.diff_hash(&stored, &options).expect("diff"));
                assert_eq!(baseline_calls, 2, "baseline computes the artifact twice");
                assert_eq!(candidate_calls, 1, "candidate computes it once");

                let kind_name = format!("{kind:?}").to_lowercase();
                harness.interleaved_pair(
                    &format!("f35_5_diff_hash_{fixture_name}_{kind_name}_baseline"),
                    || {
                        let comparison = black_box(&edited)
                            .compare_hash(black_box(&stored), black_box(&options))
                            .expect("comparison");
                        let explanation = black_box(&edited)
                            .explain_hash_diff(black_box(&stored), black_box(&options))
                            .expect("explanation");
                        black_box((comparison, explanation));
                    },
                    &format!("f35_5_diff_hash_{fixture_name}_{kind_name}_candidate"),
                    || {
                        black_box(
                            black_box(&edited)
                                .diff_hash(black_box(&stored), black_box(&options))
                                .expect("diff"),
                        );
                    },
                );
            }
        }
    }
}
