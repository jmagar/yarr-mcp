"use client";

import { useState } from "react";
import { ParamInput } from "@/components/tools/param-input";
import { ResponsePanel } from "@/components/tools/response-panel";
import { SubmitButton } from "@/components/tools/submit-button";
import { Button } from "@/components/ui/button";
import { callAction } from "@/lib/api";
import {
  DEFAULT_REST_ACTION,
  REST_ACTIONS,
  type RestActionId,
  WEB_APP_CONFIG,
} from "@/lib/template";

export default function ToolsPage() {
  const [selectedAction, setSelectedAction] = useState<RestActionId>(DEFAULT_REST_ACTION.id);
  const [paramValues, setParamValues] = useState<Record<string, string>>({});
  const [response, setResponse] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [isError, setIsError] = useState(false);

  const action = REST_ACTIONS.find((a) => a.id === selectedAction) ?? DEFAULT_REST_ACTION;

  const handleSelect = (id: RestActionId) => {
    setSelectedAction(id);
    setParamValues({});
    setResponse(null);
    setIsError(false);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setResponse(null);
    setIsError(false);

    const params: Record<string, string> = {};
    for (const [k, v] of Object.entries(paramValues)) {
      if (v.trim()) params[k] = v.trim();
    }

    const res = await callAction(selectedAction, params);
    setLoading(false);

    if (res.error) {
      setResponse(JSON.stringify({ error: res.error }, null, 2));
      setIsError(true);
    } else {
      setResponse(JSON.stringify(res.data, null, 2));
    }
  };

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
          Tool Runner
        </h1>
        <p style={{ color: "var(--aurora-text-muted)", fontSize: "0.875rem" }}>
          Call any action via{" "}
          <code
            style={{
              fontFamily: "var(--aurora-font-mono)",
              background: "var(--aurora-panel-strong)",
              padding: "0.1em 0.4em",
              borderRadius: "4px",
              fontSize: "0.8em",
            }}
          >
            POST {WEB_APP_CONFIG.restEndpoint}
          </code>
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Action selector */}
        <div
          style={{
            background: "var(--aurora-panel-medium)",
            border: "1px solid var(--aurora-border-default)",
            borderRadius: "var(--radius-lg)",
            padding: "1rem",
          }}
        >
          <p
            style={{
              color: "var(--aurora-text-muted)",
              fontSize: "0.75rem",
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.05em",
              marginBottom: "0.75rem",
            }}
          >
            Actions
          </p>
          <div className="space-y-1">
            {REST_ACTIONS.map((a) => (
              <Button
                type="button"
                key={a.id}
                onClick={() => handleSelect(a.id)}
                variant="ghost"
                size="sm"
                className="w-full justify-start border-l-2 font-[var(--aurora-font-mono)]"
                style={{
                  textAlign: "left",
                  background: selectedAction === a.id ? "var(--aurora-hover-bg)" : "transparent",
                  color:
                    selectedAction === a.id
                      ? "var(--aurora-accent-primary)"
                      : "var(--aurora-text-primary)",
                  borderLeft:
                    selectedAction === a.id
                      ? "2px solid var(--aurora-accent-primary)"
                      : "2px solid transparent",
                }}
              >
                {a.label}
              </Button>
            ))}
          </div>
        </div>

        {/* Form + response */}
        <div className="md:col-span-2 space-y-4">
          <form
            onSubmit={handleSubmit}
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
              {action.label}
            </p>
            <p
              style={{
                color: "var(--aurora-text-muted)",
                fontSize: "0.8rem",
                marginBottom: "1rem",
              }}
            >
              {action.description}
            </p>

            {action.params.length > 0 ? (
              <div className="space-y-3 mb-4">
                {action.params.map((param) => (
                  <div key={param.name}>
                    <label
                      htmlFor={param.name}
                      style={{
                        display: "block",
                        color: "var(--aurora-text-muted)",
                        fontSize: "0.75rem",
                        marginBottom: "0.25rem",
                        fontWeight: 500,
                      }}
                    >
                      {param.label}
                      {param.required && (
                        <span style={{ color: "var(--aurora-error)", marginLeft: "0.25rem" }}>
                          *
                        </span>
                      )}
                    </label>
                    <ParamInput
                      id={param.name}
                      type={param.type}
                      placeholder={param.placeholder}
                      value={paramValues[param.name] ?? ""}
                      onChange={(value) =>
                        setParamValues((prev) => ({ ...prev, [param.name]: value }))
                      }
                      required={param.required}
                    />
                  </div>
                ))}
              </div>
            ) : (
              <p
                style={{
                  color: "var(--aurora-text-muted)",
                  fontSize: "0.8rem",
                  marginBottom: "1rem",
                }}
              >
                No parameters required.
              </p>
            )}

            <SubmitButton loading={loading} />
          </form>

          {response !== null && <ResponsePanel response={response} isError={isError} />}

          {/* Request preview */}
          <div
            style={{
              background: "var(--aurora-panel-medium)",
              border: "1px solid var(--aurora-border-default)",
              borderRadius: "var(--radius-lg)",
              padding: "1rem",
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
              Request Preview
            </p>
            <pre
              style={{
                color: "var(--aurora-text-muted)",
                fontFamily: "var(--aurora-font-mono)",
                fontSize: "0.75rem",
                margin: 0,
                whiteSpace: "pre-wrap",
              }}
            >
              {`POST ${WEB_APP_CONFIG.restEndpoint}\nContent-Type: application/json\n\n${JSON.stringify({ action: selectedAction, params: Object.fromEntries(Object.entries(paramValues).filter(([, v]) => v.trim())) }, null, 2)}`}
            </pre>
          </div>
        </div>
      </div>
    </div>
  );
}
