import type {
  ImpactResponse,
  PredictionsResponse,
  RepositoryAnalysis,
  ReworkEvaluationReport,
  RippleResponse,
} from './types'
import {
  MAX_RETRIES,
  MAX_RETRY_DELAY_MS,
  exponentialBackoffDelayMs,
  queryClient,
  repositoryAnalysisQueryKey,
} from './lib/queryClient'

export class ApiError extends Error {
  status: number

  constructor(message: string, status: number) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

const VITE_API_BASE = import.meta.env.VITE_API_BASE ?? ''


function retryDelayMs(attempt: number, retryAfterSeconds: number | null): number {
  const base =
    retryAfterSeconds != null && retryAfterSeconds > 0
      ? retryAfterSeconds * 1000
      : exponentialBackoffDelayMs(attempt)
  return Math.min(base, MAX_RETRY_DELAY_MS)
}

function isRetryableError(error: unknown): boolean {
  return (
    error instanceof ApiError &&
    (error.status === 429 ||
      (error.status >= 500 && error.status < 600))
  )
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
  let lastRetryAfterSeconds: number | null = null

  async function queryFn(): Promise<RepositoryAnalysis> {
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
    lastRetryAfterSeconds = retryAfterSeconds
    throw new ApiError(
      message
        ? message
        : `Analysis request failed: HTTP ${response.status}`,
      response.status,
    )
  }

  return queryClient.fetchQuery({
    queryKey: repositoryAnalysisQueryKey(owner, repo),
    queryFn,
    staleTime: 0,
    gcTime: 0,
    retry: (failureCount, error) =>
      failureCount < MAX_RETRIES && isRetryableError(error),
    retryDelay: (failureCount) =>
      retryDelayMs(failureCount, lastRetryAfterSeconds),
  })
}

export async function fetchLocalAnalysis(repoPath: string): Promise<RepositoryAnalysis> {
  const url = `${VITE_API_BASE}/api/local/analysis`

  let response: Response
  try {
    response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: repoPath }),
    })
  } catch (err) {
    if (err instanceof Error) throw err
    throw new Error('Network error while contacting the analysis service', { cause: err })
  }

  if (response.ok) {
    return (await response.json()) as RepositoryAnalysis
  }

  const { message } = await readErrorPayload(response)
  throw new ApiError(
    message ? message : `Local analysis request failed: HTTP ${response.status}`,
    response.status,
  )
}

export function describeLocalPathError(raw: string): string | null {
  if (!raw.trim()) return 'Please enter a local repository path.'
  return null
}

export async function fetchPredictions(
  owner: string,
  repo: string,
): Promise<PredictionsResponse> {
  const url = `${VITE_API_BASE}/api/repositories/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}/predictions`
  return fetchJson<PredictionsResponse>(url, {}, 'Prediction request failed')
}

export async function fetchLocalPredictions(
  repoPath: string,
): Promise<PredictionsResponse> {
  const url = `${VITE_API_BASE}/api/local/predictions`
  return fetchJson<PredictionsResponse>(
    url,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: repoPath }),
    },
    'Local prediction request failed',
  )
}

async function fetchJson<T>(
  url: string,
  init: RequestInit,
  fallback: string,
): Promise<T> {
  let response: Response
  try {
    response = await fetch(url, init)
  } catch (err) {
    if (err instanceof Error) throw err
    throw new Error('Network error while contacting the analysis service', { cause: err })
  }

  if (response.ok) {
    return (await response.json()) as T
  }

  const { message } = await readErrorPayload(response)
  throw new ApiError(
    message ? message : `${fallback}: HTTP ${response.status}`,
    response.status,
  )
}

export async function fetchReworkEvaluation(): Promise<ReworkEvaluationReport> {
  const url = `${VITE_API_BASE}/api/evaluation/rework`

  let response: Response
  try {
    response = await fetch(url)
  } catch (err) {
    if (err instanceof Error) throw err
    throw new Error('Network error while contacting the analysis service', { cause: err })
  }

  if (!response.ok) {
    const { message } = await readErrorPayload(response)
    throw new ApiError(
      message ? message : `Evaluation request failed: HTTP ${response.status}`,
      response.status,
    )
  }

  return (await response.json()) as ReworkEvaluationReport
}

export interface RippleQuery {
  source: string
  changed?: string[]
  maxDepth?: number
  minConfidence?: number
}

export async function fetchRipple(
  owner: string,
  repo: string,
  query: RippleQuery,
): Promise<RippleResponse> {
  const params = new URLSearchParams({ source: query.source })
  if (query.changed && query.changed.length > 0) {
    params.set('changed', query.changed.join(','))
  }
  if (query.maxDepth != null) params.set('max_depth', String(query.maxDepth))
  if (query.minConfidence != null) {
    params.set('min_confidence', String(query.minConfidence))
  }
  const url = `${VITE_API_BASE}/api/repositories/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}/ripple?${params.toString()}`
  return fetchJson<RippleResponse>(url, {}, 'Ripple request failed')
}

export async function fetchLocalRipple(
  repoPath: string,
  query: RippleQuery,
): Promise<RippleResponse> {
  const url = `${VITE_API_BASE}/api/local/ripple`
  return fetchJson<RippleResponse>(
    url,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        path: repoPath,
        source: query.source,
        changed: query.changed ?? [],
        max_depth: query.maxDepth,
        min_confidence: query.minConfidence,
      }),
    },
    'Local ripple request failed',
  )
}

export interface ImpactQuery {
  changeDescription: string
  maxResults?: number
  includeLowConfidence?: boolean
}

export async function fetchImpact(
  owner: string,
  repo: string,
  query: ImpactQuery,
): Promise<ImpactResponse> {
  const params = new URLSearchParams({
    change_description: query.changeDescription,
  })
  if (query.maxResults != null) params.set('max_results', String(query.maxResults))
  if (query.includeLowConfidence != null) {
    params.set('include_low_confidence', String(query.includeLowConfidence))
  }
  const url = `${VITE_API_BASE}/api/repositories/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}/impact-simulation?${params.toString()}`
  return fetchJson<ImpactResponse>(url, {}, 'Impact simulation request failed')
}

export async function fetchLocalImpact(
  repoPath: string,
  query: ImpactQuery,
): Promise<ImpactResponse> {
  const url = `${VITE_API_BASE}/api/local/impact-simulation`
  return fetchJson<ImpactResponse>(
    url,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        path: repoPath,
        change_description: query.changeDescription,
        max_results: query.maxResults,
        include_low_confidence: query.includeLowConfidence,
      }),
    },
    'Local impact simulation request failed',
  )
}