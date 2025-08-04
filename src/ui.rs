use bevy::{color::palettes::css, prelude::*};

use crate::player::Player;

const CROSSHAIR: f32 = 20.0;
const HPBAR: f32 = 50.0;
const INVENTORY: f32 = 100.0;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, update_inventory_view);
        app.add_systems(Update, update_hpbar);

        app.add_event::<UserNotify>();
        app.add_systems(Update, update_notification);
    }
}

#[derive(Component)]
struct InventoryView(pub usize);

fn update_inventory_view(
    mut views: Query<(&InventoryView, &mut BorderColor)>,
    player: Single<&Player>,
) {
    for (view, mut border) in &mut views {
        border.0 = if view.0 == player.active_slot {
            Color::WHITE
        } else {
            Color::NONE
        };
        player.weapons[view.0];
    }
}

#[derive(Component)]
struct HpBarText;

#[derive(Component)]
struct HpBarIndicator;

fn update_hpbar(
    mut hpbar_text: Single<&mut Text, With<HpBarText>>,
    mut hpbar_indicator: Single<&mut Node, With<HpBarIndicator>>,
    player: Single<&Player>,
) {
    hpbar_indicator.width = Val::Percent(100.0 * player.hp / player.max_hp);
    hpbar_text.0 = format!("{} / {}", player.hp, player.max_hp);
}

#[derive(Component)]
struct UserNotifyLine1;

#[derive(Component)]
struct UserNotifyLine2;

#[derive(Event)]
pub struct UserNotify(pub String, pub String);

fn update_notification(
    line1: Single<(&mut Text, &mut TextColor), With<UserNotifyLine1>>,
    line2: Single<(&mut Text, &mut TextColor), (With<UserNotifyLine2>, Without<UserNotifyLine1>)>,
    mut notifications: EventReader<UserNotify>,
    time: Res<Time>,
) {
    let (mut line1, mut color1) = line1.into_inner();
    let (mut line2, mut color2) = line2.into_inner();

    let mut alpha = color1.alpha();

    if notifications.is_empty() {
        alpha -= time.delta_secs();
    } else {
        alpha = 1.0;
    }

    alpha = alpha.clamp(0.0, 1.0);

    for notification in notifications.read() {
        line1.0 = notification.0.clone();
        line2.0 = notification.1.clone();
    }

    color1.set_alpha(alpha);
    color2.set_alpha(alpha);
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let font = assets.load("./fonts/NotoSerif-Regular.ttf");
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            position_type: PositionType::Relative,
            ..Default::default()
        },
        children![
            crosshair(),
            hpbar(font.clone()),
            // inventory(),
            user_notify(font.clone()),
            // user_story(font.clone()),
        ],
    ));
}

fn crosshair() -> impl Bundle {
    (
        Node {
            width: Val::Px(CROSSHAIR),
            height: Val::Px(CROSSHAIR),
            display: Display::Flex,
            position_type: PositionType::Relative,
            margin: UiRect::all(Val::Auto),
            ..Default::default()
        },
        children![
            (
                Node {
                    width: Val::Px(2.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    margin: UiRect::all(Val::Auto),
                    left: Val::ZERO,
                    right: Val::ZERO,
                    ..Default::default()
                },
                BackgroundColor(Color::WHITE),
            ),
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(2.0),
                    position_type: PositionType::Absolute,
                    margin: UiRect::all(Val::Auto),
                    top: Val::ZERO,
                    bottom: Val::ZERO,
                    ..Default::default()
                },
                BackgroundColor(Color::WHITE),
            )
        ],
    )
}

fn hpbar(font: Handle<Font>) -> impl Bundle {
    let gap = 10.0;
    let width = 300.0;
    let height = HPBAR;

    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(height),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::End,
            align_items: AlignItems::Center,
            position_type: PositionType::Absolute,
            padding: UiRect::all(Val::Px(gap)),
            top: Val::ZERO,
            ..Default::default()
        },
        children![
            (
                Text::new("HP: "),
                TextFont {
                    font: font.clone(),
                    font_size: height * 0.5,
                    ..Default::default()
                }
            ),
            (
                Node {
                    width: Val::Px(width),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Relative,
                    ..Default::default()
                },
                BackgroundColor(css::MAROON.into()),
                children![
                    (
                        HpBarIndicator,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..Default::default()
                        },
                        BackgroundColor(css::RED.into()),
                    ),
                    (
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            position_type: PositionType::Absolute,
                            left: Val::ZERO,
                            top: Val::ZERO,
                            display: Display::Flex,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..Default::default()
                        },
                        children![(
                            HpBarText,
                            Text::new(""),
                            TextFont {
                                font: font.clone(),
                                font_size: height * 0.5,
                                ..Default::default()
                            }
                        )]
                    )
                ]
            )
        ],
    )
}

fn inventory() -> impl Bundle {
    let gap = 10.0;
    let height = INVENTORY;
    let width = height - gap * 2.0;

    (
        Node {
            width: Val::Auto,
            height: Val::Px(height),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            position_type: PositionType::Absolute,
            bottom: Val::ZERO,
            padding: UiRect::all(Val::Px(gap)),
            column_gap: Val::Px(gap),
            border: UiRect::all(Val::Px(2.0)),
            ..Default::default()
        },
        BorderColor(Color::WHITE),
        children![
            (
                InventoryView(0),
                Node {
                    width: Val::Px(width),
                    height: Val::Percent(100.0),
                    border: UiRect::all(Val::Px(5.0)),
                    ..Default::default()
                },
            ),
            (
                InventoryView(1),
                Node {
                    width: Val::Px(width),
                    height: Val::Percent(100.0),
                    border: UiRect::all(Val::Px(5.0)),
                    ..Default::default()
                },
            ),
            (
                InventoryView(2),
                Node {
                    width: Val::Px(width),
                    height: Val::Percent(100.0),
                    border: UiRect::all(Val::Px(5.0)),
                    ..Default::default()
                },
            ),
            (
                InventoryView(3),
                Node {
                    width: Val::Px(width),
                    height: Val::Percent(100.0),
                    border: UiRect::all(Val::Px(5.0)),
                    ..Default::default()
                },
            )
        ],
    )
}

fn user_notify(font: Handle<Font>) -> impl Bundle {
    let height = 150.0;
    let font_size = height * 0.5;
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(height),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            top: Val::Px(HPBAR),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::SpaceEvenly,
            align_items: AlignItems::Center,
            ..Default::default()
        },
        // BackgroundColor(css::AQUA.into()),
        children![
            (
                UserNotifyLine1,
                Text::new("1111"),
                TextFont {
                    font: font.clone(),
                    font_size: font_size * 0.6,
                    ..Default::default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
            ),
            (
                UserNotifyLine2,
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: font_size * 0.4,
                    ..Default::default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
            )
        ],
    )
}

fn user_story(font: Handle<Font>) -> impl Bundle {
    let height = 150.0;
    let font_size = height * 0.5;
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(height),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            bottom: Val::Px(INVENTORY),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::SpaceEvenly,
            align_items: AlignItems::Center,
            ..Default::default()
        },
        // BackgroundColor(css::CORAL.into()),
        children![
            // (
            //     UserNotifyLine1,
            //     Text::new(""),
            //     TextFont {
            //         font: font.clone(),
            //         font_size: font_size * 0.6,
            //         ..Default::default()
            //     },
            //     TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
            // ),
            // (
            //     UserNotifyLine2,
            //     Text::new(""),
            //     TextFont {
            //         font: font.clone(),
            //         font_size: font_size * 0.4,
            //         ..Default::default()
            //     },
            //     TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
            // )
        ],
    )
}
