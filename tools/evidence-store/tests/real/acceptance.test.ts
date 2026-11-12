// The deployed acceptance suite (`just test-real`): the same cases as
// `tests/service/acceptance.test.ts`, sent over HTTP to a deployed Worker.
// A missing prerequisite fails every case, so it can never read as a pass or
// a skip.
import { describe, it } from "vitest";
import { missingEnvironment, readTokenFile } from "../../cli/operator.ts";
import { ACCEPTANCE_CASES, type AcceptanceTarget } from "../acceptance/cases.ts";

const REQUIRED = ["EVIDENCE_STORE_ENDPOINT", "EVIDENCE_STORE_OPERATOR_TOKEN_FILE", "EVIDENCE_STORE_ACCEPTANCE_REPO"] as const;
const MAX_ATTEMPTS = 5;
const MAX_WAIT_MS = 60_000;
const REQUEST_TIMEOUT_MS = 60_000;

let target: AcceptanceTarget | undefined;

function deployed(): AcceptanceTarget {
  if (target !== undefined) return target;
  const missing = missingEnvironment(process.env, REQUIRED);
  if (missing.length > 0) throw new Error(`the deployed acceptance suite needs ${missing.join(", ")}; nothing was run`);
  const endpoint = process.env["EVIDENCE_STORE_ENDPOINT"]!.replace(/\/+$/, "");
  target = {
    repositoryId: process.env["EVIDENCE_STORE_ACCEPTANCE_REPO"]!,
    operatorToken: readTokenFile(process.env["EVIDENCE_STORE_OPERATOR_TOKEN_FILE"]!),
    // Retries only 429 and 503, which record nothing; the operator's 30
    // mutations per minute is otherwise reached by issuing and revoking each
    // case's disposable credentials.
    async send(path, init) {
      for (let attempt = 1; ; attempt += 1) {
        const response = await fetch(`${endpoint}${path}`, {
          method: init.method ?? "GET",
          headers: { authorization: `Bearer ${init.token}` },
          signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
          ...(init.body === undefined ? {} : { body: init.body }),
        });
        if ((response.status !== 429 && response.status !== 503) || attempt === MAX_ATTEMPTS) return response;
        const envelope = (await response.json().catch(() => ({}))) as { error?: { retry_after_ms?: number | null } };
        const wait = Math.min(envelope.error?.retry_after_ms ?? 1_000 * 2 ** attempt, MAX_WAIT_MS);
        await new Promise((resolve) => setTimeout(resolve, wait));
      }
    },
  };
  return target;
}

describe("deployed acceptance cases", () => {
  for (const acceptance of ACCEPTANCE_CASES) {
    if (acceptance.requires === "clock") {
      // Reported as skipped, never as passed: a deployment's clock cannot be moved.
      it.skip(`${acceptance.name} (not runnable deployed: needs a movable clock; covered in process)`, () => {});
    } else {
      it(acceptance.name, () => acceptance.run(deployed()));
    }
  }
});
