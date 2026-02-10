"use client"

import { FullscreenToggle } from "@/components/layout/full-screen-toggle"
import { ModeDropdown } from "@/components/layout/mode-dropdown"
import { ToggleMobileSidebar } from "../toggle-mobile-sidebar"

export function VerticalLayoutHeader() {
  return (
    <header className="sticky top-0 z-50 w-full bg-background border-b border-sidebar-border">
      <div className="container flex h-14 justify-between items-center gap-4">
        <ToggleMobileSidebar />
        <div className="grow flex justify-end gap-2">
          <FullscreenToggle />
          <ModeDropdown />
        </div>
      </div>
    </header>
  )
}
