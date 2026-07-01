//! Distance-based visibility culling for buildings and map layers.

use bevy::prelude::*;

use crate::components::{BuildingCenter, BuildingProxyMarker, LotData, MapCamera2d, MapLayer};
use crate::plugin::LodOptimizationEnabled;
use crate::streaming::CameraState;

/// Distance at which individual interactive building proxies are hidden.
pub const PROXY_CULL_RADIUS: f32 = 240.0;
/// Distance at which small buildings are hidden.
pub const SMALL_BUILDING_CULL_RADIUS: f32 = 80.0;
/// Height threshold in meters to distinguish small buildings.
pub const SMALL_BUILDING_HEIGHT_THRESHOLD: f32 = 15.0;

/// Distance at which merged tile layers like roads, parks, or generic buildings are hidden.
pub const MAP_LAYER_CULL_RADIUS: f32 = 360.0;
/// Distance at which minor decorative layers like parks, footpaths, or landuse are hidden.
pub const MINOR_LAYER_CULL_RADIUS: f32 = 200.0;

const CULL_MOVE_THRESHOLD_SQ: f32 = 4.0 * 4.0;

/// Toggles visibility of individual building proxies based on distance to camera.
/// This significantly reduces draw calls and ECS overhead when zoomed out.
pub fn cull_building_proxies_system(
    camera_q: Query<
        (&GlobalTransform, Option<&MapCamera2d>),
        Or<(With<Camera3d>, With<MapCamera2d>)>,
    >,
    mut proxy_q: Query<
        (
            &GlobalTransform,
            &BuildingCenter,
            &mut Visibility,
            Option<&LotData>,
        ),
        With<BuildingProxyMarker>,
    >,
    mut state: ResMut<CameraState>,
    lod: Res<LodOptimizationEnabled>,
    new_proxies: Query<(), Added<BuildingProxyMarker>>,
) {
    let Ok((cam_tf, is_2d)) = camera_q.single() else {
        return;
    };
    let cam_pos = map_camera_position(cam_tf, is_2d.is_some());
    let force_refresh = lod.is_changed() || !new_proxies.is_empty();

    if !lod.0 {
        for (_, _, mut vis, _) in &mut proxy_q {
            if *vis != Visibility::Inherited {
                *vis = Visibility::Inherited;
            }
        }
        state.last_cull_pos = cam_pos;
        return;
    }

    let moved_sq = cam_pos.distance_squared(state.last_cull_pos);
    if !force_refresh && moved_sq < CULL_MOVE_THRESHOLD_SQ {
        return;
    }
    state.last_cull_pos = cam_pos;

    let radius_sq = PROXY_CULL_RADIUS * PROXY_CULL_RADIUS;
    let small_radius_sq = SMALL_BUILDING_CULL_RADIUS * SMALL_BUILDING_CULL_RADIUS;

    for (gtf, center, mut vis, lot) in &mut proxy_q {
        // World position is the tile offset (GlobalTransform) + building local center.
        let world_pos = map_entity_position(gtf.translation() + center.0, is_2d.is_some());
        let dist_sq = cam_pos.distance_squared(world_pos);

        // Determine specific cull radius based on building height.
        let height = lot.map(|l| l.height_m).unwrap_or(20.0);
        let limit_sq = if height < SMALL_BUILDING_HEIGHT_THRESHOLD {
            small_radius_sq
        } else {
            radius_sq
        };

        if dist_sq > limit_sq {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        } else {
            if *vis != Visibility::Inherited {
                *vis = Visibility::Inherited;
            }
        }
    }
}

/// Toggles visibility of merged tile meshes (MapLayer) based on distance.
pub fn cull_map_layers_system(
    camera_q: Query<
        (&GlobalTransform, Option<&MapCamera2d>),
        Or<(With<Camera3d>, With<MapCamera2d>)>,
    >,
    mut layer_q: Query<(&GlobalTransform, &MapLayer, &mut Visibility)>,
    mut state: ResMut<CameraState>,
    lod: Res<LodOptimizationEnabled>,
    new_layers: Query<(), Added<MapLayer>>,
) {
    let Ok((cam_tf, is_2d)) = camera_q.single() else {
        return;
    };
    let cam_pos = map_camera_position(cam_tf, is_2d.is_some());
    let force_refresh = lod.is_changed() || !new_layers.is_empty();

    if !lod.0 {
        for (_, _, mut vis) in &mut layer_q {
            if *vis != Visibility::Inherited {
                *vis = Visibility::Inherited;
            }
        }
        state.last_cull_pos = cam_pos;
        return;
    }

    let moved_sq = cam_pos.distance_squared(state.last_cull_pos);
    if !force_refresh && moved_sq < CULL_MOVE_THRESHOLD_SQ {
        return;
    }

    let radius_sq = MAP_LAYER_CULL_RADIUS * MAP_LAYER_CULL_RADIUS;
    let minor_radius_sq = MINOR_LAYER_CULL_RADIUS * MINOR_LAYER_CULL_RADIUS;

    for (gtf, layer, mut vis) in &mut layer_q {
        // water is visible as far as possible to avoid voids.
        if layer.0 == "water" {
            continue;
        }

        // Decorative layers and minor footpaths are culled earlier to keep frame rates high in dense areas.
        let limit_sq = if matches!(
            layer.0.as_str(),
            "park" | "landuse" | "landcover" | "transportation_minor"
        ) {
            minor_radius_sq
        } else {
            radius_sq
        };

        let world_pos = map_entity_position(gtf.translation(), is_2d.is_some());
        let dist_sq = cam_pos.distance_squared(world_pos);
        if dist_sq > limit_sq {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        } else {
            if *vis != Visibility::Inherited {
                *vis = Visibility::Inherited;
            }
        }
    }
}

fn map_camera_position(cam_tf: &GlobalTransform, is_2d: bool) -> Vec3 {
    let cam = cam_tf.translation();
    if is_2d {
        Vec3::new(cam.x, cam.y, 0.0)
    } else {
        cam
    }
}

fn map_entity_position(pos: Vec3, is_2d: bool) -> Vec3 {
    if is_2d {
        Vec3::new(pos.x, pos.y, 0.0)
    } else {
        pos
    }
}
