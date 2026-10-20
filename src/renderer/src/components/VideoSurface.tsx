import React, { useEffect, useRef } from 'react'
import { usePlayer } from '../context/PlayerContext'

/**
 * Displays the engine's shared <video> element (the thing actually playing) by
 * re-parenting it into this container. On unmount the element returns to being
 * detached, which keeps audio playing. Only one VideoSurface should be mounted
 * at a time, since there is a single media element.
 */
export function VideoSurface({ className }: { className?: string }): React.ReactElement {
  const { engine } = usePlayer()
  const hostRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const host = hostRef.current
    if (!host) return
    const el = engine.el
    el.classList.add('video-el')
    host.appendChild(el)
    return () => {
      if (el.parentNode === host) host.removeChild(el)
    }
  }, [engine])

  return <div ref={hostRef} className={`video-surface ${className ?? ''}`} />
}
