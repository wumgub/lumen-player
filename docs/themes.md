# Themes

Change theme from the **theme picker** in the title bar, or under **⚙ Settings → Appearance**.
The change applies instantly and is remembered.

## Built-in themes

| Theme | Feel |
|---|---|
| **Neon** | Dark, with cyan and magenta glow (the default) |
| **Flashbang** | Blinding light mode |
| **Minimal Dark** | Clean, muted, dark |
| **Retro** | Green-on-black, monospace, Winamp-ish |
| **Glass** | Frosted panels, with accents that take their colour from the album art |
| **Custom** | Your own, made in the [theme editor](#the-theme-editor) |

## Tinting from album art

The **Glass** theme picks two strong colours out of the current song's cover art and uses them
for its accents and the visualizer. They change with every song. The **Tint accents from album
art** checkbox at the bottom of the theme picker menu turns this on and off. With it off, or when
a song has no art, Glass uses its own light blue and pink. The other themes keep their fixed
colours whatever this is set to.

## The theme editor

Open it from **⚙ Settings → Appearance**: click the ✏️ on the **Custom** chip. That switches you to
the Custom theme and opens the editor. Every change previews live and saves as you go. **Done**
closes it.

**Start from** copies one of the built-in themes into Custom as a starting point. This replaces
whatever Custom had, so export it first if you want to keep it.

You can change:

| Group | Colours |
|---|---|
| Backgrounds | App background · Panels & bars · Hover & elevated · Menus & modals |
| Text | Text · Muted text |
| Accents | Primary accent · Secondary accent |
| Lines | Borders |
| Visualizer | Background · Foreground · Accent · Accent 2 |
| Typography | Font family |

For each colour you can:

- click the swatch for a colour picker
- type any CSS colour in the box: `#ff3366`, `rgb(255 51 102)`, `rgba(…)`, `hotpink`…
- click the 💧 **eyedropper** and pick a colour from anywhere on your screen

**Font** has presets (System, Helvetica, Rounded, Serif, Monospace, Condensed), or you can type a
CSS font stack such as `"Inter", sans-serif`. The font has to be installed on your computer.

A few things are worked out for you: text on accent-coloured buttons turns black or white
depending on what's readable on your accent, and light or dark scrollbars follow your
background colour.

There's one Custom theme at a time. To keep several, export each one.

## Sharing themes

At the bottom of the editor:

- **Export** shows your theme as text. **Copy to clipboard**, or **Save .txt**.
- **Import**: paste a theme or **Load .txt…**, then **Apply theme**. This replaces your Custom
  theme.

Exported themes look like this:

```json
{
  "lumenTheme": 1,
  "tokens": {
    "bg": "#0a0a12",
    "bg-2": "#11111d",
    "bg-3": "#1a1a2b",
    "popover-bg": "#11111d",
    "fg": "#f0f0ff",
    "fg-dim": "#8a8aa8",
    "accent": "#22d3ee",
    "accent-2": "#c026d3",
    "border": "#23233a",
    "viz-bg": "#0a0a12",
    "viz-fg": "#ffffff",
    "viz-accent": "#22d3ee",
    "viz-accent-2": "#e879f9",
    "font": "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif"
  }
}
```

Import is forgiving. It accepts text wrapped in a Markdown code block (as chat apps tend to do),
a bare `{ "bg": …, "accent": … }` without the wrapper, or a theme with some colours missing.
Missing colours are filled in from Neon, and unknown keys are ignored.
