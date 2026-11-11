// `POST /retention/certificates` and `GET /retention/jobs/{job_id}` through
// the Worker, with an isolated retention-verifier principal (no production
// grant exists). Every refusal is checked for its public answer and for the
// retention rows D1 holds afterwards.
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import { createWorker, type Env } from "../../src/worker.ts";
import { advanceRetentionJobs } from "../../src/retention.ts";
import {
  db,
  expectError,
  harness,
  hex,
  newRepository,
  postJson,
  principal,
  repositoryState,
  type Harness,
} from "./helpers.ts";
import {
  ancestry,
  certificate,
  certificatesPath,
  certifiedCell,
  currentSnapshot,
  GRACE,
  headOf,
  jobPath,
  jobState,
  NO_RETENTION_ROWS,
  POLICY,
  retentionRows,
  retry,
  seedAt,
  submit,
  synthetic,
  type RetentionJobBody,
  type Seeded,
} from "./retention-fixtures.ts";

interface Pair {
  repo: string;
  request: Harness;
  older: Seeded;
  newer: Seeded;
}

/** A pass at head `1…` superseded by a pass at head `2…` in `CELL`. */
async function pair(): Promise<Pair> {
  const repo = await newRepository();
  const request = harness([repo]);
  const older = await seedAt(repo, headOf("1"));
  const newer = await seedAt(repo, headOf("2"));
  return { repo, request, older, newer };
}

async function prunes(p: Pair, overrides: Record<string, unknown> = {}): Promise<Record<string, unknown>> {
  return certificate(await currentSnapshot(p.request, p.repo), [certifiedCell([p.older, p.newer], [p.newer], [ancestry(p.older, p.newer)])], overrides);
}

describe("POST /retention/certificates", () => {
  it("accepts a pass superseded on a descendant head (202) and schedules its job 14 days out", async () => {
    const p = await pair();
    const body = await prunes(p);
    const response = await submit(p.request, p.repo, body);
    expect(response.status).toBe(202);
    const job = (await response.json()) as RetentionJobBody;
    expect(job).toEqual({
      job_id: expect.stringMatching(/^[0-9a-f]{32}$/),
      certificate_id: body["certificate_id"],
      state: "scheduled",
      snapshot: body["snapshot"],
      member_count: 1,
      progress: { locations_withdrawn: 0, objects_deleted: 0, observations_deleted: 0 },
      not_before_ms: p.request.clock + GRACE,
      created_at_ms: p.request.clock,
      updated_at_ms: p.request.clock,
    });
    expect(await retentionRows(p.repo)).toEqual({ certificates: 1, jobs: 1, members: 1, pins: 0, supersessions: 1, audits: 1 });
    const member = await db().prepare("SELECT observation_id, superseded_at_ms FROM retention_job_members WHERE job_id = ?1").bind(job.job_id).first();
    expect(member).toEqual({ observation_id: p.older.id, superseded_at_ms: p.request.clock });
    // The submission itself deletes nothing and moves no revision.
    expect((await repositoryState(p.repo)).observations).toHaveLength(2);
  });

  it("protects the current pass: an unsuperseded observation left out of retained_ids is refused", async () => {
    const p = await pair();
    const body = certificate(await currentSnapshot(p.request, p.repo), [certifiedCell([p.older, p.newer], [p.older], [ancestry(p.older, p.newer)])]);
    const refusal = await expectError(await submit(p.request, p.repo, body), 400, "invalid_input", "/cells/0/candidate_ids/1");
    expect(refusal.error.message).toContain("must be retained");
    expect(await retentionRows(p.repo)).toEqual(NO_RETENTION_ROWS);
  });

  it("protects an unresolved failure: a failure no edge supersedes cannot be pruned", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const failure = await seedAt(repo, headOf("1"), synthetic({ outcome: "fail" }));
    const pass = await seedAt(repo, headOf("2"));
    const body = certificate(await currentSnapshot(request, repo), [certifiedCell([failure, pass], [pass], [])]);
    await expectError(await submit(request, repo, body), 400, "invalid_input", "/cells/0/candidate_ids/0");
    expect(await retentionRows(repo)).toEqual(NO_RETENTION_ROWS);
  });

  it("protects a same-head pass/fail conflict: strict ancestry between one head is refused", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const failure = await seedAt(repo, headOf("3"), synthetic({ outcome: "fail" }));
    const pass = await seedAt(repo, headOf("3"));
    const body = certificate(await currentSnapshot(request, repo), [certifiedCell([failure, pass], [pass], [ancestry(failure, pass)])]);
    const refusal = await expectError(await submit(request, repo, body), 400, "invalid_input", "/cells/0/supersessions/0/evidence/descendant_head");
    expect(refusal.error.message).toContain("conflict");
    expect(await retentionRows(repo)).toEqual(NO_RETENTION_ROWS);
  });

  it("protects observations on unrelated heads and binds ancestry claims to the stored heads", async () => {
    const p = await pair();
    const snapshot = await currentSnapshot(p.request, p.repo);
    // No edge: both are maximal, so both must be retained.
    await expectError(await submit(p.request, p.repo, certificate(snapshot, [certifiedCell([p.older, p.newer], [p.newer], [])])), 400, "invalid_input", "/cells/0/candidate_ids/0");
    // An edge naming heads the store does not hold for those observations.
    const forged = { ...ancestry(p.older, p.newer), evidence: { kind: "strict-ancestry", ancestor_head: headOf("9"), descendant_head: p.newer.head } };
    await expectError(
      await submit(p.request, p.repo, certificate(snapshot, [certifiedCell([p.older, p.newer], [p.newer], [forged])])),
      400,
      "invalid_input",
      "/cells/0/supersessions/0/evidence/ancestor_head",
    );
    // Both retained with no edge is a valid no-op.
    const keepAll = await submit(p.request, p.repo, certificate(snapshot, [certifiedCell([p.older, p.newer], [p.older, p.newer], [])]));
    expect(keepAll.status).toBe(202);
    expect(((await keepAll.json()) as RetentionJobBody).member_count).toBe(0);
  });

  it("prunes a failed attempt resolved by a stored retry link, and refuses a chain the store does not record", async () => {
    const repo = await newRepository();
    const request = harness([repo]);
    const failed = await seedAt(repo, headOf("4"), synthetic({ outcome: "fail" }));
    const retried = await seedAt(repo, headOf("4"), synthetic({ retryOf: failed.id }));
    const unrelated = await seedAt(repo, headOf("4"), synthetic({ outcome: "fail" }));
    const snapshot = await currentSnapshot(request, repo);
    const candidates = [failed, retried, unrelated];

    // `unrelated` is not linked to `retried` by any stored retry_of.
    await expectError(
      await submit(request, repo, certificate(snapshot, [certifiedCell(candidates, [retried, failed], [retry([unrelated.id, retried.id])])])),
      400,
      "invalid_input",
      "/cells/0/supersessions/0/evidence/chain/1",
    );
    // Reversed: the stored link points the other way.
    await expectError(
      await submit(request, repo, certificate(snapshot, [certifiedCell(candidates, [failed, unrelated], [retry([retried.id, failed.id])])])),
      400,
      "invalid_input",
      "/cells/0/supersessions/0/evidence/chain/1",
    );
    expect(await retentionRows(repo)).toEqual(NO_RETENTION_ROWS);

    const accepted = await submit(request, repo, certificate(snapshot, [certifiedCell(candidates, [retried, unrelated], [retry([failed.id, retried.id])])]));
    expect(accepted.status).toBe(202);
    const job = (await accepted.json()) as RetentionJobBody;
    await advanceRetentionJobs(env, request.clock + GRACE);
    expect(await jobState(job.job_id)).toBe("completed");
    expect((await repositoryState(repo)).observations).toEqual([retried.id, unrelated.id].sort());
  });

  it("refuses a stale or pruned snapshot with 409 and a snapshot ahead of the store with 400", async () => {
    const p = await pair();
    const body = await prunes(p);
    const snapshot = body["snapshot"] as Record<string, unknown>;
    const at = (patch: Record<string, unknown>) => ({ ...body, snapshot: { ...snapshot, ...patch } });
    await expectError(await submit(p.request, p.repo, at({ generation: hex(32) })), 409, "stale_snapshot", "/snapshot/generation");
    await expectError(await submit(p.request, p.repo, at({ evidence_revision: (snapshot["evidence_revision"] as number) + 1 })), 400, "invalid_input", "/snapshot/evidence_revision");
    await expectError(await submit(p.request, p.repo, at({ repository_id: "1" })), 400, "invalid_input", "/snapshot/repository_id");

    // Any write after the snapshot makes it stale.
    await seedAt(p.repo, headOf("5"), synthetic({ package: "playa" }));
    await expectError(await submit(p.request, p.repo, body), 409, "stale_snapshot", "/snapshot/evidence_revision");
    const fresh = await currentSnapshot(p.request, p.repo);
    await expectError(await submit(p.request, p.repo, { ...body, snapshot: { ...fresh, location_event_watermark: fresh.location_event_watermark + 1 } }), 400, "invalid_input");

    // A prune since the snapshot answers snapshot_pruned.
    await db().prepare("UPDATE repositories SET retention_epoch = retention_epoch + 1 WHERE repository_id = ?1").bind(p.repo).run();
    await expectError(await submit(p.request, p.repo, { ...body, snapshot: fresh }), 409, "snapshot_pruned", "/snapshot/retention_epoch");
    expect(await retentionRows(p.repo)).toEqual(NO_RETENTION_ROWS);
  });

  it("refuses a partial candidate set and missing store data with 409 stale_snapshot", async () => {
    const p = await pair();
    const third = await seedAt(p.repo, headOf("6"));
    const snapshot = await currentSnapshot(p.request, p.repo);
    // The verifier saw only two of the cell's three observations.
    await expectError(
      await submit(p.request, p.repo, certificate(snapshot, [certifiedCell([p.older, p.newer], [p.newer], [ancestry(p.older, p.newer)])])),
      409,
      "stale_snapshot",
      "/cells/0/candidate_ids",
    );
    // The verifier names an observation the store does not hold.
    const ghost = { id: hex(64), head: headOf("7") };
    const missing = await submit(
      p.request,
      p.repo,
      certificate(snapshot, [certifiedCell([p.older, p.newer, third, ghost], [p.newer, third, ghost], [ancestry(p.older, p.newer)])]),
    );
    const refusal = await expectError(missing, 409, "stale_snapshot", "/cells/0/candidate_ids/3");
    expect(refusal.error.message).toContain("missing store data");
    // An observation stored in another cell is not a candidate of this one.
    const elsewhere = await seedAt(p.repo, headOf("8"), synthetic({ package: "playa" }));
    await expectError(
      await submit(
        p.request,
        p.repo,
        certificate(await currentSnapshot(p.request, p.repo), [certifiedCell([p.older, p.newer, third, elsewhere], [p.newer, third, elsewhere], [ancestry(p.older, p.newer)])]),
      ),
      409,
      "stale_snapshot",
      "/cells/0/candidate_ids/3",
    );
    expect(await retentionRows(p.repo)).toEqual(NO_RETENTION_ROWS);
  });

  it("refuses edges that cross cells, repeat, loop, or supersede themselves, and a cell listed twice", async () => {
    const p = await pair();
    const other = await seedAt(p.repo, headOf("9"), synthetic({ package: "playa" }));
    const snapshot = await currentSnapshot(p.request, p.repo);
    const edge = ancestry(p.older, p.newer);
    const cases: [Record<string, unknown>[], string][] = [
      [[certifiedCell([p.older, p.newer], [p.newer], [ancestry(p.older, other)])], "/cells/0/supersessions/0/superseded_by_id"],
      [[certifiedCell([p.older, p.newer], [p.newer], [edge, edge])], "/cells/0/supersessions/1"],
      [[certifiedCell([p.older, p.newer], [p.newer], [{ ...edge, superseded_by_id: p.older.id }])], "/cells/0/supersessions/0/superseded_by_id"],
      [[certifiedCell([p.older, p.newer], [p.newer], [edge, { ...ancestry(p.newer, p.older) }])], "/cells/0/supersessions"],
      [[certifiedCell([p.older, p.newer], [p.newer], [edge]), certifiedCell([p.older, p.newer], [p.newer], [edge])], "/cells/1"],
      [[certifiedCell([p.older, p.newer], [p.newer, other], [edge])], "/cells/0/retained_ids/1"],
    ];
    for (const [cells, location] of cases) {
      await expectError(await submit(p.request, p.repo, certificate(snapshot, cells)), 400, "invalid_input", location);
    }
    expect(await retentionRows(p.repo)).toEqual(NO_RETENTION_ROWS);
  });

  it("stores release pins idempotently and never prunes a release-cited observation", async () => {
    const p = await pair();
    const snapshot = await currentSnapshot(p.request, p.repo);
    const prune = [certifiedCell([p.older, p.newer], [p.newer], [ancestry(p.older, p.newer)])];

    // A pin in the same certificate.
    await expectError(
      await submit(p.request, p.repo, certificate(snapshot, prune, { release_pins: [{ release: "v1", observation_ids: [p.older.id] }] })),
      400,
      "invalid_input",
      "/release_pins/0/observation_ids/0",
    );
    // A pin naming an observation the store does not hold.
    await expectError(
      await submit(p.request, p.repo, certificate(snapshot, [], { release_pins: [{ release: "v1", observation_ids: [hex(64)] }] })),
      400,
      "invalid_input",
      "/release_pins/0/observation_ids/0",
    );
    expect(await retentionRows(p.repo)).toEqual(NO_RETENTION_ROWS);

    // An earlier certificate pins `older` (a no-op prune set); pinning it again is idempotent.
    for (const id of [crypto.randomUUID(), crypto.randomUUID()]) {
      const pinned = await submit(p.request, p.repo, certificate(snapshot, [], { certificate_id: id, release_pins: [{ release: "v1", observation_ids: [p.older.id] }] }));
      expect(pinned.status).toBe(202);
    }
    expect((await retentionRows(p.repo))["pins"]).toBe(1);
    // A stored pin protects it from every later certificate.
    await expectError(await submit(p.request, p.repo, certificate(snapshot, prune)), 400, "invalid_input", "/cells/0/candidate_ids/0");
    expect((await retentionRows(p.repo))["members"]).toBe(0);
  });

  it("answers an identical replay with 200 and the same job, and different content with 409", async () => {
    const p = await pair();
    const body = await prunes(p);
    const first = await submit(p.request, p.repo, body);
    expect(first.status).toBe(202);
    const job = await first.json();
    const before = await retentionRows(p.repo);

    // Whitespace and key order are not content.
    const reordered = `{ "release_pins": [], ${JSON.stringify(body).slice(1).replace(',"release_pins":[]', "")}`;
    for (const replayed of [body, reordered]) {
      const replay = await submit(p.request, p.repo, replayed);
      expect(replay.status).toBe(200);
      expect(await replay.json()).toEqual(job);
    }
    const conflict = await submit(p.request, p.repo, { ...body, verifier_policy: { ...POLICY, version: "2" } });
    await expectError(conflict, 409, "identity_conflict", "/certificate_id");
    expect(await retentionRows(p.repo)).toEqual(before);
  });

  it("treats an empty prune set as a completed no-op job", async () => {
    const p = await pair();
    const body = certificate(await currentSnapshot(p.request, p.repo), []);
    const response = await submit(p.request, p.repo, body);
    expect(response.status).toBe(202);
    const job = (await response.json()) as RetentionJobBody;
    expect({ state: job.state, member_count: job.member_count, not_before_ms: job.not_before_ms }).toEqual({
      state: "completed",
      member_count: 0,
      not_before_ms: p.request.clock,
    });
    const run = await advanceRetentionJobs(env, p.request.clock + GRACE, { jobId: job.job_id });
    expect(run.steps).toEqual([]);
    expect((await submit(p.request, p.repo, body)).status).toBe(200);
    expect((await repositoryState(p.repo)).observations).toHaveLength(2);
  });

  it("refuses every role but retention-verifier with 403 and writes nothing", async () => {
    const p = await pair();
    const body = await prunes(p);
    for (const as of ["producer", "verifier", "reader", "operator"]) {
      await expectError(await submit(p.request, p.repo, body, as), 403, "denied");
    }
    expect(await retentionRows(p.repo)).toEqual(NO_RETENTION_ROWS);
  });

  it("answers 404 for an unregistered repository after authorization", async () => {
    const p = await pair();
    const unknown = "9" + p.repo.slice(1);
    const request = harness([unknown]);
    const body = { ...(await prunes(p)), snapshot: { ...(await currentSnapshot(p.request, p.repo)), repository_id: unknown } };
    await expectError(await submit(request, unknown, body), 404, "not_found");
    await expectError(await submit(request, unknown, body, "producer"), 403, "denied");
  });

  it("answers 503 and writes nothing when the D1 write fails", async () => {
    const p = await pair();
    const body = await prunes(p);
    let batches = 0;
    const real = db();
    const failing = new Proxy(real, {
      get(target, property) {
        if (property === "batch") {
          return async (statements: D1PreparedStatement[]) => {
            batches += 1;
            // The first batch reads pins and jobs; the second is the write.
            if (batches === 2) throw new Error("D1 is unavailable");
            return target.batch(statements);
          };
        }
        const value = Reflect.get(target, property) as unknown;
        return typeof value === "function" ? (value as (...args: unknown[]) => unknown).bind(target) : value;
      },
    });
    const worker = createWorker({ authenticate: async () => principal("ffff", [p.repo], ["retention-verifier"]), now: () => p.request.clock });
    const response = await worker.fetch!(
      new Request(`https://evidence.test${certificatesPath(p.repo)}`, { method: "POST", body: JSON.stringify(body) }) as never,
      { ...(env as unknown as Env), EVIDENCE_DB: failing },
      {} as ExecutionContext,
    );
    const refusal = await expectError(response, 503, "unavailable");
    expect(refusal.error.retryable).toBe(true);
    expect(await retentionRows(p.repo)).toEqual(NO_RETENTION_ROWS);
    expect((await submit(p.request, p.repo, body)).status).toBe(202);
  });

  describe("input shapes", () => {
    async function setup(): Promise<{ p: Pair; body: Record<string, unknown> }> {
      const p = await pair();
      return { p, body: await prunes(p) };
    }

    it("walks absent, null, wrong-type, and empty shapes through the shared readers once", async () => {
      const { p, body } = await setup();
      const without = (key: string) => Object.fromEntries(Object.entries(body).filter(([name]) => name !== key));
      const cases: [unknown, string][] = [
        // string reader: certificate_id
        [without("certificate_id"), "/certificate_id"],
        [{ ...body, certificate_id: null }, "/certificate_id"],
        [{ ...body, certificate_id: 7 }, "/certificate_id"],
        [{ ...body, certificate_id: "" }, "/certificate_id"],
        [{ ...body, certificate_id: "not-a-uuid" }, "/certificate_id"],
        // integer reader: snapshot.evidence_revision
        [{ ...body, snapshot: { ...(body["snapshot"] as object), evidence_revision: null } }, "/snapshot/evidence_revision"],
        [{ ...body, snapshot: { ...(body["snapshot"] as object), evidence_revision: "3" } }, "/snapshot/evidence_revision"],
        [`{"certificate_id":"${body["certificate_id"]}","snapshot":${JSON.stringify(body["snapshot"]).replace(/"evidence_revision":(\d+)/, '"evidence_revision":$1.0')},"cells":[],"verifier_policy":${JSON.stringify(POLICY)},"release_pins":[]}`, "/snapshot/evidence_revision"],
        [{ ...body, snapshot: { ...(body["snapshot"] as object), evidence_revision: -1 } }, "/snapshot/evidence_revision"],
        // object reader: verifier_policy
        [without("verifier_policy"), "/verifier_policy"],
        [{ ...body, verifier_policy: null }, "/verifier_policy"],
        [{ ...body, verifier_policy: [] }, "/verifier_policy"],
        [{ ...body, verifier_policy: { ...POLICY, extra: 1 } }, "/verifier_policy/extra"],
        // array reader: cells, then its elements
        [without("cells"), "/cells"],
        [{ ...body, cells: null }, "/cells"],
        [{ ...body, cells: {} }, "/cells"],
        [{ ...body, cells: [7] }, "/cells/0"],
        [{ ...body, cells: [...(body["cells"] as unknown[]), "x"] }, "/cells/1"],
        // string-array reader: candidate_ids elements
        [{ ...body, cells: [{ ...(body["cells"] as Record<string, unknown>[])[0], candidate_ids: [p.older.id, 5] }] }, "/cells/0/candidate_ids/1"],
        [{ ...body, cells: [{ ...(body["cells"] as Record<string, unknown>[])[0], candidate_ids: [p.older.id, p.older.id] }] }, "/cells/0/candidate_ids/1"],
        // root shape
        [[], ""],
        [{ ...body, unexpected: true }, "/unexpected"],
      ];
      for (const [candidate, location] of cases) {
        const response = await postJson(p.request, certificatesPath(p.repo), candidate, "retention-verifier");
        await expectError(response, 400, "invalid_input", location);
      }
      expect(await retentionRows(p.repo)).toEqual(NO_RETENTION_ROWS);
    });

    it("wires each other field with one invalid shape (the control is the accepted certificate)", async () => {
      const { p, body } = await setup();
      const cell = (body["cells"] as Record<string, unknown>[])[0]!;
      const edge = (cell["supersessions"] as Record<string, unknown>[])[0]!;
      const withCell = (patch: Record<string, unknown>) => ({ ...body, cells: [{ ...cell, ...patch }] });
      const withEdge = (patch: Record<string, unknown>) => withCell({ supersessions: [{ ...edge, ...patch }] });
      const cases: [unknown, string][] = [
        [{ ...body, snapshot: { ...(body["snapshot"] as object), generation: "" } }, "/snapshot/generation"],
        [{ ...body, snapshot: { ...(body["snapshot"] as object), retention_epoch: true } }, "/snapshot/retention_epoch"],
        [{ ...body, snapshot: { ...(body["snapshot"] as object), observation_event_watermark: 1.5 } }, "/snapshot/observation_event_watermark"],
        [{ ...body, snapshot: { ...(body["snapshot"] as object), location_event_watermark: "0" } }, "/snapshot/location_event_watermark"],
        [{ ...body, snapshot: { ...(body["snapshot"] as object), repository_id: "abc" } }, "/snapshot/repository_id"],
        [withCell({ package: "" }), "/cells/0/package"],
        [withCell({ environment: "plan9" }), "/cells/0/environment"],
        [withCell({ gate: "L9" }), "/cells/0/gate"],
        [withCell({ retained_ids: [] }), "/cells/0/retained_ids"],
        [withCell({ supersessions: null }), "/cells/0/supersessions"],
        [withEdge({ superseded_id: "XYZ" }), "/cells/0/supersessions/0/superseded_id"],
        [withEdge({ superseded_by_id: 1 }), "/cells/0/supersessions/0/superseded_by_id"],
        [withEdge({ evidence: { kind: "timestamp" } }), "/cells/0/supersessions/0/evidence/kind"],
        [withEdge({ evidence: { kind: "retry", chain: [p.older.id] } }), "/cells/0/supersessions/0/evidence/chain"],
        [withEdge({ evidence: { kind: "retry", chain: [p.older.id, p.newer.id], ancestor_head: p.older.head } }), "/cells/0/supersessions/0/evidence/ancestor_head"],
        [withEdge({ evidence: { kind: "strict-ancestry", ancestor_head: "zz", descendant_head: p.newer.head } }), "/cells/0/supersessions/0/evidence/ancestor_head"],
        [{ ...body, verifier_policy: { ...POLICY, name: "" } }, "/verifier_policy/name"],
        [{ ...body, verifier_policy: { ...POLICY, version: 1 } }, "/verifier_policy/version"],
        [{ ...body, verifier_policy: { ...POLICY, digest: "abc" } }, "/verifier_policy/digest"],
        [{ ...body, release_pins: null }, "/release_pins"],
        [{ ...body, release_pins: [{ release: "", observation_ids: [] }] }, "/release_pins/0/release"],
        [{ ...body, release_pins: [{ release: "v1" }] }, "/release_pins/0/observation_ids"],
        [{ ...body, release_pins: [{ release: "v1", observation_ids: [] }, { release: "v1", observation_ids: [] }] }, "/release_pins/1/release"],
      ];
      for (const [candidate, location] of cases) {
        await expectError(await postJson(p.request, certificatesPath(p.repo), candidate, "retention-verifier"), 400, "invalid_input", location);
      }
      expect(await retentionRows(p.repo)).toEqual(NO_RETENTION_ROWS);
      expect((await submit(p.request, p.repo, body)).status).toBe(202);
    });

    it("reads the body through the strict loader (duplicate keys refused) and bounds its size", async () => {
      const { p, body } = await setup();
      const text = JSON.stringify(body);
      const duplicated = `{"certificate_id":"${crypto.randomUUID()}",${text.slice(1)}`;
      await expectError(await postJson(p.request, certificatesPath(p.repo), duplicated, "retention-verifier"), 400, "invalid_input");
      const oversized = `${text.slice(0, -1)},"pad":"${"x".repeat(1024 * 1024)}"}`;
      await expectError(await postJson(p.request, certificatesPath(p.repo), oversized, "retention-verifier"), 413, "size_limit");
      expect(await retentionRows(p.repo)).toEqual(NO_RETENTION_ROWS);
    });
  });
});

describe("GET /retention/jobs/{job_id}", () => {
  it("answers the job to a retention verifier and an operator, and 403 or 404 otherwise", async () => {
    const p = await pair();
    const job = (await (await submit(p.request, p.repo, await prunes(p))).json()) as RetentionJobBody;
    for (const as of ["retention-verifier", "operator"]) {
      const response = await p.request(jobPath(p.repo, job.job_id), { as });
      expect(response.status).toBe(200);
      expect(await response.json()).toEqual(job);
    }
    for (const as of ["reader", "producer", "verifier"]) await expectError(await p.request(jobPath(p.repo, job.job_id), { as }), 403, "denied");
    await expectError(await p.request(jobPath(p.repo, hex(32)), { as: "retention-verifier" }), 404, "not_found");
    await expectError(await p.request(jobPath(p.repo, "not-a-job"), { as: "retention-verifier" }), 404, "not_found");
    // A job of another repository is not found here.
    const other = await newRepository();
    const otherRequest = harness([other]);
    await expectError(await otherRequest(jobPath(other, job.job_id), { as: "retention-verifier" }), 404, "not_found");
  });
});
