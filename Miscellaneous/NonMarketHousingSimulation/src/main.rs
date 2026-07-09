// Hide the console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::prelude::*;
use bevy::window::WindowMode;
use non_market_housing_simulation::AppPlugin;

fn main() {
    // Redirect panics to the browser console on WASM.
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "NYC Non-Market Housing Simulation".into(),

                #[cfg(not(target_arch = "wasm32"))]
                resolution: (1400_u32, 900_u32).into(),

                #[cfg(not(target_arch = "wasm32"))]
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),

                canvas: Some("#bevy".to_owned()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(AppPlugin)
        .run();
}
