"use client"

import { Toaster as Sonner } from "sonner"

import type { ComponentProps } from "react"

import { useSettings } from "@/hooks/use-settings"

type ToasterProps = ComponentProps<typeof Sonner>

export function Toaster({ ...props }: ToasterProps) {
  const { settings } = useSettings()

  const mode = settings.mode

  return (
    <Sonner
      theme={mode as ToasterProps["theme"]}
      className="toaster group"
      toastOptions={{
        classNames: {
          toast:
            "group toast group-[.toaster]:bg-background group-[.toaster]:text-foreground group-[.toaster]:border-border group-[.toaster]:shadow-lg",
          description: "group-[.toast]:text-muted-foreground",
          actionButton:
            "group-[.toast]:bg-primary group-[.toast]:text-primary-foreground",
          cancelButton:
            "group-[.toast]:bg-muted group-[.toast]:text-muted-foreground",
        },
      }}
      {...props}
    />
  )
}
