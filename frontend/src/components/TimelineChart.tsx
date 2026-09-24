import type { TimelineBucket } from '../lib/analysis'

export function TimelineChart({
  buckets,
  label,
}: {
  buckets: TimelineBucket[]
  label: string
}) {
  const width = Math.max(buckets.length * 22, 120)
  const height = 120
  const padLeft = 8
  const padBottom = 18
  const plotHeight = height - padBottom - 8
  const peak = Math.max(...buckets.map((bucket) => bucket.commits), 0)
  if (buckets.length === 0 || peak === 0) {
    return <p className="history-note">No dated commits to chart.</p>
  }
  const step = (width - padLeft - 4) / buckets.length
  const barWidth = Math.max(step - 5, 3)
  return (
    <div>
      <svg
        className="timeline-chart"
        width="100%"
        height={height}
        viewBox={`0 0 ${width} ${height}`}
        preserveAspectRatio="none"
        role="img"
        aria-label={label}
      >
        {buckets.map((bucket, index) => {
          const x = padLeft + index * step
          const commitHeight = Math.max(
            (bucket.commits / peak) * plotHeight,
            bucket.commits > 0 ? 3 : 0,
          )
          const churnHeight =
            bucket.files > 0
              ? Math.max(
                  (bucket.files / Math.max(...buckets.map((item) => item.files), 1)) *
                    plotHeight *
                    0.6,
                  2,
                )
              : 0
          return (
            <g key={index}>
              <title>{`${bucket.label}: ${bucket.commits} commits, ${bucket.files} files, +${bucket.additions}/−${bucket.deletions}`}</title>
              <rect
                x={x}
                y={height - padBottom - churnHeight}
                width={barWidth}
                height={churnHeight}
                className="tl-files"
                rx={1}
              />
              <rect
                x={x}
                y={height - padBottom - commitHeight}
                width={barWidth}
                height={commitHeight}
                className="tl-commits"
                rx={1}
              />
              {index % Math.ceil(buckets.length / 6) === 0 && (
                <text x={x} y={height - 5} className="tl-label">
                  {bucket.label}
                </text>
              )}
            </g>
          )
        })}
      </svg>
      <div className="timeline-legend" aria-hidden="true">
        <span className="legend-swatch commits">commits</span>
        <span className="legend-swatch files">files changed</span>
      </div>
    </div>
  )
}
