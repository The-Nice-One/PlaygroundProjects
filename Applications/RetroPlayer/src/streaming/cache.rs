use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const CACHE_FILE_NAME: &str = ".download_cache.json";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DownloadCacheEntry {
    pub filename: String,
    pub downloaded_at: u64,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DownloadCache {
    pub entries: Vec<DownloadCacheEntry>,
}

impl DownloadCache {
    pub fn load(recommendations_dir: &Path) -> Self {
        let cache_path = recommendations_dir.join(CACHE_FILE_NAME);
        if let Ok(data) = fs::read_to_string(&cache_path) {
            if let Ok(cache) = serde_json::from_str::<DownloadCache>(&data) {
                return cache;
            }
        }
        Self::default()
    }

    pub fn save(&self, recommendations_dir: &Path) -> anyhow::Result<()> {
        let cache_path = recommendations_dir.join(CACHE_FILE_NAME);
        let json = serde_json::to_string_pretty(self)?;
        fs::write(cache_path, json)?;
        Ok(())
    }

    pub fn reconcile(&mut self, recommendations_dir: &Path) {
        let mut on_disk_files: Vec<String> = Vec::new();
        if let Ok(read_dir) = fs::read_dir(recommendations_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if ext.eq_ignore_ascii_case("mp3") {
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                if !name.starts_with('.') {
                                    on_disk_files.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        let on_disk_set: HashSet<_> = on_disk_files.iter().cloned().collect();
        self.entries.retain(|e| on_disk_set.contains(&e.filename));

        let tracked_set: HashSet<_> = self.entries.iter().map(|e| e.filename.clone()).collect();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for filename in on_disk_files {
            if !tracked_set.contains(&filename) {
                self.entries.push(DownloadCacheEntry {
                    filename,
                    downloaded_at: now,
                });
            }
        }
    }
}

pub fn record_download(recommendations_dir: &Path, filename: &str) -> anyhow::Result<()> {
    if !recommendations_dir.exists() {
        let _ = fs::create_dir_all(recommendations_dir);
    }
    let mut cache = DownloadCache::load(recommendations_dir);
    cache.reconcile(recommendations_dir);
    cache.entries.retain(|e| e.filename != filename);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    cache.entries.push(DownloadCacheEntry {
        filename: filename.to_string(),
        downloaded_at: now,
    });
    cache.save(recommendations_dir)?;
    Ok(())
}

pub fn prune_cache(
    playlist_dir: &Path,
    limit: usize,
    active_paths: &[&str],
) -> anyhow::Result<Vec<String>> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let recommendations_dir = playlist_dir.join("recommendations");
    let lyrics_dir = playlist_dir.join("lyrics");

    if !recommendations_dir.exists() {
        return Ok(Vec::new());
    }

    let mut cache = DownloadCache::load(&recommendations_dir);
    cache.reconcile(&recommendations_dir);

    let mut pruned = Vec::new();
    let mut i = 0;
    while cache.entries.len() > limit && i < cache.entries.len() {
        let entry = &cache.entries[i];
        let audio_path = recommendations_dir.join(&entry.filename);
        let audio_path_str = audio_path.to_string_lossy();

        if active_paths.iter().any(|p| *p == audio_path_str) {
            i += 1;
            continue;
        }

        let filename = entry.filename.clone();
        if audio_path.exists() {
            let _ = fs::remove_file(&audio_path);
        }

        let stem = Path::new(&filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&filename);
        let lrc_path = lyrics_dir.join(format!("{}.lrc", stem));
        if lrc_path.exists() {
            let _ = fs::remove_file(&lrc_path);
        }

        pruned.push(filename);
        cache.entries.remove(i);
    }

    let _ = cache.save(&recommendations_dir);
    Ok(pruned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconcile_and_record() {
        let temp_root = std::env::temp_dir().join(format!("retro_test_reconcile_{}", std::process::id()));
        let rec_dir = temp_root.join("recommendations");
        let _ = fs::create_dir_all(&rec_dir);

        // Pre-create untracked file
        fs::write(rec_dir.join("existing.mp3"), b"mp3").unwrap();

        // Record a new download
        record_download(&rec_dir, "new_song.mp3").unwrap();
        fs::write(rec_dir.join("new_song.mp3"), b"mp3").unwrap();

        let cache = DownloadCache::load(&rec_dir);
        let filenames: Vec<_> = cache.entries.iter().map(|e| e.filename.as_str()).collect();
        assert!(filenames.contains(&"existing.mp3"));
        assert!(filenames.contains(&"new_song.mp3"));

        // If existing.mp3 is deleted externally, reconcile removes it
        fs::remove_file(rec_dir.join("existing.mp3")).unwrap();
        let mut cache2 = DownloadCache::load(&rec_dir);
        cache2.reconcile(&rec_dir);
        let filenames2: Vec<_> = cache2.entries.iter().map(|e| e.filename.as_str()).collect();
        assert!(!filenames2.contains(&"existing.mp3"));
        assert!(filenames2.contains(&"new_song.mp3"));

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_prune_fifo_and_lyrics() {
        let temp_root = std::env::temp_dir().join(format!("retro_test_prune_fifo_{}", std::process::id()));
        let rec_dir = temp_root.join("recommendations");
        let lyrics_dir = temp_root.join("lyrics");
        let _ = fs::create_dir_all(&rec_dir);
        let _ = fs::create_dir_all(&lyrics_dir);

        for i in 1..=4 {
            let song_name = format!("track_{}.mp3", i);
            let lyrics_name = format!("track_{}.lrc", i);
            fs::write(rec_dir.join(&song_name), b"audio").unwrap();
            fs::write(lyrics_dir.join(&lyrics_name), b"lyrics").unwrap();
            record_download(&rec_dir, &song_name).unwrap();
        }

        // Limit is 2. Tracks 1 and 2 should be pruned along with their lyrics
        let pruned = prune_cache(&temp_root, 2, &[]).unwrap();
        assert_eq!(pruned, vec!["track_1.mp3", "track_2.mp3"]);

        assert!(!rec_dir.join("track_1.mp3").exists());
        assert!(!lyrics_dir.join("track_1.lrc").exists());
        assert!(!rec_dir.join("track_2.mp3").exists());
        assert!(!lyrics_dir.join("track_2.lrc").exists());
        assert!(rec_dir.join("track_3.mp3").exists());
        assert!(lyrics_dir.join("track_3.lrc").exists());
        assert!(rec_dir.join("track_4.mp3").exists());
        assert!(lyrics_dir.join("track_4.lrc").exists());

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_prune_skips_active_songs() {
        let temp_root = std::env::temp_dir().join(format!("retro_test_prune_active_{}", std::process::id()));
        let rec_dir = temp_root.join("recommendations");
        let _ = fs::create_dir_all(&rec_dir);

        for i in 1..=3 {
            let song_name = format!("song_{}.mp3", i);
            fs::write(rec_dir.join(&song_name), b"audio").unwrap();
            record_download(&rec_dir, &song_name).unwrap();
        }

        // song_1 is active in player.songs! Limit is 2.
        let active_path = rec_dir.join("song_1.mp3").to_string_lossy().to_string();
        let pruned = prune_cache(&temp_root, 2, &[&active_path]).unwrap();

        // song_1 was skipped; song_2 was pruned instead!
        assert_eq!(pruned, vec!["song_2.mp3"]);
        assert!(rec_dir.join("song_1.mp3").exists());
        assert!(!rec_dir.join("song_2.mp3").exists());
        assert!(rec_dir.join("song_3.mp3").exists());

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_prune_limit_zero_disabled() {
        let temp_root = std::env::temp_dir().join(format!("retro_test_prune_zero_{}", std::process::id()));
        let rec_dir = temp_root.join("recommendations");
        let _ = fs::create_dir_all(&rec_dir);

        for i in 1..=5 {
            let song_name = format!("song_{}.mp3", i);
            fs::write(rec_dir.join(&song_name), b"audio").unwrap();
            record_download(&rec_dir, &song_name).unwrap();
        }

        let pruned = prune_cache(&temp_root, 0, &[]).unwrap();
        assert!(pruned.is_empty());
        assert!(rec_dir.join("song_1.mp3").exists());
        assert!(rec_dir.join("song_5.mp3").exists());

        let _ = fs::remove_dir_all(&temp_root);
    }
}
