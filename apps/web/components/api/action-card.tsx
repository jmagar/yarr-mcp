import { type ACTIONS, WEB_APP_CONFIG } from "@/lib/template";
import { CodeBlock } from "./code-block";

export function ActionCard({ action }: { action: (typeof ACTIONS)[number] }) {
  const isRestAction = action.transport === "rest";
  const curlExample = `curl -X POST http://localhost:3100${WEB_APP_CONFIG.restEndpoint} \\
  -H "Content-Type: application/json" \\
  -d '${JSON.stringify(action.example)}'`;

  return (
    <div
      style={{
        background: "var(--aurora-panel-medium)",
        border: "1px solid var(--aurora-border-default)",
        borderRadius: "var(--radius-lg)",
        padding: "1.25rem",
      }}
    >
      <div
        style={{ display: "flex", alignItems: "center", gap: "0.75rem", marginBottom: "0.5rem" }}
      >
        <span
          style={{
            background: "var(--aurora-hover-bg)",
            border: "1px solid var(--aurora-border-strong)",
            color: "var(--aurora-accent-primary)",
            fontFamily: "var(--aurora-font-mono)",
            fontSize: "0.85rem",
            fontWeight: 600,
            padding: "0.15rem 0.6rem",
            borderRadius: "var(--radius-sm)",
          }}
        >
          {action.id}
        </span>
        <span
          style={{
            color: isRestAction ? "var(--aurora-success)" : "var(--aurora-warn)",
            fontFamily: "var(--aurora-font-mono)",
            fontSize: "0.7rem",
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.05em",
          }}
        >
          {isRestAction ? "REST + MCP + CLI" : "MCP only"}
        </span>
      </div>
      <p style={{ color: "var(--aurora-text-muted)", fontSize: "0.85rem", marginBottom: "1rem" }}>
        {action.description}
      </p>

      {action.params.length > 0 && (
        <div style={{ marginBottom: "1rem" }}>
          <p
            style={{
              color: "var(--aurora-text-muted)",
              fontSize: "0.7rem",
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.05em",
              marginBottom: "0.5rem",
            }}
          >
            Parameters
          </p>
          {action.params.map((p) => (
            <div
              key={p.name}
              style={{
                display: "flex",
                gap: "0.5rem",
                alignItems: "baseline",
                fontSize: "0.8rem",
                fontFamily: "var(--aurora-font-mono)",
              }}
            >
              <span style={{ color: "var(--aurora-accent-pink)" }}>{p.name}</span>
              <span style={{ color: "var(--aurora-text-muted)" }}>string</span>
              {!p.required && (
                <span style={{ color: "var(--aurora-warn)", fontSize: "0.7rem" }}>optional</span>
              )}
              <span
                style={{ color: "var(--aurora-text-muted)", fontFamily: "var(--aurora-font-sans)" }}
              >
                — {p.description}
              </span>
            </div>
          ))}
        </div>
      )}

      <div className="space-y-3">
        {isRestAction ? (
          <CodeBlock label="cURL" code={curlExample} />
        ) : (
          <CodeBlock
            label="REST availability"
            code={`${action.id} is MCP-only because it requires an interactive MCP peer.`}
          />
        )}
        <CodeBlock
          label="MCP equivalent"
          code={`${WEB_APP_CONFIG.serviceName}(action="${action.id}"${action.params
            .map((p) => `, ${p.name}="..."`)
            .join("")})`}
        />
        <CodeBlock label="Response" code={JSON.stringify(action.response, null, 2)} />
      </div>
    </div>
  );
}
