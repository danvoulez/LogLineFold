"use client"

import type { FoldSpan, FoldingRun, FoldingSummary } from "@/lib/types"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Badge } from "@/components/ui/badge"
import { AlertCircle, Beaker, Thermometer, Activity, FileText, AlertTriangle, Flame, Rocket, Wind } from "lucide-react"
import { Button } from "@/components/ui/button"

interface DiscoveryPanelProps {
  run: FoldingRun | null
  summary: FoldingSummary | null
  loading: boolean
  error: string | null
}

interface HighlightEntry {
  span: FoldSpan
  metricLabel: string
  metricValue: string
  context: string
}

function selectHighlights(spans: FoldSpan[]) {
  const applied = spans.filter((span) => !span.ghost)
  const ghost = spans.filter((span) => span.ghost)

  const stabilisers = [...applied]
    .filter((span) => span.deltaE < 0)
    .sort((a, b) => a.deltaE - b.deltaE)
    .slice(0, 3)

  const entropyPeaks = [...applied]
    .filter((span) => span.deltaS > 0)
    .sort((a, b) => b.deltaS - a.deltaS)
    .slice(0, 3)

  const ghostHotspots = [...ghost]
    .sort((a, b) => b.deltaE - a.deltaE)
    .slice(0, 3)

  return {
    stabilisers,
    entropyPeaks,
    ghostHotspots,
  }
}

export function DiscoveryPanel({ run, summary, loading, error }: DiscoveryPanelProps) {
  if (loading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Run insights</CardTitle>
          <CardDescription>Parsing span log…</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="h-32 animate-pulse rounded-md bg-muted" />
        </CardContent>
      </Card>
    )
  }

  if (error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Run insights</CardTitle>
          <CardDescription>Unable to load folding metadata.</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-3 rounded-md border border-destructive/40 bg-destructive/10 p-4 text-destructive">
            <AlertCircle className="h-5 w-5" />
            <p className="text-sm">{error}</p>
          </div>
        </CardContent>
      </Card>
    )
  }

  if (!run) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Run insights</CardTitle>
          <CardDescription>No folding session detected.</CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            Launch a run with `cargo run -- --preset demo --log logs/output.jsonl` and refresh the dashboard.
          </p>
        </CardContent>
      </Card>
    )
  }

  const metadata = run.metadata
  const violationMessages = new Set<string>([...metadata.violations, ...run.violations.map((v) => v.detail)])
  const highlightSets = selectHighlights(run.spans)

  const highlightBlocks: { title: string; icon: React.ReactNode; entries: FoldSpan[]; formatter: (span: FoldSpan) => HighlightEntry }[] = [
    {
      title: "Stabilising spans",
      icon: <Flame className="h-4 w-4 text-emerald-500" />,
      entries: highlightSets.stabilisers,
      formatter: (span) => ({
        span,
        metricLabel: "ΔE",
        metricValue: span.deltaE.toFixed(4),
        context: `Entropy +${span.deltaS.toFixed(4)} | Δθ ${span.deltaTheta.toFixed(2)}°`,
      }),
    },
    {
      title: "Entropy breakthroughs",
      icon: <Wind className="h-4 w-4 text-sky-500" />,
      entries: highlightSets.entropyPeaks,
      formatter: (span) => ({
        span,
        metricLabel: "ΔS",
        metricValue: span.deltaS.toFixed(4),
        context: `ΔE ${span.deltaE.toFixed(4)} | Gibbs ${span.gibbs.toFixed(4)}`,
      }),
    },
    {
      title: "Ghost hotspots",
      icon: <Rocket className="h-4 w-4 text-orange-500" />,
      entries: highlightSets.ghostHotspots,
      formatter: (span) => ({
        span,
        metricLabel: "ΔE",
        metricValue: span.deltaE.toFixed(4),
        context: `Entropy +${span.deltaS.toFixed(4)} | Δθ ${span.deltaTheta.toFixed(2)}°`,
      }),
    },
  ]

  return (
    <Tabs defaultValue="overview" className="w-full">
      <TabsList className="grid w-full grid-cols-3">
        <TabsTrigger value="overview">Overview</TabsTrigger>
        <TabsTrigger value="highlights">Discoveries</TabsTrigger>
        <TabsTrigger value="log">Log details</TabsTrigger>
      </TabsList>

      <TabsContent value="overview">
        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Thermometer className="h-5 w-5" />
                Environment snapshot
              </CardTitle>
              <CardDescription>Runtime configuration captured at the end of the run.</CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
              <div className="rounded-lg border border-border/70 bg-card/50 p-4">
                <p className="text-sm text-muted-foreground">Environment</p>
                <p className="text-lg font-semibold capitalize">{metadata.environment}</p>
                <Badge variant="outline" className="mt-2">
                  Contract {metadata.contractName ?? "unknown"}
                </Badge>
              </div>
              <div className="rounded-lg border border-border/70 bg-card/50 p-4">
                <p className="text-sm text-muted-foreground">Temperature</p>
                <p className="text-lg font-semibold">{metadata.temperature.toFixed(2)} K</p>
                <p className="text-xs text-muted-foreground">Δt = {metadata.timeStepMs} ms</p>
              </div>
              <div className="rounded-lg border border-border/70 bg-card/50 p-4">
                <p className="text-sm text-muted-foreground">Acceptance</p>
                <p className="text-lg font-semibold">{(metadata.acceptanceRate * 100).toFixed(1)}%</p>
                <p className="text-xs text-muted-foreground">
                  {metadata.acceptedSpans} accepted · {metadata.rejectedSpans} rejected
                </p>
              </div>
              <div className="rounded-lg border border-border/70 bg-card/50 p-4">
                <p className="text-sm text-muted-foreground">Span totals</p>
                <p className="text-lg font-semibold">{metadata.totalSpans}</p>
                <p className="text-xs text-muted-foreground">{metadata.ghostSpans} ghost spans</p>
              </div>
              <div className="rounded-lg border border-border/70 bg-card/50 p-4">
                <p className="text-sm text-muted-foreground">Physics backend</p>
                <p className="text-lg font-semibold capitalize">
                  {(metadata.physicsLevel ?? 'toy').toString()}
                </p>
                <p className="text-xs text-muted-foreground">
                  Physics spans: {metadata.physicsSpanCount ?? run.spans.filter((span) => span.physics).length}
                </p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Activity className="h-5 w-5" />
                Energy metrics
              </CardTitle>
              <CardDescription>End-of-run energy snapshot (kcal/mol).</CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4 md:grid-cols-3">
              <div className="rounded-lg border border-border/70 bg-background/40 p-4">
                <p className="text-sm text-muted-foreground">Final Gibbs (G)</p>
                <p className="text-xl font-semibold">{metadata.finalGibbsEnergy.toFixed(4)}</p>
              </div>
              <div className="rounded-lg border border-border/70 bg-background/40 p-4">
                <p className="text-sm text-muted-foreground">Potential energy</p>
                <p className="text-xl font-semibold">{metadata.finalPotentialEnergy.toFixed(4)}</p>
              </div>
              <div className="rounded-lg border border-border/70 bg-background/40 p-4">
                <p className="text-sm text-muted-foreground">Kinetic energy</p>
                <p className="text-xl font-semibold">{metadata.finalKineticEnergy.toFixed(4)}</p>
              </div>
            </CardContent>
          </Card>

          {summary && (
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Beaker className="h-5 w-5" />
                  Applied span aggregate
                </CardTitle>
                <CardDescription>Cumulative ΔE and ΔS from committed spans only.</CardDescription>
              </CardHeader>
              <CardContent className="grid gap-4 md:grid-cols-2">
                <div className="rounded-lg border border-border/70 bg-card/50 p-4">
                  <p className="text-sm text-muted-foreground">Net ΔE</p>
                  <p className="text-lg font-semibold">{summary.netDeltaEnergy.toFixed(4)}</p>
                </div>
                <div className="rounded-lg border border-border/70 bg-card/50 p-4">
                  <p className="text-sm text-muted-foreground">Net ΔS</p>
                  <p className="text-lg font-semibold">{summary.netEntropy.toFixed(4)}</p>
                </div>
              </CardContent>
            </Card>
          )}
        </div>
      </TabsContent>

      <TabsContent value="highlights">
        <div className="space-y-4">
          {highlightBlocks.map(({ title, icon, entries, formatter }) => (
            <Card key={title}>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  {icon}
                  {title}
                </CardTitle>
                <CardDescription>
                  {entries.length === 0 ? "No spans recorded for this category." : "Top events ranked by scientific impact."}
                </CardDescription>
              </CardHeader>
              {entries.length > 0 && (
                <CardContent className="space-y-3">
                  {entries.map((span) => {
                    const highlight = formatter(span)
                    return (
                      <div
                        key={span.spanUuid}
                        className="flex flex-col justify-between gap-2 rounded-lg border border-border/70 bg-card/40 p-4 md:flex-row md:items-center"
                      >
                        <div>
                          <div className="flex items-center gap-2">
                            <span className="font-semibold">{span.spanLabel}</span>
                            <Badge variant="outline">{new Date(span.timestamp).toLocaleString()}</Badge>
                          </div>
                          <p className="text-xs text-muted-foreground mt-1">{highlight.context}</p>
                        </div>
                        <div className="text-sm font-semibold">{highlight.metricLabel}: {highlight.metricValue}</div>
                      </div>
                    )
                  })}
                </CardContent>
              )}
            </Card>
          ))}

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <AlertTriangle className="h-5 w-5" />
                Enforcement log
              </CardTitle>
              <CardDescription>Violations captured by the ruleset and replay pass.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {violationMessages.size === 0 && (
                <p className="text-sm text-muted-foreground">No violations recorded.</p>
              )}
              {[...violationMessages].map((message) => (
                <div
                  key={message}
                  className="flex items-start gap-3 rounded-md border border-border/70 bg-destructive/10 p-3 text-sm"
                >
                  <AlertCircle className="mt-1 h-4 w-4 text-destructive" />
                  <div className="flex-1">
                    <p className="font-medium text-foreground">{message}</p>
                  </div>
                </div>
              ))}
            </CardContent>
          </Card>
        </div>
      </TabsContent>

      <TabsContent value="log">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <FileText className="h-5 w-5" />
              Log file
            </CardTitle>
            <CardDescription>Physical location and metadata for reproducibility.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="rounded-lg border border-border/70 bg-card/50 p-4 text-sm">
              <p className="text-muted-foreground">Path</p>
              <p className="font-mono text-xs md:text-sm">{run.path}</p>
            </div>
            <div className="rounded-lg border border-border/70 bg-card/50 p-4 text-sm">
              <p className="text-muted-foreground">Last updated</p>
              <p>{new Date(run.updatedAt).toLocaleString()}</p>
            </div>
            <div className="rounded-lg border border-border/70 bg-card/50 p-4 text-sm">
              <p className="text-muted-foreground">Version</p>
              <p>{metadata.version}</p>
            </div>
            <Button variant="outline" asChild>
              <a href={`/api/runs/latest`} rel="noreferrer">
                Download latest JSON payload
              </a>
            </Button>
          </CardContent>
        </Card>
      </TabsContent>
    </Tabs>
  )
}
