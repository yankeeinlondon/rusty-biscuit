// The hosted-demo dispatch in process with a fake fetch and spawn.
import { describe, expect, it } from "vitest";
import { run, slugFromRemote } from "../../cli/dispatch-demo.ts";
import type { OperatorIo } from "../../cli/operator.ts";

interface Call {
  url: string;
  init: RequestInit | undefined;
}

function fakeIo(env: NodeJS.ProcessEnv, answer: (call: Call) => Response, remote = "git@github.com:example-owner/rusty-biscuit.git\n") {
  const calls: Call[] = [];
  const spawns: { command: string; args: readonly string[] }[] = [];
  const out: string[] = [];
  const err: string[] = [];
  const io: OperatorIo = {
    env,
    fetch: (async (url: string, init?: RequestInit) => {
      const call = { url: String(url), init };
      calls.push(call);
      return answer(call);
    }) as typeof fetch,
    spawn: (command, args) => {
      spawns.push({ command, args });
      return { status: 0, stdout: remote, stderr: "" };
    },
    stdout: (text) => out.push(text),
    stderr: (text) => err.push(text),
    platform: process.platform,
    now: () => 0,
  };
  return { io, calls, spawns, stdout: () => out.join(""), stderr: () => err.join("") };
}

const ARGS = ["--run-id", "19012345678", "--artifact", "evidence-linux", "--collection", "evidence/collection.json"];
const API = "https://api.github.com/repos/example-owner/rusty-biscuit";

describe("demo-hosted", () => {
  it("exits 2 without a GitHub token and makes no request", async () => {
    const fake = fakeIo({}, () => new Response(null, { status: 500 }));
    expect(await run(ARGS, fake.io)).toBe(2);
    expect(fake.stderr()).toContain("set GITHUB_TOKEN or GH_TOKEN");
    expect(fake.calls).toEqual([]);
  });

  it("reports a workflow missing from main as a failed external prerequisite and never dispatches", async () => {
    const fake = fakeIo({ GH_TOKEN: "gh-token" }, () => new Response(JSON.stringify({ message: "Not Found" }), { status: 404 }));
    expect(await run(ARGS, fake.io)).toBe(2);
    expect(fake.stderr()).toContain("external prerequisite unavailable: the workflow is not on main of example-owner/rusty-biscuit");
    expect(fake.calls.map((call) => call.url)).toEqual([`${API}/contents/.github/workflows/evidence-store-demo.yml?ref=main`]);
    expect(fake.spawns).toEqual([{ command: "git", args: ["remote", "get-url", "origin"] }]);
  });

  it("dispatches on main with the inputs once the workflow is there", async () => {
    const fake = fakeIo({ GITHUB_TOKEN: "gh-token" }, (call) =>
      call.init?.method === "POST" ? new Response(null, { status: 204 }) : new Response(JSON.stringify({ name: "evidence-store-demo.yml" }), { status: 200 }),
    );
    expect(await run([...ARGS, "--proof-dir", "evidence/proofs", "--repo", "example-owner/rusty-biscuit"], fake.io)).toBe(0);

    expect(fake.spawns).toEqual([]);
    expect(fake.calls).toHaveLength(2);
    const dispatch = fake.calls[1]!;
    expect(dispatch.url).toBe(`${API}/actions/workflows/evidence-store-demo.yml/dispatches`);
    expect(dispatch.init?.method).toBe("POST");
    expect((dispatch.init?.headers as Record<string, string>)["authorization"]).toBe("Bearer gh-token");
    expect(JSON.parse(String(dispatch.init?.body))).toEqual({
      ref: "main",
      inputs: { run_id: "19012345678", artifact: "evidence-linux", collection: "evidence/collection.json", proof_dir: "evidence/proofs" },
    });
    expect(fake.stdout()).not.toContain("gh-token");
  });

  it("rejects a collection path that leaves the artifact before any request", async () => {
    const fake = fakeIo({ GH_TOKEN: "gh-token" }, () => new Response(null, { status: 200 }));
    expect(await run(["--run-id", "1", "--artifact", "x", "--collection", "../escape.json"], fake.io)).toBe(2);
    expect(fake.calls).toEqual([]);
  });

  it("reads owner/name from the usual GitHub remote spellings", () => {
    expect(slugFromRemote("git@github.com:o/r.git\n")).toBe("o/r");
    expect(slugFromRemote("https://github.com/o/r")).toBe("o/r");
    expect(slugFromRemote("ssh://git@github.com/o/r.git")).toBe("o/r");
    expect(slugFromRemote("https://gitlab.com/o/r.git")).toBeNull();
  });
});
