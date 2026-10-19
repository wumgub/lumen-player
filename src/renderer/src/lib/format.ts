export function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return '0:00'
  const m = Math.floor(seconds / 60)
  const s = Math.floor(seconds % 60)
  return `${m}:${s.toString().padStart(2, '0')}`
}

export function pluralize(n: number, word: string): string {
  return `${n.toLocaleString()} ${word}${n === 1 ? '' : 's'}`
}
