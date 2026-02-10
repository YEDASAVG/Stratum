"use client"

import { useCallback } from "react"
import { MoonStar, Sun, SunMoon } from "lucide-react"

import type { ModeType } from "@/types"

import { useSettings } from "@/hooks/use-settings"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"

const modeIcons = {
  light: Sun,
  dark: MoonStar,
  system: SunMoon,
}

export function ModeDropdown() {
  const { settings, updateSettings } = useSettings()

  const mode = settings.mode
  const ModeIcon = modeIcons[mode]

  const setMode = useCallback(
    (modeName: ModeType) => {
      updateSettings({ ...settings, mode: modeName })
    },
    [settings, updateSettings]
  )

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label="Mode">
          <ModeIcon className="size-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuLabel>Mode</DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuRadioGroup value={mode}>
          <DropdownMenuRadioItem value="light" onClick={() => setMode("light")}>
            Light
          </DropdownMenuRadioItem>
          <DropdownMenuRadioItem value="dark" onClick={() => setMode("dark")}>
            Dark
          </DropdownMenuRadioItem>
          <DropdownMenuRadioItem
            value="system"
            onClick={() => setMode("system")}
          >
            System
          </DropdownMenuRadioItem>
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
