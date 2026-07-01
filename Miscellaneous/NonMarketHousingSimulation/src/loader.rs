//! Tile fetching, decoding, and batched entity spawning.

use bevy::picking::Pickable;
use bevy::prelude::*;
use flate2::bufread::GzDecoder;
use mvt_reader::Reader;
use std::collections::VecDeque;
use std::io::Read;
use std::sync::{Arc, Mutex};

use crate::building_colors::{
    self, MAT_BUILDING_UNIFIED, MAT_PROXY_INVISIBLE, proxy_invisible_material, tier_base_material,
    tier_material_key, unified_building_material,
};
use crate::cache::{ColorMaterialCache, MaterialCache, TileLruCache};
use crate::components::{
    BaseMaterial2d, BaseMaterial3d, BuildingAabb2D, BuildingCenter, BuildingProxyMarker, MapLayer,
    OwnershipTier, PlayerConverted, Selectable, TileCoord, TileRoot,
};
use crate::events::{LoadTileEvent, UnloadTileEvent};
use crate::lot_database::{BinTiers, LotDatabase};
use crate::plugin::{MapConfig, RenderMode};
use crate::renderer::is_rail_class;
use crate::renderer::{BuildingProxy, RendererRegistry, RendererSnapshot};
use crate::style::layer_style;
use crate::types::{ParsedFeature, convert_mvt_value};

/// Dyn-compatible tile source — uses `Pin<Box<dyn Future>>` instead of `async fn`
/// so the trait can be object-safe.
pub trait TileSource: Send + Sync + 'static {
    fn get_tile(
        &self,
        z: u8,
        x: u32,
        y: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<Vec<u8>>, String>> + Send>>;
}

// Bevy resources

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_task<F>(fut: F, handle: Option<&TokioHandle>)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    handle.expect("TokioHandle missing on native").0.spawn(fut);
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_task<F>(fut: F, _handle: Option<&TokioHandle>)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(fut);
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Clone)]
pub struct TokioHandle(pub tokio::runtime::Handle);

#[cfg(target_arch = "wasm32")]
#[derive(Resource, Clone)]
pub struct TokioHandle;

#[derive(Resource, Clone)]
pub struct TileQueue(pub Arc<Mutex<VecDeque<TileRenderData>>>);

#[derive(Resource, Clone)]
pub struct ArcTileSource(pub Arc<dyn TileSource>);

impl ArcTileSource {
    pub fn new(s: impl TileSource) -> Self {
        Self(Arc::new(s))
    }
}

/// Yields control back to the browser's main thread to prevent blocking the render loop.
#[cfg(target_arch = "wasm32")]
async fn yield_to_browser() {
    use js_sys::Promise;
    use send_wrapper::SendWrapper;
    use wasm_bindgen_futures::JsFuture;
    let _ = SendWrapper::new(JsFuture::from(Promise::resolve(
        &wasm_bindgen::JsValue::UNDEFINED,
    )))
    .await;
}

// Tile render data

/// Produced entirely on the tokio worker and received by the Bevy main thread.
pub struct TileRenderData {
    pub coord: TileCoord,
    /// Non-building layers like roads, water, parks, etc.
    pub layer_meshes: Vec<LayerMeshPayload>,
    /// Unified building mesh for the whole tile.
    pub building_mesh: Option<Mesh>,
    /// Per-building AABB data for invisible picking cuboids.
    pub building_proxies: Vec<BuildingProxy>,
    /// Render state for mode-switching verification.
    pub mode: RenderMode,
}

pub struct LayerMeshPayload {
    pub layer: String,
    pub mesh: Mesh,
    pub mat_key: String,
}

// Load request system

pub fn handle_load_events_system(
    mut events: MessageReader<LoadTileEvent>,
    source: Res<ArcTileSource>,
    handle: Res<TokioHandle>,
    queue: Res<TileQueue>,
    registry: Res<RendererRegistry>,
    config: Res<MapConfig>,
    bin_tiers: Res<BinTiers>,
    mode: Res<State<RenderMode>>,
    mut cache: ResMut<TileLruCache>,
    camera_q: Query<&GlobalTransform, Or<(With<Camera3d>, With<Camera2d>)>>,
) {
    let snapshot = registry.snapshot(config.clone(), Arc::clone(&bin_tiers.0), mode.get().clone());

    let mut requests: Vec<(TileCoord, f32)> = events
        .read()
        .filter_map(|ev| {
            if cache.begin_load(ev.coord) {
                Some((ev.coord, f32::MAX))
            } else {
                None
            }
        })
        .collect();

    if requests.is_empty() {
        return;
    }

    if let Ok(cam_tf) = camera_q.single() {
        let cam_xz = Vec2::new(cam_tf.translation().x, cam_tf.translation().z);
        for (coord, dist) in &mut requests {
            *dist = cam_xz.distance(config.tile_world_xz(*coord));
        }
        requests.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    }

    for (coord, _) in requests {
        let src = Arc::clone(&source.0);
        let q = queue.0.clone();
        let snap = snapshot.clone();

        spawn_task(
            async move {
                match src.get_tile(coord.z, coord.x, coord.y).await {
                    Ok(Some(raw)) => {
                        #[cfg(target_arch = "wasm32")]
                        yield_to_browser().await; // Give the render loop a frame before heavy work

                        match decode_mesh_tile(coord, raw, &snap) {
                            Ok(data) => {
                                q.lock().unwrap().push_back(data);
                            }
                            Err(e) => warn!("Mesh build error for {coord}: {e}"),
                        }
                    }
                    Ok(None) => info!("Tile {coord} not present in source"),
                    Err(e) => warn!("Fetch error for {coord}: {e}"),
                }
            },
            Some(&handle),
        );
    }
}

// Main-thread spawn system

pub fn process_loaded_tiles_system(
    queue: Res<TileQueue>,
    config: Res<MapConfig>,
    lot_db: Res<LotDatabase>,
    bin_tiers: Res<BinTiers>,
    mode: Res<State<RenderMode>>,
    mut cache: ResMut<TileLruCache>,
    mut mat_cache: ResMut<MaterialCache>,
    mut col_cache: ResMut<ColorMaterialCache>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut col_mats: ResMut<Assets<ColorMaterial>>,
    mut unload_ev: MessageWriter<UnloadTileEvent>,
) {
    const MAX_ENTITIES_PER_FRAME: usize = 400;
    let mut entity_budget = MAX_ENTITIES_PER_FRAME;
    let mut q = queue.0.lock().unwrap();

    while entity_budget > 0 {
        let data = match q.pop_front() {
            Some(d) => d,
            None => break,
        };

        // Safety check to avoid instantiating out-of-sync tile data
        // constructed during camera layout switching.
        if data.mode != *mode.get() {
            cache.remove(&data.coord);
            continue;
        }

        // Estimate entities
        let estimated = 1
            + data.layer_meshes.len()
            + (if data.building_mesh.is_some() { 1 } else { 0 })
            + data.building_proxies.len() * 2;

        if estimated > entity_budget && entity_budget < MAX_ENTITIES_PER_FRAME {
            q.push_front(data);
            break;
        }
        entity_budget = entity_budget.saturating_sub(estimated);

        let coord = data.coord;
        let mut offset = config.tile_world_offset(coord);
        if *mode.get() == RenderMode::Mode2D {
            offset = Vec3::new(offset.x, -offset.z, 0.0);
        }
        let tile_tf = Transform::from_translation(offset);

        // Tile root
        let root_entity = commands
            .spawn((
                TileRoot,
                coord,
                tile_tf,
                Visibility::default(),
                InheritedVisibility::default(),
            ))
            .id();

        if let Some(evicted) = cache.finish_load(coord, root_entity) {
            unload_ev.write(UnloadTileEvent { coord: evicted });
        }

        commands.entity(root_entity).with_children(|parent| {
            if *mode.get() == RenderMode::Mode2D {
                info!("process_loaded_tiles_system: Spawning 2D elements for tile {}. Layers: {}, Proxies: {}", coord, data.layer_meshes.len(), data.building_proxies.len());
                let white_mat = col_cache.get_or_create("base_2d", &mut col_mats, || {
                    ColorMaterial { color: Color::WHITE, ..default() }
                });

                for payload in data.layer_meshes {
                    // 2D Layer z-ordering
                    let z = match payload.layer.as_str() {
                        "landcover"                    => 1.0,
                        "landuse"                      => 2.0,
                        "park"                         => 3.0,
                        "water" | "waterway"           => 4.0,
                        "transportation"               => 5.0,
                        "transportation_minor"         => 5.1,
                        "transportation_rail"          => 5.5,
                        "aeroway"                      => 4.5,
                        "aeroway_runway"               => 4.6,
                        _                              => 0.0,
                    };
                    parent.spawn((
                        Mesh2d(meshes.add(payload.mesh)),
                        MeshMaterial2d(white_mat.clone()),
                        Transform::from_xyz(0.0, 0.0, z),
                        Visibility::default(),
                        MapLayer(payload.layer.clone()),
                        Pickable::IGNORE,
                        Selectable {
                            tile_coord: coord,
                            layer: payload.layer,
                            feature_id: None,
                        },
                    ));
                }

                if let Some(mesh) = data.building_mesh {
                    parent.spawn((
                        Mesh2d(meshes.add(mesh)),
                        MeshMaterial2d(white_mat.clone()),
                        Transform::from_xyz(0.0, 0.0, 6.0),
                        Visibility::default(),
                        MapLayer("building".to_owned()),
                        Pickable::IGNORE,
                    ));
                }

                for proxy in data.building_proxies {
                    let lot_data = lot_db.get(proxy.bin).cloned();
                    let tier = bin_tiers.0.get(&proxy.bin).copied().unwrap_or(proxy.tier);

                    // Directly instantiate 2D color blocks because we cleared proxy vertex colors.
                    let proxy_mat = if tier == OwnershipTier::CONVERTED {
                        let mat = mat_cache.get_or_create(
                            building_colors::MAT_CONVERTED, &mut materials, building_colors::converted_material,
                        );
                        let cm = col_cache.get_or_create(building_colors::MAT_CONVERTED, &mut col_mats, || {
                            ColorMaterial { color: building_colors::converted_material().base_color, ..default() }
                        });
                        (Some(MeshMaterial3d(mat)), MeshMaterial2d(cm))
                    } else if tier == OwnershipTier::CONSTRUCTION {
                        let mat = mat_cache.get_or_create(
                            building_colors::MAT_CONSTRUCTION, &mut materials, building_colors::construction_material,
                        );
                        let cm = col_cache.get_or_create(building_colors::MAT_CONSTRUCTION, &mut col_mats, || {
                            ColorMaterial { color: building_colors::construction_material().base_color, ..default() }
                        });
                        (Some(MeshMaterial3d(mat)), MeshMaterial2d(cm))
                    } else {
                        let mat_key = format!("tier_2d_{}", tier);
                        let cm = col_cache.get_or_create(&mat_key, &mut col_mats, || {
                            ColorMaterial { color: building_colors::tier_srgb_color(tier), ..default() }
                        });
                        (None::<MeshMaterial3d<StandardMaterial>>, MeshMaterial2d(cm))
                    };

                    let mut proxy_cmd = parent.spawn((
                        Mesh2d(meshes.add(proxy.mesh)),
                        proxy_mat.1.clone(),
                        BaseMaterial2d(proxy_mat.1.0.clone()),
                        Transform::from_xyz(0.0, 0.0, 7.0),
                        Visibility::default(),
                        MapLayer("building".to_owned()),
                        BuildingProxyMarker,
                        BuildingCenter(proxy.center),
                        BuildingAabb2D {
                            min: proxy.center.truncate() - proxy.half_extents.truncate(),
                            max: proxy.center.truncate() + proxy.half_extents.truncate(),
                        },
                        Selectable {
                            tile_coord: coord,
                            layer: "building".to_owned(),
                            feature_id: Some(proxy.bin as u64),
                        },
                    ));

                    if let Some(mat3d) = proxy_mat.0 {
                        proxy_cmd.insert((mat3d.clone(), BaseMaterial3d(mat3d.0.clone())));
                    }
                    if let Some(data) = lot_data {
                        proxy_cmd.insert(data);
                    }
                }
            } else {
            for payload in data.layer_meshes {
                let style = layer_style(&payload.layer);
                let mat = mat_cache.get_or_create(&payload.mat_key, &mut materials, || {
                    style.into_material()
                });
                // Y offsets per layer to prevent z-fighting in 3D.
                let y_nudge = match payload.layer.as_str() {
                    "landcover"                 => 0.00,
                    "landuse"                   => 0.02,
                    "park"                      => 0.04,
                    "water" | "waterway"        => 0.06,
                    "transportation"            => 0.08,
                    "transportation_minor"      => 0.09,
                    "transportation_rail"       => 0.10,
                    "aeroway"                   => 0.05,
                    "aeroway_runway"            => 0.07,
                    _                           => 0.00,
                };
                parent.spawn((
                    Mesh3d(meshes.add(payload.mesh)),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, y_nudge, 0.0),
                    Visibility::default(),
                    MapLayer(payload.layer.clone()),
                    Pickable::IGNORE,
                    Selectable {
                        tile_coord: coord,
                        layer: payload.layer,
                        feature_id: None,
                    },
                ));
            }

            if let Some(mesh) = data.building_mesh {
                let mat = mat_cache.get_or_create(MAT_BUILDING_UNIFIED, &mut materials, unified_building_material);
                parent.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(mat),
                    Transform::default(),
                    Visibility::default(),
                    MapLayer("building".to_owned()),
                    Pickable::IGNORE,
                ));
            }

            // Individual buildings that are pickable and have accurate geometry
            let invisible_mat = mat_cache.get_or_create(
                MAT_PROXY_INVISIBLE,
                &mut materials,
                proxy_invisible_material,
            );

            for proxy in data.building_proxies {
                // Check BinTiers for live converted or construction state to survive mode switches.
                let effective_tier = bin_tiers.0.get(&proxy.bin).copied().unwrap_or(proxy.tier);
                let mat = if effective_tier == OwnershipTier::CONVERTED {
                    mat_cache.get_or_create(building_colors::MAT_CONVERTED, &mut materials, building_colors::converted_material)
                } else if effective_tier == OwnershipTier::CONSTRUCTION {
                    mat_cache.get_or_create(building_colors::MAT_CONSTRUCTION, &mut materials, building_colors::construction_material)
                } else {
                    let mat_key = tier_material_key(effective_tier);
                    mat_cache.get_or_create(&mat_key, &mut materials, || {
                        tier_base_material(effective_tier)
                    })
                };

                let lot_data = lot_db.get(proxy.bin).cloned();

                let mut proxy_cmd = parent
                    .spawn((
                        Mesh3d(meshes.add(proxy.mesh)),
                        MeshMaterial3d(mat.clone()),
                        BaseMaterial3d(mat),
                        Transform::default(),
                        Visibility::default(),
                        MapLayer("building".to_owned()),
                        BuildingProxyMarker,
                        BuildingCenter(proxy.center),
                        Pickable {
                            should_block_lower: false,
                            is_hoverable: false,
                        },
                        Selectable {
                            tile_coord: coord,
                            layer: "building".to_owned(),
                            feature_id: Some(proxy.bin as u64),
                        },
                    ))
                ;

                if let Some(ld) = lot_data {
                    proxy_cmd.insert(ld);
                }

                // Re-attach simulation state components so game logic sees them.
                if effective_tier == OwnershipTier::CONVERTED {
                    proxy_cmd.insert(PlayerConverted {});
                } else if effective_tier == OwnershipTier::CONSTRUCTION {
                    if let Some(site) = lot_db.constructions.get(&proxy.bin).cloned() {
                        proxy_cmd.insert(site);
                    }
                }

                let hitbox_mesh = Mesh::from(Cuboid::new(
                    proxy.half_extents.x * 2.0,
                    proxy.half_extents.y * 2.0,
                    proxy.half_extents.z * 2.0,
                ));

                proxy_cmd.with_children(|proxy_parent| {
                    proxy_parent.spawn((
                        Mesh3d(meshes.add(hitbox_mesh)),
                        MeshMaterial3d(invisible_mat.clone()),
                        Transform::from_translation(proxy.center),
                        Pickable::default(),
                    ));
                });
            }
        }});
    }
}

// Unload system

pub fn unload_tile_system(
    mut events: MessageReader<UnloadTileEvent>,
    mut commands: Commands,
    mut lru: ResMut<TileLruCache>,
) {
    for ev in events.read() {
        if let Some(root) = lru.roots.get(&ev.coord).copied() {
            commands.entity(root).despawn();
        }
        lru.remove(&ev.coord);
    }
}

// Background worker

fn decode_mesh_tile(
    coord: TileCoord,
    raw: Vec<u8>,
    snapshot: &RendererSnapshot,
) -> Result<TileRenderData, String> {
    // PMTiles reader often returns already decompressed bytes. However, try decompressing first.
    let buf = {
        let mut gz = GzDecoder::new(&raw[..]);
        let mut out = Vec::new();
        if gz.read_to_end(&mut out).is_ok() && !out.is_empty() {
            out
        } else {
            raw
        }
    };

    // Parse MVT
    let reader = Reader::new(buf).map_err(|e| format!("mvt: {e}"))?;
    let layer_meta = reader
        .get_layer_metadata()
        .map_err(|e| format!("meta: {e}"))?;

    let mut layer_meshes = Vec::new();
    let mut building_mesh = None;
    let mut building_proxies = Vec::new();

    for meta in layer_meta {
        let raw_features = match reader.get_features(meta.layer_index) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let features: Vec<ParsedFeature> = raw_features
            .into_iter()
            .map(|f| ParsedFeature {
                geometry: f.geometry,
                properties: f
                    .properties
                    .unwrap_or_default()
                    .iter()
                    .map(|(k, v)| (k.clone(), convert_mvt_value(v)))
                    .collect(),
            })
            .collect();

        let mut point_sink = Vec::new();

        if meta.name == "building" {
            let (mesh_data, proxies) =
                snapshot.build_building_layer_unified(&features, coord, &mut point_sink);

            building_mesh = mesh_data.map(|md| md.into_bevy_mesh());
            building_proxies.extend(proxies);
        } else if meta.name == "aeroway" {
            let mut runways = Vec::new();
            let mut other = Vec::new();

            for f in &features {
                let class = f.prop_str("class").unwrap_or("");
                if class == "runway" || class == "taxiway" {
                    runways.push(f.clone());
                } else {
                    other.push(f.clone());
                }
            }

            if let Some((md, mk)) = snapshot.build_layer("aeroway", &other, coord, &mut point_sink)
            {
                layer_meshes.push(LayerMeshPayload {
                    layer: "aeroway".to_owned(),
                    mesh: md.into_bevy_mesh(),
                    mat_key: mk,
                });
            }
            if let Some((md, mk)) =
                snapshot.build_layer("aeroway_runway", &runways, coord, &mut point_sink)
            {
                layer_meshes.push(LayerMeshPayload {
                    layer: "aeroway_runway".to_owned(),
                    mesh: md.into_bevy_mesh(),
                    mat_key: mk,
                });
            }
        } else if meta.name == "transportation" {
            let mut minor_feats = Vec::new();
            let mut major_feats = Vec::new();
            let mut rail_feats = Vec::new();

            for f in &features {
                let class = f.prop_str("class").unwrap_or("");
                let brunnel = f.prop_str("brunnel").unwrap_or("");

                // Drop ferry lines
                if class == "ferry" {
                    continue;
                }

                if is_rail_class(class) {
                    // Only keep rail that is visible.
                    if brunnel != "tunnel" {
                        rail_feats.push(f.clone());
                    }
                    continue;
                }

                // Classify remaining road features as major or minor for hierarchical culling.
                if matches!(
                    class,
                    "path" | "footway" | "pedestrian" | "cycleway" | "service" | "track" | "steps"
                ) {
                    minor_feats.push(f.clone());
                } else {
                    major_feats.push(f.clone());
                }
            }

            // Major roads
            if let Some((md, mk)) =
                snapshot.build_layer("transportation", &major_feats, coord, &mut point_sink)
            {
                layer_meshes.push(LayerMeshPayload {
                    layer: "transportation".to_owned(),
                    mesh: md.into_bevy_mesh(),
                    mat_key: mk,
                });
            }
            // Minor paths
            if let Some((md, mk)) =
                snapshot.build_layer("transportation", &minor_feats, coord, &mut point_sink)
            {
                layer_meshes.push(LayerMeshPayload {
                    layer: "transportation_minor".to_owned(),
                    mesh: md.into_bevy_mesh(),
                    mat_key: mk,
                });
            }
            // Above-ground rail
            if let Some((md, mk)) =
                snapshot.build_layer("transportation_rail", &rail_feats, coord, &mut point_sink)
            {
                layer_meshes.push(LayerMeshPayload {
                    layer: "transportation_rail".to_owned(),
                    mesh: md.into_bevy_mesh(),
                    mat_key: mk,
                });
            }
        } else if let Some((mesh_data, mat_key)) =
            snapshot.build_layer(&meta.name, &features, coord, &mut point_sink)
        {
            layer_meshes.push(LayerMeshPayload {
                layer: meta.name.clone(),
                mesh: mesh_data.into_bevy_mesh(),
                mat_key,
            });
        }
    }

    Ok(TileRenderData {
        coord,
        layer_meshes,
        building_mesh,
        building_proxies,
        mode: snapshot.mode.clone(),
    })
}
