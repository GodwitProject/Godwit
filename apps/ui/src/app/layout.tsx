// apps/ui/src/app/layout.tsx
import type { Metadata } from "next";
import "../styles/globals.css";
import { Shell } from "@/components/layout/Shell";
import { Providers } from "./providers";

export const metadata: Metadata = {
  title: "Godwit - LLM Proxy Admin",
  description: "Admin dashboard for Godwit LLM Proxy",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>
        <Providers>
          <Shell>{children}</Shell>
        </Providers>
      </body>
    </html>
  );
}
