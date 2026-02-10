const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3000/api"

export interface Log {
  id: string
  timestamp: string
  level: string
  service: string
  message: string
  source_file?: string
  source_line?: number
}

export interface ChatResponse {
  answer: string
  sources_count: number
  response_time_ms: number
  provider: string
  context_logs: number
  conversation_turn: number
  source_logs: string[]
}

// Generate a unique session ID for this browser session
let sessionId: string | null = null
function getSessionId(): string {
  if (!sessionId) {
    sessionId = `dashboard-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`
  }
  return sessionId
}

export interface Anomaly {
  service: string
  rule: string
  severity: string
  message: string
  current_value: number
  expected_value: number
}

export interface AnomaliesResponse {
  anomalies: Anomaly[]
  checked_at: string
}

export interface Stats {
  total_logs: number
  logs_24h: number
  error_count: number
  services_count: number
  embeddings_count: number
  storage_mb: number
}

export async function fetchLogs(params?: {
  service?: string
  level?: string
  search?: string
  limit?: number
  offset?: number
}): Promise<Log[]> {
  const searchParams = new URLSearchParams()
  if (params?.limit) searchParams.set("limit", params.limit.toString())
  if (params?.service) searchParams.set("service", params.service)
  if (params?.level) searchParams.set("level", params.level)

  // Use the new /logs/recent endpoint for chronological ordering
  const res = await fetch(`${API_BASE}/logs/recent?${searchParams}`)
  if (!res.ok) throw new Error("Failed to fetch logs")
  const data = await res.json()
  return data.map(
    (item: {
      log_id: string
      service: string
      level: string
      message: string
      timestamp: string
    }) => ({
      id: item.log_id,
      service: item.service,
      level: item.level,
      message: item.message,
      timestamp: item.timestamp,
    })
  )
}

// Semantic search for logs (returns by relevance)
export async function searchLogs(params: {
  query: string
  service?: string
  level?: string
  limit?: number
}): Promise<Log[]> {
  const searchParams = new URLSearchParams()
  searchParams.set("q", params.query)
  if (params.service) searchParams.set("service", params.service)
  if (params.level) searchParams.set("level", params.level)
  if (params.limit) searchParams.set("limit", params.limit.toString())

  const res = await fetch(`${API_BASE}/search?${searchParams}`)
  if (!res.ok) throw new Error("Failed to search logs")
  const data = await res.json()
  return data.map(
    (item: {
      log_id: string
      service: string
      level: string
      message: string
      timestamp: string
    }) => ({
      id: item.log_id,
      service: item.service,
      level: item.level,
      message: item.message,
      timestamp: item.timestamp,
    })
  )
}

export async function fetchStats(): Promise<Stats> {
  const res = await fetch(`${API_BASE}/stats`)
  if (!res.ok) throw new Error("Failed to fetch stats")
  return res.json()
}

export async function fetchAnomalies(): Promise<AnomaliesResponse> {
  const res = await fetch(`${API_BASE}/anomalies`)
  if (!res.ok) throw new Error("Failed to fetch anomalies")
  return res.json()
}

export async function sendChat(query: string): Promise<ChatResponse> {
  const res = await fetch(`${API_BASE}/chat`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      session_id: getSessionId(),
      message: query,
    }),
  })
  if (!res.ok) throw new Error("Failed to send chat")
  return res.json()
}

export async function fetchServices(): Promise<string[]> {
  const res = await fetch(`${API_BASE}/services`)
  if (!res.ok) throw new Error("Failed to fetch services")
  return res.json()
}
