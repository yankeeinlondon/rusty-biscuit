use crate::common;

use common::{CliProcessFixture, md_file};

fn rendering_command(fixture: &CliProcessFixture) -> assert_cmd::Command {
    fixture.command_builder().plain_terminal(80, 24).build()
}

fn stderr_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn layout_margin_shorthand_overrides_axis_and_side() {
    let fixture = CliProcessFixture::named("layout_margin_shorthand_overrides_axis_and_side");
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .args(["style.margin=4", "style.mx=2", "style.mt=1"])
        .output()
        .unwrap();

    assert!(output.status.success(), "margin setters should be accepted: {}", stderr_of(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    assert_eq!(lines.next(), Some(""), "style.mt=1 gives one top row:\n{stdout}");
    assert_eq!(lines.next(), Some("  Hello"), "style.mx=2 overrides the shorthand's left side:\n{stdout}");
}

#[test]
fn layout_padding_shorthand_overrides_axis_and_side() {
    let fixture = CliProcessFixture::named("layout_padding_shorthand_overrides_axis_and_side");
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .args(["style.padding=4", "style.px=2", "style.pt=1"])
        .output()
        .unwrap();

    assert!(output.status.success(), "padding setters should be accepted: {}", stderr_of(&output));
}

#[test]
fn layout_max_width_zero_rejected() {
    let fixture = CliProcessFixture::named("layout_max_width_zero_rejected");
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("style.max-width=0")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "a rejected style prints nothing");
    let stderr = stderr_of(&output);
    assert!(stderr.contains("max-width"), "should reject style.max-width=0, got: {stderr}");
}

#[test]
fn layout_max_width_positive_accepted() {
    let fixture = CliProcessFixture::named("layout_max_width_positive_accepted");
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("style.max-width=80")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr_of(&output));
}

#[test]
fn layout_fill_full_accepted() {
    let fixture = CliProcessFixture::named("layout_fill_full_accepted");
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg(r#"style={ "code-block": { width: "auto", padding: 0 } }"#)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr_of(&output));
}

#[test]
fn layout_fill_pad_fixed_accepted() {
    let fixture = CliProcessFixture::named("layout_fill_pad_fixed_accepted");
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg(r#"style={ "code-block": { padding: 0, "padding-left": 4, "padding-right": 4 } }"#)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr_of(&output));
}

#[test]
fn layout_fill_pad_percent_accepted() {
    let fixture = CliProcessFixture::named("layout_fill_pad_percent_accepted");
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg(r#"style={ "code-block": { padding: 0, "padding-left": "10%", "padding-right": "10%" } }"#)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr_of(&output));
}

#[test]
fn layout_fill_indent_max_explicit_accepted() {
    let fixture = CliProcessFixture::named("layout_fill_indent_max_explicit_accepted");
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    for setter in [
        r#"style={ "code-block": { padding: 0, "padding-left": 2 } }"#,
        "style.code-block.max-width=40",
        "style.code-block.width=60",
    ] {
        let output = rendering_command(&fixture).arg(tmp.path()).arg(setter).output().unwrap();
        assert!(output.status.success(), "{setter} should succeed: {}", stderr_of(&output));
    }
}

#[test]
fn layout_fill_unknown_kind_rejected() {
    let fixture = CliProcessFixture::named("layout_fill_unknown_kind_rejected");
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("style.code-block.unknown=4")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    assert!(stderr_of(&output).contains("unknown"), "{}", stderr_of(&output));
}

#[test]
fn layout_fill_percent_over_100_rejected() {
    let fixture = CliProcessFixture::named("layout_fill_percent_over_100_rejected");
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("style.code-block.padding-left=150%")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
}

#[test]
fn layout_fill_negative_rejected() {
    let fixture = CliProcessFixture::named("layout_fill_negative_rejected");
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("style.code-block.padding-left=-1")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
}

#[test]
fn layout_alignment_global_accepted() {
    let fixture = CliProcessFixture::named("layout_alignment_global_accepted");
    let tmp = md_file("| A | B |\n|---|---|\n| 1 | 2 |\n");
    for align in ["left", "center", "right"] {
        let output = rendering_command(&fixture).arg(tmp.path()).arg(format!("style.alignment={align}")).output().unwrap();
        assert!(output.status.success(), "style.alignment={align} should succeed: {}", stderr_of(&output));
    }
}

#[test]
fn layout_align_component_overrides_global() {
    let fixture = CliProcessFixture::named("layout_align_component_overrides_global");
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .args(["style.alignment=center", "style.code-block.alignment=left"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr_of(&output));
}

#[test]
fn layout_page_bg_accepted() {
    let fixture = CliProcessFixture::named("layout_page_bg_accepted");
    let tmp = md_file("# Hello\n");
    for bg in ["transparent", "subtle", "pronounced"] {
        let output = rendering_command(&fixture).arg(tmp.path()).arg(format!("style.page.background={bg}")).output().unwrap();
        assert!(output.status.success(), "style.page.background={bg} should succeed: {}", stderr_of(&output));
    }
}

#[test]
fn layout_page_background_top_level_alias_works() {
    let fixture = CliProcessFixture::named("layout_page_background_top_level_alias_works");
    // A page key is also accepted at the top level of `style`.
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("style.background=subtle")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr_of(&output));
}

#[test]
fn layout_page_bg_color_hex_accepted() {
    let fixture = CliProcessFixture::named("layout_page_bg_color_hex_accepted");
    let tmp = md_file("# Hello\n");
    for hex in ["#1e1e23", "#abc", "#ffffff"] {
        let output = rendering_command(&fixture).arg(tmp.path()).arg(format!("style.page.bg-color={hex}")).output().unwrap();
        assert!(output.status.success(), "style.page.bg-color={hex} should succeed; stderr: {}", stderr_of(&output));
    }
}

#[test]
fn layout_page_bg_color_rgb_value_accepted_as_hex() {
    let fixture = CliProcessFixture::named("layout_page_bg_color_rgb_value_accepted_as_hex");
    // The old flag's `30,30,35` triple: a style color spells the same RGB
    // value in hex.
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("style.page.bg-color=#1e1e23")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr_of(&output));
}

#[test]
fn layout_page_bg_color_tailwind_accepted() {
    let fixture = CliProcessFixture::named("layout_page_bg_color_tailwind_accepted");
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("style.page.bg-color=red-500")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr_of(&output));
}

#[test]
fn layout_page_bg_color_special_keyword_accepted() {
    let fixture = CliProcessFixture::named("layout_page_bg_color_special_keyword_accepted");
    let tmp = md_file("# Hello\n");
    // The old flag's `currentColor` is the style color `current`.
    for kw in ["transparent", "current", "inherit"] {
        let output = rendering_command(&fixture).arg(tmp.path()).arg(format!("style.page.bg-color={kw}")).output().unwrap();
        assert!(output.status.success(), "style.page.bg-color={kw} should succeed: {}", stderr_of(&output));
    }
}

#[test]
fn layout_page_bg_color_invalid_rejected() {
    let fixture = CliProcessFixture::named("layout_page_bg_color_invalid_rejected");
    let tmp = md_file("# Hello\n");
    for bad in ["not-a-color", "256,0,0", "1,2", "purple-555"] {
        let output = rendering_command(&fixture).arg(tmp.path()).arg(format!("style.page.bg-color={bad}")).output().unwrap();
        assert_eq!(output.status.code(), Some(1), "style.page.bg-color={bad} should fail: {}", stderr_of(&output));
        assert!(output.stdout.is_empty(), "style.page.bg-color={bad}: a rejected style prints nothing");
    }
}

#[test]
fn layout_width_flag_wraps_prose_at_its_columns() {
    let fixture = CliProcessFixture::named("layout_width_flag_wraps_prose_at_its_columns");
    let tmp = md_file("one two three four five six seven eight nine ten\n");
    let run = |width: &str| {
        let output = rendering_command(&fixture).arg(tmp.path()).args(["--width", width]).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8_lossy(&output.stdout).into_owned()
    };
    let wide = run("80");
    assert_eq!(wide.trim_end().lines().count(), 1, "{wide}");
    let narrow = run("20");
    assert!(narrow.trim_end().lines().count() > 1, "{narrow}");
    assert!(narrow.lines().all(|line| line.chars().count() <= 20), "{narrow}");
}

#[test]
fn layout_page_side_keys_work() {
    let fixture = CliProcessFixture::named("layout_page_side_keys_work");
    // The canonical `page.margin-*` and `page.padding-*` keys, which the old
    // `--margin-top` and friends spelled as flags.
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .args([
            "style.page.margin-top=1",
            "style.page.margin-bottom=1",
            "style.page.margin-left=1",
            "style.page.margin-right=1",
            "style.page.padding-top=1",
            "style.page.padding-bottom=1",
            "style.page.padding-left=1",
            "style.page.padding-right=1",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "the page side keys should be accepted; stderr: {}", stderr_of(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    assert_eq!((lines.next(), lines.next()), (Some(""), Some("")), "one margin row and one padding row on top:\n{stdout}");
    assert_eq!(lines.next(), Some("  Hello"), "one margin cell and one padding cell on the left:\n{stdout}");
}

#[test]
fn layout_line_numbers_true_accepted() {
    let fixture = CliProcessFixture::named("layout_line_numbers_true_accepted");
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("style.code-block.line-numbers=true")
        .output()
        .unwrap();

    assert!(output.status.success(), "style.code-block.line-numbers=true should be accepted: {}", stderr_of(&output));
}

#[test]
fn layout_line_numbers_false_accepted() {
    let fixture = CliProcessFixture::named("layout_line_numbers_false_accepted");
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("style.code-block.line-numbers=false")
        .output()
        .unwrap();

    assert!(output.status.success(), "style.code-block.line-numbers=false should be accepted: {}", stderr_of(&output));
}

#[test]
fn layout_fill_component_specific_accepted() {
    let fixture = CliProcessFixture::named("layout_fill_component_specific_accepted");
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("style.code-block.max-width=40")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr_of(&output));
}

#[test]
fn layout_margin_negative_rejected() {
    let fixture = CliProcessFixture::named("layout_margin_negative_rejected");
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .arg("style.margin=-1")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
}

#[test]
fn layout_no_flags_preserves_existing_behavior() {
    let fixture = CliProcessFixture::named("layout_no_flags_preserves_existing_behavior");
    let tmp = md_file("# Hello World\n\nSome prose here.\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hello World"),
        "output should contain heading without layout flags"
    );
}

#[test]
fn layout_combined_margin_padding_bg() {
    let fixture = CliProcessFixture::named("layout_combined_margin_padding_bg");
    let tmp = md_file("# Hello\n");
    let output = rendering_command(&fixture)
        .arg(tmp.path())
        .args(["style.margin=2", "style.padding=1", "style.page.background=subtle"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr_of(&output));
}
