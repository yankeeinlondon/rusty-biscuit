//! The font-resolution chain: which face draws (and measures) a run of text.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::{Arc, Mutex, OnceLock};

use resvg::usvg::fontdb::{Database, FaceInfo, ID, Source, Stretch, Style};

use super::bundled;
use super::matching::best_match;
use super::metadata::{FaceMetadata, SANS_SERIF_CLASS, choose_sans_serif_family, family_class, fold};
use super::query::{FamilyName, FontQuery, GenericFamily};

/// The environment variable naming a family that every diagram label uses.
pub(crate) const MERMAID_FONT_ENV: &str = "MERMAID_FONT";

/// Process-wide guard: the `MERMAID_FONT` warning is logged at most once per
/// process, however many chains are built.
static MERMAID_FONT_WARNED: AtomicBool = AtomicBool::new(false);

/// Weight, style, and stretch: what a fallback order depends on.
type FaceAttributes = (u16, Style, Stretch);

/// The step of the chain that produced a face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolutionStep {
    /// Step 1: the family named by `MERMAID_FONT`.
    MermaidFont,
    /// Step 2: a named family of the query's stack.
    StackName,
    /// Step 2: a generic family of the query's stack, answered by the OS.
    StackGeneric,
    /// Step 3: the OS's sans-serif family.
    OsSansSerif,
    /// Step 4: the installed sans-serif rule.
    InstalledSansSerif,
}

/// Where the chain gets the OS's answer for a generic family.
enum GenericSource {
    /// fontique: fontconfig on Linux, CoreText on macOS, DirectWrite on
    /// Windows.
    Os,
    /// A fixed answer per generic (tests).
    #[cfg_attr(not(test), allow(dead_code))]
    Fixed(Vec<(GenericFamily, String)>),
}

/// A face's identity for the Mermaid cache key (D63, D70).
///
/// Only the faces a key's stacks can draw with (see
/// [`FontChain::cache_identity`]) and the bundled faces are identified by
/// contents; any other file-backed face is identified by file metadata, so a
/// file replaced in place with the same size and modification time is not
/// detected (the residual risk D70 accepts).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum FaceIdentity {
    /// The content hash of the face's font data and the face's index in it.
    /// Two hosts holding the same file at different paths give the same
    /// identity. In-memory installed faces are always identified this way:
    /// their data is already in memory and they have no file metadata.
    Contents { hash: u64, index: u32 },
    /// The face's file, never read. `size` and `modified` are `None` when the
    /// file's metadata cannot be read.
    File {
        path: PathBuf,
        size: Option<u64>,
        modified: Option<SystemTime>,
        index: u32,
    },
}

impl FaceIdentity {
    /// Appends an unambiguous byte encoding, for hashing.
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            FaceIdentity::Contents { hash, index } => {
                out.push(0);
                out.extend(hash.to_le_bytes());
                out.extend(index.to_le_bytes());
            }
            FaceIdentity::File {
                path,
                size,
                modified,
                index,
            } => {
                out.push(1);
                let path = path.as_os_str().as_encoded_bytes();
                out.extend((path.len() as u64).to_le_bytes());
                out.extend(path);
                match size {
                    Some(size) => {
                        out.push(1);
                        out.extend(size.to_le_bytes());
                    }
                    None => out.push(0),
                }
                match modified.map(|time| time.duration_since(UNIX_EPOCH)) {
                    Some(Ok(after)) => {
                        out.push(1);
                        out.extend(after.as_nanos().to_le_bytes());
                    }
                    Some(Err(before)) => {
                        out.push(2);
                        out.extend(before.duration().as_nanos().to_le_bytes());
                    }
                    None => out.push(0),
                }
                out.extend(index.to_le_bytes());
            }
        }
    }
}

/// The faces one query actually reaches: the resolved face and, in order,
/// every face a missing character falls back to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QueryIdentity {
    /// `None` when the chain resolves no face (the text is not drawn).
    pub(crate) resolved: Option<FaceIdentity>,
    /// The fallback order after the resolved face; empty when `resolved` is.
    pub(crate) fallbacks: Vec<FaceIdentity>,
}

impl QueryIdentity {
    fn encode(&self, out: &mut Vec<u8>) {
        match &self.resolved {
            Some(identity) => {
                out.push(1);
                identity.encode(out);
            }
            None => out.push(0),
        }
        out.extend((self.fallbacks.len() as u64).to_le_bytes());
        for identity in &self.fallbacks {
            identity.encode(out);
        }
    }
}

/// A face that can be matched by family name (installed, or bundled Inter).
struct NamedFace {
    id: ID,
    /// Case-folded family names, every language fontdb read.
    folded_names: Vec<String>,
    installed: bool,
}

/// One font-resolution chain, shared by the text measurer and the
/// rasterizer's usvg `FontResolver`, over one font database.
///
/// The database holds, in this order: the installed faces, the bundled Inter
/// Regular and Bold (family `Inter`), and on Linux builds the bundled Noto
/// Emoji. [`database`](Self::database) is the very `Arc` handed to usvg, so a
/// face ID means the same face to the measurer and the rasterizer.
///
/// Resolution ([`resolve`](Self::resolve)) tries, in order:
///
/// 1. the family named by `MERMAID_FONT` (installed or bundled Inter);
/// 2. each family of the query's stack: named families matched ignoring
///    case against installed families and the bundled Inter, generic ones by
///    the OS's answer for that generic;
/// 3. the OS's answer for `sans-serif` (a generic in the stack was already
///    answered at step 2);
/// 4. the installed sans-serif rule of
///    [`choose_sans_serif_family`](super::metadata::choose_sans_serif_family),
///    over installed faces only.
///
/// Within the family a step names, the face is chosen by weight, style, and
/// stretch as fontdb's `query` chooses (CSS font matching); steps 1–3 choose
/// among installed faces and bundled Inter, step 4 among installed faces.
/// Noto Emoji is never matched by name.
///
/// Missing characters fall back in [`fallback_order`](Self::fallback_order).
pub(crate) struct FontChain {
    db: Arc<Database>,
    named: Vec<NamedFace>,
    installed: Vec<ID>,
    last_resort: Vec<ID>,
    /// Folded `MERMAID_FONT`, kept only when some face is in that family.
    mermaid_font: Option<String>,
    generics: GenericSource,
    generic_answers: OnceLock<HashMap<GenericFamily, String>>,
    sans_serif_family: OnceLock<Option<String>>,
    fallback_orders: Mutex<HashMap<FaceAttributes, Arc<[ID]>>>,
    /// Content hash per font file (or in-memory font), each hashed once.
    content_hashes: Mutex<HashMap<FileKey, u64>>,
    /// How many font files' contents have been hashed.
    contents_hashed: AtomicUsize,
    cache_identities: Mutex<HashMap<Vec<FontQuery>, u64>>,
}

/// The process's chain over the host's installed fonts, built on first use;
/// `MERMAID_FONT` is read once, here.
pub(crate) fn default_chain() -> &'static Arc<FontChain> {
    static CHAIN: OnceLock<Arc<FontChain>> = OnceLock::new();
    CHAIN.get_or_init(|| {
        let mut db = Database::new();
        db.load_system_fonts();
        let mermaid_font = std::env::var(MERMAID_FONT_ENV).ok();
        Arc::new(FontChain::build(db, GenericSource::Os, mermaid_font.as_deref()))
    })
}

impl FontChain {
    /// A chain whose installed faces are `installed` (whole font files, in
    /// memory), whose OS generic answers are `generic_answers` (no fontique),
    /// and whose `MERMAID_FONT` is `mermaid_font`. The bundled faces are
    /// registered as in the default chain.
    #[cfg(test)]
    pub(crate) fn injected(
        installed: &[&[u8]],
        generic_answers: &[(GenericFamily, &str)],
        mermaid_font: Option<&str>,
    ) -> Self {
        let mut db = Database::new();
        for bytes in installed {
            db.load_font_source(Source::Binary(Arc::new(bytes.to_vec())));
        }
        FontChain::injected_db(db, generic_answers, mermaid_font)
    }

    /// [`injected`](Self::injected), with the installed faces loaded from the
    /// font files at `installed` (file-backed faces, as on a real host).
    #[cfg(test)]
    pub(crate) fn injected_files(
        installed: &[&Path],
        generic_answers: &[(GenericFamily, &str)],
        mermaid_font: Option<&str>,
    ) -> Self {
        let mut db = Database::new();
        for path in installed {
            db.load_font_file(path).expect("an injected font file loads");
        }
        FontChain::injected_db(db, generic_answers, mermaid_font)
    }

    #[cfg(test)]
    fn injected_db(
        db: Database,
        generic_answers: &[(GenericFamily, &str)],
        mermaid_font: Option<&str>,
    ) -> Self {
        let generics = GenericSource::Fixed(
            generic_answers
                .iter()
                .map(|(generic, name)| (*generic, (*name).to_string()))
                .collect(),
        );
        FontChain::build(db, generics, mermaid_font)
    }

    fn build(mut db: Database, generics: GenericSource, mermaid_font: Option<&str>) -> Self {
        let installed: Vec<ID> = db.faces().map(|face| face.id).collect();
        let mut bundled_named = Vec::new();
        for bytes in [bundled::INTER_REGULAR, bundled::INTER_BOLD] {
            bundled_named.extend(db.load_font_source(Source::Binary(Arc::new(bytes))));
        }
        #[allow(unused_mut)]
        let mut last_resort: Vec<ID> = Vec::new();
        #[cfg(target_os = "linux")]
        last_resort.extend(db.load_font_source(Source::Binary(Arc::new(bundled::NOTO_EMOJI))));

        let named = installed
            .iter()
            .map(|id| (*id, true))
            .chain(bundled_named.iter().map(|id| (*id, false)))
            .filter_map(|(id, installed)| {
                let face = db.face(id)?;
                Some(NamedFace {
                    id,
                    folded_names: face.families.iter().map(|(name, _)| fold(name)).collect(),
                    installed,
                })
            })
            .collect();

        let mut chain = FontChain {
            db: Arc::new(db),
            named,
            installed,
            last_resort,
            mermaid_font: None,
            generics,
            generic_answers: OnceLock::new(),
            sans_serif_family: OnceLock::new(),
            fallback_orders: Mutex::new(HashMap::new()),
            content_hashes: Mutex::new(HashMap::new()),
            contents_hashed: AtomicUsize::new(0),
            cache_identities: Mutex::new(HashMap::new()),
        };
        chain.mermaid_font = chain.check_mermaid_font(mermaid_font);
        chain
    }

    /// Step 1's family, or `None` (silently when unset or empty; with the
    /// once-per-process warning when no face is in that family).
    fn check_mermaid_font(&self, value: Option<&str>) -> Option<String> {
        let name = value.map(str::trim).filter(|name| !name.is_empty())?;
        let folded = fold(name);
        if self.named.iter().any(|face| face.folded_names.contains(&folded)) {
            return Some(folded);
        }
        if !MERMAID_FONT_WARNED.swap(true, Ordering::Relaxed) {
            tracing::warn!(
                family = name,
                "{MERMAID_FONT_ENV} names a font family that is neither installed nor bundled; ignoring it"
            );
        }
        None
    }

    /// The font database the chain resolves in; hand this `Arc` to usvg.
    pub(crate) fn database(&self) -> &Arc<Database> {
        &self.db
    }

    /// The face for `query`, or `None` when no step yields one (no installed
    /// face qualifies and nothing names the bundled Inter).
    pub(crate) fn resolve(&self, query: &FontQuery) -> Option<ID> {
        self.resolve_with_step(query).map(|(id, _)| id)
    }

    /// [`resolve`](Self::resolve), also naming the step that answered.
    pub(crate) fn resolve_with_step(&self, query: &FontQuery) -> Option<(ID, ResolutionStep)> {
        if let Some(family) = &self.mermaid_font
            && let Some(id) = self.match_family(family, query, false)
        {
            return Some((id, ResolutionStep::MermaidFont));
        }
        for family in &query.families {
            let found = match family {
                FamilyName::Named(name) => self
                    .match_family(&fold(name), query, false)
                    .map(|id| (id, ResolutionStep::StackName)),
                FamilyName::Generic(generic) => self
                    .generic_answer(*generic)
                    .and_then(|name| self.match_family(&name, query, false))
                    .map(|id| (id, ResolutionStep::StackGeneric)),
            };
            if found.is_some() {
                return found;
            }
        }
        if let Some(id) = self
            .generic_answer(GenericFamily::SansSerif)
            .and_then(|name| self.match_family(&name, query, false))
        {
            return Some((id, ResolutionStep::OsSansSerif));
        }
        self.installed_sans_serif_family()
            .and_then(|name| self.match_family(&name, query, true))
            .map(|id| (id, ResolutionStep::InstalledSansSerif))
    }

    /// The face of the family `folded` that CSS matching picks for `query`,
    /// among installed faces (and the bundled Inter unless `installed_only`).
    fn match_family(&self, folded: &str, query: &FontQuery, installed_only: bool) -> Option<ID> {
        let candidates: Vec<&FaceInfo> = self
            .named
            .iter()
            .filter(|face| face.installed || !installed_only)
            .filter(|face| face.folded_names.iter().any(|name| name == folded))
            .filter_map(|face| self.db.face(face.id))
            .collect();
        best_match(&candidates, query.weight, query.style, query.stretch).map(|i| candidates[i].id)
    }

    /// The OS's family for `generic`, case-folded.
    fn generic_answer(&self, generic: GenericFamily) -> Option<String> {
        self.generic_answers
            .get_or_init(|| match &self.generics {
                GenericSource::Os => self.os_generic_answers(),
                GenericSource::Fixed(answers) => answers
                    .iter()
                    .map(|(generic, name)| (*generic, fold(name)))
                    .collect(),
            })
            .get(&generic)
            .cloned()
    }

    /// Asks fontique for each generic family, then maps fontique's first face
    /// that is also an installed face in this database (same file and index)
    /// to its fontdb family name. A fontique family no installed face matches
    /// by file is matched by name instead.
    fn os_generic_answers(&self) -> HashMap<GenericFamily, String> {
        let mut collection = fontique::Collection::new(fontique::CollectionOptions {
            shared: false,
            system_fonts: true,
        });
        let mut answers = HashMap::new();
        for generic in GenericFamily::ALL {
            let fontique_generic = match generic {
                GenericFamily::Serif => fontique::GenericFamily::Serif,
                GenericFamily::SansSerif => fontique::GenericFamily::SansSerif,
                GenericFamily::Monospace => fontique::GenericFamily::Monospace,
                GenericFamily::Cursive => fontique::GenericFamily::Cursive,
                GenericFamily::Fantasy => fontique::GenericFamily::Fantasy,
            };
            let family_ids: Vec<_> = collection.generic_families(fontique_generic).collect();
            let mut by_name = None;
            for family_id in family_ids {
                let Some(family) = collection.family(family_id) else {
                    continue;
                };
                let Some(font) = family.match_font(
                    fontique::FontWidth::NORMAL,
                    fontique::FontStyle::Normal,
                    fontique::FontWeight::NORMAL,
                    false,
                ) else {
                    continue;
                };
                if let fontique::SourceKind::Path(path) = font.source().kind()
                    && let Some(name) = self.installed_family_at(path, font.index())
                {
                    answers.insert(generic, name);
                    by_name = None;
                    break;
                }
                by_name.get_or_insert_with(|| fold(family.name()));
            }
            if let Some(name) = by_name {
                answers.insert(generic, name);
            }
        }
        tracing::debug!(?answers, "fontique generic families");
        answers
    }

    /// The folded first family name of the installed face at `path`/`index`.
    fn installed_family_at(&self, path: &Path, index: u32) -> Option<String> {
        let at = |same: &dyn Fn(&Path) -> bool| {
            self.installed.iter().find_map(|id| {
                let face = self.db.face(*id)?;
                let face_path = source_path(&face.source)?;
                (face.index == index && same(face_path))
                    .then(|| face.families.first().map(|(name, _)| fold(name)))
                    .flatten()
            })
        };
        at(&|face_path| face_path == path).or_else(|| {
            let canonical = std::fs::canonicalize(path).ok()?;
            at(&|face_path| std::fs::canonicalize(face_path).ok().as_ref() == Some(&canonical))
        })
    }

    /// Step 4's family, computed on first need: reading every installed
    /// face's tables is too slow to do for a chain that never gets here.
    fn installed_sans_serif_family(&self) -> Option<String> {
        self.sans_serif_family
            .get_or_init(|| {
                let faces: Vec<FaceMetadata> = self
                    .installed
                    .iter()
                    .filter_map(|id| {
                        let face = self.db.face(*id)?;
                        let family = face.families.first().map(|(name, _)| name.as_str())?;
                        self.db
                            .with_face_data(*id, |data, index| {
                                // Only sans-serif faces need their coverage counted.
                                (family_class(data, index) == Some(SANS_SERIF_CLASS))
                                    .then(|| FaceMetadata::read(data, index, family))
                                    .flatten()
                            })
                            .flatten()
                    })
                    .collect();
                choose_sans_serif_family(&faces)
            })
            .clone()
    }

    /// A face, other than those in `already_tried`, that maps `c`: the first
    /// in [`fallback_order`](Self::fallback_order) for the face
    /// `already_tried[0]` (the resolved face, which the caller has tried
    /// first). Same contract as usvg's `FontResolver::select_fallback`.
    pub(crate) fn fallback(&self, c: char, already_tried: &[ID]) -> Option<ID> {
        self.fallback_order(already_tried.first().copied())
            .iter()
            .copied()
            .filter(|id| !already_tried.contains(id))
            .find(|id| self.has_glyph(*id, c))
    }

    /// Every face a missing character can fall back to, in order, for text
    /// whose resolved face is `base`:
    ///
    /// 1. installed faces, by case-folded first family name; within one
    ///    family, first the face CSS matching picks for `base`'s weight,
    ///    style, and stretch, then the rest by PostScript name, face index,
    ///    and file path;
    /// 2. on Linux builds, the bundled Noto Emoji, always last.
    ///
    /// The bundled Inter is not in the list: it is used only when a query
    /// resolves to it. The order depends only on the installed faces and
    /// `base`'s attributes, never on font-loading order. `base` itself stays
    /// in the list; [`fallback`](Self::fallback) skips faces already tried.
    pub(crate) fn fallback_order(&self, base: Option<ID>) -> Arc<[ID]> {
        let (weight, style, stretch) = base
            .and_then(|id| self.db.face(id))
            .map_or((400, Style::Normal, Stretch::Normal), |face| {
                (face.weight.0, face.style, face.stretch)
            });
        let mut orders = self.fallback_orders.lock().unwrap_or_else(|e| e.into_inner());
        orders
            .entry((weight, style, stretch))
            .or_insert_with(|| self.build_fallback_order(weight, style, stretch))
            .clone()
    }

    fn build_fallback_order(&self, weight: u16, style: Style, stretch: Stretch) -> Arc<[ID]> {
        let mut families: Vec<(String, Vec<&FaceInfo>)> = Vec::new();
        for face in self.installed.iter().filter_map(|id| self.db.face(*id)) {
            let folded = face.families.first().map(|(name, _)| fold(name)).unwrap_or_default();
            match families.iter_mut().find(|(name, _)| *name == folded) {
                Some((_, faces)) => faces.push(face),
                None => families.push((folded, vec![face])),
            }
        }
        families.sort_by(|a, b| a.0.cmp(&b.0));
        let mut order = Vec::with_capacity(self.installed.len() + self.last_resort.len());
        for (_, mut faces) in families {
            faces.sort_by(|a, b| {
                (&a.post_script_name, a.index, source_path(&a.source))
                    .cmp(&(&b.post_script_name, b.index, source_path(&b.source)))
            });
            if let Some(best) = best_match(&faces, weight, style, stretch) {
                order.push(faces.remove(best).id);
            }
            order.extend(faces.iter().map(|face| face.id));
        }
        order.extend(&self.last_resort);
        order.into()
    }

    fn has_glyph(&self, id: ID, c: char) -> bool {
        self.db
            .with_face_data(id, |data, index| {
                ttf_parser::Face::parse(data, index)
                    .ok()
                    .and_then(|face| face.glyph_index(c))
                    .is_some()
            })
            .unwrap_or(false)
    }

    /// The font part of the Mermaid cache key for a design whose font stacks
    /// are `stacks`: the [`font_set_hash`](Self::font_set_hash) combined with
    /// the [`query_identities`](Self::query_identities) of `stacks`.
    ///
    /// Memoized per chain and stack list, so for the default chain each
    /// distinct `stacks` is computed once per process; font files' metadata
    /// is read at that time and not again.
    pub(crate) fn cache_identity(&self, stacks: &[FontQuery]) -> u64 {
        let mut identities = self.cache_identities.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(identity) = identities.get(stacks) {
            return *identity;
        }
        let started = std::time::Instant::now();
        let contents = self.content_faces(stacks);
        let mut bytes = self.font_set_hash_over(&contents).to_le_bytes().to_vec();
        for identity in self.query_identities_over(stacks, &contents) {
            identity.encode(&mut bytes);
        }
        let identity = biscuit_hash::xx_hash_bytes(&bytes);
        tracing::debug!(
            stacks = stacks.len(),
            files_hashed = self.contents_hashed.load(Ordering::Relaxed),
            elapsed_ms = started.elapsed().as_millis() as u64,
            "font cache identity"
        );
        identities.insert(stacks.to_vec(), identity);
        identity
    }

    /// For each of `stacks`, the identity of its resolved face and of every
    /// face in its fallback order, so a cache key changes when `MERMAID_FONT`,
    /// the installed fonts, or the fallback order change which faces are used.
    /// Faces are identified as in [`FaceIdentity`], with `stacks` deciding
    /// which faces are hashed by contents.
    #[cfg(test)]
    pub(crate) fn query_identities(&self, stacks: &[FontQuery]) -> Vec<QueryIdentity> {
        self.query_identities_over(stacks, &self.content_faces(stacks))
    }

    fn query_identities_over(&self, stacks: &[FontQuery], contents: &HashSet<ID>) -> Vec<QueryIdentity> {
        stacks
            .iter()
            .map(|query| match self.resolve(query) {
                Some(id) => QueryIdentity {
                    resolved: self.face_identity(id, contents),
                    fallbacks: self
                        .fallback_order(Some(id))
                        .iter()
                        .filter(|fallback| **fallback != id)
                        .filter_map(|fallback| self.face_identity(*fallback, contents))
                        .collect(),
                },
                None => QueryIdentity {
                    resolved: None,
                    fallbacks: Vec::new(),
                },
            })
            .collect()
    }

    /// One hash over the [`FaceIdentity`] of every face in the database
    /// (installed and bundled), independent of loading order, with `stacks`
    /// deciding which faces are hashed by contents. This is the set of faces
    /// fallback can reach, so it also covers a family's own CSS asking for a
    /// font its stack does not name.
    #[cfg(test)]
    pub(crate) fn font_set_hash(&self, stacks: &[FontQuery]) -> u64 {
        self.font_set_hash_over(&self.content_faces(stacks))
    }

    fn font_set_hash_over(&self, contents: &HashSet<ID>) -> u64 {
        let mut entries: Vec<Vec<u8>> = self
            .db
            .faces()
            .filter_map(|face| self.face_identity(face.id, contents))
            .map(|identity| {
                let mut bytes = Vec::new();
                identity.encode(&mut bytes);
                bytes
            })
            .collect();
        entries.sort_unstable();
        entries.dedup();
        biscuit_hash::xx_hash_bytes(&entries.concat())
    }

    /// The faces D70 hashes by contents for `stacks`: the faces the chain
    /// resolves for each stack and for each generic family (at regular and at
    /// every weight, style, and stretch `stacks` ask for), and the bundled
    /// faces.
    fn content_faces(&self, stacks: &[FontQuery]) -> HashSet<ID> {
        let mut attributes: Vec<FaceAttributes> = vec![(400, Style::Normal, Stretch::Normal)];
        for query in stacks {
            let attribute = (query.weight, query.style, query.stretch);
            if !attributes.contains(&attribute) {
                attributes.push(attribute);
            }
        }
        let generics = attributes.iter().flat_map(|(weight, style, stretch)| {
            GenericFamily::ALL.into_iter().map(|generic| {
                FontQuery::new(vec![FamilyName::Generic(generic)])
                    .with_weight(*weight)
                    .with_style(*style)
                    .with_stretch(*stretch)
            })
        });
        let resolved: Vec<ID> = stacks
            .iter()
            .cloned()
            .chain(generics)
            .filter_map(|query| self.resolve(&query))
            .collect();
        self.named
            .iter()
            .filter(|face| !face.installed)
            .map(|face| face.id)
            .chain(self.last_resort.iter().copied())
            .chain(resolved)
            .collect()
    }

    /// The [`FaceIdentity`] of face `id`, hashing its contents when it is in
    /// `contents` or held in memory.
    fn face_identity(&self, id: ID, contents: &HashSet<ID>) -> Option<FaceIdentity> {
        let face = self.db.face(id)?;
        let index = face.index;
        match source_path(&face.source) {
            Some(path) if !contents.contains(&id) => {
                let metadata = std::fs::metadata(path).ok();
                Some(FaceIdentity::File {
                    path: path.to_path_buf(),
                    size: metadata.as_ref().map(std::fs::Metadata::len),
                    modified: metadata.and_then(|metadata| metadata.modified().ok()),
                    index,
                })
            }
            _ => Some(FaceIdentity::Contents {
                hash: self.content_hash(face)?,
                index,
            }),
        }
    }

    /// The content hash of `face`'s font data, hashing each file once per
    /// chain. fontdb memory-maps a file face only while it is read.
    fn content_hash(&self, face: &FaceInfo) -> Option<u64> {
        let key = match &face.source {
            Source::Binary(data) => FileKey::Memory(data_address((**data).as_ref())),
            Source::File(path) | Source::SharedFile(path, _) => FileKey::Path(path.clone()),
        };
        let mut hashes = self.content_hashes.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(hash) = hashes.get(&key) {
            return Some(*hash);
        }
        let hash = self.db.with_face_data(face.id, |data, _| biscuit_hash::xx_hash_bytes(data))?;
        self.contents_hashed.fetch_add(1, Ordering::Relaxed);
        hashes.insert(key, hash);
        Some(hash)
    }

    /// How many font files' (or in-memory fonts') contents this chain has
    /// hashed.
    #[cfg(test)]
    pub(crate) fn contents_hashed(&self) -> usize {
        self.contents_hashed.load(Ordering::Relaxed)
    }

    /// Every family name (each language fontdb read) that resolution can
    /// match by name: installed families and the bundled `Inter`, sorted and
    /// deduplicated. The bundled Noto Emoji is not matched by name and is
    /// not listed.
    pub(crate) fn family_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .named
            .iter()
            .filter_map(|face| self.db.face(face.id))
            .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    #[cfg(test)]
    pub(crate) fn bundled_ids(&self) -> Vec<ID> {
        self.named
            .iter()
            .filter(|face| !face.installed)
            .map(|face| face.id)
            .chain(self.last_resort.iter().copied())
            .collect()
    }
}

#[derive(PartialEq, Eq, Hash)]
enum FileKey {
    Path(PathBuf),
    Memory(usize),
}

fn data_address(data: &[u8]) -> usize {
    data.as_ptr() as usize
}

fn source_path(source: &Source) -> Option<&Path> {
    match source {
        Source::File(path) | Source::SharedFile(path, _) => Some(path),
        Source::Binary(_) => None,
    }
}
