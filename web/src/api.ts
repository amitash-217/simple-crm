import type { Rec } from "./types";

export class ApiError extends Error {
  constructor(public status: number, public code: string, message: string) {
    super(message);
  }
}

export const errorMessage = (e: unknown): string => (e instanceof Error ? e.message : String(e));

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(path, {
    method,
    headers: body !== undefined ? { "content-type": "application/json" } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
    credentials: "same-origin",
  });
  const text = await res.text();
  let data: any = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    // A non-JSON body (a proxy error page, say) falls through to the status text below.
  }
  if (!res.ok) {
    throw new ApiError(res.status, data?.error?.code ?? "error", data?.error?.message ?? (res.statusText || "request failed"));
  }
  return data as T;
}

export const api = {
  get: <T>(path: string) => request<T>("GET", path),
  post: <T>(path: string, body?: unknown) => request<T>("POST", path, body ?? {}),
  patch: <T>(path: string, body: unknown) => request<T>("PATCH", path, body),
  del: <T>(path: string) => request<T>("DELETE", path),
};

export interface ListArgs {
  filter?: unknown;
  sort?: unknown;
  q?: string;
  limit?: number;
  cursor?: string | null;
}

export function listRecords(table: string, a: ListArgs): Promise<{ records: Rec[]; next_cursor: string | null }> {
  const p = new URLSearchParams();
  if (a.filter) p.set("filter", JSON.stringify(a.filter));
  if (Array.isArray(a.sort) && a.sort.length) p.set("sort", JSON.stringify(a.sort));
  if (a.q?.trim()) p.set("q", a.q.trim());
  if (a.limit) p.set("limit", String(a.limit));
  if (a.cursor) p.set("cursor", a.cursor);
  const qs = p.toString();
  return api.get(`/api/tables/${encodeURIComponent(table)}/records${qs ? `?${qs}` : ""}`);
}
