"use client"

import { useMemo } from "react"
import {
  ResponsiveContainer,
  BarChart,
  Bar,
  CartesianGrid,
  XAxis,
  YAxis,
  Tooltip,
  RadarChart,
  PolarGrid,
  PolarAngleAxis,
  PolarRadiusAxis,
  Radar,
} from "recharts"
import { Bot, Gauge } from "lucide-react"

import type { FoldingRun } from "@/lib/types"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"

interface AgentExplorerProps {
  run: FoldingRun | null
  loading: boolean
}

interface AgentMetric {
  label: string
  applied: number
  ghost: number
  netDeltaE: number
  netDeltaS: number
  lastGibbs: number
}

function buildAgentMetrics(run: FoldingRun | null): AgentMetric[] {
  if (!run) return []
  const map = new Map<string, AgentMetric>()
  for (const span of run.spans) {
    const key = span.spanLabel
    const entry = map.get(key) ?? {
      label: key,
      applied: 0,
      ghost: 0,
      netDeltaE: 0,
      netDeltaS: 0,
      lastGibbs: span.gibbs,
    }
    if (span.ghost) {
      entry.ghost += 1
    } else {
      entry.applied += 1
      entry.netDeltaE += span.deltaE
      entry.netDeltaS += span.deltaS
      entry.lastGibbs = span.gibbs
    }
    map.set(key, entry)
  }
  return Array.from(map.values()).sort((a, b) => b.applied - a.applied)
}

export function AgentExplorer({ run, loading }: AgentExplorerProps) {
  const metrics = useMemo(() => buildAgentMetrics(run), [run])
  const hasData = metrics.length > 0

  if (loading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Span Agents</CardTitle>
          <CardDescription>Evaluating span groups…</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="h-32 animate-pulse rounded-md bg-muted" />
        </CardContent>
      </Card>
    )
  }

  if (!hasData) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Span Agents</CardTitle>
          <CardDescription>No committed spans available for aggregation.</CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            Once the engine emits spans, they are grouped by `span_label` and visualized here.
          </p>
        </CardContent>
      </Card>
    )
  }

  const topAgents = metrics.slice(0, 5)
  const radarData = topAgents.map((agent) => ({
    label: agent.label,
    stability: Math.max(0, -agent.netDeltaE),
    entropy: Math.abs(agent.netDeltaS),
    ghostLoad: agent.ghost,
  }))

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Bot className="h-5 w-5" />
            Span cohorts
          </CardTitle>
          <CardDescription>Groups of spans by label with aggregate energy deltas.</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 lg:grid-cols-2">
          <div className="h-72 rounded-xl bg-card p-4">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={metrics}>
                <CartesianGrid strokeDasharray="3 3" strokeOpacity={0.2} />
                <XAxis dataKey="label" hide />
                <YAxis />
                <Tooltip />
                <Bar dataKey="netDeltaE" name="ΣΔE" fill="rgb(var(--chart-1))" />
                <Bar dataKey="applied" name="Applied spans" fill="rgb(var(--chart-4))" />
              </BarChart>
            </ResponsiveContainer>
          </div>
          <div className="h-72 rounded-xl bg-card p-4">
            <ResponsiveContainer width="100%" height="100%">
              <RadarChart data={radarData} outerRadius="80%">
                <PolarGrid strokeOpacity={0.3} />
                <PolarAngleAxis dataKey="label" tick={{ fill: "rgb(var(--muted-foreground))", fontSize: 12 }} />
                <PolarRadiusAxis angle={30} domain={[0, "auto"]} />
                <Radar name="Stability" dataKey="stability" stroke="rgb(var(--chart-2))" fill="rgb(var(--chart-2))" fillOpacity={0.4} />
                <Radar name="Entropy" dataKey="entropy" stroke="rgb(var(--chart-5))" fill="rgb(var(--chart-5))" fillOpacity={0.3} />
              </RadarChart>
            </ResponsiveContainer>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Gauge className="h-5 w-5" />
            Focus agents
          </CardTitle>
          <CardDescription>Highest activity span labels with stability indicators.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {topAgents.map((agent) => (
            <div
              key={agent.label}
              className="flex flex-col gap-2 rounded-lg border border-border/70 bg-card/40 p-4 md:flex-row md:items-center md:justify-between"
            >
              <div>
                <div className="flex items-center gap-3">
                  <span className="font-semibold">{agent.label}</span>
                  <Badge variant="outline">{agent.applied} spans</Badge>
                  {agent.ghost > 0 && <Badge variant="destructive">{agent.ghost} ghost</Badge>}
                </div>
                <p className="text-xs text-muted-foreground mt-1">Last Gibbs: {agent.lastGibbs.toFixed(4)}</p>
              </div>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <p className="text-muted-foreground">ΣΔE</p>
                  <p className="font-semibold">{agent.netDeltaE.toFixed(4)}</p>
                </div>
                <div>
                  <p className="text-muted-foreground">ΣΔS</p>
                  <p className="font-semibold">{agent.netDeltaS.toFixed(4)}</p>
                </div>
              </div>
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  )
}
