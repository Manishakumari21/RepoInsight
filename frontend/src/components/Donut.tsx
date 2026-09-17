import type { ReactNode } from 'react'

export function Donut({
  segments,
  size = 124,
  thickness = 14,
  children,
}: {
  segments: { value: number; color: string }[]
  size?: number
  thickness?: number
  children?: ReactNode
}) {
  const total = segments.reduce((sum, segment) => sum + segment.value, 0)
  const radius = (size - thickness) / 2
  const circumference = 2 * Math.PI * radius
  const lengths = segments.map(
    (segment) => (total > 0 ? (segment.value / total) * circumference : 0),
  )
  const offsets = lengths.reduce<number[]>(
    (acc, length) => [...acc, (acc.at(-1) ?? 0) + length],
    [],
  )

  return (
    <div className="donut-frame" style={{ width: size, height: size }}>
      <svg
        className="donut"
        width={size}
        height={size}
        viewBox={`0 0 ${size} ${size}`}
        role="img"
      >
        <circle
          className="donut-track"
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          strokeWidth={thickness}
        />
        {total > 0 && (
          <g transform={`rotate(-90 ${size / 2} ${size / 2})`}>
            {segments.map((segment, index) => (
              <circle
                key={index}
                className="donut-seg"
                cx={size / 2}
                cy={size / 2}
                r={radius}
                fill="none"
                stroke={segment.color}
                strokeWidth={thickness}
                strokeDasharray={`${lengths[index]} ${circumference - lengths[index]}`}
                strokeDashoffset={lengths[index] - offsets[index]}
              />
            ))}
          </g>
        )}
      </svg>
      {children && <div className="donut-center">{children}</div>}
    </div>
  )
}