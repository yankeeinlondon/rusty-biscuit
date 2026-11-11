// The `CredentialRequest` field shapes, walked once through the shared
// validator that both `issueCredential` and the CLI grant file use. Each
// consumer's wiring is covered by one case in its own test file.
import { describe, expect, it } from "vitest";
import { parseCredentialGrant, parseCredentialRequest } from "../../src/credential-request.ts";
import { ApiError } from "../../src/http.ts";

const encoder = new TextEncoder();
const valid = () => ({ label: "rig-linux-01", repository_ids: ["873451209", "873451210"], roles: ["producer", "reader"] }) as Record<string, unknown>;

function refusal(document: Record<string, unknown>, parse: (bytes: Uint8Array) => unknown = parseCredentialRequest) {
  try {
    parse(encoder.encode(JSON.stringify(document)));
  } catch (error) {
    if (!(error instanceof ApiError)) throw error;
    return { code: error.code, location: error.location };
  }
  throw new Error("expected the request to be refused");
}

describe("parseCredentialRequest", () => {
  it("accepts a request and defaults an absent expiry to 90 days", () => {
    expect(parseCredentialRequest(encoder.encode(JSON.stringify(valid())))).toEqual({ ...valid(), expires_in_days: 90 });
    const full = { ...valid(), expires_in_days: 7, replaces: "f".repeat(32) };
    expect(parseCredentialRequest(encoder.encode(JSON.stringify(full)))).toEqual(full);
  });

  it("accepts empty repository and role lists, which deny everything", () => {
    const empty = { label: "nothing", repository_ids: [], roles: [] };
    expect(parseCredentialRequest(encoder.encode(JSON.stringify(empty)))).toMatchObject(empty);
  });

  const cases: [string, Record<string, unknown>, string][] = [
    ["absent label", { label: undefined }, "/label"],
    ["null label", { label: null }, "/label"],
    ["numeric label", { label: 7 }, "/label"],
    ["empty label", { label: "" }, "/label"],
    ["label over 128 characters", { label: "x".repeat(129) }, "/label"],
    ["absent repository_ids", { repository_ids: undefined }, "/repository_ids"],
    ["null repository_ids", { repository_ids: null }, "/repository_ids"],
    ["scalar repository_ids", { repository_ids: "873451209" }, "/repository_ids"],
    ["one bad repository ID", { repository_ids: ["873451209", "0123"] }, "/repository_ids/1"],
    ["every repository ID bad", { repository_ids: [1, 2] }, "/repository_ids/0"],
    ["a repeated repository ID", { repository_ids: ["873451209", "873451209"] }, "/repository_ids/1"],
    ["absent roles", { roles: undefined }, "/roles"],
    ["null roles", { roles: null }, "/roles"],
    ["scalar roles", { roles: "reader" }, "/roles"],
    ["the operator role", { roles: ["reader", "operator"] }, "/roles/1"],
    ["every role bad", { roles: ["admin", "root"] }, "/roles/0"],
    ["a repeated role", { roles: ["reader", "reader"] }, "/roles/1"],
    ["null expires_in_days", { expires_in_days: null }, "/expires_in_days"],
    ["string expires_in_days", { expires_in_days: "90" }, "/expires_in_days"],
    ["fractional expires_in_days", { expires_in_days: 1.5 }, "/expires_in_days"],
    ["zero expires_in_days", { expires_in_days: 0 }, "/expires_in_days"],
    ["expires_in_days over 90", { expires_in_days: 91 }, "/expires_in_days"],
    ["null replaces", { replaces: null }, "/replaces"],
    ["malformed replaces", { replaces: "F".repeat(32) }, "/replaces"],
    ["an unknown field", { secret_file: "/tmp/token" }, "/secret_file"],
  ];
  for (const [name, edit, location] of cases) {
    it(`refuses ${name}, located at ${location}`, () => {
      expect(refusal({ ...valid(), ...edit })).toEqual({ code: "invalid_input", location });
    });
  }
});

describe("parseCredentialGrant", () => {
  const grant = () => ({ ...valid(), secret_file: "/home/ci/rig.token" });

  it("accepts a grant and keeps secret_file out of the request fields", () => {
    expect(parseCredentialGrant(encoder.encode(JSON.stringify(grant())))).toEqual({ ...valid(), expires_in_days: 90, secret_file: "/home/ci/rig.token" });
  });

  for (const [name, value] of [
    ["absent", undefined],
    ["null", null],
    ["empty", ""],
    ["numeric", 7],
  ] as const) {
    it(`refuses ${name} secret_file`, () => {
      expect(refusal({ ...grant(), secret_file: value }, parseCredentialGrant)).toEqual({ code: "invalid_input", location: "/secret_file" });
    });
  }
});
