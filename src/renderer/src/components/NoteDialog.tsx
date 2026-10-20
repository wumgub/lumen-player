import React, { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

export interface NoteRequest {
  /** Track title, shown for context. */
  title: string
  initial: string
  onSubmit: (value: string) => void
}

/** Edit a track's free-text note. Empty is allowed (clears the note).
 * ⌘/Ctrl+Enter saves; Esc cancels. */
export function NoteDialog({
  request,
  onClose
}: {
  request: NoteRequest | null
  onClose: () => void
}): React.ReactElement | null {
  const [value, setValue] = useState('')
  const ref = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    if (request) {
      setValue(request.initial)
      requestAnimationFrame(() => {
        ref.current?.focus()
        ref.current?.select()
      })
    }
  }, [request])

  if (!request) return null

  const submit = (): void => {
    const trimmed = value.trim()
    if (trimmed !== request.initial.trim()) request.onSubmit(trimmed)
    onClose()
  }

  return createPortal(
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-title">Note</div>
        <div className="modal-label">
          A note for <strong>{request.title}</strong>. It&rsquo;s searchable — add a romanized
          title or keywords to find songs you can&rsquo;t type.
        </div>
        <textarea
          ref={ref}
          className="modal-input modal-textarea"
          rows={3}
          placeholder="e.g. pachka sigaret — kino"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) submit()
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
