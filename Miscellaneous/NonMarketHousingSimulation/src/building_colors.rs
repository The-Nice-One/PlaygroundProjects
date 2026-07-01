//! Ownership-tier color palette and `StandardMaterial` builders.

use crate::components::OwnershipTier;
use bevy::prelude::*;

// Material cache keys

pub const MAT_PROXY_INVISIBLE: &str = "building_proxy_invisible";
pub const MAT_CONVERTED: &str = "building_converted";
pub const MAT_CONSTRUCTION: &str = "building_construction";
pub const MAT_BUILDING_UNIFIED: &str = "building_unified";

/// Returns the cache key for the per-tier building visual material.
pub fn tier_material_key(tier: u8) -> String {
    format!("building_tier_{tier}")
}

// sRGB tier colors

/// sRGB `Color` for the building's visual material of a given ownership tier.
/// Used as `base_color` in a plain `StandardMaterial`.
pub fn tier_srgb_color(tier: u8) -> Color {
    match tier {
        OwnershipTier::CITY => Color::srgb(0.85, 0.28, 0.18), // terracotta
        OwnershipTier::AUTHORITY => Color::srgb(0.90, 0.40, 0.10), // deep orange for NYCHA
        OwnershipTier::EXEMPT_RES => Color::srgb(0.82, 0.66, 0.08), // gold for HDFC
        OwnershipTier::KEYWORD => Color::srgb(0.58, 0.28, 0.82), // violet for Mitchell-Lama
        OwnershipTier::PARKING => Color::srgb(0.18, 0.72, 0.32), // green for buildable parking
        OwnershipTier::CONVERTED => Color::srgb(0.0, 0.8, 1.0), // cyan
        OwnershipTier::CONSTRUCTION => Color::srgb(1.0, 0.5, 0.0), // orange
        _ => Color::srgb(0.88, 0.78, 0.60),                   // warm sand (market)
    }
}

/// sRGB color used in UI badges and the map legend.
pub fn tier_ui_color(tier: u8) -> Color {
    tier_srgb_color(tier)
}

/// Label for each ownership tier.
pub fn tier_label(tier: u8) -> &'static str {
    match tier {
        OwnershipTier::CITY => "City-Owned",
        OwnershipTier::AUTHORITY => "Public Authority (NYCHA)",
        OwnershipTier::EXEMPT_RES => "Exempt Residential (HDFC)",
        OwnershipTier::KEYWORD => "Regulated (Mitchell-Lama)",
        OwnershipTier::PARKING => "Parking / Garage",
        OwnershipTier::CONVERTED => "City Acquired (Player)",
        OwnershipTier::CONSTRUCTION => "Under Construction",
        _ => "Market Rate",
    }
}

// Material builders

/// Single vertex-color material for all buildings.
pub fn unified_building_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.85,
        metallic: 0.0,
        double_sided: false,
        cull_mode: Some(bevy::render::render_resource::Face::Back),
        ..default()
    }
}

/// Standard PBR material for a given ownership tier.
/// Buildings of the same tier share one material handle in `MaterialCache`.
pub fn tier_base_material(tier: u8) -> StandardMaterial {
    StandardMaterial {
        base_color: tier_srgb_color(tier),
        perceptual_roughness: 0.85,
        metallic: 0.0,
        double_sided: false,
        cull_mode: Some(bevy::render::render_resource::Face::Back),
        ..default()
    }
}

/// Fully transparent but hit-testable by `MeshPickingPlugin`.
pub fn proxy_invisible_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(0.0, 0.0, 0.0, 0.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    }
}

/// Bright cyan for player-converted buildings.
pub fn converted_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(0.10, 0.88, 0.82),
        emissive: LinearRgba::new(0.04, 0.38, 0.36, 1.0),
        perceptual_roughness: 0.60,
        double_sided: false,
        cull_mode: Some(bevy::render::render_resource::Face::Back),
        ..default()
    }
}

/// Orange for construction buildings.
pub fn construction_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(0.95, 0.55, 0.10),
        emissive: LinearRgba::new(0.20, 0.08, 0.0, 1.0),
        perceptual_roughness: 0.70,
        double_sided: false,
        cull_mode: Some(bevy::render::render_resource::Face::Back),
        ..default()
    }
}
