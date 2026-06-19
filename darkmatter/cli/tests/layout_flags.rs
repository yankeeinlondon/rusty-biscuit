mod common;

use common::{md_cmd, md_file};

#[test]
fn layout_margin_shorthand_overrides_axis_and_side() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--margin")
        .arg("4")
        .arg("--mx")
        .arg("2")
        .arg("--mt")
        .arg("1")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "margin flags should parse and be accepted"
    );
}

#[test]
fn layout_padding_shorthand_overrides_axis_and_side() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--padding")
        .arg("4")
        .arg("--px")
        .arg("2")
        .arg("--pt")
        .arg("1")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "padding flags should parse and be accepted"
    );
}

#[test]
fn layout_max_width_zero_rejected() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--max-width")
        .arg("0")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("max-width") || stderr.contains("0"),
        "should reject --max-width 0, got: {stderr}"
    );
}

#[test]
fn layout_max_width_positive_accepted() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--max-width")
        .arg("80")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_fill_full_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill")
        .arg("full")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_fill_pad_fixed_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill")
        .arg("pad=4")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_fill_pad_percent_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill")
        .arg("pad=10%")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_fill_indent_max_explicit_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    for fill in ["indent=2", "max=40", "explicit=60"] {
        let output = md_cmd()
            .arg(tmp.path())
            .arg("--fill")
            .arg(fill)
            .output()
            .unwrap();
        assert!(output.status.success(), "--fill {fill} should succeed");
    }
}

#[test]
fn layout_fill_unknown_kind_rejected() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill")
        .arg("unknown=4")
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn layout_fill_percent_over_100_rejected() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill")
        .arg("pad=150%")
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn layout_fill_negative_rejected() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill")
        .arg("pad=-1")
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn layout_alignment_global_accepted() {
    let tmp = md_file("| A | B |\n|---|---|\n| 1 | 2 |\n");
    for align in ["left", "center", "right"] {
        let output = md_cmd()
            .arg(tmp.path())
            .arg("--alignment")
            .arg(align)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "--alignment {align} should succeed"
        );
    }
}

#[test]
fn layout_align_component_overrides_global() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--alignment")
        .arg("center")
        .arg("--align-code-blocks")
        .arg("left")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_page_bg_accepted() {
    let tmp = md_file("# Hello\n");
    for bg in ["transparent", "subtle", "pronounced"] {
        let output = md_cmd()
            .arg(tmp.path())
            .arg("--page-bg")
            .arg(bg)
            .output()
            .unwrap();
        assert!(output.status.success(), "--page-bg {bg} should succeed");
    }
}

#[test]
fn layout_page_background_alias_works() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--page-background")
        .arg("subtle")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_page_bg_color_hex_accepted() {
    let tmp = md_file("# Hello\n");
    for hex in ["#1e1e23", "#abc", "#ffffff"] {
        let output = md_cmd()
            .arg(tmp.path())
            .arg("--page-bg-color")
            .arg(hex)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "--page-bg-color {hex} should succeed; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn layout_page_bg_color_rgb_triple_accepted() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--page-bg-color")
        .arg("30,30,35")
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn layout_page_bg_color_tailwind_accepted() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--page-bg-color")
        .arg("red-500")
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn layout_page_bg_color_special_keyword_accepted() {
    let tmp = md_file("# Hello\n");
    for kw in ["transparent", "currentColor", "inherit"] {
        let output = md_cmd()
            .arg(tmp.path())
            .arg("--page-bg-color")
            .arg(kw)
            .output()
            .unwrap();
        assert!(output.status.success(), "--page-bg-color {kw} should succeed");
    }
}

#[test]
fn layout_page_bg_color_invalid_rejected() {
    let tmp = md_file("# Hello\n");
    for bad in ["not-a-color", "256,0,0", "1,2", "purple-555"] {
        let output = md_cmd()
            .arg(tmp.path())
            .arg("--page-bg-color")
            .arg(bad)
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "--page-bg-color {bad} should fail"
        );
    }
}

#[test]
fn layout_width_flag_rejected() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--width")
        .arg("80")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--width") || stderr.contains("max-width"),
        "--width should be rejected with a helpful error; got: {stderr}"
    );
}

#[test]
fn layout_margin_aliases_work() {
    // `--margin-top` and friends should be accepted as visible_aliases for
    // the existing `--mt` / `--mb` / `--ml` / `--mr` flags.
    let tmp = md_file("# Hello\n");
    for args in [
        ["--margin-top", "1"],
        ["--margin-bottom", "1"],
        ["--margin-left", "1"],
        ["--margin-right", "1"],
        ["--padding-top", "1"],
        ["--padding-bottom", "1"],
        ["--padding-left", "1"],
        ["--padding-right", "1"],
    ] {
        let output = md_cmd()
            .arg(tmp.path())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {} should be accepted; stderr: {}",
            args[0],
            args[1],
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn layout_line_numbers_flag_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--line-numbers")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "--line-numbers flag should be accepted"
    );
}

#[test]
fn layout_line_numbers_true_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--line-numbers")
        .arg("true")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "--line-numbers true should be accepted"
    );
}

#[test]
fn layout_line_numbers_false_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--line-numbers")
        .arg("false")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "--line-numbers false should be accepted"
    );
}

#[test]
fn layout_fill_component_specific_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill-code-blocks")
        .arg("max=40")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_margin_negative_rejected() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--margin")
        .arg("-1")
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn layout_no_flags_preserves_existing_behavior() {
    let tmp = md_file("# Hello World\n\nSome prose here.\n");
    let output = md_cmd().arg(tmp.path()).output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hello World"),
        "output should contain heading without layout flags"
    );
}

#[test]
fn layout_combined_margin_padding_bg() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--margin")
        .arg("2")
        .arg("--padding")
        .arg("1")
        .arg("--page-bg")
        .arg("subtle")
        .output()
        .unwrap();

    assert!(output.status.success());
}
