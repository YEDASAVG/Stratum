"use client"

import * as SeparatorPrimitive from "@radix-ui/react-separator"

import type { ComponentProps } from "react"

import { cn } from "@/lib/utils"

export function Separator({
  className,
  orientation = "horizontal",
  decorative = true,
  ...props
}: ComponentProps<typeof SeparatorPrimitive.Root>) {
  return (
    <SeparatorPrimitive.Root
      data-slot="separator-root"
      decorative={decorative}
      orientation={orientation}
      className={cn(
        "shrink-0 bg-border",
        orientation === "horizontal" ? "h-[1px] w-full" : "h-full w-[1px]",
        className
      )}
      {...props}
    />
  )
}

export function SeparatorWithText({
  className,
  orientation = "horizontal",
  decorative = true,
  children,
  ...props
}: ComponentProps<typeof SeparatorPrimitive.Root>) {
  return (
    <div
      data-slot="separator-with-text"
      className={cn(
        "flex justify-between items-center",
        orientation === "horizontal" ? "w-full" : "flex-col h-full",
        className
      )}
    >
      <Separator
        decorative={decorative}
        orientation={orientation}
        className="shrink"
        {...props}
      />
      <span
        className={cn(
          "shrink-0 px-2 text-sm text-muted-foreground uppercase",
          orientation === "vertical" && "-rotate-90 rtl:rotate-90"
        )}
      >
        {children}
      </span>
      <Separator
        decorative={decorative}
        orientation={orientation}
        className="shrink"
        {...props}
      />
    </div>
  )
}
