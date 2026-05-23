export function CodeBlock({ label, code }: { label: string; code: string }) {
  return (
    <div>
      <p
        style={{
          color: "var(--aurora-text-muted)",
          fontSize: "0.7rem",
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.05em",
          marginBottom: "0.25rem",
        }}
      >
        {label}
      </p>
      <pre
        style={{
          background: "var(--aurora-control-surface)",
          border: "1px solid var(--aurora-border-default)",
          borderRadius: "var(--radius-md)",
          padding: "0.75rem",
          color: "var(--aurora-accent-strong)",
          fontFamily: "var(--aurora-font-mono)",
          fontSize: "0.75rem",
          overflow: "auto",
          margin: 0,
          whiteSpace: "pre-wrap",
          wordBreak: "break-word",
        }}
      >
        {code}
      </pre>
    </div>
  );
}
