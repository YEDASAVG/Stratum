import type { ComponentProps } from "react"

import { cn } from "@/lib/utils"

export function Keyboard({
  className,
  children,
  ...props
}: ComponentProps<"kbd">) {
  return (
    <kbd
      className={cn(
        "pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground opacity-100",
        className
      )}
      {...props}
    >
      <span className="text-xs">⌘</span>
      {children}
    </kbd>
  )
}
