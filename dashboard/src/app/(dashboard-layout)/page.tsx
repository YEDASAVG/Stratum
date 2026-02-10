"use client"

import { useCallback, useEffect, useState } from "react"
import { AlertTriangle, Database, FileText, Server } from "lucide-react"

import type { Stats } from "@/lib/api"

import { fetchStats } from "@/lib/api"

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"

export default function OverviewPage() {
  const [stats, setStats] = useState<Stats | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const loadStats = useCallback(() => {
    fetchStats()
      .then(setStats)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false))
  }, [])

  useEffect(() => {
    loadStats()
  }, [loadStats])

  // Auto-refresh stats every 30 seconds (stats don't change as fast)
  useEffect(() => {
    const interval = setInterval(loadStats, 30000)
    return () => clearInterval(interval)
  }, [loadStats])

  if (loading) {
    return (
      <section className="container p-6">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-muted rounded w-1/4"></div>
          <div className="grid grid-cols-4 gap-4">
            {[...Array(4)].map((_, i) => (
              <div key={i} className="h-32 bg-muted rounded"></div>
            ))}
          </div>
        </div>
      </section>
    )
  }

  if (error) {
    return (
      <section className="container p-6">
        <Card className="border-destructive">
          <CardHeader>
            <CardTitle className="text-destructive">
              Error Loading Stats
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p>{error}</p>
            <p className="text-sm text-muted-foreground mt-2">
              Make sure the API server is running on port 3000
            </p>
          </CardContent>
        </Card>
      </section>
    )
  }

  const errorRate =
    stats && stats.total_logs > 0
      ? ((stats.error_count / stats.total_logs) * 100).toFixed(1)
      : "0"

  return (
    <section className="container p-6 space-y-6">
      <h1 className="text-2xl font-bold">Overview</h1>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Total Logs</CardTitle>
            <FileText className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {stats?.total_logs.toLocaleString()}
            </div>
            <p className="text-xs text-muted-foreground">
              {stats?.logs_24h.toLocaleString()} in last 24h
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Services</CardTitle>
            <Server className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {stats?.services_count || 0}
            </div>
            <p className="text-xs text-muted-foreground">Active services</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Errors</CardTitle>
            <AlertTriangle className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-red-500">
              {stats?.error_count.toLocaleString()}
            </div>
            <p className="text-xs text-muted-foreground">
              {errorRate}% error rate
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Embeddings</CardTitle>
            <Database className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {stats?.embeddings_count.toLocaleString()}
            </div>
            <p className="text-xs text-muted-foreground">
              {stats?.storage_mb.toFixed(1)} MB storage
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Info Card */}
      <Card>
        <CardHeader>
          <CardTitle>System Status</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            <div>
              <p className="text-muted-foreground">API Status</p>
              <p className="font-medium text-green-500">● Online</p>
            </div>
            <div>
              <p className="text-muted-foreground">Vector DB</p>
              <p className="font-medium text-green-500">● Connected</p>
            </div>
            <div>
              <p className="text-muted-foreground">ClickHouse</p>
              <p className="font-medium text-green-500">● Connected</p>
            </div>
            <div>
              <p className="text-muted-foreground">LLM Provider</p>
              <p className="font-medium">Groq</p>
            </div>
          </div>
        </CardContent>
      </Card>
    </section>
  )
}
