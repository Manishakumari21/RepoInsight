export function Sparkline({
  values,
  label,
  width = 120,
  height = 28,
}: {
  values: number[]
  label: string
  width?: number
  height?: number
}) {
  const peak = Math.max(...values, 0)
  const barWidth = values.length > 0 ? width / values.length : width
  return (
    <svg
      className="sparkline"
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      role="img"
      aria-label={label}
    >
      {values.map((value, index) => {
        const barHeight =
          peak > 0 ? Math.max((value / peak) * (height - 4), value > 0 ? 2 : 0) : 0
        return (
          <rect
            key={index}
            x={index * barWidth + 1}
            y={height - barHeight}
            width={Math.max(barWidth - 2, 1)}
            height={barHeight}
            rx={1}
          />
        )
      })}
    </svg>
  )
}
