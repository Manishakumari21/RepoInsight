export function Logo({ size = 20, title }: { size?: number; title?: string }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      role="img"
      aria-label={title ?? 'RepoInsight logo'}
      aria-hidden={title ? undefined : true}
    >
      {}
      <line x1="2.5" y1="12" x2="6" y2="12" stroke="var(--muted)" strokeWidth="1.8" strokeLinecap="round" strokeDasharray="2.5 2.5" />
      {}
      <circle cx="9.5" cy="12" r="3.2" fill="var(--primary)" />
      <circle cx="9.5" cy="12" r="3.2" stroke="var(--text)" strokeOpacity="0.35" strokeWidth="1" />
      {}
      <line x1="12.2" y1="10.2" x2="17.2" y2="6.4" stroke="var(--cyan)" strokeWidth="1.8" strokeLinecap="round" />
      <line x1="12.2" y1="13.8" x2="17.2" y2="17.6" stroke="var(--teal)" strokeWidth="1.8" strokeLinecap="round" />
      {}
      <circle cx="18.8" cy="5.4" r="2.2" fill="var(--bg-secondary)" stroke="var(--cyan)" strokeWidth="1.8" />
      <circle cx="18.8" cy="18.6" r="2.2" fill="var(--bg-secondary)" stroke="var(--teal)" strokeWidth="1.8" />
    </svg>
  )
}
