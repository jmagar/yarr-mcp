/**
 * Typed client for the rustarr REST API.
 *
 * All actions are dispatched via POST /v1/rustarr with:
 *   { "action": "<action>", "params": { ... } }
 *
 * The base URL is relative (empty string) so the same binary serves
 * both the API and the web UI — no CORS or cross-origin config needed.
 */

import { endpoint, WEB_APP_CONFIG } from "@/lib/template";

export interface ApiResponse<T = unknown> {
  data?: T;
  error?: string;
}

export interface StatusResult {
  status: string;
  server: string;
  version: string;
  transport: string;
}

export interface HealthResult {
  status: string;
}

export interface Integration {
  name: string;
  kind: string;
  base_url_configured: boolean;
  api_key_configured: boolean;
  token_configured: boolean;
  username_configured: boolean;
  password_configured: boolean;
}

export interface IntegrationsResult {
  supported: string[];
  configured: Integration[];
}

/** Shared fetch helper — handles JSON parsing and error normalisation. */
export async function apiFetch<T>(url: string, options?: RequestInit): Promise<ApiResponse<T>> {
  try {
    const res = await fetch(url, options);
    const text = await res.text();
    const json = parseJsonBody(text);
    if (!res.ok) {
      const error =
        isRecord(json) && typeof json.error === "string" ? json.error : `HTTP ${res.status}`;
      return { error };
    }
    return { data: json as T };
  } catch (e) {
    return { error: e instanceof Error ? e.message : "Network error" };
  }
}

export function parseJsonBody(text: string): unknown {
  if (!text.trim()) return {};
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** POST /v1/rustarr — dispatch an action */
export function callAction<T = unknown>(
  action: string,
  params: Record<string, unknown> = {},
): Promise<ApiResponse<T>> {
  return apiFetch<T>(endpoint(WEB_APP_CONFIG.restEndpoint), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ action, params }),
  });
}

/** GET /health */
export function getHealth(): Promise<ApiResponse<HealthResult>> {
  return apiFetch<HealthResult>(endpoint(WEB_APP_CONFIG.healthEndpoint));
}

/** GET /status */
export function getStatus(): Promise<ApiResponse<StatusResult>> {
  return apiFetch<StatusResult>(endpoint(WEB_APP_CONFIG.statusEndpoint));
}

export const integrations = () => callAction<IntegrationsResult>("integrations");

export const help = () => callAction<Record<string, unknown>>("help");
