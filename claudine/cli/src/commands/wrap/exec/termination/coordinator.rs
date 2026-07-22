//! Process-scoped interrupt bookkeeping shared by every concurrent wait loop.
//!
//! A parallel sequence group runs several prompt tasks in sibling threads of
//! one Claudine process, so each one enters its own wrapper wait loop. An OS
//! interrupt is delivered to the *process*, not to a loop, which makes press
//! counting, escalation state, and fan-out target selection process-scoped
//! concerns rather than per-loop ones. This module owns exactly that
//! bookkeeping and nothing platform-specific: it decides *which* children an
//! interrupt should reach and *how far* the ladder has advanced for each of
//! them, while the caller performs the actual platform termination.
//!
//! Two registries live here:
//!
//! - **Children.** Each active child registers a caller-chosen payload (a pid,
//!   a Job handle, whatever the platform needs) and receives a
//!   [`ChildToken`]. Escalation state is per registration, so one child
//!   reaching the forceful rung cannot suppress a sibling's graceful rung.
//! - **Interrupt flags.** A sequence run registers its shared `interrupted`
//!   `AtomicBool`; every press sets every registered flag, which is what makes
//!   step boundaries and cooperative shell-task polling observe an interrupt.
//!
//! The registry is generic over the child payload so it can be exercised on
//! any host, including ones whose platform wait loop is not compiled.

use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};

/// Identity of one registered child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ChildToken(u64);

/// Identity of one registered sequence interrupt flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FlagToken(u64);

/// How far along the interrupt ladder a press moves one child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PressAction {
    /// First press for this child — ask it to stop.
    Graceful,
    /// A repeat press for this child — destroy it and its descendants.
    Force,
}

/// One child selected for fan-out, paired with the rung it should receive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PressTarget<T> {
    pub(crate) token: ChildToken,
    pub(crate) action: PressAction,
    pub(crate) child: T,
}

/// Everything one press implies, resolved under a single lock acquisition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PressOutcome<T> {
    /// Process-wide press count after this press. Drives the user-facing
    /// feedback wording, which is a property of the process rather than of any
    /// one child.
    pub(crate) count: u8,
    /// Every child registered at press time, in registration order.
    pub(crate) targets: Vec<PressTarget<T>>,
}

struct ChildEntry<T> {
    token: ChildToken,
    child: T,
    /// Presses this specific child has been targeted by. A child that
    /// registers after the first press still gets its own graceful rung.
    presses: u8,
}

struct RegistryState<T> {
    next_token: u64,
    press_count: u8,
    children: Vec<ChildEntry<T>>,
    flags: Vec<(FlagToken, Weak<AtomicBool>)>,
}

/// Process-scoped interrupt coordinator.
///
/// ## Examples
///
/// ```ignore
/// let registry = InterruptRegistry::<u32>::new();
/// let token = registry.register_child(4321);
/// let outcome = registry.record_press();
/// assert_eq!(outcome.targets[0].action, PressAction::Graceful);
/// registry.deregister_child(token);
/// ```
pub(crate) struct InterruptRegistry<T> {
    state: Mutex<RegistryState<T>>,
}

impl<T: Clone> Default for InterruptRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> InterruptRegistry<T> {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(RegistryState {
                next_token: 0,
                press_count: 0,
                children: Vec::new(),
                flags: Vec::new(),
            }),
        }
    }

    /// A poisoned coordinator must still be able to kill children, so a panic
    /// in some unrelated holder is recovered from rather than propagated.
    fn lock(&self) -> MutexGuard<'_, RegistryState<T>> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn register_child(&self, child: T) -> ChildToken {
        let mut state = self.lock();
        state.next_token += 1;
        let token = ChildToken(state.next_token);
        state.children.push(ChildEntry {
            token,
            child,
            presses: 0,
        });
        token
    }

    pub(crate) fn deregister_child(&self, token: ChildToken) {
        let mut state = self.lock();
        state.children.retain(|entry| entry.token != token);
    }

    /// Register a sequence's shared interrupt flag.
    ///
    /// The flag is held weakly, so a sequence that returns without
    /// deregistering cannot keep its `AtomicBool` alive or make a later press
    /// write through a dangling registration.
    pub(crate) fn register_flag(&self, flag: &Arc<AtomicBool>) -> FlagToken {
        let mut state = self.lock();
        state.next_token += 1;
        let token = FlagToken(state.next_token);
        state.flags.push((token, Arc::downgrade(flag)));
        token
    }

    pub(crate) fn deregister_flag(&self, token: FlagToken) {
        let mut state = self.lock();
        state.flags.retain(|(existing, _)| *existing != token);
    }

    /// Record one process-wide interrupt press.
    ///
    /// Sets every live registered flag and advances every registered child's
    /// ladder by one rung.
    ///
    /// ## Returns
    ///
    /// The new process-wide press count and one [`PressTarget`] per registered
    /// child. Children are returned in registration order so fan-out is
    /// deterministic.
    pub(crate) fn record_press(&self) -> PressOutcome<T> {
        let mut state = self.lock();
        state.press_count = state.press_count.saturating_add(1);
        let count = state.press_count;

        state.flags.retain(|(_, weak)| match weak.upgrade() {
            Some(flag) => {
                flag.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        });

        let targets = state
            .children
            .iter_mut()
            .map(|entry| {
                entry.presses = entry.presses.saturating_add(1);
                PressTarget {
                    token: entry.token,
                    action: if entry.presses == 1 {
                        PressAction::Graceful
                    } else {
                        PressAction::Force
                    },
                    child: entry.child.clone(),
                }
            })
            .collect();

        PressOutcome { count, targets }
    }

    /// Presses this child has been targeted by, or `0` once it deregisters.
    pub(crate) fn child_presses(&self, token: ChildToken) -> u8 {
        self.lock()
            .children
            .iter()
            .find(|entry| entry.token == token)
            .map_or(0, |entry| entry.presses)
    }

    #[cfg(test)]
    pub(crate) fn registered_children(&self) -> usize {
        self.lock().children.len()
    }

    #[cfg(test)]
    pub(crate) fn registered_flags(&self) -> usize {
        self.lock().flags.len()
    }
}

/// The install/remove transition an [`InstallRefcount`] operation implies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefcountTransition {
    /// First holder — install the process-wide resource now.
    Install,
    /// Last holder released — remove it now.
    Remove,
    /// Neither edge; the resource's state is already correct.
    Unchanged,
}

/// Refcount for a process-wide resource that must be installed exactly once
/// no matter how many concurrent holders want it.
///
/// `SetConsoleCtrlHandler` stacks registrations, so a handler installed per
/// wait loop would count one chord once per running child. Holders acquire and
/// release through this counter and act only on the reported edge.
pub(crate) struct InstallRefcount {
    count: Mutex<usize>,
}

impl InstallRefcount {
    pub(crate) const fn new() -> Self {
        Self {
            count: Mutex::new(0),
        }
    }

    pub(crate) fn acquire(&self) -> RefcountTransition {
        let mut count = self.count.lock().unwrap_or_else(|e| e.into_inner());
        *count += 1;
        if *count == 1 {
            RefcountTransition::Install
        } else {
            RefcountTransition::Unchanged
        }
    }

    pub(crate) fn release(&self) -> RefcountTransition {
        let mut count = self.count.lock().unwrap_or_else(|e| e.into_inner());
        // An unbalanced release must not report a second `Remove`, which would
        // uninstall a handler another holder still depends on.
        if *count == 0 {
            return RefcountTransition::Unchanged;
        }
        *count -= 1;
        if *count == 0 {
            RefcountTransition::Remove
        } else {
            RefcountTransition::Unchanged
        }
    }
}

/// A process-wide resource that must exist while any holder wants it.
///
/// The installer is a trait parameter rather than a `cfg`-selected body for the
/// same reason [`HandleCloser`](super::handle::HandleCloser) is: the acquire /
/// release bookkeeping — installed on the first holder, removed only on the
/// last, never removed while an unrelated holder is still live — is exercised
/// by ordinary unit tests on every host, not only where `SetConsoleCtrlHandler`
/// compiles.
pub(crate) trait ProcessHandler {
    /// The counter arbitrating this handler's single installation. One static
    /// per implementor; sharing one between two handlers would let either
    /// suppress the other's install edge.
    fn refcount() -> &'static InstallRefcount;
    fn install();
    fn remove();
}

/// RAII holder of a [`ProcessHandler`] installation.
///
/// Holders are heterogeneous: a compose run, a sequence run, and every
/// concurrent child wait loop can each want the console handler at once, in any
/// interleaving. Only the edges reported by [`InstallRefcount`] reach `H`.
pub(crate) struct HandlerGuard<H: ProcessHandler>(PhantomData<H>);

impl<H: ProcessHandler> HandlerGuard<H> {
    pub(crate) fn acquire() -> Self {
        if H::refcount().acquire() == RefcountTransition::Install {
            H::install();
        }
        Self(PhantomData)
    }
}

impl<H: ProcessHandler> Drop for HandlerGuard<H> {
    fn drop(&mut self) {
        if H::refcount().release() == RefcountTransition::Remove {
            H::remove();
        }
    }
}

#[cfg(test)]
mod tests;
