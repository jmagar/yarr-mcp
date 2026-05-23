import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { ACTIONS, normalizeApiBaseUrl, REST_ACTIONS } from "./template";

type OpenApiActionMetadata = {
  components: {
    schemas: {
      ActionName: {
        enum: string[];
      };
    };
  };
  "x-template": {
    rest_actions: string[];
    mcp_only_actions: string[];
  };
};

const here = dirname(fileURLToPath(import.meta.url));
const openApi = JSON.parse(
  readFileSync(resolve(here, "../../../docs/generated/openapi.json"), "utf8"),
) as OpenApiActionMetadata;

describe("template action metadata", () => {
  it("keeps REST actions aligned with generated OpenAPI metadata", () => {
    const webRestActions = REST_ACTIONS.map((action) => action.id);
    expect(webRestActions).toEqual(openApi.components.schemas.ActionName.enum);
    expect(webRestActions).toEqual(openApi["x-template"].rest_actions);
  });

  it("keeps MCP-only actions aligned with generated OpenAPI metadata", () => {
    const webMcpOnlyActions = ACTIONS.filter((action) => action.transport === "mcp-only").map(
      (action) => action.id,
    );
    expect(webMcpOnlyActions).toEqual(openApi["x-template"].mcp_only_actions);
  });

  it("does not duplicate action identifiers", () => {
    const ids = ACTIONS.map((action) => action.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});

describe("normalizeApiBaseUrl", () => {
  it("removes one or more trailing slashes", () => {
    expect(normalizeApiBaseUrl("http://localhost:3100/")).toBe("http://localhost:3100");
    expect(normalizeApiBaseUrl("http://localhost:3100///")).toBe("http://localhost:3100");
  });

  it("preserves empty same-origin configuration", () => {
    expect(normalizeApiBaseUrl("")).toBe("");
  });
});
