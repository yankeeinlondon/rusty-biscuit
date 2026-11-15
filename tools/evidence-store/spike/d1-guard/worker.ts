//! Disposable Worker for the Phase 1 D1 feasibility checkpoint.
//!
//! It exercises exactly what local SQLite cannot settle: D1 prepared bindings,
//! generated columns, triggers, transactional `batch()` rollback under
//! concurrent requests, and Worker CPU at the declared payload bound. It is
//! deployed by `cli/prove-d1.ts` to a throwaway database and deleted after the
//! run; it is not the evidence-store service.

export interface SpikeEnv {
  SPIKE_DB: D1Database;
  SPIKE_TOKEN: string;
}

interface IngestRequest {
  repository_id: string;
  collection_id: string;
  environment: string;
  head: string;
  tree: string;
  source_version: number;
  principal: string;
  observations: { observation_id: string; payload_json: string }[];
}

const INSERT_OBSERVATIONS = `
INSERT INTO observations(
    repository_id, observation_id, environment, head, tree, source_version,
    payload_json, accepted_principal, accepted_request_id, received_at_ms
)
SELECT ?1, json_extract(value, '$.observation_id'), ?2, ?3, ?4, ?5,
       json_extract(value, '$.payload_json'), ?6, ?7, ?8
FROM json_each(?9)
WHERE true
ON CONFLICT(repository_id, observation_id) DO NOTHING`;

const INSERT_COLLECTION = `
INSERT INTO collections(
    repository_id, collection_id, environment, head, tree, source_version,
    observation_count, accepted_principal, received_at_ms
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
ON CONFLICT(repository_id, collection_id) DO NOTHING`;

const INSERT_MEMBERS = `
INSERT INTO collection_members(repository_id, collection_id, position, observation_id)
SELECT ?1, ?2, CAST(key AS INTEGER), value FROM json_each(?3)
WHERE true
ON CONFLICT(repository_id, collection_id, position) DO NOTHING`;

const SELECT_OUTCOME = `
SELECT observation_id, accepted_request_id = ?2 AS accepted
FROM observations
WHERE repository_id = ?1
  AND observation_id IN (SELECT value FROM json_each(?3))
ORDER BY observation_id`;

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

/** The D1 error message names the RAISE text; anything else is unexpected. */
function conflictOrThrow(error: unknown): Response {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes("identity_conflict")) {
    return json({ error: "identity_conflict" }, 409);
  }
  return json({ error: "batch_failed", message }, 500);
}

async function ingest(db: D1Database, request: IngestRequest): Promise<Response> {
  const requestId = crypto.randomUUID();
  const now = Date.now();
  const ids = request.observations.map((o) => o.observation_id);
  const idsJson = JSON.stringify(ids);
  try {
    const results = await db.batch([
      db.prepare(INSERT_OBSERVATIONS).bind(
        request.repository_id,
        request.environment,
        request.head,
        request.tree,
        request.source_version,
        request.principal,
        requestId,
        now,
        JSON.stringify(request.observations),
      ),
      db.prepare(INSERT_COLLECTION).bind(
        request.repository_id,
        request.collection_id,
        request.environment,
        request.head,
        request.tree,
        request.source_version,
        request.observations.length,
        request.principal,
        now,
      ),
      db.prepare(INSERT_MEMBERS).bind(request.repository_id, request.collection_id, idsJson),
      db.prepare(SELECT_OUTCOME).bind(request.repository_id, requestId, idsJson),
      db
        .prepare("SELECT evidence_revision FROM repositories WHERE repository_id = ?1")
        .bind(request.repository_id),
    ]);
    const outcome = (results[3]?.results ?? []) as { observation_id: string; accepted: number }[];
    const revision = (results[4]?.results?.[0] as { evidence_revision: number } | undefined)
      ?.evidence_revision;
    return json({
      accepted: outcome.filter((row) => row.accepted === 1).map((row) => row.observation_id),
      already_present: outcome.filter((row) => row.accepted !== 1).map((row) => row.observation_id),
      evidence_revision: revision,
    });
  } catch (error) {
    return conflictOrThrow(error);
  }
}

async function state(db: D1Database, repositoryId: string): Promise<Response> {
  const row = await db
    .prepare(
      `SELECT r.evidence_revision AS revision,
              (SELECT COUNT(*) FROM observations WHERE repository_id = r.repository_id) AS observations,
              (SELECT COUNT(*) FROM collections WHERE repository_id = r.repository_id) AS collections,
              (SELECT COUNT(*) FROM collection_members WHERE repository_id = r.repository_id) AS members,
              (SELECT COUNT(*) FROM observation_events WHERE repository_id = r.repository_id) AS events,
              (SELECT json_group_array(observation_id) FROM observations WHERE repository_id = r.repository_id) AS ids,
              (SELECT json_group_array(collection_id) FROM collections WHERE repository_id = r.repository_id) AS collection_ids
       FROM repositories r WHERE r.repository_id = ?1`,
    )
    .bind(repositoryId)
    .first<Record<string, unknown>>();
  if (row === null) return json({ error: "not_found" }, 404);
  return json({
    ...row,
    ids: JSON.parse(String(row.ids)),
    collection_ids: JSON.parse(String(row.collection_ids)),
  });
}

/**
 * A batch whose final statement violates a foreign key after earlier
 * statements succeeded. Rollback means none of those earlier rows survive.
 */
async function lateFailure(db: D1Database, request: IngestRequest): Promise<Response> {
  const requestId = crypto.randomUUID();
  const now = Date.now();
  try {
    await db.batch([
      db.prepare(INSERT_OBSERVATIONS).bind(
        request.repository_id,
        request.environment,
        request.head,
        request.tree,
        request.source_version,
        request.principal,
        requestId,
        now,
        JSON.stringify(request.observations),
      ),
      db
        .prepare(
          "INSERT INTO collection_members(repository_id, collection_id, position, observation_id) VALUES (?1, ?2, 0, ?3)",
        )
        .bind(request.repository_id, "no-such-collection", "no-such-observation"),
    ]);
    return json({ error: "late_failure_committed" }, 500);
  } catch (error) {
    return json({ rejected: true, message: error instanceof Error ? error.message : String(error) });
  }
}

/** Generated-column CHECKs and immutability triggers, each expected to refuse. */
async function guards(db: D1Database, request: IngestRequest): Promise<Response> {
  const outcomes: Record<string, string> = {};
  const attempt = async (name: string, statement: D1PreparedStatement) => {
    try {
      await statement.run();
      outcomes[name] = "accepted";
    } catch (error) {
      outcomes[name] = `rejected: ${error instanceof Error ? error.message : String(error)}`;
    }
  };
  const first = request.observations[0];
  if (first === undefined) return json({ error: "need one observation" }, 400);
  const badGate = JSON.parse(first.payload_json) as { cell: { gate: string } };
  badGate.cell.gate = "nightly";
  await attempt(
    "generated_gate_check",
    db
      .prepare(
        `INSERT INTO observations(repository_id, observation_id, environment, head, tree, source_version,
           payload_json, accepted_principal, accepted_request_id, received_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, 3, ?6, 'spike', 'guard', 0)`,
      )
      .bind(
        request.repository_id,
        first.observation_id,
        request.environment,
        request.head,
        request.tree,
        JSON.stringify(badGate),
      ),
  );
  await attempt(
    "immutable_update",
    db
      .prepare("UPDATE observations SET head = head WHERE repository_id = ?1")
      .bind(request.repository_id),
  );
  return json(outcomes);
}

/**
 * Runs every statement of the d1-model query catalog once with bound
 * parameters, so a statement D1 cannot prepare fails the checkpoint.
 */
async function catalog(db: D1Database, repositoryId: string, head: string, tree: string): Promise<Response> {
  const now = Date.now();
  const heads = JSON.stringify([head]);
  const run = async (sql: string, ...values: unknown[]) =>
    (await db.prepare(sql).bind(...values).all()).results.length;
  const snapshot = await db
    .prepare(
      `SELECT r.evidence_revision,
              COALESCE((SELECT MAX(o.seq) FROM observations o WHERE o.repository_id = r.repository_id), 0) AS observation_watermark,
              COALESCE((SELECT MAX(l.seq) FROM report_locations l WHERE l.repository_id = r.repository_id), 0) AS location_watermark
       FROM repositories r WHERE r.repository_id = ?1`,
    )
    .bind(repositoryId)
    .first<{ evidence_revision: number; observation_watermark: number; location_watermark: number }>();
  if (snapshot === null) return json({ error: "not_found" }, 404);
  const sample = await db
    .prepare("SELECT observation_id, package, gate FROM observations WHERE repository_id = ?1 ORDER BY seq LIMIT 1")
    .bind(repositoryId)
    .first<{ observation_id: string; package: string; gate: string }>();
  if (sample === null) return json({ error: "no observations to query" }, 400);
  const rows: Record<string, number> = {};
  rows.q4_history = await run(
    `SELECT o.* FROM observations o
     WHERE o.repository_id = ?1 AND o.package = ?2 AND o.gate = ?3 AND o.seq <= ?4
       AND o.head IN (SELECT value FROM json_each(?5)) ORDER BY o.seq`,
    repositoryId, sample.package, sample.gate, snapshot.observation_watermark, heads,
  );
  rows.q5_retry_successors = await run(
    `SELECT * FROM observations WHERE repository_id = ?1 AND retry_of = ?2 AND seq <= ?3 ORDER BY seq`,
    repositoryId, sample.observation_id, snapshot.observation_watermark,
  );
  await db
    .prepare(
      `INSERT INTO report_locations(repository_id, observation_id, report_path, location_key, location_kind,
         locator_json, state, verification_method, checked_at_ms, valid_until_ms, checked_by)
       VALUES (?1, ?2, 'junit.xml', 'object:spike', 'object', '{"key":"spike"}', 'verified', 'bytes', ?3, ?4, 'spike')`,
    )
    .bind(repositoryId, sample.observation_id, now, now + 86_400_000)
    .run();
  const locationWatermark = snapshot.location_watermark + 1_000_000;
  rows.q7_usable_locations = await run(
    `WITH latest AS (
       SELECT *, ROW_NUMBER() OVER (PARTITION BY repository_id, observation_id, report_path, location_key ORDER BY seq DESC) AS position
       FROM report_locations WHERE repository_id = ?1 AND observation_id = ?2 AND seq <= ?3)
     SELECT * FROM latest WHERE position = 1 AND state = 'verified' AND valid_until_ms > ?4
     ORDER BY report_path, location_key`,
    repositoryId, sample.observation_id, locationWatermark, now,
  );
  rows.q8_needing_probe = await run(
    `WITH latest AS (
       SELECT *, ROW_NUMBER() OVER (PARTITION BY repository_id, observation_id, report_path, location_key ORDER BY seq DESC) AS position
       FROM report_locations WHERE repository_id = ?1 AND seq <= ?2
     ), usable AS (
       SELECT observation_id, report_path FROM latest
       WHERE position = 1 AND state = 'verified' AND valid_until_ms > ?3)
     SELECT o.observation_id, json_extract(j.value, '$.path') AS report_path, j.value AS original_reference_json
     FROM observations o, json_each(o.payload_json, '$.reports') j
     WHERE o.repository_id = ?1 AND o.seq <= ?4
       AND o.observation_id IN (SELECT value FROM json_each(?5))
       AND NOT EXISTS (SELECT 1 FROM usable u WHERE u.observation_id = o.observation_id
                       AND u.report_path = json_extract(j.value, '$.path'))`,
    repositoryId, locationWatermark, now, snapshot.observation_watermark, JSON.stringify([sample.observation_id]),
  );
  const revision = (await db
    .prepare("SELECT evidence_revision FROM repositories WHERE repository_id = ?1")
    .bind(repositoryId)
    .first<{ evidence_revision: number }>())!.evidence_revision;
  const evaluationInsert = `
    INSERT INTO evaluations(evaluation_id, repository_id, purpose, head, tree, plan_identity, verifier_identity,
      evidence_revision, observation_watermark, location_watermark, required_cells_json, decisions_json,
      satisfied, evaluated_at_ms, valid_until_ms)
    SELECT ?1, repository_id, 'ci-reuse', ?2, ?3, 'plan', 'verifier', evidence_revision, ?4, ?5, '[]', '[]', 0, ?6, ?7
    FROM repositories WHERE repository_id = ?8 AND evidence_revision = ?9`;
  rows.q9_current_evaluation_changes = (await db
    .prepare(evaluationInsert)
    .bind(`eval-${crypto.randomUUID()}`, head, tree, snapshot.observation_watermark, locationWatermark, now, now + 60_000, repositoryId, revision)
    .run()).meta.changes;
  rows.q9_stale_evaluation_changes = (await db
    .prepare(evaluationInsert)
    .bind(`eval-${crypto.randomUUID()}`, head, tree, snapshot.observation_watermark, locationWatermark, now, now + 60_000, repositoryId, revision - 1)
    .run()).meta.changes;
  rows.q10_quick_answer = await run(
    `SELECT e.* FROM evaluations e JOIN repositories r ON r.repository_id = e.repository_id
     WHERE e.repository_id = ?1 AND e.purpose = 'ci-reuse' AND e.head = ?2 AND e.tree = ?3
       AND e.plan_identity = 'plan' AND e.verifier_identity = 'verifier'
       AND e.evidence_revision = r.evidence_revision AND e.valid_until_ms > ?4
     ORDER BY e.seq DESC LIMIT 1`,
    repositoryId, head, tree, now,
  );
  rows.q13_export = await run(
    `SELECT payload_json FROM observations WHERE repository_id = ?1 AND head = ?2 AND tree = ?3
       AND environment = (SELECT environment FROM observations WHERE repository_id = ?1 LIMIT 1)
       AND seq <= ?4 ORDER BY observation_id`,
    repositoryId, head, tree, snapshot.observation_watermark,
  );
  rows.q14_feed = await run(
    `SELECT * FROM observations WHERE repository_id = ?1 AND seq > ?2 AND seq <= ?3 ORDER BY seq LIMIT ?4`,
    repositoryId, 0, snapshot.observation_watermark, 100,
  );
  rows.q15_growth = await run(
    `SELECT environment, json_extract(payload_json, '$.producer.kind') AS producer_kind, COUNT(*) AS observations,
            SUM(length(CAST(payload_json AS BLOB))) AS payload_bytes, MAX(received_at_ms) AS last_received_at_ms
     FROM observations WHERE repository_id = ?1 GROUP BY environment, producer_kind ORDER BY environment, producer_kind`,
    repositoryId,
  );
  return json({ snapshot, rows });
}

/**
 * CPU proxy at the declared request bound: parse, key-sort canonicalize, and
 * SHA-256 every observation and the collection. The Phase 2 lossless port
 * replaces this proxy; Free-tier CPU exhaustion surfaces as a failed request.
 */
async function cpuSample(body: string): Promise<Response> {
  const started = performance.now();
  const document = JSON.parse(body) as { observations: unknown[] };
  const canonical = (value: unknown): string => {
    if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
    if (value !== null && typeof value === "object") {
      const entries = Object.keys(value as object)
        .sort()
        .map((key) => `${JSON.stringify(key)}:${canonical((value as Record<string, unknown>)[key])}`);
      return `{${entries.join(",")}}`;
    }
    return JSON.stringify(value);
  };
  const encoder = new TextEncoder();
  for (const observation of document.observations) {
    await crypto.subtle.digest("SHA-256", encoder.encode(canonical(observation)));
  }
  await crypto.subtle.digest("SHA-256", encoder.encode(canonical(document)));
  return json({
    observations: document.observations.length,
    request_bytes: body.length,
    elapsed_ms: performance.now() - started,
  });
}

export default {
  async fetch(request: Request, env: SpikeEnv): Promise<Response> {
    // An unset secret must not turn the literal "Bearer undefined" into a key.
    if (!env.SPIKE_TOKEN) return json({ error: "unavailable" }, 503);
    if (request.headers.get("authorization") !== `Bearer ${env.SPIKE_TOKEN}`) {
      return json({ error: "unauthenticated" }, 401);
    }
    if (request.method !== "POST") return json({ error: "method" }, 405);
    const path = new URL(request.url).pathname;
    if (path === "/cpu-sample") return cpuSample(await request.text());
    const body = (await request.json()) as Record<string, unknown> & IngestRequest;
    switch (path) {
      case "/repo":
        await env.SPIKE_DB.prepare("INSERT INTO repositories(repository_id, slug) VALUES (?1, ?2)")
          .bind(body.repository_id, String(body.slug))
          .run();
        return json({ ok: true });
      case "/ingest":
        return ingest(env.SPIKE_DB, body);
      case "/state":
        return state(env.SPIKE_DB, body.repository_id);
      case "/late-failure":
        return lateFailure(env.SPIKE_DB, body);
      case "/guards":
        return guards(env.SPIKE_DB, body);
      case "/catalog":
        return catalog(env.SPIKE_DB, body.repository_id, body.head, body.tree);
      default:
        return json({ error: "not_found" }, 404);
    }
  },
} satisfies ExportedHandler<SpikeEnv>;
