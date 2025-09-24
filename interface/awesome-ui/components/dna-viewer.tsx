"use client"

import { useCallback, useEffect, useRef, useState } from "react"
import { Upload, Download, RotateCcw, Play, Pause, Zap, Eye, Shuffle, AlertCircle } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Slider } from "@/components/ui/slider"
import { Badge } from "@/components/ui/badge"

interface GenomePayload {
  name: string
  sequence: string
  length: number
}

export function DNAViewer() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [sequence, setSequence] = useState("ATCGATCGATCGATCG")
  const [genomeMeta, setGenomeMeta] = useState<{ name: string; length: number } | null>(null)
  const [status, setStatus] = useState<"idle" | "loading" | "error" | "ready">("idle")

  const [rotation, setRotation] = useState([0])
  const [zoom, setZoom] = useState([1])
  const [isAnimating, setIsAnimating] = useState(false)
  const [mutationRate, setMutationRate] = useState([0.1])
  const [fusionMode, setFusionMode] = useState(false)
  const [selectedBase, setSelectedBase] = useState<number | null>(null)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)

  const fetchGenome = useCallback(async () => {
    setStatus("loading")
    setErrorMessage(null)
    try {
      const response = await fetch("/api/genome")
      if (!response.ok) {
        const payload = await response.json().catch(() => ({}))
        throw new Error(payload.message ?? "Unable to fetch genome sequence")
      }
      const payload = (await response.json()) as GenomePayload
      setSequence(payload.sequence || "ATCG")
      setGenomeMeta({ name: payload.name, length: payload.length })
      setStatus("ready")
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "Unknown failure")
      setStatus("error")
    }
  }, [])

  useEffect(() => {
    fetchGenome()
  }, [fetchGenome])

  const getComplement = (base: string): string => {
    const complements: Record<string, string> = { A: "T", T: "A", G: "C", C: "G" }
    return complements[base] || "A"
  }

  const generateRandomDNA = (length = 32) => {
    const bases = ["A", "T", "G", "C"]
    return Array.from({ length }, () => bases[Math.floor(Math.random() * bases.length)]).join("")
  }

  const simulateMutation = () => {
    const bases = ["A", "T", "G", "C"]
    const letters = sequence.split("")
    const numMutations = Math.max(1, Math.floor(letters.length * mutationRate[0]))
    for (let i = 0; i < numMutations; i++) {
      const index = Math.floor(Math.random() * letters.length)
      const currentBase = letters[index]
      const candidates = bases.filter((base) => base !== currentBase)
      letters[index] = candidates[Math.floor(Math.random() * candidates.length)]
    }
    setSequence(letters.join(""))
  }

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext("2d")
    if (!ctx) return

    const width = canvas.width
    const height = canvas.height
    const centerX = width / 2
    const centerY = height / 2

    ctx.fillStyle = "rgb(var(--background))"
    ctx.fillRect(0, 0, width, height)

    const helixRadius = 80 * zoom[0]
    const helixHeight = height * 0.8
    const turns = 3
    const baseSpacing = helixHeight / sequence.length

    ctx.strokeStyle = "rgb(var(--primary))"
    ctx.lineWidth = 3

    ctx.beginPath()
    for (let i = 0; i <= 100; i++) {
      const t = i / 100
      const y = centerY - helixHeight / 2 + t * helixHeight
      const angle = t * turns * 2 * Math.PI + rotation[0] * 0.1
      const x1 = centerX + Math.cos(angle) * helixRadius
      if (i === 0) ctx.moveTo(x1, y)
      else ctx.lineTo(x1, y)
    }
    ctx.stroke()

    ctx.beginPath()
    for (let i = 0; i <= 100; i++) {
      const t = i / 100
      const y = centerY - helixHeight / 2 + t * helixHeight
      const angle = t * turns * 2 * Math.PI + rotation[0] * 0.1 + Math.PI
      const x2 = centerX + Math.cos(angle) * helixRadius
      if (i === 0) ctx.moveTo(x2, y)
      else ctx.lineTo(x2, y)
    }
    ctx.stroke()

    sequence.split("").forEach((base, index) => {
      const t = index / Math.max(sequence.length - 1, 1)
      const y = centerY - helixHeight / 2 + t * helixHeight
      const angle = t * turns * 2 * Math.PI + rotation[0] * 0.1
      const x1 = centerX + Math.cos(angle) * helixRadius
      const x2 = centerX + Math.cos(angle + Math.PI) * helixRadius
      const complement = getComplement(base)

      ctx.strokeStyle = fusionMode ? "rgb(var(--accent))" : "rgb(var(--muted-foreground))"
      ctx.lineWidth = 2
      ctx.beginPath()
      ctx.moveTo(x1, y)
      ctx.lineTo(x2, y)
      ctx.stroke()

      const baseColors: Record<string, string> = {
        A: "rgb(var(--chart-1))",
        T: "rgb(var(--chart-2))",
        G: "rgb(var(--chart-3))",
        C: "rgb(var(--chart-4))",
      }

      ctx.fillStyle = baseColors[base] || "rgb(var(--muted))"
      ctx.beginPath()
      ctx.arc(x1, y, selectedBase === index ? 8 : 6, 0, 2 * Math.PI)
      ctx.fill()

      ctx.fillStyle = baseColors[complement] || "rgb(var(--muted))"
      ctx.beginPath()
      ctx.arc(x2, y, 6, 0, 2 * Math.PI)
      ctx.fill()

      ctx.fillStyle = "rgb(var(--foreground))"
      ctx.font = "12px monospace"
      ctx.textAlign = "center"
      ctx.fillText(base, x1, y + 4)
      ctx.fillText(complement, x2, y + 4)
    })
  }, [fusionMode, rotation, selectedBase, sequence, zoom])

  useEffect(() => {
    if (!isAnimating) return
    const interval = setInterval(() => setRotation((prev) => [(prev[0] + 1) % 360]), 60)
    return () => clearInterval(interval)
  }, [isAnimating])

  return (
    <div className="space-y-6">
      <Tabs defaultValue="visualizer" className="w-full">
        <TabsList className="grid w-full grid-cols-4">
          <TabsTrigger value="visualizer">Visualizer</TabsTrigger>
          <TabsTrigger value="import">Import DNA</TabsTrigger>
          <TabsTrigger value="generate">Generate</TabsTrigger>
          <TabsTrigger value="mutations">Mutations</TabsTrigger>
        </TabsList>

        <TabsContent value="visualizer" className="space-y-4">
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    <Eye className="h-5 w-5" />
                    DNA Helix Visualization
                  </CardTitle>
                  <CardDescription>
                    {status === "loading" && "Loading genome sequence…"}
                    {status === "ready" && genomeMeta && `${genomeMeta.name} · ${genomeMeta.length} bp`}
                    {status === "error" && errorMessage}
                  </CardDescription>
                </div>
                <div className="flex items-center gap-2">
                  <Button variant="outline" size="sm" onClick={fetchGenome} disabled={status === "loading"}>
                    <Download className="mr-2 h-4 w-4" />Reload FASTA
                  </Button>
                  <Button variant={isAnimating ? "default" : "outline"} size="icon" onClick={() => setIsAnimating(!isAnimating)}>
                    {isAnimating ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
                  </Button>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              {status === "error" && errorMessage && (
                <div className="flex items-center gap-2 rounded-md border border-destructive/60 bg-destructive/10 p-3 text-sm text-destructive">
                  <AlertCircle className="h-4 w-4" />
                  <span>{errorMessage}</span>
                </div>
              )}
              <div className="relative">
                <canvas
                  ref={canvasRef}
                  width={640}
                  height={360}
                  className="w-full rounded-xl border border-border bg-background/80"
                  onMouseMove={(event) => {
                    const rect = event.currentTarget.getBoundingClientRect()
                    const y = event.clientY - rect.top
                    const index = Math.floor((y / rect.height) * sequence.length)
                    setSelectedBase(isNaN(index) ? null : index)
                  }}
                  onMouseLeave={() => setSelectedBase(null)}
                />
              </div>
              <div className="grid gap-4 md:grid-cols-3">
                <div className="space-y-2">
                  <Label>Rotation</Label>
                  <Slider value={rotation} onValueChange={setRotation} max={360} />
                </div>
                <div className="space-y-2">
                  <Label>Zoom</Label>
                  <Slider value={zoom} onValueChange={setZoom} min={0.5} max={2.5} step={0.1} />
                </div>
                <div className="space-y-2">
                  <Label>Fusion mode</Label>
                  <Button variant={fusionMode ? "default" : "outline"} className="w-full" onClick={() => setFusionMode(!fusionMode)}>
                    <Zap className="mr-2 h-4 w-4" /> {fusionMode ? "Enabled" : "Disabled"}
                  </Button>
                </div>
              </div>
              <div className="rounded-md border border-border/60 bg-card/40 p-4 text-sm">
                <p className="text-muted-foreground">Sequence preview</p>
                <p className="font-mono text-xs md:text-sm break-all">{sequence}</p>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="import">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Upload className="h-5 w-5" />
                Paste FASTA sequence
              </CardTitle>
              <CardDescription>Only canonical bases (A, T, G, C) are rendered in the helix.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <Input
                placeholder="ATCG…"
                value={sequence}
                onChange={(event) => setSequence(event.target.value.replace(/[^ATGC]/gi, "").toUpperCase())}
              />
              <p className="text-xs text-muted-foreground">Length: {sequence.length} bases</p>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="generate">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Shuffle className="h-5 w-5" />
                Synthetic sequence
              </CardTitle>
              <CardDescription>Create a random sequence for quick visualization.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <Button
                variant="outline"
                onClick={() => setSequence(generateRandomDNA(64))}
              >
                Generate 64 bp
              </Button>
              <Button
                variant="outline"
                onClick={() => setSequence(generateRandomDNA(128))}
              >
                Generate 128 bp
              </Button>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="mutations">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <RotateCcw className="h-5 w-5" />
                Mutation simulator
              </CardTitle>
              <CardDescription>Apply random mutations to the active sequence.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label>Mutation rate {Math.round(mutationRate[0] * 100)}%</Label>
                <Slider value={mutationRate} onValueChange={setMutationRate} min={0.01} max={0.5} step={0.01} />
              </div>
              <Button onClick={simulateMutation}>
                Apply mutations
              </Button>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
        <Badge variant="outline">Bases {sequence.length}</Badge>
        {genomeMeta && <Badge variant="outline">Source · {genomeMeta.name}</Badge>}
        <Badge variant="outline">Status · {status}</Badge>
      </div>
    </div>
  )
}
