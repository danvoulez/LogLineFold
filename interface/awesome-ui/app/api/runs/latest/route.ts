import { NextResponse } from 'next/server'
import { getLatestRun } from '@/lib/logs'
import type { RunResponse } from '@/lib/types'

export async function GET() {
  const run = await getLatestRun()
  if (!run) {
    return NextResponse.json(
      { message: 'No folding runs found. Generate a log with `cargo run -- ... --log logs/output.jsonl`.' },
      { status: 404 },
    )
  }

  const applied = run.spans.filter((span) => !span.ghost)
  const physicsSpanCount = run.metadata.physicsSpanCount ?? run.spans.filter((span) => span.physics).length
  const physicsMetrics = (run.metadata.physicsMetrics && run.metadata.physicsMetrics.length > 0)
    ? run.metadata.physicsMetrics
    : run.spans
        .map((span) => span.physicsMetrics)
        .filter((metric): metric is NonNullable<typeof metric> => Boolean(metric))
  const metricsCount = physicsMetrics.length
  const averageRmsd = metricsCount
    ? physicsMetrics.reduce((acc, metric) => acc + metric.rmsd, 0) / metricsCount
    : undefined
  const maxRmsd = metricsCount ? Math.max(...physicsMetrics.map((metric) => metric.rmsd)) : undefined
  const averageRadiusOfGyration = metricsCount
    ? physicsMetrics.reduce((acc, metric) => acc + metric.radiusOfGyration, 0) / metricsCount
    : undefined
  const totalSimulationTimePs = metricsCount
    ? physicsMetrics.reduce((acc, metric) => acc + metric.simulationTimePs, 0)
    : undefined
  const response: RunResponse = {
    run,
    summary: {
      appliedCount: applied.length,
      ghostCount: run.spans.length - applied.length,
      netDeltaEnergy: applied.reduce((acc, span) => acc + span.deltaE, 0),
      netEntropy: applied.reduce((acc, span) => acc + span.deltaS, 0),
      physicsSpanCount,
      averageRmsd,
      maxRmsd,
      averageRadiusOfGyration,
      totalSimulationTimePs,
    },
  }

  return NextResponse.json(response)
}
