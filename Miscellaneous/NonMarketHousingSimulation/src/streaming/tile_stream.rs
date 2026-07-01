//! Tile coordinate request generation based on camera movement.

use bevy::prelude::*;

use crate::cache::TileLruCache;
use crate::components::{MapCamera2d, MapCamera3d, TileCoord};
use crate::events::LoadTileEvent;
use crate::plugin::MapConfig;

pub const LOAD_RADIUS: i64 = 1;

/// Fires `LoadTileEvent` for the initial 5×5 grid around the configured origin.
pub fn request_initial_tiles(config: Res<MapConfig>, mut events: MessageWriter<LoadTileEvent>) {
    let origin = config.origin_tile();
    for dy in -LOAD_RADIUS..=LOAD_RADIUS {
        for dx in -LOAD_RADIUS..=LOAD_RADIUS {
            let x = (origin.x as i64 + dx).max(0) as u32;
            let y = (origin.y as i64 + dy).max(0) as u32;
            events.write(LoadTileEvent {
                coord: TileCoord::new(origin.z, x, y),
            });
        }
    }
}

/// Fires `LoadTileEvent` for any tile within `LOAD_RADIUS` of the camera each frame.
pub fn camera_tile_streaming_system(
    camera_q: Query<
        (&GlobalTransform, Option<&MapCamera2d>),
        Or<(With<MapCamera3d>, With<MapCamera2d>)>,
    >,
    config: Res<MapConfig>,
    cache: Res<TileLruCache>,
    mut events: MessageWriter<LoadTileEvent>,
) {
    let Ok((cam_tf, is_2d)) = camera_q.single() else {
        return;
    };
    let cam = cam_tf.translation();

    let tile_size = config.tile_world_size();
    if tile_size < 1e-6 {
        return;
    }

    let origin = config.origin_tile();

    let map_x = cam.x;
    let map_z = if is_2d.is_some() { -cam.y } else { cam.z };

    let cx = origin.x as i64 + (map_x / tile_size).floor() as i64;
    let cy = origin.y as i64 + (map_z / tile_size).floor() as i64;

    let mut requested = 0;
    for dy in -LOAD_RADIUS..=LOAD_RADIUS {
        for dx in -LOAD_RADIUS..=LOAD_RADIUS {
            let tx = cx + dx;
            let ty = cy + dy;
            if tx < 0 || ty < 0 || tx > 65535 || ty > 65535 {
                continue;
            }
            let coord = TileCoord::new(origin.z, tx as u32, ty as u32);
            if !cache.is_loaded(&coord) && !cache.is_in_flight(&coord) {
                events.write(LoadTileEvent { coord });
                requested += 1;
            }
        }
    }

    if requested > 0 {
        info!(
            "camera_tile_streaming_system: requested {} tiles. cam_x={}, cam_z={}, is_2d={}",
            requested,
            map_x,
            map_z,
            is_2d.is_some()
        );
    }
}
