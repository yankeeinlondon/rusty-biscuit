//! The persistent index store: per-file facts behind references, heading
//! rename, file rename, and workspace symbols.
//!
//! The store **narrows**; the graph **decides**. For each Markdown file a
//! root's SQLite store holds [`FileFacts`] (headings, links, wiki links,
//! transclusions, and [candidate keys](keys)). A request asks the store which
//! files could reference its target ([`IndexStore::candidates_for`]) and
//! builds a small `WorkspaceGraph` over only those; SQL never resolves a link.
//! The store is only a cache: [`RootFacts`] falls back to [`MemoryFacts`]
//! whenever it is unusable, and its directory is always safe to delete.
//!
//! - [`facts`]: the stored row and its invariant
//! - [`keys`]: candidate keys
//! - [`location`]: store directory, file names, mtime touch, and the sweep
//! - [`lock`]: the liveness lock beside each store
//! - [`store`]: [`IndexStore`], the SQLite layout and its reads and writes
//! - [`source`]: [`FactsSource`] and the failure ladder
//! - [`workspace`]: [`WorkspaceFacts`], every root's facts kept equal to
//!   disk (revalidation, ready, live updates, the open-buffer overlay)

pub mod facts;
pub mod failure;
pub mod keys;
pub mod location;
pub mod lock;
pub mod source;
pub mod store;
pub mod workspace;

pub use facts::FileFacts;
pub use failure::StoreFailure;
pub use location::{STALE_STORE_AGE, StoreLocation, SweepReport, lock_path, root_key, store_path, sweep_stale_stores, touch_mtime};
pub use lock::StoreLock;
pub use source::{FactsSource, MemoryFacts, RootFacts, RootOpened, StoreFacts, StoreOpened, View, ViewRead};
pub use store::{IndexStore, OpenReport, UpsertReport};
pub use workspace::{PassReport, WorkspaceFacts};

/// The table layout's version, recorded in each store's `meta` table.
pub const STORE_FORMAT: u32 = 1;

/// The version of what extraction produces. It names the store file, so
/// builds with different versions keep separate stores. Bump it whenever a
/// change to parsing changes any stored fact; the golden facts test fails
/// until it is bumped.
pub const FACTS_VERSION: u32 = 1;
