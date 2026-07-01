//! Global event/message definitions for tile loading and simulation actions.

use bevy::prelude::*;

use crate::components::TileCoord;

// Tile lifecycle

#[derive(Event, Message, Debug, Clone)]
pub struct LoadTileEvent {
    pub coord: TileCoord,
}

#[derive(Event, Message, Debug, Clone)]
pub struct UnloadTileEvent {
    pub coord: TileCoord,
}

// Interaction

#[derive(Event, Message, Debug, Default, Clone)]
pub struct SelectionClearedEvent;

/// Fired when the user clicks a building proxy that has `LotData`.
/// Used by `building_panel::update_building_panel_system`.
#[derive(Event, Message, Debug, Clone)]
pub struct ShowBuildingPanel {
    pub entity: Entity,
}

// Simulation actions

/// Written by the "Convert to Non-Market" button in the building panel.
/// Used by `sim_budget::process_convert_actions_system`.
#[derive(Event, Message, Debug, Clone)]
pub struct ConvertBuildingAction {
    pub entity: Entity,
    pub bin: u32,
}

/// Written by the "Build Housing" button in the building panel.
/// Used by `sim_budget::process_build_actions_system`.
#[derive(Event, Message, Debug, Clone)]
pub struct BuildOnLotAction {
    pub entity: Entity,
    pub bin: u32,
}

#[derive(Event, Message, Debug, Default, Clone)]
pub struct ToggleSelectionModeEvent;
