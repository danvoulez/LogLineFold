"use client"

import type React from "react"
import { useCallback, useEffect, useMemo, useState } from "react"
import type { FoldingRun, FoldingSummary, RunListEntry, RunResponse } from "@/lib/types"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { DNAViewer } from "@/components/dna-viewer"
import { SpanTimeline } from "@/components/span-timeline"
import { AgentExplorer } from "@/components/agent-explorer"
import { DiscoveryPanel } from "@/components/discovery-panel"
import { UIThemeSystem } from "@/components/ui-theme-system"
import { Menu, X, RefreshCw, Dna, Bot, Lightbulb, Palette, Baseline as Timeline, AlertCircle } from "lucide-react"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"

interface Panel {
  id: string
  label: string
  component: string
  icon: React.ComponentType<any>
}

const panels: Panel[] = [
  {
    id: "dna_panel",
    label: "Genome",
    component: "sequence.viewer",
    icon: Dna,
  },
  {
    id: "span_panel",
    label: "Span Timeline",
    component: "span.timeline",
    icon: Timeline,
  },
  {
    id: "agent_panel",
    label: "Span Agents",
    component: "span.cohorts",
    icon: Bot,
  },
  {
    id: "discovery_panel",
    label: "Insights",
    component: "runtime.insights",
    icon: Lightbulb,
  },
  {
    id: "render_panel",
    label: "Theme",
    component: "ui.styler",
    icon: Palette,
  },
]

export default function LogLinePage() {
  const [activePanel, setActivePanel] = useState("span_panel")
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const [run, setRun] = useState<FoldingRun | null>(null)
  const [summary, setSummary] = useState<FoldingSummary | null>(null)
  const [runList, setRunList] = useState<RunListEntry[]>([])
  const [runListLoading, setRunListLoading] = useState(false)
  const [selectedRun, setSelectedRun] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [refreshing, setRefreshing] = useState(false)
  const [initialised, setInitialised] = useState(false)
  const [runTriggerPending, setRunTriggerPending] = useState(false)
  const [runTriggerMessage, setRunTriggerMessage] = useState<string | null>(null)

  const loadRun = useCallback(
    async (filename?: string, options: { markRefreshing?: boolean } = {}) => {
      if (options.markRefreshing) {
        setRefreshing(true)
      }
      setLoading(true)
      setError(null)
      try {
        const endpoint = filename
          ? `/api/runs/${encodeURIComponent(filename)}`
          : "/api/runs/latest"
        const response = await fetch(endpoint, { cache: "no-store" })
        if (!response.ok) {
          const payload = await response.json().catch(() => ({}))
          throw new Error(payload.message ?? `Failed to load run ${filename ?? "latest"}.`)
        }
        const payload = (await response.json()) as RunResponse
        setRun(payload.run)
        setSummary(payload.summary)
        setSelectedRun(payload.run.filename)
        setError(null)
      } catch (err) {
        setRun(null)
        setSummary(null)
        setError(err instanceof Error ? err.message : "Unexpected error")
      } finally {
        setLoading(false)
        if (options.markRefreshing) {
          setRefreshing(false)
        }
      }
    },
    [],
  )

  const fetchRunList = useCallback(async (): Promise<RunListEntry[]> => {
    setRunListLoading(true)
    try {
      const response = await fetch("/api/runs", { cache: "no-store" })
      if (!response.ok) {
        const payload = await response.json().catch(() => ({}))
        throw new Error(payload.message ?? "Unable to list runs.")
      }
      const payload = await response.json()
      const entries: RunListEntry[] = Array.isArray(payload.runs) ? payload.runs : []
      setRunList(entries)
      return entries
    } catch (err) {
      console.error("failed to fetch run list", err)
      setRunList([])
      if (!run) {
        setError(err instanceof Error ? err.message : "Unable to list runs")
      }
      return []
    } finally {
      setRunListLoading(false)
    }
  }, [run])

  useEffect(() => {
    fetchRunList()
  }, [fetchRunList])

  useEffect(() => {
    if (initialised) {
      return
    }
    if (!runListLoading && runList.length === 0) {
      setInitialised(true)
      setLoading(false)
      return
    }
    if (!runListLoading && runList.length > 0) {
      setInitialised(true)
      setSelectedRun(runList[0].filename)
      loadRun(runList[0].filename)
    }
  }, [initialised, loadRun, runList, runListLoading])

  const handleRunSelect = useCallback(
    async (value: string) => {
      setSelectedRun(value)
      await loadRun(value, { markRefreshing: true })
    },
    [loadRun],
  )

  const handleRefresh = useCallback(async () => {
    const entries = await fetchRunList()
    const target = selectedRun && entries.some((entry) => entry.filename === selectedRun)
      ? selectedRun
      : entries[0]?.filename
    await loadRun(target, { markRefreshing: true })
  }, [fetchRunList, loadRun, selectedRun])

  const handleBenchmarkTrigger = useCallback(async () => {
    setRunTriggerPending(true)
    setRunTriggerMessage(null)
    try {
      const response = await fetch('/api/run-benchmarks', { method: 'POST' })
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`)
      }
      const data = await response.json()
      setRunTriggerMessage(
        Array.isArray(data.instructions)
          ? `Execute manual benchmark: ${data.instructions.join(' && ')}`
          : data.message ?? 'Manual benchmark execution required.'
      )
    } catch (err) {
      setRunTriggerMessage(`Failed to trigger benchmarks: ${err instanceof Error ? err.message : String(err)}`)
    } finally {
      setRunTriggerPending(false)
    }
  }, [])

  const activePanelData = useMemo(() => panels.find((panel) => panel.id === activePanel), [activePanel])
  const physicsSpanCount = useMemo(() => {
    if (summary?.physicsSpanCount !== undefined) {
      return summary.physicsSpanCount
    }
    if (run?.metadata.physicsSpanCount !== undefined) {
      return run.metadata.physicsSpanCount
    }
    if (run) {
      return run.spans.filter((span) => span.physics).length
    }
    return 0
  }, [run, summary])
  const physicsLevel = run?.metadata.physicsLevel ?? null

  const renderPanelContent = () => {
    switch (activePanel) {
      case "dna_panel":
        return <DNAViewer />
      case "span_panel":
        return <SpanTimeline run={run} loading={loading} />
      case "agent_panel":
        return <AgentExplorer run={run} loading={loading} />
      case "discovery_panel":
        return <DiscoveryPanel run={run} summary={summary} loading={loading} error={error} />
      case "render_panel":
        return <UIThemeSystem />
      default:
        return (
          <Card className="min-h-[320px]">
            <CardHeader>
              <CardTitle>{activePanelData?.label ?? "Panel"}</CardTitle>
              <CardDescription>Panel content will be rendered here.</CardDescription>
            </CardHeader>
            <CardContent className="flex h-48 items-center justify-center text-sm text-muted-foreground">
              Select a panel from the navigation to continue.
            </CardContent>
          </Card>
        )
    }
  }

  const headerStatus = () => {
    if (loading) return "Loading…"
    if (refreshing) return "Refreshing…"
    if (run) return `Spans: ${run.metadata.totalSpans}`
    return "Awaiting run"
  }

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border bg-card/50 backdrop-blur-sm">
        <div className="flex h-16 items-center justify-between px-6">
          <div className="flex items-center gap-4">
            <Button variant="ghost" size="sm" onClick={() => setSidebarOpen(!sidebarOpen)} className="lg:hidden">
              {sidebarOpen ? <X className="h-4 w-4" /> : <Menu className="h-4 w-4" />}
            </Button>
            <div className="flex items-center gap-2">
              <div className="h-8 w-8 rounded-lg bg-primary flex items-center justify-center">
                <Dna className="h-5 w-5 text-primary-foreground" />
              </div>
              <div>
                <h1 className="text-xl font-bold">LogLine Folding Dashboard</h1>
                <p className="text-xs text-muted-foreground">Real-time view into the latest folding run</p>
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant="secondary" className="text-xs">
              {headerStatus()}
            </Badge>
            {run && (
              <Badge variant="outline" className="text-xs">
                {run.metadata.environment} · {run.metadata.temperature.toFixed(1)} K
              </Badge>
            )}
            <Select
              value={selectedRun ?? undefined}
              onValueChange={handleRunSelect}
              disabled={runListLoading || runList.length === 0 || loading}
            >
              <SelectTrigger className="w-[220px]">
                <SelectValue placeholder={runListLoading ? "Loading runs…" : "Select run"} />
              </SelectTrigger>
              <SelectContent>
                {runList.map((entry) => (
                  <SelectItem key={entry.filename} value={entry.filename}>
                    {entry.filename}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Button variant="outline" size="sm" onClick={handleRefresh} disabled={refreshing || loading}>
              <RefreshCw className="mr-2 h-4 w-4" /> Refresh
            </Button>
            <Button
              variant="default"
              size="sm"
              onClick={handleBenchmarkTrigger}
              disabled={runTriggerPending}
            >
              Trigger Benchmarks
            </Button>
          </div>
        </div>
      </header>

      {error && !loading && (
        <div className="border-b border-border bg-destructive/10 text-destructive">
          <div className="mx-auto flex max-w-3xl items-center gap-3 px-6 py-3 text-sm">
            <AlertCircle className="h-4 w-4" />
            <span>{error}</span>
          </div>
        </div>
      )}

      <div className="flex">
        <aside className={`${sidebarOpen ? "w-64" : "w-0"} transition-all duration-300 overflow-hidden border-r border-border bg-sidebar`}>
          <div className="p-4">
            <h2 className="text-sm font-semibold text-sidebar-foreground mb-4">Panels</h2>
            <nav className="space-y-2">
              {panels.map((panel) => {
                const Icon = panel.icon
                const isActive = activePanel === panel.id
                return (
                  <Button
                    key={panel.id}
                    variant={isActive ? "default" : "ghost"}
                    className="w-full justify-start gap-3 h-auto p-3"
                    onClick={() => setActivePanel(panel.id)}
                  >
                    <Icon className="h-4 w-4" />
                    <div className="text-left">
                      <div className="font-medium">{panel.label}</div>
                      <div className="text-xs text-muted-foreground">{panel.component}</div>
                    </div>
                  </Button>
                )
              })}
            </nav>
          </div>
        </aside>

        <main className="flex-1 p-6 space-y-6">
          {runTriggerMessage && (
            <Card>
              <CardHeader>
                <CardTitle>Benchmark Trigger</CardTitle>
                <CardDescription>This environment requires manual execution.</CardDescription>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground break-words">{runTriggerMessage}</p>
              </CardContent>
            </Card>
          )}
          {run && (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
              <Card>
                <CardHeader>
                  <CardTitle>Total spans</CardTitle>
                  <CardDescription>Committed + ghost</CardDescription>
                </CardHeader>
                <CardContent className="text-2xl font-bold">
                  {run.metadata.totalSpans}
                  <span className="ml-2 text-sm text-muted-foreground">({run.metadata.ghostSpans} ghost)</span>
                </CardContent>
              </Card>
              <Card>
                <CardHeader>
                  <CardTitle>Acceptance rate</CardTitle>
                  <CardDescription>Metropolis statistics</CardDescription>
                </CardHeader>
                <CardContent className="text-2xl font-bold">
                  {(run.metadata.acceptanceRate * 100).toFixed(1)}%
                </CardContent>
              </Card>
              <Card>
                <CardHeader>
                  <CardTitle>Final Gibbs energy</CardTitle>
                  <CardDescription>kcal/mol</CardDescription>
                </CardHeader>
                <CardContent className="text-2xl font-bold">
                  {run.metadata.finalGibbsEnergy.toFixed(3)}
                </CardContent>
              </Card>
              <Card>
                <CardHeader>
                  <CardTitle>Physics backend</CardTitle>
                  <CardDescription>Kernel + span coverage</CardDescription>
                </CardHeader>
                <CardContent className="text-2xl font-bold uppercase">
                  {physicsLevel ?? 'toy'}
                  <span className="ml-2 text-sm text-muted-foreground normal-case">
                    {physicsSpanCount} spans
                  </span>
                </CardContent>
              </Card>
            </div>
          )}

          {renderPanelContent()}
        </main>
      </div>
    </div>
  )
}
