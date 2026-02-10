"use client"

import { useMedia } from "react-use"

import { useSettings } from "@/hooks/use-settings"

export function useIsDarkMode() {
  const { settings } = useSettings()
  const isDarkModePreferred = useMedia("(prefers-color-scheme: dark)")

  let resolvedMode = settings.mode

  if (resolvedMode === "system") {
    resolvedMode = isDarkModePreferred ? "dark" : "light"
  }

  return resolvedMode === "dark"
}
