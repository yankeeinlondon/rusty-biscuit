import { readFileSync, readdirSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";
import {
  API_PREFIX,
  ERROR_CODES,
  LIMITS,
  OPERATIONS,
  OPERATOR_PATH_PREFIX,
} from "../../src/contracts/api.ts";

const METHODS = ["get", "put", "post", "delete", "patch", "head", "options", "trace"] as const;

interface DocumentedOperation {
  method: string;
  path: string;
  operationId: unknown;
  roles: unknown;
}

function documentedOperations(document: Record<string, unknown>): DocumentedOperation[] {
  const paths = document["paths"] as Record<string, Record<string, Record<string, unknown>>>;
  const found: DocumentedOperation[] = [];
  for (const [path, item] of Object.entries(paths)) {
    for (const method of METHODS) {
      const operation = item[method];
      if (operation !== undefined) {
        found.push({
          method: method.toUpperCase(),
          path,
          operationId: operation["operationId"],
          roles: operation["x-roles"],
        });
      }
    }
  }
  return found;
}

function expectedPath(path: string): string {
  return path.startsWith("/operator/") ? `${OPERATOR_PATH_PREFIX}${path}` : `${API_PREFIX}${path}`;
}

function loadOpenApi(text: string): Record<string, unknown> {
  return parse(text, { uniqueKeys: true }) as Record<string, unknown>;
}

describe("api/openapi.yaml", () => {
  it("documents exactly the OPERATIONS table with matching method, path, and roles", () => {
    const document = loadOpenApi(readFileSync(new URL("../../api/openapi.yaml", import.meta.url), "utf8"));
    const documented = documentedOperations(document);
    const byId = new Map(documented.map((operation) => [operation.operationId, operation]));

    expect(byId.size).toBe(documented.length);
    for (const [operationId, operation] of Object.entries(OPERATIONS)) {
      const found = byId.get(operationId);
      expect(found, operationId).toBeDefined();
      expect(found?.method, operationId).toBe(operation.method);
      expect(found?.path, operationId).toBe(expectedPath(operation.path));
      expect(found?.roles, operationId).toEqual([...operation.roles]);
    }
    expect([...byId.keys()].sort()).toEqual(Object.keys(OPERATIONS).sort());
  });

  it("enumerates exactly the ERROR_CODES in the shared ErrorBody", () => {
    const document = loadOpenApi(readFileSync(new URL("../../api/openapi.yaml", import.meta.url), "utf8"));
    const schemas = (document["components"] as Record<string, Record<string, Record<string, unknown>>>)["schemas"];
    const errorCode = schemas?.["ErrorCode"] as { enum: string[] };
    const errorBody = schemas?.["ErrorBody"] as {
      properties: { error: { properties: { code: { $ref: string } } } };
    };

    expect(errorBody.properties.error.properties.code.$ref).toBe("#/components/schemas/ErrorCode");
    expect([...errorCode.enum].sort()).toEqual(Object.keys(ERROR_CODES).sort());
  });

  it("publishes x-limits equal to LIMITS", () => {
    const document = loadOpenApi(readFileSync(new URL("../../api/openapi.yaml", import.meta.url), "utf8"));

    expect(document["x-limits"]).toEqual(LIMITS);
  });
});

describe("api/examples", () => {
  it("parses every example and names an existing operation", () => {
    const directory = new URL("../../api/examples/", import.meta.url);
    const names = readdirSync(directory).filter((name) => name.endsWith(".json"));

    expect(names.length).toBeGreaterThan(0);
    for (const name of names) {
      expect(() => JSON.parse(readFileSync(new URL(name, directory), "utf8")), name).not.toThrow();
      if (name.startsWith("error.")) {
        continue;
      }
      const match = /^([A-Za-z]+)\.(request|response)\.json$/.exec(name);
      expect(match, name).not.toBeNull();
      expect(Object.keys(OPERATIONS), name).toContain(match?.[1]);
    }
  });
});

describe("api/schemas", () => {
  it("parses every configuration schema and closes its root", () => {
    const directory = new URL("../../api/schemas/", import.meta.url);
    const names = readdirSync(directory).filter((name) => name.endsWith(".schema.json"));

    expect(names.length).toBeGreaterThan(0);
    for (const name of names) {
      const schema = JSON.parse(readFileSync(new URL(name, directory), "utf8")) as Record<string, unknown>;
      expect(schema["$schema"], name).toBe("https://json-schema.org/draft/2020-12/schema");
      expect(schema["additionalProperties"], name).toBe(false);
    }
  });
});
