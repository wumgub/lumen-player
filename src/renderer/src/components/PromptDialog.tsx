import React, { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

/** A simple one-field text prompt modal (Electron doesn't support window.prompt). */
export function PromptDialog({
  open,
  title,
  label,
  placeholder,
  initial = '',
  submitLabel = 'Create',
  onSubmit,
  onClose
}: {
  open: boolean
  title: string
  label?: string
  placeholder?: string
  initial?: string
  submitLabel?: string
  onSubmit: (value: string) => void
  onClose: () => void
}): React.ReactElement | null {
  const [value, setValue] = useState(initial)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (open) {
      setValue(initial)
      requestAnimationFrame(() => {
        inputRef.current?.focus()
        inputRef.current?.select()
      })
    }
  }, [open, initial])

  if (!open) return null

  const submit = (): void => {
    const trimmed = value.trim()
    if (trimmed) onSubmit(trimmed)
    onClose()
  }

  return createPortal(
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-title">{title}</div>
        {label && <div className="modal-label">{label}</div>}
        <input
          ref={inputRef}
          className="modal-input"
          placeholder={placeholder}
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
            {submitLabel}
          </button>
        </div>
      </div>
    </div>,
    document.body
  )
}
