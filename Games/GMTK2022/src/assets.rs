use emerald::*;

pub fn pack_all_assets(emd: &mut Emerald) -> Result<(), EmeraldError> {
    // Font
    emd.loader().pack_asset_bytes(
        "./assets/fonts/ka1.ttf",
        include_bytes!("../assets/fonts/ka1.ttf").to_vec(),
    )?;

    // Sounds
    emd.loader().pack_asset_bytes(
        "./assets/sounds/HoliznaCC0 - Complications.ogg",
        include_bytes!("../assets/sounds/HoliznaCC0 - Complications.ogg").to_vec(),
    )?;
    emd.loader().pack_asset_bytes(
        "./assets/sounds/spawn.wav",
        include_bytes!("../assets/sounds/spawn.wav").to_vec(),
    )?;
    emd.loader().pack_asset_bytes(
        "./assets/sounds/move.wav",
        include_bytes!("../assets/sounds/move.wav").to_vec(),
    )?;
    emd.loader().pack_asset_bytes(
        "./assets/sounds/bridge.wav",
        include_bytes!("../assets/sounds/bridge.wav").to_vec(),
    )?;
    emd.loader().pack_asset_bytes(
        "./assets/sounds/hit.wav",
        include_bytes!("../assets/sounds/hit.wav").to_vec(),
    )?;
    emd.loader().pack_asset_bytes(
        "./assets/sounds/extradmg.wav",
        include_bytes!("../assets/sounds/extradmg.wav").to_vec(),
    )?;
    emd.loader().pack_asset_bytes(
        "./assets/sounds/cant_move.wav",
        include_bytes!("../assets/sounds/cant_move.wav").to_vec(),
    )?;
    emd.loader().pack_asset_bytes(
        "./assets/sounds/game_over.wav",
        include_bytes!("../assets/sounds/game_over.wav").to_vec(),
    )?;

    // Textures - Board
    emd.loader().pack_asset_bytes(
        "./assets/textures/board/border.png",
        include_bytes!("../assets/textures/board/border.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/board/dirt1.png",
        include_bytes!("../assets/textures/board/dirt1.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/board/dirt2.png",
        include_bytes!("../assets/textures/board/dirt2.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/board/grass.png",
        include_bytes!("../assets/textures/board/grass.png").to_vec(),
    )?;

    // Textures - Dice
    emd.loader().pack_asset_bytes(
        "./assets/textures/dice/dice1.png",
        include_bytes!("../assets/textures/dice/dice1.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/dice/dice2.png",
        include_bytes!("../assets/textures/dice/dice2.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/dice/dice3.png",
        include_bytes!("../assets/textures/dice/dice3.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/dice/dice4.png",
        include_bytes!("../assets/textures/dice/dice4.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/dice/dice5.png",
        include_bytes!("../assets/textures/dice/dice5.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/dice/dice6.png",
        include_bytes!("../assets/textures/dice/dice6.png").to_vec(),
    )?;

    // Textures - Enemies
    emd.loader().pack_asset_bytes(
        "./assets/textures/enemy/enemy1.png",
        include_bytes!("../assets/textures/enemy/enemy1.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/enemy/enemy2.png",
        include_bytes!("../assets/textures/enemy/enemy2.png").to_vec(),
    )?;

    // Textures - Player
    emd.loader().pack_asset_bytes(
        "./assets/textures/player/player1.png",
        include_bytes!("../assets/textures/player/player1.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/player/player2.png",
        include_bytes!("../assets/textures/player/player2.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/player/player3.png",
        include_bytes!("../assets/textures/player/player3.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/player/player4.png",
        include_bytes!("../assets/textures/player/player4.png").to_vec(),
    )?;

    // Textures - Towers
    emd.loader().pack_asset_bytes(
        "./assets/textures/tower/tower1.png",
        include_bytes!("../assets/textures/tower/tower1.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/tower/tower2.png",
        include_bytes!("../assets/textures/tower/tower2.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/tower/tower3.png",
        include_bytes!("../assets/textures/tower/tower3.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/tower/tower4.png",
        include_bytes!("../assets/textures/tower/tower4.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/tower/bridge1.png",
        include_bytes!("../assets/textures/tower/bridge1.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/tower/bridge2.png",
        include_bytes!("../assets/textures/tower/bridge2.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/tower/side_tower1.png",
        include_bytes!("../assets/textures/tower/side_tower1.png").to_vec(),
    )?;

    emd.loader().pack_asset_bytes(
        "./assets/textures/tower/side_tower2.png",
        include_bytes!("../assets/textures/tower/side_tower2.png").to_vec(),
    )?;

    Ok(())
}
