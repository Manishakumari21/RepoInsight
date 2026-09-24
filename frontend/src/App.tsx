import { useCallback, useEffect, useRef, useState } from 'react'
import {
  ApiError,
  describeLocalPathError,
  fetchLocalAnalysis,
  fetchLocalPredictions,
  fetchPredictions,
  fetchRepositoryAnalysis,
} from './api'
import {
  describeGitHubUrlError,
  parseGitHubUrl,
} from './lib/githubUrl'
import type { PredictionsResponse, RepositoryAnalysis } from './types'
import type { ActiveSource } from './lib/repoSource'
import { Landing } from './components/Landing'
import { Dashboard } from './components/Dashboard'
import { DashboardSkeleton } from './components/primitives'
import { ErrorBanner } from './components/Status'
import './App.css'

function useRevealOnScroll(enabled: boolean, rescan: unknown) {
  useEffect(() => {
    if (!enabled) return
    const elements = Array.from(
      document.querySelectorAll('[data-reveal]:not(.in)'),
    )
    if (elements.length === 0) return
    if (typeof IntersectionObserver === 'undefined') {
      for (const element of elements) element.classList.add('in')
      return
    }
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            entry.target.classList.add('in')
            observer.unobserve(entry.target)
          }
        }
      },
      { rootMargin: '0px 0px -8% 0px', threshold: 0.05 },
    )
    for (const element of elements) observer.observe(element)
    return () => observer.disconnect()
  }, [enabled, rescan])
}

export function analysisErrorHint(status: number): string {
  switch (status) {
    case 400:
    case 422:
      return 'The repository input was rejected by the analysis service.'
    case 403:
      return 'Access was denied. The repository may be private or GitHub is throttling this request.'
    case 404:
      return 'Repository not found. Check the URL or local path and try again.'
    case 429:
      return 'GitHub API rate limit reached. Wait a moment and try again.'
    case 500:
      return 'The analysis service hit an internal error. Please try again.'
    case 502:
    case 503:
    case 504:
      return 'The analysis service is unreachable (HTTP 502). Start the backend on port 3000 with `cargo run` in backend/ and try again.'
    default:
      return `The analysis request failed with HTTP ${status}.`
  }
}

function describeAnalysisError(err: unknown): string {
  if (err instanceof ApiError) {
    const hint = analysisErrorHint(err.status)
    const detail =
      err.message && !/HTTP \d/.test(err.message) ? err.message.trim() : ''
    return detail ? `${hint} ${detail}` : hint
  }
  if (err instanceof Error && err.message) return err.message
  return 'Failed to analyze repository. Please try again.'
}

function App() {
  const [source, setSource] = useState<ActiveSource | null>(null)
  const [analysis, setAnalysis] = useState<RepositoryAnalysis | null>(null)
  const [predictions, setPredictions] = useState<PredictionsResponse | null>(null)
  const [predictionsLoading, setPredictionsLoading] = useState(false)
  const [predictionsError, setPredictionsError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [refreshing, setRefreshing] = useState(false)
  const cancelledRef = useRef(false)

  useRevealOnScroll(analysis !== null, loading)

  useEffect(() => {
    cancelledRef.current = false
    return () => {
      cancelledRef.current = true
    }
  }, [])

  const loadPredictions = useCallback(async (active: ActiveSource) => {
    setPredictionsLoading(true)
    setPredictionsError(null)
    try {
      const result =
        active.kind === 'github'
          ? await fetchPredictions(active.owner, active.repo)
          : await fetchLocalPredictions(active.path)
      if (cancelledRef.current) return
      setPredictions(result)
    } catch (err) {
      if (cancelledRef.current) return
      setPredictions(null)
      setPredictionsError(describeAnalysisError(err))
    } finally {
      if (!cancelledRef.current) setPredictionsLoading(false)
    }
  }, [])

  const loadGithub = useCallback(async (ownerName: string, repoName: string) => {
    setLoading(true)
    setError(null)
    setPredictions(null)
    try {
      const result = await fetchRepositoryAnalysis(ownerName, repoName)
      if (cancelledRef.current) return
      const active: ActiveSource = { kind: 'github', owner: ownerName, repo: repoName }
      setSource(active)
      setAnalysis(result)
      void loadPredictions(active)
    } catch (err) {
      if (cancelledRef.current) return
      setError(describeAnalysisError(err))
    } finally {
      if (!cancelledRef.current) setLoading(false)
    }
  }, [loadPredictions])

  const loadLocal = useCallback(async (repoPath: string) => {
    const localError = describeLocalPathError(repoPath)
    if (localError) {
      setError(localError)
      setLoading(false)
      return
    }
    setLoading(true)
    setError(null)
    setPredictions(null)
    try {
      const result = await fetchLocalAnalysis(repoPath.trim())
      if (cancelledRef.current) return
      const active: ActiveSource = {
        kind: 'local',
        path: repoPath.trim(),
        label: result.repository.name,
      }
      setSource(active)
      setAnalysis(result)
      void loadPredictions(active)
    } catch (err) {
      if (cancelledRef.current) return
      setError(describeAnalysisError(err))
    } finally {
      if (!cancelledRef.current) setLoading(false)
    }
  }, [loadPredictions])

  function handleGithubSubmit(rawUrl: string) {
    const parsed = parseGitHubUrl(rawUrl)
    if (!parsed.ok) {
      setError(describeGitHubUrlError(parsed.error))
      setLoading(false)
      return
    }
    setError(null)
    void loadGithub(parsed.owner, parsed.repo)
  }

  function handleClearError() {
    setError(null)
  }

  async function handleRefresh() {
    if (!source || refreshing || loading) return
    setRefreshing(true)
    setError(null)
    try {
      const result =
        source.kind === 'github'
          ? await fetchRepositoryAnalysis(source.owner, source.repo)
          : await fetchLocalAnalysis(source.path)
      if (cancelledRef.current) return
      setAnalysis(result)
      await loadPredictions(source)
    } catch (err) {
      if (cancelledRef.current) return
      setError(describeAnalysisError(err))
    } finally {
      if (!cancelledRef.current) setRefreshing(false)
    }
  }

  function handleImport() {
    setSource(null)
    setAnalysis(null)
    setPredictions(null)
    setPredictionsError(null)
    setError(null)
  }

  return (
    <div className="app">
      <a className="skip-link" href="#main-content">
        Skip to dashboard content
      </a>
      {analysis && source ? (
        <>
          {error && (
            <div style={{ padding: '12px 22px 0' }}>
              <ErrorBanner
                message={error}
                onRetry={() => {
                  if (source.kind === 'github') {
                    void loadGithub(source.owner, source.repo)
                  } else {
                    void loadLocal(source.path)
                  }
                }}
              />
            </div>
          )}
          {loading ? (
            <div className="main full">
              <main className="content" id="main-content" tabIndex={-1}>
                <DashboardSkeleton />
              </main>
            </div>
          ) : (
            <Dashboard
              source={source}
              analysis={analysis}
              predictions={predictions}
              predictionsLoading={predictionsLoading}
              predictionsError={predictionsError}
              onRetryPredictions={() => void loadPredictions(source)}
              onImport={handleImport}
              onRefresh={() => void handleRefresh()}
              refreshing={refreshing}
            />
          )}
        </>
      ) : (
        <div className="main full">
          <main className="content" id="main-content" tabIndex={-1}>
            <Landing
              loading={loading}
              error={error}
              onGithubSubmit={handleGithubSubmit}
              onLocalSubmit={(path) => void loadLocal(path)}
              onClearError={handleClearError}
            />
          </main>
        </div>
      )}
    </div>
  )
}

export default App
