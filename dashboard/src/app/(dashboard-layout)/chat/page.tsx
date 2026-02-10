"use client"

import { useEffect, useRef, useState } from "react"
import ReactMarkdown from "react-markdown"
import {
  Bot,
  ChevronDown,
  ChevronRight,
  Loader2,
  Send,
  User,
} from "lucide-react"

import type { ChatResponse } from "@/lib/api"

import { sendChat } from "@/lib/api"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"

interface Message {
  role: "user" | "assistant"
  content: string
  metadata?: {
    sourcesCount: number
    sourceLogs: string[]
    responseTime: number
    provider: string
    contextLogs: number
  }
}

export default function ChatPage() {
  const [messages, setMessages] = useState<Message[]>([])
  const [input, setInput] = useState("")
  const [loading, setLoading] = useState(false)
  const [expandedSources, setExpandedSources] = useState<Set<number>>(new Set())
  const scrollRef = useRef<HTMLDivElement>(null)

  const toggleSources = (index: number) => {
    setExpandedSources((prev) => {
      const next = new Set(prev)
      if (next.has(index)) next.delete(index)
      else next.add(index)
      return next
    })
  }

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [messages])

  const handleSend = async () => {
    if (!input.trim() || loading) return

    const userMessage = input.trim()
    setInput("")
    setMessages((prev) => [...prev, { role: "user", content: userMessage }])
    setLoading(true)

    try {
      const response: ChatResponse = await sendChat(userMessage)
      setMessages((prev) => [
        ...prev,
        {
          role: "assistant",
          content: response.answer,
          metadata: {
            sourcesCount: response.sources_count,
            sourceLogs: response.source_logs,
            responseTime: response.response_time_ms,
            provider: response.provider,
            contextLogs: response.context_logs,
          },
        },
      ])
    } catch (error) {
      setMessages((prev) => [
        ...prev,
        {
          role: "assistant",
          content: `Error: ${error instanceof Error ? error.message : "Failed to get response"}. Make sure the API is running.`,
        },
      ])
    } finally {
      setLoading(false)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  return (
    <section className="container p-6 h-[calc(100vh-80px)] flex flex-col">
      <h1 className="text-2xl font-bold mb-4">Chat with Your Logs</h1>

      <Card className="flex-1 flex flex-col">
        <CardHeader className="border-b">
          <CardTitle className="text-sm text-muted-foreground">
            Ask questions about your logs in natural language
          </CardTitle>
        </CardHeader>

        <ScrollArea className="flex-1 p-4" ref={scrollRef}>
          <div className="space-y-4">
            {messages.length === 0 ? (
              <div className="text-center py-12 text-muted-foreground">
                <Bot className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <p className="text-lg mb-2">No messages yet</p>
                <p className="text-sm">Try asking questions like:</p>
                <div className="mt-4 space-y-2">
                  <Badge
                    variant="outline"
                    className="mx-1 cursor-pointer"
                    onClick={() =>
                      setInput("What errors occurred in the last hour?")
                    }
                  >
                    What errors occurred in the last hour?
                  </Badge>
                  <Badge
                    variant="outline"
                    className="mx-1 cursor-pointer"
                    onClick={() => setInput("Show me authentication failures")}
                  >
                    Show me authentication failures
                  </Badge>
                  <Badge
                    variant="outline"
                    className="mx-1 cursor-pointer"
                    onClick={() =>
                      setInput("Which service has the most errors?")
                    }
                  >
                    Which service has the most errors?
                  </Badge>
                </div>
              </div>
            ) : (
              messages.map((message, index) => (
                <div
                  key={index}
                  className={`flex gap-3 ${
                    message.role === "user" ? "justify-end" : "justify-start"
                  }`}
                >
                  {message.role === "assistant" && (
                    <div className="h-8 w-8 rounded-full bg-primary/10 flex items-center justify-center flex-shrink-0">
                      <Bot className="h-4 w-4 text-primary" />
                    </div>
                  )}
                  <div
                    className={`max-w-[85%] rounded-lg p-4 ${
                      message.role === "user"
                        ? "bg-primary text-primary-foreground"
                        : "bg-muted"
                    }`}
                  >
                    {message.role === "assistant" ? (
                      <div className="prose prose-sm dark:prose-invert max-w-none prose-p:my-1 prose-ul:my-1 prose-li:my-0 prose-pre:bg-background/50 prose-pre:text-xs prose-code:text-xs prose-code:before:content-none prose-code:after:content-none">
                        <ReactMarkdown>{message.content}</ReactMarkdown>
                      </div>
                    ) : (
                      <p className="whitespace-pre-wrap">{message.content}</p>
                    )}
                    {message.metadata && (
                      <div className="mt-3 pt-3 border-t border-border/50 text-xs space-y-2">
                        <div className="flex gap-2 flex-wrap text-muted-foreground">
                          <Badge variant="outline" className="text-xs">
                            {message.metadata.provider}
                          </Badge>
                          <Badge variant="outline" className="text-xs">
                            {message.metadata.responseTime}ms
                          </Badge>
                          <Badge variant="outline" className="text-xs">
                            {message.metadata.contextLogs} logs analyzed
                          </Badge>
                        </div>
                        {message.metadata.sourceLogs.length > 0 && (
                          <div>
                            <button
                              onClick={() => toggleSources(index)}
                              className="flex items-center gap-1 text-muted-foreground hover:text-foreground transition-colors"
                            >
                              {expandedSources.has(index) ? (
                                <ChevronDown className="h-3 w-3" />
                              ) : (
                                <ChevronRight className="h-3 w-3" />
                              )}
                              View source logs (
                              {message.metadata.sourceLogs.length})
                            </button>
                            {expandedSources.has(index) && (
                              <div className="mt-2 p-2 bg-background/50 rounded text-xs font-mono max-h-40 overflow-y-auto space-y-1">
                                {message.metadata.sourceLogs.map((log, i) => (
                                  <div
                                    key={i}
                                    className="text-muted-foreground truncate hover:text-clip hover:whitespace-normal"
                                  >
                                    {log}
                                  </div>
                                ))}
                              </div>
                            )}
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                  {message.role === "user" && (
                    <div className="h-8 w-8 rounded-full bg-primary flex items-center justify-center flex-shrink-0">
                      <User className="h-4 w-4 text-primary-foreground" />
                    </div>
                  )}
                </div>
              ))
            )}
            {loading && (
              <div className="flex gap-3">
                <div className="h-8 w-8 rounded-full bg-primary/10 flex items-center justify-center">
                  <Bot className="h-4 w-4 text-primary" />
                </div>
                <div className="bg-muted rounded-lg p-4">
                  <Loader2 className="h-5 w-5 animate-spin" />
                </div>
              </div>
            )}
          </div>
        </ScrollArea>

        <CardContent className="border-t p-4">
          <div className="flex gap-2">
            <Input
              placeholder="Ask about your logs..."
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              disabled={loading}
              className="flex-1"
            />
            <Button onClick={handleSend} disabled={loading || !input.trim()}>
              {loading ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Send className="h-4 w-4" />
              )}
            </Button>
          </div>
        </CardContent>
      </Card>
    </section>
  )
}
