//! Pointer-based mesh selection, hover effects, and mass-drag logic.

use bevy::color::Hsla;
use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;

use crate::cache::{ColorMaterialCache, MaterialCache};
use crate::components::{
    BaseMaterial2d, BaseMaterial3d, BuildingAabb2D, BuildingCenter, BuildingProxyMarker, Hovered,
    LotData, MapCamera2d, MapCamera3d, OwnershipTier, PlayerConverted, Selectable, Selected,
};
use crate::events::{
    BuildOnLotAction, ConvertBuildingAction, SelectionClearedEvent, ShowBuildingPanel,
    ToggleSelectionModeEvent,
};
use crate::plugin::RenderMode;
use crate::sim_budget::SimulationBudget;

// Selection state

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    #[default]
    Single,
    Mass,
}

#[derive(Resource, Default)]
pub struct DragState {
    pub start: Option<Vec2>,
    pub current: Option<Vec2>,
}

#[derive(Resource, Default)]
pub struct SelectionState {
    pub selected: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct HoverState {
    pub hovered_2d: Option<Entity>,
}

// Click observer

/// Registered as a global observer on `Pointer<Click>`.
pub fn on_mesh_click(
    mut click: On<Pointer<Click>>,
    selectables: Query<(&Selectable, Option<&LotData>, Option<&BuildingProxyMarker>)>,
    parents: Query<&ChildOf>,
    mut state: ResMut<SelectionState>,
    mode: Res<SelectionMode>,
    render_mode: Res<State<RenderMode>>,
    mut clr_ev: MessageWriter<SelectionClearedEvent>,
    mut panel_ev: MessageWriter<ShowBuildingPanel>,
    mut commands: Commands,
) {
    // If in mass mode, use drag system instead.
    if *mode == SelectionMode::Mass {
        return;
    }

    // Disable 3D observer handling in 2D mode so it doesn't double-process 2D picks
    // or incorrectly deselect buildings during a 2D map drag.
    if *render_mode.get() == RenderMode::Mode2D {
        return;
    }

    let mut entity = click.entity;

    // If the clicked entity hitbox isn't selectable, check its parent which is the building.
    if !selectables.contains(entity) {
        if let Ok(parent) = parents.get(entity) {
            entity = parent.get();
        }
    }

    // Check if we clicked something selectable.
    let Ok((sel, lot_data, is_proxy)) = selectables.get(entity) else {
        if let Some(prev) = state.selected.take() {
            if let Some(mut e) = commands.get_entity(prev).ok() {
                e.remove::<Selected>();
            }
        }
        clr_ev.write(SelectionClearedEvent);
        return;
    };

    // Check if this selectable has valid LotData.
    if lot_data.is_none() {
        if let Some(prev) = state.selected.take() {
            if let Some(mut e) = commands.get_entity(prev).ok() {
                e.remove::<Selected>();
            }
        }
        clr_ev.write(SelectionClearedEvent);
        return;
    }

    // Stop the click event from bubbling up to parents or the background, which would cause an accidental deselection.
    click.propagate(false);

    // Deselect the previous entity if it differs.
    if let Some(prev) = state.selected {
        if prev != entity {
            if let Some(mut e) = commands.get_entity(prev).ok() {
                e.remove::<Selected>();
            }
        }
    }

    // Establish the new selection
    state.selected = Some(entity);
    if let Some(mut e) = commands.get_entity(entity).ok() {
        e.insert(Selected);
    }

    // Building proxies with lot data show the panel.
    if is_proxy.is_some() {
        panel_ev.write(ShowBuildingPanel { entity });
    }

    info!(
        "[Selected] layer='{}' tile={} id={:?}",
        sel.layer, sel.tile_coord, sel.feature_id
    );
}

// Hover Observers for 3D

pub fn on_mesh_over(
    over: On<Pointer<Over>>,
    parents: Query<&ChildOf>,
    proxies: Query<Option<&LotData>, With<BuildingProxyMarker>>,
    mode: Res<SelectionMode>,
    mut commands: Commands,
) {
    if *mode == SelectionMode::Mass {
        return;
    }

    let mut entity = over.entity;
    if let Ok(parent) = parents.get(entity) {
        entity = parent.get();
    }
    if let Ok(lot) = proxies.get(entity) {
        if lot.is_some() {
            if let Some(mut e) = commands.get_entity(entity).ok() {
                e.insert(Hovered);
            }
        }
    }
}

pub fn on_mesh_out(
    out: On<Pointer<Out>>,
    parents: Query<&ChildOf>,
    mode: Res<SelectionMode>,
    mut commands: Commands,
) {
    if *mode == SelectionMode::Mass {
        return;
    }

    let mut entity = out.entity;
    if let Ok(parent) = parents.get(entity) {
        entity = parent.get();
    }
    if let Some(mut e) = commands.get_entity(entity).ok() {
        e.remove::<Hovered>();
    }
}

// 2D pick building

pub fn pick_building_2d(
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MapCamera2d>>,
    buildings_q: Query<(Entity, &BuildingAabb2D, &GlobalTransform, &LotData)>,
    ui_interactions: Query<&Interaction, With<Node>>,
    mut state: ResMut<SelectionState>,
    mode: Res<SelectionMode>,
    mut clr_ev: MessageWriter<SelectionClearedEvent>,
    mut panel_ev: MessageWriter<ShowBuildingPanel>,
    mut commands: Commands,
    mut mouse_down_pos: Local<Option<Vec2>>,
    mut was_ui_active: Local<bool>,
    mut last_touch_time: Local<f32>,
    mut last_pan_time: Local<f32>,
    time: Res<Time>,
) {
    if *mode == SelectionMode::Mass {
        return;
    }

    // Evaluate UI interaction logic before handling the pointer release
    let is_ui_active = ui_interactions.iter().any(|i| *i != Interaction::None);
    // Include last frame's UI activity to protect touches that were released
    let block_click = is_ui_active || *was_ui_active;
    *was_ui_active = is_ui_active;

    let Ok(window) = windows.single() else { return };

    // Store recent touch interactions to guard against simulated mouse clicks
    if touches.iter().count() > 0 || touches.iter_just_released().count() > 0 {
        *last_touch_time = time.elapsed_secs();
    }

    // Flag if touch is currently panning the map
    let mut touch_is_panning = false;
    for touch in touches.iter() {
        if touch.position().distance_squared(touch.start_position()) >= 100.0 {
            touch_is_panning = true;
        }
    }
    for touch in touches.iter_just_released() {
        if touch.position().distance_squared(touch.start_position()) >= 100.0 {
            touch_is_panning = true;
        }
    }
    if touch_is_panning {
        *last_pan_time = time.elapsed_secs();
    }

    // Store mouse down positions to distinguish between clicks and pan drags.
    if mouse.just_pressed(MouseButton::Left) {
        *mouse_down_pos = window.cursor_position();
    }

    // Collect the release position from either mouse or touch.
    let mut release_pos: Option<Vec2> = None;

    for touch in touches.iter_just_released() {
        // Only select a building if it was a tap and not a long pan drag
        if touch.position().distance_squared(touch.start_position()) < 100.0 {
            release_pos = Some(touch.position());
        }
        break;
    }

    if mouse.just_released(MouseButton::Left) {
        // Only process mouse clicks if no touch events happened recently
        // and the user has not touch-panned recently. This intercepts and drops OS-simulated touch-panning clicks.
        let quiet_from_touch = time.elapsed_secs() - *last_touch_time > 0.5;
        let quiet_from_pan = time.elapsed_secs() - *last_pan_time > 0.5;

        if quiet_from_touch && quiet_from_pan {
            if let (Some(start), Some(end)) = (*mouse_down_pos, window.cursor_position()) {
                if start.distance_squared(end) < 100.0 {
                    release_pos = Some(end);
                }
            }
        }
        *mouse_down_pos = None;
    }

    let Some(cursor_screen) = release_pos else {
        return;
    };

    // Prevent map click if the pointer is interacting with UI.
    if block_click {
        return;
    }

    let Ok((cam, cam_tf)) = camera_q.single() else {
        return;
    };
    let Ok(cursor_world) = cam.viewport_to_world_2d(cam_tf, cursor_screen) else {
        return;
    };

    let mut best_hit = None;
    let mut best_dist = f32::MAX;

    for (entity, aabb, tf, _) in &buildings_q {
        let world_min = tf.translation().truncate() + aabb.min;
        let world_max = tf.translation().truncate() + aabb.max;
        if cursor_world.x >= world_min.x
            && cursor_world.x <= world_max.x
            && cursor_world.y >= world_min.y
            && cursor_world.y <= world_max.y
        {
            let center = (world_min + world_max) / 2.0;
            let dist = center.distance_squared(cursor_world);
            if dist < best_dist {
                best_dist = dist;
                best_hit = Some(entity);
            }
        }
    }

    if let Some(entity) = best_hit {
        if let Some(prev) = state.selected {
            if prev != entity {
                if let Some(mut e) = commands.get_entity(prev).ok() {
                    e.remove::<Selected>();
                }
            }
        }
        state.selected = Some(entity);
        if let Some(mut e) = commands.get_entity(entity).ok() {
            e.insert(Selected);
        }

        if let Ok((..)) = buildings_q.get(entity) {
            panel_ev.write(ShowBuildingPanel { entity });
        }
    } else {
        if let Some(prev) = state.selected.take() {
            if let Some(mut e) = commands.get_entity(prev).ok() {
                e.remove::<Selected>();
            }
        }
        clr_ev.write(SelectionClearedEvent);
    }
}

pub fn hover_building_2d_system(
    windows: Query<&Window>,
    touches: Res<Touches>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MapCamera2d>>,
    buildings_q: Query<(Entity, &BuildingAabb2D, &GlobalTransform), With<LotData>>,
    ui_interactions: Query<&Interaction, With<Node>>,
    mut state: ResMut<HoverState>,
    mut commands: Commands,
    mode: Res<SelectionMode>,
) {
    if *mode == SelectionMode::Mass {
        if let Some(prev) = state.hovered_2d {
            if let Some(mut e) = commands.get_entity(prev).ok() {
                e.remove::<Hovered>();
            }
            state.hovered_2d = None;
        }
        return;
    }

    let Ok(window) = windows.single() else { return };
    let mut cursor_screen = window.cursor_position();

    // Prioritize active touches to bypass inaccurate desktop touch simulations and allow mobile panning highlights.
    if let Some(touch) = touches.iter().next() {
        cursor_screen = Some(touch.position());
    }

    let Some(cursor_screen) = cursor_screen else {
        if let Some(prev) = state.hovered_2d {
            if let Some(mut e) = commands.get_entity(prev).ok() {
                e.remove::<Hovered>();
            }
            state.hovered_2d = None;
        }
        return;
    };

    // Clear hover effect if moving pointer into UI.
    if ui_interactions.iter().any(|i| *i != Interaction::None) {
        if let Some(prev) = state.hovered_2d {
            if let Some(mut e) = commands.get_entity(prev).ok() {
                e.remove::<Hovered>();
            }
            state.hovered_2d = None;
        }
        return;
    }

    let Ok((cam, cam_tf)) = camera_q.single() else {
        return;
    };
    let Ok(cursor_world) = cam.viewport_to_world_2d(cam_tf, cursor_screen) else {
        return;
    };

    let mut best_hit = None;
    let mut best_dist = f32::MAX;

    // Pick the overlapping AABB whose center is closest to the cursor.
    // This prevents flickering of building hover effects.
    for (entity, aabb, tf) in &buildings_q {
        let world_min = tf.translation().truncate() + aabb.min;
        let world_max = tf.translation().truncate() + aabb.max;
        if cursor_world.x >= world_min.x
            && cursor_world.x <= world_max.x
            && cursor_world.y >= world_min.y
            && cursor_world.y <= world_max.y
        {
            let center = (world_min + world_max) / 2.0;
            let dist = center.distance_squared(cursor_world);
            if dist < best_dist {
                best_dist = dist;
                best_hit = Some(entity);
            }
        }
    }

    if best_hit != state.hovered_2d {
        if let Some(prev) = state.hovered_2d {
            if let Some(mut e) = commands.get_entity(prev).ok() {
                e.remove::<Hovered>();
            }
        }
        if let Some(new) = best_hit {
            if let Some(mut e) = commands.get_entity(new).ok() {
                e.insert(Hovered);
            }
        }
        state.hovered_2d = best_hit;
    }
}

// Unified Visuals Update System

pub fn update_proxy_visuals_system(
    sel_added: Query<Entity, Added<Selected>>,
    mut sel_removed: RemovedComponents<Selected>,
    hov_added: Query<Entity, Added<Hovered>>,
    mut hov_removed: RemovedComponents<Hovered>,
    base3d_changed: Query<Entity, Changed<BaseMaterial3d>>,
    base2d_changed: Query<Entity, Changed<BaseMaterial2d>>,
    mut query: Query<(
        Option<&Selected>,
        Option<&Hovered>,
        Option<&BaseMaterial3d>,
        Option<&BaseMaterial2d>,
        Option<&mut MeshMaterial3d<StandardMaterial>>,
        Option<&mut MeshMaterial2d<ColorMaterial>>,
    )>,
    mut mats3d: ResMut<Assets<StandardMaterial>>,
    mut mats2d: ResMut<Assets<ColorMaterial>>,
    mut mat_cache: ResMut<MaterialCache>,
    mut col_cache: ResMut<ColorMaterialCache>,
) {
    let mut entities = std::collections::HashSet::new();
    for e in &sel_added {
        entities.insert(e);
    }
    for e in sel_removed.read() {
        entities.insert(e);
    }
    for e in &hov_added {
        entities.insert(e);
    }
    for e in hov_removed.read() {
        entities.insert(e);
    }
    for e in &base3d_changed {
        entities.insert(e);
    }
    for e in &base2d_changed {
        entities.insert(e);
    }

    for entity in entities {
        let Ok((selected, hovered, base3d, base2d, mat3d, mat2d)) = query.get_mut(entity) else {
            continue;
        };

        // Handle 3D Materials
        if let (Some(base3d_handle), Some(mut m3d)) = (base3d, mat3d) {
            let mat_handle = if selected.is_some() || hovered.is_some() {
                if let Some(base_mat) = mats3d.get(&base3d_handle.0).cloned() {
                    let key = format!(
                        "highlight_{:?}_{}_{}",
                        base_mat.base_color,
                        selected.is_some(),
                        hovered.is_some()
                    );
                    mat_cache.get_or_create(&key, &mut mats3d, || {
                        let mut m = base_mat;
                        let mut hsla: Hsla = m.base_color.into();
                        hsla.saturation = (hsla.saturation * 1.5).min(1.0);
                        let l_bump = if selected.is_some() { 1.3 } else { 1.15 };
                        hsla.lightness = (hsla.lightness * l_bump).min(1.0);
                        m.base_color = hsla.into();
                        m
                    })
                } else {
                    base3d_handle.0.clone()
                }
            } else {
                base3d_handle.0.clone()
            };

            // Only update the handle if it actually changed
            if m3d.0 != mat_handle {
                m3d.0 = mat_handle;
            }
        }

        // Handle 2D Materials
        if let (Some(base2d_handle), Some(mut m2d)) = (base2d, mat2d) {
            let mat_handle = if selected.is_some() || hovered.is_some() {
                if let Some(base_mat) = mats2d.get(&base2d_handle.0).cloned() {
                    let key = format!(
                        "highlight_{:?}_{}_{}",
                        base_mat.color,
                        selected.is_some(),
                        hovered.is_some()
                    );
                    col_cache.get_or_create(&key, &mut mats2d, || {
                        let mut m = base_mat;
                        let mut hsla: Hsla = m.color.into();
                        hsla.saturation = (hsla.saturation * 1.5).min(1.0);
                        let l_bump = if selected.is_some() { 1.3 } else { 1.15 };
                        hsla.lightness = (hsla.lightness * l_bump).min(1.0);
                        m.color = hsla.into();
                        m
                    })
                } else {
                    base2d_handle.0.clone()
                }
            } else {
                base2d_handle.0.clone()
            };

            if m2d.0 != mat_handle {
                m2d.0 = mat_handle;
            }
        }
    }
}

// Clear on empty click

pub fn clear_selection_system(
    mut events: ParamSet<(
        MessageReader<SelectionClearedEvent>,
        MessageWriter<SelectionClearedEvent>,
    )>,
    mut convert_events: MessageReader<ConvertBuildingAction>,
    mut build_events: MessageReader<BuildOnLotAction>,
    selected: Query<Entity, With<Selected>>,
    mut commands: Commands,
    mut state: ResMut<SelectionState>,
) {
    let mut should_clear = false;

    // Automatically deselect and close the details panel when convert or build operations are fired
    let mut action_fired = false;
    for _ in convert_events.read() {
        action_fired = true;
    }
    for _ in build_events.read() {
        action_fired = true;
    }

    if action_fired {
        should_clear = true;
    }

    for _ in events.p0().read() {
        should_clear = true;
    }

    if should_clear {
        for entity in &selected {
            if let Some(mut e) = commands.get_entity(entity).ok() {
                e.remove::<Selected>();
            }
        }
        state.selected = None;

        // Notify other systems like UI hide_building_panel_system that the panel should close
        if action_fired {
            events.p1().write(SelectionClearedEvent);
        }
    }
}

pub fn toggle_selection_mode_system(
    mut events: MessageReader<ToggleSelectionModeEvent>,
    mut mode: ResMut<SelectionMode>,
) {
    for _ in events.read() {
        *mode = match *mode {
            SelectionMode::Single => SelectionMode::Mass,
            SelectionMode::Mass => SelectionMode::Single,
        };
        info!("Selection Mode: {:?}", *mode);
    }
}

pub fn drag_selection_system(
    mode: Res<SelectionMode>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    mut drag: ResMut<DragState>,
    camera_q: Query<(&Camera, &GlobalTransform), Or<(With<MapCamera3d>, With<MapCamera2d>)>>,
    buildings_q: Query<
        (
            Entity,
            &GlobalTransform,
            &LotData,
            &BuildingCenter,
            Option<&Hovered>,
        ),
        (With<BuildingProxyMarker>, Without<PlayerConverted>),
    >,
    budget: Res<SimulationBudget>,
    mut convert_w: MessageWriter<ConvertBuildingAction>,
    mut commands: Commands,
) {
    // If not in mass mode, ensure any leftover drag state or highlights are cleared.
    if *mode != SelectionMode::Mass {
        if drag.start.is_some() || drag.current.is_some() {
            drag.start = None;
            drag.current = None;
            for (entity, _, _, _, hovered) in &buildings_q {
                if hovered.is_some() {
                    if let Some(mut e) = commands.get_entity(entity).ok() {
                        e.remove::<Hovered>();
                    }
                }
            }
        }
        return;
    }

    let Ok(window) = windows.single() else { return };

    let mut input_just_pressed = mouse_input.just_pressed(MouseButton::Left);
    let mut input_pressed = mouse_input.pressed(MouseButton::Left);
    let mut input_just_released = mouse_input.just_released(MouseButton::Left);
    let mut cursor_pos = window.cursor_position();

    if let Some(touch) = touches.iter_just_pressed().next() {
        input_just_pressed = true;
        input_pressed = true;
        cursor_pos = Some(touch.position());
    } else if let Some(touch) = touches.iter().next() {
        input_pressed = true;
        cursor_pos = Some(touch.position());
    } else if let Some(touch) = touches.iter_just_released().next() {
        input_just_released = true;
        cursor_pos = Some(touch.position());
    }

    let Some(cursor_pos) = cursor_pos else {
        return;
    };

    if input_just_pressed {
        drag.start = Some(cursor_pos);
        drag.current = Some(cursor_pos);
    } else if input_pressed {
        drag.current = Some(cursor_pos);
    } else if input_just_released {
        // Trigger conversion on release
        if let (Some(start), Some(end)) = (drag.start, drag.current) {
            if let Ok((camera, cam_tf)) = camera_q.single() {
                let min = Vec2::new(start.x.min(end.x), start.y.min(end.y));
                let max = Vec2::new(start.x.max(end.x), start.y.max(end.y));

                let mut candidates = Vec::new();
                for (entity, tf, lot, center, hovered) in &buildings_q {
                    if hovered.is_some() {
                        if let Some(mut e) = commands.get_entity(entity).ok() {
                            e.remove::<Hovered>();
                        }
                    }

                    if lot.ownership == OwnershipTier::Market && lot.units_res > 0 {
                        let world_center = tf.translation() + center.0;
                        if let Ok(screen_pos) = camera.world_to_viewport(cam_tf, world_center) {
                            if screen_pos.x >= min.x
                                && screen_pos.x <= max.x
                                && screen_pos.y >= min.y
                                && screen_pos.y <= max.y
                            {
                                candidates.push((entity, lot));
                            }
                        }
                    }
                }

                // Sort to guarantee the same order used for budgeting during drag.
                candidates.sort_by_key(|(_, lot)| lot.bin);

                let mut total_cost = 0;
                let mut to_convert = Vec::new();

                for (entity, lot) in candidates {
                    let cost = lot.conversion_one_time_cost();
                    if budget.can_afford(total_cost + cost) {
                        total_cost += cost;
                        to_convert.push((entity, lot.bin));
                    }
                }

                if !to_convert.is_empty() {
                    info!("Mass converting {} buildings...", to_convert.len());
                    for (entity, bin) in to_convert {
                        convert_w.write(ConvertBuildingAction { entity, bin });
                    }
                }
            }
        }
        drag.start = None;
        drag.current = None;
        return;
    }

    // Highlighting logic while dragging
    if let (Some(start), Some(current)) = (drag.start, drag.current) {
        if let Ok((camera, cam_tf)) = camera_q.single() {
            let min = Vec2::new(start.x.min(current.x), start.y.min(current.y));
            let max = Vec2::new(start.x.max(current.x), start.y.max(current.y));

            let mut candidates = Vec::new();
            let mut others = Vec::new();

            for (entity, tf, lot, center, hovered) in &buildings_q {
                let mut in_bounds = false;

                if lot.ownership == OwnershipTier::Market && lot.units_res > 0 {
                    let world_center = tf.translation() + center.0;
                    if let Ok(screen_pos) = camera.world_to_viewport(cam_tf, world_center) {
                        if screen_pos.x >= min.x
                            && screen_pos.x <= max.x
                            && screen_pos.y >= min.y
                            && screen_pos.y <= max.y
                        {
                            in_bounds = true;
                        }
                    }
                }

                if in_bounds {
                    candidates.push((entity, lot, hovered.is_some()));
                } else if hovered.is_some() {
                    others.push(entity);
                }
            }

            candidates.sort_by_key(|(_, lot, _)| lot.bin);

            let mut total_cost = 0;
            for (entity, lot, is_hovered) in candidates {
                let cost = lot.conversion_one_time_cost();
                let should_hover = budget.can_afford(total_cost + cost);

                if should_hover {
                    total_cost += cost;
                    if !is_hovered {
                        if let Some(mut e) = commands.get_entity(entity).ok() {
                            e.insert(Hovered);
                        }
                    }
                } else if is_hovered {
                    if let Some(mut e) = commands.get_entity(entity).ok() {
                        e.remove::<Hovered>();
                    }
                }
            }

            // Remove hover from valid buildings that went out of the selection bounds
            for entity in others {
                if let Some(mut e) = commands.get_entity(entity).ok() {
                    e.remove::<Hovered>();
                }
            }
        }
    }
}

/// Visualizes the selection box using a UI node.
#[derive(Component)]
pub struct SelectionBoxUi;

pub fn draw_selection_box_system(
    drag: Res<DragState>,
    ui_scale: Res<bevy::ui::UiScale>,
    mut query: Query<&mut Node, With<SelectionBoxUi>>,
) {
    let Ok(mut node) = query.single_mut() else {
        return;
    };
    if let (Some(start), Some(current)) = (drag.start, drag.current) {
        let scale = ui_scale.0;
        node.display = Display::Flex;
        node.left = Val::Px(start.x.min(current.x) / scale);
        node.top = Val::Px(start.y.min(current.y) / scale);
        node.width = Val::Px((start.x - current.x).abs() / scale);
        node.height = Val::Px((start.y - current.y).abs() / scale);
    } else {
        node.display = Display::None;
    }
}
