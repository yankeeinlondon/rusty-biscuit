/**
 * Completeness reconciler for `inventory.md` (fix 2026-09-07-faster-claudine-tests, Phase 2).
 *
 * RB1 requires every test identity in the eight scoped packages to land in
 * exactly one evaluation row. Seven thousand four hundred identities cannot be
 * enumerated by hand, so the inventory declares *families* and this gate proves
 * the declaration is total and non-overlapping:
 *
 *   1. the runner universe is rebuilt from the `cargo nextest list` captures
 *      under `enumeration/`, one capture per package × feature selection;
 *   2. every identity is matched against `families.json`; zero matches or more
 *      than one match is a violation;
 *   3. `inventory.md`'s family index must agree with `families.json` on ids,
 *      on identity counts, and must carry a disposition for every row;
 *   4. the source-side attribute population is scanned independently and
 *      diffed against the runner universe; a source test the runner never
 *      lists must be declared as a cfg/feature exclusion with a reason and an
 *      execution route.
 *
 * Exit codes: 0 clean, 1 violations, 2 usage error. A run that prints a miss
 * and exits 0 is not a gate.
 */

import { readFileSync, readdirSync, existsSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/** A capture, manifest, or family declaration that cannot be interpreted. */
export class MalformedInput extends Error {}

/** A usage problem: missing argument, unreadable directory. */
export class UsageError extends Error {}

export type ViolationKind =
    | 'malformed-capture'
    | 'missing-capture'
    | 'duplicate-identity'
    | 'unassigned-identity'
    | 'double-assigned-identity'
    | 'stale-family'
    | 'inventory-drift'
    | 'missing-disposition'
    | 'undeclared-exclusion'
    | 'stale-exclusion';

export interface Violation {
    kind: ViolationKind;
    detail: string;
}

// ---------------------------------------------------------------------------
// Rust source lexing
// ---------------------------------------------------------------------------

/**
 * Blank out string literals, char literals, and comments, preserving length
 * and line structure so offsets and line numbers survive.
 *
 * Attribute scanning without this step reports the fixture inside
 * `test_placement.rs`'s raw string as a real test — the analyzer that file
 * tests exists for exactly this reason.
 */
export function blankLiteralsAndComments(source: string): string {
    const out = source.split('');
    const n = source.length;
    let i = 0;
    const blank = (from: number, to: number) => {
        for (let k = from; k < to && k < n; k += 1) {
            if (out[k] !== '\n') out[k] = ' ';
        }
    };
    while (i < n) {
        const c = source[i];
        const next = source[i + 1];
        // line comment
        if (c === '/' && next === '/') {
            let j = i;
            while (j < n && source[j] !== '\n') j += 1;
            blank(i, j);
            i = j;
            continue;
        }
        // block comment (nesting, as Rust allows)
        if (c === '/' && next === '*') {
            let depth = 1;
            let j = i + 2;
            while (j < n && depth > 0) {
                if (source[j] === '/' && source[j + 1] === '*') {
                    depth += 1;
                    j += 2;
                } else if (source[j] === '*' && source[j + 1] === '/') {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            if (depth > 0) throw new MalformedInput('unterminated block comment');
            blank(i, j);
            i = j;
            continue;
        }
        // raw string: r"..." / r#"..."# / br##"..."##
        const rawStart = source.slice(i, i + 3);
        const rawMatch = /^(b?r)(#*)"/.exec(source.slice(i, i + 40));
        if (rawMatch && (c === 'r' || (c === 'b' && next === 'r')) && !isIdentChar(source[i - 1] ?? ' ')) {
            const hashes = rawMatch[2];
            const closer = `"${hashes}`;
            const bodyStart = i + rawMatch[0].length;
            const end = source.indexOf(closer, bodyStart);
            if (end === -1) throw new MalformedInput('unterminated raw string');
            blank(i, end + closer.length);
            i = end + closer.length;
            continue;
        }
        void rawStart;
        // ordinary string
        if (c === '"') {
            let j = i + 1;
            while (j < n) {
                if (source[j] === '\\') {
                    j += 2;
                    continue;
                }
                if (source[j] === '"') break;
                j += 1;
            }
            if (j >= n) throw new MalformedInput('unterminated string literal');
            blank(i, j + 1);
            i = j + 1;
            continue;
        }
        // char literal, distinguished from a lifetime (`'a`) by the closing quote
        if (c === "'") {
            const m = /^'(\\.|[^\\'])'/.exec(source.slice(i, i + 12));
            if (m) {
                blank(i, i + m[0].length);
                i += m[0].length;
                continue;
            }
        }
        i += 1;
    }
    return out.join('');
}

function isIdentChar(ch: string): boolean {
    return /[A-Za-z0-9_]/.test(ch);
}

// ---------------------------------------------------------------------------
// Source-side attribute scan
// ---------------------------------------------------------------------------

export interface SourceTest {
    /** Package the file belongs to. */
    pkg: string;
    /** Repository-relative path. */
    file: string;
    /** 1-indexed line of the `fn` (or of the macro invocation that made it). */
    line: number;
    name: string;
    /** `cfg(...)` attributes in force, innermost last. `cfg(test)` is dropped. */
    cfg: string[];
    ignored: boolean;
    /** True when the identity comes from a `macro_rules!` test template. */
    fromMacro: boolean;
}

const TEST_ATTR = /^#\[(?:tokio::test|test|rstest|bench)\b/;
const CFG_ATTR = /^#\[cfg\(/;

/**
 * Scan one Rust file for test identities.
 *
 * Handles the two shapes this area actually uses: plain attributed functions,
 * and a `macro_rules!` template whose invocations name the generated tests
 * (`wrap_compose_agent.rs`'s `direct_wrap_dry_run_test!`). Macro *templates*
 * are not identities; their invocation sites are.
 */
export function scanRustTests(source: string, pkg: string, file: string): SourceTest[] {
    const code = blankLiteralsAndComments(source);
    const lines = code.split('\n');
    // Structure is read from the blanked text; attribute *text* comes from the
    // original so `#[cfg(target_os = "linux")]` keeps its literal.
    const original = source.split('\n');
    const found: SourceTest[] = [];
    const modCfg: { depth: number; cfg: string[] }[] = [];
    const macroTemplates = new Map<string, { cfg: string[]; ignored: boolean }>();

    let depth = 0;
    let pending: string[] = [];
    let macro: { name: string; depth: number; cfg: string[]; ignored: boolean; sawTest: boolean } | null = null;

    const activeCfg = () => modCfg.flatMap((m) => m.cfg);

    for (let i = 0; i < lines.length; i += 1) {
        const text = lines[i].trim();
        const macroDecl = /^macro_rules!\s+([A-Za-z0-9_]+)/.exec(text);

        if (macro) {
            if (TEST_ATTR.test(text)) macro.sawTest = true;
            if (/^#\[ignore\b/.test(text)) macro.ignored = true;
            if (CFG_ATTR.test(text)) macro.cfg.push(original[i].trim());
        }

        if (macroDecl && !macro) {
            macro = { name: macroDecl[1], depth, cfg: [], ignored: false, sawTest: false };
            pending = [];
        } else if (/^#!?\[/.test(text)) {
            pending.push(original[i].trim());
        } else if (/^(pub(\([^)]*\))?\s+)?(async\s+)?(unsafe\s+)?(extern\s+"[^"]*"\s+)?fn\s/.test(text) || /^fn\s/.test(text)) {
            if (!macro && pending.some((a) => TEST_ATTR.test(a))) {
                const nameMatch = /\bfn\s+([A-Za-z0-9_]+)/.exec(lines[i]);
                if (nameMatch) {
                    found.push({
                        pkg,
                        file,
                        line: i + 1,
                        name: nameMatch[1],
                        cfg: dedupeCfg([...activeCfg(), ...pending.filter((a) => CFG_ATTR.test(a))]),
                        ignored: pending.some((a) => /^#\[ignore\b/.test(a)),
                        fromMacro: false,
                    });
                }
            }
            pending = [];
        } else if (/^(pub(\([^)]*\))?\s+)?mod\s+[A-Za-z0-9_]+/.test(text)) {
            if (text.includes('{')) {
                modCfg.push({ depth, cfg: pending.filter((a) => CFG_ATTR.test(a)) });
            }
            pending = [];
        } else if (text.length > 0) {
            const invocation = /^([A-Za-z0-9_]+)!\s*[({]\s*([A-Za-z0-9_]+)\s*,/.exec(text);
            if (invocation && macroTemplates.has(invocation[1])) {
                const tpl = macroTemplates.get(invocation[1])!;
                found.push({
                    pkg,
                    file,
                    line: i + 1,
                    name: invocation[2],
                    cfg: dedupeCfg([...activeCfg(), ...tpl.cfg]),
                    ignored: tpl.ignored,
                    fromMacro: true,
                });
            }
            pending = [];
        }

        const opens = countChar(lines[i], '{');
        const closes = countChar(lines[i], '}');
        depth += opens - closes;

        if (macro && depth <= macro.depth) {
            if (macro.sawTest) macroTemplates.set(macro.name, { cfg: macro.cfg, ignored: macro.ignored });
            macro = null;
        }
        while (modCfg.length > 0 && depth <= modCfg[modCfg.length - 1].depth) modCfg.pop();
    }
    return found;
}

function dedupeCfg(cfg: string[]): string[] {
    const keep = cfg.filter((c) => c !== '#[cfg(test)]');
    return [...new Set(keep)];
}

function countChar(text: string, ch: string): number {
    let n = 0;
    for (const c of text) if (c === ch) n += 1;
    return n;
}

/** Recursively collect `.rs` files under `dir`. */
export function rustFilesUnder(dir: string): string[] {
    if (!existsSync(dir)) return [];
    const out: string[] = [];
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const p = join(dir, entry.name);
        if (entry.isDirectory()) out.push(...rustFilesUnder(p));
        else if (entry.name.endsWith('.rs')) out.push(p);
    }
    return out.sort();
}

export interface PackageRoot {
    pkg: string;
    /** Directory relative to the repository root. */
    dir: string;
}

/** Scan every `src`/`tests`/`benches` tree of the declared package roots. */
export function scanSourcePopulation(repoRoot: string, roots: PackageRoot[]): SourceTest[] {
    const out: SourceTest[] = [];
    for (const { pkg, dir } of roots) {
        for (const sub of ['src', 'tests', 'benches']) {
            const abs = join(repoRoot, dir, sub);
            for (const file of rustFilesUnder(abs)) {
                const rel = relative(repoRoot, file).split(sep).join('/');
                out.push(...scanRustTests(readFileSync(file, 'utf8'), pkg, rel));
            }
        }
    }
    return out;
}

// ---------------------------------------------------------------------------
// Runner universe
// ---------------------------------------------------------------------------

export interface CaptureDecl {
    /** File stem under the enumeration directory, without `.json`. */
    label: string;
    packages: string[];
    features: string[];
    command: string;
}

export interface CaptureManifest {
    revision: string;
    toolchain: string;
    nextest: string;
    platform: string;
    captures: CaptureDecl[];
}

export interface Identity {
    id: string;
    pkg: string;
    binaryId: string;
    kind: string;
    name: string;
    captures: string[];
}

export interface Universe {
    identities: Map<string, Identity>;
    /** Suites that exist as build targets but list no test. */
    emptySuites: { pkg: string; binaryId: string; kind: string }[];
    violations: Violation[];
}

function requireString(value: unknown, what: string): string {
    if (typeof value !== 'string' || value.length === 0) {
        throw new MalformedInput(`${what}: expected a non-empty string, got ${JSON.stringify(value)}`);
    }
    return value;
}

function requireArray(value: unknown, what: string): unknown[] {
    if (!Array.isArray(value)) throw new MalformedInput(`${what}: expected an array`);
    return value;
}

export function parseManifest(text: string): CaptureManifest {
    let raw: unknown;
    try {
        raw = JSON.parse(text);
    } catch (error) {
        throw new MalformedInput(`captures.json is not JSON: ${(error as Error).message}`);
    }
    const obj = raw as Record<string, unknown>;
    const captures = requireArray(obj.captures, 'captures').map((entry, index) => {
        const c = entry as Record<string, unknown>;
        return {
            label: requireString(c.label, `captures[${index}].label`),
            packages: requireArray(c.packages, `captures[${index}].packages`).map((p, j) =>
                requireString(p, `captures[${index}].packages[${j}]`),
            ),
            features: requireArray(c.features ?? [], `captures[${index}].features`).map((f, j) =>
                requireString(f, `captures[${index}].features[${j}]`),
            ),
            command: requireString(c.command, `captures[${index}].command`),
        };
    });
    if (captures.length === 0) throw new MalformedInput('captures.json declares no captures');
    return {
        revision: requireString(obj.revision, 'revision'),
        toolchain: requireString(obj.toolchain, 'toolchain'),
        nextest: requireString(obj.nextest, 'nextest'),
        platform: requireString(obj.platform, 'platform'),
        captures,
    };
}

/** Read one `cargo nextest list --message-format json` capture. */
export function parseCapture(label: string, text: string): { identities: Omit<Identity, 'captures'>[]; empty: Universe['emptySuites'] } {
    let raw: unknown;
    try {
        raw = JSON.parse(text);
    } catch (error) {
        throw new MalformedInput(`${label}: not JSON (${(error as Error).message})`);
    }
    const suites = (raw as Record<string, unknown>)['rust-suites'];
    if (suites === undefined || suites === null || typeof suites !== 'object') {
        throw new MalformedInput(`${label}: no "rust-suites" object; this is not a nextest listing`);
    }
    const identities: Omit<Identity, 'captures'>[] = [];
    const empty: Universe['emptySuites'] = [];
    const seen = new Set<string>();
    for (const suite of Object.values(suites as Record<string, unknown>)) {
        const s = suite as Record<string, unknown>;
        const pkg = requireString(s['package-name'], `${label}: package-name`);
        const binaryId = requireString(s['binary-id'], `${label}: binary-id`);
        const kind = requireString(s.kind, `${label}: kind`);
        const cases = s.testcases;
        if (cases === undefined || cases === null || typeof cases !== 'object') {
            throw new MalformedInput(`${label}: suite ${binaryId} has no testcases object`);
        }
        const names = Object.keys(cases as Record<string, unknown>);
        if (names.length === 0) {
            empty.push({ pkg, binaryId, kind });
            continue;
        }
        for (const name of names) {
            const id = `${binaryId} :: ${name}`;
            if (seen.has(id)) {
                throw new MalformedInput(`${label}: duplicate identity ${id} within one listing`);
            }
            seen.add(id);
            identities.push({ id, pkg, binaryId, kind, name });
        }
    }
    return { identities, empty };
}

export function buildUniverse(dir: string, manifest: CaptureManifest): Universe {
    const identities = new Map<string, Identity>();
    const emptySuites = new Map<string, Universe['emptySuites'][number]>();
    const violations: Violation[] = [];

    const declared = new Set(manifest.captures.map((c) => `${c.label}.json`));
    const onDisk = new Set(readdirSync(dir).filter((f) => f.endsWith('.json') && f !== 'captures.json'));
    for (const file of onDisk) {
        if (!declared.has(file)) {
            violations.push({
                kind: 'malformed-capture',
                detail: `${file} is present in the enumeration directory but not declared in captures.json`,
            });
        }
    }

    for (const capture of manifest.captures) {
        const path = join(dir, `${capture.label}.json`);
        if (!existsSync(path)) {
            violations.push({ kind: 'missing-capture', detail: `declared capture ${capture.label} has no ${capture.label}.json` });
            continue;
        }
        const parsed = parseCapture(capture.label, readFileSync(path, 'utf8'));
        for (const entry of parsed.identities) {
            const existing = identities.get(entry.id);
            if (existing) existing.captures.push(capture.label);
            else identities.set(entry.id, { ...entry, captures: [capture.label] });
        }
        for (const e of parsed.empty) emptySuites.set(e.binaryId, e);
    }
    return { identities, emptySuites: [...emptySuites.values()], violations };
}

// ---------------------------------------------------------------------------
// Families
// ---------------------------------------------------------------------------

export interface FamilyMatch {
    suites?: string[];
    modules?: string[];
    tests?: string[];
}

export interface Family {
    id: string;
    package: string;
    match: FamilyMatch;
    exclude?: FamilyMatch;
    /** Declared reason a family legitimately matches nothing on this platform. */
    expectEmpty?: string;
}

export interface Exclusion {
    package: string;
    name: string;
    /** The gate that keeps it off this platform, e.g. `cfg(windows)`. */
    gate: string;
    /** Where it does execute. */
    route: string;
}

export interface FamilyFile {
    families: Family[];
    exclusions: Exclusion[];
}

export function parseFamilyFile(text: string): FamilyFile {
    let raw: unknown;
    try {
        raw = JSON.parse(text);
    } catch (error) {
        throw new MalformedInput(`families.json is not JSON: ${(error as Error).message}`);
    }
    const obj = raw as Record<string, unknown>;
    const parseMatch = (value: unknown, what: string): FamilyMatch | undefined => {
        if (value === undefined) return undefined;
        const m = value as Record<string, unknown>;
        const out: FamilyMatch = {};
        for (const key of ['suites', 'modules', 'tests'] as const) {
            if (m[key] === undefined) continue;
            out[key] = requireArray(m[key], `${what}.${key}`).map((v, i) => requireString(v, `${what}.${key}[${i}]`));
        }
        if (Object.keys(out).length === 0) throw new MalformedInput(`${what}: declares no suites, modules or tests`);
        return out;
    };
    const families = requireArray(obj.families, 'families').map((entry, index) => {
        const f = entry as Record<string, unknown>;
        const id = requireString(f.id, `families[${index}].id`);
        const match = parseMatch(f.match, `families[${index}].match`);
        if (!match) throw new MalformedInput(`families[${index}] (${id}): no match block`);
        return {
            id,
            package: requireString(f.package, `families[${index}].package`),
            match,
            exclude: parseMatch(f.exclude, `families[${index}].exclude`),
            expectEmpty: f.expectEmpty === undefined ? undefined : requireString(f.expectEmpty, `families[${index}].expectEmpty`),
        };
    });
    const ids = new Set<string>();
    for (const f of families) {
        if (ids.has(f.id)) throw new MalformedInput(`families: duplicate id ${f.id}`);
        ids.add(f.id);
    }
    const exclusions = requireArray(obj.exclusions ?? [], 'exclusions').map((entry, index) => {
        const e = entry as Record<string, unknown>;
        return {
            package: requireString(e.package, `exclusions[${index}].package`),
            name: requireString(e.name, `exclusions[${index}].name`),
            gate: requireString(e.gate, `exclusions[${index}].gate`),
            route: requireString(e.route, `exclusions[${index}].route`),
        };
    });
    return { families, exclusions };
}

/**
 * `suites` alone claims whole suites; `suites` plus `modules`/`tests` narrows
 * within them; `modules`/`tests` alone reach every suite of the package.
 */
function matchesBlock(identity: Identity, block: FamilyMatch | undefined): boolean {
    if (!block) return false;
    if (block.suites && !block.suites.includes(identity.binaryId)) return false;
    if (!block.modules && !block.tests) return block.suites !== undefined;
    if (block.tests?.includes(identity.name)) return true;
    if (block.modules?.some((m) => identity.name === m || identity.name.startsWith(`${m}::`))) return true;
    return false;
}

export function familiesMatching(identity: Identity, families: Family[]): Family[] {
    return families.filter((family) => {
        if (family.package !== identity.pkg) return false;
        if (!matchesBlock(identity, family.match)) return false;
        if (family.exclude && matchesBlock(identity, family.exclude)) return false;
        return true;
    });
}

export interface ReconcileResult {
    violations: Violation[];
    counts: Map<string, number>;
}

export function reconcileFamilies(universe: Universe, families: Family[]): ReconcileResult {
    const violations: Violation[] = [];
    const counts = new Map<string, number>(families.map((f) => [f.id, 0]));
    for (const identity of universe.identities.values()) {
        const hits = familiesMatching(identity, families);
        if (hits.length === 0) {
            violations.push({ kind: 'unassigned-identity', detail: `${identity.id} matches no family` });
            continue;
        }
        if (hits.length > 1) {
            violations.push({
                kind: 'double-assigned-identity',
                detail: `${identity.id} matches ${hits.length} families: ${hits.map((h) => h.id).join(', ')}`,
            });
            continue;
        }
        counts.set(hits[0].id, (counts.get(hits[0].id) ?? 0) + 1);
    }
    for (const family of families) {
        if ((counts.get(family.id) ?? 0) === 0 && family.expectEmpty === undefined) {
            violations.push({
                kind: 'stale-family',
                detail: `family ${family.id} matches no identity and declares no expectEmpty reason`,
            });
        }
    }
    return { violations, counts };
}

// ---------------------------------------------------------------------------
// inventory.md cross-check
// ---------------------------------------------------------------------------

export interface InventoryRow {
    id: string;
    identities: number;
    disposition: string;
}

const DISPOSITIONS = ['satisfactory', 'remediation in this fix', 'follow-up'];

/**
 * Read the family index out of `inventory.md`.
 *
 * The index is the GFM table whose header row names `Family`, `Identities`,
 * and `Disposition`; column order is read from the header rather than assumed.
 */
export function parseInventoryFamilyIndex(markdown: string): InventoryRow[] {
    const lines = markdown.split('\n');
    const rows: InventoryRow[] = [];
    let cols: { id: number; identities: number; disposition: number } | null = null;
    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed.startsWith('|')) {
            cols = null;
            continue;
        }
        const cells = splitTableRow(trimmed);
        if (cols === null) {
            const lower = cells.map((c) => c.toLowerCase());
            const id = lower.indexOf('family');
            const identities = lower.indexOf('identities');
            const disposition = lower.indexOf('disposition');
            if (id >= 0 && identities >= 0 && disposition >= 0) cols = { id, identities, disposition };
            continue;
        }
        if (cells.every((c) => /^:?-{2,}:?$/.test(c))) continue;
        const id = cells[cols.id]?.replace(/`/g, '').trim();
        if (!id) continue;
        const countText = cells[cols.identities]?.trim() ?? '';
        if (!/^\d+$/.test(countText)) {
            throw new MalformedInput(`inventory family index: row ${id} has a non-numeric identity count ${JSON.stringify(countText)}`);
        }
        rows.push({ id, identities: Number(countText), disposition: cells[cols.disposition]?.trim() ?? '' });
    }
    return rows;
}

function splitTableRow(line: string): string[] {
    return line
        .replace(/^\|/, '')
        .replace(/\|$/, '')
        .split('|')
        .map((c) => c.trim());
}

export function checkInventory(rows: InventoryRow[], families: Family[], counts: Map<string, number>): Violation[] {
    const violations: Violation[] = [];
    const declared = new Set(families.map((f) => f.id));
    const listed = new Set(rows.map((r) => r.id));
    for (const id of declared) {
        if (!listed.has(id)) {
            violations.push({ kind: 'inventory-drift', detail: `family ${id} is declared in families.json but has no inventory row` });
        }
    }
    for (const row of rows) {
        if (!declared.has(row.id)) {
            violations.push({ kind: 'inventory-drift', detail: `inventory row ${row.id} names no declared family` });
            continue;
        }
        const expected = counts.get(row.id) ?? 0;
        if (row.identities !== expected) {
            violations.push({
                kind: 'inventory-drift',
                detail: `family ${row.id}: inventory says ${row.identities} identities, the captures say ${expected}`,
            });
        }
        if (row.disposition.length === 0) {
            violations.push({ kind: 'missing-disposition', detail: `family ${row.id} has an empty disposition` });
        } else if (!DISPOSITIONS.some((d) => row.disposition.toLowerCase().startsWith(d))) {
            violations.push({
                kind: 'missing-disposition',
                detail: `family ${row.id}: disposition ${JSON.stringify(row.disposition)} is not one of ${DISPOSITIONS.join(' / ')}`,
            });
        }
    }
    return violations;
}

// ---------------------------------------------------------------------------
// Source ↔ runner diff
// ---------------------------------------------------------------------------

export interface SourceDiff {
    /** Source tests the runner never lists, keyed `package::name`. */
    sourceOnly: { pkg: string; name: string; count: number; files: string[] }[];
    /** Runner identities with no source-side definition (macro or generated). */
    runnerOnly: { pkg: string; name: string; count: number }[];
}

/**
 * Diff the source population against the runner universe by
 * `(package, leaf name)` multiset.
 *
 * Leaf names rather than full module paths: nextest reports a lib test as
 * `a::b::tests::name` while the source scan sees only `name`, and rebuilding
 * the module path from text would guess. Counting per name keeps a
 * `cfg(unix)` / `cfg(windows)` pair of the same name visible — the source has
 * two, the runner one.
 */
export function diffSourceAgainstRunner(source: SourceTest[], universe: Universe): SourceDiff {
    const runner = new Map<string, number>();
    for (const identity of universe.identities.values()) {
        const leaf = identity.name.split('::').pop()!;
        const key = `${identity.pkg} ${leaf}`;
        runner.set(key, (runner.get(key) ?? 0) + 1);
    }
    const srcCounts = new Map<string, { pkg: string; name: string; count: number; files: string[] }>();
    for (const test of source) {
        const key = `${test.pkg} ${test.name}`;
        const entry = srcCounts.get(key) ?? { pkg: test.pkg, name: test.name, count: 0, files: [] };
        entry.count += 1;
        entry.files.push(`${test.file}:${test.line}`);
        srcCounts.set(key, entry);
    }
    const sourceOnly: SourceDiff['sourceOnly'] = [];
    for (const [key, entry] of srcCounts) {
        const run = runner.get(key) ?? 0;
        if (entry.count > run) sourceOnly.push({ ...entry, count: entry.count - run });
    }
    const runnerOnly: SourceDiff['runnerOnly'] = [];
    for (const [key, count] of runner) {
        const [pkg, name] = key.split(' ');
        const src = srcCounts.get(key)?.count ?? 0;
        if (count > src) runnerOnly.push({ pkg, name, count: count - src });
    }
    sourceOnly.sort((a, b) => a.pkg.localeCompare(b.pkg) || a.name.localeCompare(b.name));
    runnerOnly.sort((a, b) => a.pkg.localeCompare(b.pkg) || a.name.localeCompare(b.name));
    return { sourceOnly, runnerOnly };
}

export function checkExclusions(diff: SourceDiff, exclusions: Exclusion[]): Violation[] {
    const violations: Violation[] = [];
    const declared = new Map(exclusions.map((e) => [`${e.package} ${e.name}`, e]));
    const observed = new Set(diff.sourceOnly.map((s) => `${s.pkg} ${s.name}`));
    for (const entry of diff.sourceOnly) {
        const key = `${entry.pkg} ${entry.name}`;
        if (!declared.has(key)) {
            violations.push({
                kind: 'undeclared-exclusion',
                detail: `${entry.pkg} :: ${entry.name} is defined in source (${entry.files.join(', ')}) but never listed by the runner, and no exclusion declares why`,
            });
        }
    }
    for (const [key, entry] of declared) {
        if (!observed.has(key)) {
            violations.push({
                kind: 'stale-exclusion',
                detail: `exclusion ${entry.package} :: ${entry.name} no longer describes an unlisted test; delete it`,
            });
        }
    }
    return violations;
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

export interface GateInput {
    enumerationDir: string;
    familiesPath: string;
    inventoryPath: string;
    repoRoot: string;
    roots: PackageRoot[];
}

export interface GateOutput {
    violations: Violation[];
    counts: Map<string, number>;
    universe: Universe;
    diff: SourceDiff;
    table: string;
}

export const CLAUDINE_PACKAGE_ROOTS: PackageRoot[] = [
    { pkg: 'claudine-catalog-types', dir: 'claudine/catalog-types' },
    { pkg: 'claudine', dir: 'claudine/lib' },
    { pkg: 'claudine-contract', dir: 'claudine/contract' },
    { pkg: 'claudine-cli', dir: 'claudine/cli' },
    { pkg: 'claudine-gen', dir: 'claudine/gen' },
    { pkg: 'rendezvous-core', dir: 'claudine/rendezvous/core' },
    { pkg: 'rendezvous-daemon', dir: 'claudine/rendezvous/daemon' },
    { pkg: 'rendezvous-client', dir: 'claudine/rendezvous/client' },
];

export function runGate(input: GateInput): GateOutput {
    const manifestPath = join(input.enumerationDir, 'captures.json');
    if (!existsSync(manifestPath)) {
        throw new UsageError(`${input.enumerationDir}: no captures.json; this is not an enumeration directory`);
    }
    const manifest = parseManifest(readFileSync(manifestPath, 'utf8'));
    const universe = buildUniverse(input.enumerationDir, manifest);
    const familyFile = parseFamilyFile(readFileSync(input.familiesPath, 'utf8'));
    const { violations: familyViolations, counts } = reconcileFamilies(universe, familyFile.families);
    const rows = parseInventoryFamilyIndex(readFileSync(input.inventoryPath, 'utf8'));
    const inventoryViolations = checkInventory(rows, familyFile.families, counts);
    const source = scanSourcePopulation(input.repoRoot, input.roots);
    const diff = diffSourceAgainstRunner(source, universe);
    const exclusionViolations = checkExclusions(diff, familyFile.exclusions);

    const violations = [
        ...universe.violations,
        ...familyViolations,
        ...inventoryViolations,
        ...exclusionViolations,
    ];
    return { violations, counts, universe, diff, table: renderTable(familyFile.families, counts, source, universe) };
}

function renderTable(families: Family[], counts: Map<string, number>, source: SourceTest[], universe: Universe): string {
    const byPackage = new Map<string, number>();
    for (const identity of universe.identities.values()) {
        byPackage.set(identity.pkg, (byPackage.get(identity.pkg) ?? 0) + 1);
    }
    const lines: string[] = [];
    lines.push('| Family | Package | Identities |');
    lines.push('|---|---|---:|');
    for (const family of families) {
        lines.push(`| \`${family.id}\` | \`${family.package}\` | ${counts.get(family.id) ?? 0} |`);
    }
    lines.push('');
    lines.push('| Package | Runner identities | Source attributes |');
    lines.push('|---|---:|---:|');
    const srcByPackage = new Map<string, number>();
    for (const t of source) srcByPackage.set(t.pkg, (srcByPackage.get(t.pkg) ?? 0) + 1);
    let runnerTotal = 0;
    let sourceTotal = 0;
    for (const pkg of [...byPackage.keys()].sort()) {
        const r = byPackage.get(pkg) ?? 0;
        const s = srcByPackage.get(pkg) ?? 0;
        runnerTotal += r;
        sourceTotal += s;
        lines.push(`| \`${pkg}\` | ${r} | ${s} |`);
    }
    lines.push(`| **total** | **${runnerTotal}** | **${sourceTotal}** |`);
    return lines.join('\n');
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

export function main(argv: string[], out: (line: string) => void = console.log, err: (line: string) => void = console.error): number {
    const positional = argv.filter((a) => !a.startsWith('--'));
    const flag = (name: string): string | undefined => {
        const index = argv.indexOf(`--${name}`);
        return index === -1 ? undefined : argv[index + 1];
    };
    const enumerationDir = positional[0];
    if (!enumerationDir) {
        err('usage: inventory-reconciler.ts <enumeration-dir> --families <families.json> --inventory <inventory.md> --repo-root <dir>');
        return 2;
    }
    const familiesPath = flag('families');
    const inventoryPath = flag('inventory');
    const repoRoot = flag('repo-root') ?? process.cwd();
    if (!familiesPath || !inventoryPath) {
        err('usage: --families and --inventory are both required');
        return 2;
    }
    for (const [what, path] of [['enumeration dir', enumerationDir], ['families', familiesPath], ['inventory', inventoryPath], ['repo root', repoRoot]] as const) {
        if (!existsSync(path)) {
            err(`${what} not found: ${path}`);
            return 2;
        }
    }
    if (!statSync(enumerationDir).isDirectory()) {
        err(`enumeration dir is not a directory: ${enumerationDir}`);
        return 2;
    }

    let result: GateOutput;
    try {
        result = runGate({
            enumerationDir,
            familiesPath,
            inventoryPath,
            repoRoot,
            roots: CLAUDINE_PACKAGE_ROOTS,
        });
    } catch (error) {
        if (error instanceof UsageError) {
            err(String(error.message));
            return 2;
        }
        if (error instanceof MalformedInput) {
            err(`1 violation(s):\n  [malformed-capture] ${error.message}`);
            err('GATE EXIT=1');
            return 1;
        }
        throw error;
    }

    out(result.table);
    out('');
    if (result.diff.sourceOnly.length > 0) {
        out(`cfg/feature exclusions (source-defined, runner never lists): ${result.diff.sourceOnly.reduce((a, b) => a + b.count, 0)}`);
    }
    if (result.diff.runnerOnly.length > 0) {
        out(`runner identities with no plain source definition: ${result.diff.runnerOnly.length}`);
    }
    if (result.universe.emptySuites.length > 0) {
        out(`build targets listing no test: ${result.universe.emptySuites.map((s) => s.binaryId).join(', ')}`);
    }
    if (result.violations.length > 0) {
        err(`${result.violations.length} violation(s):`);
        for (const v of result.violations) err(`  [${v.kind}] ${v.detail}`);
        err('GATE EXIT=1');
        return 1;
    }
    out('GATE EXIT=0');
    return 0;
}

const invokedDirectly = process.argv[1]?.endsWith('inventory-reconciler.ts');
if (invokedDirectly) {
    process.exit(main(process.argv.slice(2)));
}
