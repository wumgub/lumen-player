import { app, BrowserWindow, dialog, ipcMain, protocol, shell } from 'electron'
import { createReadStream, existsSync, promises as fs } from 'fs'
import { Readable } from 'stream'
import { extname, join } from 'path'
import { fileURLToPath } from 'url'
import {
  addFolder,
  addToPlaylist,
  createPlaylist,
  deletePlaylist,
  getAppSettings,
  getArt,
  getTrack,
  initDb,
  listAlbums,
  listArtists,
  listFolders,
  listPlaylists,
  playlistTracks,
  queryTracks,
  removeFolder,
  removeFromPlaylist,
  renameAlbum,
  renameArtist,
  renamePlaylist,
  renameTrack,
  reorderAlbum,
  setAppSettings,
  setTrackAlbum,
  stats
} from './db'
import { scanFolders } from './scanner'
import { presence } from './presence'
import type { AppSettings, LibraryQuery, PresencePayload } from '../shared/types'

const __dirname = fileURLToPath(new URL('.', import.meta.url))

const MIME: Record<string, string> = {
  '.mp3': 'audio/mpeg',
  '.flac': 'audio/flac',
  '.wav': 'audio/wav',
  '.aif': 'audio/aiff',
  '.aiff': 'audio/aiff',
  '.m4a': 'audio/mp4',
  '.aac': 'audio/aac',
  '.ogg': 'audio/ogg',
  '.oga': 'audio/ogg',
  '.opus': 'audio/ogg'
}

protocol.registerSchemesAsPrivileged([
  { scheme: 'lumen-media', privileges: { stream: true, supportFetchAPI: true, bypassCSP: true } },
  { scheme: 'lumen-art', privileges: { supportFetchAPI: true, bypassCSP: true } }
])

function registerProtocols(): void {
  // Streams an audio file by track id with HTTP Range support (so seeking works).
  protocol.handle('lumen-media', async (request) => {
    const url = new URL(request.url) // lumen-media://t/<id>
    const id = Number(url.pathname.replace(/^\//, '') || url.hostname)
    const track = getTrack(id)
    if (!track) return new Response('Not found', { status: 404 })

    let size: number
    try {
      size = (await fs.stat(track.path)).size
    } catch {
      return new Response('File missing', { status: 404 })
    }
    const type = MIME[extname(track.path).toLowerCase()] ?? 'application/octet-stream'
    const range = request.headers.get('Range')

    if (range) {
      const m = /bytes=(\d*)-(\d*)/.exec(range)
      const start = m && m[1] ? parseInt(m[1], 10) : 0
      const end = m && m[2] ? Math.min(parseInt(m[2], 10), size - 1) : size - 1
      const stream = createReadStream(track.path, { start, end })
      return new Response(Readable.toWeb(stream) as ReadableStream, {
        status: 206,
        headers: {
          'Content-Type': type,
          'Content-Length': String(end - start + 1),
          'Content-Range': `bytes ${start}-${end}/${size}`,
          'Accept-Ranges': 'bytes'
        }
      })
    }

    const stream = createReadStream(track.path)
    return new Response(Readable.toWeb(stream) as ReadableStream, {
      status: 200,
      headers: { 'Content-Type': type, 'Content-Length': String(size), 'Accept-Ranges': 'bytes' }
    })
  })

  // Serves embedded album art by track id.
  protocol.handle('lumen-art', async (request) => {
    const url = new URL(request.url)
    const id = Number(url.pathname.replace(/^\//, '') || url.hostname)
    const art = getArt(id)
    if (!art) return new Response('No art', { status: 404 })
    return new Response(new Uint8Array(art.data), {
      status: 200,
      headers: { 'Content-Type': art.mime, 'Cache-Control': 'max-age=86400' }
    })
  })
}

function createWindow(): void {
  const win = new BrowserWindow({
    width: 1280,
    height: 820,
    minWidth: 900,
    minHeight: 600,
    show: false,
    backgroundColor: '#0b0b12',
    titleBarStyle: process.platform === 'darwin' ? 'hiddenInset' : 'default',
    webPreferences: {
      preload: join(__dirname, '../preload/index.mjs'),
      sandbox: false,
      contextIsolation: true,
      nodeIntegration: false
    }
  })

  win.on('ready-to-show', () => win.show())

  win.webContents.setWindowOpenHandler(({ url }) => {
    shell.openExternal(url)
    return { action: 'deny' }
  })

  if (process.env.ELECTRON_RENDERER_URL) {
    win.loadURL(process.env.ELECTRON_RENDERER_URL)
  } else {
    win.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

function registerIpc(): void {
  ipcMain.handle('dialog:pickFolder', async () => {
    const res = await dialog.showOpenDialog({
      properties: ['openDirectory', 'multiSelections']
    })
    return res.canceled ? [] : res.filePaths
  })

  ipcMain.handle('library:folders', () => listFolders())
  ipcMain.handle('library:addFolder', (_e, p: string) => addFolder(p))
  ipcMain.handle('library:removeFolder', (_e, p: string) => removeFolder(p))

  ipcMain.handle('library:scan', async (e) => {
    const folders = listFolders()
    return scanFolders(folders, (progress) => {
      if (!e.sender.isDestroyed()) e.sender.send('library:scanProgress', progress)
    })
  })

  ipcMain.handle('library:query', (_e, q: LibraryQuery) => queryTracks(q))
  ipcMain.handle('library:artists', () => listArtists())
  ipcMain.handle('library:albums', () => listAlbums())
  ipcMain.handle('library:stats', () => stats())
  ipcMain.handle('track:get', (_e, id: number) => getTrack(id))

  ipcMain.handle('meta:renameTrack', (_e, id: number, title: string) => renameTrack(id, title))
  ipcMain.handle('meta:renameArtist', (_e, oldName: string, newName: string) =>
    renameArtist(oldName, newName)
  )
  ipcMain.handle('meta:renameAlbum', (_e, oldName: string, newName: string) =>
    renameAlbum(oldName, newName)
  )
  ipcMain.handle('meta:setTrackAlbum', (_e, trackIds: number[], album: string) =>
    setTrackAlbum(trackIds, album)
  )
  ipcMain.handle('album:reorder', (_e, album: string, orderedTrackIds: number[]) =>
    reorderAlbum(album, orderedTrackIds)
  )

  ipcMain.handle('playlist:list', () => listPlaylists())
  ipcMain.handle('playlist:create', (_e, name: string) => createPlaylist(name))
  ipcMain.handle('playlist:delete', (_e, id: number) => deletePlaylist(id))
  ipcMain.handle('playlist:rename', (_e, id: number, name: string) => renamePlaylist(id, name))
  ipcMain.handle('playlist:add', (_e, id: number, trackIds: number[]) => addToPlaylist(id, trackIds))
  ipcMain.handle('playlist:remove', (_e, id: number, trackId: number) =>
    removeFromPlaylist(id, trackId)
  )
  ipcMain.handle('playlist:tracks', (_e, id: number) => playlistTracks(id))

  ipcMain.handle('settings:get', () => getAppSettings())
  ipcMain.handle('settings:set', async (_e, patch: Partial<AppSettings>) => {
    const next = setAppSettings(patch)
    await presence.configure(next.discordEnabled, next.discordClientId, next.discordLargeImageKey)
    return next
  })

  ipcMain.handle('presence:set', (_e, payload: PresencePayload | null) => presence.update(payload))
}

app.whenReady().then(() => {
  initDb()
  registerProtocols()
  registerIpc()

  // Show the custom icon in the dock during development (packaged builds get it
  // from the app bundle via electron-builder).
  if (process.platform === 'darwin' && app.dock) {
    const devIcon = join(process.cwd(), 'build', 'icon.png')
    if (existsSync(devIcon)) app.dock.setIcon(devIcon)
  }

  createWindow()

  // Restore Discord presence connection from saved settings.
  const settings = getAppSettings()
  void presence.configure(
    settings.discordEnabled,
    settings.discordClientId,
    settings.discordLargeImageKey
  )

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('before-quit', () => {
  void presence.shutdown()
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})
