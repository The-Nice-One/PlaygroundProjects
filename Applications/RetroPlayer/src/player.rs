use crate::{logger, logger::*};
use indexmap::IndexMap;
use rand::rng;
use rand::seq::SliceRandom;
use std::fs::read_dir;
use std::path::Path;

pub const ALLOWED_AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "m4a", "aac", "opus", "wma",
];

pub fn is_allowed_audio_file(file_name: &str) -> bool {
    Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| ALLOWED_AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[derive(Default)]
pub struct PlayerSession {
    pub songs: IndexMap<String, String>,
    pub current_index: usize,
}

impl PlayerSession {
    pub fn add_songs(&mut self, directory: &str) {
        let song_directory = match read_dir(directory) {
            Ok(value) => value,
            Err(_) => {
                logger!().add_entry(
                    EntryId::Player,
                    Entry::new(format!("Playlist directory not found: {}", directory)),
                );
                return;
            }
        };

        for song in song_directory {
            let song = match song {
                Ok(s) => s,
                Err(_) => continue,
            };
            let file_type = match song.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            let path_buf = song.path();
            let full_path = match path_buf.to_str() {
                Some(p) => p.to_owned(),
                None => continue,
            };
            let file_name = song.file_name().to_string_lossy().to_string();

            if file_type.is_dir() {
                let lower = file_name.to_lowercase();
                if lower == "recommendations" || lower == "lyrics" {
                    continue;
                }
                self.add_songs(&full_path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            if !is_allowed_audio_file(&file_name) {
                continue;
            }

            let song_name = file_name
                .as_str()
                .split(".")
                .collect::<Vec<&str>>()
                .split_last()
                .unwrap()
                .1
                .join(".")
                .to_owned();

            self.songs.insert(song_name, full_path);
        }

        logger!().add_entry(
            EntryId::Player,
            Entry::new_complete(format!("Loaded {} songs into the player", self.songs.len())),
        );
    }
    pub fn load_playlist(&mut self, directory: &str) {
        self.songs.clear();
        self.current_index = 0;
        if directory.is_empty() {
            logger!().add_entry(
                EntryId::Player,
                Entry::new("No playlist directory configured"),
            );
            return;
        }
        self.add_songs(directory);
    }
    pub fn previous(&mut self) {
        if self.songs.is_empty() {
            return;
        }
        if self.current_index == 0 {
            self.current_index = self.songs.len() - 1;
        } else {
            self.current_index -= 1;
        }
    }
    pub fn next(&mut self) {
        if self.songs.is_empty() {
            return;
        }
        if self.current_index >= self.songs.len() - 1 {
            self.current_index = 0;
        } else {
            self.current_index += 1;
        }
    }
    pub fn peek_next(&self) -> Option<(&String, &String)> {
        if self.songs.is_empty() {
            return None;
        }
        let index = (self.current_index + 1) % self.songs.len();
        self.songs.get_index(index)
    }
    pub fn current(&self) -> Option<(&String, &String)> {
        self.songs.get_index(self.current_index)
    }
    pub fn peek_previous(&self) -> Option<(&String, &String)> {
        if self.songs.is_empty() {
            return None;
        }
        let index = if self.current_index == 0 {
            self.songs.len() - 1
        } else {
            self.current_index - 1
        };
        self.songs.get_index(index)
    }
    pub fn shuffle(&mut self) {
        if self.songs.is_empty() {
            return;
        }
        let current_song_name = self.current().unwrap().0.clone();
        let mut keys: Vec<String> = self.songs.keys().cloned().collect();
        let mut rng = rng();
        keys.shuffle(&mut rng);

        let mut shuffled_map: IndexMap<String, String> = IndexMap::new();
        for key in keys {
            if let Some(song_file) = self.songs.swap_remove(&key) {
                shuffled_map.insert(key, song_file);
            }
        }
        self.songs = shuffled_map;
        self.current_index = self.songs.get_index_of(&current_song_name).unwrap();
    }
    pub fn insert_after(
        &mut self,
        anchor_key: Option<&str>,
        song_name: String,
        path: String,
    ) -> usize {
        let target_index = if let Some(key) = anchor_key {
            if let Some(pos) = self.songs.get_index_of(key) {
                (pos + 1).min(self.songs.len())
            } else {
                (self.current_index + 1).min(self.songs.len())
            }
        } else {
            (self.current_index + 1).min(self.songs.len())
        };

        self.songs.shift_insert(target_index, song_name, path);
        if target_index <= self.current_index && !self.songs.is_empty() {
            self.current_index += 1;
        }
        target_index
    }
    pub fn insert_streaming_jitter(&mut self, song_name: String, path: String) -> usize {
        let slot = if self.songs.len() <= 1 {
            self.songs.len()
        } else {
            use rand::Rng;
            let min_slot = (self.current_index + 2).min(self.songs.len());
            let max_slot = (self.current_index + 4).min(self.songs.len());
            if min_slot >= max_slot {
                min_slot
            } else {
                rng().random_range(min_slot..=max_slot)
            }
        };

        self.songs.shift_insert(slot, song_name, path);
        if slot <= self.current_index && !self.songs.is_empty() {
            self.current_index += 1;
        }
        slot
    }
    pub fn advance_and_remove_current(&mut self) -> Option<(String, String)> {
        if self.songs.is_empty() {
            return None;
        }
        let removed = self.songs.shift_remove_index(self.current_index);
        if self.songs.is_empty() {
            self.current_index = 0;
        } else {
            self.current_index = self.current_index % self.songs.len();
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_allowed_audio_file() {
        assert!(is_allowed_audio_file("track.mp3"));
        assert!(is_allowed_audio_file("track.FLAC"));
        assert!(is_allowed_audio_file("track.wav"));
        assert!(is_allowed_audio_file("track.ogg"));
        assert!(is_allowed_audio_file("track.m4a"));
        assert!(is_allowed_audio_file("track.aac"));
        assert!(is_allowed_audio_file("track.opus"));
        assert!(is_allowed_audio_file("track.wma"));

        assert!(!is_allowed_audio_file("track.lrc"));
        assert!(!is_allowed_audio_file("track.json"));
        assert!(!is_allowed_audio_file("track.txt"));
        assert!(!is_allowed_audio_file("track.png"));
        assert!(!is_allowed_audio_file("no_extension"));
    }

    #[test]
    fn test_add_songs_skips_excluded_dirs_and_non_audio() {
        let temp_dir = std::env::temp_dir().join("retro_player_test_add_songs");
        let recs_dir = temp_dir.join("recommendations");
        let lyrics_dir = temp_dir.join("lyrics");
        std::fs::create_dir_all(&recs_dir).unwrap();
        std::fs::create_dir_all(&lyrics_dir).unwrap();

        std::fs::write(temp_dir.join("Valid 1.mp3"), b"test").unwrap();
        std::fs::write(temp_dir.join("Valid 2.flac"), b"test").unwrap();
        std::fs::write(temp_dir.join("Ignored.lrc"), b"test").unwrap();
        std::fs::write(temp_dir.join("Ignored.json"), b"test").unwrap();
        std::fs::write(recs_dir.join("Rec Song.mp3"), b"test").unwrap();
        std::fs::write(lyrics_dir.join("Valid 1.lrc"), b"test").unwrap();

        let mut session = PlayerSession::default();
        session.add_songs(temp_dir.to_str().unwrap());

        assert_eq!(session.songs.len(), 2);
        assert!(session.songs.contains_key("Valid 1"));
        assert!(session.songs.contains_key("Valid 2"));
        assert!(!session.songs.contains_key("Rec Song"));
        assert!(!session.songs.contains_key("Ignored"));

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_player_shuffle_preserves_songs() {
        let mut player = PlayerSession::default();
        player.songs.insert("A".into(), "1".into());
        player.songs.insert("B".into(), "2".into());
        player.songs.insert("C".into(), "3".into());
        player.current_index = 0;
        let count_before = player.songs.len();
        player.shuffle();
        assert_eq!(player.songs.len(), count_before);
        assert!(player.songs.contains_key("A"));
        assert!(player.songs.contains_key("B"));
        assert!(player.songs.contains_key("C"));
        assert_eq!(player.current().unwrap().0, "A");
    }

    #[test]
    fn test_insert_after_chains_in_order() {
        let mut player = PlayerSession::default();
        player.songs.insert("Track 1".into(), "p1".into());
        player.songs.insert("Track 2".into(), "p2".into());
        player.current_index = 0;

        // Insert Manual A after Track 1
        let pos_a = player.insert_after(Some("Track 1"), "Manual A".into(), "pa".into());
        assert_eq!(pos_a, 1);
        assert_eq!(player.songs.get_index(1).unwrap().0, "Manual A");
        assert_eq!(player.current_index, 0);

        // Insert Manual B after Manual A
        let pos_b = player.insert_after(Some("Manual A"), "Manual B".into(), "pb".into());
        assert_eq!(pos_b, 2);
        assert_eq!(player.songs.get_index(2).unwrap().0, "Manual B");
        assert_eq!(player.songs.get_index(3).unwrap().0, "Track 2");
        assert_eq!(player.current_index, 0);
    }

    #[test]
    fn test_insert_streaming_jitter_placement() {
        let mut player = PlayerSession::default();
        for i in 1..=10 {
            player.songs.insert(format!("Song {}", i), format!("p{}", i));
        }
        player.current_index = 2; // currently at Song 3

        let inserted_idx = player.insert_streaming_jitter("Rec 1".into(), "prec1".into());
        // Must be between current_index + 2 (4) and current_index + 4 (6)
        assert!(inserted_idx >= 4 && inserted_idx <= 6);
        // Never steals immediate next (current_index + 1 = 3)
        assert_ne!(inserted_idx, 3);
        assert_eq!(player.current_index, 2);
    }

    #[test]
    fn test_advance_and_remove_current_middle() {
        let mut player = PlayerSession::default();
        player.songs.insert("A".into(), "pa".into());
        player.songs.insert("Rec 1".into(), "prec1".into());
        player.songs.insert("B".into(), "pb".into());
        player.current_index = 1; // Playing Rec 1

        let removed = player.advance_and_remove_current();
        assert_eq!(removed, Some(("Rec 1".into(), "prec1".into())));
        assert_eq!(player.songs.len(), 2);
        assert_eq!(player.current_index, 1);
        assert_eq!(player.current().unwrap().0, "B");
    }

    #[test]
    fn test_advance_and_remove_current_last() {
        let mut player = PlayerSession::default();
        player.songs.insert("A".into(), "pa".into());
        player.songs.insert("Rec 1".into(), "prec1".into());
        player.current_index = 1; // Playing last song

        let removed = player.advance_and_remove_current();
        assert_eq!(removed, Some(("Rec 1".into(), "prec1".into())));
        assert_eq!(player.songs.len(), 1);
        assert_eq!(player.current_index, 0); // Wrapped to first song
        assert_eq!(player.current().unwrap().0, "A");
    }

    #[test]
    fn test_advance_and_remove_current_single() {
        let mut player = PlayerSession::default();
        player.songs.insert("Only".into(), "ponly".into());
        player.current_index = 0;

        let removed = player.advance_and_remove_current();
        assert_eq!(removed, Some(("Only".into(), "ponly".into())));
        assert_eq!(player.songs.len(), 0);
        assert_eq!(player.current_index, 0);
        assert!(player.current().is_none());
    }
}
