import type { NavigationType } from "@/types"

export const navigationsData: NavigationType[] = [
  {
    title: "Stratum",
    items: [
      {
        title: "Overview",
        href: "/",
        iconName: "LayoutDashboard",
      },
      {
        title: "Logs",
        href: "/logs",
        iconName: "ScrollText",
      },
      {
        title: "Chat",
        href: "/chat",
        iconName: "MessageSquare",
      },
      {
        title: "Anomalies",
        href: "/anomalies",
        iconName: "TriangleAlert",
      },
    ],
  },
]
