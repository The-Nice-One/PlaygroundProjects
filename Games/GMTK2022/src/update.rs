use crate::Window;
use emerald::{rand::gen_range, *};

impl Window {
    pub fn update_enemies(&mut self, emd: &mut Emerald) -> Result<(), EmeraldError> {
        if self.tick >= 1000 {
            self.tick = self.spawn_tick;

            if self.spawn_tick < 750 {
                self.spawn_tick += 25;
            }

            let snd = emd.loader().sound("sounds/spawn.wav")?;
            emd.audio().mixer("sfx1")?.play(snd)?;

            // enemy from left movement
            let mut left: Vec<usize> = vec![];
            for (index, _) in self.data.iter().enumerate() {
                if self.data[index] == 61 {
                    left.push(index)
                }
            }

            for (index, _) in left.iter().enumerate() {
                if self.data[left[index] + 1] == 4 {
                    self.cause = String::from("You lost because you were eaten by a enemy!");
                    self.ingame = false;
                }
                if self.shadow[left[index] + 1] == 1 {
                    let enemy_index = self
                        .enemy_position
                        .iter()
                        .position(|&r| r == left[index] as u16)
                        .unwrap();

                    self.data[left[index]] = self.shadow[left[index]];
                    self.data[left[index] + 1] = 61;
                    self.enemy_position[enemy_index] += 1;
                } else {
                    let enemy_index = self
                        .enemy_position
                        .iter()
                        .position(|&r| r == left[index] as u16)
                        .unwrap();
                    self.enemy_position.remove(enemy_index);
                    self.enemy_health.remove(enemy_index);

                    self.data[left[index]] = self.shadow[left[index]];
                    self.castle_hp -= 1;
                }
            }

            // enemy from right movement - fixed the bug where the game would crash if an
            // enemy was spawned on the right side, as previously the enemy had the same
            // logic as a enemy on the left side; fixed after the Game Jam.
            let mut right: Vec<usize> = vec![];
            for (index, _) in self.data.iter().enumerate() {
                if self.data[index] == 62 {
                    right.push(index)
                }
            }
            for (index, _) in right.iter().enumerate() {
                if self.data[right[index] - 1] == 4 {
                    self.cause = String::from("You lost because you were eaten by a enemy!");
                    self.ingame = false;
                }
                if self.shadow[right[index] - 1] == 1 {
                    let enemy_index = self
                        .enemy_position
                        .iter()
                        .position(|&r| r == right[index] as u16)
                        .unwrap();

                    self.data[right[index]] = self.shadow[right[index]];
                    self.data[right[index] - 1] = 62;
                    self.enemy_position[enemy_index] -= 1;
                } else {
                    let enemy_index = self
                        .enemy_position
                        .iter()
                        .position(|&r| r == right[index] as u16)
                        .unwrap();
                    self.enemy_position.remove(enemy_index);
                    self.enemy_health.remove(enemy_index);

                    self.data[right[index]] = self.shadow[right[index]];
                    self.castle_hp -= 1;
                }
            }

            let mut available_spawns: Vec<usize> = vec![];
            for (index, _) in self.data.iter().enumerate() {
                if self.data[index] == 51 || self.data[index] == 52 {
                    available_spawns.push(index);
                }
            }

            let spawn_index = gen_range(0, available_spawns.len());
            let tile_index = available_spawns[spawn_index];

            self.enemy_health.push(6);
            self.enemy_position.push(tile_index as u16);

            if tile_index % 2 == 0 {
                self.data[tile_index] = 62;
            } else {
                self.data[tile_index] = 61;
            }
        }
        Ok(())
    }
    pub fn update_player(&mut self, emd: &mut Emerald) -> Result<(), EmeraldError> {
        let last_position = self.data.iter().position(|&r| r == 4).unwrap_or_default();
        let mut new_position = self.data.iter().position(|&r| r == 4).unwrap_or_default();
        let mut moved = false;

        // up
        if emd.input().is_key_just_pressed(KeyCode::Up) {
            moved = true;
            new_position -= 16;
            self.player_direction = 1;
        }
        // down
        if emd.input().is_key_just_pressed(KeyCode::Down) {
            moved = true;
            new_position += 16;
            self.player_direction = 3;
        }
        // left
        if emd.input().is_key_just_pressed(KeyCode::Left) {
            moved = true;
            new_position -= 1;
            self.player_direction = 4;
        }
        // right
        if emd.input().is_key_just_pressed(KeyCode::Right) {
            moved = true;
            new_position += 1;
            self.player_direction = 2;
        }

        if moved {
            if self.data[new_position] != 1 {
                // when going through the bridge
                if self.data[new_position] == 38 {
                    self.data[new_position + 2] = 4;
                    self.data[last_position] = 1;
                    let snd = emd.loader().sound("sounds/bridge.wav")?;

                    emd.audio().mixer("sfx4")?.play(snd)?;
                } else if self.data[new_position] == 37 {
                    self.data[new_position - 2] = 4;
                    self.data[last_position] = 1;
                    let snd = emd.loader().sound("sounds/bridge.wav")?;

                    emd.audio().mixer("sfx4")?.play(snd)?;
                } else {
                    // when player just cant move
                    self.data[last_position] = 4;
                    let snd = emd.loader().sound("sounds/cant_move.wav")?;

                    emd.audio().mixer("sfx2")?.play(snd)?;

                    emd.audio().mixer("sfx2")?.set_volume(1.5)?;
                }
            } else {
                self.data[new_position] = 4;
                self.data[last_position] = 1;
                let snd = emd.loader().sound("sounds/move.wav")?;
                emd.audio().mixer("sfx3")?.play(snd)?;
            }
        }

        if self.reload < 300 {
            self.reload += 1;
        }

        // when shooting
        if emd.input().is_key_just_pressed(KeyCode::Space) && self.reload > 299 {
            self.reload = 0;
            let mut cx = 25.0;
            let mut cy = 575.0;

            for tile in self.data.iter_mut() {
                if *tile == 4 {
                    break;
                }

                cx += 50.0;
                if cx > 775.0 {
                    cx = 25.0;
                    cy -= 50.0;
                }
            }
            self.dice_x.push(cx);
            self.dice_y.push(cy);
            self.dice_side.push(1);
            self.dice_direction.push(self.player_direction);
        }
        Ok(())
    }
    pub fn update_dice(&mut self, emd: &mut Emerald) -> Result<(), EmeraldError> {
        // update dice to random side
        self.dice_tick += 1;
        if self.dice_tick >= 25 {
            self.dice_tick = 0;
            for (index, _) in self.dice_x.iter().enumerate() {
                self.dice_side[index] = gen_range(1, 6);
            }
        }

        // dice movement
        for (index, _) in self.dice_side.iter().enumerate() {
            let (dice_x, dice_y) = match self.dice_direction[index] {
                1 => (0.0, 2.0),
                2 => (2.0, 0.0),
                3 => (0.0, -2.0),
                4 => (-2.0, 0.0),
                _ => (0.0, 0.0),
            };
            self.dice_x[index] += dice_x;
            self.dice_y[index] += dice_y;
        }

        // dice collision
        for (index, _) in self.dice_direction.clone().iter().enumerate() {
            if self.dice_x[index] as i32 % 25 == 0 && self.dice_y[index] as i32 % 25 == 0 {
                let mut cx = 25.0;
                let mut cy = 575.0;
                let mut tile: usize = 0;
                for _ in self.data.iter() {
                    if cx == self.dice_x[index] && cy == self.dice_y[index] {
                        break;
                    }
                    tile += 1;
                    cx += 50.0;
                    if cx > 775.0 {
                        cx = 25.0;
                        cy -= 50.0;
                    }
                }

                if self.data[tile] != 1 {
                    if self.data[tile] == 61 || self.data[tile] == 62 {
                        let tile_index = self
                            .enemy_position
                            .iter()
                            .position(|&r| r == tile as u16)
                            .unwrap();
                        if (self.enemy_health[tile_index] as i16
                            - (self.dice_side[index] as i16 + self.extradmg as i16))
                            < 1
                        {
                            self.enemy_position.remove(tile_index);
                            self.enemy_health.remove(tile_index);
                            self.score += 1;

                            self.data[tile] = self.shadow[tile];
                        } else {
                            self.enemy_health[tile_index] -= self.dice_side[index] + self.extradmg;
                        }
                    }
                    self.dice_x.remove(index);
                    self.dice_y.remove(index);
                    self.dice_side.remove(index);
                    self.dice_direction.remove(index);

                    let snd = emd.loader().sound("sounds/hit.wav")?;
                    emd.audio().mixer("sfx5")?.play(snd)?;
                    emd.audio().mixer("sfx5")?.set_volume(1.5)?;
                }
            }
        }
        Ok(())
    }
}
