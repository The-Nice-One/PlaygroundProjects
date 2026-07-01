//! Main MapSimulationPlugin registering systems, resources, and state.

use bevy::prelude::*;

use crate::building_panel::{
    LodToggleFill, ToggleKnob, ToggleTrack, hide_building_panel_system, spawn_hud,
    update_building_panel_system, update_hud_system,
};
use crate::cache::{ColorMaterialCache, MaterialCache, TileLruCache};
use crate::components::{MapCamera2d, MapCamera3d, TileCoord};
use crate::events::{
    BuildOnLotAction, ConvertBuildingAction, LoadTileEvent, SelectionClearedEvent,
    ShowBuildingPanel, ToggleSelectionModeEvent, UnloadTileEvent,
};
use crate::interaction::{
    DragState, HoverState, SelectionMode, SelectionState, clear_selection_system,
    drag_selection_system, draw_selection_box_system, hover_building_2d_system, on_mesh_click,
    on_mesh_out, on_mesh_over, pick_building_2d, toggle_selection_mode_system,
    update_proxy_visuals_system,
};
use crate::loader::{
    ArcTileSource, TileQueue, TokioHandle, handle_load_events_system, process_loaded_tiles_system,
    spawn_task, unload_tile_system,
};
use crate::lot_database::{BinTiers, LotDatabase, poll_lot_db_load, start_lot_db_load};
use crate::renderer::RendererRegistry;
use crate::sim_budget::{
    HousingStats, SimulationBudget, advance_year_system, process_build_actions_system,
    process_convert_actions_system,
};
use crate::streaming::{
    CameraState, camera_tile_streaming_system, cull_building_proxies_system,
    cull_map_layers_system, request_initial_tiles,
};
#[cfg(target_arch = "wasm32")]
use crate::tiles_source::PmTilesTileSource;
#[cfg(not(target_arch = "wasm32"))]
use crate::tiles_source::{MbtilesTileSource, PmTilesTileSource};
use std::sync::{Arc, Mutex};

// App state

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug, Reflect)]
pub enum AppState {
    #[default]
    Loading,
    Playing,
}

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default, Reflect)]
pub enum RenderMode {
    #[default]
    Mode3D,
    Mode2D,
}

#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub struct LodOptimizationEnabled(pub bool);

impl Default for LodOptimizationEnabled {
    fn default() -> Self {
        Self(true)
    }
}

// Loading screen marker

#[derive(Component)]
pub struct LoadingScreen;

// MapConfig

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct MapConfig {
    pub tile_size: f32,
    pub tile_scale: f32,
    pub height_scale: f32,
    pub ground_y: f32,
    pub lru_capacity: usize,
    pub zoom: u8,
    pub origin_x: u32,
    pub origin_y: u32,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            tile_size: 81.92,
            tile_scale: 0.02,
            height_scale: 0.0442,
            ground_y: 0.0,
            lru_capacity: 25,
            zoom: 14,
            origin_x: 0,
            origin_y: 0,
        }
    }
}

impl MapConfig {
    /// World-space size of one square tile.
    pub fn tile_world_size(&self) -> f32 {
        self.tile_size
    }

    /// Origin tile as a `TileCoord`.
    pub fn origin_tile(&self) -> TileCoord {
        TileCoord::new(self.zoom, self.origin_x, self.origin_y)
    }

    /// World-space XZ position of the center of `coord`.
    pub fn tile_world_xz(&self, coord: TileCoord) -> Vec2 {
        let off = self.tile_world_offset(coord);
        Vec2::new(off.x, off.z)
    }

    /// World-space translation for a tile's root `Transform`.
    pub fn tile_world_offset(&self, coord: TileCoord) -> Vec3 {
        let dx = (coord.x as i64 - self.origin_x as i64) as f32 * self.tile_size;
        let dz = (coord.y as i64 - self.origin_y as i64) as f32 * self.tile_size;
        Vec3::new(dx, 0.0, dz)
    }
}

// Plugin

#[derive(Resource, Clone)]
pub struct MapSimulationPlugin {
    pub config: MapConfig,
    pub pmtiles_url: String,
    pub mbtiles_path: String,
}

#[derive(Resource)]
struct TileSourceReceiver(tokio::sync::oneshot::Receiver<ArcTileSource>);

impl Plugin for MapSimulationPlugin {
    fn build(&self, app: &mut App) {
        // State
        app.init_state::<AppState>();
        app.init_state::<RenderMode>();

        // Resources
        app.insert_resource(self.clone())
            .insert_resource(self.config.clone())
            .insert_resource(CameraState::default())
            .insert_resource(SelectionState::default())
            .init_resource::<HoverState>()
            .init_resource::<SelectionMode>()
            .init_resource::<DragState>()
            .init_resource::<LodOptimizationEnabled>()
            .init_resource::<MaterialCache>()
            .init_resource::<ColorMaterialCache>()
            .init_resource::<TileLruCache>()
            .init_resource::<RendererRegistry>()
            .init_resource::<LotDatabase>()
            .init_resource::<BinTiers>()
            .init_resource::<SimulationBudget>()
            .init_resource::<HousingStats>()
            .insert_resource(StaticTransformOptimizations::enabled());

        app.add_plugins(MeshPickingPlugin);

        // Messages
        app.add_message::<LoadTileEvent>()
            .add_message::<UnloadTileEvent>()
            .add_message::<SelectionClearedEvent>()
            .add_message::<ShowBuildingPanel>()
            .add_message::<ConvertBuildingAction>()
            .add_message::<BuildOnLotAction>()
            .add_message::<ToggleSelectionModeEvent>();

        // Tokio runtime for native
        #[cfg(not(target_arch = "wasm32"))]
        {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("tokio runtime");
            app.insert_resource(TokioHandle(rt.handle().clone()));
            std::mem::forget(rt);
        }
        // stub handle for WASM
        #[cfg(target_arch = "wasm32")]
        app.insert_resource(TokioHandle);

        // Tile queue
        app.insert_resource(TileQueue(Arc::new(Mutex::new(
            std::collections::VecDeque::new(),
        ))));

        // Startup
        app.add_systems(
            Startup,
            (
                setup_renderer_registry,
                setup_tile_source,
                start_lot_db_load,
                spawn_loading_screen,
            ),
        );

        // Loading state
        app.add_systems(
            Update,
            (poll_lot_db_load, poll_tile_source_load).run_if(in_state(AppState::Loading)),
        );
        app.add_systems(OnExit(AppState::Loading), despawn_loading_screen);

        // Startup
        app.add_systems(
            OnEnter(AppState::Playing),
            (request_initial_tiles, spawn_hud),
        );

        // Playing
        app.add_systems(
            Update,
            (
                camera_tile_streaming_system,
                crate::streaming::camera_controller_system,
                crate::streaming::camera_controller_2d_system,
                cull_building_proxies_system,
                cull_map_layers_system,
                handle_load_events_system,
                process_loaded_tiles_system,
                update_proxy_visuals_system,
                toggle_selection_mode_system,
                drag_selection_system,
                draw_selection_box_system,
                pick_building_2d,
                hover_building_2d_system,
            )
                .run_if(in_state(AppState::Playing)),
        );

        // Execute unloads in PostUpdate to ensure interaction systems finish applying
        // components for the frame.
        app.add_systems(
            PostUpdate,
            unload_tile_system.run_if(in_state(AppState::Playing)),
        );

        app.add_systems(
            Update,
            (
                clear_selection_system,
                update_building_panel_system,
                hide_building_panel_system,
                update_hud_system,
                update_toggle_button_system,
            )
                .run_if(in_state(AppState::Playing)),
        );

        app.add_systems(
            Update,
            (
                advance_year_system,
                process_convert_actions_system,
                process_build_actions_system,
            )
                .run_if(in_state(AppState::Playing)),
        );

        // Global observers
        app.add_observer(on_mesh_click);
        app.add_observer(on_mesh_over);
        app.add_observer(on_mesh_out);

        app.add_systems(Update, toggle_render_mode);
        app.add_systems(OnEnter(RenderMode::Mode2D), switch_to_2d);
        app.add_systems(OnEnter(RenderMode::Mode3D), switch_to_3d);
        app.add_systems(Update, update_lod_toggle_button_system);
    }
}

// Setup helpers

fn setup_tile_source(
    mut commands: Commands,
    plugin: Res<MapSimulationPlugin>,
    handle: Res<TokioHandle>,
) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    commands.insert_resource(TileSourceReceiver(rx));

    let url = plugin.pmtiles_url.clone();
    let path = plugin.mbtiles_path.clone();
    spawn_task(
        async move {
            if crate::RUN_LOCAL {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    match MbtilesTileSource::open(&path).await {
                        Ok(src) => {
                            let _ = tx.send(ArcTileSource::new(src));
                        }
                        Err(e) => error!("Failed to open local mbtiles '{}': {}", path, e),
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    error!("Local MBTiles loading is not supported on WASM");
                }
            } else {
                match PmTilesTileSource::new(url).await {
                    Ok(src) => {
                        let _ = tx.send(ArcTileSource::new(src));
                    }
                    Err(e) => error!("Failed to initialize PMTiles source: {e}"),
                }
            }
        },
        Some(&handle),
    );
}

fn poll_tile_source_load(mut commands: Commands, mut receiver: Option<ResMut<TileSourceReceiver>>) {
    let Some(ref mut rx) = receiver else {
        return;
    };
    if let Ok(source) = rx.0.try_recv() {
        commands.insert_resource(source);
        commands.remove_resource::<TileSourceReceiver>();
    }
}

fn setup_renderer_registry(mut registry: ResMut<RendererRegistry>) {
    use crate::renderer::{BuildingRenderer, FlatPolygonRenderer, LineRenderer, RailLineRenderer};
    registry.register("building", BuildingRenderer);
    registry.register(
        "transportation",
        LineRenderer {
            width_override: None,
        },
    );
    registry.register("transportation_rail", RailLineRenderer);
    registry.register("water", FlatPolygonRenderer);
    registry.register("landuse", FlatPolygonRenderer);
    registry.register("landcover", FlatPolygonRenderer);
    registry.register("park", FlatPolygonRenderer);
    registry.register("aeroway", FlatPolygonRenderer);
    registry.register(
        "aeroway_runway",
        LineRenderer {
            width_override: None,
        },
    );
}

// Updates the top-center toggle button appearance to reflect the current RenderMode.
fn update_toggle_button_system(
    state: Res<State<RenderMode>>,
    track_q: Query<Entity, With<ToggleTrack>>,
    mut knob_q: Query<&mut Node, With<ToggleKnob>>,
) {
    let is_2d = matches!(state.get(), RenderMode::Mode2D);
    if track_q.is_empty() {
        return;
    }

    for mut node in &mut knob_q {
        node.position_type = PositionType::Absolute;
        node.top = Val::Px(3.0);
        if is_2d {
            node.left = Val::Px(3.0);
            node.right = Val::Auto;
        } else {
            node.left = Val::Auto;
            node.right = Val::Px(3.0);
        }
    }
}

fn update_lod_toggle_button_system(
    lod: Res<LodOptimizationEnabled>,
    mut fill_q: Query<&mut BackgroundColor, With<LodToggleFill>>,
) {
    let enabled = lod.0;
    for mut fill in &mut fill_q {
        fill.0 = if enabled {
            Color::WHITE
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.0)
        };
    }
}

fn spawn_loading_screen(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.10, 0.97)),
            LoadingScreen,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("NYC Housing Simulation"),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            p.spawn((
                Text::new("Loading property data..."),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.55, 0.65)),
            ));
        });
}

fn despawn_loading_screen(query: Query<Entity, With<LoadingScreen>>, mut commands: Commands) {
    for e in &query {
        commands.entity(e).despawn();
    }
}

fn toggle_render_mode(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    state: Res<State<RenderMode>>,
    mut next_state: ResMut<NextState<RenderMode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Tab) {
        next_state.set(match state.get() {
            RenderMode::Mode3D => RenderMode::Mode2D,
            RenderMode::Mode2D => RenderMode::Mode3D,
        });
    }
}

fn switch_to_2d(
    camera3d_q: Query<Entity, With<MapCamera3d>>,
    mut commands: Commands,
    mut unload_ev: MessageWriter<UnloadTileEvent>,
    lru: Res<TileLruCache>,
    state: Res<crate::streaming::CameraState>,
) {
    for e in &camera3d_q {
        commands.entity(e).despawn();
    }
    commands.spawn((
        Camera2d,
        Transform::from_xyz(state.target.x, -state.target.z, 0.0)
            .with_scale(Vec3::new(1.0 / 8.0, 1.0 / 8.0, 1.0))
            .with_rotation(Quat::from_rotation_z(state.angle_2d)),
        MapCamera2d,
    ));
    for coord in lru.roots.keys() {
        unload_ev.write(UnloadTileEvent { coord: *coord });
    }
}

fn switch_to_3d(
    camera2d_q: Query<Entity, With<MapCamera2d>>,
    mut commands: Commands,
    mut unload_ev: MessageWriter<UnloadTileEvent>,
    lru: Res<TileLruCache>,
    state: Res<crate::streaming::CameraState>,
) {
    for e in &camera2d_q {
        commands.entity(e).despawn();
    }

    let offset = Vec3::new(0.0, 180.0, 120.0);
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(state.target + offset).looking_at(state.target, Vec3::Y),
        AmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
            ..default()
        },
        MapCamera3d,
    ));
    for coord in lru.roots.keys() {
        unload_ev.write(UnloadTileEvent { coord: *coord });
    }
}
