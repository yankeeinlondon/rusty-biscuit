// The field-shape matrix of the shared request readers (`src/requests.ts`),
// walked once per reader, plus the strict loader's document shapes once
// through `readRequestObject`. Route tests then need only one wiring case per
// field.
import { describe, expect, it } from "vitest";
import { loadStrict, type JsonObject } from "../../src/contracts/index.ts";
import { ApiError } from "../../src/http.ts";
import {
  optionalInteger,
  optionalObject,
  optionalString,
  optionalStringArray,
  readRequestObject,
  requiredInteger,
  requiredObject,
  requiredString,
  requiredStringArray,
} from "../../src/requests.ts";

const encode = (text: string) => new TextEncoder().encode(text);

function body(text: string): JsonObject {
  const loaded = loadStrict(encode(text));
  if (!loaded.ok) throw new Error(text);
  return loaded.value as JsonObject;
}

function refusal(read: () => unknown): { code: string; location: string | null; message: string } {
  try {
    read();
  } catch (error) {
    if (!(error instanceof ApiError)) throw error;
    return { code: error.code, location: error.location, message: error.message };
  }
  throw new Error("expected the reader to refuse");
}

function expectRefused(read: () => unknown, code: string, location: string) {
  expect(refusal(read)).toMatchObject({ code, location });
}

describe("readRequestObject", () => {
  it("returns an object whose keys are all allowed and refuses an unknown key at its pointer", () => {
    expect(readRequestObject(encode('{"a":1}'), ["a", "b"])).toHaveProperty("a");
    expectRefused(() => readRequestObject(encode('{"a":1,"c/d":2}'), ["a"]), "invalid_input", "/c~1d");
  });

  it("refuses every loader document shape through the shared strict loader", () => {
    const cases: [Uint8Array, string, string][] = [
      [encode('{"a":1,"a":2}'), "duplicate-key", "/a"],
      [encode('{"a":1} {"a":2}'), "trailing-content", ""],
      [new Uint8Array([0xef, 0xbb, 0xbf, ...encode('{"a":1}')]), "byte-order-mark", ""],
      [encode("  \n"), "empty-document", ""],
      [new Uint8Array(), "empty-document", ""],
      [new Uint8Array([0x7b, 0xff, 0x7d]), "invalid-utf8", ""],
      [encode('{"a":NaN}'), "nonfinite-number", "/a"],
    ];
    expect(
      cases.map(([bytes]) => {
        const refused = refusal(() => readRequestObject(bytes, ["a"]));
        return [refused.message.split(":")[0], refused.location, refused.code];
      }),
    ).toEqual(cases.map(([, loaderCode, location]) => [loaderCode, location, "invalid_input"]));
  });

  it("refuses a document that is not an object at the root pointer", () => {
    for (const text of ["[]", '"text"', "3", "null", "true"]) {
      expectRefused(() => readRequestObject(encode(text), []), "invalid_input", "");
    }
  });
});

describe("string readers", () => {
  const spec = { oneOf: ["x", "yy", "zzz"], maxLength: 2 };

  it("reads a valid value and treats an absent optional as undefined", () => {
    expect(requiredString(body('{"s":"x"}'), "s", "", spec)).toBe("x");
    expect(optionalString(body("{}"), "s", "", spec)).toBeUndefined();
    expect(requiredString(body('{"s":"ab"}'), "s", "/outer", { pattern: /^[a-z]+$/ })).toBe("ab");
  });

  it("refuses absent-required, null, wrong type, empty, out-of-range, and pattern misses at the field pointer", () => {
    expectRefused(() => requiredString(body("{}"), "s", ""), "invalid_input", "/s");
    for (const read of [requiredString, optionalString]) {
      expectRefused(() => read(body('{"s":null}'), "s", ""), "invalid_input", "/s");
      for (const value of ["3", "true", "[]", "{}"]) {
        expectRefused(() => read(body(`{"s":${value}}`), "s", ""), "invalid_input", "/s");
      }
      expectRefused(() => read(body('{"s":""}'), "s", ""), "invalid_input", "/s");
      expectRefused(() => read(body('{"s":"zzz"}'), "s", "", spec), "invalid_input", "/s");
      expectRefused(() => read(body('{"s":"q"}'), "s", "", spec), "invalid_input", "/s");
      expectRefused(() => read(body('{"s":"A1"}'), "s", "/outer", { pattern: /^[a-z]+$/ }), "invalid_input", "/outer/s");
    }
  });
});

describe("integer readers", () => {
  const range = { min: 1, max: 256 };

  it("reads an integer token in range and treats an absent optional as undefined", () => {
    expect(requiredInteger(body('{"n":256}'), "n", "", range)).toBe(256);
    expect(optionalInteger(body('{"n":1}'), "n", "", range)).toBe(1);
    expect(optionalInteger(body("{}"), "n", "", range)).toBeUndefined();
  });

  it("refuses absent-required, null, non-integer tokens, and out-of-range values", () => {
    expectRefused(() => requiredInteger(body("{}"), "n", "", range), "invalid_input", "/n");
    for (const read of [requiredInteger, optionalInteger]) {
      for (const value of ["null", "3.0", "3e0", "true", '"3"', "[3]", "{}", "0", "257", "-1", "99999999999999999999"]) {
        expectRefused(() => read(body(`{"n":${value}}`), "n", "", range), "invalid_input", "/n");
      }
    }
  });
});

describe("string-array readers", () => {
  const element = { pattern: /^[a-f]+$/ };
  const bounds = { maxItems: 3, unique: true };

  it("reads a valid array, an empty array, and an absent optional", () => {
    expect(requiredStringArray(body('{"a":["ab","cd"]}'), "a", "", element, bounds)).toEqual(["ab", "cd"]);
    expect(requiredStringArray(body('{"a":[]}'), "a", "", element, bounds)).toEqual([]);
    expect(optionalStringArray(body("{}"), "a", "", element, bounds)).toBeUndefined();
  });

  it("refuses the whole array for one bad element and locates the first bad element", () => {
    expectRefused(() => requiredStringArray(body("{}"), "a", "", element, bounds), "invalid_input", "/a");
    for (const read of [requiredStringArray, optionalStringArray]) {
      expectRefused(() => read(body('{"a":null}'), "a", "", element, bounds), "invalid_input", "/a");
      expectRefused(() => read(body('{"a":"ab"}'), "a", "", element, bounds), "invalid_input", "/a");
      expectRefused(() => read(body('{"a":["ab",3]}'), "a", "", element, bounds), "invalid_input", "/a/1");
      expectRefused(() => read(body('{"a":["zz",null]}'), "a", "", element, bounds), "invalid_input", "/a/0");
      expectRefused(() => read(body('{"a":["ab",""]}'), "a", "", element, bounds), "invalid_input", "/a/1");
      expectRefused(() => read(body('{"a":["ab","cd","ab"]}'), "a", "", element, bounds), "invalid_input", "/a/2");
      expectRefused(() => read(body('{"a":["a","b","c","d"]}'), "a", "", element, bounds), "size_limit", "/a");
    }
    // Repeats are allowed unless `unique` is asked for.
    expect(requiredStringArray(body('{"a":["ab","ab"]}'), "a", "", element)).toEqual(["ab", "ab"]);
  });
});

describe("object readers", () => {
  it("reads an object with allowed keys, an empty object, and an absent optional", () => {
    expect(requiredObject(body('{"o":{"k":1}}'), "o", "", ["k"])).toHaveProperty("k");
    expect(optionalObject(body('{"o":{}}'), "o", "", ["k"])).toEqual({});
    expect(optionalObject(body("{}"), "o", "", ["k"])).toBeUndefined();
  });

  it("refuses absent-required, null, wrong type, and an unknown nested key at its pointer", () => {
    expectRefused(() => requiredObject(body("{}"), "o", "", ["k"]), "invalid_input", "/o");
    for (const read of [requiredObject, optionalObject]) {
      for (const value of ["null", "[]", '"o"', "1", "false"]) {
        expectRefused(() => read(body(`{"o":${value}}`), "o", "", ["k"]), "invalid_input", "/o");
      }
      expectRefused(() => read(body('{"o":{"k":1,"x":2}}'), "o", "/p", ["k"]), "invalid_input", "/p/o/x");
    }
  });
});
