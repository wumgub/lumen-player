# Browsing and search

## Views

The top of the sidebar switches between the main views:

| View | Shows |
|---|---|
| **Songs** | Every song in your library |
| **Artists** | A card for every artist (grouped by album artist when a song has one) with a song count. Click a card to see that artist's songs. |
| **Albums** | A cover for every album. Click one to open it. Songs with no album aren't listed here. |
| **Stats** | Your [listening stats](stats.md) |

Clicking a playlist or folder in the sidebar shows just those songs.

## Song lists

Every list of songs (all songs, an artist, an album, a folder, a playlist) works the same way:

- **Click a song** to play it. The whole list you're looking at becomes the
  [queue](playback.md#the-queue), starting from that song.
- **Play all** (top right) plays the list from the top.
- **Right-click a song**, or click its **⋯** button, for the song menu:

  | Menu item | Does |
  |---|---|
  | Play next | Puts it straight after the current song in the queue |
  | Add to queue | Puts it at the end of the queue |
  | Rename song… | Changes the title Lumen shows ([Editing](editing.md#renaming)) |
  | Add note… / Edit note… | Attaches a searchable note ([below](#notes)) |
  | Show file | Opens Finder / Explorer / your file manager at the file |
  | Add to album… | Moves it onto an album, existing or new ([Editing](editing.md#making-an-album)) |
  | Remove from album | Takes it off its album (only shown if it's on one) |
  | Remove from playlist | Only shown inside a playlist |
  | Add to playlist | Your playlists, one click to add |

- The song that's playing is highlighted, with a little animated equaliser in place of its
  number.

### Edit mode and selecting several songs

Click **Edit** at the top of a list to change what clicks do:

- **In most lists**, clicking a song ticks it. Once anything is ticked, a bar appears at the
  bottom with **Add to playlist…**, **Add to queue**, **Add to album…** and **Clear**.
- **In an album sorted by Track order**, Edit lets you drag songs into a new order instead
  ([Track order](editing.md#track-order)).

Click **Done** to leave edit mode. Switching to another view also leaves it.

## Sorting

The **Sort** control at the top of a list picks the order. The arrow next to it flips between
ascending and descending.

| Sort | Notes |
|---|---|
| Title | Default for Songs and folders |
| Artist | |
| Album | Default for an artist's page |
| Date added | When Lumen first found the file |
| Duration | |
| Track order | Albums only, and their default. Your [custom order](editing.md#track-order) if you've set one, otherwise disc then track number. |

Ties are broken by album, then disc number, then track number, then title, so an album's songs
stay together in a sensible order.

Playlists have no sort control. They always play in the order songs were added.

Each view starts with its default sort when you open it. Lumen doesn't remember a sort you
picked last time.

## Search

Click the **magnifying glass** in the title bar and type. Search looks through every song's
**title, artist, album and note** for the text you typed, anywhere inside them (so `sun` finds
*Звезда по имени Солнце*'s note "a star called sun").

- Typing jumps you to the **Songs** view to show the results.
- Press **Esc** in the search box to clear it.
- The box stays open while there's text in it, and shrinks back to the icon when it's empty.

> [!NOTE]
> Matching ignores upper/lower case **only for plain English letters (A–Z)**. For other
> alphabets case matters, so `кино` won't find `Кино`. Type it with the right
> capitals, or add a [note](#notes) with a romanised version you can type easily.

## Notes

A note is a bit of free text you attach to a song. It shows in small text under the song's
artist and, more importantly, **search matches it**. That makes notes handy for:

- songs in a script you can't easily type: add a romanised title (`pachka sigaret`)
- translations, so you can search in your own language
- anything else you want to find a song by: "the one from that film", "wedding", etc.

Right-click a song → **Add note…**. Press **⌘ Enter** (Ctrl Enter on Windows/Linux) or click
**Save**. Save it empty to remove the note. Notes live only in Lumen and are never written to the
file.
