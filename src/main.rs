extern crate env_logger;
extern crate librespot_audio;
extern crate librespot_core;
extern crate librespot_metadata;
#[macro_use]
extern crate log;
extern crate regex;
extern crate tokio;

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::process;
use std::process::Command;

use env_logger::{Builder, Env};
use librespot_audio::{AudioDecrypt, AudioFile};
use librespot_core::authentication::Credentials;
use librespot_core::config::SessionConfig;
use librespot_core::session::Session;
use librespot_core::spotify_id::SpotifyId;
use librespot_metadata::{Album, Artist, Episode, FileFormat, Metadata, Playlist, Show, Track};
use regex::Regex;
use tokio::runtime::Runtime;

fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let args: Vec<_> = env::args().collect();

    maybe_info_and_exit(&args);

    let input_reader = get_file_reader(&args[3].to_owned());

    let runtime = get_runtime();
    let session = get_session(&runtime, args[1].to_owned(), args[2].to_owned());

    let (track_id_list, episode_id_list) = url_uri_to_track_and_episode_id_list(&runtime, &session, input_reader);

    track_id_list.iter().for_each(|id| {
        match download_track(&runtime, &session, *id) {
            Ok(value) => value,
            Err(message) => warn!("{}", message),
        };
    });
    episode_id_list.iter().for_each(|id| {
        match download_episode(&runtime, &session, *id) {
            Ok(value) => value,
            Err(message) => warn!("{}", message),
        };
    });
}

fn maybe_info_and_exit(args: &Vec<String>) {
    if args.len() != 4 {
        info!("Usage: oggify USER PASSWORD TRACK_FILE");
        process::exit(0);
    }
}

fn get_runtime() -> Runtime {
    Runtime::new().unwrap()
}

fn get_session(runtime: &Runtime, user: String, password: String) -> Session {
    let session_config = SessionConfig::default();
    let credentials = Credentials::with_password(user, password);
    info!("Connecting ...");
    let session = runtime
        .block_on(Session::connect(session_config, credentials, None))
        .unwrap();
    info!("Connected!");
    session
}

fn get_file_reader(file_name: &String) -> BufReader<std::fs::File> {
    let some_file = File::open(file_name);
    match some_file {
        Ok(file) => BufReader::new(file),
        Err(_) => {
            info!("File {} not found.", file_name);
            process::exit(0);
        }
    }
}

fn url_uri_to_track_and_episode_id_list(
    runtime: &Runtime,
    session: &Session,
    input_reader: BufReader<std::fs::File>,
) -> (Vec<SpotifyId>, Vec<SpotifyId>) {
    let spotify_track_uri = Regex::new(r"spotify:track:([[:alnum:]]+)").unwrap();
    let spotify_track_url = Regex::new(r"open\.spotify\.com/track/([[:alnum:]]+)").unwrap();
    let spotify_album_uri = Regex::new(r"spotify:album:([[:alnum:]]+)").unwrap();
    let spotify_album_url = Regex::new(r"open\.spotify\.com/album/([[:alnum:]]+)").unwrap();
    let spotify_playlist_uri = Regex::new(r"spotify:playlist:([[:alnum:]]+)").unwrap();
    let spotify_playlist_url = Regex::new(r"open\.spotify\.com/playlist/([[:alnum:]]+)").unwrap();
    let spotify_episode_uri = Regex::new(r"spotify:episode:([[:alnum:]]+)").unwrap();
    let spotify_episode_url = Regex::new(r"open\.spotify\.com/episode/([[:alnum:]]+)").unwrap();
    let spotify_show_uri = Regex::new(r"spotify:show:([[:alnum:]]+)").unwrap();
    let spotify_show_url = Regex::new(r"open\.spotify\.com/show/([[:alnum:]]+)").unwrap();

    let mut track_id_list: Vec<SpotifyId> = vec![];
    let mut episode_id_list: Vec<SpotifyId> = vec![];

    for (_index, line) in input_reader.lines().enumerate() {
        let line = line.unwrap();

        if let Some(captures) = spotify_track_url.captures(&line) {
            track_id_list.push(SpotifyId::from_base62(&captures[1]).ok().unwrap());
        } else if let Some(captures) = spotify_track_uri.captures(&line) {
            track_id_list.push(SpotifyId::from_base62(&captures[1]).ok().unwrap());
        } else if let Some(captures) = spotify_album_url.captures(&line) {
            let album_id: SpotifyId = SpotifyId::from_base62(&captures[1]).ok().unwrap();
            let album = runtime
                .block_on(Album::get(&session, album_id))
                .expect("Cannot get album metadata.");
            for track_id in album.tracks.into_iter() {
                track_id_list.push(track_id)
            }
        } else if let Some(captures) = spotify_album_uri.captures(&line) {
            let album_id: SpotifyId = SpotifyId::from_base62(&captures[1]).ok().unwrap();
            let album = runtime
                .block_on(Album::get(&session, album_id))
                .expect("Cannot get album metadata.");
            for track_id in album.tracks.into_iter() {
                track_id_list.push(track_id)
            }
        } else if let Some(captures) = spotify_playlist_url.captures(&line) {
            let playlist_id: SpotifyId = SpotifyId::from_base62(&captures[1]).ok().unwrap();
            let playlist = runtime
                .block_on(Playlist::get(&session, playlist_id))
                .expect("Cannot get playlist metadata.");
            for track_id in playlist.tracks.into_iter() {
                track_id_list.push(track_id)
            }
        } else if let Some(captures) = spotify_playlist_uri.captures(&line) {
            let playlist_id: SpotifyId = SpotifyId::from_base62(&captures[1]).ok().unwrap();
            let playlist = runtime
                .block_on(Playlist::get(&session, playlist_id))
                .expect("Cannot get playlist metadata.");
            for track_id in playlist.tracks.into_iter() {
                track_id_list.push(track_id)
            }
        } else if let Some(captures) = spotify_episode_url.captures(&line) {
            episode_id_list.push(SpotifyId::from_base62(&captures[1]).ok().unwrap());
        } else if let Some(captures) = spotify_episode_uri.captures(&line) {
            episode_id_list.push(SpotifyId::from_base62(&captures[1]).ok().unwrap());
        } else if let Some(captures) = spotify_show_url.captures(&line) {
            let show_id: SpotifyId = SpotifyId::from_base62(&captures[1]).ok().unwrap();
            let show = runtime
                .block_on(Show::get(&session, show_id))
                .expect("Cannot get show metadata.");
            for track_id in show.episodes.into_iter() {
                episode_id_list.push(track_id)
            }
        } else if let Some(captures) = spotify_show_uri.captures(&line) {
            let show_id: SpotifyId = SpotifyId::from_base62(&captures[1]).ok().unwrap();
            let show = runtime
                .block_on(Show::get(&session, show_id))
                .expect("Cannot get show metadata.");
            for track_id in show.episodes.into_iter() {
                episode_id_list.push(track_id)
            }
        } else {
            warn!("Line \"{}\" is not a valid URL/URI.", line);
        }
    }
    (track_id_list, episode_id_list)
}

fn download_track(runtime: &Runtime, session: &Session, id: SpotifyId) -> Result<(), String> {
    info!("Getting track {}...", id.to_base62());
    let mut track = runtime
        .block_on(Track::get(&session, id))
        .expect("Cannot get track metadata");
    if !track.available {
        info!(
            "Track {} is not available, finding alternative...",
            id.to_base62()
        );
        let alt_track = track.alternatives.iter().find_map(|id| {
            let alt_track = runtime
                .block_on(Track::get(&session, *id))
                .expect("Cannot get track metadata");
            match alt_track.available {
                true => Some(alt_track),
                false => None,
            }
        });
        track = match alt_track {
            Some(alt_track) => alt_track,
            None => {
                return Err(format!(
                    "Could not find alternative for track {}",
                    id.to_base62()
                ))
            }
        };
        info!(
            "Found track alternative {} -> {}",
            id.to_base62(),
            track.id.to_base62()
        );
    }
    let artists_strs: Vec<_> = track
        .artists
        .iter()
        .map(|id| {
            runtime
                .block_on(Artist::get(&session, *id))
                .expect("Cannot get artist metadata")
                .name
        })
        .collect();
    debug!(
        "File formats: {}",
        track
            .files
            .keys()
            .map(|filetype| format!("{:?}", filetype))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let ok_artists_name = remove_restricted_file_name_chars(&artists_strs.join(", "));
    let ok_track_name = remove_restricted_file_name_chars(&track.name);
    let file_name = format!("{} - {}.ogg", ok_artists_name, ok_track_name);
    if std::path::Path::new(&file_name).exists() {
        warn!("File \"{}\" already exists, download skipped.", file_name);
    } else {
        let file_id = track
            .files
            .get(&FileFormat::OGG_VORBIS_320)
            // .or(track.files.get(&FileFormat::OGG_VORBIS_160))
            // .or(track.files.get(&FileFormat::OGG_VORBIS_96))
            .expect("Could not find a OGG_VORBIS_320 format for the track.");
        let key = runtime
            .block_on(session.audio_key().request(track.id, *file_id))
            .expect("Cannot get audio key");
        let mut encrypted_file = runtime
            .block_on(AudioFile::open(&session, *file_id, 320, true))
            .unwrap();
        let mut buffer = Vec::new();
        let _read_all = encrypted_file.read_to_end(&mut buffer);
        let mut decrypted_buffer = Vec::new();
        AudioDecrypt::new(key, &buffer[..])
            .read_to_end(&mut decrypted_buffer)
            .expect("Cannot decrypt stream");
        std::fs::write(&file_name, &decrypted_buffer[0xa7..])
            .expect("Cannot write decrypted track");
        info!("Filename: {}", file_name);
        let album = runtime
            .block_on(Album::get(&session, track.album))
            .expect("Cannot get album metadata");
        tag_file(file_name, track.name, album.name, artists_strs.join(", "), id.to_base62());
    }
    Ok(())
}

fn download_episode(runtime: &Runtime, session: &Session, id: SpotifyId) -> Result<(), String> {
    info!("Getting episode {}...", id.to_base62());
    let episode = runtime
        .block_on(Episode::get(&session, id))
        .expect("Cannot get episode metadata");
//    if !episode.available {
//        warn!("Episode {} is not available.", id.to_base62());
//    }
    let show = runtime
        .block_on(Show::get(&session, episode.show))
        .expect("Cannot get show metadata");
    let artists_str = show.name;
    debug!(
        "File formats: {}",
        episode
            .files
            .keys()
            .map(|filetype| format!("{:?}", filetype))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let ok_artists_name = remove_restricted_file_name_chars(&artists_str);
    let ok_track_name = remove_restricted_file_name_chars(&episode.name);
    let file_name = format!("{} - {}.ogg", ok_artists_name, ok_track_name);
    if std::path::Path::new(&file_name).exists() {
        warn!("File \"{}\" already exists, download skipped.", file_name);
    } else {
        let file_id = episode
            .files
            .get(&FileFormat::OGG_VORBIS_160)
            .or(episode.files.get(&FileFormat::OGG_VORBIS_96))
            .or(episode.files.get(&FileFormat::OGG_VORBIS_320))
            .expect("Could not find a OGG_VORBIS format for the episode.");
        let key = runtime
            .block_on(session.audio_key().request(episode.id, *file_id))
            .expect("Cannot get audio key");
        let mut encrypted_file = runtime
            .block_on(AudioFile::open(&session, *file_id, 320, true))
            .unwrap();
        let mut buffer = Vec::new();
        let _read_all = encrypted_file.read_to_end(&mut buffer);
        let mut decrypted_buffer = Vec::new();
        AudioDecrypt::new(key, &buffer[..])
            .read_to_end(&mut decrypted_buffer)
            .expect("Cannot decrypt stream");
        std::fs::write(&file_name, &decrypted_buffer[0xa7..])
            .expect("Cannot write decrypted track");
        info!("Filename: {}", file_name);
        tag_file(file_name, episode.name, show.name, artists_str, id.to_base62());
    }
    Ok(())
}

fn tag_file(file_name: String, title: String, album: String, artists: String, spotify_id: String) {
    Command::new("vorbiscomment")
        .arg("--append")
        .args(["--tag", &format!("TITLE={}", title)])
        .args(["--tag", &format!("ALBUM={}", album)])
        .args(["--tag", &format!("ARTIST={}", artists)])
        .args(["--tag", &format!("SPOTIFY_ID={}", spotify_id)])
        .arg(file_name)
        .spawn()
        .expect("Failed to tag file with vorbiscomment.");
}

fn remove_restricted_file_name_chars(name: &String) -> String {
    Regex::new(r#"[\?\\\*":/<>|]"#)
        .unwrap()
        .replace_all(name.as_str(), "_")
        .into_owned()
}
