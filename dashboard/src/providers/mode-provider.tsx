"use client"

import { useEffect } from "react"

import type { ReactNode } from "react"

import { useIsDarkMode } from "@/hooks/use-mode"

const defaultModes = ["light", "dark"]

export function ModeProvider({ children }: { children: ReactNode }) {
  const isDarkMode = useIsDarkMode()
  const mode = isDarkMode ? "dark" : "light"

  useEffect(() => {
    const rootElement = document.documentElement

    // Update class names in the <html> tag
    rootElement.classList.remove(...defaultModes)
    rootElement.classList.add(mode)
  }, [mode])

  return <>{children}</>
}
