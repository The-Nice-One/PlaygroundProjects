#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

use bevy::{prelude::*, window::WindowMode};

use crate::plugin::{MapConfig, MapSimulationPlugin};

pub const RUN_LOCAL: bool = false;
const PMTILES_URL: &str = "https://content-spf.funguylabs.app/user-uploads/new-york.pmtiles";
const MBTILES_PATH: &str = "./res/new-york.mbtiles";

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    run_app();
}

fn run_app() {
    let config = MapConfig {
        zoom: 14,
        origin_x: 4822, // center of Manhattan at zoom 14
        origin_y: 6160,
        lru_capacity: 25,
        ..MapConfig::default()
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "NYC Non-Market Housing Simulation".into(),
                resolution: (1400_u32, 900_u32).into(),
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MapSimulationPlugin {
            config,
            pmtiles_url: PMTILES_URL.to_string(),
            mbtiles_path: MBTILES_PATH.to_string(),
        })
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(mut commands: Commands) {
    // Directional sun light
    commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));
}
