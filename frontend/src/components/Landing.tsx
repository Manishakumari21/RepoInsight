import { useState, type FormEvent } from 'react'
import { ErrorBanner } from './Status'

export function Landing({
  loading,
  error,
  onSubmit,
  onClearError,
}: {
  loading: boolean
  error: string | null
  onSubmit: (url: string) => void
  onClearError: () => void
}) {
  const [url, setUrl] = useState('')

  function handleSubmit(event: FormEvent) {
    event.preventDefault()
    onSubmit(url)
  }

  return (
    <div className="landing">
      <div className="landing-hero">
        <div className="landing-eyebrow">Repository Intelligence</div>
        <h1 className="landing-title">Analyze your first repository</h1>
        <p className="landing-sub">
          RepoInsight looks at commit history and code structure to show
          what&apos;s likely to break together.
        </p>
      </div>

      <form className="landing-form" onSubmit={handleSubmit} noValidate>
        <div className={error ? 'landing-input-wrap has-error' : 'landing-input-wrap'}>
          <input
            className="landing-input"
            type="text"
            inputMode="url"
            value={url}
            onChange={(event) => {
              setUrl(event.target.value)
              onClearError()
            }}
            placeholder="https://github.com/{owner}/{repository}"
            aria-label="GitHub repository URL"
            autoFocus
            spellCheck={false}
          />
          <button className="landing-button" type="submit" disabled={loading}>
            {loading ? 'Analyzing…' : 'Analyze a repository'}
          </button>
        </div>
        {error && (
          <div className="landing-feedback">
            <ErrorBanner message={error} onRetry={() => onSubmit(url)} />
          </div>
        )}
        {loading && (
          <div className="landing-loading" role="status">
            <div className="spinner" aria-hidden="true" />
            <span>Analyzing commits…</span>
          </div>
        )}
      </form>

      <div className="landing-hint">
        <span>Accepts</span>
        <code>https://github.com/{'{'}owner{'}'}/{'{'}repository{'}'}</code>
        <span>· trailing slashes and</span>
        <code>.git</code>
        <span>are fine</span>
      </div>
    </div>
  )
}