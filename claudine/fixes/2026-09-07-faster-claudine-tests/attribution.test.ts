/**
 * Tests for `attribution.ts`.
 *
 * Run with `npx tsx --test attribution.test.ts` from this directory.
 * `node:test` rather than Vitest, matching `junit-metrics.test.ts` and
 * `inventory-reconciler.test.ts`.
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, readFileSync, rmSync, readdirSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
    MalformedLog,
    UsageError,
    stripAnsi,
    packageOf,
    parseNextestLog,
    checkRun,
    decidedRecords,
    attribute,
    attributeByBinary,
    renderBinaryTable,
    deriveBudgets,
    renderAttributionTable,
    renderBudgetTable,
    runGate,
    main,
    REQUIRED_RUNS_PER_LEG,
    type BudgetInput,
    type ParsedRun,
} from './attribution.ts';
import { parseFamilyFile, type Family } from './inventory-reconciler.ts';

const HERE = dirname(fileURLToPath(import.meta.url));
const FAMILIES = join(HERE, 'families.json');
const LOCAL_GATES = join(HERE, 'baseline', 'local-gates');
const RECIPES = join(HERE, 'enumeration', 'recipes');
const RUNS = join(HERE, 'attribution', 'runs');

const ESC = '';

function tempDir(): string {
    return mkdtempSync(join(tmpdir(), 'attribution-'));
}

function pass(seconds: string, index: number, total: number, binaryId: string, name: string): string {
    return `        PASS [${seconds.padStart(8)}s] (${String(index).padStart(4)}/${total}) ${binaryId} ${name}`;
}

function summary(elapsed: string, run: number, passed: number, extra = ''): string {
    return `     Summary [${elapsed.padStart(8)}s] ${run} tests run: ${passed} passed${extra}`;
}

function log(...lines: string[]): string {
    return `${lines.join('\n')}\n`;
}

const TWO_FAMILIES: Family[] = [
    { id: 'alpha', package: 'demo-cli', match: { suites: ['demo-cli::alpha'] } },
    { id: 'beta', package: 'demo-cli', match: { suites: ['demo-cli::beta'] } },
];

const SIMPLE_LOG = log(
    '    Starting 3 tests across 2 binaries',
    pass('0.500', 1, 3, 'demo-cli::alpha', 'first'),
    pass('1.500', 2, 3, 'demo-cli::alpha', 'second'),
    pass('0.250', 3, 3, 'demo-cli::beta', 'third'),
    '────────────',
    summary('2.000', 3, 3),
);

// ---------------------------------------------------------------------------
// Lexing and shape
// ---------------------------------------------------------------------------

test('strips the sgr sequences nextest writes even into a redirected file', () => {
    assert.equal(stripAnsi(`${ESC}[32mPASS${ESC}[0m`), 'PASS');
    assert.equal(stripAnsi(`${ESC}[1m${ESC}[31mclaudine-cli${ESC}[0m`), 'claudine-cli');
});

test('a duration in brackets survives ansi stripping', () => {
    const line = `        ${ESC}[32m    PASS${ESC}[0m [   0.082s] (   1/2489) claudine-cli::argv_normalization a_test`;
    const runs = parseNextestLog(log('    Starting 1 tests across 1 binary', line, summary('0.100', 1, 1)), 'x');
    assert.equal(runs[0].records[0].seconds, 0.082);
});

test('package is the segment before the first double colon, and a bare suite is its own package', () => {
    assert.equal(packageOf('claudine-cli::context_command'), 'claudine-cli');
    assert.equal(packageOf('claudine-cli::bin/claudine'), 'claudine-cli');
    assert.equal(packageOf('claudine'), 'claudine');
});

test('parses result lines, the starting banner and the summary', () => {
    const [run] = parseNextestLog(SIMPLE_LOG, 'simple');
    assert.equal(run.label, 'simple');
    assert.equal(run.started, 3);
    assert.equal(run.binaries, 2);
    assert.equal(run.summary.run, 3);
    assert.equal(run.summary.passed, 3);
    assert.equal(run.summary.elapsed, 2);
    assert.deepEqual(
        run.records.map((r) => [r.binaryId, r.name, r.seconds, r.status, r.attempt]),
        [
            ['demo-cli::alpha', 'first', 0.5, 'PASS', 1],
            ['demo-cli::alpha', 'second', 1.5, 'PASS', 1],
            ['demo-cli::beta', 'third', 0.25, 'PASS', 1],
        ],
    );
});

test('a summary with skipped and slow counts still parses', () => {
    const [run] = parseNextestLog(
        log('    Starting 1 tests across 1 binary (10 tests skipped)', pass('0.010', 1, 1, 'demo-cli::alpha', 'only'), summary('0.100', 1, 1, ' (1 slow), 10 skipped')),
        'skips',
    );
    assert.equal(run.summary.skipped, 10);
    assert.equal(run.summary.failed, 0);
});

test('a slow notice is informational and never becomes a result', () => {
    const [run] = parseNextestLog(
        log(
            '    Starting 1 tests across 1 binary',
            '        SLOW [>  5.000s] (─────────) demo-cli::alpha only',
            pass('6.000', 1, 1, 'demo-cli::alpha', 'only'),
            summary('6.100', 1, 1, ' (1 slow)'),
        ),
        'slow',
    );
    assert.equal(run.records.length, 1);
    assert.deepEqual(run.slowNotices, ['demo-cli::alpha only']);
    assert.deepEqual(checkRun(run), []);
});

test('one log holding several nextest invocations yields one run per summary', () => {
    const text = log(
        '    Starting 1 tests across 1 binary',
        pass('0.100', 1, 1, 'demo-cli::alpha', 'first'),
        summary('0.200', 1, 1),
        '',
        '    Starting 1 tests across 1 binary',
        pass('0.300', 1, 1, 'demo-cli::beta', 'second'),
        summary('0.400', 1, 1),
    );
    const runs = parseNextestLog(text, 'multi');
    assert.equal(runs.length, 2);
    assert.deepEqual(runs.map((r) => r.label), ['multi', 'multi#2']);
    assert.equal(runs[0].records.length, 1);
    assert.equal(runs[1].records[0].name, 'second');
});

test('a failure line carries its status rather than being dropped', () => {
    const [run] = parseNextestLog(
        log('    Starting 1 tests across 1 binary', '        FAIL [   1.200s] (   1/1) demo-cli::alpha only', summary('1.300', 1, 0, ', 1 failed')),
        'fail',
    );
    assert.equal(run.records[0].status, 'FAIL');
    assert.equal(run.summary.failed, 1);
});

test('leak, timeout and abort statuses parse as terminal results', () => {
    for (const status of ['LEAK', 'LEAK-FAIL', 'TIMEOUT', 'SIGSEGV', 'ABORT']) {
        const [run] = parseNextestLog(
            log('    Starting 1 tests across 1 binary', `        ${status} [   1.000s] (   1/1) demo-cli::alpha only`, summary('1.100', 1, 0, ', 1 failed')),
            status,
        );
        assert.equal(run.records[0].status, status, `${status} should survive parsing`);
    }
});

test('a retry line records its attempt number', () => {
    const [run] = parseNextestLog(
        log(
            '    Starting 1 tests across 1 binary',
            '    TRY 1 FAIL [   1.000s] (   1/1) demo-cli::alpha flaky',
            '        PASS [   0.500s] (   1/1) demo-cli::alpha flaky',
            summary('1.600', 1, 1),
        ),
        'retry',
    );
    assert.deepEqual(run.records.map((r) => [r.attempt, r.status]), [[1, 'FAIL'], [1, 'PASS']]);
});

// ---------------------------------------------------------------------------
// Rejections — a printed miss is not a gate
// ---------------------------------------------------------------------------

test('a log with no summary is truncated evidence and is rejected', () => {
    assert.throws(
        () => parseNextestLog(log('    Starting 2 tests across 1 binary', pass('0.100', 1, 2, 'demo-cli::alpha', 'first')), 'cut'),
        MalformedLog,
    );
});

test('a log that is not a nextest run at all is rejected, not attributed as zero cost', () => {
    // The neuter that disables the "no summary" guard is only visible here:
    // with result lines present the truncated-tail guard fires anyway, so a
    // log holding neither — a lint or build log — is what proves the first
    // guard. Without it the gate would report an empty table and exit 0.
    const lintLog = log('    Checking claudine-cli v0.1.0', '    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.62s', 'EXIT=0');
    assert.throws(() => parseNextestLog(lintLog, 'lint'), MalformedLog);
    assert.throws(() => parseNextestLog('', 'empty'), MalformedLog);
});

test('main exits one when handed a log that is not a nextest run', () => {
    const dir = tempDir();
    try {
        const logPath = join(dir, 'just-lint.log');
        const familiesPath = join(dir, 'families.json');
        writeFileSync(logPath, log('    Finished `test` profile', 'EXIT=0'));
        writeFileSync(familiesPath, JSON.stringify({ families: TWO_FAMILIES }));
        const out = capture();
        const err = capture();
        assert.equal(main([logPath, '--families', familiesPath], out.sink, err.sink), 1);
        assert.match(err.lines.join('\n'), /truncated-log/);
        assert.doesNotMatch(out.lines.join('\n'), /GATE EXIT=0/);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('result lines after the last summary are rejected as a truncated tail', () => {
    assert.throws(
        () =>
            parseNextestLog(
                log('    Starting 1 tests across 1 binary', pass('0.100', 1, 1, 'demo-cli::alpha', 'first'), summary('0.200', 1, 1), pass('0.100', 1, 1, 'demo-cli::beta', 'orphan')),
                'tail',
            ),
        MalformedLog,
    );
});

test('a non-numeric duration is rejected rather than silently costing zero', () => {
    assert.throws(
        () =>
            parseNextestLog(
                log('    Starting 1 tests across 1 binary', '        PASS [   ...s] (   1/1) demo-cli::alpha only', summary('0.200', 1, 1)),
                'bad',
            ),
        MalformedLog,
    );
});

test('a run whose result lines disagree with its own summary count is a violation', () => {
    const [run] = parseNextestLog(
        log('    Starting 3 tests across 1 binary', pass('0.100', 1, 3, 'demo-cli::alpha', 'first'), summary('0.200', 3, 3)),
        'short',
    );
    const violations = checkRun(run);
    assert.equal(violations.length, 1);
    assert.equal(violations[0].kind, 'count-mismatch');
});

test('a red run measures nothing and is rejected', () => {
    const [run] = parseNextestLog(
        log('    Starting 1 tests across 1 binary', '        FAIL [   1.200s] (   1/1) demo-cli::alpha only', summary('1.300', 1, 0, ', 1 failed')),
        'red',
    );
    assert.deepEqual(checkRun(run).map((v) => v.kind), ['failed-run']);
});

test('the same identity reported twice at the same attempt is a duplicate', () => {
    const [run] = parseNextestLog(
        log(
            '    Starting 2 tests across 1 binary',
            pass('0.100', 1, 2, 'demo-cli::alpha', 'twice'),
            pass('0.200', 2, 2, 'demo-cli::alpha', 'twice'),
            summary('0.300', 2, 2),
        ),
        'dupe',
    );
    assert.equal(checkRun(run).filter((v) => v.kind === 'duplicate-identity').length, 1);
});

test('a retried test counts once and its final attempt decides the cost', () => {
    const [run] = parseNextestLog(
        log(
            '    Starting 1 tests across 1 binary',
            '    TRY 1 FAIL [   1.000s] (   1/1) demo-cli::alpha flaky',
            '    TRY 2 PASS [   0.500s] (   1/1) demo-cli::alpha flaky',
            summary('1.600', 1, 1),
        ),
        'retry',
    );
    assert.deepEqual(checkRun(run), []);
    const decided = decidedRecords(run);
    assert.equal(decided.length, 1);
    assert.equal(decided[0].seconds, 0.5);
    assert.equal(decided[0].status, 'PASS');
});

// ---------------------------------------------------------------------------
// Attribution
// ---------------------------------------------------------------------------

test('every result lands in exactly one family and the totals are its own', () => {
    const runs = parseNextestLog(SIMPLE_LOG, 'simple');
    const result = attribute(runs, TWO_FAMILIES);
    assert.deepEqual(result.violations, []);
    assert.equal(result.count, 3);
    assert.equal(result.summed, 2.25);
    assert.equal(result.elapsed, 2);
    const alpha = result.families.find((f) => f.id === 'alpha')!;
    assert.equal(alpha.count, 2);
    assert.equal(alpha.summed, 2);
    assert.equal(alpha.mean, 1);
    assert.equal(alpha.max, 1.5);
    assert.equal(alpha.min, 0.5);
    assert.equal(alpha.slowest.name, 'demo-cli::alpha second');
});

test('families are ranked by summed cost, not by name or count', () => {
    const runs = parseNextestLog(SIMPLE_LOG, 'simple');
    const result = attribute(runs, TWO_FAMILIES);
    assert.deepEqual(result.families.map((f) => f.id), ['alpha', 'beta']);
});

test('shares are fractions of the attributed total and sum to one', () => {
    const runs = parseNextestLog(SIMPLE_LOG, 'simple');
    const result = attribute(runs, TWO_FAMILIES);
    const total = result.families.reduce((a, f) => a + f.share, 0);
    assert.ok(Math.abs(total - 1) < 1e-9, `shares summed to ${total}`);
});

test('an identity no family claims is a violation and costs nothing', () => {
    const runs = parseNextestLog(SIMPLE_LOG, 'simple');
    const result = attribute(runs, [TWO_FAMILIES[0]]);
    assert.deepEqual(result.violations.map((v) => v.kind), ['unassigned-identity']);
    assert.equal(result.summed, 2);
    assert.equal(result.count, 2);
});

test('an identity two families claim is a violation and is counted by neither', () => {
    const overlapping: Family[] = [
        { id: 'alpha', package: 'demo-cli', match: { suites: ['demo-cli::alpha'] } },
        { id: 'alpha-too', package: 'demo-cli', match: { suites: ['demo-cli::alpha'] } },
        TWO_FAMILIES[1],
    ];
    const result = attribute(parseNextestLog(SIMPLE_LOG, 'simple'), overlapping);
    assert.equal(result.violations.filter((v) => v.kind === 'double-assigned-identity').length, 2);
    assert.equal(result.summed, 0.25);
    assert.deepEqual(result.families.map((f) => f.id), ['beta']);
});

test('attribution accumulates across several runs of the same family', () => {
    const runs = [...parseNextestLog(SIMPLE_LOG, 'a'), ...parseNextestLog(SIMPLE_LOG, 'b')];
    const result = attribute(runs, TWO_FAMILIES);
    assert.equal(result.count, 6);
    assert.equal(result.summed, 4.5);
    assert.equal(result.elapsed, 4);
});

test('a module-scoped family claims only its module, as in the reconciler', () => {
    const families: Family[] = [
        { id: 'unit', package: 'demo', match: { suites: ['demo'] }, exclude: { modules: ['heavy'] } },
        { id: 'unit-heavy', package: 'demo', match: { suites: ['demo'], modules: ['heavy'] } },
    ];
    const runs = parseNextestLog(
        log(
            '    Starting 2 tests across 1 binary',
            pass('0.100', 1, 2, 'demo', 'light::tests::cheap'),
            pass('3.000', 2, 2, 'demo', 'heavy::tests::expensive'),
            summary('3.100', 2, 2),
        ),
        'modules',
    );
    const result = attribute(runs, families);
    assert.deepEqual(result.violations, []);
    assert.deepEqual(
        result.families.map((f) => [f.id, f.summed]),
        [
            ['unit-heavy', 3],
            ['unit', 0.1],
        ],
    );
});

test('the rendered table carries the three costs and never invents a fourth', () => {
    const result = attribute(parseNextestLog(SIMPLE_LOG, 'simple'), TWO_FAMILIES);
    const table = renderAttributionTable(result);
    assert.match(table, /\| `alpha` \| `demo-cli` \| 2 \| 2\.00 s \|/);
    assert.match(table, /summed 2\.25 s; runner elapsed 2\.00 s/);
});

test('the table can be capped to the most expensive families', () => {
    const result = attribute(parseNextestLog(SIMPLE_LOG, 'simple'), TWO_FAMILIES);
    const table = renderAttributionTable(result, 1);
    assert.match(table, /`alpha`/);
    assert.doesNotMatch(table, /`beta`/);
});

// ---------------------------------------------------------------------------
// Per-binary attribution — repeated setup is visible as processes × floor
// ---------------------------------------------------------------------------

test('per-binary attribution counts one process per result and keeps the cheapest case', () => {
    const runs = parseNextestLog(SIMPLE_LOG, 'simple');
    const binaries = attributeByBinary(runs);
    assert.deepEqual(
        binaries.map((b) => [b.binaryId, b.count, b.summed, b.min, b.max]),
        [
            ['demo-cli::alpha', 2, 2, 0.5, 1.5],
            ['demo-cli::beta', 1, 0.25, 0.25, 0.25],
        ],
    );
});

test('per-binary attribution needs no families and so cannot be skewed by one', () => {
    const scan = log(
        '    Starting 3 tests across 1 binary',
        pass('1.800', 1, 3, 'demo-cli::guards', 'scans_then_asserts'),
        pass('1.750', 2, 3, 'demo-cli::guards', 'scans_then_asserts_again'),
        pass('0.012', 3, 3, 'demo-cli::guards', 'unit_test_of_the_scanner'),
        summary('1.810', 3, 3),
    );
    const [binary] = attributeByBinary(parseNextestLog(scan, 'scan'));
    assert.equal(binary.count, 3);
    assert.ok(Math.abs(binary.summed - 3.562) < 1e-9);
    assert.equal(binary.min, 0.012);
    assert.equal(binary.max, 1.8);
});

test('a retried test is one process in the per-binary table, not two', () => {
    const [binary] = attributeByBinary(
        parseNextestLog(
            log(
                '    Starting 1 tests across 1 binary',
                '    TRY 1 FAIL [   1.000s] (   1/1) demo-cli::alpha flaky',
                '    TRY 2 PASS [   0.500s] (   1/1) demo-cli::alpha flaky',
                summary('1.600', 1, 1),
            ),
            'retry',
        ),
    );
    assert.equal(binary.count, 1);
    assert.equal(binary.summed, 0.5);
});

test('the binary table renders the process count beside the summed cost', () => {
    const table = renderBinaryTable(attributeByBinary(parseNextestLog(SIMPLE_LOG, 'simple')));
    assert.match(table, /\| Binary \| Processes \| Summed \| Mean \| Max \| Min \|/);
    assert.match(table, /\| `demo-cli::alpha` \| 2 \| 2\.00 s \|/);
});

test('main renders the binary table when asked and the family table otherwise', () => {
    const dir = tempDir();
    try {
        const logPath = join(dir, 'run.log');
        const familiesPath = join(dir, 'families.json');
        writeFileSync(logPath, SIMPLE_LOG);
        writeFileSync(familiesPath, JSON.stringify({ families: TWO_FAMILIES }));
        const byBinary = capture();
        assert.equal(main([logPath, '--families', familiesPath, '--by-binary'], byBinary.sink, capture().sink), 0);
        assert.match(byBinary.lines.join('\n'), /Processes/);
        const byFamily = capture();
        assert.equal(main([logPath, '--families', familiesPath], byFamily.sink, capture().sink), 0);
        assert.doesNotMatch(byFamily.lines.join('\n'), /Processes/);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// ---------------------------------------------------------------------------
// Budgets — the refusals are the point
// ---------------------------------------------------------------------------

const CI_BUDGET_INPUT: BudgetInput = {
    provenance: { kind: 'ci', legs: ['ubuntu-latest'], runsPerLeg: { 'ubuntu-latest': 3 } },
    perLegFamilySummed: { 'ubuntu-latest': { alpha: [10, 12, 11] } },
};

test('a budget is never derived from local evidence', () => {
    const result = deriveBudgets({
        ...CI_BUDGET_INPUT,
        provenance: { kind: 'local', legs: ['this-host'], runsPerLeg: { 'this-host': 5 } },
        perLegFamilySummed: { 'this-host': { alpha: [10, 12, 11, 10, 11] } },
    });
    assert.deepEqual(result.budgets, []);
    assert.deepEqual(result.violations.map((v) => v.kind), ['local-derived-budget']);
});

test('a leg with fewer than three consecutive green runs yields no budget', () => {
    const result = deriveBudgets({
        provenance: { kind: 'ci', legs: ['macos-latest'], runsPerLeg: { 'macos-latest': 2 } },
        perLegFamilySummed: { 'macos-latest': { alpha: [10, 12] } },
    });
    assert.deepEqual(result.budgets, []);
    assert.deepEqual(result.violations.map((v) => v.kind), ['insufficient-runs']);
    assert.match(result.violations[0].detail, new RegExp(`${REQUIRED_RUNS_PER_LEG} consecutive`));
});

test('a declared leg with no measurements is a missing leg, not an average of the others', () => {
    const result = deriveBudgets({
        provenance: { kind: 'ci', legs: ['ubuntu-latest', 'windows-latest'], runsPerLeg: { 'ubuntu-latest': 3, 'windows-latest': 3 } },
        perLegFamilySummed: { 'ubuntu-latest': { alpha: [10, 12, 11] } },
    });
    assert.deepEqual(result.budgets, []);
    assert.deepEqual(result.violations.map((v) => v.kind), ['missing-leg']);
});

test('a family with fewer samples than runs yields no budget for the whole set', () => {
    const result = deriveBudgets({
        provenance: { kind: 'ci', legs: ['ubuntu-latest'], runsPerLeg: { 'ubuntu-latest': 3 } },
        perLegFamilySummed: { 'ubuntu-latest': { alpha: [10, 12, 11], beta: [4] } },
    });
    assert.deepEqual(result.budgets, []);
    assert.deepEqual(result.violations.map((v) => v.kind), ['insufficient-runs']);
});

test('no leg at all is a violation rather than an empty pass', () => {
    const result = deriveBudgets({ provenance: { kind: 'ci', legs: [], runsPerLeg: {} }, perLegFamilySummed: {} });
    assert.deepEqual(result.violations.map((v) => v.kind), ['missing-leg']);
});

test('a negative or non-finite headroom is rejected before any budget is computed', () => {
    for (const headroom of [-0.1, Number.NaN, Number.POSITIVE_INFINITY]) {
        const result = deriveBudgets({ ...CI_BUDGET_INPUT, headroom });
        assert.deepEqual(result.budgets, []);
        assert.deepEqual(result.violations.map((v) => v.kind), ['invalid-headroom']);
    }
});

test('a complete ci set yields a budget from the worst run plus headroom', () => {
    const result = deriveBudgets(CI_BUDGET_INPUT);
    assert.deepEqual(result.violations, []);
    assert.deepEqual(result.budgets, [{ leg: 'ubuntu-latest', family: 'alpha', observedMax: 12, budget: 15 }]);
});

test('headroom is explicit and changes the ceiling, not the observed maximum', () => {
    const result = deriveBudgets({ ...CI_BUDGET_INPUT, headroom: 0 });
    assert.deepEqual(result.budgets, [{ leg: 'ubuntu-latest', family: 'alpha', observedMax: 12, budget: 12 }]);
});

test('every leg gets its own budget: cross-platform counts are never merged', () => {
    const result = deriveBudgets({
        provenance: {
            kind: 'ci',
            legs: ['ubuntu-latest', 'windows-latest'],
            runsPerLeg: { 'ubuntu-latest': 3, 'windows-latest': 3 },
        },
        perLegFamilySummed: {
            'ubuntu-latest': { alpha: [10, 12, 11] },
            'windows-latest': { alpha: [40, 44, 41] },
        },
        headroom: 0.25,
    });
    assert.deepEqual(result.violations, []);
    assert.deepEqual(
        result.budgets.map((b) => [b.leg, b.budget]),
        [
            ['ubuntu-latest', 15],
            ['windows-latest', 55],
        ],
    );
});

test('the budget table renders one row per leg and family', () => {
    const { budgets } = deriveBudgets(CI_BUDGET_INPUT);
    assert.match(renderBudgetTable(budgets), /\| ubuntu-latest \| `alpha` \| 12\.00 s \| 15\.00 s \|/);
});

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function capture(): { lines: string[]; sink: (line: string) => void } {
    const lines: string[] = [];
    return { lines, sink: (line: string) => lines.push(line) };
}

test('main exits two without a log or a families file', () => {
    const out = capture();
    const err = capture();
    assert.equal(main([], out.sink, err.sink), 2);
    assert.equal(main(['some.log'], out.sink, err.sink), 2);
    assert.match(err.lines.join('\n'), /usage:/);
});

test('main exits two when a named path does not exist', () => {
    const dir = tempDir();
    try {
        const err = capture();
        assert.equal(main([join(dir, 'missing.log'), '--families', FAMILIES], capture().sink, err.sink), 2);
        assert.match(err.lines.join('\n'), /not a readable file/);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('main exits two when --top is not a non-negative integer', () => {
    const dir = tempDir();
    try {
        const logPath = join(dir, 'run.log');
        const familiesPath = join(dir, 'families.json');
        writeFileSync(logPath, SIMPLE_LOG);
        writeFileSync(familiesPath, JSON.stringify({ families: TWO_FAMILIES }));
        const err = capture();
        assert.equal(main([logPath, '--families', familiesPath, '--top', 'lots'], capture().sink, err.sink), 2);
        assert.match(err.lines.join('\n'), /--top expects/);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('main exits one on a truncated log rather than throwing', () => {
    const dir = tempDir();
    try {
        const logPath = join(dir, 'run.log');
        const familiesPath = join(dir, 'families.json');
        writeFileSync(logPath, log('    Starting 1 tests across 1 binary', pass('0.100', 1, 1, 'demo-cli::alpha', 'first')));
        writeFileSync(familiesPath, JSON.stringify({ families: TWO_FAMILIES }));
        const err = capture();
        assert.equal(main([logPath, '--families', familiesPath], capture().sink, err.sink), 1);
        assert.match(err.lines.join('\n'), /truncated-log/);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('main exits one when a result belongs to no family', () => {
    const dir = tempDir();
    try {
        const logPath = join(dir, 'run.log');
        const familiesPath = join(dir, 'families.json');
        writeFileSync(logPath, SIMPLE_LOG);
        writeFileSync(familiesPath, JSON.stringify({ families: [TWO_FAMILIES[0]] }));
        const err = capture();
        assert.equal(main([logPath, '--families', familiesPath], capture().sink, err.sink), 1);
        assert.match(err.lines.join('\n'), /unassigned-identity/);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('main exits one and prints no budget table when the budget input is local', () => {
    const dir = tempDir();
    try {
        const logPath = join(dir, 'run.log');
        const familiesPath = join(dir, 'families.json');
        const budgetsPath = join(dir, 'budgets.json');
        writeFileSync(logPath, SIMPLE_LOG);
        writeFileSync(familiesPath, JSON.stringify({ families: TWO_FAMILIES }));
        writeFileSync(
            budgetsPath,
            JSON.stringify({
                provenance: { kind: 'local', legs: ['this-host'], runsPerLeg: { 'this-host': 5 } },
                perLegFamilySummed: { 'this-host': { alpha: [1, 1, 1, 1, 1] } },
            }),
        );
        const out = capture();
        const err = capture();
        assert.equal(main([logPath, '--families', familiesPath, '--budgets', budgetsPath], out.sink, err.sink), 1);
        assert.match(err.lines.join('\n'), /local-derived-budget/);
        assert.doesNotMatch(out.lines.join('\n'), /Budget/);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('main exits zero and prints the table on a clean log', () => {
    const dir = tempDir();
    try {
        const logPath = join(dir, 'run.log');
        const familiesPath = join(dir, 'families.json');
        writeFileSync(logPath, SIMPLE_LOG);
        writeFileSync(familiesPath, JSON.stringify({ families: TWO_FAMILIES }));
        const out = capture();
        assert.equal(main([logPath, '--families', familiesPath], out.sink, capture().sink), 0);
        assert.match(out.lines.join('\n'), /GATE EXIT=0/);
        assert.match(out.lines.join('\n'), /`alpha`/);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

test('a non-json budget file is a usage error, not a violation', () => {
    const dir = tempDir();
    try {
        const logPath = join(dir, 'run.log');
        const familiesPath = join(dir, 'families.json');
        const budgetsPath = join(dir, 'budgets.json');
        writeFileSync(logPath, SIMPLE_LOG);
        writeFileSync(familiesPath, JSON.stringify({ families: TWO_FAMILIES }));
        writeFileSync(budgetsPath, 'not json');
        assert.throws(() => runGate({ logPaths: [logPath], familiesPath, budgetsPath }), UsageError);
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }
});

// ---------------------------------------------------------------------------
// Passive corpus over the shipped evidence
// ---------------------------------------------------------------------------

function shippedLogs(): string[] {
    const paths: string[] = [];
    for (const dir of [LOCAL_GATES, RECIPES, RUNS]) {
        if (!existsSync(dir)) continue;
        for (const entry of readdirSync(dir)) {
            if (!entry.endsWith('.log')) continue;
            const path = join(dir, entry);
            // Not nextest runs: lint/build gates, doctests, and the
            // single-process `cargo test` comparisons Phase 3 measured.
            if (/^(ci-local|just-lint|just-check-windows|just-doctest|cargotest-)/.test(entry)) continue;
            paths.push(path);
        }
    }
    return paths;
}

test('every shipped nextest log parses and reports a complete, green run', () => {
    const logs = shippedLogs();
    assert.ok(logs.length > 0, 'the fix ships nextest run logs to attribute');
    for (const path of logs) {
        const runs = parseNextestLog(readFileSync(path, 'utf8'), path);
        for (const run of runs) {
            assert.deepEqual(checkRun(run), [], `${path}: ${JSON.stringify(checkRun(run))}`);
        }
    }
});

test('every result in every shipped log belongs to exactly one declared family', () => {
    const families = parseFamilyFile(readFileSync(FAMILIES, 'utf8')).families;
    const runs: ParsedRun[] = [];
    for (const path of shippedLogs()) runs.push(...parseNextestLog(readFileSync(path, 'utf8'), path));
    const result = attribute(runs, families);
    assert.deepEqual(result.violations, [], JSON.stringify(result.violations.slice(0, 5), null, 2));
    assert.ok(result.count > 7000, `attributed ${result.count} results`);
});

test('the shipped local gate logs attribute the same summed cost every time they are read', () => {
    const families = parseFamilyFile(readFileSync(FAMILIES, 'utf8')).families;
    const path = join(LOCAL_GATES, 'just-test-cli.log');
    const first = attribute(parseNextestLog(readFileSync(path, 'utf8'), 'cli'), families);
    const second = attribute(parseNextestLog(readFileSync(path, 'utf8'), 'cli'), families);
    assert.equal(first.summed, second.summed);
    assert.equal(first.count, second.count);
    assert.deepEqual(first.families.map((f) => f.id), second.families.map((f) => f.id));
});

test('the whole gate runs end to end over the real local gate log and the real families', () => {
    const result = runGate({ logPaths: [join(LOCAL_GATES, 'just-test-cli.log')], familiesPath: FAMILIES });
    assert.deepEqual(result.violations, []);
    assert.ok(result.attribution.count > 2000);
    assert.match(result.table, /\| Family \| Package \| Tests \| Summed \| Mean \| Max \| Share \|/);
});
