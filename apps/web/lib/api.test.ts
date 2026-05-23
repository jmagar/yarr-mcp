import { afterEach, describe, expect, it, vi } from "vitest";
import { apiFetch, parseJsonBody } from "./api";

describe("apiFetch", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns parsed JSON for successful responses", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify({ status: "ok" }), { status: 200 })),
    );

    await expect(apiFetch<{ status: string }>("/health")).resolves.toEqual({
      data: { status: "ok" },
    });
  });

  it("uses structured API error messages when available", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify({ error: "forbidden" }), { status: 403 })),
    );

    await expect(apiFetch("/v1/example")).resolves.toEqual({ error: "forbidden" });
  });

  it("preserves HTTP status when an error body has no error field", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("Bad gateway", { status: 502 })),
    );

    await expect(apiFetch("/v1/example")).resolves.toEqual({ error: "HTTP 502" });
  });

  it("normalizes thrown fetch failures", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new Error("connection refused");
      }),
    );

    await expect(apiFetch("/health")).resolves.toEqual({ error: "connection refused" });
  });
});

describe("parseJsonBody", () => {
  it("handles empty bodies", () => {
    expect(parseJsonBody("")).toEqual({});
    expect(parseJsonBody("   ")).toEqual({});
  });

  it("returns non-JSON text unchanged", () => {
    expect(parseJsonBody("not json")).toBe("not json");
  });
});
