//! Feature renderers for buildings, lines, polygons, and off-thread rendering snapshot.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;

use crate::components::{OwnershipTier, TileCoord};
use crate::mesher::{MeshData, geometry_to_mesh, geometry_to_mesh_2d};
use crate::plugin::{MapConfig, RenderMode};
use crate::style::road_class_width;
use crate::types::ParsedFeature;

/// Returns true for rail-based transport classes.
pub fn is_rail_class(class: &str) -> bool {
    matches!(
        class,
        "rail"
            | "subway"
            | "narrow_gauge"
            | "tram"
            | "monorail"
            | "funicular"
            | "light_rail"
            | "transit"
            | "metro"
            | "railway"
    )
}

/// Returns true for ferry and boat-way transport classes.
pub fn is_ferry_class(class: &str) -> bool {
    class == "ferry"
}

/// Per-building axis-aligned bounding box used for invisible picking cuboids.
#[derive(Debug, Clone)]
pub struct BuildingProxy {
    pub bin: u32,
    pub tier: u8,
    /// The geometry for this building.
    pub mesh: Mesh,
    /// Center in tile-local world space.
    pub center: Vec3,
    /// Half-extents of the bounding box for the hitbox.
    pub half_extents: Vec3,
}

// Feature renderer trait

pub trait FeatureRenderer: Send + Sync {
    fn mesh_feature(
        &self,
        feature: &ParsedFeature,
        layer: &str,
        coord: TileCoord,
        config: &MapConfig,
        mode: &RenderMode,
        color: [f32; 4],
        point_out: &mut Vec<[f32; 3]>,
    ) -> Option<MeshData>;

    fn material_key(&self) -> &'static str;
}

// Concrete renderers

pub struct BuildingRenderer;

impl FeatureRenderer for BuildingRenderer {
    fn material_key(&self) -> &'static str {
        "building"
    }

    fn mesh_feature(
        &self,
        feature: &ParsedFeature,
        _layer: &str,
        _coord: TileCoord,
        config: &MapConfig,
        mode: &RenderMode,
        color: [f32; 4],
        points: &mut Vec<[f32; 3]>,
    ) -> Option<MeshData> {
        let scale = config.tile_scale;
        let hs = config.height_scale;
        if *mode == RenderMode::Mode2D {
            geometry_to_mesh_2d(&feature.geometry, scale, 0.0, 0.0, color, points)
        } else {
            let render_min = feature.prop_f32("render_min_height").unwrap_or(0.0) * hs;
            let render_top = feature.prop_f32("render_height").unwrap_or(3.66) * hs;
            let top = config.ground_y + render_top;
            let bot = config.ground_y + render_min;
            geometry_to_mesh(&feature.geometry, scale, bot, Some(top), 0.0, color, points)
        }
    }
}

pub struct FlatPolygonRenderer;

impl FeatureRenderer for FlatPolygonRenderer {
    fn material_key(&self) -> &'static str {
        "flat"
    }

    fn mesh_feature(
        &self,
        feature: &ParsedFeature,
        _layer: &str,
        _coord: TileCoord,
        config: &MapConfig,
        mode: &RenderMode,
        color: [f32; 4],
        points: &mut Vec<[f32; 3]>,
    ) -> Option<MeshData> {
        // z/y offset is applied at the entity Transform level in loader.rs.
        // This avoids 2D sort order confusion and 3D z-fighting from geometry floating at different heights.
        if *mode == RenderMode::Mode2D {
            geometry_to_mesh_2d(
                &feature.geometry,
                config.tile_scale,
                0.0,
                0.0,
                color,
                points,
            )
        } else {
            geometry_to_mesh(
                &feature.geometry,
                config.tile_scale,
                config.ground_y,
                None,
                0.0,
                color,
                points,
            )
        }
    }
}

pub struct LineRenderer {
    pub width_override: Option<f32>,
}

impl FeatureRenderer for LineRenderer {
    fn material_key(&self) -> &'static str {
        "line"
    }

    fn mesh_feature(
        &self,
        feature: &ParsedFeature,
        _layer: &str,
        _coord: TileCoord,
        config: &MapConfig,
        mode: &RenderMode,
        color: [f32; 4],
        points: &mut Vec<[f32; 3]>,
    ) -> Option<MeshData> {
        let class = feature.prop_str("class").unwrap_or("residential");
        let brunnel = feature.prop_str("brunnel").unwrap_or("");
        let tunnel = brunnel == "tunnel";

        // Ignore ferry lines
        if is_ferry_class(class) {
            return None;
        }
        // Rail classes are handled by RailLineRenderer.
        if is_rail_class(class) {
            return None;
        }
        // Skip road sections that pass through tunnels.
        if tunnel {
            return None;
        }

        let width = self
            .width_override
            .unwrap_or_else(|| road_class_width(class, config.tile_scale));
        if *mode == RenderMode::Mode2D {
            geometry_to_mesh_2d(
                &feature.geometry,
                config.tile_scale,
                1.0,
                width,
                color,
                points,
            )
        } else {
            geometry_to_mesh(
                &feature.geometry,
                config.tile_scale,
                config.ground_y + 0.01,
                None,
                width,
                color,
                points,
            )
        }
    }
}

/// Renders above-ground subway, commuter rail, tram, and light-rail lines.
/// Tunnel segments and ferry routes are always skipped.
pub struct RailLineRenderer;

impl FeatureRenderer for RailLineRenderer {
    fn material_key(&self) -> &'static str {
        "rail_line"
    }

    fn mesh_feature(
        &self,
        feature: &ParsedFeature,
        _layer: &str,
        _coord: TileCoord,
        config: &MapConfig,
        mode: &RenderMode,
        color: [f32; 4],
        points: &mut Vec<[f32; 3]>,
    ) -> Option<MeshData> {
        let class = feature.prop_str("class").unwrap_or("");
        let brunnel = feature.prop_str("brunnel").unwrap_or("");

        // Only render rails
        if !is_rail_class(class) || is_ferry_class(class) {
            return None;
        }
        // Ignore tunnel segments.
        if brunnel == "tunnel" {
            return None;
        }

        let width = road_class_width("rail", config.tile_scale);
        if *mode == RenderMode::Mode2D {
            geometry_to_mesh_2d(
                &feature.geometry,
                config.tile_scale,
                1.0,
                width,
                color,
                points,
            )
        } else {
            // Slight y-lift so rail sits just above road surfaces in 3D without z-fighting.
            geometry_to_mesh(
                &feature.geometry,
                config.tile_scale,
                config.ground_y + 0.02,
                None,
                width,
                color,
                points,
            )
        }
    }
}

// Registry

#[derive(Resource, Default)]
pub struct RendererRegistry {
    renderers: HashMap<String, Arc<dyn FeatureRenderer>>,
    default: Option<Arc<dyn FeatureRenderer>>,
}

impl RendererRegistry {
    pub fn register(&mut self, layer: impl Into<String>, r: impl FeatureRenderer + 'static) {
        self.renderers.insert(layer.into(), Arc::new(r));
    }

    pub fn snapshot(
        &self,
        config: MapConfig,
        bin_tiers: Arc<HashMap<u32, u8>>,
        mode: RenderMode,
    ) -> RendererSnapshot {
        RendererSnapshot {
            renderers: self.renderers.clone(),
            default: self.default.clone(),
            config,
            bin_tiers,
            mode,
        }
    }
}

// Snapshot

#[derive(Clone)]
pub struct RendererSnapshot {
    renderers: HashMap<String, Arc<dyn FeatureRenderer>>,
    default: Option<Arc<dyn FeatureRenderer>>,
    pub config: MapConfig,
    pub bin_tiers: Arc<HashMap<u32, u8>>,
    pub mode: RenderMode,
}

impl RendererSnapshot {
    fn get_renderer(&self, layer: &str) -> Option<&Arc<dyn FeatureRenderer>> {
        self.renderers.get(layer).or(self.default.as_ref())
    }

    /// Merge all features of a non-building layer into a single `MeshData`.
    pub fn build_layer(
        &self,
        layer: &str,
        features: &[ParsedFeature],
        coord: TileCoord,
        point_out: &mut Vec<[f32; 3]>,
    ) -> Option<(MeshData, String)> {
        if matches!(layer, "waterway") {
            return None;
        }

        let renderer = self.get_renderer(layer)?;

        let color = if self.mode == RenderMode::Mode2D {
            crate::style::layer_style(layer)
                .color
                .to_linear()
                .to_f32_array()
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };

        let mat_key = format!("{layer}_{}", renderer.material_key());

        let mut combined = MeshData::default();
        for feature in features {
            if let Some(md) = renderer.mesh_feature(
                feature,
                layer,
                coord,
                &self.config,
                &self.mode,
                color,
                point_out,
            ) {
                combined.merge(md);
            }
        }
        if combined.is_empty() {
            return None;
        }
        Some((combined, mat_key))
    }

    /// Build the building layer into a single unified mesh using vertex colors.
    pub fn build_building_layer_unified(
        &self,
        features: &[ParsedFeature],
        coord: TileCoord,
        point_out: &mut Vec<[f32; 3]>,
    ) -> (Option<MeshData>, Vec<BuildingProxy>) {
        use crate::building_colors::tier_srgb_color;
        let renderer = match self.get_renderer("building") {
            Some(r) => r,
            None => return (None, Vec::new()),
        };

        let mut combined = MeshData::default();
        let mut proxies: Vec<BuildingProxy> = Vec::new();

        for feature in features {
            let bin = feature
                .prop_str("bin")
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(0);
            let tier = if bin > 0 {
                *self.bin_tiers.get(&bin).unwrap_or(&OwnershipTier::MARKET)
            } else {
                OwnershipTier::MARKET
            };

            // Proxies always use pure white vertex colors allowing dynamic materials
            // to accurately reflect player changes without mixing colors.
            // Generic baked buildings encode standard default tier color.
            let mesh_color = if bin > 0 {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                tier_srgb_color(tier).to_linear().to_f32_array()
            };

            let Some(md) = renderer.mesh_feature(
                feature,
                "building",
                coord,
                &self.config,
                &self.mode,
                mesh_color,
                point_out,
            ) else {
                continue;
            };

            if bin > 0 {
                if let Some((center, half_extents)) = calculate_aabb(&md) {
                    proxies.push(BuildingProxy {
                        bin,
                        tier,
                        mesh: md.into_bevy_mesh(),
                        center,
                        half_extents: half_extents + Vec3::splat(0.05),
                    });
                }
            } else {
                combined.merge(md);
            }
        }

        (Some(combined).filter(|m| !m.is_empty()), proxies)
    }
}

fn calculate_aabb(md: &MeshData) -> Option<(Vec3, Vec3)> {
    if md.positions.is_empty() {
        return None;
    }

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for &[x, y, z] in &md.positions {
        min = min.min(Vec3::new(x, y, z));
        max = max.max(Vec3::new(x, y, z));
    }

    let center = (min + max) * 0.5;
    let half_extents = (max - min) * 0.5;

    Some((center, half_extents))
}
