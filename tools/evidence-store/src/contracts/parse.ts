//! Public entry points from raw bytes to validated evidence, each answering
//! with located problems rather than throwing.

import { canonical, collectionIdentity } from "./canonical.ts";
import type { ContractResult, JsonObject, JsonValue } from "./json.ts";
import { loadStrict } from "./loader.ts";
import { validateCollection, validateReceipt } from "./validate.ts";

export interface ParsedCollection {
  readonly collection: JsonValue;
  readonly canonical: string;
  readonly collection_id: string;
  readonly observation_ids: readonly string[];
}

/** One version-3 collection: strict load, `validate_collection`, then identities. */
export function parseCollection(bytes: Uint8Array): ContractResult<ParsedCollection> {
  const loaded = loadStrict(bytes);
  if (!loaded.ok) return loaded;
  const problems = validateCollection(loaded.value);
  if (problems.length > 0) return { ok: false, problems };
  const collection = loaded.value as JsonObject;
  const observations = collection["observations"] as JsonObject[];
  return {
    ok: true,
    value: {
      collection,
      canonical: canonical(collection),
      collection_id: collectionIdentity(collection),
      observation_ids: observations.map((observation) => observation["observation_id"] as string),
    },
  };
}

/** One version-2 receipt: strict load, then `validate_receipt`. */
export function parseReceipt(bytes: Uint8Array): ContractResult<JsonValue> {
  const loaded = loadStrict(bytes);
  if (!loaded.ok) return loaded;
  const problems = validateReceipt(loaded.value);
  return problems.length > 0 ? { ok: false, problems } : loaded;
}
