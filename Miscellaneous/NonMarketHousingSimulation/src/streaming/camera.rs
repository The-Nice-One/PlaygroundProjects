//! 3D orbital and 2D pan/zoom camera controllers.

use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::components::MapCamera2d;
use crate::interaction::SelectionMode;

// 2D camera tuning constants
const CAMERA_2D_ROT_SPEED: f32 = 1.5; // radians/sec per key hold multiplier
const CAMERA_2D_ZOOM_MULT: f32 = 5.0; // multiplier to make 2D zoom feel faster

#[derive(Resource, Default)]
pub struct CameraState {
    /// The point in world space the camera is orbiting.
    pub target: Vec3,
    /// Last camera position where culling was calculated.
    pub last_cull_pos: Vec3,
    /// Current 2D camera rotation angle in radians, where 0 = north-up.
    pub angle_2d: f32,
}

/// Basic orbital camera controller.
/// - Right Click and Drag to rotate.
/// - Mouse Wheel to zoom.
/// - Left Click and Drag to pan map only in Single Selection Mode.
pub fn camera_controller_system(
    time: Res<Time>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera3d>>,
    mut state: ResMut<CameraState>,
    mode: Res<SelectionMode>,
) {
    let transform = query.single_mut();
    let mut transform = match transform {
        Ok(tf) => tf,
        Err(_) => return, // No camera found
    };

    // Keyboard Pan
    let mut move_dir = Vec3::ZERO;
    let forward = *transform.forward();
    let right = *transform.right();
    // Flatten the vectors to the XZ plane and re-normalize
    let pan_fwd = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let pan_side = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        move_dir += pan_fwd;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        move_dir -= pan_fwd;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        move_dir -= pan_side;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        move_dir += pan_side;
    }

    if move_dir.length_squared() > 0.0 {
        let speed = 100.0;
        let delta = move_dir.normalize() * speed * time.delta_secs();
        // Pan both the camera and the target center together.
        transform.translation += delta;
        state.target += delta;
    }

    // Keyboard Rotation
    let mut rot_delta = 0.0;
    if keyboard.pressed(KeyCode::KeyQ) {
        rot_delta += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyE) {
        rot_delta -= 1.0;
    }
    if rot_delta != 0.0 {
        let yaw = Quat::from_rotation_y(rot_delta * 1.5 * time.delta_secs());
        let focus = state.target;
        let mut offset = transform.translation - focus;
        offset = yaw * offset;
        transform.translation = focus + offset;
        transform.look_at(focus, Vec3::Y);
    }

    // Keyboard Zoom
    let mut kb_zoom = 0.0;
    if keyboard.pressed(KeyCode::KeyR) {
        kb_zoom += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyF) {
        kb_zoom -= 1.0;
    }
    if kb_zoom != 0.0 {
        let forward = transform.forward();
        transform.translation += forward * kb_zoom * 50.0 * time.delta_secs();
    }

    // Zoom
    for event in mouse_wheel.read() {
        // WASM/Browsers often use MouseScrollUnit::Pixel which is much larger than Line.
        // We normalize the sensitivity here to prevent fast zooming.
        let sensitivity = match event.unit {
            MouseScrollUnit::Line => 5.0,
            MouseScrollUnit::Pixel => 0.15,
        };

        let forward = transform.forward();
        transform.translation += forward * event.y * sensitivity;
    }

    // Collect mouse motions to process rotation/pan in a single event query loop
    let mut motion_delta = Vec2::ZERO;
    let mut has_motion = false;
    for event in mouse_motion.read() {
        motion_delta += event.delta;
        has_motion = true;
    }

    if has_motion {
        // Left-Click hold and drag map pan in single mode only
        if *mode == SelectionMode::Single && mouse_button.pressed(MouseButton::Left) {
            let offset = transform.translation - state.target;
            let radius = offset.length();
            let factor = 0.002 * radius;

            // Pan relative to looking direction flat on the ground plane
            let pan_delta = -pan_side * motion_delta.x * factor + pan_fwd * motion_delta.y * factor;
            transform.translation += pan_delta;
            state.target += pan_delta;
        }
        // Rotate / Orbit
        else if mouse_button.pressed(MouseButton::Right) {
            // Rotate around the Y axis
            let yaw = Quat::from_rotation_y(-motion_delta.x * 0.005);
            // Rotate around the camera's local X axis
            let pitch = Quat::from_rotation_x(-motion_delta.y * 0.005);

            // Rotate around the dynamic target instead of the world origin.
            let focus = state.target;
            let offset = transform.translation - focus;
            let radius = offset.length();
            let mut new_offset = yaw * pitch * offset;

            // Prevent looking from underside
            // Also clamps top angle to prevent flip over the direct vertical axis.
            let min_y = (radius * 0.05).max(1.0); // Minimum positive Y offset
            let max_y = radius * 0.98; // Maximum positive Y offset
            if new_offset.y < min_y {
                new_offset.y = min_y;
                let xz_len = (radius * radius - min_y * min_y).sqrt();
                let current_xz_len = Vec2::new(new_offset.x, new_offset.z).length();
                if current_xz_len > 0.0 {
                    new_offset.x = (new_offset.x / current_xz_len) * xz_len;
                    new_offset.z = (new_offset.z / current_xz_len) * xz_len;
                }
            } else if new_offset.y > max_y {
                new_offset.y = max_y;
                let xz_len = (radius * radius - max_y * max_y).sqrt();
                let current_xz_len = Vec2::new(new_offset.x, new_offset.z).length();
                if current_xz_len > 0.0 {
                    new_offset.x = (new_offset.x / current_xz_len) * xz_len;
                    new_offset.z = (new_offset.z / current_xz_len) * xz_len;
                }
            }

            transform.translation = focus + new_offset;
            transform.look_at(focus, Vec3::Y);
        }
    }
}

pub fn camera_controller_2d_system(
    time: Res<Time>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<MapCamera2d>>,
    mut state: ResMut<CameraState>,
    mode: Res<SelectionMode>,
) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    // Pan
    let mut move_dir = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        move_dir.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        move_dir.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        move_dir.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        move_dir.x += 1.0;
    }

    let current_scale = transform.scale.x;
    if move_dir.length_squared() > 0.0 {
        let speed = 500.0 * current_scale;
        let dir = move_dir.normalize();
        // Rotate input by camera angle so movement follows camera orientation
        let cos = state.angle_2d.cos();
        let sin = state.angle_2d.sin();
        let rx = dir.x * cos - dir.y * sin;
        let ry = dir.x * sin + dir.y * cos;
        let delta = Vec2::new(rx, ry) * speed * time.delta_secs();

        transform.translation.x += delta.x;
        transform.translation.y += delta.y;

        state.target.x = transform.translation.x;
        state.target.z = -transform.translation.y;
    }

    // Rotation Q/E
    let mut rot_delta = 0.0;
    if keyboard.pressed(KeyCode::KeyQ) {
        rot_delta += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyE) {
        rot_delta -= 1.0;
    }
    if rot_delta != 0.0 {
        let ang = rot_delta * CAMERA_2D_ROT_SPEED * time.delta_secs();
        state.angle_2d += ang;

        // Rotate camera position around the target
        let cam_x = transform.translation.x;
        let cam_y = -transform.translation.y;
        let focus_x = state.target.x;
        let focus_y = state.target.z;

        let offset_x = cam_x - focus_x;
        let offset_y = cam_y - focus_y;
        let cos = ang.cos();
        let sin = ang.sin();
        let rx = offset_x * cos - offset_y * sin;
        let ry = offset_x * sin + offset_y * cos;

        transform.translation.x = focus_x + rx;
        transform.translation.y = -(focus_y + ry);
        transform.rotation = Quat::from_rotation_z(state.angle_2d);
    }

    // Collect mouse motions to process rotation/pan in a single event query loop
    let mut motion_delta = Vec2::ZERO;
    let mut has_motion = false;
    for event in mouse_motion.read() {
        motion_delta += event.delta;
        has_motion = true;
    }

    if has_motion {
        // Left-click drag panning in 2D in single mode.
        if *mode == SelectionMode::Single && mouse_button.pressed(MouseButton::Left) {
            let cos = state.angle_2d.cos();
            let sin = state.angle_2d.sin();

            let drag_x = -motion_delta.x * current_scale;
            let drag_y = motion_delta.y * current_scale;

            let rx = drag_x * cos - drag_y * sin;
            let ry = drag_x * sin + drag_y * cos;

            transform.translation.x += rx;
            transform.translation.y += ry;

            state.target.x = transform.translation.x;
            state.target.z = -transform.translation.y;
        }
        // Right-click drag rotation
        else if mouse_button.pressed(MouseButton::Right) {
            let ang = -motion_delta.x * 0.005;
            state.angle_2d += ang;

            let cam_x = transform.translation.x;
            let cam_y = -transform.translation.y;
            let focus_x = state.target.x;
            let focus_y = state.target.z;

            let offset_x = cam_x - focus_x;
            let offset_y = cam_y - focus_y;
            let cos = ang.cos();
            let sin = ang.sin();
            let rx = offset_x * cos - offset_y * sin;
            let ry = offset_x * sin + offset_y * cos;

            transform.translation.x = focus_x + rx;
            transform.translation.y = -(focus_y + ry);
            transform.rotation = Quat::from_rotation_z(state.angle_2d);
        }
    }

    // Zoom
    let mut zoom_delta = 0.0;
    if keyboard.pressed(KeyCode::KeyR) {
        zoom_delta -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyF) {
        zoom_delta += 1.0;
    }

    for event in mouse_wheel.read() {
        let sensitivity = match event.unit {
            MouseScrollUnit::Line => 0.1 * CAMERA_2D_ZOOM_MULT,
            MouseScrollUnit::Pixel => 0.005 * CAMERA_2D_ZOOM_MULT,
        };
        zoom_delta -= event.y * sensitivity;
    }

    if zoom_delta != 0.0 {
        let new_scale =
            (transform.scale.x * (1.0 + zoom_delta * 5.0 * time.delta_secs())).clamp(0.01, 10.0);
        transform.scale = Vec3::new(new_scale, new_scale, 1.0);
    }
}
