import React, { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

export interface RenameRequest {
  kind: 'song' | 'artist' | 'album'
  label: string
  initial: string
  onSubmit: (value: string) => void
}

export function RenameDialog({
  request,
  onClose
}: {
  request: RenameRequest | null
  onClose: () => void
}): React.ReactElement | null {
  const [value, setValue] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (request) {
      setValue(request.initial)
      // Focus + select the whole name for quick replacement.
      requestAnimationFrame(() => {
        inputRef.current?.focus()
        inputRef.current?.select()
      })
    }
  }, [request])

  if (!request) return null

  const submit = (): void => {
    const trimmed = value.trim()
    if (trimmed && trimmed !== request.initial) request.onSubmit(trimmed)
    onClose()
  }

  return createPortal(
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-title">Rename {request.kind}</div>
        <div className="modal-label">{request.label}</div>
        <input
          ref={inputRef}
          className="modal-input"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') submit()
            else if (e.key === 'Escape') onClose()
          }}
        />
        <div className="modal-actions">
          <button className="modal-btn ghost" onClick={onClose}>
            Cancel
          </button>
          <button className="modal-btn primary" onClick={submit}>
            Save
          </button>
        </div>
      </div>
    </div>,
    document.body
  )
}
