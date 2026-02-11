"use client"

import { useEffect, useState } from "react"
import { AlertTriangle, RefreshCw, Server } from "lucide-react"

import type { Anomaly } from "@/lib/api"

import { fetchAnomalies } from "@/lib/api"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { ScrollArea } from "@/components/ui/scroll-area"

const SEVERITY_VARIANTS: Record<
  string,
  "destructive" | "secondary" | "default"
> = {
  critical: "destructive",
  high: "destructive",
  medium: "secondary",
  low: "default",
}

export default function AnomaliesPage() {
  const [anomalies, setAnomalies] = useState<Anomaly[]>([])
  const [checkedAt, setCheckedAt] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const loadAnomalies = async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await fetchAnomalies()
      setAnomalies(data.anomalies)
      setCheckedAt(data.checked_at)
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load anomalies")
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadAnomalies()
  }, [])

  return (
    <section className="container p-6 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Anomalies</h1>
          {checkedAt && (
            <p className="text-sm text-muted-foreground">
              Last checked: {new Date(checkedAt).toLocaleString()}
            </p>
          )}
        </div>
        <Button variant="outline" size="sm" onClick={loadAnomalies}>
          <RefreshCw
            className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`}
          />
          Refresh
        </Button>
      </div>

      {error ? (
        <Card className="border-destructive">
          <CardContent className="p-6">
            <p className="text-destructive">{error}</p>
            <p className="text-sm text-muted-foreground mt-2">
              Make sure the API server is running
            </p>
          </CardContent>
        </Card>
      ) : loading ? (
        <div className="space-y-4">
          {[...Array(3)].map((_, i) => (
            <Card key={i}>
              <CardContent className="p-6">
                <div className="animate-pulse space-y-3">
                  <div className="h-5 bg-muted rounded w-1/3"></div>
                  <div className="h-4 bg-muted rounded w-2/3"></div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      ) : anomalies.length === 0 ? (
        <Card>
          <CardContent className="p-12 text-center">
            <AlertTriangle className="h-12 w-12 mx-auto mb-4 text-muted-foreground opacity-50" />
            <h3 className="text-lg font-semibold mb-2">
              No Anomalies Detected
            </h3>
            <p className="text-muted-foreground">
              Great news! No anomalies have been detected in your logs.
            </p>
          </CardContent>
        </Card>
      ) : (
        <ScrollArea className="h-[calc(100vh-200px)]">
          <div className="space-y-4">
            {anomalies.map((anomaly, index) => (
              <Card
                key={`${anomaly.service}-${anomaly.rule}-${index}`}
                className="hover:shadow-md transition-shadow"
              >
                <CardHeader className="pb-2">
                  <div className="flex items-start justify-between">
                    <div className="flex items-center gap-2">
                      <AlertTriangle className="h-5 w-5 text-amber-500" />
                      <CardTitle className="text-base">
                        {anomaly.rule}
                      </CardTitle>
                    </div>
                    <Badge
                      variant={SEVERITY_VARIANTS[anomaly.severity] || "default"}
                    >
                      {anomaly.severity}
                    </Badge>
                  </div>
                </CardHeader>
                <CardContent>
                  <p className="text-sm mb-4">{anomaly.message}</p>
                  <div className="flex flex-wrap gap-4 text-sm text-muted-foreground">
                    <div className="flex items-center gap-1">
                      <Server className="h-4 w-4" />
                      {anomaly.service}
                    </div>
                    <div>
                      Current: {anomaly.current_value.toFixed(1)} / Expected:{" "}
                      {anomaly.expected_value.toFixed(1)}
                    </div>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        </ScrollArea>
      )}
    </section>
  )
}
