export function LoadingState({ detail }: { detail?: string }) {
  return (
    <div className="loading" role="status">
      <div className="spinner" aria-hidden="true" />
      <div className="loading-text">{detail ?? 'Analyzing repository…'}</div>
    </div>
  )
}

export function NoticeBanner({ message }: { message: string }) {
  return (
    <div className="notice-banner" role="status">
      <span>{message}</span>
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
        Try again
      </button>
    </div>
  )
}

export function Empty({ text }: { text: string }) {
  return <div className="empty">{text}</div>
}