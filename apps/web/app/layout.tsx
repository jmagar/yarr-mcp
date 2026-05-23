import type { Metadata } from "next";
import { Inter, JetBrains_Mono, Manrope } from "next/font/google";
import Link from "next/link";
import { WEB_APP_CONFIG } from "@/lib/template";
import "./globals.css";

const inter = Inter({
  variable: "--font-inter",
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
});

const manrope = Manrope({
  variable: "--font-manrope",
  subsets: ["latin"],
  weight: ["400", "500", "600", "700", "800"],
});

const jetbrainsMono = JetBrains_Mono({
  variable: "--font-jetbrains-mono",
  subsets: ["latin"],
  weight: ["400", "500", "600"],
});

export const metadata: Metadata = {
  title: `${WEB_APP_CONFIG.displayName} — ${WEB_APP_CONFIG.dashboardTitle}`,
  description: WEB_APP_CONFIG.description,
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark">
      <body
        className={`${inter.variable} ${manrope.variable} ${jetbrainsMono.variable} antialiased`}
        style={{ background: "var(--aurora-page-bg)", color: "var(--aurora-text-primary)" }}
      >
        <div className="min-h-screen flex flex-col">
          {/* Nav */}
          <nav
            style={{
              background: "var(--aurora-nav-bg)",
              borderBottom: "1px solid var(--aurora-border-default)",
            }}
            className="px-6 py-3 flex items-center gap-6"
          >
            <span
              style={{
                color: "var(--aurora-accent-primary)",
                fontFamily: "var(--aurora-font-display)",
              }}
              className="font-bold text-lg tracking-tight"
            >
              {WEB_APP_CONFIG.displayName}
            </span>
            <div className="flex gap-1">
              <NavLink href="/">Dashboard</NavLink>
              <NavLink href="/tools/">Tools</NavLink>
              <NavLink href="/api/">API</NavLink>
            </div>
          </nav>

          {/* Main */}
          <main className="flex-1 p-6">{children}</main>
        </div>
      </body>
    </html>
  );
}

function NavLink({ href, children }: { href: string; children: React.ReactNode }) {
  return (
    <Link
      href={href}
      className="nav-link"
      style={{
        color: "var(--aurora-text-muted)",
        borderRadius: "var(--radius-md)",
        padding: "0.25rem 0.75rem",
        fontSize: "0.875rem",
        fontWeight: 500,
        textDecoration: "none",
        transition: "color 0.15s, background 0.15s",
      }}
    >
      {children}
    </Link>
  );
}
