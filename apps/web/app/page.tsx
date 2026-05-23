"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { ActionButton } from "@/components/dashboard/action-button";
import { Card } from "@/components/dashboard/card";
import { Button } from "@/components/ui/button";
import { echo, getHealth, getStatus, greet, type StatusResult, status } from "@/lib/api";
import { WEB_APP_CONFIG } from "@/lib/template";

type HealthState = "ok" | "error" | "loading";

interface ActivityItem {
  id: number;
  time: string;
  action: string;
  result: string;
  ok: boolean;
}

export default function DashboardPage() {
  const [health, setHealth] = useState<HealthState>("loading");
  const [serverStatus, setServerStatus] = useState<StatusResult | null>(null);
  const [activity, setActivity] = useState<ActivityItem[]>([]);
  const nextIdRef = useRef(1);

  const checkHealth = useCallback(async () => {
    const res = await getHealth();
    setHealth(res.data?.status === "ok" ? "ok" : "error");
  }, []);

  const checkStatus = useCallback(async () => {
    const res = await getStatus();
    if (res.data) setServerStatus(res.data);
  }, []);

  useEffect(() => {
    checkHealth();
    checkStatus();
    const interval = setInterval(checkHealth, 10_000);
    return () => clearInterval(interval);
  }, [checkHealth, checkStatus]);

  const addActivity = useCallback((action: string, result: string, ok: boolean) => {
    const id = nextIdRef.current++;
    const item: ActivityItem = { id, time: new Date().toLocaleTimeString(), action, result, ok };
    setActivity((prev) => [item, ...prev].slice(0, 20));
  }, []);

  const handleGreet = async () => {
    const res = await greet("Alice");
    addActivity("greet(Alice)", res.data?.greeting ?? res.error ?? "error", !res.error);
  };

  const handleEcho = async () => {
    const res = await echo("Hello from the dashboard!");
    addActivity("echo", res.data?.echo ?? res.error ?? "error", !res.error);
  };

  const handleStatus = async () => {
    const res = await status();
    addActivity("status", res.data?.status ?? res.error ?? "error", !res.error);
  };

  const statusColor: Record<HealthState, string> = {
    ok: "var(--aurora-success)",
    error: "var(--aurora-error)",
    loading: "var(--aurora-text-muted)",
  };

  const statusLabel: Record<HealthState, string> = {
    ok: "Healthy",
    error: "Unreachable",
    loading: "Checking…",
  };

  return (
    <div className="max-w-5xl mx-auto space-y-6">
      {/* Header */}
      <div>
        <h1
          style={{
            fontFamily: "var(--aurora-font-display)",
            color: "var(--aurora-text-primary)",
            fontSize: "1.75rem",
            fontWeight: 700,
            marginBottom: "0.25rem",
          }}
        >
          {WEB_APP_CONFIG.dashboardTitle}
        </h1>
        <p style={{ color: "var(--aurora-text-muted)", fontSize: "0.875rem" }}>
          {WEB_APP_CONFIG.displayName} MCP server — real-time status and quick actions
        </p>
      </div>

      {/* Status cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card title="Server Health">
          <div className="flex items-center gap-2">
            <div
              style={{
                width: 10,
                height: 10,
                borderRadius: "50%",
                background: statusColor[health],
                boxShadow: health === "ok" ? `0 0 6px var(--aurora-success)` : undefined,
              }}
            />
            <span style={{ color: statusColor[health], fontWeight: 600, fontSize: "1rem" }}>
              {statusLabel[health]}
            </span>
          </div>
          <p
            style={{ color: "var(--aurora-text-muted)", fontSize: "0.75rem", marginTop: "0.5rem" }}
          >
            Polls {WEB_APP_CONFIG.healthEndpoint} every 10s
          </p>
        </Card>

        <Card title="API URL">
          <p
            style={{
              color: "var(--aurora-accent-primary)",
              fontFamily: "var(--aurora-font-mono)",
              fontSize: "0.8rem",
              wordBreak: "break-all",
            }}
          >
            {serverStatus?.api_url ?? "—"}
          </p>
        </Card>

        <Card title="Status">
          <p
            style={{
              color:
                serverStatus?.status === "ok"
                  ? "var(--aurora-success)"
                  : "var(--aurora-text-muted)",
              fontWeight: 600,
            }}
          >
            {serverStatus?.status ?? "—"}
          </p>
          {serverStatus?.note && (
            <p
              style={{
                color: "var(--aurora-text-muted)",
                fontSize: "0.75rem",
                marginTop: "0.25rem",
              }}
            >
              {serverStatus.note}
            </p>
          )}
        </Card>
      </div>

      {/* Quick actions */}
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
            fontWeight: 600,
            marginBottom: "1rem",
            fontSize: "0.9rem",
            textTransform: "uppercase",
            letterSpacing: "0.05em",
          }}
        >
          Quick Actions
        </h2>
        <div className="flex flex-wrap gap-3">
          <ActionButton onClick={handleGreet} label="Greet Alice" />
          <ActionButton onClick={handleEcho} label="Echo Test" />
          <ActionButton onClick={handleStatus} label="Server Status" />
          <Button asChild variant="neutral">
            <a href="/tools/">Open Tool Runner →</a>
          </Button>
        </div>
      </div>

      {/* Activity feed */}
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
            fontWeight: 600,
            marginBottom: "1rem",
            fontSize: "0.9rem",
            textTransform: "uppercase",
            letterSpacing: "0.05em",
          }}
        >
          Recent Activity
        </h2>
        {activity.length === 0 ? (
          <p style={{ color: "var(--aurora-text-muted)", fontSize: "0.875rem" }}>
            No activity yet — click a quick action above.
          </p>
        ) : (
          <div className="space-y-2">
            {activity.map((item) => (
              <div
                key={item.id}
                style={{
                  display: "flex",
                  gap: "0.75rem",
                  alignItems: "flex-start",
                  padding: "0.5rem 0.75rem",
                  background: "var(--aurora-control-surface)",
                  borderRadius: "var(--radius-sm)",
                  border: "1px solid var(--aurora-border-default)",
                }}
              >
                <span
                  style={{
                    color: item.ok ? "var(--aurora-success)" : "var(--aurora-error)",
                    fontFamily: "var(--aurora-font-mono)",
                    fontSize: "0.75rem",
                    minWidth: "4rem",
                  }}
                >
                  {item.time}
                </span>
                <span
                  style={{
                    color: "var(--aurora-accent-primary)",
                    fontFamily: "var(--aurora-font-mono)",
                    fontSize: "0.75rem",
                    minWidth: "8rem",
                  }}
                >
                  {item.action}
                </span>
                <span style={{ color: "var(--aurora-text-primary)", fontSize: "0.8rem", flex: 1 }}>
                  {item.result}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
