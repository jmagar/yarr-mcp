import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** Development-only warning helper used by Aurora components. */
export function devWarn(message: string): void {
  if (process.env.NODE_ENV !== "production") {
    console.warn(`[Aurora] ${message}`);
  }
}
