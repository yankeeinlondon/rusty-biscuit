// Focused correctness cases for a deployed evidence store, written once and run
// over two transports: the in-process Worker with the real authenticator
// (`tests/service/acceptance.test.ts`) and a deployed endpoint over HTTP
// (`tests/real/acceptance.test.ts`). Cases observe only the public API, so
// they must not import `cloudflare:workers` or read D1.
//
// Every case works on fresh observation identities and fresh random heads, so
// it is repeatable against a repository that already holds evidence, and
// revokes every credential it issued. Tokens never appear in a test name or an
// assertion message.
import { expect } from "vitest";
import { canonical, identity, jsonNumber, loadStrict, observationIdentity, type JsonObject } from "../../src/contracts/index.ts";
import rigFixture from "../fixtures/parity/collection-rig.json?raw";

export interface AcceptanceTarget {
  send(path: string, init: { method?: string; body?: string | Uint8Array; token: string }): Promise<Response>;
  readonly repositoryId: string;
  readonly operatorToken: string;
  /** Moves the Worker's clock; only an in-process target can. */
  readonly advanceClock?: (ms: number) => void;
}

export interface AcceptanceCase {
  readonly name: string;
  /** `clock`: needs `target.advanceClock`, so it cannot run against a deployment. */
  readonly requires?: "clock";
  run(target: AcceptanceTarget): Promise<void>;
}

const DAY_MS = 86_400_000;
const PART = 1024 * 1024;
const GATE = { package: "biscuit-hash", gate: "L1" } as const;

type Role = "reader" | "producer" | "verifier";

interface Credentials {
  issue(roles: Role[], extra?: Record<string, unknown>): Promise<{ token: string; id: string }>;
  revoke(id: string): Promise<Response>;
}

interface ErrorEnvelope {
  error?: { code?: string; message?: string };
}

/** Fails with the status and error code only; never with a body that could carry a token. */
async function expectStatus(response: Response, status: number, code?: string): Promise<void> {
  if (response.status === status && code === undefined) return;
  const envelope = (await response.clone().json().catch(() => ({}))) as ErrorEnvelope;
  expect({ status: response.status, code: envelope.error?.code }, envelope.error?.message ?? "").toEqual({ status, code });
}

async function json<T>(response: Response, status: number): Promise<T> {
  await expectStatus(response, status);
  return (await response.json()) as T;
}

/** Runs `body` with disposable credentials and revokes each one afterward, even after a failure. */
async function withCredentials(target: AcceptanceTarget, body: (credentials: Credentials) => Promise<void>): Promise<void> {
  const issued: string[] = [];
  const revoke = (id: string) =>
    target.send(`/v1/operator/credentials/${id}/revoke`, { method: "POST", body: '{"reason":"acceptance case finished"}', token: target.operatorToken });
  const credentials: Credentials = {
    async issue(roles, extra = {}) {
      const response = await target.send("/v1/operator/credentials", {
        method: "POST",
        body: JSON.stringify({ label: "acceptance", repository_ids: [target.repositoryId], roles, ...extra }),
        token: target.operatorToken,
      });
      const { secret, credential } = await json<{ secret: string; credential: { credential_id: string } }>(response, 201);
      issued.push(credential.credential_id);
      return { token: secret, id: credential.credential_id };
    },
    revoke,
  };
  let failure: unknown;
  try {
    await body(credentials);
  } catch (error) {
    failure = error;
  }
  const unrevoked: string[] = [];
  for (const id of issued) if ((await revoke(id)).status !== 200) unrevoked.push(id);
  if (failure !== undefined) throw failure;
  if (unrevoked.length > 0) throw new Error(`could not revoke disposable credentials ${unrevoked.join(", ")}`);
}

function hex(length: number): string {
  return Array.from(crypto.getRandomValues(new Uint8Array(length / 2)), (b) => b.toString(16).padStart(2, "0")).join("");
}

async function sha256Hex(bytes: Uint8Array): Promise<string> {
  return Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)), (b) => b.toString(16).padStart(2, "0")).join("");
}

function rig(): JsonObject {
  const loaded = loadStrict(new TextEncoder().encode(rigFixture));
  if (!loaded.ok) throw new Error("collection-rig fixture does not load");
  return loaded.value as JsonObject;
}

/** The rig fixture's observation under a new run ID (so a new identity), optionally citing `report` bytes. */
async function freshObservation(report?: Uint8Array): Promise<JsonObject> {
  const observation = (rig()["observations"] as JsonObject[])[0]!;
  (observation["producer"] as JsonObject)["run_id"] = `acceptance-${hex(16)}`;
  if (report !== undefined) {
    const entry = (observation["reports"] as JsonObject[])[0]!;
    entry["sha256"] = await sha256Hex(report);
    entry["bytes"] = jsonNumber(String(report.byteLength));
  }
  observation["observation_id"] = observationIdentity(observation);
  return observation;
}

const idOf = (observation: JsonObject) => observation["observation_id"] as string;

function collectionOf(head: string, tree: string, observations: JsonObject[]): JsonObject {
  const sorted = [...observations].sort((a, b) => (idOf(a) < idOf(b) ? -1 : 1));
  return { ...rig(), head, tree, observations: sorted };
}

function submit(target: AcceptanceTarget, token: string, collection: JsonObject): Promise<Response> {
  return target.send(`/v1/repos/${target.repositoryId}/collections`, {
    method: "POST",
    body: `{"source_version":3,"collection":${canonical(collection)}}`,
    token,
  });
}

async function ingest(target: AcceptanceTarget, token: string, head: string, observations: JsonObject[]): Promise<void> {
  await expectStatus(await submit(target, token, collectionOf(head, hex(40), observations)), 200);
}

function repo(target: AcceptanceTarget, path: string): string {
  return `/v1/repos/${target.repositoryId}${path}`;
}

function post(target: AcceptanceTarget, token: string, path: string, body: unknown): Promise<Response> {
  return target.send(repo(target, path), { method: "POST", body: JSON.stringify(body), token });
}

const observationPath = (id: string) => `/observations/${id}`;

interface Snapshot {
  observation_event_watermark: number;
}

interface Page<T> {
  snapshot: Snapshot;
  items: T[];
  has_more: boolean;
  next_cursor: string | null;
}

type HistoryItem = { sequence: number; observation: { observation_id: string } };
type ChangeItem = { sequence: number; observation_id?: string };

/** Follows `first` to its last page, re-sending the query's paging fields with each cursor. */
async function follow<T>(target: AcceptanceTarget, token: string, path: string, query: Record<string, unknown>, first: Page<T>): Promise<Page<T>[]> {
  const pages = [first];
  for (let page = first; page.next_cursor !== null; ) {
    const { snapshot: _snapshot, after_sequence: _after, ...paging } = query;
    page = await json<Page<T>>(await post(target, token, path, { ...paging, cursor: page.next_cursor }), 200);
    pages.push(page);
  }
  return pages;
}

const concurrentMixedIdentityRollback: AcceptanceCase = {
  name: "a collection conflicting with an accepted identity rolls back whole while a concurrent valid one commits",
  run: (target) =>
    withCredentials(target, async ({ issue }) => {
      const producer = await issue(["producer"]);
      const reader = await issue(["reader"]);
      const tree = hex(40);
      const anchor = await freshObservation();
      await ingest(target, producer.token, hex(40), [anchor]);

      const valid = collectionOf(hex(40), tree, [await freshObservation(), await freshObservation()]);
      // The anchor's ID under another head is an identity conflict (same ID, different context).
      const conflicting = collectionOf(hex(40), tree, [anchor, await freshObservation(), await freshObservation()]);
      const [accepted, refused] = await Promise.all([submit(target, producer.token, valid), submit(target, producer.token, conflicting)]);
      await expectStatus(accepted, 200);
      await expectStatus(refused, 409, "identity_conflict");

      for (const observation of valid["observations"] as JsonObject[]) {
        await expectStatus(await target.send(repo(target, observationPath(idOf(observation))), { token: reader.token }), 200);
      }
      await expectStatus(await target.send(repo(target, `/collections/${identity(canonical(valid))}`), { token: reader.token }), 200);
      for (const observation of (conflicting["observations"] as JsonObject[]).filter((o) => idOf(o) !== idOf(anchor))) {
        await expectStatus(await target.send(repo(target, observationPath(idOf(observation))), { token: reader.token }), 404, "not_found");
      }
      await expectStatus(await target.send(repo(target, `/collections/${identity(canonical(conflicting))}`), { token: reader.token }), 404, "not_found");
    }),
};

const revokedAndUnauthorizedIdentities: AcceptanceCase = {
  name: "a revoked credential is unauthenticated and a credential without the role or repository is denied",
  run: (target) =>
    withCredentials(target, async ({ issue, revoke }) => {
      const reader = await issue(["reader"]);
      await expectStatus(await target.send(repo(target, "/snapshot"), { token: reader.token }), 200);
      await expectStatus(await submit(target, reader.token, collectionOf(hex(40), hex(40), [await freshObservation()])), 403, "denied");
      // A grant on the target repository does not open any other repository.
      const other = String(BigInt(target.repositoryId) + 1n);
      await expectStatus(await target.send(`/v1/repos/${other}/snapshot`, { token: reader.token }), 403, "denied");

      await expectStatus(await revoke(reader.id), 200);
      await expectStatus(await target.send(repo(target, "/snapshot"), { token: reader.token }), 401, "unauthenticated");
    }),
};

// The shortest lifetime `issueCredential` accepts is one day, so a deployed
// run cannot observe expiry; only a target with a movable clock runs this.
const expiredIdentity: AcceptanceCase = {
  name: "an expired credential is unauthenticated",
  requires: "clock",
  run: (target) =>
    withCredentials(target, async ({ issue }) => {
      const reader = await issue(["reader"], { expires_in_days: 1 });
      await expectStatus(await target.send(repo(target, "/snapshot"), { token: reader.token }), 200);
      target.advanceClock!(DAY_MS);
      await expectStatus(await target.send(repo(target, "/snapshot"), { token: reader.token }), 401, "unauthenticated");
    }),
};

const interruptedUploadReplay: AcceptanceCase = {
  name: "an interrupted upload resumes, replays parts and finalization idempotently, and serves the exact bytes",
  run: (target) =>
    withCredentials(target, async ({ issue }) => {
      const producer = await issue(["producer"]);
      const verifier = await issue(["verifier"]);
      const reader = await issue(["reader"]);
      const content = new Uint8Array(PART + 17);
      // getRandomValues is limited to 64 KiB; a random head keeps the digest new.
      crypto.getRandomValues(content.subarray(0, 32));
      const observation = await freshObservation(content);
      await ingest(target, producer.token, hex(40), [observation]);
      const report = (observation["reports"] as JsonObject[])[0]!;
      const digest = report["sha256"] as string;
      const request = { observation_id: idOf(observation), reference_kind: "report", reference_digest: digest, bytes: content.byteLength, path: report["path"] };

      const opened = await json<{ upload_id: string }>(await post(target, producer.token, "/uploads", request), 201);
      const partPath = (index: number) => repo(target, `/uploads/${opened.upload_id}/parts/${index}`);
      const put = (index: number) => target.send(partPath(index), { method: "PUT", body: content.slice(index * PART, (index + 1) * PART), token: producer.token });
      const first = await json<unknown>(await put(0), 200);
      expect(await json<unknown>(await put(0), 200)).toEqual(first);

      // The client lost its state after part 0: reopening finds the same upload.
      expect(await json<unknown>(await post(target, producer.token, "/uploads", request), 200)).toMatchObject({ upload_id: opened.upload_id, parts_received: 1 });
      const finalize = () => target.send(repo(target, `/uploads/${opened.upload_id}/finalize`), { method: "POST", token: verifier.token });
      await expectStatus(await finalize(), 400, "invalid_input");

      await expectStatus(await put(1), 200);
      const finalized = await json<{ sha256: string }>(await finalize(), 200);
      expect(finalized.sha256).toBe(digest);
      expect(await json<unknown>(await finalize(), 200)).toEqual(finalized);

      const fetched = await target.send(repo(target, `/objects/${idOf(observation)}/report/${encodeURIComponent(digest)}`), { token: reader.token });
      await expectStatus(fetched, 200);
      expect(fetched.headers.get("x-content-sha256")).toBe(digest);
      expect(await sha256Hex(new Uint8Array(await fetched.arrayBuffer()))).toBe(digest);
    }),
};

const withdrawalWithoutRevival: AcceptanceCase = {
  name: "replaying a location event after its withdrawal returns the original result and does not revive it",
  run: (target) =>
    withCredentials(target, async ({ issue }) => {
      const producer = await issue(["producer"]);
      const verifier = await issue(["verifier"]);
      const reader = await issue(["reader"]);
      const content = crypto.getRandomValues(new Uint8Array(64));
      const observation = await freshObservation(content);
      await ingest(target, producer.token, hex(40), [observation]);
      const report = (observation["reports"] as JsonObject[])[0]!;
      const event = (locator: Record<string, string>, kind: "verified" | "withdrawn") => ({
        event_id: crypto.randomUUID(),
        observation_id: idOf(observation),
        reference_kind: "report",
        reference_digest: report["sha256"],
        path: report["path"],
        locator,
        kind,
        ...(kind === "verified" ? { verification_method: "bytes" } : {}),
      });
      const append = (body: unknown) => post(target, verifier.token, "/locations", body);

      const hostDir = { kind: "host-dir", host: "acceptance-rig", directory: `/reports/${hex(8)}` };
      const available = event(hostDir, "verified");
      const original = await json<unknown>(await append(available), 201);
      const withdrawal = event(hostDir, "withdrawn");
      await expectStatus(await append(withdrawal), 201);
      expect(await json<unknown>(await append(available), 200)).toEqual(original);

      // An object location is the one the store itself serves: its original event is finalization.
      const opened = await json<{ upload_id: string }>(
        await post(target, producer.token, "/uploads", { observation_id: idOf(observation), reference_kind: "report", reference_digest: report["sha256"], bytes: content.byteLength, path: report["path"] }),
        201,
      );
      await expectStatus(await target.send(repo(target, `/uploads/${opened.upload_id}/parts/0`), { method: "PUT", body: content, token: producer.token }), 200);
      const finalize = () => target.send(repo(target, `/uploads/${opened.upload_id}/finalize`), { method: "POST", token: verifier.token });
      const finalized = await json<unknown>(await finalize(), 200);
      const objectPath = repo(target, `/objects/${idOf(observation)}/report/${encodeURIComponent(report["sha256"] as string)}`);
      await expectStatus(await target.send(objectPath, { token: reader.token }), 200);
      const objectWithdrawal = event({ kind: "object", upload_id: opened.upload_id }, "withdrawn");
      await expectStatus(await append(objectWithdrawal), 201);
      expect(await json<unknown>(await finalize(), 200)).toEqual(finalized);
      await expectStatus(await target.send(objectPath, { token: reader.token }), 404, "not_found");

      const detail = await json<{ locations: { event_id: string; kind: string }[] }>(
        await target.send(repo(target, observationPath(idOf(observation))), { token: reader.token }),
        200,
      );
      expect(detail.locations.map((location) => [location.event_id, location.kind]).sort()).toEqual(
        [
          [withdrawal.event_id, "withdrawn"],
          [objectWithdrawal.event_id, "withdrawn"],
        ].sort(),
      );
    }),
};

const changeFeedReplay: AcceptanceCase = {
  name: "replaying a change-feed cursor returns identical pages",
  run: (target) =>
    withCredentials(target, async ({ issue }) => {
      const producer = await issue(["producer"]);
      const reader = await issue(["reader"]);
      const start = await json<Snapshot>(await target.send(repo(target, "/snapshot"), { token: reader.token }), 200);
      const written = [await freshObservation(), await freshObservation(), await freshObservation()];
      for (const observation of written) await ingest(target, producer.token, hex(40), [observation]);

      const query = { stream: "observations", page_size: 2, after_sequence: start.observation_event_watermark };
      const first = await json<Page<ChangeItem>>(await post(target, reader.token, "/changes/query", query), 200);
      const pages = await follow(target, reader.token, "/changes/query", query, first);
      expect(pages.at(-1)!.has_more).toBe(false);
      const replayed = await follow(target, reader.token, "/changes/query", query, first);
      expect(replayed).toEqual(pages);
      const seen = pages.flatMap((page) => page.items.map((item) => item.observation_id));
      expect(seen).toEqual(expect.arrayContaining(written.map(idOf)));
    }),
};

const finitePaginationDuringWrites: AcceptanceCase = {
  name: "history and change-feed pagination stay inside the captured snapshot and terminate during concurrent writes",
  run: (target) =>
    withCredentials(target, async ({ issue }) => {
      const producer = await issue(["producer"]);
      const reader = await issue(["reader"]);
      const head = hex(40);
      const start = await json<Snapshot>(await target.send(repo(target, "/snapshot"), { token: reader.token }), 200);
      const original = [await freshObservation(), await freshObservation(), await freshObservation(), await freshObservation()];
      for (const observation of original) await ingest(target, producer.token, head, [observation]);

      const historyQuery = { ...GATE, reachable_heads: [head], page_size: 2 };
      const changesQuery = { stream: "observations", page_size: 1, after_sequence: start.observation_event_watermark };
      const firstHistory = await json<Page<HistoryItem>>(await post(target, reader.token, "/history/query", historyQuery), 200);
      const firstChanges = await json<Page<ChangeItem>>(await post(target, reader.token, "/changes/query", changesQuery), 200);
      const concurrent = [await freshObservation(), await freshObservation(), await freshObservation()];
      const [history, changes] = await Promise.all([
        follow(target, reader.token, "/history/query", historyQuery, firstHistory),
        follow(target, reader.token, "/changes/query", changesQuery, firstChanges),
        (async () => {
          for (const observation of concurrent) await ingest(target, producer.token, head, [observation]);
        })(),
      ]);

      for (const [pages, first] of [
        [history, firstHistory],
        [changes, firstChanges],
      ] as const) {
        expect(pages.at(-1)!.has_more).toBe(false);
        for (const page of pages) expect(page.snapshot).toEqual(first.snapshot);
      }
      const bound = (pages: Page<{ sequence: number }>[], first: Page<unknown>) =>
        pages.flatMap((page) => page.items).filter((item) => item.sequence > first.snapshot.observation_event_watermark);
      expect(bound(history, firstHistory)).toEqual([]);
      expect(bound(changes, firstChanges)).toEqual([]);
      expect(history.flatMap((page) => page.items.map((item) => item.observation.observation_id))).toEqual(original.map(idOf));
      const changed = changes.flatMap((page) => page.items.map((item) => item.observation_id));
      for (const observation of concurrent) expect(changed).not.toContain(idOf(observation));

      const fresh = await json<Page<HistoryItem>>(await post(target, reader.token, "/history/query", { ...GATE, reachable_heads: [head] }), 200);
      expect(new Set(fresh.items.map((item) => item.observation.observation_id))).toEqual(new Set([...original, ...concurrent].map(idOf)));
    }),
};

export const ACCEPTANCE_CASES: readonly AcceptanceCase[] = [
  concurrentMixedIdentityRollback,
  revokedAndUnauthorizedIdentities,
  expiredIdentity,
  interruptedUploadReplay,
  withdrawalWithoutRevival,
  changeFeedReplay,
  finitePaginationDuringWrites,
];
