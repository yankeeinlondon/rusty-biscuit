//! Strategy B: pointer selection is atomic for readers of local state, but
//! the committed fixed paths still need A's manifest verification.

mod common;

use common::{Seen, classify, new, old};
use publication_spike::fault::{Faults, Point};
use publication_spike::generations::{GEN_DIR, Options, publish, read_selected, recover};
use publication_spike::model::read_verified;
use publication_spike::{SpikeError, resolve};

#[test]
fn crash_at_each_point() {
    let mut rows = Vec::new();
    for point in [
        Point::BeforeStaging,
        Point::DuringStaging,
        Point::BeforeSelection,
        Point::AfterSelection,
        Point::MidReplacement(3),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        publish(root, &old(), Options::default()).unwrap();
        let err = publish(root, &new(), Options { faults: Faults::error_at(point), ..Default::default() });
        assert!(matches!(err, Err(SpikeError::Crash(_))));
        let via_pointer = classify(read_selected(root));
        let via_fixed = classify(read_verified(root));
        assert!(matches!(via_pointer, Seen::Old | Seen::New), "{point:?}: {via_pointer:?}");
        assert!(!matches!(via_fixed, Seen::Other(_)));
        recover(root, Options::default()).unwrap();
        let after = classify(read_verified(root));
        assert_eq!(after, via_pointer, "{point:?}: fixed paths converge on the selected generation");
        let fixed_label = match via_fixed { Seen::Refused(_) => "Refused".into(), other => format!("{other:?}") };
        rows.push(format!("{:<18} pointer={via_pointer:?} fixed={fixed_label:<8} post={after:?}", point.label()));
    }
    eprintln!("EVIDENCE generations\n{}", rows.join("\n"));
}

/// A fresh clone has the committed fixed paths but no (gitignored) generation
/// state, so B's reader has nothing to select; only manifest verification of
/// the fixed paths works. B therefore needs A's verifier anyway.
#[test]
fn fresh_clone_has_no_generation_state() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    publish(root, &old(), Options::default()).unwrap();
    std::fs::remove_dir_all(resolve(root, GEN_DIR)).unwrap();
    assert!(matches!(read_selected(root), Err(SpikeError::NoSnapshot)));
    assert_eq!(classify(read_verified(root)), Seen::Old);
}
