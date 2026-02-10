"use client"

import { useMedia } from "react-use"

const MOBILE_BREAKPOINT = 1024

export function useIsMobile() {
  const isMobile = useMedia(`(max-width: ${MOBILE_BREAKPOINT - 1}px)`)

  return isMobile
}
