import { clsx } from "clsx"
import { twMerge } from "tailwind-merge"

import type { ClassValue } from "clsx"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function getInitials(fullName: string) {
  if (fullName.length === 0) return ""

  // Split the name by spaces
  const names = fullName.split(" ")
  // Extract the first letter of each name and convert it to uppercase
  const initials = names.map((name) => name.charAt(0).toUpperCase()).join("")

  return initials
}

export function remToPx(rem: number) {
  // Get the root font size (default is 16px if not set otherwise)
  const rootFontSize = parseFloat(
    getComputedStyle(document.documentElement).fontSize
  )
  return rem * rootFontSize
}

export function isActivePathname(
  basePathname: string,
  currentPathname: string,
  exactMatch: boolean = false
) {
  if (typeof basePathname !== "string" || typeof currentPathname !== "string") {
    throw new Error("Both basePathname and currentPathname must be strings")
  }

  // Use this when you want a strict comparison, e.g., highlighting a specific page.
  if (exactMatch) {
    return basePathname === currentPathname
  }

  // Allow deeper routes to be considered as active.
  // Example: If basePathname is "/dashboard", it should match "/dashboard/stats".
  return (
    currentPathname.startsWith(basePathname) &&
    (currentPathname.length === basePathname.length ||
      currentPathname[basePathname.length] === "/")
  )
}

export function formatFileSize(bytes: number, decimals: number = 2) {
  if (bytes === 0) return "0 Bytes"

  const k = 1000 // Use 1024 for binary
  const dm = decimals < 0 ? 0 : decimals
  const sizes = ["Bytes", "KB", "MB", "GB", "TB", "PB"]

  const i = Math.floor(Math.log(bytes) / Math.log(k))

  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + " " + sizes[i]
}
