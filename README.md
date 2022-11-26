# oggify

Download Spotify tracks as Ogg Vorbis with a premium account.

This library uses [librespot](https://github.com/librespot-org/librespot).

The code is kind of ugly, it has been written by Rust newbies only 😁

## Usage

To download tracks as `"artists" - "title".ogg`, run:

```
oggify "spotify-user" "spotify-password" urls_list_file
```

Oggify reads from urls_list_file and looks for a track/album/playlist URL or URI on each line. The two formats are those you get with the track menu items "Share->Copy Song Link" or "Share->Copy Song URI" in the Spotify client, for example `open.spotify.com/track/1xPQDRSXDN5QJWm7qHg5Ku` or `spotify:track:1xPQDRSXDN5QJWm7qHg5Ku`.

## Converting to MP3

If you need MP3s instead of Oggs files, this piece of shell script can help you:

```
for ogg in *.ogg; do
	ffmpeg -i "$ogg" -map_metadata 0:s:0 -id3v2_version 3 -codec:a libmp3lame -qscale:a 2 "$(basename "$ogg" .ogg).mp3"
done
```
