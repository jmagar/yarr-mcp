"use client";

import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import * as React from "react";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  [
    "inline-flex items-center justify-center gap-2 whitespace-nowrap",
    "transition-all duration-150 ease-out",
    "disabled:pointer-events-none disabled:opacity-45",
    "focus-visible:outline-none",
    "select-none cursor-pointer",
  ].join(" "),
  {
    variants: {
      variant: {
        aurora: ["border text-[var(--aurora-text-primary)]", "bg-transparent"].join(" "),
        neutral: ["border text-[var(--aurora-text-primary)]", "bg-transparent"].join(" "),
        rose: ["border text-[var(--aurora-text-primary)]", "bg-transparent"].join(" "),
        ghost: [
          "border-transparent text-[var(--aurora-text-muted)]",
          "bg-transparent hover:text-[var(--aurora-text-primary)]",
        ].join(" "),
        destructive: ["border text-[var(--aurora-error)]", "bg-transparent"].join(" "),
        plain: "border-transparent bg-transparent text-inherit",
      },
      size: {
        sm: "h-7 px-3 rounded-[7px]",
        default: "h-8 px-3.5 rounded-[8px]",
        lg: "h-10 px-5 rounded-[10px]",
        icon: "size-8 rounded-[8px] p-0",
        unstyled: "",
      },
    },
    defaultVariants: {
      variant: "aurora",
      size: "default",
    },
  },
);

// Inline style maps for Aurora-specific gradients and glows
function getVariantStyle(variant: ButtonVariant | null | undefined): React.CSSProperties {
  switch (variant) {
    case "aurora":
      return {
        borderColor:
          "color-mix(in srgb, var(--aurora-accent-primary) 42%, var(--aurora-border-strong))",
        background: [
          "linear-gradient(180deg, color-mix(in srgb, var(--aurora-accent-primary) 10%, transparent), transparent 58%)",
          "var(--aurora-control-surface)",
        ].join(", "),
        boxShadow: [
          "inset 0 1px 0 rgba(255,255,255,0.055)",
          "0 0 0 1px color-mix(in srgb, var(--aurora-accent-primary) 16%, transparent)",
          "0 0 10px color-mix(in srgb, var(--aurora-accent-primary) 12%, transparent)",
        ].join(", "),
      };
    case "neutral":
      return {
        borderColor: "var(--aurora-border-strong)",
        background: "var(--aurora-control-surface)",
        boxShadow: "inset 0 1px 0 rgba(255,255,255,0.045)",
      };
    case "rose":
      return {
        borderColor:
          "color-mix(in srgb, var(--aurora-accent-pink) 52%, var(--aurora-border-strong))",
        background: [
          "linear-gradient(180deg, color-mix(in srgb, var(--aurora-accent-pink) 14%, transparent), transparent 58%)",
          "var(--aurora-control-surface)",
        ].join(", "),
        boxShadow: [
          "inset 0 1px 0 rgba(255,255,255,0.06)",
          "0 0 0 1px color-mix(in srgb, var(--aurora-accent-pink) 18%, transparent)",
          "0 0 13px color-mix(in srgb, var(--aurora-accent-pink) 16%, transparent)",
        ].join(", "),
      };
    case "ghost":
      return {};
    case "destructive":
      return {
        borderColor: "color-mix(in srgb, var(--aurora-error) 42%, var(--aurora-border-strong))",
        background: [
          "linear-gradient(180deg, color-mix(in srgb, var(--aurora-error) 9%, transparent), transparent 58%)",
          "var(--aurora-control-surface)",
        ].join(", "),
        boxShadow: [
          "inset 0 1px 0 rgba(255,255,255,0.045)",
          "0 0 0 1px color-mix(in srgb, var(--aurora-error) 14%, transparent)",
        ].join(", "),
      };
    case "plain":
      return {};
    default:
      return {};
  }
}

function getHoverStyle(variant: ButtonVariant | null | undefined): string {
  switch (variant) {
    case "aurora":
      return "hover:border-[color-mix(in_srgb,var(--aurora-accent-primary)_58%,var(--aurora-border-strong))] hover:bg-[color-mix(in_srgb,var(--aurora-accent-primary)_8%,var(--aurora-control-surface))]";
    case "neutral":
      return "hover:border-[var(--aurora-border-strong)] hover:bg-[var(--aurora-hover-bg)]";
    case "rose":
      return "hover:border-[color-mix(in_srgb,var(--aurora-accent-pink)_68%,var(--aurora-border-strong))] hover:bg-[color-mix(in_srgb,var(--aurora-accent-pink)_10%,var(--aurora-control-surface))] hover:[box-shadow:inset_0_1px_0_rgba(255,255,255,0.08),0_0_0_1px_color-mix(in_srgb,var(--aurora-accent-pink)_24%,transparent),0_0_18px_color-mix(in_srgb,var(--aurora-accent-pink)_24%,transparent)]";
    case "ghost":
      return "hover:bg-[var(--aurora-hover-bg)]";
    case "destructive":
      return "hover:border-[color-mix(in_srgb,var(--aurora-error)_58%,var(--aurora-border-strong))] hover:bg-[color-mix(in_srgb,var(--aurora-error)_7%,var(--aurora-control-surface))]";
    case "plain":
      return "";
    default:
      return "";
  }
}

type ButtonVariant = "aurora" | "neutral" | "rose" | "ghost" | "destructive" | "plain";

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, style, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    const variantStyle = getVariantStyle(variant);
    const hoverClass = getHoverStyle(variant);
    const typographyStyle =
      variant === "plain" && size === "unstyled"
        ? {}
        : {
            fontFamily: "var(--aurora-font-sans)",
            fontSize: size === "lg" ? "14px" : size === "sm" ? "12px" : "13px",
            fontWeight: size === "lg" ? 680 : 650,
            letterSpacing: "0.012em",
            lineHeight: "var(--aurora-line-ui)",
          };

    return (
      <Comp
        ref={ref}
        className={cn(buttonVariants({ variant, size }), hoverClass, className)}
        style={{
          ...typographyStyle,
          ...variantStyle,
          ...style,
        }}
        {...props}
      />
    );
  },
);
Button.displayName = "Button";

export { Button, buttonVariants };
export default Button;
