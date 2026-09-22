export type ActiveSource =
  | { kind: 'github'; owner: string; repo: string }
  | { kind: 'local'; path: string; label: string }

export function sourceLabel(source: ActiveSource): string {
  return source.kind === 'github'
    ? `${source.owner}/${source.repo}`
    : source.label
}

export function sourceHref(source: ActiveSource): string | null {
  return source.kind === 'github'
    ? `https://github.com/${source.owner}/${source.repo}`
    : null
}
