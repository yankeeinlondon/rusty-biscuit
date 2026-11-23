//! Embeds Claudine's registration inventory, `claudine/registration/`.
//!
//! The walk and member checks are `src/composition/inventory/walk.rs`, which
//! the library compiles too. A folder that is not a valid inventory fails
//! the build with every problem listed. Otherwise the build writes
//! `$OUT_DIR/registration_inventory.rs`: one `InventoryMember` per file, in
//! sorted relative-path order, each including its file's text. Cargo reruns
//! this script when the folder or any file in it changes (an addition,
//! deletion, move, or edit, at any depth) and when the walk itself changes.

use std::fmt::Write as _;
use std::path::PathBuf;

#[allow(dead_code)]
#[path = "src/composition/inventory/walk.rs"]
mod walk;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR"));
    let root = manifest_dir.join("../registration");
    println!("cargo:rerun-if-changed=src/composition/inventory/walk.rs");
    // A directory directive makes Cargo scan the whole tree for changed
    // modification times, which covers additions, deletions, and moves; each
    // file gets its own directive as well.
    println!("cargo:rerun-if-changed={}", root.display());
    let (members, directories) = match walk::walk(&root) {
        Ok(walked) => walked,
        Err(errors) => {
            let listed: Vec<String> = errors.iter().map(|error| format!("  - {error}")).collect();
            panic!("`claudine/registration/` is not a valid registration inventory:\n{}", listed.join("\n"));
        }
    };
    for directory in &directories {
        println!("cargo:rerun-if-changed={}", directory.display());
    }
    let mut table = String::from("&[\n");
    for member in &members {
        println!("cargo:rerun-if-changed={}", member.file.display());
        let file = member.file.to_str().expect("an inventory path is portable ASCII");
        let name = match &member.name {
            Some(name) => format!("Some({name:?})"),
            None => "None".to_string(),
        };
        let _ = writeln!(
            table,
            "    InventoryMember {{ path: {:?}, text: include_str!({file:?}), kind: MemberKind::{}, name: {name} }},",
            member.path,
            member.kind.variant()
        );
    }
    table.push(']');
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR")).join("registration_inventory.rs");
    std::fs::write(&out, table).unwrap_or_else(|error| panic!("cannot write `{}`: {error}", out.display()));
}
