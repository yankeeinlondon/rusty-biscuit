//! Lifecycle and error tests for [`super::ForegroundHandoff`].
//!
//! The Unix tests drive the real guard through a scripted [`TerminalOps`], so
//! they never read or change the ambient terminal. Transfer to a real child in
//! a real terminal is proven by the detached-tmux tests of its consumers.

#[cfg(unix)]
mod unix {
    use std::io;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::process::Command;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};

    use super::super::unix::{Handoff, TerminalOps};

    const HOST_GROUP: libc::pid_t = 4100;
    const SAVED_SETTINGS: u32 = 0x5e77;
    const SAVED_MASK: u32 = 0b1010;

    /// Which scripted operation fails, and with what errno.
    #[derive(Default, Clone)]
    struct Failures {
        open: Option<i32>,
        foreground_group: Option<i32>,
        settings: Option<i32>,
        /// Number of `set_foreground` calls that fail before one succeeds.
        set_foreground: usize,
        set_settings: usize,
        block_ttou: Option<i32>,
    }

    #[derive(Default)]
    struct State {
        failures: Failures,
        /// The terminal's foreground group as the script reports it.
        foreground: libc::pid_t,
        /// Every mutation and mask change, in order.
        log: Vec<String>,
        /// The calling thread's simulated mask.
        mask: u32,
    }

    #[derive(Clone)]
    struct Scripted(Arc<Mutex<State>>);

    impl Scripted {
        fn new(foreground: libc::pid_t, failures: Failures) -> Self {
            Self(Arc::new(Mutex::new(State { failures, foreground, mask: SAVED_MASK, ..State::default() })))
        }

        fn log(&self) -> Vec<String> {
            self.0.lock().unwrap().log.clone()
        }

        fn mask(&self) -> u32 {
            self.0.lock().unwrap().mask
        }

        fn record(&self, entry: impl Into<String>) {
            self.0.lock().unwrap().log.push(entry.into());
        }
    }

    fn errno(code: Option<i32>) -> io::Result<()> {
        match code {
            Some(code) => Err(io::Error::from_raw_os_error(code)),
            None => Ok(()),
        }
    }

    fn take_failure(counter: &mut usize) -> io::Result<()> {
        if *counter > 0 {
            *counter -= 1;
            Err(io::Error::from_raw_os_error(libc::EIO))
        } else {
            Ok(())
        }
    }

    const TTOU_BIT: u32 = 0b1_0000;

    impl TerminalOps for Scripted {
        type Terminal = ();
        type Settings = u32;
        type Mask = u32;

        fn open(&self) -> io::Result<Option<()>> {
            errno(self.0.lock().unwrap().failures.open).map(Some)
        }

        fn host_group(&self) -> libc::pid_t {
            HOST_GROUP
        }

        fn foreground_group(&self, _: &()) -> io::Result<libc::pid_t> {
            let state = self.0.lock().unwrap();
            errno(state.failures.foreground_group).map(|()| state.foreground)
        }

        fn settings(&self, _: &()) -> io::Result<u32> {
            errno(self.0.lock().unwrap().failures.settings).map(|()| SAVED_SETTINGS)
        }

        fn set_foreground(&self, _: &(), group: libc::pid_t) -> io::Result<()> {
            let mut state = self.0.lock().unwrap();
            assert_ne!(state.mask & TTOU_BIT, 0, "SIGTTOU must be blocked while restoring the foreground");
            take_failure(&mut state.failures.set_foreground)?;
            state.foreground = group;
            state.log.push(format!("set_foreground {group}"));
            Ok(())
        }

        fn set_settings(&self, _: &(), settings: &u32) -> io::Result<()> {
            let mut state = self.0.lock().unwrap();
            assert_ne!(state.mask & TTOU_BIT, 0, "SIGTTOU must be blocked while restoring settings");
            take_failure(&mut state.failures.set_settings)?;
            state.log.push(format!("set_settings {settings:#x}"));
            Ok(())
        }

        fn block_ttou(&self) -> io::Result<u32> {
            let mut state = self.0.lock().unwrap();
            errno(state.failures.block_ttou)?;
            let previous = state.mask;
            state.mask |= TTOU_BIT;
            Ok(previous)
        }

        fn restore_mask(&self, mask: &u32) -> io::Result<()> {
            self.0.lock().unwrap().mask = *mask;
            Ok(())
        }

        fn register_transfer(&self, _: &mut Command, _: &(), host_group: libc::pid_t) {
            self.record(format!("register_transfer from {host_group}"));
        }
    }

    /// A claim private to one test, so tests never contend with each other or
    /// with a real handoff.
    fn claim() -> &'static AtomicBool {
        Box::leak(Box::new(AtomicBool::new(false)))
    }

    fn prepare(script: &Scripted, claim: &'static AtomicBool) -> io::Result<Handoff<Scripted>> {
        Handoff::prepare(script.clone(), claim, &mut Command::new("true"))
    }

    fn restored() -> Vec<String> {
        vec![
            format!("register_transfer from {HOST_GROUP}"),
            format!("set_foreground {HOST_GROUP}"),
            format!("set_settings {SAVED_SETTINGS:#x}"),
        ]
    }

    #[test]
    fn a_foreground_host_arms_a_transfer_and_restores_group_and_settings() {
        let script = Scripted::new(HOST_GROUP, Failures::default());
        let mut handoff = prepare(&script, claim()).unwrap();
        assert!(handoff.is_active());
        // The child took the foreground.
        script.0.lock().unwrap().foreground = 7777;

        handoff.restore().unwrap();

        assert!(!handoff.is_active());
        assert_eq!(script.log(), restored());
        assert_eq!(script.0.lock().unwrap().foreground, HOST_GROUP);
        assert_eq!(script.mask(), SAVED_MASK, "the thread mask returns to its saved value");
    }

    #[test]
    fn repeated_restore_and_drop_change_nothing_more() {
        let script = Scripted::new(HOST_GROUP, Failures::default());
        let mut handoff = prepare(&script, claim()).unwrap();
        handoff.restore().unwrap();
        handoff.restore().unwrap();
        drop(handoff);
        assert_eq!(script.log(), restored());
    }

    #[test]
    fn a_background_host_is_inactive_and_mutates_nothing() {
        let script = Scripted::new(HOST_GROUP + 1, Failures::default());
        let claim = claim();
        let mut handoff = prepare(&script, claim).unwrap();
        assert!(!handoff.is_active());
        handoff.restore().unwrap();
        drop(handoff);
        assert!(script.log().is_empty(), "no transfer registered and no restoration: {:?}", script.log());
        assert_eq!(script.mask(), SAVED_MASK);
        // The claim was released immediately.
        assert!(prepare(&Scripted::new(HOST_GROUP, Failures::default()), claim).unwrap().is_active());
    }

    #[test]
    fn no_controlling_terminal_is_inactive_for_each_no_terminal_errno() {
        for code in [libc::ENOENT, libc::ENXIO, libc::ENOTTY] {
            let script = Scripted::new(HOST_GROUP, Failures { open: Some(code), ..Failures::default() });
            let handoff = prepare(&script, claim()).unwrap();
            assert!(!handoff.is_active(), "errno {code}");
            drop(handoff);
            assert!(script.log().is_empty(), "errno {code}: {:?}", script.log());
        }
        let script =
            Scripted::new(HOST_GROUP, Failures { foreground_group: Some(libc::ENOTTY), ..Failures::default() });
        assert!(!prepare(&script, claim()).unwrap().is_active());
        assert!(script.log().is_empty());
    }

    #[test]
    fn other_preparation_failures_return_the_original_error_and_release_the_claim() {
        let cases = [
            Failures { open: Some(libc::EACCES), ..Failures::default() },
            Failures { foreground_group: Some(libc::EIO), ..Failures::default() },
            Failures { settings: Some(libc::EIO), ..Failures::default() },
        ];
        for failures in cases {
            let expected = failures.open.or(failures.foreground_group).or(failures.settings);
            let script = Scripted::new(HOST_GROUP, failures);
            let claim = claim();
            let error = prepare(&script, claim).unwrap_err();
            assert_eq!(error.raw_os_error(), expected);
            assert!(script.log().is_empty(), "no transfer is registered: {:?}", script.log());
            assert!(prepare(&Scripted::new(HOST_GROUP, Failures::default()), claim).is_ok());
        }
    }

    #[test]
    fn a_second_active_handoff_would_block_until_the_first_is_restored() {
        let claim = claim();
        let first_script = Scripted::new(HOST_GROUP, Failures::default());
        let mut first = prepare(&first_script, claim).unwrap();

        let second_script = Scripted::new(HOST_GROUP, Failures::default());
        let error = prepare(&second_script, claim).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert!(second_script.log().is_empty(), "the refused request inspected nothing");

        first.restore().unwrap();
        assert!(prepare(&second_script, claim).unwrap().is_active());
    }

    #[test]
    fn a_failed_group_restore_still_restores_settings_and_returns_the_first_error() {
        let script = Scripted::new(HOST_GROUP, Failures { set_foreground: 1, ..Failures::default() });
        let claim = claim();
        let mut handoff = prepare(&script, claim).unwrap();

        let error = handoff.restore().unwrap_err();

        assert_eq!(error.raw_os_error(), Some(libc::EIO));
        assert_eq!(
            script.log(),
            vec![format!("register_transfer from {HOST_GROUP}"), format!("set_settings {SAVED_SETTINGS:#x}")]
        );
        assert!(handoff.is_active(), "incomplete restoration stays armed");
        assert_eq!(script.mask(), SAVED_MASK);
        assert_eq!(prepare(&script, claim).unwrap_err().kind(), io::ErrorKind::WouldBlock);

        // Drop retries the armed restoration once, then releases the claim.
        drop(handoff);
        assert_eq!(script.log().last().unwrap(), &format!("set_settings {SAVED_SETTINGS:#x}"));
        assert!(script.log().contains(&format!("set_foreground {HOST_GROUP}")));
        assert!(prepare(&Scripted::new(HOST_GROUP, Failures::default()), claim).is_ok());
    }

    #[test]
    fn a_failed_settings_restore_is_reported_after_the_group_is_restored() {
        let script = Scripted::new(HOST_GROUP, Failures { set_settings: 1, ..Failures::default() });
        let mut handoff = prepare(&script, claim()).unwrap();
        assert_eq!(handoff.restore().unwrap_err().raw_os_error(), Some(libc::EIO));
        assert!(handoff.is_active());
        assert_eq!(script.0.lock().unwrap().foreground, HOST_GROUP);
        handoff.restore().unwrap();
        assert!(!handoff.is_active());
    }

    #[test]
    fn drop_retries_once_and_never_panics_when_restoration_keeps_failing() {
        let script = Scripted::new(HOST_GROUP, Failures { set_foreground: 2, set_settings: 2, ..Failures::default() });
        let claim = claim();
        let mut handoff = prepare(&script, claim).unwrap();
        assert!(handoff.restore().is_err());
        drop(handoff);
        let state = script.0.lock().unwrap();
        assert_eq!((state.failures.set_foreground, state.failures.set_settings), (0, 0), "exactly one retry");
        assert_eq!(state.mask, SAVED_MASK);
        drop(state);
        assert!(prepare(&Scripted::new(HOST_GROUP, Failures::default()), claim).is_ok());
    }

    #[test]
    fn a_failed_mask_block_leaves_the_guard_armed_for_drop() {
        let script = Scripted::new(HOST_GROUP, Failures { block_ttou: Some(libc::EINVAL), ..Failures::default() });
        let mut handoff = prepare(&script, claim()).unwrap();
        assert_eq!(handoff.restore().unwrap_err().raw_os_error(), Some(libc::EINVAL));
        assert!(handoff.is_active());
        assert_eq!(script.log(), vec![format!("register_transfer from {HOST_GROUP}")]);
        script.0.lock().unwrap().failures.block_ttou = None;
        drop(handoff);
        assert_eq!(script.log(), restored());
    }

    #[test]
    fn unwinding_cleans_up_the_tree_before_restoring_the_terminal_and_releases_the_claim() {
        /// Stands in for an armed `ProcessTree`, whose drop terminates the tree.
        struct Tree(Scripted);
        impl Drop for Tree {
            fn drop(&mut self) {
                self.0.record("tree cleanup");
            }
        }

        let script = Scripted::new(HOST_GROUP, Failures::default());
        let claim = claim();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let _handoff = prepare(&script, claim).unwrap();
            let _tree = Tree(script.clone());
            panic!("caller panicked while the child ran");
        }));

        assert!(outcome.is_err());
        assert_eq!(
            script.log(),
            vec![
                format!("register_transfer from {HOST_GROUP}"),
                "tree cleanup".to_owned(),
                format!("set_foreground {HOST_GROUP}"),
                format!("set_settings {SAVED_SETTINGS:#x}"),
            ]
        );
        assert_eq!(script.mask(), SAVED_MASK);
        assert!(prepare(&Scripted::new(HOST_GROUP, Failures::default()), claim).unwrap().is_active());
    }

    #[test]
    fn the_public_guard_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<crate::process::ForegroundHandoff>();
    }
}

#[cfg(windows)]
mod windows {
    use std::process::Command;

    use crate::process::ForegroundHandoff;

    #[test]
    fn native_windows_handoffs_are_inactive_and_restore_trivially() {
        let mut command = Command::new("cmd");
        let mut handoff = ForegroundHandoff::prepare(&mut command).unwrap();
        assert!(!handoff.is_active());
        handoff.restore().unwrap();
        handoff.restore().unwrap();
        // A second guard is not refused: an inactive guard holds no claim.
        assert!(!ForegroundHandoff::prepare(&mut command).unwrap().is_active());
    }
}
