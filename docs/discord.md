# Discord Rich Presence

Lumen can show what you're listening to on your Discord profile. It appears as **Listening to
Lumen**, with the song title, "artist • album", and a progress bar that moves along by itself.

It's off by default. Setting it up takes a few minutes, once.

## Setting it up

### 1. Make a Discord application

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications) and sign in.
2. Click **New Application** and give it a name. **The name is what your profile shows after
   "Listening to"**, so `Lumen` is a good choice.
3. On the app's **General Information** page, copy the **Application ID**.

### 2. Add the artwork (recommended)

Still in the portal, open **Rich Presence → Art Assets** and upload:

| Asset name | Image | Used for |
|---|---|---|
| `lumen` | [`build/discord/lumen.png`](../build/discord/lumen.png), or any square image you like | The big picture next to the song |
| `play` (optional) | a small play icon | The little badge while playing |
| `pause` (optional) | a small pause icon | The little badge while paused |

The names have to match exactly. Without them, Discord just leaves those spots empty. Newly
uploaded assets can take a few minutes to start showing.

### 3. Turn it on in Lumen

1. Open **⚙ Settings → Discord Rich Presence**.
2. Flip the switch on and paste the Application ID into **Application Client ID**.
3. Click **Save**. This one needs Save; Cancel throws the change away.

Play something and check your profile.

## What shows

| Discord shows | From Lumen |
|---|---|
| Listening to *your app's name* | |
| First line | Song title |
| Second line | Artist • Album (just the artist if there's no album) |
| Big picture | Your `lumen` asset. Hover it for the album name. |
| Small badge | `play` or `pause` asset |
| Progress bar | Song position and length (only while playing) |

Presence updates when the song changes, when you play or pause, and when you seek. It isn't
updated every second; Discord moves the bar along itself. This keeps Lumen well inside Discord's
rate limits. When nothing is loaded, presence is cleared.

## Things that stop it showing

- **The Discord desktop app has to be running** on the same computer. Discord in a web browser
  can't receive presence.
- **Your status can't be Invisible.** Discord hides all activity while you're invisible.
- **Activity sharing must be on**: Discord **User Settings → Activity Privacy → Share your
  detected activities with others**.
- **You often can't see it on your own profile.** Ask a friend, or check from another account.

If Discord is closed, restarted or updated while Lumen is running, Lumen notices and reconnects
by itself. It first retries after 5 seconds, then waits a bit longer each time, up to once a
minute. Presence is usually back within a few seconds of Discord coming back, without you doing
anything.

## Per-song cover art

Lumen can look up each song's album cover online (through the iTunes Search API) and show that
instead of the `lumen` picture. To do that it needs to sign in to your Discord application,
which requires the app's **Client Secret**. The current Settings screen doesn't have a field
for the secret, so in practice presence always uses your `lumen` asset. If per-song covers ever
fail, Lumen falls back to the logo quietly; nothing breaks.

## Privacy

With presence on, the song title, artist and album you're playing are visible to anyone who can
see your Discord activity. Turn it off in Settings (and Save) whenever you like. The presence
clears straight away.
