/**
 * Tests for `inventory-reconciler.ts`.
 *
 * Run with `npx tsx --test inventory-reconciler.test.ts` from this directory.
 * `node:test` rather than Vitest: the monorepo ships no JavaScript test runner
 * and this fix is not the place to introduce one — the same call the
 * predecessor's `junit-metrics.test.ts` makes.
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
    MalformedInput,
    blankLiteralsAndComments,
    scanRustTests,
    parseManifest,
    parseCapture,
    buildUniverse,
    parseFamilyFile,
    familiesMatching,
    reconcileFamilies,
    parseInventoryFamilyIndex,
    checkInventory,
    diffSourceAgainstRunner,
    checkExclusions,
    scanSourcePopulation,
    CLAUDINE_PACKAGE_ROOTS,
    main,
    type Family,
    type Identity,
    type Universe,
} from './inventory-reconciler.ts';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(HERE, '..', '..', '..');
const ENUMERATION = join(HERE, 'enumeration');
const FAMILIES = join(HERE, 'families.json');
const INVENTORY = join(HERE, 'inventory.md');

function tempDir(): string {
    return mkdtempSync(join(tmpdir(), 'inventory-reconciler-'));
}

function suite(binaryId: string, pkg: string, kind: string, names: string[]): Record<string, unknown> {
    return {
        'package-name': pkg,
        'binary-id': binaryId,
        kind,
        testcases: Object.fromEntries(names.map((n) => [n, { ignored: false }])),
    };
}

function listing(...suites: Record<string, unknown>[]): string {
    return JSON.stringify({
        'rust-suites': Object.fromEntries(suites.map((s) => [s['binary-id'] as string, s])),
    });
}

const MANIFEST = (labels: string[]) =>
    JSON.stringify({
        revision: 'deadbeef',
        toolchain: 'rustc 1.97.1',
        nextest: 'cargo-nextest 0.9.136',
        platform: 'aarch64-apple-darwin',
        captures: labels.map((label) => ({ label, packages: ['p'], features: [], command: `cargo nextest list ${label}` })),
    });

function identity(binaryId: string, name: string, pkg = 'claudine'): Identity {
    return { id: `${binaryId} :: ${name}`, pkg, binaryId, kind: 'lib', name, captures: ['c'] };
}

function universeOf(identities: Identity[]): Universe {
    return { identities: new Map(identities.map((i) => [i.id, i])), emptySuites: [], violations: [] };
}

// ---------------------------------------------------------------------------
// Rust lexing
// ---------------------------------------------------------------------------

test('blanking_preserves_length_and_line_structure', () => {
    const src = 'let a = "one";\n// two\nlet b = 3;\n';
    const out = blankLiteralsAndComments(src);
    assert.equal(out.length, src.length);
    assert.equal(out.split('\n').length, src.split('\n').length);
    // `"one"` is five characters, blanked in place after the existing space.
    assert.match(out, /^let a = {6};$/m);
    assert.equal(out.split('\n')[1].trim(), '');
});

test('a_nested_block_comment_is_blanked_to_its_true_end', () => {
    const src = '/* outer /* inner */ still comment */ fn real() {}';
    const out = blankLiteralsAndComments(src);
    assert.ok(out.includes('fn real() {}'));
    assert.ok(!out.includes('inner'));
});

test('a_raw_string_with_hashes_is_blanked_whole', () => {
    const src = 'let s = r####"a "###" b"####; fn after() {}';
    const out = blankLiteralsAndComments(src);
    assert.ok(!out.includes('a "'));
    assert.ok(out.includes('fn after() {}'));
});

test('a_byte_raw_string_is_blanked_like_a_raw_string', () => {
    const out = blankLiteralsAndComments('let s = br#"x"#; fn after() {}');
    assert.ok(!out.includes('x'));
    assert.ok(out.includes('fn after() {}'));
});

test('an_escaped_quote_does_not_end_a_string', () => {
    const out = blankLiteralsAndComments('let s = "a\\"b"; fn after() {}');
    assert.ok(!out.includes('b'));
    assert.ok(out.includes('fn after() {}'));
});

test('a_lifetime_is_not_a_char_literal', () => {
    const out = blankLiteralsAndComments("fn f<'a>(x: &'a str) {}");
    assert.equal(out, "fn f<'a>(x: &'a str) {}");
});

test('a_char_literal_holding_a_brace_is_blanked_so_brace_counting_survives', () => {
    const out = blankLiteralsAndComments("let c = '}';");
    assert.ok(!out.includes('}'));
});

test('rejects_an_unterminated_block_comment', () => {
    assert.throws(() => blankLiteralsAndComments('/* never closed'), MalformedInput);
});

test('rejects_an_unterminated_string_literal', () => {
    assert.throws(() => blankLiteralsAndComments('let s = "open'), MalformedInput);
});

test('rejects_an_unterminated_raw_string', () => {
    assert.throws(() => blankLiteralsAndComments('let s = r#"open'), MalformedInput);
});

// ---------------------------------------------------------------------------
// Source attribute scan
// ---------------------------------------------------------------------------

test('finds_every_attributed_test_form', () => {
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
    const names = scanRustTests(src, 'p', 'f.rs').map((t) => t.name);
    assert.deepEqual(names, ['plain', 'asynchronous', 'parameterized', 'benched']);
});

test('records_an_ignore_attribute', () => {
    const src = '#[test]\n#[ignore = "harness"]\nfn slow() {}\n';
    assert.equal(scanRustTests(src, 'p', 'f.rs')[0].ignored, true);
});

test('records_a_cfg_on_the_function_and_keeps_its_string_literal', () => {
    const src = '#[cfg(target_os = "linux")]\n#[test]\nfn only_linux() {}\n';
    assert.deepEqual(scanRustTests(src, 'p', 'f.rs')[0].cfg, ['#[cfg(target_os = "linux")]']);
});

test('inherits_a_cfg_from_the_enclosing_module', () => {
    const src = '#[cfg(windows)]\nmod windows_only {\n    #[test]\n    fn inside() {}\n}\n#[test]\nfn outside() {}\n';
    const found = scanRustTests(src, 'p', 'f.rs');
    assert.deepEqual(found.map((t) => [t.name, t.cfg]), [
        ['inside', ['#[cfg(windows)]']],
        ['outside', []],
    ]);
});

test('cfg_test_is_not_reported_as_a_platform_gate', () => {
    const src = '#[cfg(test)]\nmod tests {\n    #[test]\n    fn inner() {}\n}\n';
    assert.deepEqual(scanRustTests(src, 'p', 'f.rs')[0].cfg, []);
});

test('a_test_module_quoted_inside_a_raw_string_is_a_fixture_not_a_test', () => {
    // The exact shape of `cli/tests/test_placement.rs`'s analyzer fixture: a
    // scanner without literal handling reports `inside_the_fixture` as real.
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
    const names = scanRustTests(src, 'p', 'f.rs').map((t) => t.name);
    assert.deepEqual(names, ['analyzer_handles_nested_braces']);
});

test('a_macro_rules_test_template_yields_identities_at_its_invocation_sites', () => {
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
    const found = scanRustTests(src, 'p', 'f.rs');
    assert.deepEqual(found.map((t) => t.name), ['delivers_for_claude', 'delivers_for_codex']);
    assert.ok(found.every((t) => t.fromMacro));
    assert.deepEqual(found[0].cfg, ['#[cfg(unix)]']);
});

test('a_macro_that_generates_no_test_contributes_no_identity', () => {
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
    assert.deepEqual(scanRustTests(src, 'p', 'f.rs').map((t) => t.name), ['real']);
});

test('the_macro_template_itself_is_never_an_identity', () => {
    const src = 'macro_rules! t {\n    ($n:ident) => {\n        #[test]\n        fn $n() {}\n    };\n}\n';
    assert.deepEqual(scanRustTests(src, 'p', 'f.rs'), []);
});

test('an_attribute_separated_from_its_function_by_a_statement_does_not_bind', () => {
    const src = '#[test]\nlet x = 1;\nfn unattributed() {}\n';
    assert.deepEqual(scanRustTests(src, 'p', 'f.rs'), []);
});

// ---------------------------------------------------------------------------
// Manifest and captures
// ---------------------------------------------------------------------------

test('rejects_a_manifest_that_is_not_json', () => {
    assert.throws(() => parseManifest('{not json'), MalformedInput);
});

test('rejects_a_manifest_with_no_captures', () => {
    assert.throws(() => parseManifest(JSON.stringify({ revision: 'a', toolchain: 'b', nextest: 'c', platform: 'd', captures: [] })), MalformedInput);
});

test('rejects_a_manifest_missing_provenance_fields', () => {
    assert.throws(
        () => parseManifest(JSON.stringify({ revision: 'a', captures: [{ label: 'x', packages: [], command: 'y' }] })),
        MalformedInput,
    );
});

test('rejects_a_capture_that_is_not_a_nextest_listing', () => {
    assert.throws(() => parseCapture('c', JSON.stringify({ suites: {} })), MalformedInput);
});

test('rejects_a_capture_that_is_not_json', () => {
    assert.throws(() => parseCapture('c', '<html>'), MalformedInput);
});

test('rejects_a_suite_with_no_testcases_object', () => {
    const text = JSON.stringify({ 'rust-suites': { a: { 'package-name': 'p', 'binary-id': 'a', kind: 'lib' } } });
    assert.throws(() => parseCapture('c', text), MalformedInput);
});

test('records_a_build_target_that_lists_no_test_instead_of_dropping_it', () => {
    const parsed = parseCapture('c', listing(suite('p::bin/p', 'p', 'bin', [])));
    assert.deepEqual(parsed.identities, []);
    assert.deepEqual(parsed.empty.map((e) => e.binaryId), ['p::bin/p']);
});

test('a_union_across_captures_counts_each_identity_once', () => {
    const dir = tempDir();
    writeFileSync(join(dir, 'captures.json'), MANIFEST(['bare', 'featured']));
    writeFileSync(join(dir, 'bare.json'), listing(suite('p', 'p', 'lib', ['a', 'b'])));
    writeFileSync(join(dir, 'featured.json'), listing(suite('p', 'p', 'lib', ['a', 'b', 'c'])));
    const universe = buildUniverse(dir, parseManifest(readFileSync(join(dir, 'captures.json'), 'utf8')));
    assert.equal(universe.identities.size, 3);
    assert.deepEqual(universe.identities.get('p :: a')!.captures, ['bare', 'featured']);
    assert.deepEqual(universe.identities.get('p :: c')!.captures, ['featured']);
    assert.deepEqual(universe.violations, []);
    rmSync(dir, { recursive: true });
});

test('rejects_a_declared_capture_whose_file_is_missing', () => {
    const dir = tempDir();
    writeFileSync(join(dir, 'captures.json'), MANIFEST(['present', 'absent']));
    writeFileSync(join(dir, 'present.json'), listing(suite('p', 'p', 'lib', ['a'])));
    const universe = buildUniverse(dir, parseManifest(readFileSync(join(dir, 'captures.json'), 'utf8')));
    assert.deepEqual(universe.violations.map((v) => v.kind), ['missing-capture']);
    rmSync(dir, { recursive: true });
});

test('rejects_a_capture_file_that_no_manifest_record_declares', () => {
    const dir = tempDir();
    writeFileSync(join(dir, 'captures.json'), MANIFEST(['declared']));
    writeFileSync(join(dir, 'declared.json'), listing(suite('p', 'p', 'lib', ['a'])));
    writeFileSync(join(dir, 'smuggled.json'), listing(suite('q', 'q', 'lib', ['z'])));
    const universe = buildUniverse(dir, parseManifest(readFileSync(join(dir, 'captures.json'), 'utf8')));
    assert.deepEqual(universe.violations.map((v) => v.kind), ['malformed-capture']);
    assert.equal(universe.identities.has('q :: z'), false);
    rmSync(dir, { recursive: true });
});

test('rejects_one_listing_that_names_an_identity_twice', () => {
    // A duplicate *object key* cannot reach the parser — `JSON.parse` collapses
    // it — so the reachable duplicate is two suite records sharing a binary-id,
    // which is what an accidentally merged listing looks like.
    const text =
        '{"rust-suites":{"one":{"package-name":"p","binary-id":"p","kind":"lib","testcases":{"a":{}}},' +
        '"two":{"package-name":"p","binary-id":"p","kind":"lib","testcases":{"a":{}}}}}';
    assert.throws(() => parseCapture('c', text), MalformedInput);
});

// ---------------------------------------------------------------------------
// Family matching
// ---------------------------------------------------------------------------

const FAM = (id: string, match: Family['match'], exclude?: Family['exclude']): Family => ({
    id,
    package: 'claudine',
    match,
    exclude,
});

test('a_suite_only_family_claims_the_whole_suite', () => {
    const hits = familiesMatching(identity('claudine', 'config::tests::a'), [FAM('all', { suites: ['claudine'] })]);
    assert.deepEqual(hits.map((h) => h.id), ['all']);
});

test('a_suite_plus_module_family_claims_only_that_module', () => {
    const families = [FAM('narrow', { suites: ['claudine'], modules: ['config::atomic'] })];
    assert.equal(familiesMatching(identity('claudine', 'config::atomic::tests::a'), families).length, 1);
    assert.equal(familiesMatching(identity('claudine', 'config::claude::tests::a'), families).length, 0);
});

test('a_module_prefix_stops_at_a_path_separator', () => {
    const families = [FAM('narrow', { modules: ['config::atomic'] })];
    assert.equal(familiesMatching(identity('claudine', 'config::atomicity::tests::a'), families).length, 0);
    assert.equal(familiesMatching(identity('claudine', 'config::atomic'), families).length, 1);
});

test('a_family_never_reaches_another_package', () => {
    const families = [FAM('lib', { suites: ['claudine'] })];
    const other: Identity = { ...identity('claudine', 'a'), pkg: 'claudine-cli' };
    assert.equal(familiesMatching(other, families).length, 0);
});

test('an_exclude_block_removes_a_carve_out_from_its_parent', () => {
    const parent = FAM('parent', { suites: ['claudine'] }, { modules: ['config::atomic'] });
    const child = FAM('child', { suites: ['claudine'], modules: ['config::atomic'] });
    const id = identity('claudine', 'config::atomic::tests::a');
    assert.deepEqual(familiesMatching(id, [parent, child]).map((f) => f.id), ['child']);
});

test('an_exact_test_entry_matches_the_full_identity_name', () => {
    const families = [FAM('one', { suites: ['claudine'], tests: ['a::b::c'] })];
    assert.equal(familiesMatching(identity('claudine', 'a::b::c'), families).length, 1);
    assert.equal(familiesMatching(identity('claudine', 'a::b::c::d'), families).length, 0);
});

test('an_identity_matched_by_no_family_is_a_violation', () => {
    const result = reconcileFamilies(universeOf([identity('claudine', 'orphan')]), [FAM('other', { suites: ['nothing'] })]);
    assert.deepEqual(result.violations.map((v) => v.kind), ['unassigned-identity', 'stale-family']);
});

test('an_identity_matched_by_two_families_is_a_violation_and_is_counted_by_neither', () => {
    const families = [FAM('a', { suites: ['claudine'] }), FAM('b', { modules: ['config'] })];
    const result = reconcileFamilies(universeOf([identity('claudine', 'config::x')]), families);
    const doubles = result.violations.filter((v) => v.kind === 'double-assigned-identity');
    assert.equal(doubles.length, 1);
    assert.match(doubles[0].detail, /a, b/);
    assert.equal(result.counts.get('a'), 0);
    assert.equal(result.counts.get('b'), 0);
});

test('a_family_matching_nothing_is_stale_unless_it_declares_why', () => {
    const empty = FAM('empty', { suites: ['gone'] });
    const declared: Family = { ...FAM('declared', { suites: ['gone'] }), expectEmpty: 'windows-only suite' };
    const withStale = reconcileFamilies(universeOf([identity('claudine', 'a')]), [FAM('real', { suites: ['claudine'] }), empty]);
    assert.ok(withStale.violations.some((v) => v.kind === 'stale-family'));
    const withDeclared = reconcileFamilies(universeOf([identity('claudine', 'a')]), [FAM('real', { suites: ['claudine'] }), declared]);
    assert.equal(withDeclared.violations.length, 0);
});

test('rejects_a_family_file_with_duplicate_ids', () => {
    const text = JSON.stringify({
        families: [
            { id: 'x', package: 'p', match: { suites: ['a'] } },
            { id: 'x', package: 'p', match: { suites: ['b'] } },
        ],
    });
    assert.throws(() => parseFamilyFile(text), MalformedInput);
});

test('rejects_a_family_with_an_empty_match_block', () => {
    const text = JSON.stringify({ families: [{ id: 'x', package: 'p', match: {} }] });
    assert.throws(() => parseFamilyFile(text), MalformedInput);
});

// ---------------------------------------------------------------------------
// inventory.md cross-check
// ---------------------------------------------------------------------------

test('reads_the_family_index_regardless_of_column_order', () => {
    const md = [
        '| Disposition | Identities | Family | Package |',
        '|---|---:|---|---|',
        '| satisfactory | 12 | `alpha` | `p` |',
    ].join('\n');
    assert.deepEqual(parseInventoryFamilyIndex(md), [{ id: 'alpha', identities: 12, disposition: 'satisfactory' }]);
});

test('ignores_tables_that_are_not_the_family_index', () => {
    const md = ['| Gate | Result |', '|---|---|', '| lint | green |', '', '| Family | Identities | Disposition |', '|---|---:|---|', '| `a` | 1 | satisfactory |'].join('\n');
    assert.deepEqual(parseInventoryFamilyIndex(md).map((r) => r.id), ['a']);
});

test('rejects_a_family_row_with_a_non_numeric_identity_count', () => {
    const md = '| Family | Identities | Disposition |\n|---|---|---|\n| `a` | many | satisfactory |';
    assert.throws(() => parseInventoryFamilyIndex(md), MalformedInput);
});

test('a_family_with_no_inventory_row_is_drift', () => {
    const violations = checkInventory([], [FAM('a', { suites: ['x'] })], new Map([['a', 3]]));
    assert.deepEqual(violations.map((v) => v.kind), ['inventory-drift']);
});

test('an_inventory_row_naming_no_family_is_drift', () => {
    const violations = checkInventory([{ id: 'ghost', identities: 1, disposition: 'satisfactory' }], [], new Map());
    assert.deepEqual(violations.map((v) => v.kind), ['inventory-drift']);
});

test('an_inventory_count_that_disagrees_with_the_captures_is_drift', () => {
    const violations = checkInventory(
        [{ id: 'a', identities: 4, disposition: 'satisfactory' }],
        [FAM('a', { suites: ['x'] })],
        new Map([['a', 3]]),
    );
    assert.equal(violations.length, 1);
    assert.match(violations[0].detail, /inventory says 4 identities, the captures say 3/);
});

test('an_empty_or_unrecognized_disposition_fails', () => {
    const families = [FAM('a', { suites: ['x'] })];
    const counts = new Map([['a', 1]]);
    assert.deepEqual(
        checkInventory([{ id: 'a', identities: 1, disposition: '' }], families, counts).map((v) => v.kind),
        ['missing-disposition'],
    );
    assert.deepEqual(
        checkInventory([{ id: 'a', identities: 1, disposition: 'looks fine to me' }], families, counts).map((v) => v.kind),
        ['missing-disposition'],
    );
    assert.deepEqual(checkInventory([{ id: 'a', identities: 1, disposition: 'follow-up: linked spec' }], families, counts), []);
});

// ---------------------------------------------------------------------------
// Source ↔ runner diff and exclusions
// ---------------------------------------------------------------------------

const src = (pkg: string, name: string, file = 'f.rs') => ({ pkg, file, line: 1, name, cfg: [], ignored: false, fromMacro: false });

test('a_source_test_the_runner_never_lists_is_source_only', () => {
    const diff = diffSourceAgainstRunner([src('claudine', 'a'), src('claudine', 'windows_only')], universeOf([identity('claudine', 'mod::tests::a')]));
    assert.deepEqual(diff.sourceOnly.map((s) => s.name), ['windows_only']);
    assert.deepEqual(diff.runnerOnly, []);
});

test('a_per_platform_pair_of_the_same_name_shows_one_source_only_copy', () => {
    const diff = diffSourceAgainstRunner(
        [src('claudine', 'both'), src('claudine', 'both')],
        universeOf([identity('claudine', 'mod::tests::both')]),
    );
    assert.deepEqual(diff.sourceOnly.map((s) => [s.name, s.count]), [['both', 1]]);
});

test('a_runner_identity_with_no_source_definition_is_reported', () => {
    const diff = diffSourceAgainstRunner([], universeOf([identity('claudine', 'generated')]));
    assert.deepEqual(diff.runnerOnly.map((r) => r.name), ['generated']);
});

test('an_undeclared_exclusion_fails_the_gate', () => {
    const diff = diffSourceAgainstRunner([src('claudine', 'unlisted')], universeOf([]));
    assert.deepEqual(checkExclusions(diff, []).map((v) => v.kind), ['undeclared-exclusion']);
});

test('a_declared_exclusion_that_now_runs_is_stale', () => {
    const diff = diffSourceAgainstRunner([src('claudine', 'a')], universeOf([identity('claudine', 'mod::a')]));
    const violations = checkExclusions(diff, [{ package: 'claudine', name: 'a', gate: 'cfg(windows)', route: 'windows-latest' }]);
    assert.deepEqual(violations.map((v) => v.kind), ['stale-exclusion']);
});

test('a_declared_exclusion_that_still_holds_passes', () => {
    const diff = diffSourceAgainstRunner([src('claudine', 'win')], universeOf([]));
    assert.deepEqual(checkExclusions(diff, [{ package: 'claudine', name: 'win', gate: 'cfg(windows)', route: 'windows-latest' }]), []);
});

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function capture(): { lines: string[]; sink: (line: string) => void } {
    const lines: string[] = [];
    return { lines, sink: (line: string) => lines.push(line) };
}

test('main_exits_two_without_an_enumeration_directory', () => {
    const err = capture();
    assert.equal(main([], () => {}, err.sink), 2);
    assert.match(err.lines.join('\n'), /usage:/);
});

test('main_exits_two_when_families_or_inventory_are_missing', () => {
    const err = capture();
    assert.equal(main([ENUMERATION], () => {}, err.sink), 2);
});

test('main_exits_two_when_a_named_path_does_not_exist', () => {
    const err = capture();
    assert.equal(main([join(tmpdir(), 'no-such-enumeration-dir'), '--families', FAMILIES, '--inventory', INVENTORY], () => {}, err.sink), 2);
    assert.match(err.lines.join('\n'), /not found/);
});

test('main_exits_two_when_the_enumeration_path_is_a_file', () => {
    const err = capture();
    assert.equal(main([FAMILIES, '--families', FAMILIES, '--inventory', INVENTORY], () => {}, err.sink), 2);
    assert.match(err.lines.join('\n'), /not a directory/);
});

test('main_exits_one_on_a_malformed_capture_rather_than_throwing', () => {
    const dir = tempDir();
    writeFileSync(join(dir, 'captures.json'), MANIFEST(['broken']));
    writeFileSync(join(dir, 'broken.json'), '{"nope":true}');
    const err = capture();
    const code = main([dir, '--families', FAMILIES, '--inventory', INVENTORY, '--repo-root', REPO_ROOT], () => {}, err.sink);
    assert.equal(code, 1);
    assert.match(err.lines.join('\n'), /GATE EXIT=1/);
    rmSync(dir, { recursive: true });
});

test('main_exits_one_when_a_family_row_is_missing_from_the_inventory', () => {
    const dir = tempDir();
    const stub = join(dir, 'inventory.md');
    writeFileSync(stub, '| Family | Package | Identities | Disposition |\n|---|---|---:|---|\n');
    const err = capture();
    const code = main([ENUMERATION, '--families', FAMILIES, '--inventory', stub, '--repo-root', REPO_ROOT], () => {}, err.sink);
    assert.equal(code, 1);
    assert.match(err.lines.join('\n'), /inventory-drift/);
    rmSync(dir, { recursive: true });
});

// ---------------------------------------------------------------------------
// Shipped artifacts, end to end
// ---------------------------------------------------------------------------

test('every_shipped_capture_parses_and_declares_a_package_kind_and_binary_id', () => {
    const files = readdirSync(ENUMERATION).filter((f) => f.endsWith('.json') && f !== 'captures.json');
    assert.ok(files.length >= 10, `expected the full capture set, found ${files.length}`);
    for (const file of files) {
        const parsed = parseCapture(file, readFileSync(join(ENUMERATION, file), 'utf8'));
        for (const id of parsed.identities) {
            assert.ok(id.pkg.length > 0 && id.binaryId.length > 0 && id.kind.length > 0, `${file}: ${id.id}`);
        }
    }
});

test('every_shipped_capture_is_declared_in_the_manifest_and_vice_versa', () => {
    const manifest = parseManifest(readFileSync(join(ENUMERATION, 'captures.json'), 'utf8'));
    const universe = buildUniverse(ENUMERATION, manifest);
    assert.deepEqual(universe.violations, []);
});

test('the_shipped_families_assign_every_shipped_identity_exactly_once', () => {
    const manifest = parseManifest(readFileSync(join(ENUMERATION, 'captures.json'), 'utf8'));
    const universe = buildUniverse(ENUMERATION, manifest);
    const { families } = parseFamilyFile(readFileSync(FAMILIES, 'utf8'));
    const { violations, counts } = reconcileFamilies(universe, families);
    assert.deepEqual(violations, []);
    const assigned = [...counts.values()].reduce((a, b) => a + b, 0);
    assert.equal(assigned, universe.identities.size);
});

test('every_source_only_test_in_the_real_tree_is_a_declared_exclusion', () => {
    const manifest = parseManifest(readFileSync(join(ENUMERATION, 'captures.json'), 'utf8'));
    const universe = buildUniverse(ENUMERATION, manifest);
    const population = scanSourcePopulation(REPO_ROOT, CLAUDINE_PACKAGE_ROOTS);
    assert.ok(population.length > 7000, `expected the real population, found ${population.length}`);
    const diff = diffSourceAgainstRunner(population, universe);
    const { exclusions } = parseFamilyFile(readFileSync(FAMILIES, 'utf8'));
    assert.deepEqual(checkExclusions(diff, exclusions), []);
});

test('the_real_tree_has_no_runner_identity_without_a_source_definition', () => {
    const manifest = parseManifest(readFileSync(join(ENUMERATION, 'captures.json'), 'utf8'));
    const universe = buildUniverse(ENUMERATION, manifest);
    const diff = diffSourceAgainstRunner(scanSourcePopulation(REPO_ROOT, CLAUDINE_PACKAGE_ROOTS), universe);
    assert.deepEqual(diff.runnerOnly, []);
});

test('the_gate_passes_on_the_shipped_inventory_and_prints_its_table', () => {
    const out = capture();
    const err = capture();
    const code = main([ENUMERATION, '--families', FAMILIES, '--inventory', INVENTORY, '--repo-root', REPO_ROOT], out.sink, err.sink);
    assert.equal(code, 0, err.lines.join('\n'));
    const text = out.lines.join('\n');
    assert.match(text, /\| \*\*total\*\* \|/);
    assert.match(text, /GATE EXIT=0/);
});

test('repeated_runs_over_one_tree_produce_an_identical_table', () => {
    const first = capture();
    const second = capture();
    main([ENUMERATION, '--families', FAMILIES, '--inventory', INVENTORY, '--repo-root', REPO_ROOT], first.sink, () => {});
    main([ENUMERATION, '--families', FAMILIES, '--inventory', INVENTORY, '--repo-root', REPO_ROOT], second.sink, () => {});
    assert.deepEqual(first.lines, second.lines);
});

test('a_neutered_family_declaration_is_caught_rather_than_silently_reducing_a_count', () => {
    // Drop one suite from a shipped family: its identities must become
    // unassigned, not quietly vanish from the totals.
    const { families } = parseFamilyFile(readFileSync(FAMILIES, 'utf8'));
    const target = families.find((f) => f.id === 'lib-corpus-replay')!;
    const neutered = families.map((f) =>
        f.id === target.id ? { ...f, match: { suites: target.match.suites!.slice(1) } } : f,
    );
    const manifest = parseManifest(readFileSync(join(ENUMERATION, 'captures.json'), 'utf8'));
    const universe = buildUniverse(ENUMERATION, manifest);
    const { violations } = reconcileFamilies(universe, neutered);
    assert.ok(violations.length > 0);
    assert.ok(violations.every((v) => v.kind === 'unassigned-identity'));
});

test('the_family_index_in_the_shipped_inventory_covers_every_declared_family', () => {
    const { families } = parseFamilyFile(readFileSync(FAMILIES, 'utf8'));
    const rows = parseInventoryFamilyIndex(readFileSync(INVENTORY, 'utf8'));
    assert.deepEqual(new Set(rows.map((r) => r.id)), new Set(families.map((f) => f.id)));
});
