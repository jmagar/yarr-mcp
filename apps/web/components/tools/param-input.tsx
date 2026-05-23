/**
 * ParamInput — a styled text input for the tool runner form.
 *
 * Uses the shared Aurora-compatible Input wrapper so focus and disabled states
 * stay aligned with the rest of the UI surface.
 */

"use client";

import { Input } from "@/components/ui/input";

interface ParamInputProps {
  id: string;
  type?: string;
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
  required?: boolean;
}

export function ParamInput({
  id,
  type = "text",
  placeholder,
  value,
  onChange,
  required,
}: ParamInputProps) {
  if (type === "checkbox") {
    return (
      <input
        id={id}
        type="checkbox"
        checked={value === "true"}
        required={required}
        onChange={(e) => onChange(e.target.checked ? "true" : "")}
        style={{ width: 18, height: 18, accentColor: "var(--aurora-accent-primary)" }}
      />
    );
  }

  if (type === "textarea") {
    return (
      <textarea
        id={id}
        placeholder={placeholder}
        value={value}
        required={required}
        onChange={(e) => onChange(e.target.value)}
        rows={5}
        style={{
          width: "100%",
          minHeight: "7rem",
          resize: "vertical",
          background: "var(--aurora-control-surface, #0c1a24)",
          color: "var(--aurora-text-primary)",
          border: "1px solid var(--aurora-border-strong)",
          borderRadius: "var(--aurora-radius-1)",
          padding: "0.5rem 0.75rem",
          fontFamily: "var(--aurora-font-mono)",
          fontSize: "0.8rem",
        }}
      />
    );
  }

  return (
    <Input
      id={id}
      type={type}
      placeholder={placeholder}
      value={value}
      required={required}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}
