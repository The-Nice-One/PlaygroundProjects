use indexmap::IndexMap;
use std::collections::HashSet;

/// Splits a library song key of the form "Artist[, Artist2, ...] - Title" (the
/// convention this app already uses for `PlayerSession.songs` keys, derived from
/// filenames minus extension) into its artist list and title.
///
/// Filenames with no " - " separator are treated as title-only with no artists,
/// rather than panicking or guessing.
pub fn parse_song_name(song_name: &str) -> (Vec<String>, String) {
    let delimiter = if song_name.contains(" - ") {
        " - "
    } else {
        "-"
    };

    if let Some((artists_part, title)) = song_name.split_once(delimiter) {
        let artists = artists_part
            .split(',')
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty())
            .collect();
        (artists, title.trim().to_string())
    } else {
        (vec![], song_name.trim().to_string())
    }
}

/// An in-memory index over the local library, built fresh whenever it's needed
/// (cheap: pure string work, no I/O) so it's always in sync with `PlayerSession.songs`.
pub struct LibraryIndex {
    /// (artist_lowercase, title_lowercase) pairs already present locally, used to
    /// filter out recommendations that duplicate something the user already has.
    known_tracks: HashSet<(String, String)>,
    /// artist name (original casing) -> number of tracks by that artist in the library.
    artist_counts: Vec<(String, u32)>,
}

impl LibraryIndex {
    /// Builds an index from any iterator of song-name strings (keys of the form
    /// "Artist - Title"). Generic over the iterator so it's easy to unit test without
    /// needing a real `PlayerSession`.
    pub fn build<'a, I: IntoIterator<Item = &'a str>>(song_names: I) -> Self {
        let mut known_tracks = HashSet::new();
        let mut counts: Vec<(String, u32)> = Vec::new();

        for song_name in song_names {
            let (artists, title) = parse_song_name(song_name);
            for artist in &artists {
                known_tracks.insert((artist.to_lowercase(), title.to_lowercase()));

                if let Some(existing) = counts.iter_mut().find(|(name, _)| name == artist) {
                    existing.1 += 1;
                } else {
                    counts.push((artist.clone(), 1));
                }
            }
        }

        Self {
            known_tracks,
            artist_counts: counts,
        }
    }

    pub fn artist_count(&self) -> usize {
        self.artist_counts.len()
    }

    /// Convenience constructor matching `PlayerSession.songs: IndexMap<String, String>`
    /// (song name -> file path).
    pub fn from_song_keys(songs: &IndexMap<String, String>) -> Self {
        Self::build(songs.keys().map(|s| s.as_str()))
    }

    pub fn contains(&self, artist: &str, title: &str) -> bool {
        self.known_tracks
            .contains(&(artist.to_lowercase(), title.to_lowercase()))
    }

    /// Picks the `n` most-represented artists in the library as recommendation seeds,
    /// most-tracks-first. Ties keep original insertion order (stable sort).
    pub fn seed_artists(&self, n: usize) -> Vec<String> {
        let mut sorted = self.artist_counts.clone();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.into_iter().take(n).map(|(name, _)| name).collect()
    }
}
