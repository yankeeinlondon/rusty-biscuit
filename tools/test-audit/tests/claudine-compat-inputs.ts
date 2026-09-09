/**
 * The frozen inputs every Claudine compatibility replay reads.
 *
 * One module so the set is enumerable: `claudine-compat-inputs.test.ts` guards
 * that no `*-claudine-compat.test.ts` reaches past it to a live consumer
 * document. `fixtures/claudine-compat/README.md` explains why that matters.
 */
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

export const REPO_ROOT = join(here, "..", "..", "..");
export const FIX_DIR = join(REPO_ROOT, "claudine/fixes/2026-09-07-faster-claudine-tests");

/** The revision every input below was produced at. */
export const ERA_REVISION = "9fc5151a0";

/** Nextest listings, preserved verbatim when the fix retook its captures. */
export const ERA_ENUMERATION_DIR = join(FIX_DIR, "enumeration", ERA_REVISION);

/** Nextest console logs from the era's gate runs. */
export const ERA_LOG_PATHS = [
  join(FIX_DIR, "baseline/local-gates/just-test.log"),
  join(FIX_DIR, "enumeration/recipes/just-test-rendezvous.log"),
  join(FIX_DIR, "baseline/local-gates/just-test-l2.log"),
];

const FIXTURES = join(here, "..", "fixtures", "claudine-compat");

/** The era's classifier, snapshotted so a later rename cannot reach the replay. */
export const ERA_FAMILIES_PATH = join(FIXTURES, "families.json");
export const ERA_EXPECTATIONS_PATH = join(FIXTURES, "expectations.json");

export interface ClaudineCompatExpectations {
  revision: string;
  renames: Record<string, string>;
  attribution: {
    count: number;
    summed: number;
    elapsed: number;
    topFamily: string;
    familyCounts: Record<string, number>;
  };
  universe: { total: number; byPackage: Record<string, number> };
  sourceScan: { byPackage: Record<string, number>; maxFilesWithParseErrors: number };
}

export function expectations(): ClaudineCompatExpectations {
  return JSON.parse(readFileSync(ERA_EXPECTATIONS_PATH, "utf8")) as ClaudineCompatExpectations;
}

/**
 * A checkout of `ERA_REVISION` for the source-scan replays, which need the tree
 * as it stood — the live one has moved past the listings.
 *
 * The one deliberate absolute path in the compatibility inputs, and the reason
 * it is here rather than in a replay: it is an *optional* external artifact
 * nobody is required to materialize, so the replays that use it skip when it is
 * absent. Override with `TEST_AUDIT_CLAUDINE_BASELINE`; create it with
 * `git worktree add <dir> <ERA_REVISION>`.
 */
export const ERA_WORKTREE = process.env["TEST_AUDIT_CLAUDINE_BASELINE"] ?? "/tmp/rb-baseline-9fc5151a0";

export function haveEraWorktree(): boolean {
  return existsSync(join(ERA_WORKTREE, "claudine", "lib", "Cargo.toml"));
}

/** Package source roots, resolved under `root` so a preserved worktree can stand in. */
export function claudinePackageRoots(root: string): { package: string; roots: string[] }[] {
  return [
    ["claudine-catalog-types", "claudine/catalog-types"],
    ["claudine", "claudine/lib"],
    ["claudine-contract", "claudine/contract"],
    ["claudine-cli", "claudine/cli"],
    ["claudine-gen", "claudine/gen"],
    ["rendezvous-core", "claudine/rendezvous/core"],
    ["rendezvous-daemon", "claudine/rendezvous/daemon"],
    ["rendezvous-client", "claudine/rendezvous/client"],
  ].map(([pkg, path]) => ({ package: pkg!, roots: [join(root, path!)] }));
}
