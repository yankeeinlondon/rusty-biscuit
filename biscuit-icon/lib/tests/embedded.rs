use biscuit_icon::domain::{DomainIcon, Os};

#[test]
fn finder_assembles_a_real_svg() {
    let svg = Os::Finder.icon().svg();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("viewBox="));
    // The placeholder body was a comment; a real body has markup.
    assert!(!svg.contains("<!-- hugeicons:apple-finder -->"));
}
