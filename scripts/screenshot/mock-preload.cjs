// Mock window.lumen for screenshot captures of the real renderer.
const { contextBridge } = require('electron')

const mk = (id, title, artist, album, year, trackNo, duration, note = '', isVideo = 0) => ({
  id,
  path: `/Music/${artist}/${album}/${title}.${isVideo ? 'mp4' : 'mp3'}`,
  title,
  artist,
  albumArtist: artist,
  album,
  genre: 'Post-punk',
  year,
  trackNo,
  discNo: 1,
  duration,
  hasArt: 1,
  addedAt: Date.now() - id * 86400000,
  note,
  isVideo
})

const songs = [
  mk(1, 'Группа крови', 'Кино', 'Группа крови', 1988, 1, 285, 'gruppa krovi — blood type'),
  mk(2, 'Закрой за мной дверь, я ухожу', 'Кино', 'Группа крови', 1988, 2, 238),
  mk(3, 'Война', 'Кино', 'Группа крови', 1988, 3, 236),
  mk(4, 'Спокойная ночь', 'Кино', 'Группа крови', 1988, 5, 376),
  mk(5, 'Звезда по имени Солнце', 'Кино', 'Звезда по имени Солнце', 1989, 2, 237, 'a star called sun'),
  mk(6, 'Пачка сигарет', 'Кино', 'Звезда по имени Солнце', 1989, 6, 258, 'pack of cigarettes'),
  mk(7, 'Печаль', 'Кино', 'Звезда по имени Солнце', 1989, 8, 309),
  mk(8, 'Судно', 'Molchat Doma', 'Этажи', 2018, 1, 171, 'sudno — bedpan'),
  mk(9, 'Танцевать', 'Molchat Doma', 'Этажи', 2018, 2, 268, '', 1),
  mk(10, 'Клетка', 'Molchat Doma', 'Этажи', 2018, 3, 214),
  mk(11, 'Люди надоели', 'Molchat Doma', 'Этажи', 2018, 4, 199, 'lyudi nadoeli'),
  mk(12, 'Волны', 'Molchat Doma', 'Монумент', 2020, 1, 236),
  mk(13, 'Дискотека', 'Molchat Doma', 'Монумент', 2020, 2, 227, 'diskoteka'),
  mk(14, 'Звёзды', 'Molchat Doma', 'Монумент', 2020, 3, 258)
]

const groupBy = (arr, key) => {
  const m = new Map()
  for (const s of arr) m.set(s[key], (m.get(s[key]) || 0) + 1)
  return m
}

// Give each album a distinct demo cover id so the gradient covers differ.
const albumId = (name) => {
  let h = 0
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0
  return (h % 997) + 1
}

const api = {
  platform: process.platform,
  appInfo: async () => ({ version: '0.1.0', commit: 'abc1234', buildDate: new Date().toISOString() }),
  checkForUpdate: async () => ({
    ok: true,
    current: 'abc1234',
    latest: 'def5678',
    date: new Date().toISOString(),
    message: 'Add the update checker',
    upToDate: false,
    url: 'https://github.com/wumgub/lumen-player/commits/main'
  }),
  pickFolder: async () => [],
  folders: async () => ['/Users/evan/Music'],
  addFolder: async () => {},
  removeFolder: async () => {},
  scan: async () => ({ added: 0, updated: 0, total: songs.length }),
  onScanProgress: () => () => {},
  query: async (q = {}) => {
    let r = songs.slice()
    if (q.artist) r = r.filter((s) => s.artist === q.artist)
    if (q.album) r = r.filter((s) => s.album === q.album)
    if (q.search) {
      const t = q.search.toLowerCase()
      r = r.filter((s) => (s.title + s.artist + s.album + (s.note || '')).toLowerCase().includes(t))
    }
    return r
  },
  artists: async () =>
    [...groupBy(songs, 'artist')].map(([artist, count]) => ({ artist, count })),
  albums: async () => {
    const seen = new Map()
    for (const s of songs) if (!seen.has(s.album)) seen.set(s.album, s.artist)
    return [...seen].map(([album, artist]) => ({
      album,
      artist,
      count: songs.filter((s) => s.album === album).length
    }))
  },
  stats: async () => ({
    tracks: songs.length,
    artists: groupBy(songs, 'artist').size,
    albums: groupBy(songs, 'album').size
  }),
  getTrack: async (id) => songs.find((s) => s.id === id) || null,
  revealTrack: async () => {},
  renameTrack: async () => {},
  renameArtist: async () => {},
  renameAlbum: async () => {},
  setTrackAlbum: async () => {},
  setTrackNote: async () => {},
  reorderAlbum: async () => {},
  playlists: async () => [
    { id: 1, name: 'Late Night Drive', count: 14, createdAt: 0 },
    { id: 2, name: 'Cold Wave', count: 9, createdAt: 0 },
    { id: 3, name: 'Focus', count: 22, createdAt: 0 }
  ],
  createPlaylist: async (name) => ({ id: 9, name, createdAt: 0 }),
  deletePlaylist: async () => {},
  renamePlaylist: async () => {},
  reorderPlaylists: async () => {},
  addToPlaylist: async () => {},
  removeFromPlaylist: async () => {},
  playlistTracks: async () => songs.slice(0, 8),
  getSettings: async () => ({
    discordEnabled: true,
    discordClientId: '',
    discordLargeImageKey: 'lumen',
    discordCoverArt: true
  }),
  setSettings: async (p) => ({
    discordEnabled: true,
    discordClientId: '',
    discordLargeImageKey: 'lumen',
    discordCoverArt: true,
    ...p
  }),
  setPresence: async () => {},
  mediaUrl: (id) => `lumen-media://t/${id}`,
  artUrl: (id) => `lumen-art://t/${id}`,
  albumArtUrl: (album) => `lumen-art://t/${albumId(album)}`,
  setAlbumCoverFromTrack: async () => {},
  setAlbumCoverCustom: async () => false,
  clearAlbumCover: async () => {},
  albumHasCover: async () => true,
  recordPlay: async () => {},
  recordPlaySync: () => {},
  playHistory: async () => [],
  clearPlayHistory: async () => {}
}

contextBridge.exposeInMainWorld('lumen', api)
contextBridge.exposeInMainWorld('__lumenVizDemo', true)
