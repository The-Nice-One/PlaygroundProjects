//! Mobile entry point for Android and iOS.
//!
//! `#[bevy_main]` lives here — and *only* here.  The root library crate
//! (`src/lib.rs`) deliberately has no `#[bevy_main]` so that the linker sees
//! exactly one `android_main` symbol when the APK is assembled.

use bevy::prelude::*;
use bevy::window::WindowMode;
use bevy::winit::WinitSettings;
use non_market_housing_simulation::AppPlugin;

/// Called directly by iOS / the Xcode-generated main.m.
#[unsafe(no_mangle)]
unsafe extern "C" fn main_rs() {
    main();
}

// This macro is a no-op on iOS and generates the `android_main` entry point
// on Android (required since Bevy 0.16 — see bevyengine/bevy#14780).
#[bevy_main]
fn main() {
    // iOS: switch the audio session to Ambient so background music from other
    // apps is not interrupted.  The default (SoloAmbient) would stop it.
    // https://developer.apple.com/documentation/avfaudio/avaudiosession/category-swift.struct/ambient
    #[cfg(target_os = "ios")]
    unsafe {
        if let Err(e) = objc2_avf_audio::AVAudioSession::sharedInstance()
            .setCategory_error(objc2_avf_audio::AVAudioSessionCategoryAmbient.unwrap())
        {
            println!("Error setting audio session category: {:?}", e);
        }
    }

    App::new()
        .insert_resource(WinitSettings::mobile())
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resizable: false,
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    ..default()
                }),
                ..default()
            }),
            AppPlugin,
        ))
        .run();
}
