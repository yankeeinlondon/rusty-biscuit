#!/usr/bin/env -S npx tsx
/**
 * Compute this fix's Outcome-table measures from `claudine-cli` L1 JUnit
 * artifacts.
 *
 * The spec makes CI JUnit the measurement of record, so the follow-up section
 * of `inventory.md` must be produced from artifacts rather than from a local
 * run. This reads one directory per environment, each holding the artifact as
 * downloaded (`<env>/L1/claudine-cli.xml`).
 *
 * Usage:
 *   gh run download <run-id> -R yankeeinlondon/rusty-biscuit \
 *     -n junit-claudine-cli-L1-<env> -D <dir>/<env>      # x4 environments
 *   npx tsx junit-metrics.ts <dir> [--label "run <id>"] [--markdown]
 */
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join } from "node:path";

/** The nineteen binaries named in Required behavior 2, first list. */
const INVENTORIED_19 = [
  "compose_header_first",
  "inline_compose_hash",
  "mcp_cli",
  "prompt_reporting",
  "sequence_perf",
  "sequence_schema",
  "shipped_prompt_contract",
  "wrap_compose_agent",
  "wrap_compose_exec",
  "wrap_compose_preflight",
  "wrap_compose_validation",
  "wrap_direct_argv",
  "wrap_inline_compose",
  "wrap_inline_compose_interactive",
  "wrap_opencode",
  "wrap_opencode_models",
  "wrap_perf",
  "wrap_structured_stream",
  "wrap_watchdog_timeout",
];

/** The ten other spawn-shaped binaries, second list. */
const OTHER_10 = [
  "wrap_basics",
  "wrap_provider_flags",
  "command_routing",
  "argv_normalization",
  "hooks_cli",
  "contextual_errors",
  "inline_compose_sequence_mismatch",
  "wrap_antigravity_exit_signal",
  "handle_repo_config",
  "characterization_error_routes",
];

/** The two ambient-context subjects: migrated, but outside the 29. */
const AMBIENT_SUBJECTS = ["ctx_launch_anchor", "propagated_context_fixtures"];

/**
 * Timeout-shaped tests and the semantic floor each one pays, in seconds.
 *
 * `budget` is the configured wall-clock or stream-silence budget the test
 * waits out (for `post_fanout`, the fixture's built-in silence, since the
 * contract there is that the 15 s budget must *not* fire); `tick` is the
 * watchdog interval it may wait one of. The spec's bound is
 * `budget + tick + 1 s` on the native runners and `+ 2 s` on WSL2.
 */
const TIMEOUT_TESTS: Record<string, { budget: number; tick: number }> = {
  watchdog_subagent_hang_terminates_and_names_stuck_ids: {
    budget: 1,
    tick: 0.2,
  },
  watchdog_stream_idle_timeout_after_tool_call_hang: { budget: 1, tick: 0.2 },
  watchdog_wall_clock_timeout_terminates_active_stream: {
    budget: 1,
    tick: 0.2,
  },
  watchdog_opencode_post_fanout_silence_does_not_kill_prematurely: {
    budget: 1,
    tick: 0.2,
  },
  compose_non_harness_respects_cli_timeout: { budget: 1, tick: 0.2 },
  inline_compose_non_harness_respects_cli_step_timeout: {
    budget: 1,
    tick: 0.2,
  },
  sequence_per_step_step_timeout_override: { budget: 0.5, tick: 0.1 },
  opencode_stderr_rate_limit_before_stdout_forces_early_termination: {
    budget: 0,
    tick: 0.2,
  },
  opencode_stderr_stream_error_cap_1_17_8_forces_early_termination: {
    budget: 0,
    tick: 0.2,
  },
};

/** The four CI environments, in the order the spec's tables list them. */
const CI_ENVIRONMENTS = [
  "ubuntu-latest",
  "macos-latest",
  "windows-latest",
  "wsl2-ubuntu",
];

/**
 * Every subdirectory of `dir` holding an artifact, CI environments first.
 *
 * Discovered rather than fixed so the same script reads a local
 * `NEXTEST_PROFILE=ci` report — which is attribution only, never a target.
 */
function environments(dir: string): string[] {
  const present = readdirSync(dir, { withFileTypes: true })
    .filter((e) => e.isDirectory())
    .map((e) => e.name)
    .filter((name) => existsSync(join(dir, name, "L1", "claudine-cli.xml")));
  const known = CI_ENVIRONMENTS.filter((env) => present.includes(env));
  const extra = present.filter((env) => !CI_ENVIRONMENTS.includes(env)).sort();
  const missing = CI_ENVIRONMENTS.filter((env) => !present.includes(env));
  for (const env of missing) console.error(`warning: no artifact for ${env}`);
  return [...known, ...extra];
}

type Case = { binary: string; name: string; seconds: number };

function parse(xml: string): Case[] {
  const cases: Case[] = [];
  const re =
    /<testcase\s+name="([^"]*)"\s+classname="([^"]*)"[^>]*?\stime="([^"]*)"/g;
  for (const match of xml.matchAll(re)) {
    const [, name, classname, time] = match;
    cases.push({
      binary: classname.replace(/^claudine-cli::/, ""),
      name,
      seconds: Number(time),
    });
  }
  return cases;
}

const sum = (cases: Case[]) => cases.reduce((a, c) => a + c.seconds, 0);
const fixed = (n: number) => n.toFixed(1);

function report(dir: string, label: string, markdown: boolean) {
  const rows: string[] = [];
  for (const env of environments(dir)) {
    const path = join(dir, env, "L1", "claudine-cli.xml");
    const cases = parse(readFileSync(path, "utf8"));
    const inventoried = cases.filter((c) => INVENTORIED_19.includes(c.binary));
    const other = cases.filter((c) => OTHER_10.includes(c.binary));
    const migrated = [...inventoried, ...other];
    const nonTimeout = migrated.filter((c) => !(c.name in TIMEOUT_TESTS));

    const overFive = migrated.filter((c) => c.seconds >= 5);
    const overTwo = nonTimeout.filter((c) => c.seconds >= 2);
    const slowest = nonTimeout.reduce(
      (a, c) => (c.seconds > a.seconds ? c : a),
      { binary: "-", name: "-", seconds: 0 },
    );

    rows.push(
      `| \`${env}\` | ${overFive.length} | ${overTwo.length} | ${fixed(
        slowest.seconds,
      )} s (\`${slowest.name}\`) | ${fixed(sum(inventoried))} s (${
        inventoried.length
      } tests) | ${fixed(sum(other))} s (${other.length} tests) |`,
    );

    if (!markdown) {
      console.log(`\n== ${env} (${label})`);
      console.log(`  migrated tests present:      ${migrated.length}`);
      console.log(`  >= 5 s (any migrated):       ${overFive.length}`);
      for (const c of overFive.sort((a, b) => b.seconds - a.seconds)) {
        console.log(`      ${fixed(c.seconds)} s  ${c.binary}::${c.name}`);
      }
      console.log(`  >= 2 s (non-timeout):        ${overTwo.length}`);
      console.log(
        `  slowest non-timeout:         ${fixed(slowest.seconds)} s  ${
          slowest.binary
        }::${slowest.name}`,
      );
      console.log(
        `  serial sum, 19 inventoried:  ${fixed(sum(inventoried))} s (${
          inventoried.length
        } tests)`,
      );
      console.log(
        `  serial sum, 10 other:        ${fixed(sum(other))} s (${
          other.length
        } tests)`,
      );
      const ambient = cases.filter((c) => AMBIENT_SUBJECTS.includes(c.binary));
      console.log(
        `  ambient-context subjects:    ${fixed(sum(ambient))} s (${
          ambient.length
        } tests)`,
      );

      const allowance = env === "wsl2-ubuntu" ? 2 : 1;
      console.log(`  timeout-shaped tests (bound = budget + tick + ${allowance} s):`);
      for (const [name, floor] of Object.entries(TIMEOUT_TESTS)) {
        const found = cases.find((c) => c.name === name);
        if (!found) {
          console.log(`      ABSENT  ${name}`);
          continue;
        }
        const bound = floor.budget + floor.tick + allowance;
        const verdict = found.seconds <= bound ? "ok  " : "MISS";
        console.log(
          `      ${verdict} ${fixed(found.seconds)} s / ${fixed(
            bound,
          )} s  ${name}`,
        );
      }
    }
  }

  console.log(`\n### ${label}\n`);
  console.log(
    "| Environment | ≥ 5 s | ≥ 2 s (non-timeout) | Slowest non-timeout | 19 inventoried, serial | 10 other, serial |",
  );
  console.log("|---|---:|---:|---|---:|---:|");
  for (const row of rows) console.log(row);
}

const args = process.argv.slice(2);
const dir = args.find((a) => !a.startsWith("--"));
if (!dir) {
  console.error("usage: junit-metrics.ts <dir> [--label <text>] [--markdown]");
  process.exit(2);
}
const labelIndex = args.indexOf("--label");
report(
  dir,
  labelIndex >= 0 ? args[labelIndex + 1] : dir,
  args.includes("--markdown"),
);
