//! `validate::Decimal`: exact parsing, ordering, equality, divisibility, and
//! the exponent bound.

use schematic_runtime::validate::{Decimal, DecimalError, MAX_EXPONENT};

fn decimal(text: &str) -> Decimal {
    Decimal::parse(text).unwrap_or_else(|error| panic!("{text} should parse: {error}"))
}

#[test]
fn integers_beyond_u64_and_i64_stay_exact_and_ordered() {
    let u64_max = decimal("18446744073709551615");
    let past_u64 = decimal("18446744073709551616");
    let i64_min = decimal("-9223372036854775808");
    let below_i64 = decimal("-9223372036854775809");

    assert!(past_u64 > u64_max);
    assert!(below_i64 < i64_min);
    assert_ne!(past_u64, u64_max, "no rounding to a nearby float");
    assert_eq!(past_u64.to_string(), "18446744073709551616");
    assert_eq!(decimal("9223372036854775808").to_string(), "9223372036854775808");
    assert_eq!(decimal("100000000000000000000").to_string(), "100000000000000000000");
}

#[test]
fn equal_values_in_different_spellings_are_equal() {
    let one = decimal("1");
    for spelling in ["1.0", "1e0", "1E0", "1.000", "10e-1", "0.1e1", "1e+0"] {
        assert_eq!(decimal(spelling), one, "{spelling}");
    }
    assert_eq!(decimal("-0"), decimal("0"));
    assert_eq!(decimal("0.0e5"), decimal("0"));
    assert!(!decimal("-0").is_negative());
    assert_eq!(decimal("1e30"), decimal("1000000000000000000000000000000"));
    assert_eq!(decimal("0.10000000000000000001").to_string(), "0.10000000000000000001", "beyond f64 digits");
}

#[test]
fn ordering_is_mathematical_across_signs_and_scales() {
    let ascending = ["-1e30", "-2", "-1.5", "-0.001", "0", "0.001", "0.1", "0.35", "1", "1.5", "2", "10", "1e30"];
    for pair in ascending.windows(2) {
        assert!(decimal(pair[0]) < decimal(pair[1]), "{} < {}", pair[0], pair[1]);
    }
    assert!(decimal("-1.5") < decimal("-1.25"));
    assert!(decimal("99") < decimal("100"));
    assert!(decimal("0.999999") < decimal("1"));
}

#[test]
fn divisibility_by_a_decimal_divisor_is_exact() {
    let tenth = decimal("0.1");
    assert_eq!(decimal("0.3").is_multiple_of(&tenth), Some(true));
    assert_eq!(decimal("0.35").is_multiple_of(&tenth), Some(false));
    assert_eq!(decimal("-0.3").is_multiple_of(&tenth), Some(true), "negative dividends");
    assert_eq!(decimal("0").is_multiple_of(&tenth), Some(true));
    assert_eq!(decimal("7").is_multiple_of(&tenth), Some(true));
    assert_eq!(decimal("1e30").is_multiple_of(&decimal("7")), Some(false));
    assert_eq!(decimal("18446744073709551616").is_multiple_of(&decimal("2")), Some(true));
    assert_eq!(decimal("4.5").is_multiple_of(&decimal("1.5")), Some(true));
    assert_eq!(decimal("4.6").is_multiple_of(&decimal("1.5")), Some(false));
}

#[test]
fn a_divisor_that_is_not_positive_has_no_answer() {
    assert_eq!(decimal("3").is_multiple_of(&decimal("0")), None);
    assert_eq!(decimal("3").is_multiple_of(&decimal("-1")), None);
}

#[test]
fn the_exponent_bound_is_inclusive_and_refuses_beyond_it() {
    assert_eq!(MAX_EXPONENT, 10_000);
    assert!(Decimal::parse("1e10000").is_ok());
    assert!(Decimal::parse("1e-10000").is_ok());
    assert!(Decimal::parse("10e9999").is_ok(), "the canonical exponent is bounded, not the written one");
    for text in ["1e10001", "1e-10001", "10e10000", "1e99999999999999999999999"] {
        assert_eq!(Decimal::parse(text), Err(DecimalError::ExponentOutOfRange), "{text}");
    }
    assert_eq!(Decimal::parse("0e99999").map(|zero| zero.is_zero()), Ok(true), "zero has no magnitude");
}

#[test]
fn text_that_is_not_a_json_number_is_a_syntax_error() {
    for text in ["", "-", "+1", "01", "-01", ".5", "5.", "1e", "1e+", "1.5.2", " 1", "1 ", "0x10", "NaN", "Infinity", "1_000"] {
        assert_eq!(Decimal::parse(text), Err(DecimalError::Syntax), "{text:?}");
    }
}

#[test]
fn display_is_canonical_and_switches_to_scientific_notation_for_extremes() {
    let cases = [
        ("1.50", "1.5"),
        ("-2.0", "-2"),
        ("0.001", "0.001"),
        ("0.0000001", "1e-7"),
        ("1.5e30", "1.5e30"),
        ("123e-2", "1.23"),
        ("1e20", "100000000000000000000"),
        ("1e21", "1e21"),
        ("0.000001", "0.000001"),
        ("-0", "0"),
    ];
    for (text, shown) in cases {
        assert_eq!(decimal(text).to_string(), shown, "{text}");
        assert_eq!(decimal(shown), decimal(text), "{shown} parses back to the same value");
    }
}

#[test]
fn a_json_float_reads_as_the_text_serde_json_writes() {
    let point_three: serde_json::Value = serde_json::from_str("0.3").unwrap();
    let number = point_three.as_number().unwrap();
    assert_eq!(Decimal::from_json(number).unwrap(), decimal("0.3"));
    assert_eq!(Decimal::from_json(&serde_json::Number::from(u64::MAX)).unwrap(), decimal("18446744073709551615"));
    assert_eq!(Decimal::from_json(&serde_json::Number::from(i64::MIN)).unwrap(), decimal("-9223372036854775808"));
    let big: serde_json::Value = serde_json::from_str("1e30").unwrap();
    assert_eq!(Decimal::from_json(big.as_number().unwrap()).unwrap(), decimal("1e30"));
}
