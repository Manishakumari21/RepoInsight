import { useState, type FormEvent } from 'react'
import { ErrorBanner } from './Status'

export type ImportMode = 'github' | 'local'

function CheckIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <circle cx="12" cy="12" r="9" strokeOpacity={0.4} />
      <path d="M8.5 12.5l2.5 2.5 4.5-5.5" />
    </svg>
  )
}

export function Landing({
  loading,
  error,
  onGithubSubmit,
  onLocalSubmit,
  onClearError,
}: {
  loading: boolean
  error: string | null
  onGithubSubmit: (url: string) => void
  onLocalSubmit: (path: string) => void
  onClearError: () => void
}) {
  const [mode, setMode] = useState<ImportMode>('github')
  const [url, setUrl] = useState('')
  const [localPath, setLocalPath] = useState('')

  function handleSubmit(event: FormEvent) {
    event.preventDefault()
    if (mode === 'github') {
      onGithubSubmit(url)
    } else {
      onLocalSubmit(localPath)
    }
  }

  const isLocal = mode === 'local'

  return (
    <div className="landing">
      <div className="landing-shell">
        <div>
          <div className="landing-brand">
            <span className="brand-mark" aria-hidden="true">RI</span>
            <div>
              <div className="brand-title">RepoInsight</div>
              <div className="brand-sub">change intelligence</div>
            </div>
          </div>
          <span className="landing-eyebrow">Repository intelligence · Phase 8</span>
          <h1 className="landing-title">
            Know what breaks <span className="grad">before you change it.</span>
          </h1>
          <p className="landing-sub">
            RepoInsight reads commit history and code structure to map
            change propagation, co-change coupling, and rework risk —
            with evidence kept separate from prediction.
          </p>
          <ul className="landing-points">
            <li><CheckIcon /> Temporal propagation graph across files and modules</li>
            <li><CheckIcon /> Per-file rework probability with feature evidence</li>
            <li><CheckIcon /> Sequences, follow-ups, and candidate rework signals</li>
            <li><CheckIcon /> Local Git analysis — no token required</li>
          </ul>
        </div>

        <div className="landing-card">
          <div className="landing-card-inner">
            <div className="landing-card-title">Analyze a repository</div>
            <p className="landing-card-sub">
              {isLocal
                ? 'Point at a local checkout. History stays on your machine.'
                : 'Paste a GitHub URL. Up to 1,000 commits sampled.'}
            </p>
            <form className="landing-form" onSubmit={handleSubmit} noValidate>
              <div className="source-toggle" role="radiogroup" aria-label="Repository source">
                <label className={`source-option${!isLocal ? ' active' : ''}`}>
                  <input
                    type="radio"
                    name="repo-source"
                    value="github"
                    checked={!isLocal}
                    onChange={() => {
                      setMode('github')
                      onClearError()
                    }}
                  />
                  <span>GitHub</span>
                </label>
                <label className={`source-option${isLocal ? ' active' : ''}`}>
                  <input
                    type="radio"
                    name="repo-source"
                    value="local"
                    checked={isLocal}
                    onChange={() => {
                      setMode('local')
                      onClearError()
                    }}
                  />
                  <span>Local Git</span>
                </label>
              </div>
              <div className={error ? 'landing-input-wrap has-error' : 'landing-input-wrap'}>
                {isLocal ? (
                  <input
                    className="landing-input"
                    type="text"
                    value={localPath}
                    onChange={(event) => {
                      setLocalPath(event.target.value)
                      onClearError()
                    }}
                    placeholder="/home/user/projects/my-repo"
                    aria-label="Local Git repository path"
                    autoFocus
                    spellCheck={false}
                  />
                ) : (
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
                )}
                <button className="landing-button" type="submit" disabled={loading}>
                  {loading ? 'Analyzing…' : 'Analyze'}
                </button>
              </div>
              {error && (
                <div className="landing-feedback">
                  <ErrorBanner
                    message={error}
                    onRetry={() => (isLocal ? onLocalSubmit(localPath) : onGithubSubmit(url))}
                  />
                </div>
              )}
              {loading && (
                <div className="landing-loading" role="status">
                  <div className="spinner" aria-hidden="true" />
                  <span>Collecting commits, structure, and history…</span>
                </div>
              )}
            </form>

            <div className="landing-hint">
              {isLocal ? (
                <>
                  <span>Filesystem path of a Git checkout</span>
                  <code>/home/user/projects/my-repo</code>
                </>
              ) : (
                <>
                  <span>Accepts</span>
                  <code>https://github.com/{'{'}owner{'}'}/{'{'}repository{'}'}</code>
                  <span>· .git suffix ok</span>
                </>
              )}
            </div>
          </div>
        </div>
      </div>

      <div className="landing-foot" aria-label="Pipeline stages">
        <span>collect → structure → temporal → dataset → predict → explain</span>
        <span>evidence ≠ prediction</span>
      </div>
    </div>
  )
}
