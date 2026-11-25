use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use super::library::LibraryIndex;
use super::recommend::{
    MbRateLimiter, RecommendedTrack, StreamingConfigSnapshot, artists_match,
    generate_recommendations, lookup_recording_metadata, sanitize_filename,
};
use super::ytdlp;

/// How many of the library's most-represented artists to use as recommendation
/// seeds per refresh. Independent of `recommendations_per_fetch` (which caps the
/// final result count) — a wider seed pool gives `generate_recommendations` more
/// chances to find candidates before falling back or running out.
const SEED_ARTIST_COUNT: usize = 8;

/// How many yt-dlp search results to consider per track when picking the best match.
const SEARCH_RESULT_COUNT: u32 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamingDisplayMode {
    Recommendations,
    Search { query: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DownloadOrigin {
    Manual {
        playlist_dir: PathBuf,
        insert_after_key: Option<String>,
    },
    Streaming {
        playlist_dir: PathBuf,
    },
}

pub enum StreamingEvent {
    RecommendationsReady(Vec<RecommendedTrack>),
    RecommendationsFailed(String),
    SearchResultsReady(String, Vec<RecommendedTrack>),
    SearchResultsFailed(String, String),
    DownloadStarted(String),
    DownloadComplete {
        in_flight_key: String,
        song_key: String,
        path: PathBuf,
        origin: DownloadOrigin,
    },
    DownloadFailed(String, String),
}

/// Owns the channel and in-memory state for the streaming feature. Lives in the
/// app's top-level state alongside `PlayerSession`, polled once per main-loop
/// iteration exactly like the existing Discord-presence/media-controls channel.
pub struct StreamingSession {
    pub recommendations: Vec<RecommendedTrack>,
    pub in_flight: HashSet<String>,
    pub refreshing: bool,
    pub searching: bool,
    pub display_mode: StreamingDisplayMode,
    pub last_manual_anchor: Option<String>,
    tx: Sender<StreamingEvent>,
    rx: Receiver<StreamingEvent>,
}

impl StreamingSession {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            recommendations: Vec::new(),
            in_flight: HashSet::new(),
            refreshing: false,
            searching: false,
            display_mode: StreamingDisplayMode::Recommendations,
            last_manual_anchor: None,
            tx,
            rx,
        }
    }

    pub fn send_event(&self, event: StreamingEvent) {
        let _ = self.tx.send(event);
    }

    /// Spawns a background thread that builds fresh recommendations from the given
    /// library snapshot. `library` should be built fresh (via `LibraryIndex::from_song_keys`)
    /// right before calling this, so it reflects any tracks downloaded since the last
    /// refresh. No-ops if a refresh is already in flight.
    pub fn refresh_recommendations(
        &mut self,
        library: LibraryIndex,
        config: StreamingConfigSnapshot,
        mbid_cache_path: PathBuf,
    ) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;

        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let seed_artists = library.seed_artists(SEED_ARTIST_COUNT);
            let result =
                generate_recommendations(&library, &seed_artists, &config, &mbid_cache_path);
            let event = match result {
                Ok(tracks) => StreamingEvent::RecommendationsReady(tracks),
                Err(e) => StreamingEvent::RecommendationsFailed(e.to_string()),
            };
            let _ = tx.send(event);
        });
    }

    /// Spawns a background thread that searches for tracks or direct URLs using yt-dlp,
    /// populating recommendations with the results.
    pub fn search(&mut self, query: String, ytdlp_path: String, max_results: usize) {
        self.display_mode = StreamingDisplayMode::Search {
            query: query.clone(),
        };
        if self.searching {
            return;
        }
        self.searching = true;

        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = ytdlp::search_query_or_url(&ytdlp_path, &query, max_results as u32);
            let event = match result {
                Ok(candidates) => {
                    let tracks: Vec<RecommendedTrack> = candidates
                        .into_iter()
                        .map(|c| ytdlp::parse_candidate(c, &query))
                        .collect();
                    StreamingEvent::SearchResultsReady(query, tracks)
                }
                Err(e) => StreamingEvent::SearchResultsFailed(query, e.to_string()),
            };
            let _ = tx.send(event);
        });
    }

    /// Spawns a background thread that downloads and extracts audio, tagging the resulting file.
    /// If `track.video_id` is already known (e.g. from a search result or direct URL), it skips
    /// re-searching and immediately downloads that exact target.
    pub fn start_download(
        &mut self,
        track: RecommendedTrack,
        download_dir: PathBuf,
        ytdlp_path: String,
        lyrics_config: crate::configuration::LyricsConfig,
        origin: DownloadOrigin,
    ) {
        let in_flight_key = sanitize_filename(&track.song_name());
        if self.in_flight.contains(&in_flight_key) {
            return;
        }
        self.in_flight.insert(in_flight_key.clone());

        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(StreamingEvent::DownloadStarted(in_flight_key.clone()));

            let result = (|| -> anyhow::Result<(String, PathBuf)> {
                let initial_song_key = sanitize_filename(&track.song_name());
                let initial_output_path = download_dir.join(format!("{}.mp3", initial_song_key));
                if initial_output_path.exists() {
                    log::debug!("Reusing cached audio file: {:?}", initial_output_path);
                    return Ok((initial_song_key, initial_output_path));
                }

                let video_id = if let Some(id) = &track.video_id {
                    id.clone()
                } else {
                    let query = ytdlp::search_query(&track);
                    let candidates = ytdlp::search(&ytdlp_path, &query, SEARCH_RESULT_COUNT)?;
                    let artist_string = track.artists.join(", ");
                    let best = ytdlp::pick_best(&candidates, &artist_string, &track.title)
                        .ok_or_else(|| {
                            anyhow::anyhow!("no matching search results for {}", query)
                        })?;
                    best.id.clone()
                };

                // Tier 2: Query MusicBrainz for canonical metadata if possible
                let mut limiter = MbRateLimiter::new();
                let (final_artists, final_title, album) =
                    if let Some(primary_artist) = track.artists.first() {
                        if let Some((mb_artists, mb_title, mb_album)) =
                            lookup_recording_metadata(primary_artist, &track.title, &mut limiter)
                        {
                            let mut combined = mb_artists;
                            // Retain cover performer/collaborator if credited in track.artists
                            if track.artists.len() > 1 {
                                for extra in &track.artists[1..] {
                                    if !combined.iter().any(|a| artists_match(a, extra)) {
                                        combined.push(extra.clone());
                                    }
                                }
                            }
                            (combined, mb_title, mb_album)
                        } else {
                            (track.artists.clone(), track.title.clone(), None)
                        }
                    } else {
                        (track.artists.clone(), track.title.clone(), None)
                    };

                let final_song_name = if final_artists.is_empty() {
                    final_title.clone()
                } else {
                    format!("{} - {}", final_artists.join(", "), final_title)
                };
                let song_key = sanitize_filename(&final_song_name);

                let output_path = download_dir.join(format!("{}.mp3", song_key));
                if output_path.exists() {
                    log::debug!("Reusing cached audio file: {:?}", output_path);
                    return Ok((song_key, output_path));
                }

                ytdlp::download(&ytdlp_path, &video_id, &output_path)?;

                let url = if video_id.starts_with("http://") || video_id.starts_with("https://") {
                    video_id
                } else {
                    format!("https://www.youtube.com/watch?v={}", video_id)
                };

                // Fetch lyrics if enabled
                let mut lyrics_str: Option<String> = None;
                if lyrics_config.enabled {
                    let artist_param = final_artists.first().map(|s| s.as_str()).unwrap_or("");
                    if let Some(track_lyrics) = crate::streaming::lyrics::fetch_best_lyrics(
                        artist_param,
                        &final_title,
                        album.as_deref(),
                        None,
                        lyrics_config.apple_developer_token.as_deref(),
                    ) {
                        let content = if lyrics_config.prefer_synced {
                            track_lyrics.to_elrc_string()
                        } else {
                            track_lyrics.plain_text.clone()
                        };
                        if lyrics_config.save_lrc_file {
                            let _ = ytdlp::save_lrc_file(&output_path, &content);
                        }
                        if lyrics_config.embed_in_audio {
                            lyrics_str = Some(content);
                        }
                    }
                }

                ytdlp::tag_file(
                    &output_path,
                    &final_artists,
                    &final_title,
                    album.as_deref(),
                    &url,
                    lyrics_str.as_deref(),
                )?;
                Ok((song_key, output_path))
            })();

            let event = match result {
                Ok((song_key, path)) => StreamingEvent::DownloadComplete {
                    in_flight_key,
                    song_key,
                    path,
                    origin,
                },
                Err(e) => StreamingEvent::DownloadFailed(in_flight_key, e.to_string()),
            };
            let _ = tx.send(event);
        });
    }

    /// Drains all pending events. Call once per main-loop iteration; the caller is
    /// responsible for acting on each event (updating `player.songs`, the
    /// recommendations Grid's labels, and the log).
    pub fn poll(&mut self) -> Vec<StreamingEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            match &event {
                StreamingEvent::RecommendationsReady(tracks) => {
                    self.refreshing = false;
                    if self.display_mode == StreamingDisplayMode::Recommendations {
                        self.recommendations = tracks.clone();
                    }
                }
                StreamingEvent::RecommendationsFailed(_) => {
                    self.refreshing = false;
                }
                StreamingEvent::SearchResultsReady(query, tracks) => {
                    self.searching = false;
                    if matches!(&self.display_mode, StreamingDisplayMode::Search { query: q } if q == query) {
                        self.recommendations = tracks.clone();
                    }
                }
                StreamingEvent::SearchResultsFailed(_, _) => {
                    self.searching = false;
                }
                StreamingEvent::DownloadComplete { in_flight_key, .. } => {
                    self.in_flight.remove(in_flight_key);
                    if self.in_flight.is_empty() {
                        self.last_manual_anchor = None;
                    }
                }
                StreamingEvent::DownloadFailed(key, _) => {
                    self.in_flight.remove(key);
                    if self.in_flight.is_empty() {
                        self.last_manual_anchor = None;
                    }
                }
                StreamingEvent::DownloadStarted(_) => {}
            }
            events.push(event);
        }
        events
    }
}

impl Default for StreamingSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_recommendation_reuse() {
        let temp_dir = std::env::temp_dir().join(format!("retro_test_cache_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let track = RecommendedTrack {
            artists: vec!["Cached Artist".to_string()],
            title: "Cached Title".to_string(),
            source_artist: "Seed Artist".to_string(),
            recording_mbid: None,
            video_id: Some("cached_id_123".to_string()),
        };
        let song_key = sanitize_filename(&track.song_name());
        let cached_file = temp_dir.join(format!("{}.mp3", song_key));
        std::fs::write(&cached_file, b"test cached audio content").unwrap();

        let mut session = StreamingSession::new();
        let lyrics_config = crate::configuration::LyricsConfig::default();
        session.start_download(
            track,
            temp_dir.clone(),
            "nonexistent-ytdlp".to_string(),
            lyrics_config,
            DownloadOrigin::Streaming {
                playlist_dir: temp_dir.clone(),
            },
        );

        // Allow worker thread to detect existing file and emit DownloadComplete
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            let events = session.poll();
            if events.iter().any(|e| matches!(
                e,
                StreamingEvent::DownloadComplete { path, origin, .. }
                    if path == &cached_file
                        && matches!(origin, DownloadOrigin::Streaming { playlist_dir } if playlist_dir == &temp_dir)
            )) {
                let _ = std::fs::remove_file(&cached_file);
                let _ = std::fs::remove_dir_all(&temp_dir);
                return;
            }
        }
        let _ = std::fs::remove_file(&cached_file);
        let _ = std::fs::remove_dir_all(&temp_dir);
        panic!("Timed out waiting for cached DownloadComplete event");
    }

    #[test]
    fn test_display_mode_protects_search() {
        let mut session = StreamingSession::new();
        session.display_mode = StreamingDisplayMode::Search {
            query: "rock".to_string(),
        };

        let track = RecommendedTrack {
            artists: vec!["Artist".to_string()],
            title: "Title".to_string(),
            source_artist: "Seed".to_string(),
            recording_mbid: None,
            video_id: None,
        };

        // Emit RecommendationsReady event while in Search mode
        session.send_event(StreamingEvent::RecommendationsReady(vec![track.clone()]));
        let _ = session.poll();

        // Visible recommendations must NOT be overwritten by recommendations event!
        assert!(session.recommendations.is_empty());

        // Now emit SearchResultsReady for the matching query
        session.send_event(StreamingEvent::SearchResultsReady("rock".to_string(), vec![track.clone()]));
        let _ = session.poll();

        // Search results must populate recommendations
        assert_eq!(session.recommendations.len(), 1);
        assert_eq!(session.recommendations[0].title, "Title");
    }

    #[test]
    fn test_stale_search_dropped() {
        let mut session = StreamingSession::new();
        session.display_mode = StreamingDisplayMode::Search {
            query: "query_b".to_string(),
        };

        let track_a = RecommendedTrack {
            artists: vec!["Artist A".to_string()],
            title: "Song A".to_string(),
            source_artist: "Search".to_string(),
            recording_mbid: None,
            video_id: None,
        };

        // Late response arrives for query_a
        session.send_event(StreamingEvent::SearchResultsReady("query_a".to_string(), vec![track_a]));
        let _ = session.poll();

        // Late response for old query must be dropped
        assert!(session.recommendations.is_empty());
    }
}

