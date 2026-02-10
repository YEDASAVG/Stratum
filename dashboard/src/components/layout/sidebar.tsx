"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"
import { ChevronDown } from "lucide-react"

import type { NavigationNestedItem, NavigationRootItem } from "@/types"

import { navigationsData } from "@/data/navigations"

import { isActivePathname } from "@/lib/utils"

import { Badge } from "@/components/ui/badge"
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarTrigger,
  Sidebar as SidebarWrapper,
  useSidebar,
} from "@/components/ui/sidebar"
import { DynamicIcon } from "@/components/dynamic-icon"
import { CommandMenu } from "./command-menu"

export function Sidebar() {
  const pathname = usePathname()
  const { openMobile, setOpenMobile, isMobile } = useSidebar()

  const renderMenuItem = (item: NavigationRootItem | NavigationNestedItem) => {
    // If the item has nested items, render it with a collapsible dropdown.
    if (item.items) {
      return (
        <Collapsible className="group/collapsible">
          <CollapsibleTrigger asChild>
            <SidebarMenuButton className="w-full justify-between [&[data-state=open]>svg]:rotate-180">
              <span className="flex items-center">
                {"iconName" in item && (
                  <DynamicIcon name={item.iconName} className="me-2 h-4 w-4" />
                )}
                <span>{item.title}</span>
                {"label" in item && (
                  <Badge variant="secondary" className="me-2">
                    {item.label}
                  </Badge>
                )}
              </span>
              <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground transition-transform duration-200" />
            </SidebarMenuButton>
          </CollapsibleTrigger>
          <CollapsibleContent className="overflow-hidden data-[state=closed]:animate-collapsible-up data-[state=open]:animate-collapsible-down">
            <SidebarMenuSub>
              {item.items.map((subItem: NavigationNestedItem) => (
                <SidebarMenuItem key={subItem.title}>
                  {renderMenuItem(subItem)}
                </SidebarMenuItem>
              ))}
            </SidebarMenuSub>
          </CollapsibleContent>
        </Collapsible>
      )
    }

    // Otherwise, render the item with a link.
    if ("href" in item) {
      const isActive = isActivePathname(item.href, pathname)

      return (
        <SidebarMenuButton
          isActive={isActive}
          onClick={() => setOpenMobile(!openMobile)}
          asChild
        >
          <Link href={item.href}>
            {"iconName" in item && (
              <DynamicIcon name={item.iconName} className="h-4 w-4" />
            )}
            <span>{item.title}</span>
            {"label" in item && <Badge variant="secondary">{item.label}</Badge>}
          </Link>
        </SidebarMenuButton>
      )
    }
  }

  return (
    <SidebarWrapper side="left" collapsible="icon">
      <SidebarHeader>
        <div className="flex items-center justify-between p-2 pb-0 mb-2 group-data-[collapsible=icon]:justify-center">
          <Link
            href="/"
            className="flex items-center gap-2 text-foreground font-black group-data-[collapsible=icon]:hidden"
            onClick={() => isMobile && setOpenMobile(!openMobile)}
          >
            <span className="text-xl">◈</span>
            <span>Stratum</span>
          </Link>
          <SidebarTrigger className="hidden lg:flex" />
        </div>
        <CommandMenu buttonClassName="max-w-full group-data-[collapsible=icon]:hidden" />
      </SidebarHeader>
      <ScrollArea>
        <SidebarContent className="gap-0">
          {navigationsData.map((nav) => (
            <SidebarGroup key={nav.title}>
              <SidebarGroupLabel>{nav.title}</SidebarGroupLabel>
              <SidebarGroupContent>
                <SidebarMenu>
                  {nav.items.map((item) => (
                    <SidebarMenuItem key={item.title}>
                      {renderMenuItem(item)}
                    </SidebarMenuItem>
                  ))}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
          ))}
        </SidebarContent>
      </ScrollArea>
    </SidebarWrapper>
  )
}
