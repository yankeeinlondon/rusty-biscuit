import { parseArgv, requireString } from "./args.ts";
import type { Command, CommandIo } from "./command.ts";
import { activeEnvironments, loadConfig } from "./config.ts";
import { UsageError } from "./errors.ts";

const USAGE = `usage: test-audit config validate --config <audit.config.json>
       test-audit config show --config <audit.config.json>`;

function show(configPath: string, io: CommandIo): number {
  const config = loadConfig(configPath);
  io.out(`area: ${config.area}`);
  io.out(`repo root: ${config.repoRoot}`);
  io.out(`evidence: ${config.evidenceRoot}`);
  io.out(`packages (${config.packages.length}): ${config.packages.map((p) => p.name).join(", ")}`);
  io.out(`selections (${config.selections.length}):`);
  for (const s of config.selections) {
    const features = s.features.length > 0 ? `--features ${s.features.join(",")}` : "(no features)";
    io.out(`  ${s.label}: ${s.packages.join(", ")} ${features} → routes ${s.routes.join(", ")}`);
  }
  io.out(`cohorts (${config.cohorts.length}):`);
  for (const c of config.cohorts) {
    io.out(`  ${c.id} [${c.population}] ${c.recipe} — ${c.description}`);
  }
  io.out(`routes (${config.routes.length}):`);
  for (const r of config.routes) {
    io.out(`  ${r.id} [${r.kind}${r.executes ? "" : ", does not execute tests"}] ${r.command}${r.ci ? ` (CI: ${r.ci})` : ""}`);
  }
  const active = activeEnvironments(config);
  io.out(`environments (${config.environments.length}, ${active.length} active):`);
  for (const e of config.environments) {
    const cells = e.cells.map((c) => `${c.tier}/${c.package}`).join(", ");
    io.out(`  ${e.name} [${e.kind} ${e.os}${e.pending ? ", pending" : ""}] ${cells || "(no cells)"}`);
  }
  if (config.counters) {
    io.out(`counter signals (${config.counters.signals.length}):`);
    for (const s of config.counters.signals) io.out(`  ${s.id} [${s.coverage}] ${s.source}`);
  }
  return 0;
}

export const configCommand: Command = {
  name: "config",
  summary: "validate or display an area configuration",
  usage: USAGE,
  run(argv, io) {
    const parsed = parseArgv(argv);
    const sub = parsed.positional[0];
    if (sub === "validate") {
      const path = requireString(parsed, "config", USAGE);
      const config = loadConfig(path);
      io.out(`config OK: ${config.area} (${config.packages.length} packages, ${config.selections.length} selections, ${config.environments.length} environments)`);
      return 0;
    }
    if (sub === "show") {
      return show(requireString(parsed, "config", USAGE), io);
    }
    throw new UsageError(USAGE);
  },
};
