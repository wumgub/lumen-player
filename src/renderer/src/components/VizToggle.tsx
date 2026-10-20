import React from 'react'

/** A small on/off switch that turns the visualizer (and its render loop) on or off. */
export function VizToggle({
  on,
  onChange,
  className
}: {
  on: boolean
  onChange: (on: boolean) => void
  className?: string
}): React.ReactElement {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      className={`viz-toggle ${on ? 'on' : ''} ${className ?? ''}`}
      title={on ? 'Turn visualizer off' : 'Turn visualizer on'}
      onClick={() => onChange(!on)}
    >
      <span className="viz-toggle-track">
        <span className="viz-toggle-knob" />
      </span>
    </button>
  )
}
