"use client"

import { useCallback, useEffect, useRef, useState } from "react"
import { Pause, Play, RefreshCw, Search } from "lucide-react"

import type { Log } from "@/lib/api"

import { fetchLogs, fetchServices, searchLogs } from "@/lib/api"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"

const LEVEL_VARIANTS: Record<
  string,
  "destructive" | "secondary" | "default" | "outline"
> = {
  Error: "destructive",
  Warn: "secondary",
  Info: "default",
  Debug: "outline",
}

export default function LogsPage() {
  const [logs, setLogs] = useState<Log[]>([])
  const [services, setServices] = useState<string[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [liveTail, setLiveTail] = useState(false)
  const intervalRef = useRef<NodeJS.Timeout | null>(null)

  // Filters
  const [search, setSearch] = useState("")
  const [selectedService, setSelectedService] = useState<string>("")
  const [selectedLevel, setSelectedLevel] = useState<string>("")

  const loadLogs = useCallback(async () => {
    setError(null)
    try {
      let data: Log[]
      if (search.trim()) {
        // Use semantic search when search text provided
        data = await searchLogs({
          query: search,
          service: selectedService || undefined,
          level: selectedLevel || undefined,
          limit: 100,
        })
      } else {
        // Use chronological recent logs
        data = await fetchLogs({
          service: selectedService || undefined,
          level: selectedLevel || undefined,
          limit: 100,
        })
      }
      setLogs(data)
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load logs")
    } finally {
      setLoading(false)
    }
  }, [search, selectedService, selectedLevel])

  useEffect(() => {
    fetchServices().then(setServices).catch(console.error)
  }, [])

  useEffect(() => {
    setLoading(true)
    loadLogs()
  }, [loadLogs])

  // Live tail: refresh every 2 seconds when enabled
  useEffect(() => {
    if (liveTail) {
      intervalRef.current = setInterval(loadLogs, 2000)
    } else if (intervalRef.current) {
      clearInterval(intervalRef.current)
      intervalRef.current = null
    }
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current)
    }
  }, [liveTail, loadLogs])

  const formatTimestamp = (ts: string) => {
    // Ensure timestamp is parsed as UTC if no timezone specified
    const date = new Date(ts.endsWith("Z") ? ts : `${ts}Z`)
    return date.toLocaleString("en-IN", {
      day: "2-digit",
      month: "short",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: true,
    })
  }

  return (
    <section className="container p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Logs</h1>
        <div className="flex gap-2">
          <Button
            variant={liveTail ? "default" : "outline"}
            size="sm"
            onClick={() => setLiveTail(!liveTail)}
          >
            {liveTail ? (
              <>
                <Pause className="h-4 w-4 mr-2" />
                Live
              </>
            ) : (
              <>
                <Play className="h-4 w-4 mr-2" />
                Live Tail
              </>
            )}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={loadLogs}
            disabled={loading}
          >
            <RefreshCw
              className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`}
            />
            Refresh
          </Button>
        </div>
      </div>

      {/* Filters */}
      <Card>
        <CardContent className="pt-4">
          <div className="flex flex-wrap gap-4">
            <div className="flex-1 min-w-[200px]">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Search logs..."
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  className="pl-9"
                />
              </div>
            </div>

            <select
              value={selectedService}
              onChange={(e) => setSelectedService(e.target.value)}
              className="px-3 py-2 border rounded-md bg-background"
            >
              <option value="">All Services</option>
              {services.map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </select>

            <select
              value={selectedLevel}
              onChange={(e) => setSelectedLevel(e.target.value)}
              className="px-3 py-2 border rounded-md bg-background"
            >
              <option value="">All Levels</option>
              <option value="Error">Error</option>
              <option value="Warn">Warn</option>
              <option value="Info">Info</option>
              <option value="Debug">Debug</option>
            </select>
          </div>
        </CardContent>
      </Card>

      {/* Log Table */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm text-muted-foreground">
            {logs.length} logs
          </CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          {error ? (
            <div className="p-4 text-destructive">{error}</div>
          ) : loading ? (
            <div className="p-4 space-y-2">
              {[...Array(5)].map((_, i) => (
                <div key={i} className="h-12 bg-muted rounded animate-pulse" />
              ))}
            </div>
          ) : logs.length === 0 ? (
            <div className="p-8 text-center text-muted-foreground">
              No logs found. Try adjusting your filters.
            </div>
          ) : (
            <ScrollArea className="h-[600px]">
              <table className="w-full">
                <thead className="sticky top-0 bg-background border-b">
                  <tr className="text-left text-sm text-muted-foreground">
                    <th className="p-3 font-medium">Timestamp</th>
                    <th className="p-3 font-medium">Level</th>
                    <th className="p-3 font-medium">Service</th>
                    <th className="p-3 font-medium">Message</th>
                  </tr>
                </thead>
                <tbody className="divide-y">
                  {logs.map((log) => (
                    <tr key={log.id} className="hover:bg-muted/50">
                      <td className="p-3 text-sm text-muted-foreground whitespace-nowrap">
                        {formatTimestamp(log.timestamp)}
                      </td>
                      <td className="p-3">
                        <Badge variant={LEVEL_VARIANTS[log.level] || "default"}>
                          {log.level}
                        </Badge>
                      </td>
                      <td className="p-3 text-sm font-medium">{log.service}</td>
                      <td className="p-3 text-sm font-mono truncate max-w-[400px]">
                        {log.message}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </ScrollArea>
          )}
        </CardContent>
      </Card>
    </section>
  )
}
