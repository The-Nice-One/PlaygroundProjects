// #![windows_subsystem = "windows"]
// Default screen size is 800, 600
use emerald::*;

mod assets;
use assets::*;
mod config;
mod draw;
mod update;
use config::*;

fn main() {
    emerald::start(
        Box::new(Window {
            ..Default::default()
        }),
        config_settings(),
    )
}

#[derive(Default)]
pub struct Window {
    data: Vec<u8>,
    shadow: Vec<u8>,
    player_direction: u8,
    ingame: bool,
    generated_menu: bool,
    tick: i128,
    spawn_tick: i128,
    castle_hp: u8,
    dice_x: Vec<f32>,
    dice_y: Vec<f32>,
    dice_side: Vec<u8>,
    dice_direction: Vec<u8>,
    dice_tick: i128,
    reload: i128,
    enemy_position: Vec<u16>,
    enemy_health: Vec<u8>,
    extradmg: u8,
    extradmgtick: u128,
    cause: String,
    score: i128,
    played: bool,
}

impl Game for Window {
    fn initialize(&mut self, mut emd: Emerald) {
        emd.set_asset_folder_root(String::from("./assets/"));
        pack_all_assets(&mut emd).unwrap();

        self.player_direction = 1;
        self.castle_hp = 3;

        self.data = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, //
            2, 51, 1, 1, 1, 1, 1, 35, 36, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 38, 37, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 35, 36, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 31, 32, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 4, 34, 33, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 35, 36, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 38, 37, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 35, 36, 1, 1, 1, 1, 1, 52, 2, //
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, //
        ];

        self.shadow = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, //
            2, 51, 1, 1, 1, 1, 1, 35, 36, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 38, 37, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 35, 36, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 31, 32, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 34, 33, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 35, 36, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 38, 37, 1, 1, 1, 1, 1, 52, 2, //
            2, 51, 1, 1, 1, 1, 1, 35, 36, 1, 1, 1, 1, 1, 52, 2, //
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, //
        ];

        let snd = emd
            .loader()
            .sound("sounds/HoliznaCC0 - Complications.ogg")
            .unwrap();
        emd.audio()
            .mixer("main")
            .unwrap()
            .play_and_loop(snd)
            .unwrap();
    }
    fn update(&mut self, mut emd: Emerald) {
        if self.ingame {
            self.tick += 1;

            self.update_enemies(&mut emd).unwrap();
            self.update_player(&mut emd).unwrap();
            self.update_dice(&mut emd).unwrap();

            self.extradmgtick += 1;
            if self.extradmgtick > 10000 {
                self.extradmgtick = 0;
                self.extradmg += 1;
                let snd = emd.loader().sound("sounds/extradmg.wav").unwrap();
                emd.audio().mixer("sfx6").unwrap().play(snd).unwrap();
            }

            if self.castle_hp < 1 {
                self.cause = String::from("You lost because too much enemies entered the castle!");
                self.ingame = false;
            }
        } else if !self.generated_menu {
            if emd.input().is_key_just_pressed(KeyCode::Space) {
                self.ingame = true;
                self.generated_menu = true;
            }
        } else if !self.played {
            let snd = emd.loader().sound("sounds/game_over.wav").unwrap();
            emd.audio().mixer("end").unwrap().play(snd).unwrap();
            self.played = true;
        }
    }
    fn draw(&mut self, mut emd: Emerald) {
        emd.graphics().begin().unwrap();
        if self.ingame {
            self.draw_tiles(&mut emd).unwrap();
            self.draw_dice(&mut emd).unwrap();
            self.draw_ui(&mut emd).unwrap();
        } else if !self.generated_menu {
            self.draw_menu(&mut emd).unwrap();
        } else {
            self.draw_game_over(&mut emd).unwrap();
        }
        emd.graphics().render().unwrap();
    }
}
