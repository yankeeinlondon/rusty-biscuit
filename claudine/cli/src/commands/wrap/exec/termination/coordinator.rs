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

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> InterruptRegistry<&'static str> {
        InterruptRegistry::new()
    }

    #[test]
    fn first_press_is_graceful_and_second_is_forceful() {
        let reg = registry();
        let token = reg.register_child("child-a");

        let first = reg.record_press();
        assert_eq!(first.count, 1);
        assert_eq!(first.targets.len(), 1);
        assert_eq!(first.targets[0].token, token);
        assert_eq!(first.targets[0].action, PressAction::Graceful);

        let second = reg.record_press();
        assert_eq!(second.count, 2);
        assert_eq!(second.targets[0].action, PressAction::Force);
    }

    #[test]
    fn press_fans_out_to_every_registered_child() {
        let reg = registry();
        reg.register_child("child-a");
        reg.register_child("child-b");
        reg.register_child("child-c");

        let outcome = reg.record_press();
        let names: Vec<&str> = outcome.targets.iter().map(|t| t.child).collect();
        assert_eq!(names, vec!["child-a", "child-b", "child-c"]);
        assert!(
            outcome
                .targets
                .iter()
                .all(|t| t.action == PressAction::Graceful)
        );
    }

    /// Regression for the defect this coordinator replaces: escalation state
    /// used to be process-global, so whichever sibling reached the forceful
    /// rung first suppressed every other sibling's kill.
    #[test]
    fn a_siblings_escalation_does_not_suppress_another_childs_ladder() {
        let reg = registry();
        reg.register_child("early");
        reg.record_press();

        reg.register_child("late");
        let second = reg.record_press();

        let by_name: Vec<(&str, PressAction)> = second
            .targets
            .iter()
            .map(|t| (t.child, t.action))
            .collect();
        assert_eq!(
            by_name,
            vec![("early", PressAction::Force), ("late", PressAction::Graceful)]
        );
    }

    #[test]
    fn deregistered_children_are_not_targeted() {
        let reg = registry();
        let gone = reg.register_child("child-a");
        reg.register_child("child-b");
        reg.deregister_child(gone);

        let outcome = reg.record_press();
        assert_eq!(outcome.targets.len(), 1);
        assert_eq!(outcome.targets[0].child, "child-b");
        assert_eq!(reg.child_presses(gone), 0);
        assert_eq!(reg.registered_children(), 1);
    }

    #[test]
    fn child_presses_tracks_only_that_child() {
        let reg = registry();
        let a = reg.register_child("child-a");
        reg.record_press();
        let b = reg.register_child("child-b");
        reg.record_press();

        assert_eq!(reg.child_presses(a), 2);
        assert_eq!(reg.child_presses(b), 1);
    }

    #[test]
    fn press_sets_every_registered_flag() {
        let reg = registry();
        let first = Arc::new(AtomicBool::new(false));
        let second = Arc::new(AtomicBool::new(false));
        reg.register_flag(&first);
        reg.register_flag(&second);

        reg.record_press();

        assert!(first.load(Ordering::SeqCst));
        assert!(second.load(Ordering::SeqCst));
    }

    #[test]
    fn deregistered_flag_is_not_set() {
        let reg = registry();
        let flag = Arc::new(AtomicBool::new(false));
        let token = reg.register_flag(&flag);
        reg.deregister_flag(token);

        reg.record_press();

        assert!(!flag.load(Ordering::SeqCst));
        assert_eq!(reg.registered_flags(), 0);
    }

    #[test]
    fn dropped_flag_registration_is_pruned_instead_of_panicking() {
        let reg = registry();
        {
            let flag = Arc::new(AtomicBool::new(false));
            reg.register_flag(&flag);
        }
        assert_eq!(reg.registered_flags(), 1);

        reg.record_press();

        assert_eq!(reg.registered_flags(), 0);
    }

    /// A press with nothing registered still counts, so a later child does not
    /// inherit a stale rung and the feedback wording stays monotonic.
    #[test]
    fn press_count_advances_with_no_children_registered() {
        let reg = registry();
        assert_eq!(reg.record_press().count, 1);
        assert_eq!(reg.record_press().count, 2);

        let token = reg.register_child("late");
        let third = reg.record_press();
        assert_eq!(third.count, 3);
        assert_eq!(third.targets[0].action, PressAction::Graceful);
        assert_eq!(reg.child_presses(token), 1);
    }

    #[test]
    fn press_count_saturates_rather_than_wrapping() {
        let reg = registry();
        for _ in 0..300 {
            reg.record_press();
        }
        assert_eq!(reg.record_press().count, u8::MAX);
    }

    #[test]
    fn refcount_installs_on_the_first_holder_and_removes_on_the_last() {
        let refcount = InstallRefcount::new();
        assert_eq!(refcount.acquire(), RefcountTransition::Install);
        assert_eq!(refcount.acquire(), RefcountTransition::Unchanged);
        assert_eq!(refcount.acquire(), RefcountTransition::Unchanged);

        assert_eq!(refcount.release(), RefcountTransition::Unchanged);
        assert_eq!(refcount.release(), RefcountTransition::Unchanged);
        assert_eq!(refcount.release(), RefcountTransition::Remove);
    }

    #[test]
    fn refcount_reinstalls_after_a_full_release() {
        let refcount = InstallRefcount::new();
        refcount.acquire();
        refcount.release();
        assert_eq!(refcount.acquire(), RefcountTransition::Install);
    }

    #[test]
    fn unbalanced_release_does_not_report_a_second_removal() {
        let refcount = InstallRefcount::new();
        assert_eq!(refcount.release(), RefcountTransition::Unchanged);
        refcount.acquire();
        assert_eq!(refcount.release(), RefcountTransition::Remove);
        assert_eq!(refcount.release(), RefcountTransition::Unchanged);
    }

    /// Whatever the interleaving, concurrent holders must produce exactly one
    /// install edge and exactly one remove edge.
    #[test]
    fn concurrent_holders_yield_one_install_and_one_remove() {
        static REFCOUNT: InstallRefcount = InstallRefcount::new();
        let installs = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let removes = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        // One long-lived holder spans every worker's lifetime, so the count
        // never returns to zero mid-run and the edges cannot be double-counted.
        assert_eq!(REFCOUNT.acquire(), RefcountTransition::Install);
        installs.fetch_add(1, Ordering::SeqCst);

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let installs = Arc::clone(&installs);
                let removes = Arc::clone(&removes);
                std::thread::spawn(move || {
                    if REFCOUNT.acquire() == RefcountTransition::Install {
                        installs.fetch_add(1, Ordering::SeqCst);
                    }
                    if REFCOUNT.release() == RefcountTransition::Remove {
                        removes.fetch_add(1, Ordering::SeqCst);
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(REFCOUNT.release(), RefcountTransition::Remove);
        removes.fetch_add(1, Ordering::SeqCst);
        assert_eq!(installs.load(Ordering::SeqCst), 1);
        assert_eq!(removes.load(Ordering::SeqCst), 1);
    }

    /// Concurrent registration and fan-out is the parallel-group shape: the
    /// registry must serialize both without losing a child.
    #[test]
    fn concurrent_registration_and_presses_stay_consistent() {
        let reg = Arc::new(InterruptRegistry::<usize>::new());
        let mut handles = Vec::new();
        for id in 0..8usize {
            let reg = Arc::clone(&reg);
            handles.push(std::thread::spawn(move || {
                let token = reg.register_child(id);
                reg.record_press();
                token
            }));
        }
        let tokens: Vec<ChildToken> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        assert_eq!(reg.registered_children(), 8);
        assert!(tokens.iter().all(|t| reg.child_presses(*t) >= 1));
        for token in tokens {
            reg.deregister_child(token);
        }
        assert_eq!(reg.registered_children(), 0);
    }
}
