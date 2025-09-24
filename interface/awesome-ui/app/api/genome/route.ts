import { NextResponse } from 'next/server'
import { loadGenome } from '@/lib/genome'

export async function GET() {
  const genome = await loadGenome()
  if (!genome) {
    return NextResponse.json({ message: 'Genome source not available.' }, { status: 404 })
  }

  return NextResponse.json(genome)
}
