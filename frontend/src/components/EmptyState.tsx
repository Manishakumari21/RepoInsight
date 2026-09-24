export function EmptyState({ title, body }: { title: string; body: string }) {
  return (
    <div className="empty-state-pro" role="status">
      <strong>{title}</strong>
      <p>{body}</p>
    </div>
  )
}
