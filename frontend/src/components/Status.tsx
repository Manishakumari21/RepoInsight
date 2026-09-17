export function LoadingState() {
  return (
    <div className="loading" role="status">
      <div className="spinner" aria-hidden="true" />
      <div className="loading-text">Analyzing repository…</div>
    </div>
  )
}

export function ErrorBanner({
  message,
  onRetry,
}: {
  message: string
  onRetry: () => void
}) {
  return (
    <div className="error-banner" role="alert">
      <span>{message}</span>
      <button type="button" className="retry-button" onClick={onRetry}>
        Retry
      </button>
    </div>
  )
}

export function Empty({ text }: { text: string }) {
  return <div className="empty">{text}</div>
}