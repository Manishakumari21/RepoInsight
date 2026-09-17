import { useState, type FormEvent } from 'react'

export function Topbar({
  owner,
  repo,
  branch,
  loading,
  onSubmit,
}: {
  owner: string
  repo: string
  branch?: string
  loading: boolean
  onSubmit: (url: string) => void
}) {
  const [url, setUrl] = useState(`https://github.com/${owner}/${repo}`)

  function handleSubmit(event: FormEvent) {
    event.preventDefault()
    onSubmit(url)
  }

  return (
    <header className="topbar">
      <div className="topbar-breadcrumb">
        <a
          className="repo-chip"
          href={`https://github.com/${owner}/${repo}`}
          target="_blank"
          rel="noreferrer"
        >
          {owner}/{repo}
        </a>
        {branch && <span className="branch-chip">{branch}</span>}
      </div>
      <form className="search" onSubmit={handleSubmit} noValidate>
        <input
          className="search-input"
          type="text"
          inputMode="url"
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          placeholder="https://github.com/{owner}/{repository}"
          aria-label="GitHub repository URL"
          spellCheck={false}
        />
        <button className="search-button" type="submit" disabled={loading}>
          {loading ? 'Analyzing…' : 'Analyze'}
        </button>
      </form>
    </header>
  )
}