"use client";

import { ActionCard } from "@/components/api/action-card";
import { EndpointRow } from "@/components/api/endpoint-row";
import { ACTIONS, WEB_APP_CONFIG } from "@/lib/template";

export default function ApiPage() {
  return (
    <div className="max-w-4xl mx-auto space-y-6">
      {/* Header */}
      <div>
        <h1
          style={{
            fontFamily: "var(--aurora-font-display)",
            fontSize: "1.75rem",
            fontWeight: 700,
            marginBottom: "0.25rem",
          }}
        >
          API Explorer
        </h1>
        <p style={{ color: "var(--aurora-text-muted)", fontSize: "0.875rem" }}>
          All surfaces (MCP, REST, CLI) call the same service methods.
        </p>
      </div>

      {/* Endpoint overview */}
      <div
        style={{
          background: "var(--aurora-panel-medium)",
          border: "1px solid var(--aurora-border-default)",
          borderRadius: "var(--radius-lg)",
          padding: "1.25rem",
        }}
      >
        <h2
          style={{
            color: "var(--aurora-text-muted)",
            fontSize: "0.75rem",
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.05em",
            marginBottom: "1rem",
          }}
        >
          Endpoint
        </h2>
        <div className="space-y-2">
          <EndpointRow
            method="POST"
            path={WEB_APP_CONFIG.restEndpoint}
            description="REST action dispatch"
          />
          <EndpointRow
            method="GET"
            path={WEB_APP_CONFIG.healthEndpoint}
            description="Liveness probe (unauthenticated)"
          />
          <EndpointRow
            method="GET"
            path={WEB_APP_CONFIG.statusEndpoint}
            description="Runtime status"
          />
          <EndpointRow
            method="POST"
            path={WEB_APP_CONFIG.mcpEndpoint}
            description="MCP Streamable HTTP transport"
          />
          <EndpointRow
            method="GET"
            path="/openapi.json"
            description="Generated OpenAPI schema for the REST surface"
          />
        </div>
      </div>

      {/* Action parity table */}
      <div
        style={{
          background: "var(--aurora-panel-medium)",
          border: "1px solid var(--aurora-border-default)",
          borderRadius: "var(--radius-lg)",
          padding: "1.25rem",
        }}
      >
        <h2
          style={{
            color: "var(--aurora-text-muted)",
            fontSize: "0.75rem",
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.05em",
            marginBottom: "1rem",
          }}
        >
          Surface Parity
        </h2>
        <div style={{ overflowX: "auto" }}>
          <table
            style={{
              width: "100%",
              borderCollapse: "collapse",
              fontSize: "0.8rem",
              fontFamily: "var(--aurora-font-mono)",
            }}
          >
            <thead>
              <tr style={{ borderBottom: "1px solid var(--aurora-border-default)" }}>
                {["Surface", "Call Pattern"].map((h) => (
                  <th
                    key={h}
                    style={{
                      textAlign: "left",
                      padding: "0.5rem 0.75rem",
                      color: "var(--aurora-text-muted)",
                      fontWeight: 600,
                      fontSize: "0.7rem",
                      textTransform: "uppercase",
                      letterSpacing: "0.05em",
                    }}
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {[
                ["MCP", `${WEB_APP_CONFIG.serviceName}(action="greet", name="Alice")`],
                [
                  "REST",
                  `POST ${WEB_APP_CONFIG.restEndpoint} {"action":"greet","params":{"name":"Alice"}}`,
                ],
                ["CLI", `${WEB_APP_CONFIG.serviceName} greet --name Alice`],
              ].map(([surface, pattern]) => (
                <tr
                  key={surface}
                  style={{ borderBottom: "1px solid var(--aurora-border-default)" }}
                >
                  <td style={{ padding: "0.5rem 0.75rem", color: "var(--aurora-accent-primary)" }}>
                    {surface}
                  </td>
                  <td
                    style={{
                      padding: "0.5rem 0.75rem",
                      color: "var(--aurora-text-muted)",
                      wordBreak: "break-all",
                    }}
                  >
                    {pattern}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Action reference */}
      <div className="space-y-4">
        {ACTIONS.map((action) => (
          <ActionCard key={action.id} action={action} />
        ))}
      </div>
    </div>
  );
}
