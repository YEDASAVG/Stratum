import { Slot } from "@radix-ui/react-slot"
import { cva } from "class-variance-authority"
import { LoaderCircle } from "lucide-react"

import type { IconType } from "@/types"
import type { VariantProps } from "class-variance-authority"
import type { ComponentProps } from "react"

import { cn } from "@/lib/utils"

export const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors cursor-pointer focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/90",
        destructive:
          "bg-destructive text-destructive-foreground hover:bg-destructive/90",
        outline:
          "border border-input bg-background hover:bg-accent hover:text-accent-foreground",
        secondary:
          "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        ghost: "hover:bg-accent hover:text-accent-foreground",
        link: "text-primary underline-offset-4 hover:underline",
      },
      size: {
        default: "h-9 px-4 py-2",
        sm: "h-8 rounded-md px-3 text-xs",
        lg: "h-10 rounded-md px-8",
        icon: "h-9 w-9",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

interface ButtonProps
  extends ComponentProps<"button">,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean
}

export function Button({
  className,
  variant,
  size,
  asChild = false,
  ...props
}: ButtonProps) {
  const Comp = asChild ? Slot : "button"

  return (
    <Comp
      data-slot="button"
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  )
}

interface ButtonLoadingProps extends ButtonProps {
  isLoading: boolean
  loadingIconClassName?: string
  iconClassName?: string
  icon?: IconType
}

export function ButtonLoading({
  isLoading,
  disabled,
  children,
  loadingIconClassName,
  iconClassName,
  icon: Icon,
  ...props
}: ButtonLoadingProps) {
  let RenderedIcon
  if (isLoading) {
    RenderedIcon = (
      <LoaderCircle
        className={cn("me-2 size-4 animate-spin", loadingIconClassName)}
        aria-hidden
      />
    )
  } else if (Icon) {
    RenderedIcon = (
      <Icon className={cn("me-2 size-4", iconClassName)} aria-hidden />
    )
  }

  return (
    <Button
      data-slot="button-loading"
      type="submit"
      disabled={isLoading || disabled}
      aria-live="assertive"
      aria-label={isLoading ? "Loading" : props["aria-label"]}
      {...props}
    >
      {RenderedIcon}
      {children}
    </Button>
  )
}
