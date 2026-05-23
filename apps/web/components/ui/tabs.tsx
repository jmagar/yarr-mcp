"use client";

/**
 * Aurora Design System — Tabs & PillGroup
 * peer dep: @radix-ui/react-tabs
 */

import * as TabsPrimitive from "@radix-ui/react-tabs";
import * as React from "react";
import { cn } from "@/lib/utils";

// ─── Line Tabs ────────────────────────────────────────────────────────────────

const Tabs = TabsPrimitive.Root;

const TabsList = React.forwardRef<
  React.ComponentRef<typeof TabsPrimitive.List>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.List>
>(({ className, style, ...props }, ref) => (
  <TabsPrimitive.List
    ref={ref}
    className={cn("flex items-end gap-1 border-b", className)}
    style={{ borderColor: "var(--aurora-border-default)", ...style }}
    {...props}
  />
));
TabsList.displayName = TabsPrimitive.List.displayName;

const TabsTrigger = React.forwardRef<
  React.ComponentRef<typeof TabsPrimitive.Trigger>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.Trigger>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.Trigger
    ref={ref}
    className={cn(
      // layout
      "relative inline-flex items-center gap-1.5 px-3 pb-2.5 pt-1",
      "select-none cursor-pointer",
      "transition-colors duration-150 focus-visible:outline-none",
      // resting state
      "text-[var(--aurora-text-muted)] hover:text-[var(--aurora-text-primary)]",
      // bottom-border indicator (pseudo element via after:)
      "after:absolute after:bottom-0 after:left-0 after:right-0 after:h-[2px]",
      "after:rounded-t-full after:bg-transparent after:transition-colors after:duration-150",
      // active state
      "data-[state=active]:text-[var(--aurora-accent-primary)]",
      "data-[state=active]:after:bg-[var(--aurora-accent-primary)]",
      className,
    )}
    style={{
      fontFamily: "var(--aurora-font-sans)",
      fontSize: "var(--aurora-type-body-sm)",
      fontWeight: "var(--aurora-weight-ui)",
      letterSpacing: "var(--aurora-letter-ui)",
      lineHeight: "var(--aurora-line-ui)",
    }}
    {...props}
  />
));
TabsTrigger.displayName = TabsPrimitive.Trigger.displayName;

const TabsContent = React.forwardRef<
  React.ComponentRef<typeof TabsPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.Content>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.Content
    ref={ref}
    className={cn("mt-4 focus-visible:outline-none", className)}
    {...props}
  />
));
TabsContent.displayName = TabsPrimitive.Content.displayName;

// ─── Pill Group ───────────────────────────────────────────────────────────────

/**
 * PillGroup — a pill-shaped segmented toggle that wraps Radix Tabs.
 *
 * Usage:
 *   <PillGroup defaultValue="a">
 *     <PillTrigger value="a">Option A</PillTrigger>
 *     <PillTrigger value="b">Option B</PillTrigger>
 *   </PillGroup>
 *
 * Note: PillGroup renders only the TabsList, not TabsContent.
 * Pair with TabsContent outside PillGroup if panel switching is needed.
 */
export type PillGroupProps = React.ComponentPropsWithoutRef<typeof TabsPrimitive.Root>;

const PillGroup = React.forwardRef<React.ComponentRef<typeof TabsPrimitive.Root>, PillGroupProps>(
  ({ className, children, style, ...props }, ref) => (
    <TabsPrimitive.Root ref={ref} {...props}>
      <TabsPrimitive.List
        className={cn("inline-flex items-center gap-1 rounded-full border p-1", className)}
        style={{
          backgroundColor: "var(--aurora-control-surface)",
          borderColor: "var(--aurora-border-default)",
          ...style,
        }}
      >
        {children}
      </TabsPrimitive.List>
    </TabsPrimitive.Root>
  ),
);
PillGroup.displayName = "PillGroup";

const PillTrigger = React.forwardRef<
  React.ComponentRef<typeof TabsPrimitive.Trigger>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.Trigger>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.Trigger
    ref={ref}
    className={cn(
      "inline-flex items-center gap-1.5 rounded-full px-4 py-1.5",
      "select-none cursor-pointer",
      "transition-all duration-150 focus-visible:outline-none",
      // resting
      "text-[var(--aurora-text-muted)] hover:text-[var(--aurora-text-primary)]",
      // active — bg + accent text + active-glow
      "data-[state=active]:text-[var(--aurora-accent-primary)]",
      "[&[data-state=active]]:bg-[var(--aurora-panel-strong)]",
      "[&[data-state=active]]:[box-shadow:var(--aurora-active-glow)]",
      className,
    )}
    style={{
      fontFamily: "var(--aurora-font-sans)",
      fontSize: "var(--aurora-type-body-sm)",
      fontWeight: "var(--aurora-weight-ui)",
      letterSpacing: "var(--aurora-letter-ui)",
      lineHeight: "var(--aurora-line-ui)",
    }}
    {...props}
  />
));
PillTrigger.displayName = "PillTrigger";

export { PillGroup, PillTrigger, Tabs, TabsContent, TabsList, TabsTrigger };
