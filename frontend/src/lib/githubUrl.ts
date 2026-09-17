export type GitHubUrlError =
  | 'empty'
  | 'invalid'
  | 'missing-owner'
  | 'missing-repo'
  | 'malformed'

export type GitHubUrlParse =
  | { ok: true; owner: string; repo: string }
  | { ok: false; error: GitHubUrlError }

function stripGitSuffix(value: string): string {
  let result = value
  while (result.toLowerCase().endsWith('.git')) {
    result = result.slice(0, result.length - 4)
  }
  return result
}

function toUrl(input: string): URL | null {
  const withScheme = /^https?:\/\//i.test(input) ? input : `https://${input}`
  try {
    return new URL(withScheme)
  } catch {
    return null
  }
}

export function parseGitHubUrl(raw: string): GitHubUrlParse {
  const input = raw.trim()
  if (!input) return { ok: false, error: 'empty' }

  const url = toUrl(input)
  if (!url) return { ok: false, error: 'malformed' }

  const protocol = url.protocol.toLowerCase()
  if (protocol !== 'http:' && protocol !== 'https:') {
    return { ok: false, error: 'invalid' }
  }
  if (url.username || url.password) {
    return { ok: false, error: 'malformed' }
  }

  const host = url.hostname.toLowerCase().replace(/^www\./i, '')
  if (host !== 'github.com') {
    return { ok: false, error: 'invalid' }
  }

  const segments = url.pathname.split('/').filter(Boolean)
  const owner = segments[0] ?? ''
  const repo = stripGitSuffix(segments[1] ?? '')

  if (segments.length === 0 || !owner) {
    return { ok: false, error: 'missing-owner' }
  }
  if (segments.length === 1 || !repo) {
    return { ok: false, error: 'missing-repo' }
  }
  if (segments.length > 2) {
    return { ok: false, error: 'malformed' }
  }

  return { ok: true, owner, repo }
}

export function describeGitHubUrlError(error: GitHubUrlError): string {
  const example = 'https://github.com/{owner}/{repository}'
  switch (error) {
    case 'empty':
      return 'Please enter a GitHub repository URL.'
    case 'invalid':
      return `The URL is not a valid GitHub URL. Expected something like ${example}.`
    case 'missing-owner':
      return `The URL is missing the repository owner. Expected ${example}.`
    case 'missing-repo':
      return `The URL is missing the repository name. Expected ${example}.`
    case 'malformed':
      return `The GitHub URL is malformed. Expected ${example}.`
  }
}