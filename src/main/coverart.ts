// Resolves public album-art URLs from the free iTunes Search API so Discord can
// display per-track cover art (it can't read local embedded art). Results are
// cached per artist/album, with in-flight de-duplication.

const cache = new Map<string, string | null>()
const inflight = new Map<string, Promise<string | null>>()

export async function getCoverUrl(
  artist: string,
  album: string,
  title: string
): Promise<string | null> {
  const useAlbum = album.trim().length > 0
  const term = [artist, useAlbum ? album : title]
    .map((s) => s.trim())
    .filter(Boolean)
    .join(' ')
    .trim()
  if (!term) return null

  const key = `${useAlbum ? 'a' : 's'}:${term.toLowerCase()}`
  if (cache.has(key)) return cache.get(key) ?? null
  const existing = inflight.get(key)
  if (existing) return existing

  const p = fetchCover(term, useAlbum)
    .then((url) => {
      cache.set(key, url)
      inflight.delete(key)
      return url
    })
    .catch(() => {
      inflight.delete(key)
      return null
    })
  inflight.set(key, p)
  return p
}

async function fetchCover(term: string, isAlbum: boolean): Promise<string | null> {
  const url = `https://itunes.apple.com/search?term=${encodeURIComponent(term)}&entity=${
    isAlbum ? 'album' : 'song'
  }&limit=1`
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), 5000)
  try {
    const res = await fetch(url, { signal: controller.signal })
    if (!res.ok) return null
    const data = (await res.json()) as {
      results?: { artworkUrl100?: string; artworkUrl60?: string }[]
    }
    const art = data.results?.[0]?.artworkUrl100 || data.results?.[0]?.artworkUrl60
    if (!art) return null
    // iTunes returns .../100x100bb.jpg — bump it up to a crisp 512px.
    return art.replace(/\/\d+x\d+bb\./, '/512x512bb.')
  } catch {
    return null
  } finally {
    clearTimeout(timer)
  }
}
