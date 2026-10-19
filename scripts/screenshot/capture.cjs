// Captures screenshots of the real built renderer with a mock data layer.
// Usage: electron scripts/screenshot/capture.cjs
const { app, BrowserWindow, protocol } = require('electron')
const path = require('path')
const fs = require('fs')

const ROOT = path.join(__dirname, '..', '..')
const INDEX = path.join(ROOT, 'out', 'renderer', 'index.html')
const PRELOAD = path.join(__dirname, 'mock-preload.cjs')
const OUT = path.join(ROOT, 'docs', 'screenshots')

const wait = (ms) => new Promise((r) => setTimeout(r, ms))

protocol.registerSchemesAsPrivileged([
  { scheme: 'lumen-art', privileges: { supportFetchAPI: true, bypassCSP: true } },
  { scheme: 'lumen-media', privileges: { supportFetchAPI: true, bypassCSP: true } }
])

function coverSvg(id) {
  const hue = (id * 47) % 360
  const h2 = (hue + 40) % 360
  return `<svg xmlns='http://www.w3.org/2000/svg' width='300' height='300'>
    <defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'>
      <stop offset='0' stop-color='hsl(${hue},55%,48%)'/>
      <stop offset='1' stop-color='hsl(${h2},60%,28%)'/></linearGradient></defs>
    <rect width='300' height='300' fill='url(#g)'/>
    <circle cx='${70 + ((id * 37) % 160)}' cy='${80 + ((id * 53) % 140)}' r='78' fill='rgba(255,255,255,0.14)'/>
    <circle cx='235' cy='225' r='95' fill='rgba(0,0,0,0.18)'/>
  </svg>`
}

async function shot(win, { theme, viz, steps, file }) {
  const wc = win.webContents
  await win.loadFile(INDEX)
  await wait(500)
  await wc.executeJavaScript(
    `localStorage.setItem('lumen.theme', ${JSON.stringify(theme)});` +
      `localStorage.setItem('lumen.viz', ${JSON.stringify(viz || 'bars')});` +
      `localStorage.setItem('lumen.volume','0.8'); true;`
  )
  await wc.reload()
  await wait(1000)
  // Belt-and-suspenders: ensure the demo visualizer flag is live in the main world.
  await wc.executeJavaScript('window.__lumenVizDemo = true').catch(() => {})
  for (const step of steps || []) {
    await wc.executeJavaScript(step).catch(() => {})
    await wait(500)
  }
  await wait(700) // let the visualizer animate into a nice frame
  const img = await wc.capturePage()
  fs.writeFileSync(path.join(OUT, file), img.toPNG())
  console.log('wrote', file, img.getSize())
}

app.whenReady().then(async () => {
  fs.mkdirSync(OUT, { recursive: true })

  protocol.handle('lumen-art', async (req) => {
    const id = Number(new URL(req.url).pathname.replace(/^\//, '') || new URL(req.url).hostname) || 1
    return new Response(coverSvg(id), { headers: { 'Content-Type': 'image/svg+xml' } })
  })
  protocol.handle('lumen-media', async () => new Response(null, { status: 204 }))

  const win = new BrowserWindow({
    width: 1360,
    height: 880,
    show: false,
    backgroundColor: '#0a0a12',
    webPreferences: {
      preload: PRELOAD,
      contextIsolation: true,
      sandbox: false,
      autoplayPolicy: 'no-user-gesture-required'
    }
  })

  // Warm up the compositor so the first real shot's canvas is sized correctly.
  await win.loadFile(INDEX)
  await wait(1200)

  const clickTrack = `document.querySelector('.track-row') && document.querySelector('.track-row').click()`
  const goAlbums = `[...document.querySelectorAll('.nav-item')][2].click()`
  const openAlbum = `document.querySelector('.album-card') && document.querySelector('.album-card').click()`

  await shot(win, { theme: 'neon', viz: 'bars', steps: [clickTrack], file: 'library.png' })
  await shot(win, { theme: 'glass', viz: 'radial', steps: [clickTrack, goAlbums], file: 'albums.png' })
  await shot(win, {
    theme: 'minimal-dark',
    viz: 'waveform',
    steps: [clickTrack, goAlbums, openAlbum],
    file: 'album.png'
  })

  win.destroy()
  app.quit()
})
