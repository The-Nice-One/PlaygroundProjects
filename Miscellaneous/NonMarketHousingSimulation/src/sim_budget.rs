//! Core housing simulation logic, budgeting, and statistics tracking.

use bevy::prelude::*;
use std::sync::Arc;

use crate::building_colors::{
    MAT_CONSTRUCTION, MAT_CONVERTED, construction_material, converted_material,
};
use crate::cache::{ColorMaterialCache, MaterialCache};
use crate::components::{
    BaseMaterial2d, BaseMaterial3d, ConstructionSite, LotData, OwnershipTier, PlayerConverted,
};
use crate::events::{BuildOnLotAction, ConvertBuildingAction};
use crate::lot_database::{BinTiers, LotDatabase};

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct SimulationBudget {
    pub game_year: u32,
    /// Dollars added to `available` each game year.
    pub annual_base: i64,
    /// Current spendable balance.
    pub available: i64,
    /// Sum of ongoing subsidies committed so far, which are subtracted each year.
    pub annual_costs: i64,
    /// Internal timer, one tick per real-time minute.
    pub year_timer: Timer,
}

impl Default for SimulationBudget {
    fn default() -> Self {
        Self {
            game_year: 2026,
            annual_base: 500_000_000,
            available: 500_000_000,
            annual_costs: 0,
            year_timer: Timer::from_seconds(60.0, TimerMode::Repeating),
        }
    }
}

impl SimulationBudget {
    pub fn can_afford(&self, cost: u64) -> bool {
        self.available >= cost as i64
    }

    pub fn spend(&mut self, one_time: u64) {
        self.available -= one_time as i64;
    }

    pub fn add_annual_cost(&mut self, annual: u64) {
        self.annual_costs += annual as i64;
    }
}

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct HousingStats {
    /// Non-market units already present at load time, from `LotDatabase`.
    pub baseline_nonmarket: u32,
    /// Residential units in market-rate buildings available to convert.
    pub baseline_market: u32,
    /// Units added by player conversions.
    pub converted_units: u32,
    /// Units added by player-built housing after construction completes.
    pub built_units: u32,
    /// Units currently under construction.
    pub under_construction: u32,
    /// Physical housing unit shortage. Based on NYC estimates about 250k to 500k.
    pub current_shortage: i64,
}

impl HousingStats {
    pub fn total_nonmarket(&self) -> u32 {
        self.baseline_nonmarket + self.converted_units + self.built_units
    }

    pub fn total_units(&self) -> u32 {
        self.baseline_market + self.baseline_nonmarket + self.converted_units + self.built_units
    }

    pub fn nonmarket_pct(&self) -> f32 {
        let t = self.total_units();
        if t == 0 {
            0.0
        } else {
            self.total_nonmarket() as f32 / t as f32 * 100.0
        }
    }

    /// Calculates global citizen happiness from 0 to 100 based on supply and affordability.
    pub fn happiness(&self) -> f32 {
        // Affordability score is normalized so small increases feel rewarding.
        // NYC baseline is about 10%. Scaled so reaching 25% non-market is a huge win.
        let affordability_score = (self.nonmarket_pct() * 4.0).min(100.0);

        // Shortage score which is based on how much of the initial 350k shortage remains.
        let initial_shortage = 350_000.0;
        let supply_score = (1.0
            - (self.current_shortage as f32 / (initial_shortage * 1.2)).clamp(0.0, 1.0))
            * 100.0;

        // Weighted average
        (supply_score * 0.6 + affordability_score * 0.4).clamp(0.0, 100.0)
    }
}

impl Default for HousingStats {
    fn default() -> Self {
        Self {
            baseline_nonmarket: 0,
            baseline_market: 0,
            converted_units: 0,
            built_units: 0,
            under_construction: 0,
            current_shortage: 350_000, // NYC estimate
        }
    }
}

// Systems

/// Advances the game year each real-time minute and settles the annual budget.
pub fn advance_year_system(
    time: Res<Time>,
    mut budget: ResMut<SimulationBudget>,
    mut stats: ResMut<HousingStats>,
    mut commands: Commands,
    mut mat_cache: ResMut<MaterialCache>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut col_cache: ResMut<ColorMaterialCache>,
    mut col_mats: ResMut<Assets<ColorMaterial>>,
    mut lot_db: ResMut<LotDatabase>,
    mut bin_tiers: ResMut<BinTiers>,
    lot_query: Query<(Entity, &LotData)>,
) {
    budget.year_timer.tick(time.delta());
    if !budget.year_timer.just_finished() {
        return;
    }

    budget.game_year += 1;

    // Dynamic budgeting so the more you solve the housing crisis, the more funding you get.
    let happiness = stats.happiness();
    let federal_bonus = (happiness * 5_000_000.0) as i64; // Up to $500M bonus at 100% happiness
    budget.available += (budget.annual_base + federal_bonus) - budget.annual_costs;

    // Growing shortage, as the city gets cheaper and more available, more people want to move in.
    let base_growth = 5_000; // Reduced natural population growth
    let attraction_growth = (350_000 - stats.current_shortage).max(0) as f32 * 0.04;
    stats.current_shortage += (base_growth as f32 + attraction_growth) as i64;

    // Complete any finished construction sites globally.
    let mut finished_bins = Vec::new();
    for (&bin, site) in &lot_db.constructions {
        if budget.game_year >= site.complete_at_year {
            finished_bins.push(bin);
        }
    }

    for bin in finished_bins {
        if let Some(site) = lot_db.constructions.remove(&bin) {
            stats.built_units += site.units_planned as u32;
            stats.current_shortage = stats
                .current_shortage
                .saturating_sub(site.units_planned as i64);
            stats.under_construction = stats
                .under_construction
                .saturating_sub(site.units_planned as u32);
        }

        if let Some(lot) = lot_db.records.get_mut(&bin) {
            lot.ownership = OwnershipTier::Converted;
            Arc::make_mut(&mut bin_tiers.0).insert(bin, OwnershipTier::CONVERTED);
        }

        // Swap material from construction to converted material for any currently spawned entities.
        let mat3d = mat_cache.get_or_create(MAT_CONVERTED, &mut mats, converted_material);
        let mat2d = col_cache.get_or_create(MAT_CONVERTED, &mut col_mats, || ColorMaterial {
            color: converted_material().base_color,
            ..default()
        });

        for (entity, lot_data) in lot_query.iter() {
            if lot_data.bin == bin {
                commands
                    .entity(entity)
                    .remove::<ConstructionSite>()
                    .insert((
                        MeshMaterial3d(mat3d.clone()),
                        BaseMaterial3d(mat3d.clone()),
                        MeshMaterial2d(mat2d.clone()),
                        BaseMaterial2d(mat2d.clone()),
                        PlayerConverted {},
                    ));
            }
        }
    }
}

/// Reads `ConvertBuildingAction` messages and converts market-rate buildings.
pub fn process_convert_actions_system(
    mut events: MessageReader<ConvertBuildingAction>,
    mut budget: ResMut<SimulationBudget>,
    mut stats: ResMut<HousingStats>,
    lot_query: Query<&LotData>,
    mut commands: Commands,
    mut mat_cache: ResMut<MaterialCache>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut col_cache: ResMut<ColorMaterialCache>,
    mut col_mats: ResMut<Assets<ColorMaterial>>,
    mut lot_db: ResMut<LotDatabase>,
    mut bin_tiers: ResMut<BinTiers>,
) {
    for ev in events.read() {
        let Ok(lot) = lot_query.get(ev.entity) else {
            warn!(
                "ConvertBuildingAction: entity {:?} has no LotData",
                ev.entity
            );
            continue;
        };

        if !matches!(lot.ownership, OwnershipTier::Market) {
            info!(
                "ConvertBuildingAction: BIN {} is already non-market",
                ev.bin
            );
            continue;
        }

        if lot.units_res == 0 {
            info!(
                "ConvertBuildingAction: BIN {} has no residential units",
                ev.bin
            );
            continue;
        }

        let one_time = lot.conversion_one_time_cost();
        let annual = lot.conversion_annual_cost();

        if !budget.can_afford(one_time) {
            info!(
                "ConvertBuildingAction: insufficient budget (need ${one_time}, have ${})",
                budget.available
            );
            continue;
        }

        budget.spend(one_time);
        budget.add_annual_cost(annual);
        stats.converted_units += lot.units_res as u32;
        stats.baseline_market = stats.baseline_market.saturating_sub(lot.units_res as u32);

        // Promote proxy from invisible to cyan.
        let mat3d = mat_cache.get_or_create(MAT_CONVERTED, &mut mats, converted_material);
        let mat2d = col_cache.get_or_create(MAT_CONVERTED, &mut col_mats, || ColorMaterial {
            color: converted_material().base_color,
            ..default()
        });
        commands.entity(ev.entity).insert((
            MeshMaterial3d(mat3d.clone()),
            BaseMaterial3d(mat3d.clone()),
            MeshMaterial2d(mat2d.clone()),
            BaseMaterial2d(mat2d.clone()),
            PlayerConverted {},
        ));

        // Persist to database so it stays cyan on next load.
        if let Some(lot_mut) = lot_db.records.get_mut(&ev.bin) {
            lot_mut.ownership = OwnershipTier::Converted;
        }
        Arc::make_mut(&mut bin_tiers.0).insert(ev.bin, OwnershipTier::CONVERTED);

        info!(
            "Converted BIN {} — {} units, ${one_time} one-time, ${annual}/yr",
            ev.bin, lot.units_res
        );
    }
}

/// Reads `BuildOnLotAction` messages and begins construction on parking lots.
pub fn process_build_actions_system(
    mut events: MessageReader<BuildOnLotAction>,
    mut budget: ResMut<SimulationBudget>,
    mut stats: ResMut<HousingStats>,
    lot_query: Query<&LotData>,
    mut commands: Commands,
    mut mat_cache: ResMut<MaterialCache>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut col_cache: ResMut<ColorMaterialCache>,
    mut col_mats: ResMut<Assets<ColorMaterial>>,
    mut lot_db: ResMut<LotDatabase>,
    mut bin_tiers: ResMut<BinTiers>,
) {
    for ev in events.read() {
        let Ok(lot) = lot_query.get(ev.entity) else {
            continue;
        };

        if !lot.ownership.is_parking() {
            info!("BuildOnLotAction: BIN {} is not a parking lot", ev.bin);
            continue;
        }

        // Don't allow double-building.
        let one_time = lot.construction_one_time_cost();
        let annual = lot.construction_annual_cost();
        let units = lot.buildable_units();

        if !budget.can_afford(one_time) {
            info!("BuildOnLotAction: insufficient budget for BIN {}", ev.bin);
            continue;
        }

        budget.spend(one_time);
        budget.add_annual_cost(annual);
        stats.under_construction += units as u32;

        // Proxy turns construction-orange and will turn cyan on completion.
        let mat3d = mat_cache.get_or_create(MAT_CONSTRUCTION, &mut mats, construction_material);
        let mat2d = col_cache.get_or_create(MAT_CONSTRUCTION, &mut col_mats, || ColorMaterial {
            color: construction_material().base_color,
            ..default()
        });

        let site = ConstructionSite {
            units_planned: units,
            complete_at_year: budget.game_year + 2,
        };

        commands.entity(ev.entity).insert((
            MeshMaterial3d(mat3d.clone()),
            BaseMaterial3d(mat3d.clone()),
            MeshMaterial2d(mat2d.clone()),
            BaseMaterial2d(mat2d.clone()),
            site.clone(),
        ));

        // Persist to database
        lot_db.constructions.insert(ev.bin, site);
        if let Some(lot_mut) = lot_db.records.get_mut(&ev.bin) {
            lot_mut.ownership = OwnershipTier::Construction;
        }
        Arc::make_mut(&mut bin_tiers.0).insert(ev.bin, OwnershipTier::CONSTRUCTION);

        info!(
            "Construction started on BIN {} — {} units, ${one_time} one-time",
            ev.bin, units
        );
    }
}
