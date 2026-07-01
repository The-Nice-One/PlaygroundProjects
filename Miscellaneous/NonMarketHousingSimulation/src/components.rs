//! Bevy ECS components used throughout the map and simulation plugin.

use bevy::prelude::*;

// Tile identity

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component)]
pub struct TileCoord {
    pub z: u8,
    pub x: u32,
    pub y: u32,
}

impl TileCoord {
    pub fn new(z: u8, x: u32, y: u32) -> Self {
        Self { z, x, y }
    }
}

impl std::fmt::Display for TileCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.z, self.x, self.y)
    }
}

#[derive(Component, Debug, Default)]
pub struct TileRoot;

// Map feature metadata

#[derive(Component, Debug, Default)]
pub struct MapCamera3d;

#[derive(Component, Debug, Default)]
pub struct MapCamera2d;

#[derive(Component, Debug, Clone)]
pub struct BuildingAabb2D {
    pub min: Vec2,
    pub max: Vec2,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct MapLayer(pub String);

// Interaction

#[derive(Component, Debug, Clone)]
pub struct Selectable {
    pub tile_coord: TileCoord,
    pub layer: String,
    pub feature_id: Option<u64>,
}

#[derive(Component, Debug, Default)]
pub struct Selected;

#[derive(Component, Debug, Default)]
pub struct Hovered;

/// Saves the base 3D material to dynamically overlay Selection or Hover graphics
#[derive(Component, Debug, Clone)]
pub struct BaseMaterial3d(pub Handle<StandardMaterial>);

/// Saves the base 2D material to dynamically overlay Selection or Hover graphics
#[derive(Component, Debug, Clone)]
pub struct BaseMaterial2d(pub Handle<ColorMaterial>);

// Ownership tier

/// Ownership classification encoded as `u8` in the binary lookup file produced by
/// `build_lot_lookup.py`.
#[repr(u8)]
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect, Default)]
#[reflect(Component)]
pub enum OwnershipTier {
    #[default]
    Market = 0, // private, market-rate
    City = 1,         // city-owned or mixed
    Authority = 2,    // public authority, state or federal
    ExemptRes = 3,    // fully exempt and residential units
    Keyword = 4,      // Mitchell-Lama, or HDFC by name only
    Parking = 5,      // parking lot or garage
    Converted = 6,    // converted by the player
    Construction = 7, // currently under construction
}

impl OwnershipTier {
    pub const MARKET: u8 = 0;
    pub const CITY: u8 = 1;
    pub const AUTHORITY: u8 = 2;
    pub const EXEMPT_RES: u8 = 3;
    pub const KEYWORD: u8 = 4;
    pub const PARKING: u8 = 5;
    pub const CONVERTED: u8 = 6;
    pub const CONSTRUCTION: u8 = 7;

    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::City,
            2 => Self::Authority,
            3 => Self::ExemptRes,
            4 => Self::Keyword,
            5 => Self::Parking,
            6 => Self::Converted,
            7 => Self::Construction,
            _ => Self::Market,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
    pub fn is_non_market(self) -> bool {
        !matches!(self, Self::Market | Self::Parking)
    }
    pub fn is_parking(self) -> bool {
        matches!(self, Self::Parking)
    }
}

// Lot data

/// Per-building property data attached to proxy entities.
/// Mirrors the 92-byte binary record from `build_lot_lookup.py`.
#[derive(Component, Clone, Debug)]
pub struct LotData {
    pub bin: u32,
    pub bbl: u64,
    pub height_m: f32, // roof height in metres
    pub assessed: u32, // assesstot, whole dollars
    pub exempt: u32,   // exempttot, whole dollars
    pub lot_area: u32, // sq ft
    pub res_far: f32,  // residential FAR
    /// Number of residential units in the building.
    pub units_res: u16,
    /// Total number of units including residential and commercial or other.
    pub units_total: u16,
    pub num_floors: u8,
    // pub landuse:     u8,    // DCP land-use code, unused.
    pub bldg_class: [u8; 2],
    pub ownership: OwnershipTier,
    pub owner: String,
}

impl LotData {
    /// Estimated market value in dollars from assessed value + building class.
    /// Uses NYC property-tax class ratios.
    pub fn estimated_market_value(&self) -> u64 {
        let mult: u64 = match self.bldg_class[0] {
            b'A' | b'B' => 17,       // Class 1: assessed ≈ 6 % of market
            b'C' | b'D' | b'R' => 2, // Class 2: assessed ≈ 45 % of market
            _ => 4,
        };
        self.assessed as u64 * mult
    }

    /// One-time cost to convert this building to non-market housing.
    /// 30% of estimated market value per residential unit.
    pub fn conversion_one_time_cost(&self) -> u64 {
        if self.units_res == 0 {
            return 0;
        }
        let mpu = self.estimated_market_value() / self.units_res as u64;
        mpu * 30 / 100 * self.units_res as u64
    }

    /// Estimated annual ongoing subsidy after conversion, which is 2% per unit.
    pub fn conversion_annual_cost(&self) -> u64 {
        if self.units_res == 0 {
            return 0;
        }
        let mpu = self.estimated_market_value() / self.units_res as u64;
        mpu * 2 / 100 * self.units_res as u64
    }

    /// Returns the number of units that can be built.
    pub fn buildable_units(&self) -> u16 {
        ((self.lot_area as f32 * self.res_far * 0.4 / 800.0) as u16).max(1)
    }

    pub fn construction_one_time_cost(&self) -> u64 {
        self.buildable_units() as u64 * 200_000
    }

    pub fn construction_annual_cost(&self) -> u64 {
        self.buildable_units() as u64 * 5_000
    }

    pub fn exempt_pct(&self) -> u8 {
        if self.assessed == 0 {
            return 0;
        }
        ((self.exempt as u64 * 100) / self.assessed as u64).min(100) as u8
    }
}

// Simulation components

/// Added when the player converts a building to non-market housing.
#[derive(Component, Debug, Clone)]
pub struct PlayerConverted {}

/// Added to a parking-lot proxy when the player builds housing.
#[derive(Component, Debug, Clone)]
pub struct ConstructionSite {
    pub units_planned: u16,
    pub complete_at_year: u32,
}

/// Marker on per-building proxy entities which are invisible picking cuboids.
/// Distinguishes them from the merged visual tile-layer meshes.
#[derive(Component, Debug, Default)]
pub struct BuildingProxyMarker;

/// The local offset of the building's center relative to the tile origin.
#[derive(Component, Debug, Clone)]
pub struct BuildingCenter(pub Vec3);
