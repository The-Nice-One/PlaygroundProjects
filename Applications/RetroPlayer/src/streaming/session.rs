use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

use super::library::LibraryIndex;
use super::recommend::{generate_recommendations, RecommendedTrack, StreamingConfigSnapshot};
use super::ytdlp;

/// How many of the library's most-represented artists to use as recommendation
/// seeds per refresh. Independent of `recommendations_per_fetch` (which caps the
/// final result count) — a wider seed pool gives `generate_recommendations` more
/// chances to find candidates before falling back or running out.
const SEED_ARTIST_COUNT: usize = 8;

/// How many yt-dlp search results to consider per track when picking the best match.
const SEARCH_RESULT_COUNT: u32 = 5;

pub enum StreamingEvent {
    RecommendationsReady(Vec<RecommendedTrack>),
    RecommendationsFailed(String),
    DownloadStarted(String),
    DownloadComplete(String, PathBuf),
    DownloadFailed(String, String),
}

/// Owns the channel and in-memory state for the streaming feature. Lives in the
/// app's top-level state alongside `PlayerSession`, polled once per main-loop
/// iteration exactly like the existing Discord-presence/media-controls channel.
pub struct StreamingSession {
    pub recommendations: Vec<RecommendedTrack>,
    pub in_flight: HashSet<String>,
    pub refreshing: bool,
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
            tx,
            rx,
        }
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
            let result = generate_recommendations(&library, &seed_artists, &config, &mbid_cache_path);
            let event = match result {
                Ok(tracks) => StreamingEvent::RecommendationsReady(tracks),
                Err(e) => StreamingEvent::RecommendationsFailed(e.to_string()),
            };
            let _ = tx.send(event);
        });
    }

    /// Spawns a background thread that searches yt-dlp for the best match, downloads
    /// and extracts audio, and tags the resulting file. No-ops (rather than queuing a
    /// duplicate) if this exact track is already downloading.
    ///
    /// Duplicate download attempts for the same track are ignored while one is in
    /// flight, so a single recommendation cannot be downloaded twice concurrently.
    pub fn start_download(&mut self, track: RecommendedTrack, download_dir: PathBuf, ytdlp_path: String) {
        let key = track.song_name();
        if self.in_flight.contains(&key) {
            return;
        }
        self.in_flight.insert(key.clone());

        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(StreamingEvent::DownloadStarted(key.clone()));

            let result = (|| -> anyhow::Result<PathBuf> {
                let query = ytdlp::search_query(&track);
                let candidates = ytdlp::search(&ytdlp_path, &query, SEARCH_RESULT_COUNT)?;
                let artist_string = track.artists.join(", ");
                let best = ytdlp::pick_best(&candidates, &artist_string, &track.title)
                    .ok_or_else(|| anyhow::anyhow!("no matching search results for {}", query))?;

                let output_path = download_dir.join(format!("{}.mp3", track.song_name()));
                ytdlp::download(&ytdlp_path, &best.id, &output_path)?;
                
                let url = format!("https://www.youtube.com/watch?v={}", best.id);
                ytdlp::tag_file(&output_path, &track.artists, &track.title, &url)?;
                Ok(output_path)
            })();

            let event = match result {
                Ok(path) => StreamingEvent::DownloadComplete(key, path),
                Err(e) => StreamingEvent::DownloadFailed(key, e.to_string()),
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
                    self.recommendations = tracks.clone();
                }
                StreamingEvent::RecommendationsFailed(_) => {
                    self.refreshing = false;
                }
                StreamingEvent::DownloadComplete(key, _) | StreamingEvent::DownloadFailed(key, _) => {
                    self.in_flight.remove(key);
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