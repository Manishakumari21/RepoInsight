import type { RepositoryAnalysis } from './types'

export class ApiError extends Error {
  status: number

  constructor(message: string, status: number) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

const VITE_API_BASE = import.meta.env.VITE_API_BASE ?? ''
const MAX_RETRIES = 2
const MAX_RETRY_DELAY_MS = 10_000

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function retryDelayMs(attempt: number, retryAfterSeconds: number | null): number {
  const base = retryAfterSeconds != null && retryAfterSeconds > 0
    ? retryAfterSeconds * 1000
    : 1000 * 2 ** attempt
  return Math.min(base, MAX_RETRY_DELAY_MS)
}

async function readErrorPayload(response: Response): Promise<{
  message: string
  retryAfterSeconds: number | null
}> {
  let message = ''
  try {
    const body = (await response.json()) as { error?: unknown }
    if (typeof body.error === 'string' && body.error.trim()) {
      message = body.error.trim()
    }
  } catch {
    // response body is not JSON; fall back to a generic message
  }

  const retryAfterHeader = response.headers.get('retry-after')
  let retryAfterSeconds: number | null = null
  if (retryAfterHeader) {
    const parsed = Number(retryAfterHeader)
    if (Number.isFinite(parsed)) retryAfterSeconds = parsed
  }

  return { message, retryAfterSeconds }
}

export async function fetchRepositoryAnalysis(
  owner: string,
  repo: string,
): Promise<RepositoryAnalysis> {
  const url = `${VITE_API_BASE}/api/repositories/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}/analysis`

  for (let attempt = 0; attempt <= MAX_RETRIES; attempt += 1) {
    let response: Response
    try {
      response = await fetch(url)
    } catch (err) {
      if (err instanceof Error) throw err
      throw new Error('Network error while contacting the analysis service', { cause: err })
    }

    if (response.ok) {
      return (await response.json()) as RepositoryAnalysis
    }

    const { message, retryAfterSeconds } = await readErrorPayload(response)

    const retryable =
      response.status === 429 || (response.status >= 500 && response.status < 600)

    if (retryable && attempt < MAX_RETRIES) {
      await wait(retryDelayMs(attempt, retryAfterSeconds))
      continue
    }

    throw new ApiError(
      message
        ? message
        : `Analysis request failed: HTTP ${response.status}`,
      response.status,
    )
  }

  throw new Error('Analysis request failed')
}