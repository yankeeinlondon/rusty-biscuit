//! Response envelopes shared by every route.
//!
//! Every non-2xx answer is an `ErrorBody` whose status and `retryable` flag
//! come from `ERROR_CODES`, so a handler can never pair a code with the wrong
//! status. Messages must never carry credentials or report bytes.

import { ERROR_CODES, type ErrorBody, type ErrorCode } from "./contracts/api.ts";

export class ApiError extends Error {
  constructor(
    readonly code: ErrorCode,
    message: string,
    readonly location: string | null = null,
    readonly retryAfterMs: number | null = null,
  ) {
    super(message);
  }
}

export function jsonResponse(body: unknown, status = 200, requestId?: string): Response {
  const headers: Record<string, string> = { "content-type": "application/json" };
  if (requestId !== undefined) headers["x-request-id"] = requestId;
  return new Response(JSON.stringify(body), { status, headers });
}

/**
 * The body is already serialized (canonical evidence text is spliced in
 * verbatim so number lexemes survive), so it is passed through untouched.
 */
export function rawJsonResponse(body: string, requestId: string, status = 200): Response {
  return new Response(body, {
    status,
    headers: { "content-type": "application/json", "x-request-id": requestId },
  });
}

export function errorResponse(error: ApiError, requestId: string): Response {
  const spec = ERROR_CODES[error.code];
  const body: ErrorBody = {
    error: {
      code: error.code,
      message: error.message,
      location: error.location,
      retryable: spec.retryable,
      retry_after_ms: error.retryAfterMs,
    },
    request_id: requestId,
  };
  const response = jsonResponse(body, spec.status, requestId);
  if (error.retryAfterMs !== null) {
    response.headers.set("retry-after", String(Math.ceil(error.retryAfterMs / 1000)));
  }
  return response;
}

/** Reads at most `limit + 1` bytes so an oversized body is refused without buffering it all. */
export async function readBody(request: Request, limit: number): Promise<Uint8Array> {
  const declared = request.headers.get("content-length");
  const tooLarge = () => new ApiError("size_limit", `request body exceeds ${limit} bytes`, "");
  if (declared !== null && Number(declared) > limit) throw tooLarge();
  if (request.body === null) return new Uint8Array();
  const chunks: Uint8Array[] = [];
  let total = 0;
  const reader = request.body.getReader();
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > limit) {
      await reader.cancel();
      throw tooLarge();
    }
    chunks.push(value);
  }
  const body = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    body.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return body;
}
