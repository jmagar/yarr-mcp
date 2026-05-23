export const WEB_APP_CONFIG = {
  serviceName: "example",
  displayName: "rmcp-template",
  dashboardTitle: "Operator Dashboard",
  description: "MCP server operator dashboard",
  apiBaseUrl: process.env.NEXT_PUBLIC_EXAMPLE_API_BASE_URL ?? "",
  restEndpoint: "/v1/example",
  healthEndpoint: "/health",
  statusEndpoint: "/status",
  mcpEndpoint: "/mcp",
} as const;

export type ActionParam = {
  name: string;
  label: string;
  type: "text";
  placeholder?: string;
  required: boolean;
  description: string;
};

export type ActionSpec = {
  id: string;
  label: string;
  description: string;
  scope: "example:read" | "example:write" | "public";
  transport: "rest" | "mcp-only";
  params: readonly ActionParam[];
  example: {
    action: string;
    params: Record<string, unknown>;
  };
  response: Record<string, unknown>;
};

export const ACTIONS = [
  {
    id: "greet",
    label: "greet",
    description: "Return a personalized greeting for the given name.",
    scope: "example:read",
    transport: "rest",
    params: [
      {
        name: "name",
        label: "Name",
        type: "text",
        placeholder: "Alice",
        required: false,
        description: "Name to greet. Defaults to World when omitted.",
      },
    ],
    example: { action: "greet", params: { name: "Alice" } },
    response: { greeting: "Hello, Alice!", target: "Alice" },
  },
  {
    id: "echo",
    label: "echo",
    description: "Echo a message back unchanged.",
    scope: "example:read",
    transport: "rest",
    params: [
      {
        name: "message",
        label: "Message",
        type: "text",
        placeholder: "Hello!",
        required: true,
        description: "Message to echo.",
      },
    ],
    example: { action: "echo", params: { message: "Hello!" } },
    response: { echo: "Hello!" },
  },
  {
    id: "status",
    label: "status",
    description: "Return server status and configuration info.",
    scope: "example:read",
    transport: "rest",
    params: [],
    example: { action: "status", params: {} },
    response: { status: "ok", api_url: "http://...", note: "stub" },
  },
  {
    id: "help",
    label: "help",
    description: "Show all available REST actions and usage documentation.",
    scope: "public",
    transport: "rest",
    params: [],
    example: { action: "help", params: {} },
    response: {
      actions: ["greet", "echo", "status", "help"],
      mcp_only_actions: ["elicit_name", "scaffold_intent"],
      usage: 'POST /v1/example with {"action":"<action>","params":{...}}',
    },
  },
  {
    id: "elicit_name",
    label: "elicit_name",
    description: "MCP elicitation demo that asks the user for a name mid-call.",
    scope: "example:read",
    transport: "mcp-only",
    params: [],
    example: { action: "elicit_name", params: {} },
    response: { greeting: "Hello, Alice!", target: "Alice", elicited: true },
  },
  {
    id: "scaffold_intent",
    label: "scaffold_intent",
    description:
      "MCP elicitation setup wizard that returns scaffold intent JSON for the scaffold-project skill.",
    scope: "example:read",
    transport: "mcp-only",
    params: [],
    example: { action: "scaffold_intent", params: {} },
    response: {
      kind: "rmcp_template_scaffold_intent",
      schema_version: 1,
      server_category: "upstream-client",
      required_surfaces: ["mcp", "cli"],
      project: {
        display_name: "Unraid MCP",
        crate_name: "unraid-mcp",
        binary_name: "unraid",
        service_name: "unraid",
        env_prefix: "UNRAID",
      },
      upstream: { base_url_env: "UNRAID_API_URL", auth_kind: "api-key" },
      runtime: { host: "127.0.0.1", port: 3100, mcp_transport: "dual" },
      mcp_primitives: ["tools", "resources", "prompts", "elicitation"],
      deployment: "none",
      plugins: ["claude", "codex"],
      publish_mcp: true,
      crawl_docs: {
        urls: ["https://docs.unraid.net/"],
        repos: [],
        search_topics: ["Unraid API authentication"],
      },
      handoff: { recommended_skill: "scaffold-project" },
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
