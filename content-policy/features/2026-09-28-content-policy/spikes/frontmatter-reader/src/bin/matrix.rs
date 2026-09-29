//! Fixture matrix: reader, locate, edit, re-read, and Darkmatter writer parity.

use darkmatter::markdown::hash::SaveDecision;
use darkmatter::markdown::hash::{StoredHash, StoredHashValue, apply_hash_save_text};
use darkmatter::markdown::{MdHashKind, MdHashOptions};
use frontmatter_reader_spike::*;
use serde_json::Value;

const NEW: &str = "2026-09-28";

#[derive(Clone, Copy)]
enum Op {
    Read,
    Prop,
    Rule(usize),
    Insert,
}

fn fixtures() -> Vec<(&'static str, String, Op)> {
    let base = "---\nhash: abc-def\nlast_updated: 2026-01-01\ncontent_policy:\n  - ValidFor(3mo, 2026-01-01)\n---\nBody\n";
    let crlf = base.replace('\n', "\r\n");
    let mut v: Vec<(&'static str, String, Op)> = vec![
        ("lf prop", base.into(), Op::Prop),
        ("lf rule", base.into(), Op::Rule(0)),
        ("crlf prop", crlf.clone(), Op::Prop),
        ("crlf rule", crlf.clone(), Op::Rule(0)),
        ("crlf insert", crlf.replace("last_updated: 2026-01-01\r\n", ""), Op::Insert),
        ("mixed prop", "---\r\nhash: abc-def\nlast_updated: 2026-01-01\r\ntitle: x\n---\r\nBody\n".into(), Op::Prop),
        ("mixed insert (last yaml line CRLF, fence LF)", "---\nhash: abc-def\ntitle: x\r\n---\nBody\n".into(), Op::Insert),
        ("lone CR prop", "---\rhash: abc-def\rlast_updated: 2026-01-01\r---\rBody\r".into(), Op::Prop),
        ("BOM prop", "\u{feff}---\nhash: abc-def\nlast_updated: 2026-01-01\n---\nBody\n".into(), Op::Prop),
        ("BOM, no frontmatter, insert", "\u{feff}# Title\n\nBody\n".into(), Op::Insert),
        ("closing `...` fence", "---\nhash: abc-def\nlast_updated: 2026-01-01\n...\nBody\n".into(), Op::Prop),
        ("closing `---` at EOF, no newline, prop", "---\nhash: abc-def\nlast_updated: 2026-01-01\n---".into(), Op::Prop),
        ("closing `---` at EOF, no newline, insert", "---\nhash: abc-def\n---".into(), Op::Insert),
        ("empty block insert", "---\n---\nBody\n".into(), Op::Insert),
        ("blank-line block insert", "---\n\n---\nBody\n".into(), Op::Insert),
        ("no frontmatter insert (LF)", "# Title\n\nBody\n".into(), Op::Insert),
        ("no frontmatter insert (CRLF)", "# Title\r\n\r\nBody\r\n".into(), Op::Insert),
        ("empty file insert", "".into(), Op::Insert),
        ("near-miss `----` fence insert", "----\ntitle: x\n----\nBody\n".into(), Op::Insert),
        // Rule forms.
        ("list item plain", rule_doc("  - ValidFor(3mo, 2026-01-01)"), Op::Rule(0)),
        ("list item single-quoted", rule_doc("  - 'ValidFor(3mo, 2026-01-01)'"), Op::Rule(0)),
        ("list item double-quoted", rule_doc("  - \"ValidFor(3mo, 2026-01-01)\""), Op::Rule(0)),
        ("list item double-quoted w/ escape", rule_doc("  - \"ValidFor(3mo,\\x202026-01-01)\""), Op::Rule(0)),
        ("list item zero-indent", rule_doc("- ValidFor(3mo, 2026-01-01)"), Op::Rule(0)),
        ("list item 2nd entry", rule_doc("  - Evergreen\n  - ValidFor(3mo, 2026-01-01)"), Op::Rule(1)),
        ("rule: plain", rule_doc("  - rule: ValidFor(3mo, 2026-01-01)\n    action: archive"), Op::Rule(0)),
        ("rule: single-quoted", rule_doc("  - rule: 'ValidFor(3mo, 2026-01-01)'\n    action: archive"), Op::Rule(0)),
        ("rule: double-quoted", rule_doc("  - rule: \"ValidFor(3mo, 2026-01-01)\"\n    action: archive"), Op::Rule(0)),
        ("rule: double-quoted w/ escape", rule_doc("  - rule: \"ValidFor(3mo,\\t2026-01-01)\"\n    action: archive"), Op::Rule(0)),
        ("rule: as 2nd key", rule_doc("  - action: archive\n    rule: ValidFor(3mo, 2026-01-01)"), Op::Rule(0)),
        ("rule: flow mapping item", rule_doc("  - {rule: \"ValidFor(3mo, 2026-01-01)\", action: archive}"), Op::Rule(0)),
        // Comments.
        ("prop trailing comment", "---\nhash: abc-def\nlast_updated: 2026-01-01 # reviewed\n---\n".into(), Op::Prop),
        ("prop quoted + trailing comment", "---\nhash: abc-def\nlast_updated: \"2026-01-01\"   # c\n---\n".into(), Op::Prop),
        ("prop trailing spaces", "---\nhash: abc-def\nlast_updated: 2026-01-01   \n---\n".into(), Op::Prop),
        ("list item trailing comment", rule_doc("  - ValidFor(3mo, 2026-01-01)  # renewed in place"), Op::Rule(0)),
        ("rule: trailing comment", rule_doc("  - rule: ValidFor(3mo, 2026-01-01) # c\n    action: archive"), Op::Rule(0)),
        ("comment line after key", rule_doc("  # reviewed quarterly\n  - ValidFor(3mo, 2026-01-01)"), Op::Rule(0)),
        ("comment on key line", "---\ncontent_policy: # note\n  - ValidFor(3mo, 2026-01-01)\n---\n".into(), Op::Rule(0)),
        // Null / odd property values.
        ("prop null (empty)", "---\nhash: abc-def\nlast_updated:\ntitle: x\n---\n".into(), Op::Prop),
        ("prop null (empty + comment)", "---\nhash: abc-def\nlast_updated:   # todo\n---\n".into(), Op::Prop),
        ("prop `~`", "---\nhash: abc-def\nlast_updated: ~\n---\n".into(), Op::Prop),
        ("prop `null`", "---\nhash: abc-def\nlast_updated: null\n---\n".into(), Op::Prop),
        ("prop single-quoted", "---\nhash: abc-def\nlast_updated: '2026-01-01'\n---\n".into(), Op::Prop),
        ("prop alias", "---\nhash: abc-def\nbase: &d 2026-01-01\nlast_updated: *d\n---\n".into(), Op::Prop),
        ("prop explicit tag", "---\nhash: abc-def\nlast_updated: !!str 2026-01-01\n---\n".into(), Op::Prop),
        ("prop after multibyte", "---\nhash: abc-def\ntitle: café ☕ 日本\nlast_updated: 2026-01-01\n---\n".into(), Op::Prop),
        ("prop only nested", "---\nhash: abc-def\nmeta:\n  last_updated: 2026-01-01\n---\n".into(), Op::Prop),
        ("prop key quoted", "---\nhash: abc-def\n\"last_updated\": 2026-01-01\n---\n".into(), Op::Prop),
        // Refusals.
        ("block scalar item", rule_doc("  - >-\n    ValidFor(3mo, 2026-01-01)"), Op::Rule(0)),
        ("multi-line plain item", rule_doc("  - ValidFor(3mo,\n    2026-01-01)"), Op::Rule(0)),
        ("flow list, edit last_updated", "---\nhash: abc-def\nlast_updated: 2026-01-01\ncontent_policy: [ValidFor(3mo, @last_updated)]\n---\n".into(), Op::Prop),
        ("flow list, edit rule date", "---\nlast_updated: 2026-01-01\ncontent_policy: [\"ValidFor(3mo, 2026-01-01)\"]\n---\n".into(), Op::Rule(0)),
        ("tab-indented list (Darkmatter fallback)", "---\nlast_updated: 2026-01-01\ncontent_policy:\n\t- ValidFor(3mo, 2026-01-01)\n---\n".into(), Op::Rule(0)),
        ("duplicate key", "---\nlast_updated: 2026-01-01\nlast_updated: 2026-02-01\n---\n".into(), Op::Prop),
        ("no frontmatter read", "# Title\n".into(), Op::Read),
        ("flow list quoted @ref, edit last_updated", "---\nhash: abc-def\nlast_updated: 2026-01-01\ncontent_policy: [\"ValidFor(3mo, @last_updated)\"]\n---\n".into(), Op::Prop),
        ("block list @ref (plain)", "---\nhash: abc-def\nlast_updated: 2026-01-01\ncontent_policy:\n  - ValidFor(3mo, @last_updated)\n---\n".into(), Op::Prop),
        ("anchor on target", "---\nhash: abc-def\nlast_updated: &lu 2026-01-01\nreviewed: *lu\n---\n".into(), Op::Prop),
        ("LF line in CRLF file, prop", "---\r\nhash: abc-def\r\nlast_updated: 2026-01-01\n---\r\nBody\r\n".into(), Op::Prop),
        ("LF block, CRLF body, insert", "---\nhash: abc-def\ntitle: x\n---\nBody\r\n".into(), Op::Insert),
        ("CRLF null value", "---\r\nhash: abc-def\r\nlast_updated:\r\n---\r\n".into(), Op::Prop),
        ("CRLF rule: + comment", rule_doc("  - rule: 'ValidFor(3mo, 2026-01-01)' # c\n    action: archive").replace('\n', "\r\n"), Op::Rule(0)),
        ("multi-line plain, date on 1st line", rule_doc("  - ValidFor(3mo, 2026-01-01\n    )"), Op::Rule(0)),
        ("YAML 1.1-ish values", "---\na_yes: yes\nb_on: on\nc_date: 2026-09-28\nd_0o7: 0o7\ne_hex: 0x1F\nf_octal_010: 010\ng_tilde: ~\nh_1e3: 1e3\ni_inf: .inf\nj_nan: .nan\nk_ts: 2026-09-28T10:00:00Z\nl_str_tag: !!str 5\nm_True: True\nn_y: y\n---\n".into(), Op::Read),
    ];
    v.shrink_to_fit();
    v
}

fn rule_doc(items: &str) -> String {
    format!("---\ntitle: t\ncontent_policy:\n{items}\n---\nBody\n")
}

fn show(bytes: &[u8]) -> String {
    format!("{:?}", String::from_utf8_lossy(bytes))
}

fn record_of(bytes: &[u8]) -> Option<Record> {
    match read_frontmatter(bytes) {
        Ok(ReadOutcome::Found(fm)) => Some(fm.record),
        Ok(_) => Some(Record::new()),
        Err(_) => None,
    }
}

fn expected_record(old: &Record, op: Op) -> Record {
    let mut r = old.clone();
    match op {
        Op::Prop | Op::Insert => {
            r.insert("last_updated".into(), Value::String(NEW.into()));
        }
        Op::Rule(i) => {
            let item = &mut r["content_policy"][i];
            let s = item
                .as_str()
                .map(String::from)
                .or_else(|| item["rule"].as_str().map(String::from))
                .unwrap();
            let old_date = valid_for_inline_date(&s).unwrap();
            let new = Value::String(s.replace(&old_date, NEW));
            if item.is_string() {
                *item = new
            } else {
                item["rule"] = new
            }
        }
        Op::Read => {}
    }
    r
}

fn dm_reference(src: &str) -> Result<String, String> {
    let decision = SaveDecision {
        kind: MdHashKind::Simple,
        new_stored: Some(StoredHash {
            kind: MdHashKind::Simple,
            value: StoredHashValue::Flat("abc-def".into()),
            ignored: vec![],
        }),
        bump_last_updated: true,
        comparison: None,
    };
    let out = apply_hash_save_text(src, &decision, &MdHashOptions::default(), NEW)
        .map_err(|e| e.to_string())?
        .ok_or("no change")?;
    if src.contains("hash: abc-def") {
        Ok(out)
    } else {
        // Darkmatter also writes the managed hash line; strip it to isolate
        // the `last_updated` behavior.
        Ok(out
            .replacen("hash: abc-def\r\n", "", 1)
            .replacen("hash: abc-def\n", "", 1))
    }
}

fn main() {
    for (name, src, op) in fixtures() {
        let bytes = src.as_bytes();
        println!("## {name}");
        println!("   src: {}", show(bytes));
        let read = read_frontmatter(bytes);
        match &read {
            Ok(ReadOutcome::Found(fm)) => println!(
                "   read: Found block_span={:?} record={}",
                fm.block_span,
                serde_json::to_string(&fm.record).unwrap()
            ),
            Ok(other) => println!("   read: {other:?}"),
            Err(e) => println!("   read: ERR {e:?}"),
        }
        let result = match op {
            Op::Read => {
                println!();
                continue;
            }
            Op::Prop => {
                if let Ok(p) = plan_edit_date(bytes, &Target::Property("last_updated".into()), NEW)
                {
                    println!(
                        "   located: {:?}",
                        p.located.as_ref().map(|r| &src[r.clone()])
                    );
                }
                set_date_property(bytes, "last_updated", NEW)
            }
            Op::Insert => insert_property(bytes, "last_updated", NEW),
            Op::Rule(i) => {
                let t = Target::RuleDate {
                    key: "content_policy".into(),
                    index: i,
                };
                if let Ok(p) = plan_edit_date(bytes, &t, NEW) {
                    println!(
                        "   located: {:?} edit={:?}",
                        p.located.as_ref().map(|r| &src[r.clone()]),
                        &src[p.span.clone()]
                    );
                }
                edit_date(bytes, &t, NEW)
            }
        };
        match result {
            Err(e) => println!("   edit: REFUSED {e:?}"),
            Ok(out) => {
                let (o, n) = byte_diff(bytes, &out);
                println!(
                    "   edit: old[{o:?}]={} -> new={}",
                    show(&bytes[o.clone()]),
                    show(&out[n.clone()])
                );
                let valid = match (record_of(bytes), record_of(&out)) {
                    (Some(old), Some(new)) => {
                        if new == expected_record(&old, op) {
                            "record OK (only target changed)".to_string()
                        } else {
                            format!("RECORD MISMATCH {}", serde_json::to_string(&new).unwrap())
                        }
                    }
                    (_, None) => "RESULT DOES NOT PARSE".into(),
                    _ => "n/a".into(),
                };
                println!("   check: {valid}");
                if matches!(op, Op::Prop | Op::Insert) {
                    match dm_reference(&src) {
                        Ok(dm) if dm.as_bytes() == out.as_slice() => {
                            println!("   darkmatter: IDENTICAL")
                        }
                        Ok(dm) => {
                            let dm_rec = record_of(dm.as_bytes());
                            let verdict = match (&dm_rec, record_of(bytes)) {
                                (None, _) => "DM OUTPUT DOES NOT PARSE".to_string(),
                                (Some(r), Some(old)) if *r == expected_record(&old, op) => {
                                    "dm record OK".into()
                                }
                                (Some(r), _) => {
                                    format!("DM RECORD WRONG {}", serde_json::to_string(r).unwrap())
                                }
                            };
                            println!(
                                "   darkmatter: DIFFERS ({verdict}) dm={}",
                                show(dm.as_bytes())
                            )
                        }
                        Err(e) => println!("   darkmatter: ERR {e}"),
                    }
                }
            }
        }
        println!();
    }
}
