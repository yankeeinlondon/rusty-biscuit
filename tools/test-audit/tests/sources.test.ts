/**
 * Tests for tree-sitter-based Rust source scanning.
 */
import { describe, it, expect } from "vitest";
import { scanRustSource, scanRustTests } from "../src/reconcile/sources.ts";

describe("Source attribute scan", () => {
  it("finds every attributed test form", async () => {
    const src = `
#[test]
fn plain() {}
#[tokio::test]
async fn asynchronous() {}
#[rstest]
fn parameterized() {}
#[bench]
fn benched(b: &mut Bencher) {}
fn not_a_test() {}
`;
    const names = (await scanRustTests(src, "p", "f.rs")).map((t) => t.name);
    expect(names).toEqual(["plain", "asynchronous", "parameterized", "benched"]);
  });

  it("records an ignore attribute", async () => {
    const src = '#[test]\n#[ignore = "harness"]\nfn slow() {}\n';
    expect((await scanRustTests(src, "p", "f.rs"))[0]!.ignored).toBe(true);
  });

  it("records a cfg on the function and keeps its string literal", async () => {
    const src = '#[cfg(target_os = "linux")]\n#[test]\nfn only_linux() {}\n';
    expect((await scanRustTests(src, "p", "f.rs"))[0]!.cfg).toEqual(['#[cfg(target_os = "linux")]']);
  });

  it("inherits a cfg from the enclosing module", async () => {
    const src = "#[cfg(windows)]\nmod windows_only {\n    #[test]\n    fn inside() {}\n}\n#[test]\nfn outside() {}\n";
    const found = await scanRustTests(src, "p", "f.rs");
    expect(found.map((t) => [t.name, t.cfg])).toEqual([
      ["inside", ["#[cfg(windows)]"]],
      ["outside", []],
    ]);
  });

  it("cfg_test is not reported as a platform gate", async () => {
    const src = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn inner() {}\n}\n";
    expect((await scanRustTests(src, "p", "f.rs"))[0]!.cfg).toEqual([]);
  });

  it("a test module quoted inside a raw string is a fixture not a test", async () => {
    const src = `
#[test]
fn analyzer_handles_nested_braces() {
    let source = r####"
#[cfg(all(test, unix))]
mod tests {
    #[test]
    fn inside_the_fixture() {
        assert_eq!('}', '}');
    }
}
"####;
    assert_eq!(count(source), 1);
}
`;
    const names = (await scanRustTests(src, "p", "f.rs")).map((t) => t.name);
    expect(names).toEqual(["analyzer_handles_nested_braces"]);
  });

  it("a macro_rules test template yields identities at its invocation sites", async () => {
    const src = `
macro_rules! per_provider_test {
    ($name:ident, $provider:literal) => {
        #[cfg(unix)]
        #[test]
        fn $name() {
            assert_provider($provider);
        }
    };
}

per_provider_test!(delivers_for_claude, "claude");
per_provider_test!(delivers_for_codex, "codex");
`;
    const found = await scanRustTests(src, "p", "f.rs");
    expect(found.map((t) => t.name)).toEqual(["delivers_for_claude", "delivers_for_codex"]);
    expect(found.every((t) => t.fromMacro)).toBe(true);
    expect(found[0]!.cfg).toEqual(["#[cfg(unix)]"]);
  });

  it("a macro that generates no test contributes no identity", async () => {
    const src = `
macro_rules! impl_thing {
    ($t:ident) => {
        impl Thing for $t {
            fn go(&self) {}
        }
    };
}
impl_thing!(Alpha, Beta);
#[test]
fn real() {}
`;
    expect((await scanRustTests(src, "p", "f.rs")).map((t) => t.name)).toEqual(["real"]);
  });

  it("the macro template itself is never an identity", async () => {
    const src = "macro_rules! t {\n    ($n:ident) => {\n        #[test]\n        fn $n() {}\n    };\n}\n";
    expect(await scanRustTests(src, "p", "f.rs")).toEqual([]);
  });

  it("an attribute separated from its function by a statement does not bind", async () => {
    const src = "#[test]\nlet x = 1;\nfn unattributed() {}\n";
    expect(await scanRustTests(src, "p", "f.rs")).toEqual([]);
  });
});

describe("Source scan: macro bodies, layout, and grammar gaps", () => {
  it("finds literal test items inside a proptest! body and marks them fromMacro", async () => {
    const src = `mod props {
    use proptest::prelude::*;
    proptest! {
        #[test]
        fn prop_one(input in any::<String>(), budget in 0usize..128) {
            let _ = (input, budget);
        }

        #[test]
        fn prop_two(a in 0..3) {}
    }
}
#[test]
fn plain() {}
`;
    const found = await scanRustTests(src, "p", "crate/src/lib.rs");
    expect(found.map((t) => [t.name, t.fromMacro, t.modulePath])).toEqual([
      ["prop_one", true, ["props"]],
      ["prop_two", true, ["props"]],
      ["plain", false, []],
    ]);
  });

  it("derives kind, target, and module path from the file layout", async () => {
    const src = "mod inner {\n    #[test]\n    fn t() {}\n}\n";
    const lib = (await scanRustTests(src, "p", "area/lib/src/a/b.rs"))[0]!;
    expect([lib.kind, lib.target, lib.identity]).toEqual(["lib", undefined, "a::b::inner::t"]);
    const modrs = (await scanRustTests(src, "p", "area/lib/src/a/mod.rs"))[0]!;
    expect(modrs.identity).toBe("a::inner::t");
    const integration = (await scanRustTests(src, "p", "area/cli/tests/toc.rs"))[0]!;
    expect([integration.kind, integration.target, integration.identity]).toEqual(["integration", "toc", "inner::t"]);
    const nested = (await scanRustTests(src, "p", "area/cli/tests/level2/support.rs"))[0]!;
    expect([nested.target, nested.identity]).toEqual(["level2", "support::inner::t"]);
    const bin = (await scanRustTests(src, "p", "area/cli/src/bin/md.rs"))[0]!;
    expect([bin.kind, bin.target]).toEqual(["bin", "md"]);
    const bench = (await scanRustTests(src, "p", "area/lib/benches/render.rs"))[0]!;
    expect([bench.kind, bench.target]).toEqual(["bench", "render"]);
  });

  it("records the ignore reason, the attribute list, and rstest cases as macro-expanded", async () => {
    const src = `#[rstest]
#[case(1)]
#[case(2)]
#[ignore = "needs a device"]
fn cased(#[case] n: u32) {}
`;
    const [t] = await scanRustTests(src, "p", "f.rs");
    expect(t!.macroExpanded).toBe(true);
    expect(t!.ignored).toBe(true);
    expect(t!.ignoreReason).toBe("needs a device");
    expect(t!.attributes).toEqual(["rstest", "case", "case", "ignore"]);
  });

  it("a grammar gap yields a parse-error diagnostic and keeps the neighboring test", async () => {
    // tree-sitter-rust 0.24 cannot parse `&raw` as a borrow of an identifier.
    const src = `fn helper(raw: &str) -> usize { let r = &raw; r.len() }
#[test]
fn still_found() { assert_eq!(helper("x"), 1); }
`;
    const scan = await scanRustSource(src, "p", "f.rs");
    expect(scan.tests.map((t) => t.name)).toEqual(["still_found"]);
    expect(scan.diagnostics.length).toBeGreaterThan(0);
    expect(scan.diagnostics[0]!.kind).toBe("parse-error");
    expect(scan.diagnostics[0]!.line).toBe(1);
  });

  it("a file-level #![cfg] gate is inherited by every test in the file", async () => {
    const src = "#![cfg(unix)]\n#[test]\nfn only_unix() {}\n";
    expect((await scanRustTests(src, "p", "f.rs"))[0]!.cfg).toEqual(["#[cfg(unix)]"]);
  });
});
