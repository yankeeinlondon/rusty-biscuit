//! Host and operator bearer tokens: `ebt_<credential_id>.<secret>`.
//!
//! The credential ID is public (32 lowercase hex characters, 128 random bits);
//! the secret is 256 random bits as unpadded base64url (43 characters). Only
//! `secretDigest(secret)` is ever stored. Pure Web Crypto, so the Worker and
//! the Node operator CLI mint and digest tokens the same way.

export const TOKEN_PREFIX = "ebt_";

const TOKEN = /^ebt_([0-9a-f]{32})\.([A-Za-z0-9_-]{43})$/;

export interface HostToken {
  readonly credentialId: string;
  readonly secret: string;
}

export interface MintedToken extends HostToken {
  /** The full bearer token; shown once, never stored. */
  readonly token: string;
}

function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function base64url(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

/** 32 lowercase hex characters from 128 random bits (credential IDs, generations). */
export function randomId(): string {
  return hex(crypto.getRandomValues(new Uint8Array(16)));
}

export function mintToken(): MintedToken {
  const credentialId = randomId();
  const secret = base64url(crypto.getRandomValues(new Uint8Array(32)));
  return { credentialId, secret, token: `${TOKEN_PREFIX}${credentialId}.${secret}` };
}

/** The token's parts, or null when it is not exactly the host-token format. */
export function parseToken(token: string): HostToken | null {
  const match = TOKEN.exec(token);
  return match === null ? null : { credentialId: match[1]!, secret: match[2]! };
}

/** Lowercase hex SHA-256 of the secret's UTF-8 bytes: the stored `secret_sha256`. */
export async function secretDigest(secret: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(secret));
  return hex(new Uint8Array(digest));
}

/** Equal-length strings compared without an early exit on the first difference. */
export function constantTimeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let difference = 0;
  for (let index = 0; index < a.length; index += 1) difference |= a.charCodeAt(index) ^ b.charCodeAt(index);
  return difference === 0;
}
