// Refer to Lucide documentation for more details https://lucide.dev/guide/packages/lucide-react
import { icons } from "lucide-react"

import type { DynamicIconNameType } from "@/types"
import type { LucideProps } from "lucide-react"

interface DynamicIconProps extends LucideProps {
  name: DynamicIconNameType
}

// Component to render a dynamic Lucide icon based on its name.
export function DynamicIcon({ name, ...props }: DynamicIconProps) {
  const LucideIcon = icons[name] // Dynamically retrieve the icon by name.

  // Return null if the icon name is invalid.
  if (!LucideIcon) return null

  return <LucideIcon {...props} />
}
