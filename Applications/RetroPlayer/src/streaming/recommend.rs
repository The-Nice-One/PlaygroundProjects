use log::{debug, warn};
use rand::seq::SliceRandom;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use super::http::{USER_AGENT, call_with_retry};
use super::library::LibraryIndex;

/// A candidate recommendation, always backed by a real MusicBrainz/ListenBrainz/
/// Last.fm entity or YouTube search/URL by the time it reaches this stage
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecommendedTrack {
    pub artists: Vec<String>,
    pub title: String,
    /// Which library artist this suggestion was derived from, shown to the user as
    /// context ("because you listen to Jamie Duffy"), or "Search" / "URL" for direct queries.
    pub source_artist: String,
    pub recording_mbid: Option<String>,
    pub video_id: Option<String>,
}

impl RecommendedTrack {
    /// Formats the song name to "Artist[, Artist2] - Title"
    pub fn song_name(&self) -> String {
        if self.artists.is_empty() {
            self.title.clone()
        } else {
            format!("{} - {}", self.artists.join(", "), self.title)
        }
    }
}

/// Sanitizes a string for safe usage as a file name across platforms (Windows, macOS, Linux).
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Maps common Latin diacritics to their base ASCII equivalents.
pub fn deaccent_char(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'Ā' => {
            'a'
        }
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'È' | 'É' | 'Ê' | 'Ë' | 'Ē' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'Ì' | 'Í' | 'Î' | 'Ï' | 'Ī' => 'i',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'Ō' => {
            'o'
        }
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ū' => 'u',
        'ý' | 'ÿ' | 'Ý' | 'Ÿ' => 'y',
        'ñ' | 'Ñ' => 'n',
        'ç' | 'Ç' => 'c',
        other => other,
    }
}

/// Normalizes an artist name for comparison by folding diacritics to ASCII,
/// lowercasing, and stripping non-alphanumeric punctuation.
pub fn normalize_artist_name(name: &str) -> String {
    name.chars()
        .map(|c| deaccent_char(c).to_ascii_lowercase())
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Checks whether two artist names are identical under case- and diacritic-insensitive normalization.
pub fn artists_match(a: &str, b: &str) -> bool {
    normalize_artist_name(a) == normalize_artist_name(b)
}

/// Plain-data snapshot of the bits of `Configuration.streaming` that the background
/// thread needs. Built once on the main thread (which can read the `CONFIGURATION`
/// `RwLock` briefly) and then moved into the spawned thread, which must never itself
/// try to acquire that lock across a multi-second network operation.
#[derive(Clone)]
pub struct StreamingConfigSnapshot {
    pub recommendations_per_fetch: usize,
    pub pool_size: usize,
    pub listenbrainz_mode: String,
}

// MusicBrainz: artist name -> MBID, with an on-disk cache (avoids re-resolving
// the same artist on every refresh) and a simple rate limiter (MusicBrainz asks
// for at most 1 request/second from unauthenticated clients).

pub struct MbRateLimiter {
    last: Option<Instant>,
}

impl MbRateLimiter {
    pub fn new() -> Self {
        Self { last: None }
    }

    pub fn wait(&mut self) {
        if let Some(last) = self.last {
            let elapsed = last.elapsed();
            let min_interval = Duration::from_millis(1100);
            if elapsed < min_interval {
                std::thread::sleep(min_interval - elapsed);
            }
        }
        self.last = Some(Instant::now());
    }
}

/// Adaptive ListenBrainz rate limiter respecting `X-RateLimit-Remaining` and
/// `X-RateLimit-Reset-In` headers alongside gentle request pacing.
pub struct LbRateLimiter {
    last: Option<Instant>,
    paused_until: Option<Instant>,
}

impl LbRateLimiter {
    pub fn new() -> Self {
        Self {
            last: None,
            paused_until: None,
        }
    }

    pub fn wait(&mut self) {
        if let Some(until) = self.paused_until.take() {
            let now = Instant::now();
            if until > now {
                std::thread::sleep(until - now);
            }
        }
        if let Some(last) = self.last {
            let elapsed = last.elapsed();
            let min_interval = Duration::from_millis(150);
            if elapsed < min_interval {
                std::thread::sleep(min_interval - elapsed);
            }
        }
        self.last = Some(Instant::now());
    }

    pub fn update_from_response(&mut self, resp: &ureq::Response) {
        let remaining = resp
            .header("X-RateLimit-Remaining")
            .and_then(|v| v.trim().parse::<u32>().ok());
        if let Some(rem) = remaining {
            if rem <= 2 {
                let reset_secs = resp
                    .header("X-RateLimit-Reset-In")
                    .and_then(|v| v.trim().parse::<u64>().ok())
                    .unwrap_or(1)
                    .clamp(1, 5);
                self.paused_until = Some(Instant::now() + Duration::from_secs(reset_secs));
            }
        }
    }
}

pub type MbidCache = HashMap<String, Option<String>>;

pub fn load_mbid_cache(path: &Path) -> MbidCache {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

pub fn save_mbid_cache(path: &Path, cache: &MbidCache) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(cache)?;
    std::fs::write(path, contents)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct MbArtistSearchResponse {
    #[serde(default)]
    artists: Vec<MbArtist>,
}

#[derive(Debug, Deserialize)]
struct MbArtist {
    id: String,
    #[serde(default)]
    score: Option<serde_json::Value>,
}

fn mb_score_as_u32(artist: &MbArtist) -> u32 {
    match &artist.score {
        Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0) as u32,
        Some(serde_json::Value::String(s)) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

/// Escapes the characters that are meaningful to MusicBrainz's Lucene-based query
/// syntax inside a quoted phrase.
fn lucene_escape(value: &str) -> String {
    value.replace('"', "\\\"")
}

/// Resolves an artist name to a MusicBrainz artist MBID, using (and updating) the
/// on-disk cache so repeated lookups for the same artist across refreshes don't hit
/// the network.
pub fn resolve_artist_mbid(
    artist_name: &str,
    cache: &mut MbidCache,
    limiter: &mut MbRateLimiter,
) -> anyhow::Result<Option<String>> {
    if let Some(cached) = cache.get(artist_name) {
        return Ok(cached.clone());
    }

    limiter.wait();

    let query = format!("artist:\"{}\"", lucene_escape(artist_name));
    let response: MbArtistSearchResponse = call_with_retry(|agent| {
        agent
            .get("https://musicbrainz.org/ws/2/artist")
            .set("User-Agent", USER_AGENT)
            .query("query", &query)
            .query("fmt", "json")
    })
    .map_err(|e| {
        warn!("MusicBrainz API error for {}: {}", artist_name, e);
        e
    })?
    .into_json()?;

    let best = response
        .artists
        .into_iter()
        .max_by_key(|a| mb_score_as_u32(a));

    let mbid = best.map(|a| a.id);
    cache.insert(artist_name.to_string(), mbid.clone());
    Ok(mbid)
}

#[derive(Debug, Deserialize)]
struct MbRecordingResponse {
    #[serde(default)]
    recordings: Vec<MbRecording>,
}

#[derive(Debug, Deserialize)]
struct MbRecording {
    title: String,
    #[serde(default)]
    score: Option<serde_json::Value>,
    #[serde(default, rename = "artist-credit")]
    artist_credit: Vec<MbArtistCredit>,
    #[serde(default)]
    releases: Vec<MbRelease>,
}

#[derive(Debug, Deserialize)]
struct MbArtistCredit {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MbRelease {
    title: String,
}

fn mb_recording_score_as_u32(score: &Option<serde_json::Value>) -> u32 {
    match score {
        Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0) as u32,
        Some(serde_json::Value::String(s)) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

/// Queries MusicBrainz's recording endpoint to resolve canonical artist(s), track title,
/// and album for a candidate artist and title pair. Tries exact query first, then flipped
/// (in case the video was "Title - Artist").
pub fn lookup_recording_metadata(
    artist: &str,
    title: &str,
    limiter: &mut MbRateLimiter,
) -> Option<(Vec<String>, String, Option<String>)> {
    if artist.is_empty() || title.is_empty() {
        return None;
    }

    let search_musicbrainz = |a: &str,
                              t: &str,
                              lim: &mut MbRateLimiter|
     -> Option<(Vec<String>, String, Option<String>)> {
        lim.wait();
        let query = format!(
            "recording:\"{}\" AND artist:\"{}\"",
            lucene_escape(t),
            lucene_escape(a)
        );

        let response: MbRecordingResponse = call_with_retry(|agent| {
            agent
                .get("https://musicbrainz.org/ws/2/recording")
                .set("User-Agent", USER_AGENT)
                .timeout(std::time::Duration::from_secs(3))
                .query("query", &query)
                .query("fmt", "json")
                .query("limit", "3")
        })
        .ok()?
        .into_json()
        .ok()?;

        let best = response.recordings.into_iter().next()?;
        let score = mb_recording_score_as_u32(&best.score);

        if score >= 80 {
            let artists: Vec<String> = best
                .artist_credit
                .into_iter()
                .map(|ac| ac.name.trim().to_string())
                .filter(|n| !n.is_empty())
                .collect();
            let album = best
                .releases
                .into_iter()
                .next()
                .map(|r| r.title.trim().to_string());
            if !artists.is_empty() && !best.title.trim().is_empty() {
                return Some((artists, best.title.trim().to_string(), album));
            }
        }
        None
    };

    // 1. Try with (artist, title)
    if let Some(res) = search_musicbrainz(artist, title, limiter) {
        return Some(res);
    }

    // 2. Try flipped (title, artist) in case the video was "Title - Artist"
    if let Some(res) = search_musicbrainz(title, artist, limiter) {
        return Some(res);
    }

    None
}

// ListenBrainz: LB Radio (artist-seed similarity) + recording metadata lookup.

#[derive(Debug, Deserialize, Clone)]
struct LbRadioEntry {
    recording_mbid: Option<String>,
    #[serde(default)]
    similar_artist_name: Option<String>,
}

type LbRadioArtistResponse = HashMap<String, Vec<LbRadioEntry>>;

/// Queries ListenBrainz's "LB Radio" endpoint for recordings from artists similar to
/// the given seed artist.
pub fn fetch_lb_radio_for_artist(
    seed_mbid: &str,
    mode: &str,
    max_similar_artists: u32,
    max_recordings_per_artist: u32,
    lb_limiter: &mut LbRateLimiter,
) -> anyhow::Result<Vec<(String, String)>> {
    lb_limiter.wait();
    let url = format!(
        "https://api.listenbrainz.org/1/lb-radio/artist/{}",
        seed_mbid
    );
    let max_sim = max_similar_artists.to_string();
    let max_rec = max_recordings_per_artist.to_string();
    let resp = call_with_retry(|agent| {
        agent
            .get(&url)
            .set("User-Agent", USER_AGENT)
            .query("mode", mode)
            .query("max_similar_artists", &max_sim)
            .query("max_recordings_per_artist", &max_rec)
            .query("pop_begin", "0")
            .query("pop_end", "100")
    })?;
    lb_limiter.update_from_response(&resp);
    let response: LbRadioArtistResponse = resp.into_json()?;

    let mut pairs = Vec::new();
    for entries in response.values() {
        for entry in entries {
            if let Some(recording_mbid) = &entry.recording_mbid {
                let artist_name = entry
                    .similar_artist_name
                    .clone()
                    .unwrap_or_else(|| "Unknown Artist".to_string());
                pairs.push((artist_name, recording_mbid.clone()));
            }
        }
    }
    Ok(pairs)
}

#[derive(Debug, Deserialize)]
struct RecordingMetadataArtist {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RecordingMetadataRecording {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RecordingMetadataEntry {
    #[serde(default)]
    artist: Option<RecordingMetadataArtist>,
    #[serde(default)]
    recording: Option<RecordingMetadataRecording>,
    #[serde(default, rename = "artist_name")]
    artist_name: Option<String>,
    #[serde(default, rename = "recording_name")]
    recording_name: Option<String>,
    #[serde(default, rename = "track_name")]
    track_name: Option<String>,
}

type RecordingMetadataResponse = HashMap<String, RecordingMetadataEntry>;

/// Resolves a batch of recording MBIDs into actual artist-credit name + track title pairs.
pub fn fetch_recording_titles(
    recording_mbids: &[String],
    lb_limiter: &mut LbRateLimiter,
) -> anyhow::Result<HashMap<String, (String, String)>> {
    if recording_mbids.is_empty() {
        return Ok(HashMap::new());
    }

    lb_limiter.wait();
    let joined = recording_mbids.join(",");
    let resp = call_with_retry(|agent| {
        agent
            .get("https://api.listenbrainz.org/1/metadata/recording/")
            .set("User-Agent", USER_AGENT)
            .query("recording_mbids", &joined)
            .query("inc", "artist")
    })
    .map_err(|e| {
        warn!("ListenBrainz Metadata error: {}", e);
        e
    })?;
    lb_limiter.update_from_response(&resp);
    let response: RecordingMetadataResponse = resp.into_json()?;

    let mut titles = HashMap::new();
    for (mbid, entry) in response {
        let artist = entry.artist.map(|a| a.name).or(entry.artist_name);
        let title = entry
            .recording
            .map(|r| r.name)
            .or(entry.recording_name)
            .or(entry.track_name);

        if let (Some(artist), Some(title)) = (artist, title) {
            titles.insert(mbid, (artist, title));
        }
    }
    Ok(titles)
}

const ARTIST_WEIGHT_SLOTS: [usize; 5] = [50, 30, 10, 5, 2];

/// Generates a deduplicated list of recommended tracks for the given seed artists.
pub fn generate_recommendations(
    library: &LibraryIndex,
    seed_artists: &[String],
    config: &StreamingConfigSnapshot,
    mbid_cache_path: &Path,
) -> anyhow::Result<Vec<RecommendedTrack>> {
    let mut mbid_cache = load_mbid_cache(mbid_cache_path);
    let mut limiter = MbRateLimiter::new();
    let mut lb_limiter = LbRateLimiter::new();
    let mut candidates: Vec<RecommendedTrack> = Vec::new();
    let mut last_error: Option<anyhow::Error> = None;
    let mut rng = rand::rng();

    if seed_artists.is_empty() {
        log::warn!("No seed artists found in library index. Check your filename formats!");
        return Ok(candidates);
    }

    let total_recs = config.pool_size.max(config.recommendations_per_fetch);

    // Assign primary weights to the top 5 artists, while retaining remaining seed artists
    // (fallback weight 2) so deficits roll over when top artists lack LB Radio data.
    let active_artists: Vec<&String> = seed_artists.iter().collect();
    let active_weights: Vec<usize> = (0..active_artists.len())
        .map(|i| ARTIST_WEIGHT_SLOTS.get(i).copied().unwrap_or(2))
        .collect();

    let total_weight: usize = active_weights.iter().sum();

    let mut remaining_recs = total_recs;
    let mut deficit = 0;

    for (index, seed_artist) in active_artists.into_iter().enumerate() {
        if remaining_recs == 0 {
            break;
        }

        log::debug!("Processing seed artist: {}", seed_artist);

        // 1. Calculate the proportional slot for this artist
        let weight = active_weights[index];
        let mut target_slot = if index == active_weights.len() - 1 {
            remaining_recs // Ensure the last artist gets all remaining slots to avoid rounding losses
        } else {
            (total_recs as f64 * (weight as f64 / total_weight as f64)).round() as usize
        };

        // 2. Add any missed quota (deficit) from previous artists, then clamp to remaining total
        target_slot += deficit;
        target_slot = target_slot.min(remaining_recs);

        // Lookup Artist MBID
        let seed_mbid = match resolve_artist_mbid(seed_artist, &mut mbid_cache, &mut limiter) {
            Ok(Some(mbid)) => mbid,
            Ok(None) => {
                deficit += target_slot;
                continue;
            }
            Err(e) => {
                warn!("Failed to resolve MBID for '{}': {}", seed_artist, e);
                last_error = Some(e);
                deficit += target_slot;
                continue;
            }
        };

        // Fetch ListenBrainz Radio
        let lb_pairs = match fetch_lb_radio_for_artist(
            &seed_mbid,
            &config.listenbrainz_mode,
            5,
            3,
            &mut lb_limiter,
        ) {
            Ok(pairs) => pairs,
            Err(e) => {
                warn!("ListenBrainz radio error for '{}': {}", seed_artist, e);
                last_error = Some(e);
                deficit += target_slot;
                continue;
            }
        };

        if lb_pairs.is_empty() {
            deficit += target_slot; // Roll over the missed slot to the next artist
            continue;
        }

        let recording_mbids: Vec<String> = lb_pairs.iter().map(|(_, mbid)| mbid.clone()).collect();
        let titles = match fetch_recording_titles(&recording_mbids, &mut lb_limiter) {
            Ok(t) => t,
            Err(e) => {
                warn!("ListenBrainz metadata error for '{}': {}", seed_artist, e);
                last_error = Some(e);
                deficit += target_slot;
                continue;
            }
        };

        debug!(
            "ListenBrainz found {} potential recordings for {}",
            lb_pairs.len(),
            seed_artist
        );

        let mut options = vec![];
        for (_, recording_mbid) in lb_pairs {
            if let Some((artist_name, title)) = titles.get(&recording_mbid) {
                if !library.contains(artist_name, title) {
                    options.push(RecommendedTrack {
                        artists: vec![artist_name.clone()],
                        title: title.clone(),
                        source_artist: seed_artist.clone(),
                        recording_mbid: Some(recording_mbid),
                        video_id: None,
                    });
                }
            }
        }

        // 3. Shuffle options and take only up to this artist's target slot
        options.shuffle(&mut rng);
        let actual_taken = options.len().min(target_slot);
        options.truncate(actual_taken);
        candidates.extend(options);

        // 4. Update the remaining counter and record any deficit for the next artist
        remaining_recs = remaining_recs.saturating_sub(actual_taken);
        deficit = target_slot.saturating_sub(actual_taken);
    }

    if let Err(e) = save_mbid_cache(mbid_cache_path, &mbid_cache) {
        warn!("Failed to save MBID cache: {}", e);
    }

    if candidates.is_empty()
        && let Some(err) = last_error
    {
        return Err(err);
    }

    // Final shuffle to mix the top artists' songs with the smaller artists' songs
    candidates.shuffle(&mut rng);
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_song_name_with_and_without_artists() {
        let track_with_artist = RecommendedTrack {
            artists: vec!["Daft Punk".to_string()],
            title: "Get Lucky".to_string(),
            source_artist: "Search".to_string(),
            recording_mbid: None,
            video_id: Some("123".to_string()),
        };
        assert_eq!(track_with_artist.song_name(), "Daft Punk - Get Lucky");

        let track_no_artist = RecommendedTrack {
            artists: vec![],
            title: "Solo Track".to_string(),
            source_artist: "URL".to_string(),
            recording_mbid: None,
            video_id: Some("456".to_string()),
        };
        assert_eq!(track_no_artist.song_name(), "Solo Track");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            sanitize_filename("Artist / Name : Track * Title ? \"Quote\" <bracket> | test"),
            "Artist _ Name _ Track _ Title _ _Quote_ _bracket_ _ test"
        );
    }

    #[test]
    fn test_artists_match_diacritics() {
        assert!(artists_match("ÊMIA", "EMIA"));
        assert!(artists_match("emia", "ÊMIA"));
        assert!(artists_match("Beyoncé", "Beyonce"));
        assert!(artists_match("Motörhead", "Motorhead"));
        assert!(!artists_match("Duumu", "EMIA"));
    }

    #[test]
    #[ignore]
    fn test_diagnose_real_playlist_recommendations() {
        let config = crate::configuration::CONFIGURATION.read().unwrap();
        let playlist_path = crate::configuration::load_last_playlist()
            .filter(|p| !p.is_empty())
            .or_else(|| {
                if !config.songs_directory.is_empty() {
                    Some(config.songs_directory.clone())
                } else {
                    config.song_directories.first().cloned()
                }
            })
            .expect("No configured playlist found on machine");

        let listenbrainz_mode = config.streaming.listenbrainz_mode.clone();
        let recommendations_per_fetch = config.streaming.recommendations_per_fetch;
        drop(config);

        println!("=== REAL PLAYLIST DIAGNOSTIC ===");
        println!("Playlist path: {}", playlist_path);
        println!("listenbrainz_mode: {}, recommendations_per_fetch: {}", listenbrainz_mode, recommendations_per_fetch);

        let mut player = crate::player::PlayerSession::default();
        player.load_playlist(&playlist_path);
        println!("Loaded songs count: {}", player.songs.len());
        for (i, key) in player.songs.keys().take(10).enumerate() {
            println!("  Sample song[{}]: {}", i, key);
        }

        let library = LibraryIndex::from_song_keys(&player.songs);
        println!("Parsed artist_count: {}", library.artist_count());
        let seed_artists = library.seed_artists(10);
        println!("Top 10 seed_artists: {:?}", seed_artists);

        let real_cache_path = crate::PROJECT_DIRECTORY.data_dir().join("mbid_cache.json");
        println!("Real MBID cache path: {:?} (exists={})", real_cache_path, real_cache_path.exists());
        let mut mbid_cache = load_mbid_cache(&real_cache_path);
        println!("MBID cache entries: {}", mbid_cache.len());

        let mut limiter = MbRateLimiter::new();
        for (idx, artist) in seed_artists.iter().enumerate() {
            let cached_state = mbid_cache.get(artist).cloned();
            println!("\n--- Seed Artist #{}: '{}' (in_cache={:?}) ---", idx + 1, artist, cached_state);
            let mbid_res = resolve_artist_mbid(artist, &mut mbid_cache, &mut limiter);
            println!("  resolve_artist_mbid -> {:?}", mbid_res);

            let Ok(Some(seed_mbid)) = mbid_res else {
                continue;
            };

            let lb_url = format!("https://api.listenbrainz.org/1/lb-radio/artist/{}", seed_mbid);
            let lb_resp = crate::streaming::http::http_agent()
                .get(&lb_url)
                .set("User-Agent", USER_AGENT)
                .query("mode", &listenbrainz_mode)
                .query("max_similar_artists", "5")
                .query("max_recordings_per_artist", "3")
                .query("pop_begin", "0")
                .query("pop_end", "100")
                .call();

            match lb_resp {
                Ok(resp) => {
                    let status = resp.status();
                    let rem = resp.header("X-RateLimit-Remaining").map(|s| s.to_string());
                    let reset = resp.header("X-RateLimit-Reset-In").map(|s| s.to_string());
                    let body_str = resp.into_string().unwrap_or_default();
                    println!(
                        "  LB Radio HTTP {} (RateLimit-Remaining={:?}, Reset-In={:?}, body_len={})",
                        status, rem, reset, body_str.len()
                    );
                    if body_str.len() <= 200 {
                        println!("  LB Radio raw body: {}", body_str);
                    }
                    if let Ok(parsed) = serde_json::from_str::<LbRadioArtistResponse>(&body_str) {
                        let mbids: Vec<String> = parsed
                            .values()
                            .flat_map(|v| v.iter().filter_map(|e| e.recording_mbid.clone()))
                            .collect();
                        println!("  LB Radio parsed {} recording MBIDs", mbids.len());
                        if !mbids.is_empty() {
                            let joined = mbids.join(",");
                            let meta_resp = crate::streaming::http::http_agent()
                                .get("https://api.listenbrainz.org/1/metadata/recording/")
                                .set("User-Agent", USER_AGENT)
                                .query("recording_mbids", &joined)
                                .query("inc", "artist")
                                .call();
                            match meta_resp {
                                Ok(mresp) => {
                                    let mstatus = mresp.status();
                                    let mrem = mresp.header("X-RateLimit-Remaining").map(|s| s.to_string());
                                    let mreset = mresp.header("X-RateLimit-Reset-In").map(|s| s.to_string());
                                    let mbody = mresp.into_string().unwrap_or_default();
                                    println!(
                                        "  LB Metadata HTTP {} (RateLimit-Remaining={:?}, Reset-In={:?}, body_len={})",
                                        mstatus, mrem, mreset, mbody.len()
                                    );
                                    if let Ok(titles) = serde_json::from_str::<RecordingMetadataResponse>(&mbody) {
                                        let mut resolved = 0usize;
                                        let mut unowned = 0usize;
                                        for entry in titles.values() {
                                            let a = entry.artist.as_ref().map(|x| x.name.as_str()).or(entry.artist_name.as_deref());
                                            let t = entry.recording.as_ref().map(|x| x.name.as_str()).or(entry.recording_name.as_deref()).or(entry.track_name.as_deref());
                                            if let (Some(a), Some(t)) = (a, t) {
                                                resolved += 1;
                                                if !library.contains(a, t) {
                                                    unowned += 1;
                                                }
                                            }
                                        }
                                        println!("  LB Metadata entries={}, resolved_artist_title={}, unowned={}", titles.len(), resolved, unowned);
                                    } else {
                                        println!("  LB Metadata JSON parse failed! sample={}", &mbody[..mbody.len().min(300)]);
                                    }
                                }
                                Err(e) => println!("  LB Metadata HTTP ERROR: {:?}", e),
                            }
                        }
                    } else {
                        println!("  LB Radio JSON parse FAILED! sample={}", &body_str[..body_str.len().min(300)]);
                    }
                }
                Err(ureq::Error::Status(code, resp)) => {
                    let rem = resp.header("X-RateLimit-Remaining").map(|s| s.to_string());
                    let reset = resp.header("X-RateLimit-Reset-In").map(|s| s.to_string());
                    let retry = resp.header("Retry-After").map(|s| s.to_string());
                    let body = resp.into_string().unwrap_or_default();
                    println!(
                        "  LB Radio HTTP ERROR status={} (Remaining={:?}, Reset={:?}, Retry-After={:?}) body={}",
                        code, rem, reset, retry, body
                    );
                }
                Err(e) => {
                    println!("  LB Radio TRANSPORT ERROR: {:?}", e);
                }
            }
        }

        // Now simulate 5 consecutive generate_recommendations runs using a temp copy of mbid_cache
        let temp_cache = std::env::temp_dir().join(format!("retro_diag_mbid_{}.json", std::process::id()));
        if real_cache_path.exists() {
            let _ = std::fs::copy(&real_cache_path, &temp_cache);
        }
        let snapshot = StreamingConfigSnapshot {
            recommendations_per_fetch,
            pool_size: recommendations_per_fetch.max(40),
            listenbrainz_mode,
        };
        println!("\n=== SIMULATING 5 CONSECUTIVE generate_recommendations() CALLS (2s gap) ===");
        for run in 1..=5 {
            let start = Instant::now();
            let res = generate_recommendations(&library, &library.seed_artists(8), &snapshot, &temp_cache);
            match &res {
                Ok(tracks) => {
                    println!("Run #{}: OK -> {} tracks in {:?}", run, tracks.len(), start.elapsed());
                    assert!(!tracks.is_empty(), "Expected non-empty recommendations on run #{}", run);
                }
                Err(e) => {
                    println!("Run #{}: ERR -> {} in {:?}", run, e, start.elapsed());
                    panic!("Expected Ok recommendations on run #{}, got Err: {}", run, e);
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
        let _ = std::fs::remove_file(&temp_cache);
    }
}
