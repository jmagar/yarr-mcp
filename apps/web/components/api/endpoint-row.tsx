export function EndpointRow({
  method,
  path,
  description,
}: {
  method: string;
  path: string;
  description: string;
}) {
  const methodColors: Record<string, string> = {
    GET: "var(--aurora-success)",
    POST: "var(--aurora-accent-primary)",
  };

  return (
    <div
      style={{
        display: "flex",
        gap: "0.75rem",
        alignItems: "center",
        padding: "0.5rem 0.75rem",
        background: "var(--aurora-control-surface)",
        borderRadius: "var(--radius-md)",
        border: "1px solid var(--aurora-border-default)",
      }}
    >
      <span
        style={{
          color: methodColors[method] ?? "var(--aurora-text-muted)",
          fontFamily: "var(--aurora-font-mono)",
          fontSize: "0.7rem",
          fontWeight: 700,
          minWidth: "3rem",
        }}
      >
        {method}
      </span>
      <span
        style={{
          color: "var(--aurora-text-primary)",
          fontFamily: "var(--aurora-font-mono)",
          fontSize: "0.8rem",
          minWidth: "12rem",
        }}
      >
        {path}
      </span>
      <span style={{ color: "var(--aurora-text-muted)", fontSize: "0.8rem" }}>{description}</span>
    </div>
  );
}
