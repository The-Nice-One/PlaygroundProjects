use indexmap::IndexMap;
use rand::rng;
use rand::seq::SliceRandom;
use std::fs::read_dir;

#[derive(Default)]
pub struct PlayerSession {
    pub songs: IndexMap<String, String>,
    pub current_index: usize,
}

impl PlayerSession {
    pub fn add_songs(&mut self, directory: &str) {
        let song_directory = read_dir(directory).unwrap();

        for song in song_directory {
            let full_path = song.as_ref().unwrap().path().to_str().unwrap().to_owned();

            let file_name = song
                .as_ref()
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .to_owned();

            let song_name = file_name
                .as_str()
                .split(".")
                .into_iter()
                .next()
                .unwrap()
                .to_owned();

            self.songs.insert(song_name, full_path);
        }
    }
    pub fn previous(&mut self) {
        if self.current_index == 0 {
            self.current_index = self.songs.len() - 1;
        } else {
            self.current_index -= 1;
        }
    }
    pub fn next(&mut self) {
        if self.current_index >= self.songs.len() - 1 {
            self.current_index = 0;
        } else {
            self.current_index += 1;
        }
    }
    pub fn peek_next(&self) -> Option<(&String, &String)> {
        let index = (self.current_index + 1) % self.songs.len();
        return self.songs.get_index(index);
    }
    pub fn current(&self) -> Option<(&String, &String)> {
        self.songs.get_index(self.current_index)
    }
    pub fn peek_previous(&self) -> Option<(&String, &String)> {
        let index = if self.current_index == 0 {
            self.songs.len() - 1
        } else {
            self.current_index - 1
        };
        self.songs.get_index(index)
    }
    pub fn shuffle(&mut self) {
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
}
