"use client"

import { useEffect } from "react"

import type { ReactNode } from "react"

import { useSettings } from "@/hooks/use-settings"

export function ThemeProvider({ children }: { children: ReactNode }) {
  const { settings } = useSettings()

  useEffect(() => {
    const bodyElement = document.body

    // Update class names in the <body> tag
    Array.from(bodyElement.classList)
      .filter(
        (className) =>
          className.startsWith("theme-") || className.startsWith("radius-")
      )
      .forEach((className) => {
        bodyElement.classList.remove(className)
      })

    bodyElement.classList.add(`theme-${settings.theme}`)
    bodyElement.classList.add(`radius-${settings.radius ?? 0.5}`)
  }, [settings.theme, settings.radius])

  return <>{children}</>
}
