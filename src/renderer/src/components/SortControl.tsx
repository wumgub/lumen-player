import React, { useEffect, useRef, useState } from 'react'
import { SORT_LABELS, type SortKey, type SortState } from '../types'

export function SortControl({
  sort,
  options,
  onChange
}: {
  sort: SortState
  options: SortKey[]
  onChange: (s: SortState) => void
}): React.ReactElement {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const onDoc = (e: MouseEvent): void => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', onDoc)
    return () => document.removeEventListener('mousedown', onDoc)
  }, [])

  const toggleDir = (): void => onChange({ ...sort, dir: sort.dir === 'asc' ? 'desc' : 'asc' })

  return (
    <div className="sort-control" ref={ref}>
      <button className="sort-main" onClick={() => setOpen((o) => !o)}>
        <span className="sort-dim">Sort</span>
        <span className="sort-key">{SORT_LABELS[sort.key]}</span>
        <span className="chevron">▾</span>
      </button>
      <button
        className="sort-dir"
        onClick={toggleDir}
        title={sort.dir === 'asc' ? 'Ascending' : 'Descending'}
      >
        {sort.dir === 'asc' ? '↑' : '↓'}
      </button>
      {open && (
        <div className="sort-menu">
          {options.map((key) => (
            <button
              key={key}
              className={`sort-option ${key === sort.key ? 'active' : ''}`}
              onClick={() => {
                onChange({ key, dir: sort.dir })
                setOpen(false)
              }}
            >
              {SORT_LABELS[key]}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
