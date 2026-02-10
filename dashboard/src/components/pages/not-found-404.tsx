import Image from "next/image"
import Link from "next/link"

import { Button } from "@/components/ui/button"

export function NotFound404() {
  return (
    <div className="min-h-screen w-full flex flex-col items-center justify-center gap-y-6 text-center text-foreground bg-background p-4">
      <div className="flex flex-col-reverse justify-center items-center gap-y-6 md:flex-row md:text-start">
        <Image
          src="/images/illustrations/characters/character-02.svg"
          alt=""
          height={232}
          width={249}
          priority
        />

        <h1 className="inline-grid text-6xl font-black">
          404 <span className="text-3xl font-semibold">Page Not Found</span>
        </h1>
      </div>
      <p className="max-w-prose text-xl text-muted-foreground">
        We couldn&apos;t find the page you&apos;re looking for. It might have
        been moved or doesn&apos;t exist.
      </p>
      <Button size="lg" asChild>
        <Link href="/">Home Page</Link>
      </Button>
    </div>
  )
}
