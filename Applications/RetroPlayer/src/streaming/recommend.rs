use log::{debug, warn};
use rand::seq::SliceRandom;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

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

/// Plain-data snapshot of the bits of `Configuration.streaming` that the background
/// thread needs. Built once on the main thread (which can read the `CONFIGURATION`
/// `RwLock` briefly) and then moved into the spawned thread, which must never itself
/// try to acquire that lock across a multi-second network operation.
#[derive(Clone)]
pub struct StreamingConfigSnapshot {
    pub recommendations_per_fetch: usize,
    pub listenbrainz_mode: String,
}

const USER_AGENT: &str =
    "RetroPlayer/0.1 (https://github.com/The-Nice-One/PlaygroundProjects; your-email@example.com)";

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
    let response: MbArtistSearchResponse = ureq::get("https://musicbrainz.org/ws/2/artist")
        .set("User-Agent", USER_AGENT)
        .query("query", &query)
        .query("fmt", "json")
        .call()
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

        let response: MbRecordingResponse = ureq::get("https://musicbrainz.org/ws/2/recording")
            .set("User-Agent", USER_AGENT)
            .timeout(std::time::Duration::from_secs(3))
            .query("query", &query)
            .query("fmt", "json")
            .query("limit", "3")
            .call()
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
) -> anyhow::Result<Vec<(String, String)>> {
    let url = format!(
        "https://api.listenbrainz.org/1/lb-radio/artist/{}",
        seed_mbid
    );
    let response: LbRadioArtistResponse = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .query("mode", mode)
        .query("max_similar_artists", &max_similar_artists.to_string())
        .query(
            "max_recordings_per_artist",
            &max_recordings_per_artist.to_string(),
        )
        .query("pop_begin", "0")
        .query("pop_end", "100")
        .call()?
        .into_json()?;

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
) -> anyhow::Result<HashMap<String, (String, String)>> {
    if recording_mbids.is_empty() {
        return Ok(HashMap::new());
    }

    let joined = recording_mbids.join(",");
    let response: RecordingMetadataResponse =
        ureq::get("https://api.listenbrainz.org/1/metadata/recording/")
            .set("User-Agent", USER_AGENT)
            .query("recording_mbids", &joined)
            .query("inc", "artist")
            .call()
            .map_err(|e| {
                warn!("ListenBrainz Metadata error: {}", e);
                e
            })?
            .into_json()?;

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
    let mut candidates: Vec<RecommendedTrack> = Vec::new();
    let mut rng = rand::rng();

    if seed_artists.is_empty() {
        log::warn!("No seed artists found in library index. Check your filename formats!");
        return Ok(candidates);
    }

    let total_recs = config.recommendations_per_fetch;

    // Safety constraint: Only process up to the number of weights we have defined
    let active_artists: Vec<&String> = seed_artists
        .iter()
        .take(ARTIST_WEIGHT_SLOTS.len())
        .collect();
    let active_weights = &ARTIST_WEIGHT_SLOTS[..active_artists.len()];

    // Sum weights to calculate accurate percentages (in case there are fewer than 5 artists)
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
            Ok(None) | Err(_) => {
                deficit += target_slot; // Roll over the missed slot to the next artist
                continue;
            }
        };

        // Fetch ListenBrainz Radio
        let lb_pairs = fetch_lb_radio_for_artist(&seed_mbid, &config.listenbrainz_mode, 5, 3)
            .unwrap_or_default();

        if lb_pairs.is_empty() {
            deficit += target_slot; // Roll over the missed slot to the next artist
            continue;
        }

        let recording_mbids: Vec<String> = lb_pairs.iter().map(|(_, mbid)| mbid.clone()).collect();
        let titles = fetch_recording_titles(&recording_mbids).unwrap_or_default();

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

    save_mbid_cache(mbid_cache_path, &mbid_cache)?;

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
}
