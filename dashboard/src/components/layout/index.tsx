"use client"

import type { ReactNode } from "react"

import { VerticalLayout } from "./vertical-layout"

export function Layout({ children }: { children: ReactNode }) {
  return <VerticalLayout>{children}</VerticalLayout>
}
