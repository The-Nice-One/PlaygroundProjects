use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use rand::seq::SliceRandom;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshMode {
    /// Populates Streaming View UI once ready + adds to global/coordinator pools.
    UiAndPool,
    /// Silent background refresh (streaming cadence or post-refresh preload). Never alters UI list.
    PoolOnly,
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
    RecommendationsReady {
        generation: u64,
        tracks: Vec<RecommendedTrack>,
        ui_tracks: Vec<RecommendedTrack>,
        update_ui: bool,
    },
    RecommendationsFailed {
        generation: u64,
        error: String,
        ui_tracks: Vec<RecommendedTrack>,
        update_ui: bool,
    },
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
    pub global_pool: Vec<RecommendedTrack>,
    pub unshown_pool: Vec<RecommendedTrack>,
    pub in_flight: HashSet<String>,
    pub refreshing: bool,
    pub searching: bool,
    pub display_mode: StreamingDisplayMode,
    pub ui_refresh_pending: bool,
    pub refresh_generation: u64,
    pub ui_batch_size: usize,
    pub last_manual_anchor: Option<String>,
    tx: Sender<StreamingEvent>,
    rx: Receiver<StreamingEvent>,
}

impl StreamingSession {
    pub fn new() -> Self {
        Self::with_batch_size(8)
    }

    pub fn with_batch_size(ui_batch_size: usize) -> Self {
        let (tx, rx) = channel();
        Self {
            recommendations: Vec::new(),
            global_pool: Vec::new(),
            unshown_pool: Vec::new(),
            in_flight: HashSet::new(),
            refreshing: false,
            searching: false,
            display_mode: StreamingDisplayMode::Recommendations,
            ui_refresh_pending: false,
            refresh_generation: 0,
            ui_batch_size: ui_batch_size.max(1),
            last_manual_anchor: None,
            tx,
            rx,
        }
    }

    pub fn send_event(&self, event: StreamingEvent) {
        let _ = self.tx.send(event);
    }

    /// Invalidates any in-flight recommendation refreshes and clears all per-playlist pools.
    pub fn invalidate_and_clear_for_playlist_switch(&mut self) {
        self.refresh_generation += 1;
        self.refreshing = false;
        self.ui_refresh_pending = false;
        self.display_mode = StreamingDisplayMode::Recommendations;
        self.last_manual_anchor = None;
        self.recommendations.clear();
        self.global_pool.clear();
        self.unshown_pool.clear();
    }

    fn merge_into_pools(&mut self, incoming: &[RecommendedTrack]) {
        for track in incoming {
            let song_name = track.song_name();
            if !self
                .global_pool
                .iter()
                .any(|existing| existing.song_name() == song_name)
            {
                self.global_pool.push(track.clone());
                self.unshown_pool.push(track.clone());
            }
        }
    }

    /// Draws up to `count` unseen tracks from `unshown_pool` (padding short tails from `global_pool`
    /// so existing lists never regress into partial `N/A` slots) and updates `self.recommendations`.
    pub fn take_next_ui_batch(&mut self, count: usize) -> Option<Vec<RecommendedTrack>> {
        if self.unshown_pool.is_empty() || count == 0 {
            return None;
        }

        let mut rng = rand::rng();
        self.unshown_pool.shuffle(&mut rng);
        let take_n = count.min(self.unshown_pool.len());
        let mut batch: Vec<RecommendedTrack> = self.unshown_pool.drain(..take_n).collect();

        if batch.len() < count && self.global_pool.len() > batch.len() {
            let mut fallback: Vec<RecommendedTrack> = self
                .global_pool
                .iter()
                .filter(|t| !batch.iter().any(|b| b.song_name() == t.song_name()))
                .cloned()
                .collect();
            fallback.shuffle(&mut rng);
            let needed = count - batch.len();
            batch.extend(fallback.into_iter().take(needed));
        }

        self.recommendations = batch.clone();
        self.ui_refresh_pending = false;
        Some(batch)
    }

    /// Shuffles up to `count` tracks from `global_pool` into `self.recommendations` as a fallback
    /// when returning to recommendations without unseen tracks left.
    pub fn restore_pool_batch_if_available(&mut self, count: usize) -> Option<Vec<RecommendedTrack>> {
        if self.global_pool.is_empty() || count == 0 {
            return None;
        }
        let mut rng = rand::rng();
        let mut batch = self.global_pool.clone();
        batch.shuffle(&mut rng);
        batch.truncate(count);
        self.recommendations = batch.clone();
        Some(batch)
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
        mode: RefreshMode,
    ) {
        if mode == RefreshMode::UiAndPool {
            self.ui_refresh_pending = true;
        }
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        let generation = self.refresh_generation;

        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let seed_artists = library.seed_artists(SEED_ARTIST_COUNT);
            let result =
                generate_recommendations(&library, &seed_artists, &config, &mbid_cache_path);
            let event = match result {
                Ok(tracks) => StreamingEvent::RecommendationsReady {
                    generation,
                    tracks,
                    ui_tracks: Vec::new(),
                    update_ui: false,
                },
                Err(e) => StreamingEvent::RecommendationsFailed {
                    generation,
                    error: e.to_string(),
                    ui_tracks: Vec::new(),
                    update_ui: false,
                },
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
        self.ui_refresh_pending = false;
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
            match event {
                StreamingEvent::RecommendationsReady {
                    generation, tracks, ..
                } => {
                    if generation != self.refresh_generation {
                        continue;
                    }
                    self.refreshing = false;
                    self.merge_into_pools(&tracks);

                    let should_update_ui = self.display_mode == StreamingDisplayMode::Recommendations
                        && self.ui_refresh_pending;
                    let mut ui_tracks = Vec::new();
                    if should_update_ui {
                        self.ui_refresh_pending = false;
                        if let Some(batch) = self.take_next_ui_batch(self.ui_batch_size) {
                            ui_tracks = batch;
                        } else if !self.recommendations.is_empty() {
                            ui_tracks = self.recommendations.clone();
                        } else if let Some(restored) =
                            self.restore_pool_batch_if_available(self.ui_batch_size)
                        {
                            ui_tracks = restored;
                        }
                    }

                    events.push(StreamingEvent::RecommendationsReady {
                        generation,
                        tracks,
                        ui_tracks,
                        update_ui: should_update_ui,
                    });
                }
                StreamingEvent::RecommendationsFailed {
                    generation, error, ..
                } => {
                    if generation != self.refresh_generation {
                        continue;
                    }
                    self.refreshing = false;

                    let should_update_ui = self.display_mode == StreamingDisplayMode::Recommendations
                        && self.ui_refresh_pending;
                    let mut ui_tracks = Vec::new();
                    if should_update_ui {
                        self.ui_refresh_pending = false;
                        if !self.recommendations.is_empty() {
                            ui_tracks = self.recommendations.clone();
                        } else if let Some(restored) =
                            self.restore_pool_batch_if_available(self.ui_batch_size)
                        {
                            ui_tracks = restored;
                        }
                    }

                    events.push(StreamingEvent::RecommendationsFailed {
                        generation,
                        error,
                        ui_tracks,
                        update_ui: should_update_ui,
                    });
                }
                StreamingEvent::SearchResultsReady(query, tracks) => {
                    self.searching = false;
                    if matches!(&self.display_mode, StreamingDisplayMode::Search { query: q } if q == &query)
                    {
                        self.recommendations = tracks.clone();
                        events.push(StreamingEvent::SearchResultsReady(query, tracks));
                    }
                }
                StreamingEvent::SearchResultsFailed(query, err) => {
                    self.searching = false;
                    if matches!(&self.display_mode, StreamingDisplayMode::Search { query: q } if q == &query)
                    {
                        events.push(StreamingEvent::SearchResultsFailed(query, err));
                    }
                }
                StreamingEvent::DownloadComplete {
                    in_flight_key,
                    song_key,
                    path,
                    origin,
                } => {
                    self.in_flight.remove(&in_flight_key);
                    if self.in_flight.is_empty() {
                        self.last_manual_anchor = None;
                    }
                    events.push(StreamingEvent::DownloadComplete {
                        in_flight_key,
                        song_key,
                        path,
                        origin,
                    });
                }
                StreamingEvent::DownloadFailed(key, err) => {
                    self.in_flight.remove(&key);
                    if self.in_flight.is_empty() {
                        self.last_manual_anchor = None;
                    }
                    events.push(StreamingEvent::DownloadFailed(key, err));
                }
                StreamingEvent::DownloadStarted(key) => {
                    events.push(StreamingEvent::DownloadStarted(key));
                }
            }
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
        session.send_event(StreamingEvent::RecommendationsReady {
            generation: 0,
            tracks: vec![track.clone()],
            ui_tracks: Vec::new(),
            update_ui: false,
        });
        let _ = session.poll();

        // Visible recommendations must NOT be overwritten by recommendations event!
        assert!(session.recommendations.is_empty());

        // Now emit SearchResultsReady for the matching query
        session.send_event(StreamingEvent::SearchResultsReady(
            "rock".to_string(),
            vec![track.clone()],
        ));
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
        session.send_event(StreamingEvent::SearchResultsReady(
            "query_a".to_string(),
            vec![track_a],
        ));
        let events = session.poll();

        // Late response for old query must be dropped
        assert!(events.is_empty());
        assert!(session.recommendations.is_empty());
    }

    fn dummy_track(artist: &str, title: &str) -> RecommendedTrack {
        RecommendedTrack {
            artists: vec![artist.to_string()],
            title: title.to_string(),
            source_artist: artist.to_string(),
            recording_mbid: None,
            video_id: None,
        }
    }

    #[test]
    fn test_recommendations_protection_blocks_background_refresh() {
        let mut session = StreamingSession::with_batch_size(2);
        session.ui_refresh_pending = true;

        let initial = vec![
            dummy_track("A", "1"),
            dummy_track("A", "2"),
            dummy_track("A", "3"),
        ];
        session.send_event(StreamingEvent::RecommendationsReady {
            generation: 0,
            tracks: initial,
            ui_tracks: Vec::new(),
            update_ui: false,
        });
        let events = session.poll();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            StreamingEvent::RecommendationsReady { update_ui: true, ui_tracks, .. } if ui_tracks.len() == 2
        ));
        let initial_shown = session.recommendations.clone();
        assert_eq!(initial_shown.len(), 2);
        assert!(!session.ui_refresh_pending);

        // Background PoolOnly refresh arrives while ui_refresh_pending is false
        let bg = vec![dummy_track("B", "4"), dummy_track("B", "5")];
        session.send_event(StreamingEvent::RecommendationsReady {
            generation: 0,
            tracks: bg,
            ui_tracks: Vec::new(),
            update_ui: false,
        });
        let bg_events = session.poll();
        assert!(matches!(
            &bg_events[0],
            StreamingEvent::RecommendationsReady { update_ui: false, .. }
        ));
        // Displayed recommendations stayed locked, while pools gained the new tracks
        assert_eq!(session.recommendations, initial_shown);
        assert_eq!(session.global_pool.len(), 5);
        assert_eq!(session.unshown_pool.len(), 3);
    }

    #[test]
    fn test_pool_first_refresh_and_padding() {
        let mut session = StreamingSession::with_batch_size(3);
        session.ui_refresh_pending = true;

        // Fetch 4 tracks total -> 3 displayed on initial fill, 1 left in unshown_pool
        let tracks = vec![
            dummy_track("A", "1"),
            dummy_track("A", "2"),
            dummy_track("A", "3"),
            dummy_track("A", "4"),
        ];
        session.send_event(StreamingEvent::RecommendationsReady {
            generation: 0,
            tracks,
            ui_tracks: Vec::new(),
            update_ui: false,
        });
        let _ = session.poll();
        assert_eq!(session.recommendations.len(), 3);
        assert_eq!(session.unshown_pool.len(), 1);

        // Explicit Refresh draws the 1 unshown track + pads 2 from global_pool so there are no N/A slots
        let batch = session.take_next_ui_batch(3).unwrap();
        assert_eq!(batch.len(), 3);
        assert!(batch.iter().any(|t| t.title == "4"));
        assert!(session.unshown_pool.is_empty());
    }

    #[test]
    fn test_empty_or_failed_refresh_preserves_existing_list() {
        let mut session = StreamingSession::with_batch_size(2);
        session.ui_refresh_pending = true;

        session.send_event(StreamingEvent::RecommendationsReady {
            generation: 0,
            tracks: vec![dummy_track("A", "1"), dummy_track("A", "2")],
            ui_tracks: Vec::new(),
            update_ui: false,
        });
        let _ = session.poll();
        let locked_list = session.recommendations.clone();

        // Explicit UiAndPool refresh runs when pool is empty, and returns 0 tracks
        session.ui_refresh_pending = true;
        session.send_event(StreamingEvent::RecommendationsReady {
            generation: 0,
            tracks: Vec::new(),
            ui_tracks: Vec::new(),
            update_ui: false,
        });
        let empty_events = session.poll();
        assert!(matches!(
            &empty_events[0],
            StreamingEvent::RecommendationsReady { update_ui: true, ui_tracks, .. } if ui_tracks == &locked_list
        ));
        assert_eq!(session.recommendations, locked_list);

        // Now API errors instead
        session.ui_refresh_pending = true;
        session.send_event(StreamingEvent::RecommendationsFailed {
            generation: 0,
            error: "rate limit".to_string(),
            ui_tracks: Vec::new(),
            update_ui: false,
        });
        let err_events = session.poll();
        assert!(matches!(
            &err_events[0],
            StreamingEvent::RecommendationsFailed { update_ui: true, ui_tracks, .. } if ui_tracks == &locked_list
        ));
        assert_eq!(session.recommendations, locked_list);
    }

    #[test]
    fn test_playlist_switch_invalidates_generation_and_clears_pools() {
        let mut session = StreamingSession::with_batch_size(2);
        session.ui_refresh_pending = true;
        session.refreshing = true;

        session.send_event(StreamingEvent::RecommendationsReady {
            generation: 0,
            tracks: vec![dummy_track("Old", "1"), dummy_track("Old", "2")],
            ui_tracks: Vec::new(),
            update_ui: false,
        });
        let _ = session.poll();
        assert_eq!(session.recommendations.len(), 2);

        // User switches playlist while a gen-0 refresh was still in flight
        session.refreshing = true;
        session.invalidate_and_clear_for_playlist_switch();
        assert_eq!(session.refresh_generation, 1);
        assert!(!session.refreshing);
        assert!(session.recommendations.is_empty());
        assert!(session.global_pool.is_empty());
        assert!(session.unshown_pool.is_empty());

        // Stale gen-0 result arrives from the previous playlist -> must be completely dropped
        session.ui_refresh_pending = true;
        session.send_event(StreamingEvent::RecommendationsReady {
            generation: 0,
            tracks: vec![dummy_track("Old", "Stale")],
            ui_tracks: Vec::new(),
            update_ui: false,
        });
        let events = session.poll();
        assert!(events.is_empty());
        assert!(session.recommendations.is_empty());
        assert!(session.global_pool.is_empty());
    }
}

