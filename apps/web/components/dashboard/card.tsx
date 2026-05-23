export function Card({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div
      style={{
        background: "var(--aurora-panel-medium)",
        border: "1px solid var(--aurora-border-default)",
        borderRadius: "var(--radius-lg)",
        padding: "1.25rem",
      }}
    >
      <p
        style={{
          color: "var(--aurora-text-muted)",
          fontSize: "0.75rem",
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.05em",
          marginBottom: "0.5rem",
        }}
      >
        {title}
      </p>
      {children}
    </div>
  );
}
