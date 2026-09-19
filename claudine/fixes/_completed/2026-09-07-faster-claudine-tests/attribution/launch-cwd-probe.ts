/**
 * Launch-CWD probe (fix 2026-09-07-faster-claudine-tests, Phase 3).
 *
 * `context_command.rs` runs the real binary with `current_dir(repo_root())`
 * in 22 of its 27 cases. This probe measures what that costs, separately from
 * anything the test itself does: the same `claudine context --values`
 * invocation from the monorepo root and from a directory outside any
 * repository, at several concurrency levels.
 *
 * It is an evidence generator, not a gate — its only correctness claim is that
 * every child it timed exited 0, and it exits 1 when one did not, so a table
 * of durations can never come from failed runs.
 *
 *   npx tsx attribution/launch-cwd-probe.ts --bin ../../../target/debug/claudine \
 *     --repo-root ../../.. --out attribution/launch-cwd.tsv
 */

import { spawn } from 'node:child_process';
import { mkdtempSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

interface Sample {
    label: string;
    concurrency: number;
    elapsedMs: number;
    perProcessMeanMs: number;
    perProcessMaxMs: number;
}

function runOnce(bin: string, cwd: string): Promise<number> {
    const started = performance.now();
    return new Promise((resolveDuration, reject) => {
        const child = spawn(bin, ['context', '--values'], {
            cwd,
            stdio: 'ignore',
            env: { ...process.env, NO_COLOR: '1' },
        });
        child.on('error', reject);
        child.on('close', (code) => {
            if (code !== 0) {
                reject(new Error(`${bin} exited ${code} in ${cwd}`));
                return;
            }
            resolveDuration(performance.now() - started);
        });
    });
}

async function burst(bin: string, cwd: string, concurrency: number, label: string): Promise<Sample> {
    const started = performance.now();
    const durations = await Promise.all(Array.from({ length: concurrency }, () => runOnce(bin, cwd)));
    return {
        label,
        concurrency,
        elapsedMs: performance.now() - started,
        perProcessMeanMs: durations.reduce((a, b) => a + b, 0) / durations.length,
        perProcessMaxMs: Math.max(...durations),
    };
}

export async function main(argv: string[]): Promise<number> {
    const flag = (name: string): string | undefined => {
        const index = argv.indexOf(`--${name}`);
        return index === -1 ? undefined : argv[index + 1];
    };
    const bin = resolve(flag('bin') ?? '');
    const repoRoot = resolve(flag('repo-root') ?? '');
    const out = flag('out');
    if (!existsSync(bin) || !existsSync(repoRoot) || out === undefined) {
        console.error('usage: launch-cwd-probe.ts --bin <claudine> --repo-root <dir> --out <file.tsv>');
        return 2;
    }
    const outside = mkdtempSync(join(tmpdir(), 'launch-cwd-'));
    const samples: Sample[] = [];
    try {
        // One untimed burst per location warms the page cache so the first
        // timed sample is not measuring the loader.
        await burst(bin, repoRoot, 4, 'warmup');
        await burst(bin, outside, 4, 'warmup');
        for (const [label, cwd] of [
            ['repo-root', repoRoot],
            ['outside-repo', outside],
        ] as const) {
            for (const concurrency of [1, 8, 16, 27]) {
                samples.push(await burst(bin, cwd, concurrency, label));
            }
        }
    } catch (error) {
        console.error(`probe aborted: ${(error as Error).message}`);
        return 1;
    }

    const rows = ['cwd\tconcurrency\telapsed_ms\tper_process_mean_ms\tper_process_max_ms'];
    for (const sample of samples) {
        rows.push(
            `${sample.label}\t${sample.concurrency}\t${sample.elapsedMs.toFixed(0)}\t` +
                `${sample.perProcessMeanMs.toFixed(0)}\t${sample.perProcessMaxMs.toFixed(0)}`,
        );
    }
    writeFileSync(out, `${rows.join('\n')}\n`);
    console.log(rows.join('\n'));
    return 0;
}

const invokedDirectly = process.argv[1]?.endsWith('launch-cwd-probe.ts');
if (invokedDirectly) {
    // `void` plus an explicit exit rather than top-level await: tsx transforms
    // this file to CommonJS, where top-level await is a transform error.
    void main(process.argv.slice(2)).then((code) => process.exit(code));
}
