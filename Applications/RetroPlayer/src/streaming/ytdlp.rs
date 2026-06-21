use serde::Deserialize;
use std::path::Path;
use std::process::Command;

use super::recommend::RecommendedTrack;

#[derive(Debug, Clone, Deserialize)]
pub struct YtdlpCandidate {
    pub id: String,
    pub title: String,
    /// Duration in seconds. yt-dlp's `--dump-json` reports this as a number; some
    /// live streams/playlists omit it entirely, so it's optional and treated as 0
    /// (which the matching heuristic then penalizes, rather than panicking).
    #[serde(default)]
    pub duration: Option<f64>,
}

/// True if `yt-dlp` resolves on PATH. Checked once at startup so the rest of the app
/// can simply disable the streaming feature for the session rather than failing
/// confusingly partway through a download.
pub fn is_available(ytdlp_path: &str) -> bool {
    Command::new(ytdlp_path)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Runs `yt-dlp --dump-json "ytsearch{count}:{query}"` and parses one JSON object per
/// line (yt-dlp's `--dump-json` output format: one self-contained JSON document per
/// matched video, newline-separated).
pub fn search(ytdlp_path: &str, query: &str, count: u32) -> anyhow::Result<Vec<YtdlpCandidate>> {
    let search_term = format!("ytsearch{}:{}", count, query);
    let output = Command::new(ytdlp_path)
        .arg("--dump-json")
        .arg("--no-warnings")
        .arg(&search_term)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("yt-dlp search failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut candidates = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(candidate) = serde_json::from_str::<YtdlpCandidate>(line) {
            candidates.push(candidate);
        }
        // A line that doesn't parse is skipped rather than failing the whole search —
        // yt-dlp occasionally interleaves non-JSON warning text even with --no-warnings.
    }
    Ok(candidates)
}

/// Picks the best-matching candidate from a search result list for a given
/// artist/title pair. Title token overlap is the primary signal (artist name is
/// often, but not always, present in the video title); duration outside a plausible
/// instrumental-track runtime (30s-15min) is penalized rather than excluded outright,
/// in case nothing better is available.
pub fn pick_best<'a>(
    candidates: &'a [YtdlpCandidate],
    artist: &str,
    title: &str,
) -> Option<&'a YtdlpCandidate> {
    if candidates.is_empty() {
        return None;
    }

    let title_tokens: Vec<String> = title
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    let artist_tokens: Vec<String> = artist
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let mut best: Option<(&YtdlpCandidate, i64)> = None;

    for candidate in candidates {
        let candidate_lower = candidate.title.to_lowercase();

        let title_matches = title_tokens
            .iter()
            .filter(|t| candidate_lower.contains(t.as_str()))
            .count() as i64;
        let artist_matches = artist_tokens
            .iter()
            .filter(|t| candidate_lower.contains(t.as_str()))
            .count() as i64;

        let mut score = title_matches * 10 + artist_matches * 3;

        let duration = candidate.duration.unwrap_or(0.0);
        if duration < 30.0 || duration > 900.0 {
            score -= 20;
        }

        if best.is_none() || score > best.unwrap().1 {
            best = Some((candidate, score));
        }
    }

    best.map(|(candidate, _)| candidate)
}

/// Downloads and extracts audio for a single video ID to `output_path` (which should
/// already include the desired filename, e.g. ".../Artist - Title.mp3" — yt-dlp's
/// output template uses `%(ext)s` for the extension, which `--audio-format mp3`
/// pins to "mp3").
pub fn download(ytdlp_path: &str, video_id: &str, output_path: &Path) -> anyhow::Result<()> {
    let output_template = output_path.with_extension("%(ext)s");
    let url = format!("https://www.youtube.com/watch?v={}", video_id);

    let output = Command::new(ytdlp_path)
        .arg("-f")
        .arg("ba")
        .arg("-x")
        .arg("--audio-format")
        .arg("mp3")
        .arg("--audio-quality")
        .arg("320")
        .arg("-o")
        .arg(output_template.to_string_lossy().as_ref())
        .arg("--write-thumbnail")
        .arg("--convert-thumbnails")
        .arg("jpg")
        .arg("--no-warnings")
        .arg(&url)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("yt-dlp download failed: {}", stderr);
    }

    Ok(())
}

/// Writes artist/title tags onto a downloaded file so it's self-describing even if
/// inspected outside this app. Verified against lofty 0.21's `Probe`/`TaggedFileExt`/
/// `Accessor`/`TagExt` API.
pub fn tag_file(path: &Path, artists: &[String], title: &str, url: &str) -> anyhow::Result<()> {
    use lofty::config::WriteOptions;
    use lofty::file::TaggedFileExt;
    use lofty::picture::{MimeType, Picture, PictureType};
    use lofty::probe::Probe;
    use lofty::tag::{Accessor, ItemKey, ItemValue, Tag, TagExt, TagItem};

    let mut tagged_file = Probe::open(path)?.read()?;
    let tag_type = tagged_file.primary_tag_type();

    if tagged_file.primary_tag().is_none() {
        tagged_file.insert_tag(Tag::new(tag_type));
    }

    let tag = tagged_file
        .primary_tag_mut()
        .expect("tag was just inserted if missing");
    tag.set_artist(artists.join(", "));
    tag.set_title(title.to_string());
    tag.set_album(title.to_string());

    tag.insert(TagItem::new(
        ItemKey::AudioSourceUrl,
        ItemValue::Locator(url.to_string()),
    ));

    let thumb_path = path.with_extension("jpg");
    if thumb_path.exists() {
        if let Ok(pic_data) = std::fs::read(&thumb_path) {
            let pic = Picture::new_unchecked(
                PictureType::CoverFront,
                Some(MimeType::Jpeg),
                Some(String::new()),
                pic_data,
            );
            tag.push_picture(pic);
        }
        let _ = std::fs::remove_file(thumb_path);
    }

    let mut write_options = WriteOptions::new();
    write_options.use_id3v23(true);
    tag.save_to_path(path, write_options)?;
    Ok(())
}

/// Builds the search query for a `RecommendedTrack`, matching this app's existing
/// "Artist - Title" filename convention.
pub fn search_query(track: &RecommendedTrack) -> String {
    format!("{} - {}", track.artists.join(", "), track.title)
}
