"use client"

import { useEffect, useState } from "react"

import { Button } from "@/components/ui/button"
import { DynamicIcon } from "@/components/dynamic-icon"

// Extend the global `Document` and `HTMLElement` interfaces to handle fullscreen API variations across browsers
declare global {
  interface Document {
    webkitExitFullscreen?: () => Promise<void>
    msExitFullscreen?: () => Promise<void>
    webkitFullscreenElement?: Element | null
    msFullscreenElement?: Element | null
  }

  interface HTMLElement {
    webkitRequestFullscreen?: () => Promise<void>
    msRequestFullscreen?: () => Promise<void>
  }
}

export function FullscreenToggle() {
  const [isFullscreen, setIsFullscreen] = useState(false)

  const toggleFullscreen = () => {
    const element = document.documentElement

    // If fullscreen mode is not active, activate it
    if (!isFullscreen) {
      if (element.requestFullscreen) {
        // Standard fullscreen API
        element.requestFullscreen()
      } else if (element.webkitRequestFullscreen) {
        // For Safari
        element.webkitRequestFullscreen()
      } else if (element.msRequestFullscreen) {
        // For IE/Edge
        element.msRequestFullscreen()
      } else {
        alert("Fullscreen mode is not supported in this browser.")
      }

      // If fullscreen mode is active, deactivate it
    } else {
      if (document.exitFullscreen) {
        // Standard fullscreen API
        document.exitFullscreen()
      } else if (document.webkitExitFullscreen) {
        // For Safari
        document.webkitExitFullscreen()
      } else if (document.msExitFullscreen) {
        // For IE/Edge
        document.msExitFullscreen()
      }
    }
  }

  const handleFullscreenChange = () => {
    // Update the fullscreen state when fullscreen changes
    setIsFullscreen(
      !!document.fullscreenElement ||
        !!document.webkitFullscreenElement ||
        !!document.msFullscreenElement
    )
  }

  useEffect(() => {
    // Add event listeners for fullscreen changes across various browsers
    document.addEventListener("fullscreenchange", handleFullscreenChange)
    document.addEventListener("webkitfullscreenchange", handleFullscreenChange)
    document.addEventListener("msfullscreenchange", handleFullscreenChange)

    // Cleanup event listeners to avoid memory leaks
    return () => {
      document.removeEventListener("fullscreenchange", handleFullscreenChange)
      document.removeEventListener(
        "webkitfullscreenchange",
        handleFullscreenChange
      )
      document.removeEventListener("msfullscreenchange", handleFullscreenChange)
    }
  }, [])

  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={toggleFullscreen}
      aria-label="Toggle Fullscreen"
      className="hidden md:inline-flex"
    >
      <DynamicIcon
        name={isFullscreen ? "Shrink" : "Expand"}
        className="size-4"
      />
    </Button>
  )
}
