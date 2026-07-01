//! Unit and snapshot tests for the inline-content visual-test matrix.
//!
//! Regenerate snapshots after intentional changes with:
//! `INSTA_UPDATE=always cargo test -p biscuit-terminal --test inline_content_matrix`
//! then review the `.snap` diff.

mod inline_content_matrix_support;

use biscuit_terminal::prelude::strip_escape_codes;
use inline_content_matrix_support::{
    InlineComponentCase, InlineScenario, inline_component_cases, inline_scenarios,
};

#[test]
fn scenario_count_is_fifteen() {
    // One baseline + eleven box-property N/A scenarios + three style-property
    // Honored scenarios.
    assert_eq!(inline_scenarios().len(), 15);
}

#[test]
fn scenario_names_are_unique() {
    let names: Vec<&str> = inline_scenarios().iter().map(|s| s.name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "scenario names must be unique");
}

#[test]
fn component_case_count_is_four() {
    assert_eq!(inline_component_cases().len(), 4);
}

#[test]
fn every_case_renders_non_empty() {
    for case in inline_component_cases() {
        for scenario in inline_scenarios() {
            let out = (case.render)(&scenario);
            assert!(
                !out.is_empty(),
                "{}/{}: render output was empty",
                case.name,
                scenario.name
            );
        }
    }
}

#[test]
fn inline_content_matrix_snapshots() {
    for case in inline_component_cases() {
        for scenario in inline_scenarios() {
            let out = (case.render)(&scenario);
            let block = strip_escape_codes(&out);
            insta::assert_snapshot!(format!("{}__{}", case.name, scenario.name), block);
        }
    }
}

// ── N/A assertions: box properties must not affect inline output ──

fn baseline_output(case: &InlineComponentCase, width: u32) -> String {
    let baseline = InlineScenario {
        name: "baseline",
        layout: renderable::layout::Layout::default(),
        style: renderable::style::Style::default(),
        width,
        is_style: false,
    };
    strip_escape_codes((case.render)(&baseline))
}

fn assert_box_is_na(case: &InlineComponentCase, scenario: &InlineScenario) {
    let base = baseline_output(case, scenario.width);
    let actual = strip_escape_codes((case.render)(scenario));
    assert_eq!(
        base, actual,
        "{} / {}: box property must be N/A for inline component",
        case.name, scenario.name
    );
}

#[test]
fn inline_box_margin_left_is_na() {
    for case in inline_component_cases() {
        let scenario = inline_scenarios()
            .into_iter()
            .find(|s| s.name == "left_margin_4")
            .expect("scenario exists");
        assert_box_is_na(&case, &scenario);
    }
}

#[test]
fn inline_box_margin_right_is_na() {
    for case in inline_component_cases() {
        let scenario = inline_scenarios()
            .into_iter()
            .find(|s| s.name == "right_margin_4")
            .expect("scenario exists");
        assert_box_is_na(&case, &scenario);
    }
}

#[test]
fn inline_box_margin_top_is_na() {
    for case in inline_component_cases() {
        let scenario = inline_scenarios()
            .into_iter()
            .find(|s| s.name == "top_margin_2")
            .expect("scenario exists");
        assert_box_is_na(&case, &scenario);
    }
}

#[test]
fn inline_box_margin_bottom_is_na() {
    for case in inline_component_cases() {
        let scenario = inline_scenarios()
            .into_iter()
            .find(|s| s.name == "bottom_margin_2")
            .expect("scenario exists");
        assert_box_is_na(&case, &scenario);
    }
}

#[test]
fn inline_box_alignment_center_is_na() {
    for case in inline_component_cases() {
        let scenario = inline_scenarios()
            .into_iter()
            .find(|s| s.name == "align_center")
            .expect("scenario exists");
        assert_box_is_na(&case, &scenario);
    }
}

#[test]
fn inline_box_alignment_right_is_na() {
    for case in inline_component_cases() {
        let scenario = inline_scenarios()
            .into_iter()
            .find(|s| s.name == "align_right")
            .expect("scenario exists");
        assert_box_is_na(&case, &scenario);
    }
}

#[test]
fn inline_box_max_width_is_na() {
    for case in inline_component_cases() {
        let scenario = inline_scenarios()
            .into_iter()
            .find(|s| s.name == "max_width_40")
            .expect("scenario exists");
        assert_box_is_na(&case, &scenario);
    }
}

#[test]
fn inline_box_width_auto_is_na() {
    for case in inline_component_cases() {
        let scenario = inline_scenarios()
            .into_iter()
            .find(|s| s.name == "width_auto_fill")
            .expect("scenario exists");
        assert_box_is_na(&case, &scenario);
    }
}

#[test]
fn inline_box_width_fit_content_is_na() {
    for case in inline_component_cases() {
        let scenario = inline_scenarios()
            .into_iter()
            .find(|s| s.name == "width_fit_content")
            .expect("scenario exists");
        assert_box_is_na(&case, &scenario);
    }
}

#[test]
fn inline_box_width_fixed_pct_is_na() {
    for case in inline_component_cases() {
        let scenario = inline_scenarios()
            .into_iter()
            .find(|s| s.name == "width_fixed_pct_50")
            .expect("scenario exists");
        assert_box_is_na(&case, &scenario);
    }
}

#[test]
fn inline_box_padding_is_na() {
    for case in inline_component_cases() {
        let scenario = inline_scenarios()
            .into_iter()
            .find(|s| s.name == "padding_all_1")
            .expect("scenario exists");
        assert_box_is_na(&case, &scenario);
    }
}

// ── Honored assertions: style properties must lower to SGR ──

#[test]
fn inline_style_color_is_honored() {
    let scenario = inline_scenarios()
        .into_iter()
        .find(|s| s.name == "color_red")
        .expect("scenario exists");
    for case in inline_component_cases() {
        let out = (case.render)(&scenario);
        assert!(
            out.contains('\x1b'),
            "{} / color_red: color must lower to ANSI SGR",
            case.name
        );
    }
}

#[test]
fn inline_style_emphasis_is_honored() {
    let scenario = inline_scenarios()
        .into_iter()
        .find(|s| s.name == "emphasis_bold")
        .expect("scenario exists");
    for case in inline_component_cases() {
        let out = (case.render)(&scenario);
        assert!(
            out.contains("\x1b[1m"),
            "{} / emphasis_bold: bold must lower to \\x1b[1m SGR",
            case.name
        );
    }
}

#[test]
fn inline_style_background_is_honored() {
    let scenario = inline_scenarios()
        .into_iter()
        .find(|s| s.name == "background_inline")
        .expect("scenario exists");
    // Status has no inline background representation; it renders identically to
    // baseline. The other three components wrap a `Prose` background span.
    for case in inline_component_cases() {
        let out = (case.render)(&scenario);
        if case.name == "Status" {
            let base = baseline_output(&case, scenario.width);
            assert_eq!(
                strip_escape_codes(&out),
                base,
                "Status / background_inline: inline background is N/A"
            );
        } else {
            assert!(
                out.contains("\x1b[48;"),
                "{} / background_inline: inline background must lower to \\x1b[48; SGR",
                case.name
            );
        }
    }
}
