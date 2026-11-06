#!/bin/bash
# Retake the README screenshots (docs/screenshots/) from the real app.
#
#   scripts/screenshots.sh                 a made-up library (generated tones
#                                          with tags and gradient covers)
#   scripts/screenshots.sh <library.db>    a real library. It works on a copy,
#                                          leaves out songs with no cover, and
#                                          shows the three biggest albums.
#
# Needs sqlite3 (and ffmpeg for the made-up library). macOS only (sips).
set -euo pipefail
cd "$(dirname "$0")/.."
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
MUSIC="$WORK/music"
DB="$WORK/library.db"

shot() { # out theme viz view play seek
  sqlite3 "$DB" "INSERT OR REPLACE INTO settings (key, value) VALUES ('uiPrefs',
    json_object('theme','$2','viz','$3','vizOn',json('true'),'adaptive',json('true'),'volume',0.9))"
  target/release/lumen --db "$DB" --ao null --view "$4" --play "$5" --seek "$6" \
    --screenshot "$WORK/$1" --frames 150
  # Keep the files small: 1600 px wide is plenty for the README.
  sips -Z 1600 "$WORK/$1" --out "docs/screenshots/$1" > /dev/null
}

if [ $# -ge 1 ]; then
  sqlite3 -readonly "$1" ".backup '$DB'"
  sqlite3 "$DB" "PRAGMA foreign_keys = ON;
    DELETE FROM tracks WHERE hasArt = 0 AND NOT EXISTS
      (SELECT 1 FROM album_covers c WHERE c.album = tracks.album AND tracks.album != '');"
  # The biggest albums, and a song from the middle of each.
  albums=$(sqlite3 "$DB" "SELECT album FROM tracks WHERE album != '' AND isVideo = 0
    GROUP BY album ORDER BY COUNT(*) DESC LIMIT 3")
  IFS=$'\n' read -r -d '' a1 a2 a3 <<<"$albums" || true
  pick() { sqlite3 "$DB" "SELECT title FROM tracks WHERE album = '${1//\'/\'\'}' AND isVideo = 0
    ORDER BY discNo, trackNo LIMIT 1 OFFSET 1"; }
  cargo build -q --release -p lumen
  shot library.png neon bars songs "$(pick "$a1")" 60
  shot albums.png glass radial albums "$(pick "$a2")" 60
  shot album.png minimal-dark waveform "album:$a3" "$(pick "$a3")" 45
  echo "Saved to docs/screenshots/"
  exit 0
fi

# artist | album | cover colours (r,g,b:r,g,b) | titles
ALBUMS=(
  "Night Static|Concrete Bloom|43,16,85:213,58,157|Glass Avenue;Tram Lights;Concrete Bloom;Late Shift;Signal Lost"
  "Velvet Antenna|Signal Fade|15,32,39:44,156,156|Antenna;Static Hearts;Slow Dial;Fade Out"
  "Moth Parade|Low Season|58,28,28:242,160,61|Low Season;Porch Light;Summer Ends;Moth"
  "Северный ветер|Ночные окна|11,26,58:106,140,255|Ночные окна;Дождь;Последний трамвай"
)
for entry in "${ALBUMS[@]}"; do
  IFS='|' read -r artist album colours titles <<<"$entry"
  dir="$MUSIC/$artist/$album"
  mkdir -p "$dir"
  # A diagonal gradient between the two colours.
  IFS=',' read -r r1 g1 b1 <<<"${colours%%:*}"
  IFS=',' read -r r2 g2 b2 <<<"${colours##*:}"
  ffmpeg -loglevel error -f lavfi -i "color=black:s=600x600,format=rgb24" -frames:v 1 \
    -vf "geq=r='$r1+($r2-$r1)*(X+Y)/1200':g='$g1+($g2-$g1)*(X+Y)/1200':b='$b1+($b2-$b1)*(X+Y)/1200'" "$dir/cover.jpg"
  n=0
  IFS=';' read -ra list <<<"$titles"
  for title in "${list[@]}"; do
    n=$((n + 1))
    # A bass pulse, a ticking high note and a little noise, so the
    # visualizers have something to move to.
    f=$((40 + n * 7))
    len=$((150 + (n * 37) % 110))
    ffmpeg -loglevel error -f lavfi \
      -i "aevalsrc='(0.6*sin(2*PI*$f*t)+0.3*sin(4*PI*$f*t))*exp(-6*mod(t,0.5))+0.15*sin(2*PI*$((f * 5))*t)+0.1*sin(2*PI*$((f * 7))*t)*(0.5+0.5*sin(PI*t))+0.06*sin(2*PI*$((f * 16))*t)*lt(mod(t,0.25),0.03)+0.004*(random(0)*2-1)':s=22050:d=$len" \
      -i "$dir/cover.jpg" -map 0 -map 1 -c:a libmp3lame -b:a 32k -c:v copy -id3v2_version 3 \
      -metadata title="$title" -metadata artist="$artist" -metadata album="$album" -metadata track="$n" \
      -metadata date=2024 -metadata:s:v comment="Cover (front)" "$dir/$(printf %02d "$n") $title.mp3"
  done
  rm "$dir/cover.jpg"
done
cargo run -q --release -p lumen-core --example scan_folder -- "$DB" "$MUSIC"
cargo build -q --release -p lumen

shot library.png neon bars songs "Tram Lights" 40
shot albums.png glass radial albums "Ночные окна" 60
shot album.png minimal-dark waveform "album:Low Season" "Porch Light" 30
echo "Saved to docs/screenshots/"
