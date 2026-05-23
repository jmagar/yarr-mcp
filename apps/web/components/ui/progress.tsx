"use client";

import * as React from "react";
import { cn } from "@/lib/utils";

// ─── Keyframes ────────────────────────────────────────────────────────────────

const SHIMMER_KEYFRAMES = `
@keyframes aurora-progress-shimmer {
  0%   { transform: translateX(-100%); }
  100% { transform: translateX(400%); }
}
@keyframes aurora-progress-indeterminate {
  0%   { left: -35%; right: 100%; }
  60%  { left: 100%; right: -90%; }
  100% { left: 100%; right: -90%; }
}
`;

let shimmerInjected = false;
function ensureShimmerKeyframes() {
  if (shimmerInjected || typeof document === "undefined") return;
  const style = document.createElement("style");
  style.textContent = SHIMMER_KEYFRAMES;
  document.head.appendChild(style);
  shimmerInjected = true;
}

// ─── Fill color map ───────────────────────────────────────────────────────────

type ProgressVariant = "default" | "warn" | "error" | "rose";

const fillStyleMap: Record<ProgressVariant, React.CSSProperties> = {
  default: {
    background:
      "linear-gradient(90deg, var(--aurora-accent-button) 0%, var(--aurora-accent-lift) 60%, var(--aurora-accent-strong) 100%)",
    boxShadow:
      "0 0 8px color-mix(in srgb, var(--aurora-accent-primary) 50%, transparent), 0 0 2px var(--aurora-accent-primary)",
  },
  warn: {
    background:
      "linear-gradient(90deg, color-mix(in srgb, var(--aurora-warn) 72%, black) 0%, var(--aurora-warn) 100%)",
    boxShadow: "0 0 8px color-mix(in srgb, var(--aurora-warn) 40%, transparent)",
  },
  error: {
    background:
      "linear-gradient(90deg, color-mix(in srgb, var(--aurora-error) 72%, black) 0%, var(--aurora-error) 100%)",
    boxShadow: "0 0 8px color-mix(in srgb, var(--aurora-error) 40%, transparent)",
  },
  rose: {
    background:
      "linear-gradient(90deg, var(--aurora-accent-pink-deep) 0%, var(--aurora-accent-pink) 100%)",
    boxShadow: "0 0 8px color-mix(in srgb, var(--aurora-accent-pink) 40%, transparent)",
  },
};

const shimmerColorMap: Record<ProgressVariant, string> = {
  default: "rgba(255,255,255,0.30)",
  warn: "rgba(255,255,255,0.20)",
  error: "rgba(255,255,255,0.18)",
  rose: "rgba(255,255,255,0.22)",
};

// ─── Size map ─────────────────────────────────────────────────────────────────

const heightMap = {
  sm: 4,
  default: 6,
  lg: 10,
} as const;

type ProgressSize = keyof typeof heightMap;

// ─── Props ────────────────────────────────────────────────────────────────────

export interface ProgressProps extends React.HTMLAttributes<HTMLDivElement> {
  /** 0–100. If undefined the bar is indeterminate. */
  value?: number;
  /** Color variant */
  variant?: ProgressVariant;
  /** Height preset */
  size?: ProgressSize;
  /** Show percentage label */
  showLabel?: boolean;
  /** Override label text */
  label?: string;
  /** Max value (default 100) */
  max?: number;
}

// ─── Component ────────────────────────────────────────────────────────────────

const Progress = React.forwardRef<HTMLDivElement, ProgressProps>(
  (
    {
      className,
      value,
      variant = "default",
      size = "default",
      showLabel = false,
      label,
      max = 100,
      style,
      ...props
    },
    ref,
  ) => {
    React.useEffect(() => {
      ensureShimmerKeyframes();
    }, []);

    const isIndeterminate = value === undefined || value === null;
    const clampedValue = isIndeterminate ? 0 : Math.min(Math.max(value, 0), max);
    const percentage = isIndeterminate ? 0 : Math.round((clampedValue / max) * 100);
    const height = heightMap[size];
    const fillStyle = fillStyleMap[variant];
    const shimmerColor = shimmerColorMap[variant];

    const displayLabel = label ?? `${percentage}%`;

    return (
      <div className={cn("flex flex-col gap-1.5 w-full", className)} style={style} {...props}>
        {showLabel && (
          <div className="flex items-center justify-between">
            <span
              style={{
                fontSize: 11,
                fontFamily: "var(--aurora-font-mono, 'JetBrains Mono', monospace)",
                color: "var(--aurora-text-muted)",
                letterSpacing: "0.05em",
              }}
            >
              {displayLabel}
            </span>
          </div>
        )}

        {/* Track */}
        <div
          ref={ref}
          role="progressbar"
          aria-valuenow={isIndeterminate ? undefined : clampedValue}
          aria-valuemin={0}
          aria-valuemax={max}
          aria-label={isIndeterminate ? "Loading…" : displayLabel}
          style={{
            height,
            borderRadius: height,
            background: "var(--aurora-control-surface, #0c1a24)",
            border: "1px solid var(--aurora-border-default, #1d3d4e)",
            overflow: "hidden",
            position: "relative",
            width: "100%",
          }}
        >
          {/* Fill */}
          <div
            aria-hidden="true"
            style={{
              position: "absolute",
              top: 0,
              bottom: 0,
              left: 0,
              borderRadius: "inherit",
              transition: isIndeterminate ? "none" : "width 400ms cubic-bezier(0.4, 0, 0.2, 1)",
              width: isIndeterminate ? "40%" : `${percentage}%`,
              ...(isIndeterminate
                ? {
                    animation: "aurora-progress-indeterminate 1.6s ease-in-out infinite",
                  }
                : {}),
              ...fillStyle,
            }}
          >
            {/* Shimmer highlight */}
            <span
              aria-hidden="true"
              style={{
                position: "absolute",
                inset: 0,
                background: `linear-gradient(90deg, transparent 0%, ${shimmerColor} 50%, transparent 100%)`,
                animation: "aurora-progress-shimmer 2.2s ease-in-out infinite",
              }}
            />
          </div>
        </div>
      </div>
    );
  },
);
Progress.displayName = "Progress";

export { Progress };
export default Progress;
