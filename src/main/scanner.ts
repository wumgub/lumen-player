import { readdir, stat } from 'fs/promises'
import type { Dirent } from 'fs'
import { basename, extname, join } from 'path'
import { parseFile } from 'music-metadata'
import { AUDIO_EXTENSIONS, type ScanProgress, type ScanResult } from '../shared/types'
import { listAllPaths, removeMissing, upsertTrack } from './db'

const AUDIO = new Set<string>(AUDIO_EXTENSIONS)

async function walk(dir: string, out: string[]): Promise<void> {
  let entries: Dirent<string>[]
  try {
    entries = (await readdir(dir, { withFileTypes: true, encoding: 'utf8' })) as Dirent<string>[]
  } catch {
    return
  }
  for (const entry of entries) {
    if (entry.name.startsWith('.')) continue
    const full = join(dir, entry.name)
    if (entry.isDirectory()) {
      await walk(full, out)
    } else if (entry.isFile() && AUDIO.has(extname(entry.name).toLowerCase())) {
      out.push(full)
    }
  }
}

function pickArt(pictures: { format: string; data: Uint8Array }[] | undefined) {
  if (!pictures || pictures.length === 0) return null
  // Prefer a front-cover-ish picture; metadata lib doesn't always flag type, so take first.
  const p = pictures[0]
  return { mime: p.format || 'image/jpeg', data: Buffer.from(p.data) }
}

/**
 * Scan the given folders recursively, upserting every audio file found.
 * `onProgress` is called periodically so the UI can show a progress bar.
 * Tracks whose files no longer exist under any scanned folder are pruned.
 */
export async function scanFolders(
  folders: string[],
  onProgress: (p: ScanProgress) => void
): Promise<ScanResult> {
  onProgress({ phase: 'discovering', processed: 0, total: 0 })

  const files: string[] = []
  for (const folder of folders) {
    await walk(folder, files)
  }

  const total = files.length
  let added = 0
  let updated = 0
  const seen = new Set<string>()

  for (let i = 0; i < files.length; i++) {
    const file = files[i]
    seen.add(file)
    try {
      const meta = await parseFile(file, { duration: true, skipCovers: false })
      const c = meta.common
      const title = c.title?.trim() || basename(file, extname(file))
      const res = upsertTrack({
        path: file,
        title,
        artist: c.artist?.trim() || c.artists?.[0]?.trim() || 'Unknown Artist',
        albumArtist: c.albumartist?.trim() || '',
        album: c.album?.trim() || '',
        genre: c.genre?.[0]?.trim() || '',
        year: c.year ?? null,
        trackNo: c.track?.no ?? null,
        discNo: c.disk?.no ?? null,
        duration: meta.format.duration ?? 0,
        art: pickArt(c.picture)
      })
      if (res.isNew) added++
      else updated++
    } catch {
      // Unreadable/corrupt file — index it minimally so it's still visible.
      try {
        const res = upsertTrack({
          path: file,
          title: basename(file, extname(file)),
          artist: 'Unknown Artist',
          albumArtist: '',
          album: '',
          genre: '',
          year: null,
          trackNo: null,
          discNo: null,
          duration: 0,
          art: null
        })
        if (res.isNew) added++
      } catch {
        /* skip */
      }
    }

    if (i % 10 === 0 || i === files.length - 1) {
      onProgress({ phase: 'parsing', processed: i + 1, total, current: basename(file) })
    }
  }

  // Prune DB rows whose files vanished — but only prune files that live under
  // one of the folders we just scanned (leave other roots untouched).
  const underScanned = (p: string): boolean => folders.some((f) => p.startsWith(f))
  const survivors = new Set<string>()
  for (const p of listAllPaths()) {
    if (seen.has(p) || !underScanned(p)) survivors.add(p)
  }
  removeMissing(survivors)

  onProgress({ phase: 'done', processed: total, total })
  return { added, updated, total }
}

export async function isDirectory(p: string): Promise<boolean> {
  try {
    return (await stat(p)).isDirectory()
  } catch {
    return false
  }
}
