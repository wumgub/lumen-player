import React, { useEffect, useRef, useState } from 'react'
import { Search } from './icons'

/**
 * Compact search: just a magnifying glass until you use it. Tapping the glass
 * expands an input; it collapses back to the glass when left empty. Stays open
 * while there's a query so you can see what you searched.
 */
export function SearchBox({
  value,
  onChange
}: {
  value: string
  onChange: (v: string) => void
}): React.ReactElement {
  const [open, setOpen] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const expanded = open || value.length > 0

  useEffect(() => {
    if (open) inputRef.current?.focus()
  }, [open])

  return (
    <div className={`search-box ${expanded ? 'open' : 'collapsed'}`}>
      <button
        type="button"
        className="search-icon"
        title="Search"
        aria-label="Search"
        onClick={() => {
          setOpen(true)
          inputRef.current?.focus()
        }}
      >
        <Search size={16} />
      </button>
      <input
        ref={inputRef}
        className="search-input"
        aria-label="Search songs, artists, albums"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            onChange('')
            setOpen(false)
            inputRef.current?.blur()
          }
        }}
        onBlur={() => {
          if (!value) setOpen(false)
        }}
      />
    </div>
  )
}
