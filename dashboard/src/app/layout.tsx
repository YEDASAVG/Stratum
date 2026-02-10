import { Poppins } from "next/font/google"

import { cn } from "@/lib/utils"

import "./globals.css"

import { Providers } from "@/providers"

import type { Metadata } from "next"
import type { ReactNode } from "react"

import { Toaster as Sonner } from "@/components/ui/sonner"
import { Toaster } from "@/components/ui/toaster"

// Define metadata for the application
// More info: https://nextjs.org/docs/app/building-your-application/optimizing/metadata
export const metadata: Metadata = {
  title: {
    template: "%s | Stratum",
    default: "Stratum - Layers of Insight",
  },
  description: "Self-hosted intelligent log analysis with AI chat",
}

// Define fonts for the application
// More info: https://nextjs.org/docs/app/building-your-application/optimizing/fonts
const poppinsFont = Poppins({
  subsets: ["latin"],
  weight: ["100", "200", "300", "400", "500", "600", "700", "800", "900"],
  style: ["normal", "italic"],
  variable: "--font-poppins",
})

export default function RootLayout(props: { children: ReactNode }) {
  const { children } = props

  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <body
        className={cn(
          "font-poppins",
          "bg-background text-foreground antialiased overscroll-none",
          poppinsFont.variable
        )}
      >
        <Providers locale="en">
          {children}
          <Toaster />
          <Sonner />
        </Providers>
      </body>
    </html>
  )
}
