import type { ReactNode } from 'react'
import { Cell, Pie, PieChart } from 'recharts'

export function Donut({
  segments,
  size = 124,
  thickness = 14,
  children,
  label,
}: {
  segments: { value: number; color: string; name?: string }[]
  size?: number
  thickness?: number
  children?: ReactNode
  label?: string
}) {
  const total = segments.reduce((sum, segment) => sum + segment.value, 0)
  const ariaLabel =
    label ??
    segments
      .map(
        (segment, index) =>
          `${segment.name ?? `segment ${index + 1}`}: ${segment.value}`,
      )
      .join(', ')
  const innerRadius = Math.max(size / 2 - thickness, 0)
  const outerRadius = size / 2 - 2

  return (
    <div className="donut-frame" style={{ width: size, height: size }}>
      <div
        className="donut"
        role="img"
        aria-label={ariaLabel}
        style={{ width: size, height: size }}
      >
        {total > 0 ? (
          <PieChart width={size} height={size}>
            <Pie
              data={segments}
              dataKey="value"
              nameKey="name"
              cx="50%"
              cy="50%"
              innerRadius={innerRadius}
              outerRadius={Math.max(outerRadius, innerRadius + 1)}
              startAngle={90}
              endAngle={-270}
              stroke="none"
              isAnimationActive={false}
            >
              {segments.map((segment, index) => (
                <Cell
                  key={index}
                  fill={segment.color}
                  className="donut-seg"
                />
              ))}
            </Pie>
          </PieChart>
        ) : (
          <svg
            className="donut"
            width={size}
            height={size}
            viewBox={`0 0 ${size} ${size}`}
            aria-hidden="true"
          >
            <circle
              className="donut-track"
              cx={size / 2}
              cy={size / 2}
              r={(size - thickness) / 2}
              fill="none"
              strokeWidth={thickness}
            />
          </svg>
        )}
      </div>
      {children && <div className="donut-center">{children}</div>}
    </div>
  )
}
