export const WEB_APP_CONFIG = {
  serviceName: "rustarr",
  displayName: "rustarr",
  dashboardTitle: "Operator Dashboard",
  description: "MCP server operator dashboard",
  apiBaseUrl: process.env.NEXT_PUBLIC_RUSTARR_API_BASE_URL ?? "",
  restEndpoint: "/v1/rustarr",
  healthEndpoint: "/health",
  statusEndpoint: "/status",
  mcpEndpoint: "/mcp",
} as const;

export type ActionParam = {
  name: string;
  label: string;
  type: "text" | "textarea" | "checkbox";
  placeholder?: string;
  required: boolean;
  description: string;
};

export type ActionSpec = {
  id: string;
  label: string;
  description: string;
  scope: "rustarr:read" | "rustarr:write" | "public";
  transport: "rest" | "mcp-only";
  params: readonly ActionParam[];
  rustarr: {
    action: string;
    params: Record<string, unknown>;
  };
  response: Record<string, unknown>;
};

export const ACTIONS = [
  {
    id: "integrations",
    label: "integrations",
    description: "List supported and configured ARR/media services.",
    scope: "rustarr:read",
    transport: "rest",
    params: [],
    rustarr: { action: "integrations", params: {} },
    response: { supported: ["sonarr", "radarr"], configured: [] },
  },
  {
    id: "service_status",
    label: "service_status",
    description: "Fetch the service-specific status endpoint for one configured service.",
    scope: "rustarr:read",
    transport: "rest",
    params: [
      {
        name: "service",
        label: "Service",
        type: "text",
        placeholder: "sonarr",
        required: true,
        description: "Configured service name or service kind.",
      },
    ],
    rustarr: { action: "service_status", params: { service: "sonarr" } },
    response: { version: "4.0.0" },
  },
  {
    id: "api_get",
    label: "api_get",
    description: "Proxy a credentialed GET request to an allowed upstream API prefix.",
    scope: "rustarr:write",
    transport: "rest",
    params: [
      {
        name: "service",
        label: "Service",
        type: "text",
        placeholder: "sonarr",
        required: true,
        description: "Configured service name or service kind.",
      },
      {
        name: "path",
        label: "Path",
        type: "text",
        placeholder: "/api/v3/system/status",
        required: true,
        description: "Safe relative API path.",
      },
    ],
    rustarr: {
      action: "api_get",
      params: { service: "sonarr", path: "/api/v3/system/status" },
    },
    response: { version: "4.0.0" },
  },
  {
    id: "api_post",
    label: "api_post",
    description: "Proxy a confirmed credentialed POST request to an allowed upstream API prefix.",
    scope: "rustarr:write",
    transport: "rest",
    params: [
      {
        name: "service",
        label: "Service",
        type: "text",
        placeholder: "radarr",
        required: true,
        description: "Configured service name or service kind.",
      },
      {
        name: "path",
        label: "Path",
        type: "text",
        placeholder: "/api/v3/command",
        required: true,
        description: "Safe relative API path.",
      },
      {
        name: "body",
        label: "Body",
        type: "textarea",
        placeholder: '{"name":"RefreshMovie"}',
        required: true,
        description: "JSON request body.",
      },
      {
        name: "confirm",
        label: "Confirm",
        type: "checkbox",
        required: true,
        description: "Must be true because generic POST can mutate upstream services.",
      },
    ],
    rustarr: {
      action: "api_post",
      params: { service: "radarr", path: "/api/v3/command", body: {}, confirm: true },
    },
    response: { id: 123, name: "RefreshMovie" },
  },
  {
    id: "help",
    label: "help",
    description: "Show all available REST actions and usage documentation.",
    scope: "public",
    transport: "rest",
    params: [],
    rustarr: { action: "help", params: {} },
    response: {
      actions: ["integrations", "service_status", "api_get", "api_post", "help"],
      mcp_only_actions: [],
      usage: 'POST /v1/rustarr with {"action":"<action>","params":{...}}',
    },
  },
] as const satisfies readonly ActionSpec[];

export type RestAction = Extract<(typeof ACTIONS)[number], { transport: "rest" }>;
export type RestActionId = RestAction["id"];

export const REST_ACTIONS = ACTIONS.filter((action) => action.transport === "rest") as RestAction[];
export const DEFAULT_REST_ACTION = REST_ACTIONS[0];

export function normalizeApiBaseUrl(apiBaseUrl: string): string {
  return apiBaseUrl.replace(/\/+$/, "");
}

export function endpoint(path: string): string {
  return `${normalizeApiBaseUrl(WEB_APP_CONFIG.apiBaseUrl)}${path}`;
}
