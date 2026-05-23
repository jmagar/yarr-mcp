"use client";

import * as React from "react";
import { cn } from "@/lib/utils";

export interface SeparatorProps extends React.HTMLAttributes<HTMLDivElement> {
  orientation?: "horizontal" | "vertical";
  decorative?: boolean;
}

const Separator = React.forwardRef<HTMLDivElement, SeparatorProps>(
  ({ className, orientation = "horizontal", decorative = true, style, ...props }, ref) => {
    const separatorProps =
      decorative || orientation === "horizontal"
        ? {}
        : ({ "aria-orientation": orientation } as const);

    return (
      <div
        ref={ref}
        role={decorative ? "none" : "separator"}
        className={cn(
          orientation === "vertical" ? "h-full min-h-5 w-px" : "h-px w-full",
          className,
        )}
        style={{
          background: "var(--aurora-border-default)",
          ...style,
        }}
        {...separatorProps}
        {...props}
      />
    );
  },
);
Separator.displayName = "Separator";

export { Separator };
export default Separator;
