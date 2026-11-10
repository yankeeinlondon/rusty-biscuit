//! The one grammar for naming a value inside a document's data: a dotted
//! property path such as `style.page.max-width`.
//!
//! A path has one syntactic reading whatever the document holds, and the
//! document decides what its segments mean. [`PropertyPath::parse`] keeps
//! decoded text segments, because `0` is an array index or the mapping key
//! `"0"` depending on the value it meets. [`PropertyPath::resolve`] makes that
//! decision against a value, and only its [`Resolved`] result converts to
//! [`PathStep`] or [`JsonPointer`].
//!
//! [`set_at`] and [`RemovalPlan`] are the pure edits. Each decides success
//! before changing anything, so a failed edit leaves its input untouched.

use std::fmt;
use std::hash::{Hash, Hasher};

use indexmap::IndexMap;
use serde_json::Value;

use crate::coercion::ValueKind;
use crate::completion::PathStep;
use crate::validation::JsonPointer;

/// A frontmatter document's top level.
type TopLevel = IndexMap<String, Value>;

/// A parsed dotted path.
///
/// Equality and hashing compare decoded segments, so `C:\dir` and `C:\\dir`
/// are the same path, and `a\.b` differs from `a.b`. [`Display`](fmt::Display)
/// prints the canonical spelling; [`Self::as_written`] returns the argument.
#[derive(Debug, Clone)]
pub struct PropertyPath {
    written: String,
    segments: Vec<String>,
}

/// Why text is not a [`PropertyPath`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PropertyPathError {
    /// A longer path holds an empty segment (`a.`, `.a`, `a..b`). Only the
    /// whole argument `''` may name an empty key.
    #[error("property path `{argument}` has an empty segment at position {position}")]
    EmptySegment {
        /// The argument as written.
        argument: String,
        /// The zero-based position of the empty segment.
        position: usize,
    },
}

impl PropertyPath {
    /// Parses a path, after the shell has removed its quoting.
    ///
    /// `.` separates. `\.` and `\\` are the two escapes, matched left to right;
    /// a `\` before anything else is itself. Nothing is trimmed or normalized.
    ///
    /// ## Examples
    ///
    /// ```
    /// use simplified_schema::PropertyPath;
    ///
    /// let path = PropertyPath::parse(r"style.a\.b").unwrap();
    /// assert_eq!(path.segments(), ["style", "a.b"]);
    /// assert_eq!(PropertyPath::parse("").unwrap().segments(), [""]);
    /// assert!(PropertyPath::parse("a..b").is_err());
    /// ```
    ///
    /// ## Errors
    ///
    /// [`PropertyPathError::EmptySegment`] for an empty segment in a longer path.
    pub fn parse(argument: &str) -> Result<Self, PropertyPathError> {
        let mut segments = Vec::new();
        let mut segment = String::new();
        let mut characters = argument.chars().peekable();
        while let Some(character) = characters.next() {
            match character {
                '\\' => match characters.peek() {
                    Some(&escaped @ ('.' | '\\')) => {
                        segment.push(escaped);
                        characters.next();
                    }
                    _ => segment.push('\\'),
                },
                '.' => segments.push(std::mem::take(&mut segment)),
                other => segment.push(other),
            }
        }
        segments.push(segment);
        if segments.len() > 1
            && let Some(position) = segments.iter().position(String::is_empty)
        {
            return Err(PropertyPathError::EmptySegment {
                argument: argument.to_string(),
                position,
            });
        }
        Ok(Self {
            written: argument.to_string(),
            segments,
        })
    }

    /// The path for decoded segments, whose written form is the canonical one.
    ///
    /// ## Errors
    ///
    /// [`PropertyPathError::EmptySegment`] for segments no path can spell: none
    /// at all, or an empty one among several.
    pub fn from_segments<S: Into<String>>(segments: impl IntoIterator<Item = S>) -> Result<Self, PropertyPathError> {
        let segments: Vec<String> = segments.into_iter().map(Into::into).collect();
        let written = canonical(&segments);
        if segments.is_empty() {
            return Err(PropertyPathError::EmptySegment {
                argument: written,
                position: 0,
            });
        }
        Self::parse(&written)
    }

    /// The argument exactly as it was written, for result fields and messages.
    pub fn as_written(&self) -> &str {
        &self.written
    }

    /// The decoded segments, outermost first. Never empty.
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Walks `root` as far as this path reaches.
    ///
    /// ## Examples
    ///
    /// ```
    /// use serde_json::json;
    /// use simplified_schema::PropertyPath;
    ///
    /// let root = json!({ "items": [{ "name": "a" }, { "name": "b" }] });
    /// let path = PropertyPath::parse("items.1.name").unwrap();
    /// let resolved = path.resolve(&root);
    /// assert_eq!(resolved.value(), Some(&json!("b")));
    /// assert_eq!(resolved.pointer().as_pointer_string(), "/items/1/name");
    /// ```
    pub fn resolve<'a>(&'a self, root: &'a Value) -> Resolved<'a> {
        walk(root, &self.segments, Vec::with_capacity(self.segments.len()))
    }

    /// [`Self::resolve`] from a frontmatter map's top level.
    pub fn resolve_map<'a>(&'a self, root: &'a IndexMap<String, Value>) -> Resolved<'a> {
        let (first, rest) = self.split_first();
        match root.get(first) {
            Some(value) => {
                let mut steps = Vec::with_capacity(self.segments.len());
                steps.push(PathStep::Key(first));
                walk(value, rest, steps)
            }
            None => Resolved {
                steps: Vec::new(),
                stop: ResolveStop::MissingKey,
                unresolved: &self.segments,
            },
        }
    }

    fn split_first(&self) -> (&str, &[String]) {
        // `parse` always produces at least one segment.
        let (first, rest) = self.segments.split_first().expect("a path has a segment");
        (first, rest)
    }
}

impl PartialEq for PropertyPath {
    fn eq(&self, other: &Self) -> bool {
        self.segments == other.segments
    }
}

impl Eq for PropertyPath {}

impl Hash for PropertyPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.segments.hash(state);
    }
}

impl fmt::Display for PropertyPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&canonical(&self.segments))
    }
}

fn canonical(segments: &[impl AsRef<str>]) -> String {
    segments
        .iter()
        .map(|segment| segment.as_ref().replace('\\', r"\\").replace('.', r"\."))
        .collect::<Vec<_>>()
        .join(".")
}

/// Where a [`PropertyPath::resolve`] walk ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResolveStop<'a> {
    /// Every segment was consumed, ending on this value.
    Found(&'a Value),
    /// A mapping does not hold the next segment.
    MissingKey,
    /// An array has no element at the next segment: it is out of range, or not
    /// a canonical index at all.
    OutOfRange {
        /// The array's length.
        len: usize,
    },
    /// A scalar or `null` stands where the next segment needs a container.
    NotContainer {
        /// The value that cannot be walked into.
        found: ValueKind,
    },
}

/// How far a path reached into a value.
///
/// The typed steps are the conversion a path's text cannot make alone: each
/// says whether its segment met an array or a mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved<'a> {
    steps: Vec<PathStep<'a>>,
    stop: ResolveStop<'a>,
    unresolved: &'a [String],
}

impl<'a> Resolved<'a> {
    /// The value at the path, when every segment was consumed.
    pub fn value(&self) -> Option<&'a Value> {
        match self.stop {
            ResolveStop::Found(value) => Some(value),
            _ => None,
        }
    }

    /// Where the walk ended.
    pub fn stop(&self) -> ResolveStop<'a> {
        self.stop
    }

    /// The steps taken before the walk ended.
    pub fn steps(&self) -> &[PathStep<'a>] {
        &self.steps
    }

    /// The segments the walk did not consume; the first is the one it stopped on.
    pub fn unresolved(&self) -> &'a [String] {
        self.unresolved
    }

    /// The steps taken, as a JSON Pointer.
    pub fn pointer(&self) -> JsonPointer {
        let mut pointer = String::new();
        for step in &self.steps {
            pointer.push('/');
            match step {
                PathStep::Key(key) => pointer.push_str(&key.replace('~', "~0").replace('/', "~1")),
                PathStep::Index(index) => pointer.push_str(&index.to_string()),
            }
        }
        JsonPointer::parse(&pointer)
    }
}

fn walk<'a>(root: &'a Value, segments: &'a [String], mut steps: Vec<PathStep<'a>>) -> Resolved<'a> {
    let mut current = root;
    for (position, segment) in segments.iter().enumerate() {
        let stopped = |steps, stop| Resolved {
            steps,
            stop,
            unresolved: &segments[position..],
        };
        match current {
            Value::Object(map) => match map.get(segment) {
                Some(next) => {
                    steps.push(PathStep::Key(segment));
                    current = next;
                }
                None => return stopped(steps, ResolveStop::MissingKey),
            },
            Value::Array(items) => match canonical_index(segment).and_then(|index| Some((index, items.get(index)?))) {
                Some((index, next)) => {
                    steps.push(PathStep::Index(index));
                    current = next;
                }
                None => return stopped(steps, ResolveStop::OutOfRange { len: items.len() }),
            },
            scalar => {
                let found = ValueKind::of(scalar);
                return stopped(steps, ResolveStop::NotContainer { found });
            }
        }
    }
    Resolved {
        steps,
        stop: ResolveStop::Found(current),
        unresolved: &[],
    }
}

/// The index a segment denotes on an array: canonical decimal digits only, so
/// one element has one spelling. A value too large for `usize` is `None`, which
/// callers read as out of range.
fn canonical_index(segment: &str) -> Option<usize> {
    let digits = !segment.is_empty() && segment.bytes().all(|byte| byte.is_ascii_digit());
    let canonical = digits && (segment == "0" || !segment.starts_with('0'));
    canonical.then(|| segment.parse().ok()).flatten()
}

fn all_digits(segment: &str) -> bool {
    !segment.is_empty() && segment.bytes().all(|byte| byte.is_ascii_digit())
}

/// Why a path edit cannot be made. `path` names where the edit failed, in
/// canonical spelling; the whole path can be longer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathEditError {
    /// The parent is a scalar or `null`, which cannot hold a child.
    #[error("`{path}` is {found}, so it cannot hold a child")]
    ParentNotContainer {
        /// The parent's path.
        path: String,
        /// What the parent is.
        found: ValueKind,
    },
    /// An array has no element at this segment.
    #[error("`{path}` is out of range: the array has {len} element(s)")]
    IndexOutOfRange {
        /// The path through the segment that failed.
        path: String,
        /// The array's length.
        len: usize,
    },
    /// A number under a parent that does not exist cannot say whether it means
    /// an element or a key, even when a schema suggests one.
    #[error("`{path}` is a number under a parent that does not exist, so it could be an array index or a key")]
    NumberUnderMissingParent {
        /// The path through the number.
        path: String,
    },
    /// The path names nothing.
    #[error("`{path}` does not exist")]
    Missing {
        /// The path through the missing segment.
        path: String,
    },
    /// Two removals name one target, or one contains the other.
    #[error("cannot remove both `{first}` and `{second}`: they name the same value or one contains the other")]
    RemovalConflict {
        /// The earlier path, as written.
        first: String,
        /// The later path, as written.
        second: String,
    },
}

/// The canonical spelling of `segments[..=through]`.
fn path_through(segments: &[String], through: usize) -> String {
    canonical(&segments[..=through])
}

/// Sets the value at `path`, creating missing mapping parents.
///
/// Naming a whole `null` or scalar replaces it. A replacement keeps its key's
/// position, and a new key appends to its parent. On an error nothing changed.
///
/// ## Examples
///
/// ```
/// use indexmap::IndexMap;
/// use serde_json::json;
/// use simplified_schema::{PropertyPath, set_at};
///
/// let mut frontmatter = IndexMap::new();
/// set_at(&mut frontmatter, &PropertyPath::parse("author.name").unwrap(), json!("Ken")).unwrap();
/// assert_eq!(frontmatter["author"], json!({ "name": "Ken" }));
/// ```
///
/// ## Errors
///
/// A [`PathEditError`] naming the parent or index that failed.
pub fn set_at(root: &mut IndexMap<String, Value>, path: &PropertyPath, value: Value) -> Result<(), PathEditError> {
    let segments = path.segments();
    let (first, rest) = path.split_first();
    if rest.is_empty() {
        root.insert(first.to_string(), value);
        return Ok(());
    }
    match root.get_mut(first) {
        Some(existing) => set_in(existing, segments, 1, value),
        None => {
            let created = nested_value(segments, 1, value)?;
            root.insert(first.to_string(), created);
            Ok(())
        }
    }
}

/// Sets `segments[position..]` inside `current`, whose own path is
/// `segments[..position]`.
fn set_in(current: &mut Value, segments: &[String], position: usize, value: Value) -> Result<(), PathEditError> {
    let segment = &segments[position];
    let is_last = position + 1 == segments.len();
    match current {
        Value::Object(map) if is_last => {
            map.insert(segment.clone(), value);
            Ok(())
        }
        Value::Object(map) => match map.get_mut(segment) {
            Some(next) => set_in(next, segments, position + 1, value),
            None => {
                let created = nested_value(segments, position + 1, value)?;
                map.insert(segment.clone(), created);
                Ok(())
            }
        },
        Value::Array(items) => {
            let len = items.len();
            let Some(element) = canonical_index(segment).and_then(|index| items.get_mut(index)) else {
                return Err(PathEditError::IndexOutOfRange {
                    path: path_through(segments, position),
                    len,
                });
            };
            if is_last {
                *element = value;
                Ok(())
            } else {
                set_in(element, segments, position + 1, value)
            }
        }
        scalar => Err(PathEditError::ParentNotContainer {
            path: path_through(segments, position - 1),
            found: ValueKind::of(scalar),
        }),
    }
}

/// The nested mappings for `segments[from..]`, which sit under a key that does
/// not exist yet. Built whole before insertion, so a refusal changes nothing.
fn nested_value(segments: &[String], from: usize, value: Value) -> Result<Value, PathEditError> {
    if let Some(offset) = segments[from..].iter().position(|segment| all_digits(segment)) {
        return Err(PathEditError::NumberUnderMissingParent {
            path: path_through(segments, from + offset),
        });
    }
    Ok(segments[from..].iter().rev().fold(value, |child, segment| {
        let mut parent = serde_json::Map::new();
        parent.insert(segment.clone(), child);
        Value::Object(parent)
    }))
}

/// One step of a planned removal.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OwnedStep {
    Key(String),
    Index(usize),
}

/// A set of removals checked against one unedited document.
///
/// Every path resolves against the original, so array indexes are original
/// indexes: `items.0 items.2` on `[a,b,c,d]` leaves `[b,d]` in either argument
/// order. Removal leaves emptied parents in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalPlan {
    targets: Vec<Vec<OwnedStep>>,
}

impl RemovalPlan {
    /// Plans removing `paths` from `root`. Nothing changes until [`Self::apply`].
    ///
    /// ## Errors
    ///
    /// A [`PathEditError`] for a target that does not exist, or
    /// [`PathEditError::RemovalConflict`] when two paths resolve to one target
    /// or one contains the other. Paths compare as resolved, so a conflict needs
    /// no matching spelling.
    pub fn new(root: &TopLevel, paths: &[PropertyPath]) -> Result<Self, PathEditError> {
        let mut targets: Vec<Vec<OwnedStep>> = Vec::with_capacity(paths.len());
        for (position, path) in paths.iter().enumerate() {
            let target = removal_target(root, path)?;
            for (earlier, other) in targets.iter().enumerate() {
                let shared = target.len().min(other.len());
                if target[..shared] == other[..shared] {
                    return Err(PathEditError::RemovalConflict {
                        first: paths[earlier].as_written().to_string(),
                        second: paths[position].as_written().to_string(),
                    });
                }
            }
            targets.push(target);
        }
        Ok(Self { targets })
    }

    /// The planned targets as typed steps, in the order the paths were given.
    pub fn targets(&self) -> impl Iterator<Item = Vec<PathStep<'_>>> {
        self.targets.iter().map(|target| {
            target
                .iter()
                .map(|step| match step {
                    OwnedStep::Key(key) => PathStep::Key(key),
                    OwnedStep::Index(index) => PathStep::Index(*index),
                })
                .collect()
        })
    }

    /// Removes every target from `root`, which must be the document the plan
    /// was made against. Surviving mapping keys keep their order.
    pub fn apply(&self, root: &mut IndexMap<String, Value>) {
        let mut ordered: Vec<&[OwnedStep]> = self.targets.iter().map(Vec::as_slice).collect();
        ordered.sort_by(|left, right| removal_order(left, right));
        for target in ordered {
            remove_target(root, target);
        }
    }
}

/// Removes `paths` from `root`, or changes nothing.
///
/// ## Errors
///
/// As [`RemovalPlan::new`].
pub fn remove_at(root: &mut IndexMap<String, Value>, paths: &[PropertyPath]) -> Result<(), PathEditError> {
    RemovalPlan::new(root, paths)?.apply(root);
    Ok(())
}

fn removal_target(root: &TopLevel, path: &PropertyPath) -> Result<Vec<OwnedStep>, PathEditError> {
    let resolved = path.resolve_map(root);
    let reached = resolved.steps().len();
    let segments = path.segments();
    match resolved.stop() {
        ResolveStop::Found(_) => Ok(resolved
            .steps()
            .iter()
            .map(|step| match step {
                PathStep::Key(key) => OwnedStep::Key((*key).to_string()),
                PathStep::Index(index) => OwnedStep::Index(*index),
            })
            .collect()),
        ResolveStop::MissingKey => Err(PathEditError::Missing {
            path: path_through(segments, reached),
        }),
        ResolveStop::OutOfRange { len } => Err(PathEditError::IndexOutOfRange {
            path: path_through(segments, reached),
            len,
        }),
        ResolveStop::NotContainer { found } => Err(PathEditError::ParentNotContainer {
            // A walk stops on a scalar only after stepping onto it.
            path: path_through(segments, reached - 1),
            found,
        }),
    }
}

/// Orders removals so an earlier one never moves a later one's element: higher
/// indexes of one array go first. Other differences only need a consistent order.
fn removal_order(left: &[OwnedStep], right: &[OwnedStep]) -> std::cmp::Ordering {
    let first_difference = left.iter().zip(right).find(|(left_step, right_step)| left_step != right_step);
    match first_difference {
        Some((OwnedStep::Index(left_index), OwnedStep::Index(right_index))) => right_index.cmp(left_index),
        Some((OwnedStep::Key(left_key), OwnedStep::Key(right_key))) => left_key.cmp(right_key),
        Some((OwnedStep::Key(_), OwnedStep::Index(_))) => std::cmp::Ordering::Less,
        Some((OwnedStep::Index(_), OwnedStep::Key(_))) => std::cmp::Ordering::Greater,
        None => left.len().cmp(&right.len()),
    }
}

fn remove_target(root: &mut TopLevel, target: &[OwnedStep]) {
    let Some((OwnedStep::Key(first), rest)) = target.split_first() else {
        return;
    };
    let Some((last, parents)) = rest.split_last() else {
        root.shift_remove(first);
        return;
    };
    let mut current = root.get_mut(first);
    for step in parents {
        current = current.and_then(|value| match (value, step) {
            (Value::Object(map), OwnedStep::Key(key)) => map.get_mut(key),
            (Value::Array(items), OwnedStep::Index(index)) => items.get_mut(*index),
            _ => None,
        });
    }
    match (current, last) {
        (Some(Value::Object(map)), OwnedStep::Key(key)) => {
            // `Map::remove` is a swap under `preserve_order`, which would move
            // the last sibling into the gap.
            map.shift_remove(key);
        }
        (Some(Value::Array(items)), OwnedStep::Index(index)) if *index < items.len() => {
            items.remove(*index);
        }
        _ => {}
    }
}
