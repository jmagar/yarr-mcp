import tailwindcssAnimate from "tailwindcss-animate";

/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class"], // Common for Shadcn UI, assuming class-based dark mode
  content: [
    './pages/**/*.{ts,tsx}',
    './components/**/*.{ts,tsx}',
    './app/**/*.{ts,tsx}',
    './src/**/*.{ts,tsx}',
    './index.html' // ensure index.html is scanned if classes are there
  ],
  prefix: "", // No prefix, common for Shadcn
  theme: {
    container: { // Common Shadcn container settings
      center: true,
      padding: "2rem",
      screens: {
        "2xl": "1400px",
      },
    },
    extend: {
      colors: {
        // border: 'hsl(var(--border))', // Handled by @theme
        // input: 'hsl(var(--input))', // Handled by @theme
        // ring: 'hsl(var(--ring))', // Handled by @theme
        // background: 'hsl(var(--background))', // Handled by @theme
        // foreground: 'hsl(var(--foreground))', // Handled by @theme
        // primary: { // Handled by @theme
        //   DEFAULT: 'hsl(var(--primary))',
        //   foreground: 'hsl(var(--primary-foreground))',
        // },
        // secondary: { // Handled by @theme
        //   DEFAULT: 'hsl(var(--secondary))',
        //   foreground: 'hsl(var(--secondary-foreground))',
        // },
        // destructive: { // Handled by @theme
        //   DEFAULT: 'hsl(var(--destructive))',
        //   foreground: 'hsl(var(--destructive-foreground))',
        // },
        // muted: { // Handled by @theme
        //   DEFAULT: 'hsl(var(--muted))',
        //   foreground: 'hsl(var(--muted-foreground))',
        // },
        // accent: { // Handled by @theme
        //   DEFAULT: 'hsl(var(--accent))',
        //   foreground: 'hsl(var(--accent-foreground))',
        // },
        // popover: { // Handled by @theme
        //   DEFAULT: 'hsl(var(--popover))',
        //   foreground: 'hsl(var(--popover-foreground))',
        // },
        // card: { // Handled by @theme
        //   DEFAULT: 'hsl(var(--card))',
        //   foreground: 'hsl(var(--card-foreground))',
        // },
      },
      // borderRadius: { // Handled by @theme
      //   lg: 'var(--radius)',
      //   md: 'calc(var(--radius) - 2px)',
      //   sm: 'calc(var(--radius) - 4px)',
      // },
      keyframes: { // Common for Shadcn UI animations
        "accordion-down": {
          from: { height: "0" },
          to: { height: "var(--radix-accordion-content-height)" },
        },
        "accordion-up": {
          from: { height: "var(--radix-accordion-content-height)" },
          to: { height: "0" },
        },
      },
      animation: { // Common for Shadcn UI animations
        "accordion-down": "accordion-down 0.2s ease-out",
        "accordion-up": "accordion-up 0.2s ease-out",
      },
    },
  },
  plugins: [tailwindcssAnimate],
} 