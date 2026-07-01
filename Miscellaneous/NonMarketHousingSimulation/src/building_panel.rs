//! UI systems for the building selection panel and persistent HUD.

use bevy::prelude::*;

use crate::building_colors::{tier_label, tier_ui_color};
use crate::components::{ConstructionSite, LotData, OwnershipTier, PlayerConverted};
use crate::events::{
    BuildOnLotAction, ConvertBuildingAction, SelectionClearedEvent, ShowBuildingPanel,
    ToggleSelectionModeEvent,
};
use crate::interaction::{SelectionBoxUi, SelectionMode};
use crate::plugin::{LodOptimizationEnabled, RenderMode};
use crate::sim_budget::{HousingStats, SimulationBudget};

const PANEL_BG: Color = Color::srgba(0.05, 0.05, 0.12, 0.88);

// Marker components

#[derive(Component)]
pub struct BuildingPanelRoot;
#[derive(Component)]
pub struct HousingHudRoot;
#[derive(Component)]
pub struct StatsHudRoot;
#[derive(Component)]
pub struct ControlsHudRoot;
#[derive(Component)]
pub struct LegendRoot;
#[derive(Component)]
pub struct ConvertButton {
    pub entity: Entity,
    pub bin: u32,
}
#[derive(Component)]
pub struct BuildButton {
    pub entity: Entity,
    pub bin: u32,
}
#[derive(Component)]
pub struct HudYearText;
#[derive(Component)]
pub struct HudBudgetText;
#[derive(Component)]
pub struct HudHousingText;
#[derive(Component)]
pub struct HudConvertedText;
#[derive(Component)]
pub struct HudBuiltText;
#[derive(Component)]
pub struct HudMaintenanceText;
#[derive(Component)]
pub struct HudShortageText;
#[derive(Component)]
pub struct HudHappinessText;
#[derive(Component)]
pub struct MassModeText;

// UI markers for the render-mode toggle and view-source banner
#[derive(Component)]
pub struct ToggleModeButton;
#[derive(Component)]
pub struct ToggleTrack;
#[derive(Component)]
pub struct ToggleKnob;
#[derive(Component)]
pub struct LodToggleButton;
#[derive(Component)]
pub struct LodToggleBox;
#[derive(Component)]
pub struct LodToggleFill;
#[derive(Component)]
pub struct LodToggleLabel;
#[derive(Component)]
pub struct ViewSourceButton;

// Panel systems

pub fn update_building_panel_system(
    mut events: MessageReader<ShowBuildingPanel>,
    lot_query: Query<(
        &LotData,
        Option<&PlayerConverted>,
        Option<&ConstructionSite>,
    )>,
    panel_query: Query<Entity, With<BuildingPanelRoot>>,
    budget: Res<SimulationBudget>,
    mut commands: Commands,
) {
    for ev in events.read() {
        for e in &panel_query {
            commands.entity(e).despawn();
        }
        let Ok((lot, converted, site)) = lot_query.get(ev.entity) else {
            continue;
        };
        spawn_panel(&mut commands, lot, converted, site, &budget, ev.entity);
    }
}

pub fn hide_building_panel_system(
    mut events: MessageReader<SelectionClearedEvent>,
    panel_query: Query<Entity, With<BuildingPanelRoot>>,
    mut commands: Commands,
) {
    for _ in events.read() {
        for e in &panel_query {
            commands.entity(e).despawn();
        }
    }
}

// HUD

pub fn spawn_hud(
    mut commands: Commands,
    mode: Res<SelectionMode>,
    lod: Res<LodOptimizationEnabled>,
) {
    // Selection box UI when dragging
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.3, 0.6, 1.0, 0.2)),
        BorderColor::all(Color::srgb(0.3, 0.6, 1.0)),
        SelectionBoxUi,
    ));

    // View source banner on top-left corner
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(16.0),
                width: Val::Px(164.0),
                height: Val::Px(26.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            Button,
            ViewSourceButton,
            BackgroundColor(PANEL_BG),
        ))
        .observe(on_view_source_clicked)
        .with_children(|bp| {
            bp.spawn((
                Text::new("View source on GitHub"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });

    // Controls HUD below source banner
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(42.0),
                left: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(4.0),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            ControlsHudRoot,
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("Controls"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.75)),
                Node {
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
            ));

            let control_text = [
                "WASD / Arrows: Pan",
                "Right Click + Drag: Orbit",
                "Scroll / R, F: Zoom",
                "Q / E: Rotate",
                "Tab: Toggle 2D/3D",
            ];

            for text in control_text {
                p.spawn((
                    Text::new(text),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            }
        });

    // render mode toggle and LOD checkbox in top center
    commands
        .spawn((Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-104.0)),
            width: Val::Px(208.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(8.0),
            ..default()
        },))
        .with_children(|top| {
            top.spawn((
                Node {
                    width: Val::Px(112.0),
                    height: Val::Px(30.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::horizontal(Val::Px(8.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                Button,
                ToggleModeButton,
                BackgroundColor(PANEL_BG),
            ))
            .observe(on_toggle_mode_clicked)
            .with_children(|bp| {
                bp.spawn((
                    Text::new("2D"),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                bp.spawn((
                    Node {
                        width: Val::Px(50.0),
                        height: Val::Px(20.0),
                        margin: UiRect::horizontal(Val::Px(5.0)),
                        border_radius: BorderRadius::all(Val::Px(20.0)),
                        position_type: PositionType::Relative,
                        ..default()
                    },
                    ToggleTrack,
                    BackgroundColor(Color::srgba(0.22, 0.22, 0.30, 0.98)),
                ))
                .with_children(|tb| {
                    tb.spawn((
                        Node {
                            width: Val::Px(14.0),
                            height: Val::Px(14.0),
                            position_type: PositionType::Absolute,
                            border_radius: BorderRadius::all(Val::Px(999.0)),
                            ..default()
                        },
                        ToggleKnob,
                        BackgroundColor(Color::WHITE),
                    ));
                });

                bp.spawn((
                    Text::new("3D"),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });

            top.spawn((
                Button,
                Node {
                    width: Val::Px(64.0),
                    height: Val::Px(30.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                LodToggleButton,
                BackgroundColor(PANEL_BG),
            ))
            .observe(on_toggle_lod_clicked)
            .with_children(|bp| {
                bp.spawn((
                    Node {
                        width: Val::Px(16.0),
                        height: Val::Px(16.0),
                        margin: UiRect::right(Val::Px(5.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border_radius: BorderRadius::all(Val::Px(5.0)),
                        ..default()
                    },
                    LodToggleBox,
                    BackgroundColor(Color::srgba(0.22, 0.22, 0.30, 0.98)),
                ))
                .with_children(|box_node| {
                    box_node.spawn((
                        Node {
                            width: Val::Px(10.0),
                            height: Val::Px(10.0),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        LodToggleFill,
                        BackgroundColor(if lod.0 {
                            Color::WHITE
                        } else {
                            Color::srgba(0.0, 0.0, 0.0, 0.0)
                        }),
                    ));
                });
                bp.spawn((
                    Text::new("LOD"),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    LodToggleLabel,
                ));
            });
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                right: Val::Px(16.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(20.0),
                padding: UiRect::all(Val::Px(10.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            HousingHudRoot,
        ))
        .with_children(|p| {
            hud_label(p, "Year", HudYearText, "2026");
            hud_label(p, "Budget", HudBudgetText, "$500 M");
            hud_label(p, "Non-market", HudHousingText, "—");

            // Mass selection toggle button
            p.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 1.0)),
            ))
            .observe(on_mass_toggle_clicked)
            .with_children(|bp| {
                bp.spawn((
                    Text::new(mode_label(*mode)),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    MassModeText,
                ));
            });
        });

    // Secondary stats HUD
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(76.0),
                right: Val::Px(16.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(20.0),
                padding: UiRect::all(Val::Px(10.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            StatsHudRoot,
        ))
        .with_children(|p| {
            hud_label(p, "Converted", HudConvertedText, "0");
            hud_label(p, "New Built", HudBuiltText, "0");
            hud_label(p, "Maintenance", HudMaintenanceText, "$0/yr");
            hud_label(p, "Shortage", HudShortageText, "350 K");
            hud_label(p, "Happiness", HudHappinessText, "50%");
        });

    // Legend in bottom left corner
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(16.0),
                left: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(6.0),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                min_width: Val::Px(180.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            LegendRoot,
        ))
        .with_children(|lp| {
            lp.spawn((
                Text::new("Building Legend"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.75)),
                Node {
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
            ));

            // Standard tiers
            for tier in 0..=5 {
                legend_row(lp, tier_label(tier), tier_ui_color(tier));
            }
            divider(lp);
            // Simulation states
            legend_row(lp, "Player Converted", Color::srgb(0.10, 0.88, 0.82));
            legend_row(lp, "Under Construction", Color::srgb(0.95, 0.55, 0.10));
        });
}

pub fn update_hud_system(
    budget: Res<SimulationBudget>,
    stats: Res<HousingStats>,
    mode: Res<SelectionMode>,
    mut year_q: Query<&mut Text, With<HudYearText>>,
    mut budget_q: Query<&mut Text, (With<HudBudgetText>, Without<HudYearText>)>,
    mut housing_q: Query<
        &mut Text,
        (
            With<HudHousingText>,
            Without<HudYearText>,
            Without<HudBudgetText>,
        ),
    >,
    mut mode_q: Query<
        &mut Text,
        (
            With<MassModeText>,
            Without<HudYearText>,
            Without<HudBudgetText>,
            Without<HudHousingText>,
        ),
    >,
    mut conv_q: Query<
        &mut Text,
        (
            With<HudConvertedText>,
            Without<HudYearText>,
            Without<HudBudgetText>,
            Without<HudHousingText>,
            Without<MassModeText>,
        ),
    >,
    mut built_q: Query<
        &mut Text,
        (
            With<HudBuiltText>,
            Without<HudYearText>,
            Without<HudBudgetText>,
            Without<HudHousingText>,
            Without<MassModeText>,
            Without<HudConvertedText>,
        ),
    >,
    mut maint_q: Query<
        &mut Text,
        (
            With<HudMaintenanceText>,
            Without<HudYearText>,
            Without<HudBudgetText>,
            Without<HudHousingText>,
            Without<MassModeText>,
            Without<HudConvertedText>,
            Without<HudBuiltText>,
        ),
    >,
    mut short_q: Query<
        &mut Text,
        (
            With<HudShortageText>,
            Without<HudYearText>,
            Without<HudBudgetText>,
            Without<HudHousingText>,
            Without<MassModeText>,
            Without<HudConvertedText>,
            Without<HudBuiltText>,
            Without<HudMaintenanceText>,
        ),
    >,
    mut happy_q: Query<
        &mut Text,
        (
            With<HudHappinessText>,
            Without<HudYearText>,
            Without<HudBudgetText>,
            Without<HudHousingText>,
            Without<MassModeText>,
            Without<HudConvertedText>,
            Without<HudBuiltText>,
            Without<HudMaintenanceText>,
            Without<HudShortageText>,
        ),
    >,
) {
    for mut t in &mut year_q {
        t.0 = format!("{}", budget.game_year);
    }
    for mut t in &mut budget_q {
        t.0 = fmt_dollars(budget.available);
    }
    for mut t in &mut housing_q {
        t.0 = format!(
            "{} ({:.1}%)",
            fmt_large(stats.total_nonmarket()),
            stats.nonmarket_pct()
        );
    }
    for mut t in &mut conv_q {
        t.0 = fmt_large(stats.converted_units);
    }
    for mut t in &mut built_q {
        t.0 = fmt_large(stats.built_units);
    }
    for mut t in &mut maint_q {
        t.0 = format!("{}/yr", fmt_dollars(budget.annual_costs));
    }
    for mut t in &mut short_q {
        t.0 = fmt_large(stats.current_shortage as u32);
    }
    for mut t in &mut happy_q {
        t.0 = format!("{:.0}%", stats.happiness());
    }
    for mut t in &mut mode_q {
        t.0 = mode_label(*mode).to_string();
    }
}

fn mode_label(mode: SelectionMode) -> &'static str {
    match mode {
        SelectionMode::Single => "Single Select",
        SelectionMode::Mass => "Mass Convert (Drag)",
    }
}

// Panel builder

fn spawn_panel(
    commands: &mut Commands,
    lot: &LotData,
    converted: Option<&PlayerConverted>,
    site: Option<&ConstructionSite>,
    budget: &SimulationBudget,
    entity: Entity,
) {
    let tier = lot.ownership.as_u8();
    let can_conv = matches!(lot.ownership, OwnershipTier::Market)
        && lot.units_res > 0
        && converted.is_none()
        && site.is_none();
    let can_build = lot.ownership.is_parking() && converted.is_none() && site.is_none();

    let cost = if can_conv {
        lot.conversion_one_time_cost()
    } else if can_build {
        lot.construction_one_time_cost()
    } else {
        0
    };
    let affordable = budget.can_afford(cost);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                bottom: Val::Px(16.0),
                width: Val::Px(340.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(16.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            BuildingPanelRoot,
        ))
        .with_children(|p| {
            let badge_color = if converted.is_some() || site.is_some() {
                Color::srgb(0.10, 0.88, 0.82)
            } else {
                tier_ui_color(tier)
            };

            // Owner name and color indicator header.
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|hp| {
                hp.spawn((
                    Text::new(lot.owner.as_str()),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                hp.spawn((
                    Node {
                        width: Val::Px(12.0),
                        height: Val::Px(12.0),
                        border_radius: BorderRadius::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(badge_color),
                ));
            });

            divider(p);

            // Building details
            stat_row(
                p,
                "Units (residential / total)",
                &format!("{} / {}", lot.units_res, lot.units_total),
            );
            stat_row(
                p,
                "Floors / Height",
                &format!("{} / {:.1}m", lot.num_floors, lot.height_m),
            );

            divider(p);

            // Financial data
            stat_row(
                p,
                "Estimated Market Value",
                &fmt_dollars(lot.estimated_market_value() as i64),
            );
            stat_row(p, "Assessed", &fmt_dollars(lot.assessed as i64));
            if lot.exempt > 0 {
                stat_row(
                    p,
                    "Exempt",
                    &format!("{} ({}%)", fmt_dollars(lot.exempt as i64), lot.exempt_pct()),
                );
            }

            divider(p);

            // Lot and zoning details
            stat_row(p, "Lot Area", &format!("{} sq ft", fmt_large(lot.lot_area)));
            if lot.ownership.is_parking() {
                stat_row(p, "Buildable Units", &lot.buildable_units().to_string());
            }
            stat_row(p, "BBL / BIN", &format!("{} / {}", lot.bbl, lot.bin));

            // Action area
            if can_conv || can_build {
                divider(p);

                let (label, cost_line, annual_line) = if can_conv {
                    (
                        "Convert to Non-Market",
                        format!(
                            "One-time: {}",
                            fmt_dollars(lot.conversion_one_time_cost() as i64)
                        ),
                        format!(
                            "Annual:   {}/yr",
                            fmt_dollars(lot.conversion_annual_cost() as i64)
                        ),
                    )
                } else {
                    (
                        "Build Affordable Housing",
                        format!(
                            "One-time: {}",
                            fmt_dollars(lot.construction_one_time_cost() as i64)
                        ),
                        format!(
                            "Annual:   {}/yr",
                            fmt_dollars(lot.construction_annual_cost() as i64)
                        ),
                    )
                };

                p.spawn((
                    Text::new(cost_line),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.70, 0.70, 0.70)),
                ));
                p.spawn((
                    Text::new(annual_line),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.70, 0.70, 0.70)),
                ));

                let btn_color = if affordable {
                    Color::srgb(0.12, 0.55, 0.30)
                } else {
                    Color::srgb(0.40, 0.20, 0.20)
                };
                let btn_text = if affordable {
                    label
                } else {
                    "Insufficient budget"
                };
                let bin = lot.bin;

                let mut btn = p.spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(36.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        margin: UiRect::top(Val::Px(6.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(btn_color),
                ));

                if affordable {
                    if can_conv {
                        btn.insert(ConvertButton { entity, bin });
                        btn.observe(on_convert_clicked);
                    } else {
                        btn.insert(BuildButton { entity, bin });
                        btn.observe(on_build_clicked);
                    }
                }

                btn.with_children(|bp| {
                    bp.spawn((
                        Text::new(btn_text),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            } else if converted.is_some() {
                divider(p);
                p.spawn((
                    Text::new("Already non-market housing."),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.60, 0.90, 0.72)),
                ));
            } else if let Some(s) = site {
                divider(p);
                p.spawn((
                    Text::new(format!("Completes year {}.", s.complete_at_year)),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.90, 0.72, 0.30)),
                ));
            }
        });
}

// Button observers

fn on_mass_toggle_clicked(
    _click: On<Pointer<Click>>,
    mut writer: MessageWriter<ToggleSelectionModeEvent>,
) {
    writer.write(ToggleSelectionModeEvent);
}

fn on_toggle_mode_clicked(
    _click: On<Pointer<Click>>,
    mut next_state: ResMut<NextState<RenderMode>>,
    state: Res<State<RenderMode>>,
) {
    next_state.set(match state.get() {
        RenderMode::Mode3D => RenderMode::Mode2D,
        RenderMode::Mode2D => RenderMode::Mode3D,
    });
}

fn on_toggle_lod_clicked(_click: On<Pointer<Click>>, mut lod: ResMut<LodOptimizationEnabled>) {
    lod.0 = !lod.0;
}

fn on_view_source_clicked(_click: On<Pointer<Click>>) {
    webbrowser::open("https://github.com/The-Nice-One/PlaygroundProjects/tree/main/Miscellaneous/NonMarketHousingSimulation").unwrap();
}

fn on_convert_clicked(
    click: On<Pointer<Click>>,
    btns: Query<&ConvertButton>,
    mut writer: MessageWriter<ConvertBuildingAction>,
) {
    if let Ok(btn) = btns.get(click.entity) {
        writer.write(ConvertBuildingAction {
            entity: btn.entity,
            bin: btn.bin,
        });
    }
}

fn on_build_clicked(
    click: On<Pointer<Click>>,
    btns: Query<&BuildButton>,
    mut writer: MessageWriter<BuildOnLotAction>,
) {
    if let Ok(btn) = btns.get(click.entity) {
        writer.write(BuildOnLotAction {
            entity: btn.entity,
            bin: btn.bin,
        });
    }
}

// UI helpers

fn legend_row(p: &mut ChildSpawnerCommands, label: &str, color: Color) {
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(10.0),
        ..default()
    })
    .with_children(|row| {
        // Color icon
        row.spawn((
            Node {
                width: Val::Px(12.0),
                height: Val::Px(12.0),
                border_radius: BorderRadius::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(color),
        ));
        row.spawn((
            Text::new(label),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    });
}

fn stat_row(p: &mut ChildSpawnerCommands, label: &str, value: &str) {
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        width: Val::Percent(100.0),
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            Text::new(label),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.65, 0.65, 0.65)),
        ));
        row.spawn((
            Text::new(value),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    });
}

fn divider(p: &mut ChildSpawnerCommands) {
    p.spawn(Node {
        width: Val::Percent(100.0),
        height: Val::Px(1.0),
        margin: UiRect::axes(Val::Px(0.0), Val::Px(6.0)),
        ..default()
    });
}

fn hud_label<T: Component>(p: &mut ChildSpawnerCommands, header: &str, marker: T, initial: &str) {
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Start,
        ..default()
    })
    .with_children(|col| {
        col.spawn((
            Text::new(header),
            TextFont {
                font_size: 10.0,
                ..default()
            },
            TextColor(Color::srgb(0.55, 0.55, 0.65)),
        ));
        col.spawn((
            Text::new(initial),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::WHITE),
            marker,
        ));
    });
}

// Formatters

fn fmt_dollars(v: i64) -> String {
    if v.abs() >= 1_000_000_000 {
        format!("${:.1} B", v as f64 / 1e9)
    } else if v.abs() >= 1_000_000 {
        format!("${:.1} M", v as f64 / 1e6)
    } else if v.abs() >= 1_000 {
        format!("${:.0} K", v as f64 / 1e3)
    } else {
        format!("${v}")
    }
}

fn fmt_large(v: u32) -> String {
    if v >= 1_000_000 {
        format!("{:.1} M", v as f64 / 1e6)
    } else if v >= 1_000 {
        format!("{:.0} K", v as f64 / 1e3)
    } else {
        v.to_string()
    }
}
