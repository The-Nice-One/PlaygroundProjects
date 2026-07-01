//! Per-layer visual styling, colors, and line widths for the PBR renderer.

use bevy::pbr::StandardMaterial;
use bevy::prelude::*;

// Style descriptor

pub struct LayerStyle {
    pub color: Color,
    pub roughness: f32,
    pub metallic: f32,
    pub double_sided: bool,
    pub unlit: bool,
    pub emissive: LinearRgba,
}

impl Default for LayerStyle {
    fn default() -> Self {
        Self {
            color: Color::srgb(0.55, 0.55, 0.55),
            roughness: 0.9,
            metallic: 0.0,
            double_sided: true, // visible from both sides which is good for flat quads
            unlit: false,
            emissive: LinearRgba::BLACK,
        }
    }
}

impl LayerStyle {
    pub fn into_material(self) -> StandardMaterial {
        StandardMaterial {
            base_color: self.color,
            perceptual_roughness: self.roughness,
            metallic: self.metallic,
            double_sided: self.double_sided,
            cull_mode: if self.double_sided {
                None
            } else {
                Some(bevy::render::render_resource::Face::Back)
            },
            unlit: self.unlit,
            emissive: self.emissive,
            ..default()
        }
    }
}

// Per-layer lookup

/// Returns a visually distinct style per OpenMapTiles layer name.
pub fn layer_style(layer: &str) -> LayerStyle {
    match layer {
        // Ground cover
        "landcover" => LayerStyle {
            color: Color::srgb(0.35, 0.65, 0.28), // green
            roughness: 1.0,
            ..default()
        },
        "landuse" => LayerStyle {
            color: Color::srgb(0.42, 0.60, 0.30),
            roughness: 1.0,
            ..default()
        },
        "park" | "nature" => LayerStyle {
            color: Color::srgb(0.28, 0.72, 0.22), // bright green
            roughness: 1.0,
            ..default()
        },

        // Water
        "water" | "waterway" | "water_name" => LayerStyle {
            color: Color::srgb(0.48, 0.75, 0.95), // blue
            roughness: 0.1,
            metallic: 0.0,
            ..default()
        },

        // Buildings
        "building" => LayerStyle {
            color: Color::srgb(0.88, 0.78, 0.60), // tan or sand color
            roughness: 0.85,
            double_sided: false,
            ..default()
        },

        // Roads and transport
        "transportation" | "transportation_name" | "transportation_minor" => LayerStyle {
            color: Color::srgb(0.72, 0.68, 0.60), // grey
            roughness: 0.95,
            double_sided: false,
            ..default()
        },

        // Rail, subway, commuter train, etc.
        "transportation_rail" => LayerStyle {
            color: Color::srgb(0.52, 0.52, 0.56),
            roughness: 0.60,
            metallic: 0.30,
            double_sided: false,
            ..default()
        },

        // Administrative
        "boundary" => LayerStyle {
            color: Color::srgb(0.90, 0.20, 0.20), // bright red
            roughness: 0.8,
            ..default()
        },

        // Aeroway
        "aeroway" => LayerStyle {
            color: Color::srgb(0.50, 0.50, 0.55), // grey
            roughness: 0.9,
            ..default()
        },
        "aeroway_runway" => LayerStyle {
            color: Color::srgb(0.35, 0.35, 0.38), // dark grey
            roughness: 0.95,
            ..default()
        },

        // POI / labels
        "poi" | "place" | "housenumber" => LayerStyle {
            color: Color::srgb(0.95, 0.75, 0.10), // bright amber
            roughness: 0.5,
            metallic: 0.2,
            ..default()
        },

        _ => LayerStyle::default(),
    }
}

pub fn road_class_width(class: &str, tile_scale: f32) -> f32 {
    let mvt_width: f32 = match class {
        "motorway" => 28.0,
        "trunk" => 22.0,
        "primary" => 18.0,
        "secondary" => 14.0,
        "tertiary" => 10.0,
        "residential" | "unclassified" => 8.0,
        "service" => 6.0,
        "track" | "path" => 3.0,
        "rail" | "transit" => 5.0,
        "runway" => 35.0,
        "taxiway" => 18.0,
        _ => 7.0,
    };
    mvt_width * tile_scale
}
