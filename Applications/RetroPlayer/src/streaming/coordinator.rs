use std::collections::HashSet;
use super::recommend::{RecommendedTrack, artists_match};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeightedCandidate {
    pub track: RecommendedTrack,
    pub weight: u32,
}

#[derive(Default)]
pub struct StreamingCoordinator {
    pub candidate_pool: Vec<WeightedCandidate>,
    pub session_played: HashSet<String>,
    pub songs_played_since_refresh: usize,
    pub is_downloading: bool,
}

impl StreamingCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merges newly fetched recommendations into the candidate pool,
    /// assigning weights based on seed artist representation in the library.
    pub fn add_candidates(
        &mut self,
        tracks: Vec<RecommendedTrack>,
        seed_artists: &[String],
    ) {
        const ARTIST_WEIGHT_SLOTS: [u32; 5] = [50, 30, 10, 5, 2];

        for track in tracks {
            let song_name = track.song_name();
            // Deduplicate against existing candidate pool or songs already played this session
            if self.session_played.contains(&song_name)
                || self.candidate_pool.iter().any(|c| c.track.song_name() == song_name)
            {
                continue;
            }

            // Assign weight based on seed artist ranking
            let artist_rank = seed_artists
                .iter()
                .position(|a| artists_match(a, &track.source_artist));
            let weight = match artist_rank {
                Some(rank) if rank < ARTIST_WEIGHT_SLOTS.len() => ARTIST_WEIGHT_SLOTS[rank],
                _ => 5,
            };

            self.candidate_pool.push(WeightedCandidate { track, weight });
        }
    }

    /// Picks a candidate track using weighted random sampling, removing it from the pool.
    /// Excludes any tracks where `is_in_playlist` returns true or already played this session.
    pub fn pick_candidate<F>(&mut self, is_in_playlist: F) -> Option<RecommendedTrack>
    where
        F: Fn(&str) -> bool,
    {
        // Purge any candidates that are already present in the playlist or played this session
        self.candidate_pool
            .retain(|c| !is_in_playlist(&c.track.song_name()) && !self.session_played.contains(&c.track.song_name()));

        if self.candidate_pool.is_empty() {
            return None;
        }

        let total_weight: u32 = self.candidate_pool.iter().map(|c| c.weight).sum();
        if total_weight == 0 {
            return Some(self.candidate_pool.swap_remove(0).track);
        }

        use rand::Rng;
        let mut rng = rand::rng();
        let mut choice = rng.random_range(0..total_weight);

        for i in 0..self.candidate_pool.len() {
            let w = self.candidate_pool[i].weight;
            if choice < w {
                return Some(self.candidate_pool.swap_remove(i).track);
            }
            choice -= w;
        }

        Some(self.candidate_pool.swap_remove(0).track)
    }

    /// Marks an auto-streamed song as played for this session so it won't be re-picked.
    pub fn mark_played(&mut self, song_name: &str) {
        self.session_played.insert(song_name.to_string());
    }

    /// Increments the played counter and returns `true` if it's time to refresh recommendations (every 3 songs).
    pub fn on_song_played(&mut self) -> bool {
        self.songs_played_since_refresh += 1;
        if self.songs_played_since_refresh >= 3 {
            self.songs_played_since_refresh = 0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_candidates_and_weights() {
        let mut coordinator = StreamingCoordinator::default();
        let seeds = vec!["Daft Punk".to_string(), "Gorillaz".to_string()];
        let tracks = vec![
            RecommendedTrack {
                artists: vec!["Daft Punk".into()],
                title: "One More Time".into(),
                source_artist: "Daft Punk".into(),
                recording_mbid: None,
                video_id: None,
            },
            RecommendedTrack {
                artists: vec!["Gorillaz".into()],
                title: "Feel Good Inc".into(),
                source_artist: "Gorillaz".into(),
                recording_mbid: None,
                video_id: None,
            },
            RecommendedTrack {
                artists: vec!["Other".into()],
                title: "Indie Track".into(),
                source_artist: "Other".into(),
                recording_mbid: None,
                video_id: None,
            },
        ];

        coordinator.add_candidates(tracks, &seeds);
        assert_eq!(coordinator.candidate_pool.len(), 3);
        assert_eq!(coordinator.candidate_pool[0].weight, 50); // Rank 0
        assert_eq!(coordinator.candidate_pool[1].weight, 30); // Rank 1
        assert_eq!(coordinator.candidate_pool[2].weight, 5);  // Fallback
    }

    #[test]
    fn test_pick_candidate_removes_from_pool() {
        let mut coordinator = StreamingCoordinator::default();
        let seeds = vec!["Daft Punk".to_string()];
        let tracks = vec![
            RecommendedTrack {
                artists: vec!["Daft Punk".into()],
                title: "Around the World".into(),
                source_artist: "Daft Punk".into(),
                recording_mbid: None,
                video_id: None,
            },
        ];
        coordinator.add_candidates(tracks, &seeds);
        assert_eq!(coordinator.candidate_pool.len(), 1);

        let picked = coordinator.pick_candidate(|_| false);
        assert!(picked.is_some());
        assert_eq!(picked.unwrap().title, "Around the World");
        assert_eq!(coordinator.candidate_pool.len(), 0);
    }

    #[test]
    fn test_on_song_played_trigger() {
        let mut coordinator = StreamingCoordinator::default();
        assert_eq!(coordinator.on_song_played(), false); // 1
        assert_eq!(coordinator.on_song_played(), false); // 2
        assert_eq!(coordinator.on_song_played(), true);  // 3 -> triggers refresh!
        assert_eq!(coordinator.on_song_played(), false); // 1
    }

    #[test]
    fn test_session_played_filter() {
        let mut coordinator = StreamingCoordinator::default();
        let seeds = vec!["Daft Punk".to_string()];
        let tracks = vec![
            RecommendedTrack {
                artists: vec!["Daft Punk".into()],
                title: "One More Time".into(),
                source_artist: "Daft Punk".into(),
                recording_mbid: None,
                video_id: None,
            },
        ];

        coordinator.mark_played("Daft Punk - One More Time");
        coordinator.add_candidates(tracks, &seeds);
        // Track was already played this session, so it should be ignored!
        assert_eq!(coordinator.candidate_pool.len(), 0);
        assert!(coordinator.pick_candidate(|_| false).is_none());
    }
}
