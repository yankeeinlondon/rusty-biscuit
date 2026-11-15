//! Opaque, HMAC-signed page cursors.
//!
//! A cursor binds the repository, the restore generation, the retention
//! epoch, a digest of the query it continues, the position after which the
//! next page starts, and (for snapshot-bounded reads) the captured snapshot
//! bounds. Any edit, a cursor minted for another query or repository, or one
//! minted before a restore is refused rather than silently restarting the
//! listing.

import { LIMITS } from "./contracts/api.ts";
import { ApiError } from "./http.ts";

/** The part of a `Snapshot` that bounds every later page of a read. */
export interface SnapshotBounds {
  readonly evidence_revision: number;
  readonly observation_event_watermark: number;
  readonly location_event_watermark: number;
}

export interface CursorState {
  readonly repository_id: string;
  readonly generation: string;
  readonly retention_epoch: number;
  readonly query_digest: string;
  readonly position: number;
  readonly issued_at_ms: number;
  /**
   * Set by snapshot-bounded reads (history and change feeds), so a later page
   * never reads past what the first page captured. Signed with the rest.
   */
  readonly bounds?: SnapshotBounds;
}

const encoder = new TextEncoder();

function base64url(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

function fromBase64url(text: string): Uint8Array | null {
  if (!/^[A-Za-z0-9_-]*$/.test(text)) return null;
  try {
    const binary = atob(text.replaceAll("-", "+").replaceAll("_", "/"));
    return Uint8Array.from(binary, (char) => char.charCodeAt(0));
  } catch {
    return null;
  }
}

async function hmacKey(secret: string): Promise<CryptoKey> {
  if (secret.length === 0) throw new ApiError("unavailable", "cursor signing key is not configured");
  return crypto.subtle.importKey("raw", encoder.encode(secret), { name: "HMAC", hash: "SHA-256" }, false, [
    "sign",
    "verify",
  ]);
}

export async function queryDigest(parts: readonly (string | number)[]): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", encoder.encode(JSON.stringify(parts)));
  return base64url(new Uint8Array(digest));
}

export async function signCursor(secret: string, state: CursorState): Promise<string> {
  const payload = encoder.encode(JSON.stringify(state));
  const mac = await crypto.subtle.sign("HMAC", await hmacKey(secret), payload);
  return `${base64url(payload)}.${base64url(new Uint8Array(mac))}`;
}

/**
 * Verify `cursor` against what the current request expects. `location` is
 * the request position the cursor came from, reported on every refusal.
 */
export async function openCursor(
  secret: string,
  cursor: string,
  expected: Omit<CursorState, "position" | "issued_at_ms" | "bounds">,
  nowMs: number,
  location: string,
): Promise<CursorState> {
  const invalid = (why: string) => new ApiError("invalid_cursor", `cursor ${why}`, location);
  const [payloadText, macText, extra] = cursor.split(".");
  if (payloadText === undefined || macText === undefined || extra !== undefined) throw invalid("is malformed");
  const payload = fromBase64url(payloadText);
  const mac = fromBase64url(macText);
  if (payload === null || mac === null) throw invalid("is malformed");
  if (!(await crypto.subtle.verify("HMAC", await hmacKey(secret), mac, payload))) {
    throw invalid("signature does not verify");
  }
  // The signature proves this service minted the payload, so its shape is trusted.
  const state = JSON.parse(new TextDecoder().decode(payload)) as CursorState;
  if (state.repository_id !== expected.repository_id) throw invalid("belongs to another repository");
  if (state.query_digest !== expected.query_digest) throw invalid("belongs to another query");
  if (state.generation !== expected.generation) throw invalid("predates a store restore");
  if (nowMs - state.issued_at_ms > LIMITS.cursor_expiry_hours * 3_600_000) {
    throw new ApiError("cursor_expired", "cursor has expired; start again from a fresh snapshot", location);
  }
  if (state.retention_epoch !== expected.retention_epoch) {
    throw new ApiError("snapshot_pruned", "evidence was pruned after this cursor was issued", location);
  }
  return state;
}
