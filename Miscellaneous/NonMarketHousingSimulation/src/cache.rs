//! LRU caching and material re-use primitives for GPU memory management.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::pbr::StandardMaterial;
use bevy::prelude::*;

use crate::components::TileCoord;

// Material cache

/// Stores one `Handle<StandardMaterial>` per layer-style key.
#[derive(Resource, Default)]
pub struct MaterialCache {
    cache: HashMap<String, Handle<StandardMaterial>>,
}

impl MaterialCache {
    /// Return a cached handle if one exists for `key`, otherwise call `build`,
    /// insert the result into `Assets<StandardMaterial>`, cache, and return it.
    pub fn get_or_create(
        &mut self,
        key: &str,
        materials: &mut Assets<StandardMaterial>,
        build: impl FnOnce() -> StandardMaterial,
    ) -> Handle<StandardMaterial> {
        self.cache
            .entry(key.to_owned())
            .or_insert_with(|| materials.add(build()))
            .clone()
    }
}

#[derive(Resource, Default)]
pub struct ColorMaterialCache {
    cache: HashMap<String, Handle<ColorMaterial>>,
}

impl ColorMaterialCache {
    pub fn get_or_create(
        &mut self,
        key: &str,
        materials: &mut Assets<ColorMaterial>,
        build: impl FnOnce() -> ColorMaterial,
    ) -> Handle<ColorMaterial> {
        self.cache
            .entry(key.to_owned())
            .or_insert_with(|| materials.add(build()))
            .clone()
    }
}

// Tile LRU cache

/// Tracks loaded tiles in LRU order and signals which tile to evict when
/// `max_tiles` is exceeded.
#[derive(Resource)]
pub struct TileLruCache {
    /// Maximum number of tiles kept in GPU memory simultaneously.
    pub max_tiles: usize,
    /// Maps tile coordinates to their root entity ID for O(1) despawning.
    pub roots: HashMap<TileCoord, Entity>,
    /// Front = LRU (oldest), back = MRU (newest).
    order: VecDeque<TileCoord>,
    loaded: HashSet<TileCoord>,
    /// Tiles whose load request is in-flight. Stored to prevent duplicate fetches.
    in_flight: HashSet<TileCoord>,
}

impl TileLruCache {
    pub fn new(max_tiles: usize) -> Self {
        Self {
            max_tiles,
            roots: HashMap::with_capacity(max_tiles + 1),
            order: VecDeque::with_capacity(max_tiles + 1),
            loaded: HashSet::with_capacity(max_tiles + 1),
            in_flight: HashSet::new(),
        }
    }

    // In-flight tracking

    /// Returns `true` if the tile should be fetched, and is thus not loaded or
    /// in-flight.  Marks it as in-flight as a side effect.
    pub fn begin_load(&mut self, coord: TileCoord) -> bool {
        if self.loaded.contains(&coord) || self.in_flight.contains(&coord) {
            return false;
        }
        self.in_flight.insert(coord);
        true
    }

    // LRU access

    /// Called when a tile finishes loading.
    /// Returns `Some(evicted_coord)` if an old tile should be despawned.
    pub fn finish_load(&mut self, coord: TileCoord, root: Entity) -> Option<TileCoord> {
        self.in_flight.remove(&coord);

        if self.loaded.contains(&coord) {
            // Tile already present, just refresh MRU order
            self.order.retain(|c| *c != coord);
            self.order.push_back(coord);
            self.roots.insert(coord, root);
            return None;
        }

        self.loaded.insert(coord);
        self.roots.insert(coord, root);
        self.order.push_back(coord);

        if self.loaded.len() > self.max_tiles {
            let evicted = self.order.pop_front().unwrap();
            self.loaded.remove(&evicted);
            Some(evicted)
        } else {
            None
        }
    }

    /// Mark a tile as manually unloaded
    pub fn remove(&mut self, coord: &TileCoord) {
        self.loaded.remove(coord);
        self.in_flight.remove(coord);
        self.roots.remove(coord);
        self.order.retain(|c| c != coord);
    }

    // Queries

    pub fn is_loaded(&self, coord: &TileCoord) -> bool {
        self.loaded.contains(coord)
    }
    pub fn is_in_flight(&self, coord: &TileCoord) -> bool {
        self.in_flight.contains(coord)
    }
}

impl Default for TileLruCache {
    fn default() -> Self {
        Self::new(64)
    }
}
