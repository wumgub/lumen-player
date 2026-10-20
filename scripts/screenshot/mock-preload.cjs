// Mock window.lumen for screenshot captures of the real renderer.
const { contextBridge } = require('electron')

const mk = (id, title, artist, album, year, trackNo, duration, note = '') => ({
  id,
  path: `/Music/${artist}/${album}/${title}.mp3`,
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
  note
})

const songs = [
  mk(1, 'Судно', 'Molchat Doma', 'Этажи', 2018, 1, 171, 'sudno — molchat doma'),
  mk(2, 'Танцевать', 'Molchat Doma', 'Этажи', 2018, 2, 268),
  mk(3, 'Клетка', 'Molchat Doma', 'Этажи', 2018, 3, 214),
  mk(4, 'Люди надоели', 'Molchat Doma', 'Этажи', 2018, 4, 199),
  mk(5, 'Волны', 'Molchat Doma', 'Монумент', 2020, 1, 236),
  mk(6, 'Дискотека', 'Molchat Doma', 'Монумент', 2020, 2, 227),
  mk(7, 'Звезды', 'Molchat Doma', 'Монумент', 2020, 3, 258),
  mk(8, 'Age of Consent', 'New Order', 'Power, Corruption & Lies', 1983, 1, 316),
  mk(9, 'The Village', 'New Order', 'Power, Corruption & Lies', 1983, 2, 271),
  mk(10, 'Your Silent Face', 'New Order', 'Power, Corruption & Lies', 1983, 3, 359),
  mk(11, 'Enjoy the Silence', 'Depeche Mode', 'Violator', 1990, 1, 372),
  mk(12, 'Personal Jesus', 'Depeche Mode', 'Violator', 1990, 2, 296),
  mk(13, 'Policy of Truth', 'Depeche Mode', 'Violator', 1990, 3, 298),
  mk(14, 'Shadowplay', 'Joy Division', 'Unknown Pleasures', 1979, 1, 213),
  mk(15, 'Disorder', 'Joy Division', 'Unknown Pleasures', 1979, 2, 209),
  mk(16, 'New Dawn Fades', 'Joy Division', 'Unknown Pleasures', 1979, 3, 285)
]

const groupBy = (arr, key) => {
  const m = new Map()
  for (const s of arr) m.set(s[key], (m.get(s[key]) || 0) + 1)
  return m
}

const api = {
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
  artUrl: (id) => `lumen-art://t/${id}`
}

contextBridge.exposeInMainWorld('lumen', api)
contextBridge.exposeInMainWorld('__lumenVizDemo', true)
