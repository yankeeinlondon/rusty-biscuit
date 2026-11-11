//! The D1 feasibility scenarios, shared by the local Workers-pool test and
//! the deployed run in `cli/prove-d1.ts`. Each scenario records a named check;
//! the deployed checkpoint passes only when every check passes on real D1.

export interface SpikeResponse {
  status: number;
  body: unknown;
}

export type SpikePost = (path: string, body: unknown) => Promise<SpikeResponse>;

export interface SpikeCheck {
  name: string;
  ok: boolean;
  detail: string;
}

export interface ScenarioOptions {
  /** Concurrent conflicting pairs; the plan requires 20 on deployed D1. */
  pairs: number;
  /** Observations in the CPU sample (0 skips it); the declared collection maximum is 256. */
  cpuObservations: number;
  /** Approximate request size for the CPU sample; the declared bound is 1 MiB. */
  cpuRequestBytes: number;
}

export const DEPLOYED_OPTIONS: ScenarioOptions = {
  pairs: 20,
  cpuObservations: 256,
  cpuRequestBytes: 1024 * 1024 - 4096,
};

interface SpikeObservation {
  observation_id: string;
  payload_json: string;
}

interface StateBody {
  revision: number;
  observations: number;
  collections: number;
  members: number;
  events: number;
  ids: string[];
  collection_ids: string[];
}

const HEAD_A = "a".repeat(40);
const HEAD_B = "b".repeat(40);
const TREE = "c".repeat(40);

async function sha256(text: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** A minimal observation carrying the fields the generated columns read. */
async function observation(label: string, padding = 0): Promise<SpikeObservation> {
  const body = {
    cell: {
      package: `spike-${label}`,
      gate: "L1",
      outcome: "pass",
      gate_input_identity: "0123456789abcdef",
      report: "junit.xml",
    },
    padding: "x".repeat(padding),
    reports: [{ path: "junit.xml", sha256: "0".repeat(64), bytes: 1, store: "artifact" }],
  };
  const id = await sha256(JSON.stringify(body));
  return { observation_id: id, payload_json: JSON.stringify({ observation_id: id, ...body }) };
}

function collection(
  repository: string,
  head: string,
  members: SpikeObservation[],
  collectionId: string,
): Record<string, unknown> {
  const sorted = [...members].sort((a, b) => (a.observation_id < b.observation_id ? -1 : 1));
  return {
    repository_id: repository,
    collection_id: collectionId,
    environment: "ubuntu-latest",
    head,
    tree: TREE,
    source_version: 3,
    principal: "spike-producer",
    observations: sorted,
  };
}

export async function runScenarios(post: SpikePost, options: ScenarioOptions): Promise<SpikeCheck[]> {
  const checks: SpikeCheck[] = [];
  const check = (name: string, ok: boolean, detail: unknown) =>
    checks.push({ name, ok, detail: typeof detail === "string" ? detail : JSON.stringify(detail) });
  const repository = `spike-${crypto.randomUUID()}`;
  const state = async () => (await post("/state", { repository_id: repository })).body as StateBody;

  const registered = await post("/repo", { repository_id: repository, slug: repository });
  check("register repository", registered.status === 200, registered);

  // Idempotent delivery.
  const a = await observation("a");
  const b = await observation("b");
  const first = await post("/ingest", collection(repository, HEAD_A, [a, b], "collection-1"));
  const afterFirst = await state();
  check(
    "first delivery accepts both",
    first.status === 200 && (first.body as { accepted: string[] }).accepted.length === 2,
    first,
  );
  const replay = await post("/ingest", collection(repository, HEAD_A, [a, b], "collection-1"));
  const afterReplay = await state();
  check(
    "replay is already-present without revision or event change",
    replay.status === 200 &&
      (replay.body as { already_present: string[] }).already_present.length === 2 &&
      afterReplay.revision === afterFirst.revision &&
      afterReplay.events === afterFirst.events,
    { replay: replay.body, before: afterFirst.revision, after: afterReplay.revision },
  );

  // A conflicting context rolls back its fresh sibling, manifest, and membership.
  const c = await observation("c");
  const conflict = await post("/ingest", collection(repository, HEAD_B, [a, c], "collection-2"));
  const afterConflict = await state();
  check(
    "conflicting context is rejected with identity_conflict",
    conflict.status === 409 && (conflict.body as { error: string }).error === "identity_conflict",
    conflict,
  );
  check(
    "rejected batch leaves no sibling, manifest, member, event, or revision",
    !afterConflict.ids.includes(c.observation_id) &&
      !afterConflict.collection_ids.includes("collection-2") &&
      afterConflict.members === afterReplay.members &&
      afterConflict.events === afterReplay.events &&
      afterConflict.revision === afterReplay.revision,
    afterConflict,
  );

  // A failure in a later statement of the batch rolls back earlier statements.
  const d = await observation("d");
  const late = await post("/late-failure", collection(repository, HEAD_A, [d], "collection-late"));
  const afterLate = await state();
  check(
    "late statement failure rolls back the whole batch",
    late.status === 200 &&
      (late.body as { rejected?: boolean }).rejected === true &&
      !afterLate.ids.includes(d.observation_id) &&
      afterLate.revision === afterConflict.revision,
    { late: late.body, revision: afterLate.revision },
  );

  const guards = await post("/guards", collection(repository, HEAD_A, [await observation("guard")], "unused"));
  const guardBody = guards.body as Record<string, string>;
  check(
    "generated-column CHECK and immutability trigger refuse",
    guards.status === 200 &&
      String(guardBody.generated_gate_check).startsWith("rejected") &&
      String(guardBody.immutable_update).startsWith("rejected"),
    guardBody,
  );

  // Concurrent pairs: one shared observation under two contexts plus a fresh sibling each.
  const beforePairs = await state();
  const pairs = await Promise.all(
    Array.from({ length: options.pairs }, async (_, index) => {
      const shared = await observation(`shared-${index}`);
      const left = await observation(`left-${index}`);
      const right = await observation(`right-${index}`);
      const [l, r] = await Promise.all([
        post("/ingest", collection(repository, HEAD_A, [shared, left], `pair-${index}-left`)),
        post("/ingest", collection(repository, HEAD_B, [shared, right], `pair-${index}-right`)),
      ]);
      return { index, left, right, l, r };
    }),
  );
  const afterPairs = await state();
  let winners = 0;
  const pairProblems: string[] = [];
  for (const { index, left, right, l, r } of pairs) {
    const statuses = [l.status, r.status].sort().join(",");
    if (statuses !== "200,409") {
      pairProblems.push(`pair ${index}: statuses ${statuses}`);
      continue;
    }
    winners += 1;
    const loser = l.status === 409 ? left : right;
    const loserCollection = l.status === 409 ? `pair-${index}-left` : `pair-${index}-right`;
    const winner = l.status === 409 ? right : left;
    if (afterPairs.ids.includes(loser.observation_id)) pairProblems.push(`pair ${index}: loser sibling survived`);
    if (afterPairs.collection_ids.includes(loserCollection)) pairProblems.push(`pair ${index}: loser manifest survived`);
    if (!afterPairs.ids.includes(winner.observation_id)) pairProblems.push(`pair ${index}: winner sibling missing`);
  }
  check(
    `${options.pairs} concurrent conflicting pairs each commit exactly one side`,
    pairProblems.length === 0 && winners === options.pairs,
    pairProblems.length === 0 ? `${winners} winners` : pairProblems,
  );
  check(
    "concurrent pairs advance revision, events, and members by winners only",
    afterPairs.revision === beforePairs.revision + 2 * winners &&
      afterPairs.events === beforePairs.events + 2 * winners &&
      afterPairs.members === beforePairs.members + 2 * winners,
    { before: beforePairs, after: { ...afterPairs, ids: afterPairs.ids.length } },
  );

  const catalog = await post("/catalog", { repository_id: repository, head: HEAD_A, tree: TREE });
  const rows = (catalog.body as { rows?: Record<string, number> }).rows ?? {};
  check(
    "query catalog prepares and runs with bound parameters",
    catalog.status === 200 &&
      (rows.q4_history ?? 0) >= 1 &&
      (rows.q7_usable_locations ?? 0) >= 1 &&
      rows.q8_needing_probe === 0 &&
      rows.q9_current_evaluation_changes === 1 &&
      rows.q9_stale_evaluation_changes === 0 &&
      rows.q10_quick_answer === 1 &&
      (rows.q13_export ?? 0) >= 1,
    catalog.body,
  );

  if (options.cpuObservations > 0) {
    checks.push(await cpuSample(post, options.cpuObservations, options.cpuRequestBytes));
  }
  return checks;
}

/** One CPU sample: `observations` members padded to about `requestBytes` in total. */
export async function cpuSample(post: SpikePost, observations: number, requestBytes: number): Promise<SpikeCheck> {
  const members: unknown[] = [];
  const padding = Math.max(0, Math.floor(requestBytes / Math.max(1, observations)) - 400);
  for (let index = 0; index < observations; index += 1) {
    members.push(JSON.parse((await observation(`cpu-${index}`, padding)).payload_json));
  }
  const body = JSON.stringify({ schema_version: 3, observations: members });
  const response = await post("/cpu-sample", body);
  return {
    name: `CPU sample of ${observations} observations (${body.length} bytes) completes`,
    ok: response.status === 200 && (response.body as { observations?: number }).observations === observations,
    detail: JSON.stringify(response),
  };
}
