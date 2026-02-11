"use client"

import { useEffect, useRef, useState } from "react"
import ReactMarkdown from "react-markdown"
import {
  AlertTriangle,
  ArrowDown,
  Bot,
  ChevronDown,
  ChevronRight,
  Loader2,
  Send,
  User,
  Zap,
} from "lucide-react"

import type { CausalChain, ChatResponse } from "@/lib/api"

import { sendChat } from "@/lib/api"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"

interface Message {
  role: "user" | "assistant"
  content: string
  metadata?: {
    sourcesCount: number
    sourceLogs: string[]
    responseTime: number
    provider: string
    contextLogs: number
    causalChain?: CausalChain
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

  // Scroll to bottom only when new messages arrive
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [messages.length]) // Only trigger on message count change, not content

  // When expanding sources, ensure the message stays visible
  useEffect(() => {
    if (scrollRef.current && expandedSources.size > 0) {
      // Use requestAnimationFrame to wait for DOM update
      requestAnimationFrame(() => {
        const expandedIndex = Array.from(expandedSources).pop()
        const messageEl = scrollRef.current?.querySelector(
          `[data-message-index="${expandedIndex}"]`
        )
        if (messageEl && scrollRef.current) {
          const containerRect = scrollRef.current.getBoundingClientRect()
          const messageRect = messageEl.getBoundingClientRect()
          // If message top is above visible area, scroll to show it
          if (messageRect.top < containerRect.top) {
            messageEl.scrollIntoView({ behavior: "smooth", block: "start" })
          }
        }
      })
    }
  }, [expandedSources])

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
            causalChain: response.causal_chain,
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
    <section className="h-[calc(100vh-56px)] flex flex-col px-4 py-3">
      <Card className="flex-1 flex flex-col min-h-0 overflow-hidden">
        <CardHeader className="border-b flex-shrink-0 py-3 px-4">
          <CardTitle className="text-lg font-semibold">Chat with Your Logs</CardTitle>
        </CardHeader>

        <div className="flex-1 min-h-0 overflow-y-auto p-4" ref={scrollRef}>
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
                  data-message-index={index}
                  className={`flex gap-3 ${
                    message.role === "user" ? "justify-end" : "justify-start"
                  }`}
                >
                  {message.role === "assistant" && (
                    <div className="h-7 w-7 rounded-full bg-primary/10 flex items-center justify-center flex-shrink-0">
                      <Bot className="h-4 w-4 text-primary" />
                    </div>
                  )}
                  <div
                    className={`max-w-[90%] rounded-lg p-3 ${
                      message.role === "user"
                        ? "bg-primary text-primary-foreground"
                        : "bg-muted"
                    }`}
                  >
                    {message.role === "assistant" ? (
                      <div className="prose prose-sm dark:prose-invert max-w-none text-sm leading-relaxed prose-p:my-1 prose-ul:my-1 prose-li:my-0 prose-pre:bg-background/50 prose-pre:text-xs prose-code:text-xs prose-code:before:content-none prose-code:after:content-none">
                        <ReactMarkdown>{message.content}</ReactMarkdown>
                      </div>
                    ) : (
                      <p className="whitespace-pre-wrap text-sm">{message.content}</p>
                    )}
                    {message.metadata && (
                      <div className="mt-2 pt-2 border-t border-border/50 text-xs space-y-2">
                        <div className="flex gap-2 flex-wrap text-muted-foreground">
                          <Badge variant="outline" className="text-xs">
                            {message.metadata.provider}
                          </Badge>
                          <Badge variant="outline" className="text-xs">
                            {message.metadata.responseTime}ms
                          </Badge>
                          <Badge variant="outline" className="text-xs">
                            {message.metadata.contextLogs} logs
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
                              <div className="mt-2 p-2 bg-background/50 rounded text-xs font-mono max-h-32 overflow-y-auto space-y-1">
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
                        {/* Causal Chain Visualization */}
                        {message.metadata.causalChain && (
                          <div className="mt-3 p-3 bg-muted/50 rounded-lg border border-border">
                            <div className="flex items-center gap-2 mb-2">
                              <Zap className="h-4 w-4 text-foreground" />
                              <span className="font-semibold text-sm text-foreground">
                                Root Cause Analysis
                              </span>
                            </div>

                            {/* The Effect (crash/error) */}
                            <div className="flex items-start gap-2 p-2 bg-background rounded border border-border mb-2">
                              <AlertTriangle className="h-4 w-4 text-muted-foreground mt-0.5 flex-shrink-0" />
                              <div className="flex-1 min-w-0">
                                <div className="text-xs text-muted-foreground font-medium uppercase tracking-wide">
                                  Effect
                                </div>
                                <div className="text-sm mt-1">
                                  {message.metadata.causalChain.effect.message}
                                </div>
                                <div className="text-xs text-muted-foreground mt-1">
                                  {message.metadata.causalChain.effect.service}{" "}
                                  • {message.metadata.causalChain.effect.level}
                                </div>
                              </div>
                            </div>

                            {/* Causal Chain Links */}
                            {message.metadata.causalChain.chain.map(
                              (link, i) => (
                                <div key={i} className="ml-4">
                                  <div className="flex items-center gap-1 py-1 text-muted-foreground">
                                    <ArrowDown className="h-3 w-3" />
                                    <span className="text-xs">
                                      {Math.round(link.confidence * 100)}%
                                    </span>
                                  </div>
                                  <div className="flex items-start gap-2 p-2 bg-background rounded border border-border">
                                    <div className="w-5 h-5 rounded-full bg-muted flex items-center justify-center flex-shrink-0 mt-0.5">
                                      <span className="text-xs text-muted-foreground">
                                        {i + 1}
                                      </span>
                                    </div>
                                    <div className="flex-1 min-w-0">
                                      <div className="text-sm">
                                        {link.cause.message}
                                      </div>
                                      <div className="text-xs text-muted-foreground mt-1">
                                        {link.cause.service} •{" "}
                                        {link.cause.level}
                                      </div>
                                      <div className="text-xs text-muted-foreground mt-1 italic">
                                        {link.explanation}
                                      </div>
                                    </div>
                                  </div>
                                </div>
                              )
                            )}

                            {/* Root Cause */}
                            {message.metadata.causalChain.root_cause && (
                              <div className="ml-4">
                                <div className="flex items-center gap-1 py-1">
                                  <ArrowDown className="h-3 w-3 text-muted-foreground" />
                                  <span className="text-xs font-medium text-muted-foreground uppercase">
                                    Root Cause
                                  </span>
                                </div>
                                <div className="flex items-start gap-2 p-2 bg-background rounded border border-border">
                                  <Zap className="h-4 w-4 text-muted-foreground mt-0.5 flex-shrink-0" />
                                  <div className="flex-1 min-w-0">
                                    <div className="text-sm">
                                      {
                                        message.metadata.causalChain.root_cause
                                          .message
                                      }
                                    </div>
                                    <div className="text-xs text-muted-foreground mt-1">
                                      {
                                        message.metadata.causalChain.root_cause
                                          .service
                                      }{" "}
                                      •{" "}
                                      {
                                        message.metadata.causalChain.root_cause
                                          .level
                                      }
                                    </div>
                                  </div>
                                </div>
                              </div>
                            )}

                            {/* Recommendation */}
                            {message.metadata.causalChain.recommendation && (
                              <div className="mt-2 pt-2 border-t border-border">
                                <div className="p-2 bg-background rounded border border-border text-sm prose prose-sm dark:prose-invert max-w-none">
                                  <span className="font-medium text-muted-foreground">
                                    💡 Recommendation:
                                  </span>
                                  <ReactMarkdown>{message.metadata.causalChain.recommendation}</ReactMarkdown>
                                </div>
                              </div>
                            )}
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                  {message.role === "user" && (
                    <div className="h-7 w-7 rounded-full bg-primary flex items-center justify-center flex-shrink-0">
                      <User className="h-4 w-4 text-primary-foreground" />
                    </div>
                  )}
                </div>
              ))
            )}
            {loading && (
              <div className="flex gap-3">
                <div className="h-7 w-7 rounded-full bg-primary/10 flex items-center justify-center">
                  <Bot className="h-4 w-4 text-primary" />
                </div>
                <div className="bg-muted rounded-lg p-3">
                  <Loader2 className="h-4 w-4 animate-spin" />
                </div>
              </div>
            )}
          </div>
        </div>

        <CardContent className="border-t p-3 flex-shrink-0">
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
