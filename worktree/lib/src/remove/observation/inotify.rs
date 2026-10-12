//! The Linux adapter: one inotify instance watching the parent and the admin
//! directory non-recursively and every directory of the tree.
//!
//! - Watch descriptors follow inodes, so the tree's watches keep reporting
//!   under the same tree-relative paths after the move; relabeling needs no
//!   new watch and leaves no gap.
//! - A drain reads until `EAGAIN` on the calling thread; there is no delivery
//!   thread. inotify queues an event inside the system call that causes it, so
//!   a change whose call returned before the drain started is already queued.
//! - The kernel queue (`max_queued_events`) bounds what waits between drains;
//!   `IN_Q_OVERFLOW` is coverage loss. So is an `IN_IGNORED` the operation did
//!   not cause: a tree directory's watch may end only after its own
//!   `IN_DELETE_SELF`, and the parent's never does.
//! - A directory created in the tree after setup is not watched. Its creation
//!   is reported by its parent, and new content is already a reason to stop.

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::io;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use inotify::{EventMask, WatchDescriptor, WatchMask};
use sniff::filesystem::query::EvidenceKind;

use super::{Backend, CoverageLost, EventKind, ObservationError, RawEvent, Root, Sink, Targets};

const BUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone)]
struct Watch {
    root: Root,
    /// Relative to the root; empty for the root itself.
    path: PathBuf,
    descriptor: WatchDescriptor,
}

pub(super) struct Inotify {
    inotify: inotify::Inotify,
    watches: HashMap<i32, Watch>,
    /// Watches whose directory reported `IN_DELETE_SELF`, or that the session
    /// removed itself; their `IN_IGNORED` is expected.
    ending: HashSet<i32>,
    admin_released: bool,
    buffer: Vec<u8>,
}

fn setup(path: &Path) -> impl FnOnce(io::Error) -> ObservationError + use<> {
    let path = path.to_path_buf();
    move |source| ObservationError::Setup { path, source }
}

fn parent_mask() -> WatchMask {
    WatchMask::CREATE
        | WatchMask::DELETE
        | WatchMask::ATTRIB
        | WatchMask::MOVED_FROM
        | WatchMask::MOVED_TO
        | WatchMask::DELETE_SELF
        | WatchMask::MOVE_SELF
        | WatchMask::DONT_FOLLOW
        | WatchMask::ONLYDIR
}

fn content_mask() -> WatchMask {
    parent_mask() | WatchMask::MODIFY | WatchMask::CLOSE_WRITE
}

impl Inotify {
    pub fn start(targets: &Targets) -> Result<Self, ObservationError> {
        let inotify = inotify::Inotify::init().map_err(setup(targets.original()))?;
        let mut adapter = Self {
            inotify,
            watches: HashMap::new(),
            ending: HashSet::new(),
            admin_released: false,
            buffer: vec![0; BUFFER_BYTES],
        };
        adapter.add(targets.parent(), Root::Parent, PathBuf::new(), parent_mask()).map_err(setup(targets.parent()))?;
        adapter.add(targets.admin(), Root::Admin, PathBuf::new(), content_mask()).map_err(setup(targets.admin()))?;
        adapter.add_tree(targets.original())?;
        Ok(adapter)
    }

    /// Watches each directory before listing it, so a child created while the
    /// walk runs is either listed or reported by its parent's watch.
    fn add_tree(&mut self, tree: &Path) -> Result<(), ObservationError> {
        self.add(tree, Root::Tree, PathBuf::new(), content_mask()).map_err(setup(tree))?;
        let mut pending = vec![PathBuf::new()];
        while let Some(relative) = pending.pop() {
            let directory = tree.join(&relative);
            for entry in std::fs::read_dir(&directory).map_err(setup(&directory))? {
                let entry = entry.map_err(setup(&directory))?;
                // `DirEntry::file_type` does not follow links.
                if !entry.file_type().map_err(setup(&entry.path()))?.is_dir() {
                    continue;
                }
                let child = relative.join(entry.file_name());
                match self.add(&entry.path(), Root::Tree, child.clone(), content_mask()) {
                    Ok(()) => pending.push(child),
                    // Removed or replaced since the listing: the parent's
                    // watch, added before the listing, has queued that.
                    Err(error) if matches!(error.raw_os_error(), Some(libc::ENOENT | libc::ENOTDIR)) => {}
                    Err(error) => return Err(setup(&entry.path())(error)),
                }
            }
        }
        Ok(())
    }

    /// A descriptor that is already in use means one directory was reached
    /// twice (a bind mount, or the admin directory inside the tree); paths
    /// could not be told apart, so setup fails.
    fn add(&mut self, path: &Path, root: Root, relative: PathBuf, mask: WatchMask) -> io::Result<()> {
        let descriptor = self.inotify.watches().add(path, mask)?;
        let id = descriptor.get_watch_descriptor_id();
        if self.watches.contains_key(&id) {
            return Err(io::Error::other("the directory is already watched under another path"));
        }
        self.watches.insert(id, Watch { root, path: relative, descriptor });
        Ok(())
    }

    /// Turns one inotify event into raw events or a loss.
    fn translate(&mut self, id: i32, mask: EventMask, name: Option<&OsStr>, received: SystemTime, sink: &mut Sink<'_>) {
        if mask.contains(EventMask::Q_OVERFLOW) {
            sink.lose(CoverageLost::Overflow);
            return;
        }
        let Some(watch) = self.watches.get(&id) else {
            return;
        };
        let root = watch.root;
        let released = root == Root::Admin && self.admin_released;
        if mask.contains(EventMask::IGNORED) {
            self.watches.remove(&id);
            if !self.ending.remove(&id) {
                sink.lose(CoverageLost::WatchLost { root });
            }
            return;
        }
        if released {
            return;
        }
        if mask.contains(EventMask::UNMOUNT) {
            sink.lose(CoverageLost::WatchLost { root });
            return;
        }
        if mask.contains(EventMask::DELETE_SELF) {
            // A tree directory's deletion is reported by the directory that
            // held it; only the parent and admin directory must outlive the
            // session.
            match root {
                Root::Tree => {
                    self.ending.insert(id);
                }
                Root::Parent | Root::Admin => sink.lose(CoverageLost::WatchLost { root }),
            }
            return;
        }
        if mask.contains(EventMask::MOVE_SELF) {
            // A moved tree directory is reported by the directory it left.
            // Its watch then keeps its old label; the move alone stops the
            // removal. A moved parent or admin directory no longer names what
            // the session watches.
            if root != Root::Tree {
                sink.lose(CoverageLost::WatchLost { root });
            }
            return;
        }
        let kind = if mask.contains(EventMask::CREATE) {
            EventKind::Created
        } else if mask.contains(EventMask::DELETE) {
            EventKind::Removed
        } else if mask.intersects(EventMask::MODIFY | EventMask::CLOSE_WRITE) {
            EventKind::Modified
        } else if mask.contains(EventMask::ATTRIB) {
            EventKind::Metadata
        } else if mask.contains(EventMask::MOVED_FROM) {
            EventKind::MovedFrom
        } else if mask.contains(EventMask::MOVED_TO) {
            EventKind::MovedTo
        } else {
            return;
        };
        let path = match name {
            Some(name) => watch.path.join(name),
            None => watch.path.clone(),
        };
        sink.push(RawEvent { root, path, kind, directory: Some(mask.contains(EventMask::ISDIR)), received });
    }
}

impl Backend for Inotify {
    fn drain(&mut self, sink: &mut Sink<'_>) {
        let mut buffer = std::mem::take(&mut self.buffer);
        loop {
            let events = match self.inotify.read_events(&mut buffer) {
                Ok(events) => events,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => {
                    sink.lose(CoverageLost::ReadFailed { message: error.to_string() });
                    break;
                }
            };
            let received = SystemTime::now();
            for event in events {
                self.translate(event.wd.get_watch_descriptor_id(), event.mask, event.name, received, sink);
            }
        }
        self.buffer = buffer;
    }

    fn release_admin(&mut self) {
        if self.admin_released {
            return;
        }
        self.admin_released = true;
        let admin = self.watches.iter().find(|(_, watch)| watch.root == Root::Admin).map(|(id, watch)| (*id, watch.descriptor.clone()));
        if let Some((id, descriptor)) = admin {
            // `EINVAL` means the watch already ended; its `IN_IGNORED` is
            // queued, and the release makes it expected.
            self.ending.insert(id);
            let _ = self.inotify.watches().remove(descriptor);
        }
    }

    /// Sniff reports each watch inside the checkout with the instance's file
    /// descriptor; the parent and admin watches are outside it.
    fn handles(&self) -> Vec<(EvidenceKind, u64)> {
        let descriptor = self.inotify.as_raw_fd();
        vec![(EvidenceKind::WatchRegistration, u64::try_from(descriptor).expect("an open descriptor is not negative"))]
    }
}
