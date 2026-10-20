import React, { useEffect, useRef } from 'react'
import { usePlayer } from '../context/PlayerContext'

/**
 * A hidden, always-in-document home for the shared <video> element. Removing a
 * *playing* media element from the DOM makes the browser pause it, so when the
 * video isn't visible (e.g. the user switched back to album art) we move the
 * element here rather than removing it — keeping playback going.
 */
let videoPark: HTMLElement | null = null
function getVideoPark(): HTMLElement {
  if (!videoPark) {
    videoPark = document.createElement('div')
    videoPark.style.display = 'none'
    videoPark.setAttribute('aria-hidden', 'true')
    document.body.appendChild(videoPark)
  }
  return videoPark
}

/**
 * Displays the engine's shared <video> element (the thing actually playing) by
 * re-parenting it into this container. On unmount the element is moved to a
 * hidden in-document park (not removed), so switching away from Video doesn't
 * pause playback. Only one VideoSurface should be mounted at a time, since there
 * is a single media element.
 */
export function VideoSurface({ className }: { className?: string }): React.ReactElement {
  const { engine } = usePlayer()
  const hostRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const host = hostRef.current
    if (!host) return
    const el = engine.el
    el.classList.add('video-el')
    // Re-parenting in a single call keeps the element continuously in the
    // document, so the browser never runs its pause-on-removal steps.
    host.appendChild(el)
    return () => {
      getVideoPark().appendChild(el)
    }
  }, [engine])

  return <div ref={hostRef} className={`video-surface ${className ?? ''}`} />
}
