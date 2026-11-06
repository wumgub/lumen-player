# Editing your library

You can fix names, build albums and change covers without touching your files. **Every edit
here only changes Lumen's own library**; the tags inside the music files stay as they were.
Other apps won't see your changes, and your edits in Lumen won't be undone by the files.

## Renaming

| Rename | How | Effect |
|---|---|---|
| A song | Right-click → **Rename song…** | Changes that one song's title |
| An artist | Open the artist and click the ✏️ next to their name, or hover their card in *Artists* and click ✏️ | Changes the name on **every** song by them |
| An album | Open the album and click the ✏️ next to its title, or hover its cover in *Albums* and click ✏️ | Changes the album name on **every** song on it |

In the rename box, **Enter** saves and **Esc** cancels. Saving the same name, or an empty one,
does nothing.

Some things to know:

- **Renaming an album or artist to a name that already exists merges them.** Rename the
  album *Etazhi* to *Этажи*, and if there's already an *Этажи*, the songs all end up together
  as one album. That's an easy way to merge an album that got split across two spellings.
- Renaming an artist changes the album artist where a song has one, and the plain artist where
  it doesn't. That's the same name the *Artists* view groups by.
- **A custom cover follows the album when you rename it.** If you rename into an album that
  already has its own custom cover (a merge), that album's cover is kept.

## Making an album

Songs that came without an album tag (or with the wrong one) can be put on an album by hand.
That album can be one that already exists or a brand-new one.

**One song**: right-click it → **Add to album…**.

**Several songs at once**:

1. Find the songs. *Songs* with a search, or the folder they're in, works well.
2. Click **Edit** at the top of the list and tick each song.
3. Click **Add to album…** in the bar at the bottom.

In the box that opens, start typing:

- pick an existing album from the list to add the songs to it, or
- click **Create album "…"** (or press **Enter**) to make a new album with exactly that name.

The album shows up in *Albums* right away.

**Taking a song off an album**: right-click → **Remove from album**. The song goes back to having
no album, and it stays that way after a rescan even if the file has an album tag.

## Track order

An album plays in disc-then-track-number order by default. To set your own order:

1. Open the album.
2. Click **Edit**. The sort switches to *Track order* if it wasn't already.
3. Drag songs by the **⠿** handle to where you want them.
4. Click **Done**.

The new order is saved immediately and is used whenever the album is sorted by *Track order*
(which is the default). It's also what decides the album's [default cover](#album-covers).

Dragging only works when the album is sorted by *Track order* and there's no search text. A
song added to the album later goes to the end. Moving a song off the album and back again resets
its position.

## Album covers

By default an album's cover is the embedded art of its first song (in track order) that has
any. If no song on the album has art, you get a colourful placeholder with a disc icon.

To change it, hover the cover (in *Albums*, or at the top of the album's page) and click the
small 🖼 button. You get three choices:

| Choice | Does |
|---|---|
| **Custom image…** | Pick an image file (JPG, PNG, WebP, GIF or BMP). A copy is saved inside Lumen's library, so moving or deleting the original afterwards is fine. |
| **Choose from a song…** | Use the art from any song in your library. Search by title, artist or album. Handy when only one song on an album has art, or for a single's art on an album. |
| **Reset to default** | Go back to the first song's art |

The album cover is what Lumen shows in *Albums* and at the top of the album's page. Song rows, the
Now Playing panel and the transport bar still show each song's **own** embedded art. The Glass
theme's tinting and the OS "Now Playing" widget use the song's art too.

## What survives a rescan

Lumen remembers which fields you've changed and never overwrites them from the file's tags:

| Edit | Survives a rescan? |
|---|---|
| Song title | ✅ |
| Artist name | ✅ |
| Album (renaming it, adding songs to it, removing songs from it) | ✅ |
| Track order | ✅ |
| Album cover | ✅ (and it moves with the album if you rename it) |
| Notes | ✅ (scans never touch notes) |

Fields you haven't edited keep updating from the file. If you retag the genre or year in
another app, the next rescan picks it up.

**None of this survives the file being moved or renamed on disk.** Lumen tracks songs by their
path, so a moved file comes back as a new song with its original tags
([more](library.md#moving-and-renaming-files)).

## Undoing edits

There's no undo button and no "reset to file tags" option yet. To undo a rename, rename it
back. To get the file's own tags back for everything, [start the library over](settings-and-data.md#starting-over).
That also loses notes, playlists and stats, though.
