use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::http::{USER_AGENT, http_agent};

const REQUEST_TIMEOUT_SECS: u64 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SyncGranularity {
    Plain = 0,
    Sentence = 1,
    Word = 2,
    Syllable = 3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimedUnit {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LyricLine {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
    /// Granular sub-units: per-word or per-syllable timings
    pub sub_units: Vec<TimedUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackLyrics {
    pub artist: String,
    pub title: String,
    pub granularity: SyncGranularity,
    pub is_instrumental: bool,
    pub lines: Vec<LyricLine>,
    pub plain_text: String,
    pub source: String,
    pub offset_ms: i64,
}

impl TrackLyrics {
    /// Serializes lyrics into the Enhanced LRC (ELRC) format.
    /// Standard players see standard `[mm:ss.xx] Line` format.
    /// Karaoke / word-sync players parse inline `<mm:ss.xx>` word tags.
    pub fn to_elrc_string(&self) -> String {
        if self.is_instrumental {
            return "[00:00.00] ♪ Instrumental ♪\n".to_string();
        }

        if self.lines.is_empty() {
            return self.plain_text.clone();
        }

        let mut out = String::new();
        if self.offset_ms != 0 {
            out.push_str(&format!("[offset:{}]\n", self.offset_ms));
        }

        for line in &self.lines {
            let line_tag = format_ms(line.start_ms);
            if self.granularity >= SyncGranularity::Word && !line.sub_units.is_empty() {
                out.push_str(&format!("[{}]", line_tag));
                for unit in &line.sub_units {
                    let unit_tag = format_ms(unit.start_ms);
                    out.push_str(&format!(" <{}>{}", unit_tag, unit.text));
                }
                out.push('\n');
            } else {
                out.push_str(&format!("[{}] {}\n", line_tag, line.text));
            }
        }

        out
    }
}

/// Formats milliseconds into the standard `mm:ss.xx` timecode.
pub fn format_ms(ms: u64) -> String {
    let minutes = ms / 60_000;
    let seconds = (ms % 60_000) / 1000;
    let hundredths = (ms % 1000) / 10;
    format!("{:02}:{:02}.{:02}", minutes, seconds, hundredths)
}

/// Parses a timecode string like `[01:23.45]` or `01:23.45` into milliseconds.
pub fn parse_lrc_timestamp(s: &str) -> Option<u64> {
    let trimmed = s
        .trim()
        .trim_matches(|c| c == '[' || c == ']' || c == '<' || c == '>');
    let (min_str, sec_str) = trimmed.split_once(':')?;
    let minutes: u64 = min_str.parse().ok()?;

    let (secs_str, frac_str) = if let Some((s, f)) = sec_str.split_once('.') {
        (s, f)
    } else {
        (sec_str, "0")
    };
    let seconds: u64 = secs_str.parse().ok()?;
    let millis: u64 = match frac_str.len() {
        1 => frac_str.parse::<u64>().ok()? * 100,
        2 => frac_str.parse::<u64>().ok()? * 10,
        3 => frac_str.parse::<u64>().ok()?,
        _ => frac_str[..3].parse::<u64>().ok()?,
    };
    Some(minutes * 60_000 + seconds * 1000 + millis)
}

/// Parses a standard or enhanced LRC text into structured `LyricLine`s.
pub fn parse_lrc_text(content: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();

    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("[offset:") || trimmed.starts_with("[ti:") {
            continue;
        }

        if let Some(close_bracket) = trimmed.find(']') {
            if trimmed.starts_with('[') {
                let time_str = &trimmed[1..close_bracket];
                if let Some(start_ms) = parse_lrc_timestamp(time_str) {
                    let after_tag = trimmed[close_bracket + 1..].trim();

                    // Check for inline word timestamps: <mm:ss.xx> word
                    let mut sub_units = Vec::new();
                    let mut text_buf = String::new();

                    let mut remaining = after_tag;
                    while let Some(open) = remaining.find('<') {
                        let prefix = &remaining[..open];
                        if !prefix.is_empty() {
                            text_buf.push_str(prefix);
                        }

                        if let Some(close) = remaining[open..].find('>') {
                            let tag_content = &remaining[open + 1..open + close];
                            let word_start = parse_lrc_timestamp(tag_content).unwrap_or(start_ms);
                            let rest = &remaining[open + close + 1..];

                            let word_end_idx = rest.find('<').unwrap_or(rest.len());
                            let word_text = &rest[..word_end_idx];

                            if !word_text.is_empty() {
                                text_buf.push_str(word_text);
                                sub_units.push(TimedUnit {
                                    text: word_text.to_string(),
                                    start_ms: word_start,
                                    end_ms: word_start,
                                });
                            }
                            remaining = &rest[word_end_idx..];
                        } else {
                            break;
                        }
                    }

                    let final_text = if !text_buf.is_empty() {
                        text_buf
                    } else {
                        after_tag.to_string()
                    };

                    lines.push(LyricLine {
                        text: final_text,
                        start_ms,
                        end_ms: start_ms,
                        sub_units,
                    });
                }
            }
        }
    }

    lines
}

// ============================================================================
// LRCLIB Provider
// ============================================================================

#[derive(Debug, Deserialize)]
struct LrclibRecord {
    #[serde(default)]
    pub instrumental: bool,
    #[serde(rename = "plainLyrics")]
    pub plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
}

/// Queries LRCLIB's exact get endpoint.
fn fetch_lrclib_get(
    artist: &str,
    title: &str,
    album: Option<&str>,
    duration: Option<f64>,
) -> Option<TrackLyrics> {
    let mut req = http_agent()
        .get("https://lrclib.net/api/get")
        .set("User-Agent", USER_AGENT)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .query("artist_name", artist)
        .query("track_name", title);

    if let Some(alb) = album {
        req = req.query("album_name", alb);
    }
    if let Some(dur) = duration {
        let dur_rounded = format!("{:.0}", dur);
        req = req.query("duration", &dur_rounded);
    }

    let resp: LrclibRecord = req.call().ok()?.into_json().ok()?;
    lrclib_record_to_track_lyrics(resp, artist, title)
}

/// Queries LRCLIB's search endpoint as a fallback.
fn fetch_lrclib_search(artist: &str, title: &str) -> Option<TrackLyrics> {
    let query = format!("{} {}", artist, title);
    let resp: Vec<LrclibRecord> = http_agent()
        .get("https://lrclib.net/api/search")
        .set("User-Agent", USER_AGENT)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .query("q", &query)
        .call()
        .ok()?
        .into_json()
        .ok()?;

    for record in resp {
        if let Some(track) = lrclib_record_to_track_lyrics(record, artist, title) {
            return Some(track);
        }
    }

    None
}

fn lrclib_record_to_track_lyrics(
    record: LrclibRecord,
    artist: &str,
    title: &str,
) -> Option<TrackLyrics> {
    if record.instrumental {
        return Some(TrackLyrics {
            artist: artist.to_string(),
            title: title.to_string(),
            granularity: SyncGranularity::Sentence,
            is_instrumental: true,
            lines: Vec::new(),
            plain_text: "♪ Instrumental ♪".to_string(),
            source: "lrclib".to_string(),
            offset_ms: 0,
        });
    }

    if let Some(synced) = record.synced_lyrics {
        let lines = parse_lrc_text(&synced);
        let has_words = lines.iter().any(|l| !l.sub_units.is_empty());
        let granularity = if has_words {
            SyncGranularity::Word
        } else {
            SyncGranularity::Sentence
        };

        return Some(TrackLyrics {
            artist: artist.to_string(),
            title: title.to_string(),
            granularity,
            is_instrumental: false,
            lines,
            plain_text: record.plain_lyrics.unwrap_or_default(),
            source: "lrclib".to_string(),
            offset_ms: 0,
        });
    }

    if let Some(plain) = record.plain_lyrics {
        if !plain.trim().is_empty() {
            return Some(TrackLyrics {
                artist: artist.to_string(),
                title: title.to_string(),
                granularity: SyncGranularity::Plain,
                is_instrumental: false,
                lines: Vec::new(),
                plain_text: plain,
                source: "lrclib".to_string(),
                offset_ms: 0,
            });
        }
    }

    None
}

// ============================================================================
// Lrcmux Provider (Musixmatch / KuGou Word & Line-Level Aggregator)
// ============================================================================

#[derive(Debug, Deserialize)]
struct LrcmuxResponse {
    #[serde(default)]
    pub meta: Option<LrcmuxMeta>,
    #[serde(default)]
    pub lines: Option<Vec<LrcmuxLineItem>>,
}

#[derive(Debug, Deserialize)]
struct LrcmuxMeta {
    #[serde(default)]
    pub level: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LrcmuxLineItem {
    pub text: String,
    pub start: u64,
    #[serde(default)]
    pub end: Option<u64>,
    #[serde(default)]
    pub words: Option<Vec<LrcmuxWordItem>>,
}

#[derive(Debug, Deserialize)]
struct LrcmuxWordItem {
    pub text: String,
    pub start: u64,
    #[serde(default)]
    pub end: Option<u64>,
}

/// Queries Lrcmux public aggregator for word-level and line-level lyrics.
fn fetch_lrcmux(artist: &str, title: &str) -> Option<TrackLyrics> {
    let resp: LrcmuxResponse = http_agent()
        .get("https://api.lrcmux.dev/get")
        .set("User-Agent", USER_AGENT)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .query("artist", artist)
        .query("title", title)
        .query("sources", "musixmatch,kugou,netease")
        .query("format", "json")
        .call()
        .ok()?
        .into_json()
        .ok()?;

    let line_items = resp.lines?;
    if line_items.is_empty() {
        return None;
    }

    let is_word_level = resp
        .meta
        .as_ref()
        .and_then(|m| m.level.as_deref())
        .map(|lvl| lvl.eq_ignore_ascii_case("word"))
        .unwrap_or(false);

    let mut lines = Vec::new();
    let mut plain_accum = String::new();

    for item in line_items {
        let mut sub_units = Vec::new();
        if let Some(words) = item.words {
            for w in words {
                sub_units.push(TimedUnit {
                    text: w.text,
                    start_ms: w.start,
                    end_ms: w.end.unwrap_or(w.start),
                });
            }
        }

        plain_accum.push_str(&item.text);
        plain_accum.push('\n');

        lines.push(LyricLine {
            text: item.text,
            start_ms: item.start,
            end_ms: item.end.unwrap_or(item.start),
            sub_units,
        });
    }

    let granularity = if is_word_level && lines.iter().any(|l| !l.sub_units.is_empty()) {
        SyncGranularity::Word
    } else {
        SyncGranularity::Sentence
    };

    Some(TrackLyrics {
        artist: artist.to_string(),
        title: title.to_string(),
        granularity,
        is_instrumental: false,
        lines,
        plain_text: plain_accum.trim().to_string(),
        source: "lrcmux".to_string(),
        offset_ms: 0,
    })
}

// ============================================================================
// Multi-Tier Cascade Orchestrator
// ============================================================================

/// Orchestrates the 4-tier lyrics search:
/// Syllable -> Word-by-word -> Sentence/Line -> Plain lyrics.
pub fn fetch_best_lyrics(
    artist: &str,
    title: &str,
    album: Option<&str>,
    duration: Option<f64>,
    _apple_developer_token: Option<&str>,
) -> Option<TrackLyrics> {
    info!("Searching lyrics for '{} - {}'...", artist, title);

    // Tier 1 & 2: Check Lrcmux for word-level sync
    if let Some(lyrics) = fetch_lrcmux(artist, title) {
        if lyrics.granularity >= SyncGranularity::Word {
            info!(
                "Found word-level synced lyrics from {} for '{} - {}'",
                lyrics.source, artist, title
            );
            return Some(lyrics);
        }
    }

    // Tier 2 & 3: Check LRCLIB exact get
    if let Some(lyrics) = fetch_lrclib_get(artist, title, album, duration) {
        if lyrics.is_instrumental || lyrics.granularity >= SyncGranularity::Sentence {
            info!(
                "Found {:?} lyrics from LRCLIB for '{} - {}'",
                lyrics.granularity, artist, title
            );
            return Some(lyrics);
        }
    }

    // Tier 2 & 3: Check LRCLIB search fallback
    if let Some(lyrics) = fetch_lrclib_search(artist, title) {
        if lyrics.is_instrumental || lyrics.granularity >= SyncGranularity::Sentence {
            info!(
                "Found {:?} lyrics from LRCLIB search for '{} - {}'",
                lyrics.granularity, artist, title
            );
            return Some(lyrics);
        }
    }

    // Fallback: Check Lrcmux line-level if available
    if let Some(lyrics) = fetch_lrcmux(artist, title) {
        info!(
            "Found sentence-level lyrics from Lrcmux for '{} - {}'",
            artist, title
        );
        return Some(lyrics);
    }

    warn!("No lyrics found for '{} - {}'", artist, title);
    None
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_ms() {
        assert_eq!(format_ms(0), "00:00.00");
        assert_eq!(format_ms(1500), "00:01.50");
        assert_eq!(format_ms(65430), "01:05.43");
        assert_eq!(format_ms(245670), "04:05.67");
    }

    #[test]
    fn test_parse_lrc_timestamp() {
        assert_eq!(parse_lrc_timestamp("[00:00.00]"), Some(0));
        assert_eq!(parse_lrc_timestamp("00:01.50"), Some(1500));
        assert_eq!(parse_lrc_timestamp("<01:05.43>"), Some(65430));
        assert_eq!(parse_lrc_timestamp("[04:05.67]"), Some(245670));
    }

    #[test]
    fn test_instrumental_formatting() {
        let lyrics = TrackLyrics {
            artist: "Hans Zimmer".to_string(),
            title: "Time".to_string(),
            granularity: SyncGranularity::Sentence,
            is_instrumental: true,
            lines: Vec::new(),
            plain_text: "Instrumental".to_string(),
            source: "lrclib".to_string(),
            offset_ms: 0,
        };

        assert_eq!(lyrics.to_elrc_string(), "[00:00.00] ♪ Instrumental ♪\n");
    }

    #[test]
    fn test_sentence_elrc_formatting() {
        let lyrics = TrackLyrics {
            artist: "Coldplay".to_string(),
            title: "Yellow".to_string(),
            granularity: SyncGranularity::Sentence,
            is_instrumental: false,
            lines: vec![
                LyricLine {
                    text: "Look at the stars".to_string(),
                    start_ms: 33580,
                    end_ms: 35700,
                    sub_units: Vec::new(),
                },
                LyricLine {
                    text: "Look how they shine for you".to_string(),
                    start_ms: 36300,
                    end_ms: 40200,
                    sub_units: Vec::new(),
                },
            ],
            plain_text: "Look at the stars\nLook how they shine for you".to_string(),
            source: "lrclib".to_string(),
            offset_ms: 0,
        };

        let elrc = lyrics.to_elrc_string();
        assert!(elrc.contains("[00:33.58] Look at the stars\n"));
        assert!(elrc.contains("[00:36.30] Look how they shine for you\n"));
    }

    #[test]
    fn test_word_elrc_formatting() {
        let lyrics = TrackLyrics {
            artist: "Coldplay".to_string(),
            title: "Yellow".to_string(),
            granularity: SyncGranularity::Word,
            is_instrumental: false,
            lines: vec![LyricLine {
                text: "Look at the stars".to_string(),
                start_ms: 33580,
                end_ms: 35700,
                sub_units: vec![
                    TimedUnit {
                        text: "Look ".to_string(),
                        start_ms: 33580,
                        end_ms: 33870,
                    },
                    TimedUnit {
                        text: "at ".to_string(),
                        start_ms: 33870,
                        end_ms: 34160,
                    },
                    TimedUnit {
                        text: "the ".to_string(),
                        start_ms: 34160,
                        end_ms: 34450,
                    },
                    TimedUnit {
                        text: "stars".to_string(),
                        start_ms: 34450,
                        end_ms: 35700,
                    },
                ],
            }],
            plain_text: "Look at the stars".to_string(),
            source: "lrcmux".to_string(),
            offset_ms: 0,
        };

        let elrc = lyrics.to_elrc_string();
        assert_eq!(
            elrc,
            "[00:33.58] <00:33.58>Look  <00:33.87>at  <00:34.16>the  <00:34.45>stars\n"
        );
    }

    #[test]
    fn test_parse_lrc_text() {
        let text = "[00:33.58] Look at the stars\n[00:36.30] Look how they shine for you\n";
        let lines = parse_lrc_text(text);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "Look at the stars");
        assert_eq!(lines[0].start_ms, 33580);
        assert_eq!(lines[1].text, "Look how they shine for you");
        assert_eq!(lines[1].start_ms, 36300);
    }
}
