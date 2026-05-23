"use client";

import * as React from "react";
import { cn } from "@/lib/utils";

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  /** Optional leading icon or adornment */
  startAdornment?: React.ReactNode;
  /** Optional trailing icon or adornment */
  endAdornment?: React.ReactNode;
}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, type = "text", startAdornment, endAdornment, style, ...props }, ref) => {
    const hasStart = Boolean(startAdornment);
    const hasEnd = Boolean(endAdornment);

    return (
      <div className="relative inline-flex w-full items-center">
        {hasStart && (
          <span
            className="pointer-events-none absolute left-3 flex items-center text-[var(--aurora-text-muted)]"
            aria-hidden="true"
          >
            {startAdornment}
          </span>
        )}

        <input
          ref={ref}
          type={type}
          className={cn(
            // Layout
            "h-9 w-full px-3 py-2",
            // Typography
            "font-[var(--aurora-font-sans,_Inter,_sans-serif)]",
            "text-[var(--aurora-text-primary)]",
            "placeholder:text-[var(--aurora-text-muted)]",
            // Background & border
            "border border-[var(--aurora-border-strong)]",
            // Rounded
            "rounded-[var(--aurora-radius-1)]",
            // Transitions
            "transition-all duration-150 ease-out",
            // Focus ring
            "focus-visible:outline-none",
            // Disabled
            "disabled:pointer-events-none disabled:opacity-45 disabled:cursor-not-allowed",
            // Adornment padding adjustments
            hasStart && "pl-9",
            hasEnd && "pr-9",
            className,
          )}
          style={{
            background: "var(--aurora-control-surface, #0c1a24)",
            fontSize: "var(--aurora-type-body-sm)",
            fontWeight: "var(--aurora-weight-body)",
            letterSpacing: "var(--aurora-letter-ui)",
            lineHeight: "var(--aurora-line-ui)",
            // Focus shadow applied via CSS — we use a focus-within wrapper trick
            // or inline via onFocus/onBlur in more complex setups.
            // Here we rely on the focus-visible ring below.
            ...style,
          }}
          onFocus={(e) => {
            e.currentTarget.style.borderColor = "var(--aurora-border-strong)";
            e.currentTarget.style.boxShadow = [
              "0 0 0 3px color-mix(in srgb, var(--aurora-accent-primary) 22%, transparent)",
              "0 0 0 1px color-mix(in srgb, var(--aurora-accent-primary) 45%, transparent)",
            ].join(", ");
            props.onFocus?.(e);
          }}
          onBlur={(e) => {
            e.currentTarget.style.boxShadow = "none";
            props.onBlur?.(e);
          }}
          {...props}
        />

        {hasEnd && (
          <span className="absolute right-3 flex items-center text-[var(--aurora-text-muted)]">
            {endAdornment}
          </span>
        )}
      </div>
    );
  },
);
Input.displayName = "Input";

export { Input };
export default Input;
