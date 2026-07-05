//!
//! Exposes [`AppPlugin`], which wires together [`MapSimulationPlugin`] and the
//! initial scene setup.  Desktop entry is in `main.rs`; mobile entry is in
//! `mobile/src/lib.rs`.

#![allow(clippy::type_complexity)]

mod building_colors;
mod building_panel;
mod cache;
mod components;
mod events;
mod interaction;
mod loader;
mod lot_database;
mod mesher;
mod plugin;
mod renderer;
mod sim_budget;
mod streaming;
mod style;
mod tile_backend;
mod tiles_source;
mod types;

use bevy::prelude::*;

use crate::plugin::{MapConfig, MapSimulationPlugin};

/// Set to `true` locally to load tiles from `MBTILES_PATH` instead of the
/// remote PMTiles URL.  See the README for setup instructions.
pub const RUN_LOCAL: bool = false;

const PMTILES_URL: &str =
    "https://content-spf.funguylabs.app/user-uploads/new-york.pmtiles";
const MBTILES_PATH: &str = "./res/new-york.mbtiles";

/// Top-level plugin for the simulation.
///
/// Add this to a [`bevy::app::App`] after [`bevy::DefaultPlugins`] to bring up
/// the full NYC housing simulation.  Window / platform configuration (canvas
/// bindings, fullscreen mode, mobile settings) is left to the caller so that
/// `main.rs` and `mobile/src/lib.rs` can each supply the right settings for
/// their target.
pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        let config = MapConfig {
            zoom: 14,
            origin_x: 4822, // centre of Manhattan at zoom 14
            origin_y: 6160,
            lru_capacity: 25,
            ..MapConfig::default()
        };

        app.add_plugins(MapSimulationPlugin {
            config,
            pmtiles_url: PMTILES_URL.to_string(),
            mbtiles_path: MBTILES_PATH.to_string(),
        })
        .add_systems(Startup, setup_scene);
    }
}

/// Spawns the directional sun light that illuminates the 3-D building meshes.
fn setup_scene(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));
}
