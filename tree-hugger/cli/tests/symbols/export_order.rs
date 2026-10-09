//! Record order of the flat exports (TSV and ctags): a `classes` export is
//! one sequence of containers and members sorted together, never container
//! blocks. Expected sequences are written out from the `--sort` contract
//! (case-sensitive byte order of names; kind rank, then declaration position).

use crate::Scratch;

const ORDER_JAVA: &str = "class Zebra { void aardvark() {} }\nclass Alpha { void zulu() {} }\n";
const STATIC_JAVA: &str = "class Zebra { static void yak() {} void aardvark() {} }\nclass Alpha { static void beta() {} void zulu() {} }\n";

fn tsv_names(scratch: &Scratch, args: &[&str]) -> Vec<String> {
    let output = scratch.hug(&[args, &["--format", "tsv"]].concat());
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .skip(1)
        .map(|line| line.split('\t').nth(4).expect("name cell").to_owned())
        .collect()
}

fn ctags_names(scratch: &Scratch, args: &[&str]) -> Vec<String> {
    let output = scratch.hug(&[args, &["--format", "ctags"]].concat());
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with("!_"))
        .map(|line| line.split('\t').next().expect("tag name").to_owned())
        .collect()
}

#[test]
fn classes_exports_sort_containers_and_members_as_one_sequence() {
    let scratch = Scratch::new(&[("Order.java", ORDER_JAVA)]);
    let cases: [(&str, [&str; 4]); 3] = [
        ("name", ["Alpha", "Zebra", "aardvark", "zulu"]),
        ("kind", ["Zebra", "Alpha", "aardvark", "zulu"]),
        ("file", ["Zebra", "aardvark", "Alpha", "zulu"]),
    ];
    for (sort, expected) in cases {
        let args = ["classes", "Order.java", "--sort", sort];
        assert_eq!(tsv_names(&scratch, &args), expected, "classes TSV --sort {sort}");
        assert_eq!(ctags_names(&scratch, &args), expected, "classes ctags --sort {sort}");
    }
}

#[test]
fn symbols_exports_order_the_same_declarations_identically() {
    let scratch = Scratch::new(&[("Order.java", ORDER_JAVA)]);
    let cases: [(&str, [&str; 4]); 3] = [
        ("name", ["Alpha", "Zebra", "aardvark", "zulu"]),
        ("kind", ["Zebra", "Alpha", "aardvark", "zulu"]),
        ("file", ["Zebra", "aardvark", "Alpha", "zulu"]),
    ];
    for (sort, expected) in cases {
        let args = ["symbols", "Order.java", "--sort", sort];
        assert_eq!(tsv_names(&scratch, &args), expected, "symbols TSV --sort {sort}");
        assert_eq!(ctags_names(&scratch, &args), expected, "symbols ctags --sort {sort}");
    }
}

#[test]
fn classes_exports_keep_static_and_instance_membership_under_one_order() {
    let scratch = Scratch::new(&[("Order.java", STATIC_JAVA)]);
    let cases: [(&str, &str, [&str; 4]); 4] = [
        ("--static-only", "name", ["Alpha", "Zebra", "beta", "yak"]),
        ("--instance-only", "name", ["Alpha", "Zebra", "aardvark", "zulu"]),
        ("--static-only", "file", ["Zebra", "yak", "Alpha", "beta"]),
        ("--instance-only", "kind", ["Zebra", "Alpha", "aardvark", "zulu"]),
    ];
    for (partition, sort, expected) in cases {
        let args = ["classes", "Order.java", partition, "--sort", sort];
        assert_eq!(tsv_names(&scratch, &args), expected, "TSV {partition} --sort {sort}");
        assert_eq!(ctags_names(&scratch, &args), expected, "ctags {partition} --sort {sort}");
    }
}
