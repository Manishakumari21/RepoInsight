import { useState, type FormEvent } from 'react'
import { ErrorBanner } from './Status'
import { Logo } from './Logo'

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
            <span className="brand-mark" aria-hidden="true"><Logo size={22} /></span>
            <div>
              <div className="brand-title">RepoInsight</div>
              <div className="brand-sub">change intelligence</div>
            </div>
          </div>
          <span className="landing-eyebrow">Repository intelligence workbench</span>
          <h1 className="landing-title">
            Understand how your repository changes — <span className="grad">and what may change next.</span>
          </h1>
          <p className="landing-sub">
            Explore structure, history, coupling and prediction in one
            repository intelligence workspace. Every claim links to
            repository evidence.
          </p>
          <ul className="landing-points">
            <li><CheckIcon /> Understand structure — files, modules, dependencies</li>
            <li><CheckIcon /> Understand history — timelines, change sets, authors</li>
            <li><CheckIcon /> Discover hidden coupling — files that change together</li>
            <li><CheckIcon /> Predict likely changes — with evidence, not just scores</li>
            <li><CheckIcon /> Explain predictions — every number traces to commits</li>
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
      <div className="landing-preview" aria-label="What you can do in the workbench">
        <div className="landing-preview-card">
          <h3>Understand structure</h3>
          <p>Browse the file tree, filter by <code>Dependencies / Temporal / Co-change</code>, and inspect any file.</p>
        </div>
        <div className="landing-preview-card">
          <h3>Understand history</h3>
          <p>Filter commits by date, author, directory, and change type. Every row opens its changed files.</p>
        </div>
        <div className="landing-preview-card">
          <h3>Discover coupling</h3>
          <p>Rank <code>File A ↔ File B</code> pairs by co-changes, with trend and historical examples.</p>
        </div>
        <div className="landing-preview-card">
          <h3>Predict changes</h3>
          <p><code>Likely / Possible / Low likelihood</code> with confidence, recent activity, and coupling context.</p>
        </div>
        <div className="landing-preview-card">
          <h3>Explain with evidence</h3>
          <p>Structural, historical, and temporal evidence plus real commits — <code>evidence ≠ prediction</code>.</p>
        </div>
        <div className="landing-preview-card">
          <h3>Simulate impact</h3>
          <p>Describe a planned change in words and see which capabilities and files deserve review first.</p>
        </div>
      </div>
    </div>
  )
}
