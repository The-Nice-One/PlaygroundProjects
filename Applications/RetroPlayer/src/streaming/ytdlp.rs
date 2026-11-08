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
    #[serde(default)]
    pub uploader: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub track: Option<String>,
}

/// Checks whether a query string represents a URL or a general text query.
pub fn is_url(query: &str) -> bool {
    let trimmed = query.trim();
    trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("www.")
        || trimmed.starts_with("youtube.com/")
        || trimmed.starts_with("youtu.be/")
}

/// Strips typical branding / suffix clutter from YouTube channel names.
pub fn clean_uploader_name(uploader: &str) -> String {
    let mut u = uploader.trim();
    if let Some(stripped) = u.strip_suffix(" - Topic") {
        u = stripped.trim();
    }
    if let Some(stripped) = u.strip_suffix(" VEVO") {
        u = stripped.trim();
    }
    if let Some(stripped) = u.strip_suffix("VEVO") {
        u = stripped.trim();
    }
    if let Some(stripped) = u.strip_suffix(" Official") {
        u = stripped.trim();
    }
    u.to_string()
}

/// Checks whether a title or segment describes a cover, live solo, or instrumental performance.
pub fn is_cover_or_performance(text: &str) -> bool {
    let lower = text.to_lowercase();
    let keywords = [
        "cover",
        "piano cover",
        "street piano",
        "violin cover",
        "guitar cover",
        "drum cover",
        "acoustic cover",
        "remix",
        "tribute",
        "performed by",
    ];
    keywords.iter().any(|k| lower.contains(k))
}

/// Removes common noise and resolution tags from video titles.
pub fn strip_clutter(text: &str) -> String {
    let clutter_patterns = [
        "(official video)",
        "[official video]",
        "(official music video)",
        "[official music video]",
        "(official audio)",
        "[official audio]",
        "(lyrics)",
        "[lyrics]",
        "(lyric video)",
        "[lyric video]",
        "(audio)",
        "[audio]",
        "(visualizer)",
        "[visualizer]",
        "(4k)",
        "[4k]",
        "(hd)",
        "[hd]",
        "(1080p)",
        "[1080p]",
        "(60fps)",
        "[60fps]",
        "(piano cover)",
        "[piano cover]",
        "(violin cover)",
        "[violin cover]",
        "(guitar cover)",
        "[guitar cover]",
        "(cover)",
        "[cover]",
    ];

    let mut result = text.to_string();
    for pattern in clutter_patterns {
        let pattern_lower = pattern.to_lowercase();
        while let Some(pos) = result.to_lowercase().find(&pattern_lower) {
            result.replace_range(pos..pos + pattern.len(), "");
        }
    }
    result.trim().to_string()
}

/// Extracts featured artist mentions (e.g. `ft. X` or `feat. X`) from title text.
pub fn extract_features(text: &str) -> (Vec<String>, String) {
    let lower = text.to_lowercase();
    let markers = [" ft. ", " feat. ", " ft ", " feat ", " featuring "];

    for marker in markers {
        if let Some(idx) = lower.find(marker) {
            let left = text[..idx].trim().to_string();
            let right = &text[idx + marker.len()..];
            let clean_right = right
                .trim_end_matches(|c| c == ')' || c == ']' || c == '}')
                .trim();

            let features: Vec<String> = clean_right
                .split(|c| c == ',' || c == '&')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            return (features, left);
        }
    }

    (vec![], text.to_string())
}

/// Parses artist(s) and track title from a raw YouTube title using multi-segment
/// analysis, uploader deduplication, feature extraction, and cover crediting.
pub fn parse_youtube_title_smart(
    raw_title: &str,
    uploader: Option<&str>,
    channel: Option<&str>,
) -> (Vec<String>, String) {
    let raw_uploader = uploader.or(channel).unwrap_or("");
    let clean_uploader = clean_uploader_name(raw_uploader);
    let mut is_cover = is_cover_or_performance(raw_title);

    // 1. Split into segments using pipe (|), slash (/), bullet (•), or broken bar (¦)
    let segments: Vec<&str> = raw_title
        .split(|c| c == '|' || c == '/' || c == '•' || c == '¦')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut filtered_segments = Vec::new();
    for &seg in &segments {
        let seg_lower = seg.to_lowercase();
        let up_lower = clean_uploader.to_lowercase();

        // Drop segment if it matches the uploader branding (e.g. "YUKI PIANO")
        if !up_lower.is_empty()
            && (seg_lower == up_lower || seg_lower == format!("{} piano", up_lower))
        {
            continue;
        }

        // Drop clickbait/genre prefixes without a song delimiter (e.g. "Soulful Street Piano Cover")
        if is_cover_or_performance(seg) && !seg.contains(" - ") {
            is_cover = true;
            continue;
        }

        filtered_segments.push(seg);
    }

    if filtered_segments.is_empty() {
        filtered_segments = segments.clone();
    }

    // 2. Locate the core musical segment containing " - "
    let core_segment = filtered_segments
        .iter()
        .find(|&&s| s.contains(" - "))
        .copied()
        .unwrap_or(filtered_segments.first().copied().unwrap_or(raw_title));

    // 3. Extract primary artist and title part
    let mut artists: Vec<String> = Vec::new();
    let raw_title_part: String;

    if let Some((left, right)) = core_segment.split_once(" - ") {
        let left_trimmed = left.trim();
        let right_trimmed = right.trim();

        if left_trimmed.eq_ignore_ascii_case("full album")
            || left_trimmed.eq_ignore_ascii_case("soundtrack")
        {
            artists.push(right_trimmed.to_string());
            raw_title_part = left_trimmed.to_string();
        } else {
            for a in left_trimmed.split(|c| c == ',' || c == '&') {
                let trimmed = a.trim();
                if !trimmed.is_empty() {
                    artists.push(trimmed.to_string());
                }
            }
            raw_title_part = right_trimmed.to_string();
        }
    } else {
        raw_title_part = core_segment.to_string();
        if !clean_uploader.is_empty() {
            artists.push(clean_uploader.clone());
        } else {
            artists.push("Unknown Artist".to_string());
        }
    }

    // 4. Extract featured artists from title part (e.g. "Get Lucky ft. Pharrell Williams")
    let (features, title_without_features) = extract_features(&raw_title_part);
    for feat in features {
        if !artists.contains(&feat) {
            artists.push(feat);
        }
    }

    // 5. Strip clutter tags
    let final_title = strip_clutter(&title_without_features);

    // 6. If it is a cover/performance and uploader is distinct, credit performer as co-artist
    if is_cover && !clean_uploader.is_empty() && !artists.contains(&clean_uploader) {
        artists.push(clean_uploader);
    }

    (artists, final_title)
}

/// True if `yt-dlp` resolves on PATH.
pub fn is_available(ytdlp_path: &str) -> bool {
    Command::new(ytdlp_path)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Runs `yt-dlp --dump-json "ytsearch{count}:{query}"`.
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
    }
    Ok(candidates)
}

/// Searches yt-dlp using either a direct URL or a general query, utilizing `--flat-playlist`
/// for near-instant metadata retrieval.
pub fn search_query_or_url(
    ytdlp_path: &str,
    query: &str,
    count: u32,
) -> anyhow::Result<Vec<YtdlpCandidate>> {
    let trimmed = query.trim();
    let mut cmd = Command::new(ytdlp_path);
    cmd.arg("--dump-json")
        .arg("--no-warnings")
        .arg("--flat-playlist");

    if is_url(trimmed) {
        let full_url = if trimmed.starts_with("www.") {
            format!("https://{}", trimmed)
        } else {
            trimmed.to_string()
        };

        if full_url.contains("watch?v=") || full_url.contains("youtu.be/") {
            cmd.arg("--no-playlist").arg(&full_url);
        } else {
            cmd.arg("--playlist-end")
                .arg(count.to_string())
                .arg(&full_url);
        }
    } else {
        let search_term = format!("ytsearch{}:{}", count, trimmed);
        cmd.arg(&search_term);
    }

    let output = cmd.output()?;
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
    }
    Ok(candidates)
}

/// Converts a `YtdlpCandidate` into a `RecommendedTrack`, populating artist and title
/// via the smart multi-segment heuristic parser.
pub fn parse_candidate(candidate: YtdlpCandidate, query: &str) -> RecommendedTrack {
    let (artists, title) = if let (Some(a), Some(t)) = (candidate.artist, candidate.track) {
        if !a.trim().is_empty() && !t.trim().is_empty() {
            (vec![a.trim().to_string()], t.trim().to_string())
        } else {
            parse_youtube_title_smart(
                &candidate.title,
                candidate.uploader.as_deref(),
                candidate.channel.as_deref(),
            )
        }
    } else {
        parse_youtube_title_smart(
            &candidate.title,
            candidate.uploader.as_deref(),
            candidate.channel.as_deref(),
        )
    };

    let source = if is_url(query) {
        "URL".to_string()
    } else {
        "Search".to_string()
    };

    RecommendedTrack {
        artists,
        title,
        source_artist: source,
        recording_mbid: None,
        video_id: Some(candidate.id),
    }
}

/// Picks the best-matching candidate from a search result list for a given artist/title pair.
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

/// Downloads and extracts audio for a single video ID or direct URL to `output_path`.
pub fn download(ytdlp_path: &str, video_id_or_url: &str, output_path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = output_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let output_template = output_path.with_extension("%(ext)s");
    let url = if video_id_or_url.starts_with("http://") || video_id_or_url.starts_with("https://") {
        video_id_or_url.to_string()
    } else {
        format!("https://www.youtube.com/watch?v={}", video_id_or_url)
    };

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

/// Writes artist/title/album tags onto a downloaded file.
pub fn tag_file(
    path: &Path,
    artists: &[String],
    title: &str,
    album: Option<&str>,
    url: &str,
) -> anyhow::Result<()> {
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
    let artist_str = if artists.is_empty() {
        "Unknown Artist".to_string()
    } else {
        artists.join(", ")
    };
    tag.set_artist(artist_str);
    tag.set_title(title.to_string());
    if let Some(alb) = album {
        tag.set_album(alb.to_string());
    } else {
        tag.set_album(title.to_string());
    }

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

/// Builds the search query for a `RecommendedTrack`.
pub fn search_query(track: &RecommendedTrack) -> String {
    if track.artists.is_empty() {
        track.title.clone()
    } else {
        format!("{} - {}", track.artists.join(", "), track.title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_url() {
        assert!(is_url("https://www.youtube.com/watch?v=123"));
        assert!(is_url("http://example.com/audio.mp3"));
        assert!(is_url("www.youtube.com/watch?v=123"));
        assert!(is_url("youtu.be/12345"));
        assert!(!is_url("daft punk get lucky"));
        assert!(!is_url("coldplay - yellow"));
    }

    #[test]
    fn test_parse_candidate_piped_cover() {
        let candidate = YtdlpCandidate {
            id: "vid3e9".to_string(),
            title: "Soulful Street Piano Cover | Jamie Duffy - Solas | YUKI PIANO".to_string(),
            duration: Some(210.0),
            uploader: Some("YUKI PIANO ".to_string()),
            channel: Some("YUKI PIANO ".to_string()),
            artist: None,
            track: None,
        };
        let track = parse_candidate(candidate, "Solas Yuki Piano");
        assert_eq!(
            track.artists,
            vec!["Jamie Duffy".to_string(), "YUKI PIANO".to_string()]
        );
        assert_eq!(track.title, "Solas");
        assert_eq!(track.source_artist, "Search");
        assert_eq!(track.video_id, Some("vid3e9".to_string()));
    }

    #[test]
    fn test_parse_candidate_performer_cover() {
        let candidate = YtdlpCandidate {
            id: "vid_time".to_string(),
            title: "Hans Zimmer - Time | Street Piano | YUKI PIANO".to_string(),
            duration: Some(270.0),
            uploader: Some("YUKI PIANO".to_string()),
            channel: Some("YUKI PIANO".to_string()),
            artist: None,
            track: None,
        };
        let track = parse_candidate(candidate, "Hans Zimmer Time");
        assert_eq!(
            track.artists,
            vec!["Hans Zimmer".to_string(), "YUKI PIANO".to_string()]
        );
        assert_eq!(track.title, "Time");
    }

    #[test]
    fn test_parse_candidate_featured_artists() {
        let candidate = YtdlpCandidate {
            id: "vid_lucky".to_string(),
            title: "Daft Punk - Get Lucky (Official Audio) ft. Pharrell Williams, Nile Rodgers"
                .to_string(),
            duration: Some(248.0),
            uploader: Some("Daft Punk".to_string()),
            channel: Some("Daft Punk".to_string()),
            artist: None,
            track: None,
        };
        let track = parse_candidate(candidate, "get lucky");
        assert_eq!(
            track.artists,
            vec![
                "Daft Punk".to_string(),
                "Pharrell Williams".to_string(),
                "Nile Rodgers".to_string()
            ]
        );
        assert_eq!(track.title, "Get Lucky");
    }

    #[test]
    fn test_parse_candidate_clutter_stripping() {
        let candidate = YtdlpCandidate {
            id: "vid_queen".to_string(),
            title: "Queen - Bohemian Rhapsody (Lyrics)".to_string(),
            duration: Some(355.0),
            uploader: Some("7clouds Rock".to_string()),
            channel: Some("7clouds Rock".to_string()),
            artist: None,
            track: None,
        };
        let track = parse_candidate(candidate, "bohemian rhapsody");
        assert_eq!(track.artists, vec!["Queen".to_string()]);
        assert_eq!(track.title, "Bohemian Rhapsody");
    }

    #[test]
    fn test_parse_candidate_official_video_clutter() {
        let candidate = YtdlpCandidate {
            id: "vid_yellow".to_string(),
            title: "Coldplay - Yellow (Official Video)".to_string(),
            duration: Some(269.0),
            uploader: Some("Coldplay".to_string()),
            channel: Some("Coldplay".to_string()),
            artist: None,
            track: None,
        };
        let track = parse_candidate(candidate, "coldplay yellow");
        assert_eq!(track.artists, vec!["Coldplay".to_string()]);
        assert_eq!(track.title, "Yellow");
    }

    #[test]
    fn test_parse_candidate_topic_channel_fallback() {
        let candidate = YtdlpCandidate {
            id: "vid456".to_string(),
            title: "Viva La Vida".to_string(),
            duration: Some(240.0),
            uploader: Some("Coldplay - Topic".to_string()),
            channel: Some("Coldplay - Topic".to_string()),
            artist: None,
            track: None,
        };
        let track = parse_candidate(candidate, "https://www.youtube.com/watch?v=vid456");
        assert_eq!(track.artists, vec!["Coldplay".to_string()]);
        assert_eq!(track.title, "Viva La Vida");
        assert_eq!(track.source_artist, "URL");
        assert_eq!(track.video_id, Some("vid456".to_string()));
    }
}
