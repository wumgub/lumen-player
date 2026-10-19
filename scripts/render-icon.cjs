// Renders build/icon.svg to a transparent PNG using Electron's Chromium.
// Usage: electron scripts/render-icon.cjs <size> <outPath>
const { app, BrowserWindow } = require('electron')
const fs = require('fs')
const path = require('path')

const SIZE = parseInt(process.argv[2] || '1024', 10)
const OUT = process.argv[3] || path.join(__dirname, '..', 'build', 'icon.png')

app.disableHardwareAcceleration()

app.whenReady().then(async () => {
  const win = new BrowserWindow({
    width: SIZE,
    height: SIZE,
    show: false,
    frame: false,
    transparent: true,
    backgroundColor: '#00000000',
    useContentSize: true,
    webPreferences: { offscreen: false }
  })

  const svg = fs.readFileSync(path.join(__dirname, '..', 'build', 'icon.svg'), 'utf8')
  const html = `<!doctype html><html><head><meta charset="utf-8"><style>
    html,body{margin:0;padding:0;width:${SIZE}px;height:${SIZE}px;background:transparent;overflow:hidden}
    svg{display:block;width:${SIZE}px;height:${SIZE}px}
  </style></head><body>${svg}</body></html>`

  await win.loadURL('data:text/html;charset=utf-8,' + encodeURIComponent(html))
  await new Promise((r) => setTimeout(r, 400))

  const img = await win.webContents.capturePage()
  fs.writeFileSync(OUT, img.toPNG())
  console.log(`wrote ${OUT} (${img.getSize().width}x${img.getSize().height})`)
  app.quit()
})
