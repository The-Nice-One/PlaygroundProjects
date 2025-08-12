use crate::Window;
use emerald::*;

impl Window {
    pub fn draw_menu(&mut self, emd: &mut Emerald) -> Result<(), EmeraldError> {
        let font = emd.loader().font("fonts/ka1.ttf", 20)?;
        let mut title = Label::new("castle of the Dice", font.clone(), 50);
        title.color.r = 155;
        title.color.g = 103;
        title.color.b = 60;
        emd.graphics()
            .draw_label(&title, &Transform::from_translation((400.0, 500.0)))?;

        let msg = Label::new("Press space to play", font.clone(), 20);
        emd.graphics()
            .draw_label(&msg, &Transform::from_translation((400.0, 100.0)))?;
        Ok(())
    }
    pub fn draw_game_over(&mut self, emd: &mut Emerald) -> Result<(), EmeraldError> {
        let font = emd.loader().font("fonts/ka1.ttf", 20)?;
        let mut title = Label::new("castle of the Dice", font.clone(), 50);
        title.color.r = 155;
        title.color.g = 103;
        title.color.b = 60;
        emd.graphics()
            .draw_label(&title, &Transform::from_translation((400.0, 500.0)))?;

        let msg = Label::new(format!("Your score was {}", self.score), font.clone(), 20);
        emd.graphics()
            .draw_label(&msg, &Transform::from_translation((400.0, 300.0)))?;

        let msg = Label::new(self.cause.to_string(), font.clone(), 20);
        emd.graphics()
            .draw_label(&msg, &Transform::from_translation((400.0, 150.0)))?;
        Ok(())
    }
    pub fn draw_ui(&mut self, emd: &mut Emerald) -> Result<(), EmeraldError> {
        let font = emd.loader().font("fonts/ka1.ttf", 20)?;
        let mut msg = Label::new(
            format!("Enemy Spawns In {} seconds!", 10 - self.tick / 100),
            font.clone(),
            20,
        );
        msg.max_width = Some(800.0);
        emd.graphics()
            .draw_label(&msg, &Transform::from_translation((210.0, 590.0)))?;

        let mut msg = Label::new(format!("{} Extra Damage", self.extradmg), font.clone(), 20);
        msg.max_width = Some(800.0);
        emd.graphics()
            .draw_label(&msg, &Transform::from_translation((600.0, 590.0)))?;

        let mut msg = Label::new(
            format!("The Castle has {} Health!", self.castle_hp),
            font.clone(),
            20,
        );
        msg.max_width = Some(800.0);
        emd.graphics()
            .draw_label(&msg, &Transform::from_translation((190.0, 540.0)))?;

        let mut msg = Label::new(
            format!("Dice Reloads in {} seconds!", 3 - self.reload / 100),
            font.clone(),
            20,
        );
        msg.max_width = Some(800.0);
        emd.graphics()
            .draw_label(&msg, &Transform::from_translation((600.0, 540.0)))?;

        Ok(())
    }
    pub fn draw_tiles(&mut self, emd: &mut Emerald) -> Result<(), EmeraldError> {
        let backboard = emd.loader().sprite("textures/ui/backboard.png")?;
        let grass = emd.loader().sprite("textures/board/grass.png")?;
        let border = emd.loader().sprite("textures/board/border.png")?;

        let tower1 = emd.loader().sprite("textures/tower/tower1.png")?;
        let tower2 = emd.loader().sprite("textures/tower/tower2.png")?;
        let tower3 = emd.loader().sprite("textures/tower/tower3.png")?;
        let tower4 = emd.loader().sprite("textures/tower/tower4.png")?;

        let side_tower1 = emd.loader().sprite("textures/tower/side_tower1.png")?;
        let side_tower2 = emd.loader().sprite("textures/tower/side_tower2.png")?;

        let bridge1 = emd.loader().sprite("textures/tower/bridge1.png")?;
        let bridge2 = emd.loader().sprite("textures/tower/bridge2.png")?;

        let dirt1 = emd.loader().sprite("textures/board/dirt1.png")?;
        let dirt2 = emd.loader().sprite("textures/board/dirt2.png")?;

        let enemy1 = emd.loader().sprite("textures/enemy/enemy1.png")?;
        let enemy2 = emd.loader().sprite("textures/enemy/enemy2.png")?;

        let player1 = emd.loader().sprite("textures/player/player1.png")?;
        let player2 = emd.loader().sprite("textures/player/player2.png")?;
        let player3 = emd.loader().sprite("textures/player/player3.png")?;
        let player4 = emd.loader().sprite("textures/player/player4.png")?;

        let font = emd.loader().font("fonts/ka1.ttf", 20)?;

        let mut current_x = 25.0;
        let mut current_y = 575.0;
        for (index, tile) in self.data.iter_mut().enumerate() {
            let transform = Transform::from_translation((current_x, current_y));

            match tile.to_owned() {
                0 => emd.graphics().draw_sprite(&backboard, &transform)?,
                1 => emd.graphics().draw_sprite(&grass, &transform)?,
                2 => emd.graphics().draw_sprite(&border, &transform)?,
                4 => match self.player_direction {
                    1 => emd.graphics().draw_sprite(&player1, &transform)?,
                    2 => emd.graphics().draw_sprite(&player2, &transform)?,
                    3 => emd.graphics().draw_sprite(&player3, &transform)?,
                    4 => emd.graphics().draw_sprite(&player4, &transform)?,
                    _ => {}
                },
                31 => emd.graphics().draw_sprite(&tower1, &transform)?,
                32 => emd.graphics().draw_sprite(&tower2, &transform)?,
                33 => emd.graphics().draw_sprite(&tower3, &transform)?,
                34 => emd.graphics().draw_sprite(&tower4, &transform)?,
                35 => emd.graphics().draw_sprite(&side_tower1, &transform)?,
                36 => emd.graphics().draw_sprite(&side_tower2, &transform)?,
                37 => emd.graphics().draw_sprite(&bridge1, &transform)?,
                38 => emd.graphics().draw_sprite(&bridge2, &transform)?,
                51 => emd.graphics().draw_sprite(&dirt1, &transform)?,
                52 => emd.graphics().draw_sprite(&dirt2, &transform)?,
                61 => {
                    emd.graphics().draw_sprite(&enemy1, &transform)?;
                    let index = self
                        .enemy_position
                        .iter()
                        .position(|&r| r == index as u16)
                        .unwrap();
                    let msg = Label::new(format!("{}", self.enemy_health[index]), font.clone(), 20);
                    let transform = Transform::from_translation((
                        transform.translation.x,
                        transform.translation.y + 55.0,
                    ));
                    emd.graphics().draw_label(&msg, &transform)?;
                }
                62 => {
                    emd.graphics().draw_sprite(&enemy2, &transform)?;
                    let index = self
                        .enemy_position
                        .iter()
                        .position(|&r| r == index as u16)
                        .unwrap();
                    let msg = Label::new(format!("{}", self.enemy_health[index]), font.clone(), 20);
                    let transform = Transform::from_translation((
                        transform.translation.x,
                        transform.translation.y + 55.0,
                    ));
                    emd.graphics().draw_label(&msg, &transform)?;
                }
                _ => {}
            }
            current_x += 50.0;
            if current_x > 775.0 {
                current_x = 25.0;
                current_y -= 50.0;
            }
        }
        Ok(())
    }
    pub fn draw_dice(&mut self, emd: &mut Emerald) -> Result<(), EmeraldError> {
        let dice1 = emd.loader().sprite("textures/dice/dice1.png")?;
        let dice2 = emd.loader().sprite("textures/dice/dice2.png")?;
        let dice3 = emd.loader().sprite("textures/dice/dice3.png")?;
        let dice4 = emd.loader().sprite("textures/dice/dice4.png")?;
        let dice5 = emd.loader().sprite("textures/dice/dice5.png")?;
        let dice6 = emd.loader().sprite("textures/dice/dice6.png")?;

        for (index, _) in self.dice_x.iter().enumerate() {
            let sprite = match self.dice_side[index] {
                1 => &dice1,
                2 => &dice2,
                3 => &dice3,
                4 => &dice4,
                5 => &dice5,
                6 => &dice6,
                _ => &dice1,
            };

            emd.graphics().draw_sprite(
                sprite,
                &Transform::from_translation((self.dice_x[index], self.dice_y[index])),
            )?;
        }
        Ok(())
    }
}
