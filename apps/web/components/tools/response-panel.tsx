export function ResponsePanel({ response, isError }: { response: string; isError: boolean }) {
  return (
    <div
      style={{
        background: "var(--aurora-panel-strong)",
        border: `1px solid ${isError ? "var(--aurora-error)" : "var(--aurora-border-default)"}`,
        borderRadius: "var(--radius-lg)",
        padding: "1.25rem",
      }}
    >
      <p
        style={{
          color: isError ? "var(--aurora-error)" : "var(--aurora-text-muted)",
          fontSize: "0.75rem",
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.05em",
          marginBottom: "0.75rem",
        }}
      >
        {isError ? "Error" : "Response"}
      </p>
      <pre
        style={{
          color: isError ? "var(--aurora-error)" : "var(--aurora-accent-strong)",
          fontFamily: "var(--aurora-font-mono)",
          fontSize: "0.8rem",
          overflow: "auto",
          margin: 0,
          whiteSpace: "pre-wrap",
          wordBreak: "break-word",
        }}
      >
        {response}
      </pre>
    </div>
  );
}
