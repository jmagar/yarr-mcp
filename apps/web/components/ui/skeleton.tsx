"use client";

import { cva, type VariantProps } from "class-variance-authority";
import * as React from "react";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Keyframe injection
// ---------------------------------------------------------------------------

const SHIMMER_ID = "aurora-skeleton-shimmer";

function injectShimmer() {
  if (typeof document === "undefined") return;
  if (document.getElementById(SHIMMER_ID)) return;
  const style = document.createElement("style");
  style.id = SHIMMER_ID;
  style.textContent = `
    @keyframes aurora-shimmer {
      0%   { background-position: -800px 0; }
      100% { background-position:  800px 0; }
    }
    .aurora-shimmer {
      background: linear-gradient(
        90deg,
        var(--aurora-panel-strong) 25%,
        color-mix(in srgb, var(--aurora-border-strong) 60%, transparent) 37%,
        var(--aurora-panel-strong) 63%
      );
      background-size: 800px 100%;
      animation: aurora-shimmer 1.4s ease-in-out infinite;
    }
  `;
  document.head.appendChild(style);
}

// ---------------------------------------------------------------------------
// CVA variants
// ---------------------------------------------------------------------------

const skeletonVariants = cva("aurora-shimmer shrink-0", {
  variants: {
    variant: {
      text: "h-3.5 w-full rounded",
      title: "h-5 w-full rounded",
      avatar: "size-9 rounded-full",
      button: "h-9 w-24 rounded-lg",
      card: "h-32 w-full rounded-2xl",
    },
  },
  defaultVariants: {
    variant: "text",
  },
});

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SkeletonProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof skeletonVariants> {
  /** Override default width with a Tailwind/inline class */
  width?: string;
}

// ---------------------------------------------------------------------------
// Skeleton
// ---------------------------------------------------------------------------

export const Skeleton = React.forwardRef<HTMLDivElement, SkeletonProps>(function Skeleton(
  { variant, width, className, ...rest },
  ref,
) {
  React.useEffect(injectShimmer, []);

  return (
    <div
      ref={ref}
      aria-hidden="true"
      className={cn(skeletonVariants({ variant }), width, className)}
      {...rest}
    />
  );
});

Skeleton.displayName = "Skeleton";

// ---------------------------------------------------------------------------
// SkeletonRow — pre-composed avatar + two text lines + button
// ---------------------------------------------------------------------------

export type SkeletonRowProps = React.HTMLAttributes<HTMLDivElement>;

export const SkeletonRow = React.forwardRef<HTMLDivElement, SkeletonRowProps>(function SkeletonRow(
  { className, ...rest },
  ref,
) {
  React.useEffect(injectShimmer, []);

  return (
    <div
      ref={ref}
      aria-hidden="true"
      className={cn("flex items-center gap-3", className)}
      {...rest}
    >
      {/* Avatar */}
      <Skeleton variant="avatar" />

      {/* Text block */}
      <div className="flex flex-1 flex-col gap-2">
        <Skeleton variant="title" width="w-1/3" />
        <Skeleton variant="text" width="w-2/3" />
      </div>

      {/* Button */}
      <Skeleton variant="button" />
    </div>
  );
});

SkeletonRow.displayName = "SkeletonRow";
