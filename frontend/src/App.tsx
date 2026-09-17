import { useCallback, useEffect, useRef, useState } from 'react'
import { ApiError, fetchRepositoryAnalysis } from './api'
import {
  describeGitHubUrlError,
  parseGitHubUrl,
} from './lib/githubUrl'
import { NAV_SECTIONS, type SectionId } from './nav'
import type { RepositoryAnalysis } from './types'
import { Sidebar } from './components/Sidebar'
import { Topbar } from './components/Topbar'
import { Landing } from './components/Landing'
import { Dashboard } from './components/Dashboard'
import { ErrorBanner } from './components/Status'
import './App.css'

function buildRepositoryUrl(owner: string, repo: string): string {
  return `https://github.com/${owner}/${repo}`
}

function analysisErrorHint(status: number): string {
  switch (status) {
    case 404:
      return 'Repository not found. Check the owner and repository names in the URL.'
    case 403:
      return 'Access was denied. The repository may be private or GitHub is throttling this request.'
    case 429:
      return 'GitHub API rate limit reached. Wait a moment and try again.'
    case 500:
      return 'The analysis service hit an internal error. Please try again.'
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
  const [owner, setOwner] = useState('')
  const [repo, setRepo] = useState('')
  const [analysis, setAnalysis] = useState<RepositoryAnalysis | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [active, setActive] = useState<SectionId>('overview')
  const cancelledRef = useRef(false)

  useEffect(() => {
    cancelledRef.current = false
    return () => {
      cancelledRef.current = true
    }
  }, [])

  const load = useCallback(async (ownerName: string, repoName: string) => {
    setLoading(true)
    setError(null)
    try {
      const result = await fetchRepositoryAnalysis(ownerName, repoName)
      if (cancelledRef.current) return
      setOwner(ownerName)
      setRepo(repoName)
      setAnalysis(result)
      setActive('overview')
    } catch (err) {
      if (cancelledRef.current) return
      setError(describeAnalysisError(err))
    } finally {
      if (!cancelledRef.current) setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (!analysis) return
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) setActive(entry.target.id as SectionId)
        }
      },
      { rootMargin: '-20% 0px -70% 0px' },
    )
    for (const section of NAV_SECTIONS) {
      const element = document.getElementById(section.id)
      if (element) observer.observe(element)
    }
    return () => observer.disconnect()
  }, [analysis])

  function handleNavigate(id: SectionId) {
    document.getElementById(id)?.scrollIntoView()
    setActive(id)
  }

  function handleUrlSubmit(rawUrl: string) {
    const parsed = parseGitHubUrl(rawUrl)
    if (!parsed.ok) {
      setError(describeGitHubUrlError(parsed.error))
      setLoading(false)
      return
    }
    setError(null)
    void load(parsed.owner, parsed.repo)
  }

  function handleClearError() {
    setError(null)
  }

  return (
    <div className="app">
      <Sidebar
        active={active}
        onNavigate={handleNavigate}
        owner={owner}
        repo={repo}
      />
      <div className="main">
        {analysis && (
          <Topbar
            owner={owner}
            repo={repo}
            branch={analysis.repository.default_branch}
            loading={loading}
            onSubmit={handleUrlSubmit}
          />
        )}
        <main className="content">
          {analysis ? (
            <>
              {error && (
                <ErrorBanner
                  message={error}
                  onRetry={() =>
                    handleUrlSubmit(buildRepositoryUrl(owner, repo))
                  }
                />
              )}
              <Dashboard owner={owner} repo={repo} analysis={analysis} />
            </>
          ) : (
            <Landing
              loading={loading}
              error={error}
              onSubmit={handleUrlSubmit}
              onClearError={handleClearError}
            />
          )}
        </main>
      </div>
    </div>
  )
}

export default App