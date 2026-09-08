/**
 * Cost attribution over nextest run logs (fix 2026-09-07-faster-claudine-tests, Phase 3).
 *
 * Phase 2 proved *which* identities exist and which family owns each one.
 * Phase 3 asks what they cost, and the spec's fifth required behavior insists
 * the three costs stay apart: build/setup, runner elapsed, and summed test
 * duration. This gate reads nextest's own run output, joins every result line
 * to the same `families.json` the reconciler uses — so an attribution table can
 * never disagree with the inventory about who owns a test — and reports summed,
 * mean and maximum duration per family.
 *
 * It is a gate rather than a report:
 *
 *   1. a log with no `Summary` line is truncated evidence and is rejected;
 *   2. a run whose result lines disagree with its own summary count is
 *      rejected — a partially captured log must not read as a cheap run;
 *   3. a run containing any non-passing terminal status is rejected: a red run
 *      measures nothing;
 *   4. a result line for an identity no family claims, or that two families
 *      claim, is a violation, exactly as in the reconciler;
 *   5. `deriveBudgets` refuses to emit a budget from local evidence. Phase 3's
 *      checkpoint says budgets come from the CI baseline and that local timing
 *      may attribute cost but may not set a target, so the refusal is code
 *      rather than a promise.
 *
 * Exit codes: 0 clean, 1 violations, 2 usage error.
 */

import { readFileSync, existsSync, statSync } from 'node:fs';
import { basename } from 'node:path';

import { parseFamilyFile, familiesMatching, type Family, type Identity } from './inventory-reconciler.ts';

// ---------------------------------------------------------------------------
// Errors and violations
// ---------------------------------------------------------------------------

/** A run log that cannot be interpreted as nextest output. */
export class MalformedLog extends Error {}

/** A usage problem: missing argument, unreadable file. */
export class UsageError extends Error {}

export type AttributionViolationKind =
    | 'truncated-log'
    | 'count-mismatch'
    | 'failed-run'
    | 'duplicate-identity'
    | 'unassigned-identity'
    | 'double-assigned-identity'
    | 'local-derived-budget'
    | 'insufficient-runs'
    | 'missing-leg'
    | 'invalid-headroom';

export interface AttributionViolation {
    kind: AttributionViolationKind;
    detail: string;
}

// ---------------------------------------------------------------------------
// Nextest run-log parsing
// ---------------------------------------------------------------------------

/** Terminal statuses nextest prints once per test attempt. */
export const TERMINAL_STATUSES = ['PASS', 'FAIL', 'LEAK', 'LEAK-FAIL', 'TIMEOUT', 'SIGSEGV', 'ABORT', 'SIGABRT'] as const;
export type TerminalStatus = (typeof TERMINAL_STATUSES)[number];

export interface RunRecord {
    binaryId: string;
    pkg: string;
    name: string;
    /** Duration nextest reported for the attempt that decided the result. */
    seconds: number;
    status: TerminalStatus;
    /** Attempt number; 1 unless the line was a `TRY n` retry. */
    attempt: number;
}

export interface RunSummary {
    /** `N tests run` */
    run: number;
    passed: number;
    failed: number;
    skipped: number;
    /** Runner elapsed as nextest measured it, in seconds. */
    elapsed: number;
}

export interface ParsedRun {
    /** Log file stem plus an index when one log holds several runs. */
    label: string;
    /** `Starting N tests across M binaries`, when the log kept that line. */
    started?: number;
    binaries?: number;
    summary: RunSummary;
    records: RunRecord[];
    /** `SLOW [> …]` notices: informational, never a result. */
    slowNotices: string[];
}

const ANSI = /\u001B\[[0-9;]*[A-Za-z]/g;

/** Strip SGR and cursor sequences; nextest colorizes even when redirected. */
export function stripAnsi(text: string): string {
    return text.replace(ANSI, '');
}

const STARTING = /^\s*Starting (\d+) tests? across (\d+) binar(?:y|ies)/;
const SUMMARY = /^\s*Summary \[\s*([0-9.]+)s\]\s+(\d+) tests? run: (\d+) passed(?: \([^)]*\))?(?:, (\d+) failed)?(?:, (\d+) skipped)?/;
const RESULT =
    /^\s*(?:TRY (\d+) )?(PASS|FAIL|LEAK-FAIL|LEAK|TIMEOUT|SIGSEGV|SIGABRT|ABORT)\s+\[\s*([0-9.]+)s\]\s+\([^)]*\)\s+(\S+)\s+(\S+)\s*$/;
const SLOW = /^\s*SLOW\s+\[>\s+[0-9.]+s\]\s+\([^)]*\)\s+(\S+)\s+(\S+)\s*$/;

/** `claudine-cli::context_command` → `claudine-cli`; `claudine` → `claudine`. */
export function packageOf(binaryId: string): string {
    const marker = binaryId.indexOf('::');
    return marker === -1 ? binaryId : binaryId.slice(0, marker);
}

function toNumber(raw: string, what: string): number {
    const value = Number(raw);
    if (!Number.isFinite(value) || value < 0) {
        throw new MalformedLog(`${what}: not a usable duration: ${JSON.stringify(raw)}`);
    }
    return value;
}

/**
 * Parse every nextest run in one log.
 *
 * A single log may hold several runs: the area's `test-l2` and
 * `test-rendezvous` recipes invoke nextest once per package. Each `Summary`
 * line closes a run; result lines before the first `Starting` line still
 * belong to the first run, because some recipes trim the banner.
 *
 * ## Errors
 *
 * Throws [`MalformedLog`] when the text holds result lines but no summary
 * (a truncated capture), or a duration that is not a number.
 */
export function parseNextestLog(text: string, label: string): ParsedRun[] {
    const lines = stripAnsi(text).split(/\r?\n/);
    const runs: ParsedRun[] = [];
    let records: RunRecord[] = [];
    let slowNotices: string[] = [];
    let started: number | undefined;
    let binaries: number | undefined;

    for (const line of lines) {
        const starting = STARTING.exec(line);
        if (starting) {
            started = Number(starting[1]);
            binaries = Number(starting[2]);
            continue;
        }
        const slow = SLOW.exec(line);
        if (slow) {
            slowNotices.push(`${slow[1]} ${slow[2]}`);
            continue;
        }
        const result = RESULT.exec(line);
        if (result) {
            const [, attempt, status, seconds, binaryId, name] = result;
            records.push({
                binaryId,
                pkg: packageOf(binaryId),
                name,
                seconds: toNumber(seconds, `${label}: ${binaryId} ${name}`),
                status: status as TerminalStatus,
                attempt: attempt === undefined ? 1 : Number(attempt),
            });
            continue;
        }
        const summary = SUMMARY.exec(line);
        if (summary) {
            const [, elapsed, run, passed, failed, skipped] = summary;
            runs.push({
                label: runs.length === 0 ? label : `${label}#${runs.length + 1}`,
                started,
                binaries,
                summary: {
                    run: Number(run),
                    passed: Number(passed),
                    failed: failed === undefined ? 0 : Number(failed),
                    skipped: skipped === undefined ? 0 : Number(skipped),
                    elapsed: toNumber(elapsed, `${label}: summary elapsed`),
                },
                records,
                slowNotices,
            });
            records = [];
            slowNotices = [];
            started = undefined;
            binaries = undefined;
        }
    }

    if (runs.length === 0) {
        throw new MalformedLog(`${label}: no nextest summary line; the log is truncated or is not a run log`);
    }
    if (records.length > 0) {
        throw new MalformedLog(`${label}: ${records.length} result line(s) after the last summary; the log is truncated`);
    }
    return runs;
}

/**
 * Reject a run that cannot serve as evidence.
 *
 * Retries are collapsed first: nextest prints `TRY n` lines for the same
 * identity, and only the final attempt decides the result.
 */
export function checkRun(run: ParsedRun): AttributionViolation[] {
    const violations: AttributionViolation[] = [];
    const decided = new Map<string, RunRecord>();
    for (const record of run.records) {
        const key = `${record.binaryId} ${record.name}`;
        const previous = decided.get(key);
        if (previous === undefined) {
            decided.set(key, record);
            continue;
        }
        if (previous.attempt === record.attempt) {
            violations.push({
                kind: 'duplicate-identity',
                detail: `${run.label}: ${record.binaryId} ${record.name} reported twice at attempt ${record.attempt}`,
            });
            continue;
        }
        if (record.attempt > previous.attempt) decided.set(key, record);
    }
    if (decided.size !== run.summary.run) {
        violations.push({
            kind: 'count-mismatch',
            detail: `${run.label}: ${decided.size} decided result line(s) but the summary claims ${run.summary.run} tests run`,
        });
    }
    for (const record of decided.values()) {
        if (record.status !== 'PASS') {
            violations.push({
                kind: 'failed-run',
                detail: `${run.label}: ${record.binaryId} ${record.name} ended ${record.status}; a red run measures nothing`,
            });
        }
    }
    return violations;
}

/** The final attempt for each identity, in log order. */
export function decidedRecords(run: ParsedRun): RunRecord[] {
    const decided = new Map<string, RunRecord>();
    for (const record of run.records) {
        const key = `${record.binaryId} ${record.name}`;
        const previous = decided.get(key);
        if (previous === undefined || record.attempt > previous.attempt) decided.set(key, record);
    }
    return [...decided.values()];
}

// ---------------------------------------------------------------------------
// Family attribution
// ---------------------------------------------------------------------------

export interface FamilyCost {
    id: string;
    package: string;
    count: number;
    summed: number;
    mean: number;
    max: number;
    min: number;
    /** Identity and duration of the single most expensive member. */
    slowest: { name: string; seconds: number };
    /** Fraction of the attributed summed duration, 0–1. */
    share: number;
}

export interface Attribution {
    families: FamilyCost[];
    violations: AttributionViolation[];
    /** Summed duration over every attributed record. */
    summed: number;
    /** Runner elapsed summed over the parsed runs. */
    elapsed: number;
    count: number;
}

function toIdentity(record: RunRecord): Identity {
    return {
        id: `${record.binaryId} ${record.name}`,
        pkg: record.pkg,
        binaryId: record.binaryId,
        kind: 'test',
        name: record.name,
        captures: [],
    };
}

/**
 * Join decided results to families and total the cost of each.
 *
 * Assignment uses the reconciler's own matcher, so the two gates cannot drift
 * on family membership. An identity matched by zero or by more than one family
 * is a violation and contributes to no total — an overlap must not inflate a
 * family's cost any more than it may inflate its count.
 */
export function attribute(runs: ParsedRun[], families: Family[]): Attribution {
    const violations: AttributionViolation[] = [];
    const perFamily = new Map<string, { family: Family; durations: { name: string; seconds: number }[] }>();
    let summed = 0;
    let elapsed = 0;
    let count = 0;

    for (const run of runs) {
        elapsed += run.summary.elapsed;
        for (const record of decidedRecords(run)) {
            const hits = familiesMatching(toIdentity(record), families);
            if (hits.length === 0) {
                violations.push({ kind: 'unassigned-identity', detail: `${record.binaryId} ${record.name} matches no family` });
                continue;
            }
            if (hits.length > 1) {
                violations.push({
                    kind: 'double-assigned-identity',
                    detail: `${record.binaryId} ${record.name} matches ${hits.length} families: ${hits.map((h) => h.id).join(', ')}`,
                });
                continue;
            }
            const family = hits[0];
            const bucket = perFamily.get(family.id) ?? { family, durations: [] };
            bucket.durations.push({ name: `${record.binaryId} ${record.name}`, seconds: record.seconds });
            perFamily.set(family.id, bucket);
            summed += record.seconds;
            count += 1;
        }
    }

    const costs: FamilyCost[] = [];
    for (const { family, durations } of perFamily.values()) {
        const seconds = durations.map((d) => d.seconds);
        const familySummed = seconds.reduce((a, b) => a + b, 0);
        const slowest = durations.reduce((a, b) => (b.seconds > a.seconds ? b : a));
        costs.push({
            id: family.id,
            package: family.package,
            count: durations.length,
            summed: familySummed,
            mean: familySummed / durations.length,
            max: Math.max(...seconds),
            min: Math.min(...seconds),
            slowest,
            share: summed === 0 ? 0 : familySummed / summed,
        });
    }
    costs.sort((a, b) => b.summed - a.summed);
    return { families: costs, violations, summed, elapsed, count };
}

export interface BinaryCost {
    binaryId: string;
    package: string;
    /** Result lines, which under nextest is also the number of processes. */
    count: number;
    summed: number;
    mean: number;
    max: number;
    min: number;
}

/**
 * Total cost per test binary rather than per family.
 *
 * Nextest runs each test in its own process, so for a binary whose cases all
 * repeat one setup — a source scan, a corpus parse — `count` is the number of
 * times that setup was paid and `min` is close to the cost of a case that
 * pays it and asserts almost nothing. That pair is what makes repeated work
 * visible without instrumenting the test.
 */
export function attributeByBinary(runs: ParsedRun[]): BinaryCost[] {
    const perBinary = new Map<string, number[]>();
    for (const run of runs) {
        for (const record of decidedRecords(run)) {
            const seconds = perBinary.get(record.binaryId) ?? [];
            seconds.push(record.seconds);
            perBinary.set(record.binaryId, seconds);
        }
    }
    const costs: BinaryCost[] = [];
    for (const [binaryId, seconds] of perBinary) {
        const summed = seconds.reduce((a, b) => a + b, 0);
        costs.push({
            binaryId,
            package: packageOf(binaryId),
            count: seconds.length,
            summed,
            mean: summed / seconds.length,
            max: Math.max(...seconds),
            min: Math.min(...seconds),
        });
    }
    costs.sort((a, b) => b.summed - a.summed);
    return costs;
}

export function renderBinaryTable(costs: BinaryCost[], top = 0): string {
    const rows = top > 0 ? costs.slice(0, top) : costs;
    const lines = ['| Binary | Processes | Summed | Mean | Max | Min |', '|---|---:|---:|---:|---:|---:|'];
    for (const cost of rows) {
        lines.push(
            `| \`${cost.binaryId}\` | ${cost.count} | ${cost.summed.toFixed(2)} s | ` +
                `${cost.mean.toFixed(3)} s | ${cost.max.toFixed(3)} s | ${cost.min.toFixed(3)} s |`,
        );
    }
    return lines.join('\n');
}

// ---------------------------------------------------------------------------
// Budgets
// ---------------------------------------------------------------------------

export interface BudgetProvenance {
    /** `ci` is the only provenance a budget may be derived from. */
    kind: 'ci' | 'local';
    /** Every environment leg a budget must cover. */
    legs: string[];
    /** Consecutive green runs collected per leg. */
    runsPerLeg: Record<string, number>;
}

export interface BudgetInput {
    provenance: BudgetProvenance;
    /** leg → family → summed duration, one entry per collected run. */
    perLegFamilySummed: Record<string, Record<string, number[]>>;
    /** Fraction of headroom over the worst observed run. Defaults to 0.25. */
    headroom?: number;
}

export interface Budget {
    leg: string;
    family: string;
    /** Worst summed duration observed across the leg's runs. */
    observedMax: number;
    /** The ratified ceiling: `observedMax × (1 + headroom)`. */
    budget: number;
}

/** Consecutive green runs a leg needs before a budget may be derived from it. */
export const REQUIRED_RUNS_PER_LEG = 3;

/**
 * Derive per-family budgets, or refuse and say why.
 *
 * The refusals are the point. Phase 3's checkpoint requires that no budget be
 * derived from a local run alone and that every configured leg be represented
 * by three consecutive green runs, so `kind: 'local'`, a missing leg, and a
 * short run count each yield a violation and **no** budget rather than a
 * plausible-looking number.
 */
export function deriveBudgets(input: BudgetInput): { budgets: Budget[]; violations: AttributionViolation[] } {
    const violations: AttributionViolation[] = [];
    const headroom = input.headroom ?? 0.25;
    if (!Number.isFinite(headroom) || headroom < 0) {
        violations.push({ kind: 'invalid-headroom', detail: `headroom must be a finite fraction >= 0, got ${JSON.stringify(input.headroom)}` });
        return { budgets: [], violations };
    }
    if (input.provenance.kind !== 'ci') {
        violations.push({
            kind: 'local-derived-budget',
            detail: 'budgets are derived from the CI baseline only; local timing attributes cost and sets no target',
        });
        return { budgets: [], violations };
    }
    if (input.provenance.legs.length === 0) {
        violations.push({ kind: 'missing-leg', detail: 'provenance declares no environment leg' });
        return { budgets: [], violations };
    }

    const budgets: Budget[] = [];
    for (const leg of input.provenance.legs) {
        const runs = input.provenance.runsPerLeg[leg] ?? 0;
        const families = input.perLegFamilySummed[leg];
        if (families === undefined) {
            violations.push({ kind: 'missing-leg', detail: `${leg}: declared but carries no measurements` });
            continue;
        }
        if (runs < REQUIRED_RUNS_PER_LEG) {
            violations.push({
                kind: 'insufficient-runs',
                detail: `${leg}: ${runs} green run(s); ${REQUIRED_RUNS_PER_LEG} consecutive are required`,
            });
            continue;
        }
        for (const [family, samples] of Object.entries(families)) {
            if (samples.length < REQUIRED_RUNS_PER_LEG) {
                violations.push({
                    kind: 'insufficient-runs',
                    detail: `${leg}/${family}: ${samples.length} sample(s); ${REQUIRED_RUNS_PER_LEG} are required`,
                });
                continue;
            }
            const observedMax = Math.max(...samples);
            budgets.push({
                leg,
                family,
                observedMax,
                budget: Math.ceil(observedMax * (1 + headroom) * 100) / 100,
            });
        }
    }
    if (violations.length > 0) return { budgets: [], violations };
    return { budgets, violations };
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

export function renderAttributionTable(attribution: Attribution, top = 0): string {
    const rows = top > 0 ? attribution.families.slice(0, top) : attribution.families;
    const lines = ['| Family | Package | Tests | Summed | Mean | Max | Share |', '|---|---|---:|---:|---:|---:|---:|'];
    for (const family of rows) {
        lines.push(
            `| \`${family.id}\` | \`${family.package}\` | ${family.count} | ${family.summed.toFixed(2)} s | ` +
                `${family.mean.toFixed(3)} s | ${family.max.toFixed(2)} s | ${(family.share * 100).toFixed(1)}% |`,
        );
    }
    lines.push('');
    lines.push(
        `attributed ${attribution.count} result(s); summed ${attribution.summed.toFixed(2)} s; ` +
            `runner elapsed ${attribution.elapsed.toFixed(2)} s`,
    );
    return lines.join('\n');
}

export function renderBudgetTable(budgets: Budget[]): string {
    const lines = ['| Leg | Family | Worst observed | Budget |', '|---|---|---:|---:|'];
    for (const budget of budgets) {
        lines.push(`| ${budget.leg} | \`${budget.family}\` | ${budget.observedMax.toFixed(2)} s | ${budget.budget.toFixed(2)} s |`);
    }
    return lines.join('\n');
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

export interface GateInput {
    logPaths: string[];
    familiesPath: string;
    budgetsPath?: string;
    top?: number;
    /** Render the per-binary table instead of the per-family one. */
    byBinary?: boolean;
}

export interface GateOutput {
    attribution: Attribution;
    binaries: BinaryCost[];
    violations: AttributionViolation[];
    budgets: Budget[];
    table: string;
}

export function runGate(input: GateInput): GateOutput {
    const families = parseFamilyFile(readFileSync(input.familiesPath, 'utf8')).families;
    const runs: ParsedRun[] = [];
    const violations: AttributionViolation[] = [];
    for (const path of input.logPaths) {
        const parsed = parseNextestLog(readFileSync(path, 'utf8'), basename(path).replace(/\.log$/, ''));
        for (const run of parsed) violations.push(...checkRun(run));
        runs.push(...parsed);
    }
    const attribution = attribute(runs, families);
    violations.push(...attribution.violations);

    let budgets: Budget[] = [];
    if (input.budgetsPath !== undefined) {
        let raw: unknown;
        try {
            raw = JSON.parse(readFileSync(input.budgetsPath, 'utf8'));
        } catch (error) {
            throw new UsageError(`${input.budgetsPath}: not JSON: ${(error as Error).message}`);
        }
        const derived = deriveBudgets(raw as BudgetInput);
        budgets = derived.budgets;
        violations.push(...derived.violations);
    }

    const binaries = attributeByBinary(runs);
    const table = input.byBinary
        ? renderBinaryTable(binaries, input.top ?? 0)
        : renderAttributionTable(attribution, input.top ?? 0);
    return { attribution, binaries, violations, budgets, table };
}

export function main(argv: string[], out: (line: string) => void = console.log, err: (line: string) => void = console.error): number {
    const flagValue = (name: string): string | undefined => {
        const index = argv.indexOf(`--${name}`);
        return index === -1 ? undefined : argv[index + 1];
    };
    const flagged = new Set<string>();
    for (const name of ['families', 'budgets', 'top']) {
        const index = argv.indexOf(`--${name}`);
        if (index !== -1) {
            flagged.add(argv[index]);
            if (argv[index + 1] !== undefined) flagged.add(argv[index + 1]);
        }
    }
    const logPaths = argv.filter((a) => !a.startsWith('--') && !flagged.has(a));
    const familiesPath = flagValue('families');
    if (logPaths.length === 0 || familiesPath === undefined) {
        err('usage: attribution.ts <run.log...> --families <families.json> [--budgets <budgets.json>] [--top <n>] [--by-binary]');
        return 2;
    }
    const budgetsPath = flagValue('budgets');
    for (const path of [...logPaths, familiesPath, ...(budgetsPath ? [budgetsPath] : [])]) {
        if (!existsSync(path) || !statSync(path).isFile()) {
            err(`not a readable file: ${path}`);
            return 2;
        }
    }
    const top = flagValue('top') === undefined ? 0 : Number(flagValue('top'));
    if (!Number.isInteger(top) || top < 0) {
        err(`--top expects a non-negative integer, got ${JSON.stringify(flagValue('top'))}`);
        return 2;
    }

    let result: GateOutput;
    try {
        result = runGate({ logPaths, familiesPath, budgetsPath, top, byBinary: argv.includes('--by-binary') });
    } catch (error) {
        if (error instanceof UsageError) {
            err(String(error.message));
            return 2;
        }
        if (error instanceof MalformedLog) {
            err(`1 violation(s):\n  [truncated-log] ${error.message}`);
            err('GATE EXIT=1');
            return 1;
        }
        throw error;
    }

    out(result.table);
    if (result.budgets.length > 0) {
        out('');
        out(renderBudgetTable(result.budgets));
    }
    if (result.violations.length > 0) {
        err(`${result.violations.length} violation(s):`);
        for (const violation of result.violations) err(`  [${violation.kind}] ${violation.detail}`);
        err('GATE EXIT=1');
        return 1;
    }
    out('GATE EXIT=0');
    return 0;
}

const invokedDirectly = process.argv[1]?.endsWith('attribution.ts');
if (invokedDirectly) {
    process.exit(main(process.argv.slice(2)));
}
