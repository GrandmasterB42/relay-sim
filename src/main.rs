#![allow(clippy::too_many_arguments)]

use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    window::PrimaryWindow,
};

#[cfg(debug_assertions)]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use rand::Rng;

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::BLACK)).add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Circuit Simulator".to_string(),
                resolution: WINDOWRESOULTION.into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                resizable: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        SimPlugin,
    ));

    #[cfg(debug_assertions)]
    app.add_plugins(WorldInspectorPlugin::new());

    app.run();
}

// A Simple circuit simulation containing only a power source, buttons, lights and relays with their coil for activation and the switch part
struct SimPlugin;

const GRIDORIGIN: (f32, f32) = (-360., -360.);
const WINDOWRESOULTION: (f32, f32) = (1280., 720.);

#[derive(Component, Debug, Clone, Copy, PartialEq)]
struct GridPosition {
    x: usize,
    y: usize,
}

impl From<Vec2> for GridPosition {
    fn from(vec: Vec2) -> Self {
        Self {
            x: vec.x as usize,
            y: vec.y as usize,
        }
    }
}

// Label for power source is -K{id}
#[derive(Component)]
struct RelayCoil {
    id: usize,
    top: GridPosition,
    bottom: GridPosition,
    activated: bool,
}

// Label for relays is -K{id}
#[derive(Component)]
struct RelaySwitch {
    id: usize,
    typ: SwitchType,
    top: GridPosition,
    bottom: GridPosition,
}

impl From<&RelaySwitch> for Wire {
    fn from(relay: &RelaySwitch) -> Self {
        Self {
            first: relay.top,
            second: relay.bottom,
        }
    }
}

#[derive(Component)]
struct RelayCoilSelect {
    id: usize,
}

#[derive(Component)]
struct RelaySwitchSelect {
    id: usize,
    typ: SwitchType,
}

// Label for buttons is -S{id}
// This is the UI part of the button
#[derive(Component)]
struct UIButton {
    id: usize,
    has_been_pressed: bool,
}

#[derive(Component)]
struct ButtonSelect {
    id: usize,
    typ: SwitchType,
}

// This is the actual switch of the button
#[derive(Component)]
struct ButtonSwitch {
    id: usize,
    typ: SwitchType,
    top: GridPosition,
    bottom: GridPosition,
}

impl From<&ButtonSwitch> for Wire {
    fn from(button: &ButtonSwitch) -> Self {
        Self {
            first: button.top,
            second: button.bottom,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum SwitchType {
    NormallyOpen,
    NormallyClosed,
}

// A Wire represented as 2 points with a line between, can only go horizontally or vertically
#[derive(Component, Clone)]
struct Wire {
    first: GridPosition,
    second: GridPosition,
}

// Label for lights is -P{id}
#[derive(Component)]
struct Light {
    id: usize,
    top: GridPosition,
    bottom: GridPosition,
}

#[derive(Component)]
struct UILight {
    id: usize,
    is_lit: bool,
}

#[derive(Component)]
struct GridOrigin;

#[derive(Component, PartialEq)]
struct Power(PowerType);

#[derive(PartialEq)]
enum PowerType {
    Positive,
    Negative,
}

#[derive(Resource, Default)]
struct CircuitHandles {
    wire_point_mesh: Mesh2dHandle,
    wire_material: Handle<ColorMaterial>,
    light_material: Handle<ColorMaterial>,
}

#[derive(Resource, Clone)]
enum CurrentlyPlacing {
    Wire,
    RelayCoil {
        id: usize,
        label: String,
    },
    RelaySwitch {
        id: usize,
        label: String,
        typ: SwitchType,
    },
    Light {
        id: usize,
        label: String,
    },
    Button {
        id: usize,
        label: String,
        typ: SwitchType,
    },
}

impl Default for CurrentlyPlacing {
    fn default() -> Self {
        Self::Wire
    }
}

#[derive(Resource, Default)]
struct IsRunning(bool);

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(20.))
            .init_resource::<CircuitHandles>()
            .init_resource::<CurrentlyPlacing>()
            .init_resource::<IsRunning>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    accept_input,
                    change_light_opacity,
                    handle_light_button_press,
                    handle_button_button_press,
                    handle_relay_switch_button_press,
                    handle_relay_coil_button_press,
                ),
            )
            .add_systems(FixedUpdate, simulate);
    }
}

fn setup(
    mut cmd: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut handles: ResMut<CircuitHandles>,
) {
    cmd.spawn(Camera2dBundle::default());

    let circle_mesh: Mesh2dHandle = meshes
        .add(
            shape::Circle {
                radius: 5.,
                ..Default::default()
            }
            .into(),
        )
        .into();
    let wire_material = materials.add(ColorMaterial::from(Color::GRAY));
    let light_material = materials.add(ColorMaterial::from(Color::YELLOW));
    handles.wire_point_mesh = circle_mesh;
    handles.wire_material = wire_material;
    handles.light_material = light_material;

    // UI
    cmd.spawn(
        // Root Element
        (
            NodeBundle {
                style: Style {
                    height: Val::Percent(100.),
                    width: Val::Percent(100.),
                    ..Default::default()
                },
                ..Default::default()
            },
            Name::new("UI Root"),
        ),
    )
    .with_children(|root| {
        // Left section
        root.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(280.),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    ..Default::default()
                },
                background_color: BackgroundColor(Color::rgb(0.1, 0.1, 0.1)),
                ..Default::default()
            },
            Name::new("Left Section"),
        ))
        .with_children(|root| {
            let mut random = rand::thread_rng();

            root.spawn((
                NodeBundle {
                    style: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        width: Val::Px(100.),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Name::from("Light container"),
            ))
            .with_children(|root| {
                for i in 1..=6 {
                    root.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(50.),
                                height: Val::Px(50.),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(7.)),
                                ..Default::default()
                            },
                            border_color: BorderColor(Color::Rgba {
                                red: 0.9,
                                green: 0.9,
                                blue: 0.9,
                                alpha: 0.,
                            }),
                            background_color: BackgroundColor(Color::Rgba {
                                red: random.gen_range(0.0..1.0),
                                green: random.gen_range(0.0..1.0),
                                blue: random.gen_range(0.0..1.0),
                                alpha: 1.,
                            }),

                            ..Default::default()
                        },
                        Name::new(format!("Light {} Button", i)),
                        UILight {
                            id: i,
                            is_lit: false,
                        },
                    ))
                    .with_children(|root| {
                        root.spawn((
                            TextBundle::from_section(
                                format!("-P{i}"),
                                TextStyle {
                                    font_size: 20.,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..Default::default()
                                },
                            ),
                            Name::new(format!("Light {} Button Text", i)),
                        ));
                    });
                }
            });
            root.spawn((
                NodeBundle {
                    style: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Name::new("Button Container"),
            ))
            .with_children(|root| {
                for i in 1..=6 {
                    let color = Color::Rgba {
                        red: random.gen_range(0.0..1.0),
                        green: random.gen_range(0.0..1.0),
                        blue: random.gen_range(0.0..1.0),
                        alpha: 1.,
                    };
                    root.spawn((
                        NodeBundle {
                            style: Style {
                                display: Display::Flex,
                                flex_direction: FlexDirection::Row,
                                height: Val::Px(50.),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        Name::new(format!("Button {} Container", i)),
                    ))
                    .with_children(|root| {
                        // Button for pressing
                        root.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(50.),
                                    height: Val::Px(50.),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..Default::default()
                                },
                                background_color: BackgroundColor(color),

                                ..Default::default()
                            },
                            Name::new(format!("Button {} Button", i)),
                            UIButton {
                                id: i,
                                has_been_pressed: false,
                            },
                        ))
                        .with_children(|root| {
                            root.spawn((
                                TextBundle::from_section(
                                    format!("-S{i}"),
                                    TextStyle {
                                        font_size: 20.,
                                        color: Color::rgb(0.9, 0.9, 0.9),
                                        ..Default::default()
                                    },
                                ),
                                Name::new(format!("Button {} Button Text", i)),
                            ));
                        });
                        // The two buttons for placing the normally open and normally closed switch

                        root.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(50.),
                                    height: Val::Px(50.),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(7.)),
                                    ..Default::default()
                                },
                                border_color: BorderColor(Color::Rgba {
                                    red: 0.9,
                                    green: 0.9,
                                    blue: 0.9,
                                    alpha: 0.4,
                                }),
                                background_color: BackgroundColor(color),
                                ..Default::default()
                            },
                            Name::new(format!("Button {} NO Button", i)),
                            ButtonSelect {
                                id: i,
                                typ: SwitchType::NormallyOpen,
                            },
                        ))
                        .with_children(|root| {
                            root.spawn((
                                TextBundle::from_section(
                                    "NO",
                                    TextStyle {
                                        font_size: 20.,
                                        color: Color::rgb(0.9, 0.9, 0.9),
                                        ..Default::default()
                                    },
                                ),
                                Name::new(format!("Button {} NO Button Text", i)),
                            ));
                        });

                        root.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(50.),
                                    height: Val::Px(50.),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(7.)),
                                    ..Default::default()
                                },
                                border_color: BorderColor(Color::Rgba {
                                    red: 0.9,
                                    green: 0.9,
                                    blue: 0.9,
                                    alpha: 0.4,
                                }),
                                background_color: BackgroundColor(color),

                                ..Default::default()
                            },
                            Name::new(format!("Button {} NC Button", i)),
                            ButtonSelect {
                                id: i,
                                typ: SwitchType::NormallyClosed,
                            },
                        ))
                        .with_children(|root| {
                            root.spawn((
                                TextBundle::from_section(
                                    "NC",
                                    TextStyle {
                                        font_size: 20.,
                                        color: Color::rgb(0.9, 0.9, 0.9),
                                        ..Default::default()
                                    },
                                ),
                                Name::new(format!("Button {} NC Button Text", i)),
                            ));
                        });
                    });
                }
            });
            root.spawn((
                NodeBundle {
                    style: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Name::new("Relay Container"),
            ))
            .with_children(|root| {
                for i in 1..=6 {
                    root.spawn((
                        NodeBundle {
                            style: Style {
                                display: Display::Flex,
                                flex_direction: FlexDirection::Row,
                                height: Val::Px(50.),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        Name::new(format!("Relay {} Container", i)),
                    ))
                    .with_children(|root| {
                        // Like the button with three buttons, one with label -K{id} for the coil, one for NO and one for NC for the switches
                        let color = Color::Rgba {
                            red: random.gen_range(0.0..1.0),
                            green: random.gen_range(0.0..1.0),
                            blue: random.gen_range(0.0..1.0),
                            alpha: 1.,
                        };

                        root.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(50.),
                                    height: Val::Px(50.),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(7.)),
                                    ..Default::default()
                                },
                                border_color: BorderColor(Color::Rgba {
                                    red: 0.9,
                                    green: 0.9,
                                    blue: 0.9,
                                    alpha: 0.4,
                                }),
                                background_color: BackgroundColor(color),

                                ..Default::default()
                            },
                            Name::new(format!("Relay {} Coil Button", i)),
                            RelayCoilSelect { id: i },
                        ))
                        .with_children(|root| {
                            root.spawn((
                                TextBundle::from_section(
                                    format!("-K{i}"),
                                    TextStyle {
                                        font_size: 20.,
                                        color: Color::rgb(0.9, 0.9, 0.9),
                                        ..Default::default()
                                    },
                                ),
                                Name::new(format!("Relay {} Coil Button Text", i)),
                            ));
                        });

                        root.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(50.),
                                    height: Val::Px(50.),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(7.)),
                                    ..Default::default()
                                },
                                border_color: BorderColor(Color::Rgba {
                                    red: 0.9,
                                    green: 0.9,
                                    blue: 0.9,
                                    alpha: 0.4,
                                }),
                                background_color: BackgroundColor(color),

                                ..Default::default()
                            },
                            Name::new(format!("Relay {} NO Button", i)),
                            RelaySwitchSelect {
                                id: i,
                                typ: SwitchType::NormallyOpen,
                            },
                        ))
                        .with_children(|root| {
                            root.spawn((
                                TextBundle::from_section(
                                    "NO",
                                    TextStyle {
                                        font_size: 20.,
                                        color: Color::rgb(0.9, 0.9, 0.9),
                                        ..Default::default()
                                    },
                                ),
                                Name::new(format!("Relay {} NO Button Text", i)),
                            ));
                        });

                        root.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(50.),
                                    height: Val::Px(50.),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(7.)),
                                    ..Default::default()
                                },
                                border_color: BorderColor(Color::Rgba {
                                    red: 0.9,
                                    green: 0.9,
                                    blue: 0.9,
                                    alpha: 0.4,
                                }),
                                background_color: BackgroundColor(color),

                                ..Default::default()
                            },
                            Name::new(format!("Relay {} NC Button", i)),
                            RelaySwitchSelect {
                                id: i,
                                typ: SwitchType::NormallyClosed,
                            },
                        ))
                        .with_children(|root| {
                            root.spawn((
                                TextBundle::from_section(
                                    "NC",
                                    TextStyle {
                                        font_size: 20.,
                                        color: Color::rgb(0.9, 0.9, 0.9),
                                        ..Default::default()
                                    },
                                ),
                                Name::new(format!("Relay {} NC Button Text", i)),
                            ));
                        });
                    });
                }
            });
        });
    });

    // Point Grid, the ui section stretches out 280 pixels, meaning there is 1000 pixels left for the grid

    // 48 * 48 grid with origin at the bottom left, 20 pixels of distance between each point, also that distance to the border

    let circle_mesh: Mesh2dHandle = meshes
        .add(
            shape::Circle {
                radius: 2.5,
                ..Default::default()
            }
            .into(),
        )
        .into();

    let circle_material = materials.add(ColorMaterial::from(Color::GREEN));

    let grid_origin = cmd
        .spawn((
            SpatialBundle {
                transform: Transform::from_translation(Vec3::new(GRIDORIGIN.0, GRIDORIGIN.1, 0.)),
                ..Default::default()
            },
            Name::new("Grid Origin"),
            GridOrigin,
        ))
        .id();

    let background_points = cmd
        .spawn((SpatialBundle::default(), Name::new("Background Points")))
        .set_parent(grid_origin)
        .id();

    for x in 0..50 {
        for y in 0..36 {
            cmd.spawn((
                MaterialMesh2dBundle {
                    mesh: circle_mesh.clone(),
                    material: circle_material.clone(),
                    transform: Transform::from_translation(Vec3::new(
                        20. * x as f32 + 10.,
                        20. * y as f32 + 10.,
                        0.,
                    )),
                    ..Default::default()
                },
                GridPosition { x, y },
                Name::new(format!("GridMarker {}, {}", x, y)),
            ))
            .set_parent(background_points);
        }
    }

    // The default power source
    cmd.spawn((
        Name::new("Power Source Positive"),
        Power(PowerType::Positive),
        GridPosition { x: 0, y: 19 },
        MaterialMesh2dBundle {
            material: materials.add(ColorMaterial::from(Color::RED)),
            mesh: meshes
                .add(shape::Quad::new(Vec2 { x: 20., y: 20. }).into())
                .into(),
            transform: Transform::from_translation(Vec3::new(10., 20. * 19. + 10., 5.)),
            ..Default::default()
        },
    ))
    .set_parent(grid_origin);

    cmd.spawn((
        Name::new("Power Source Negative"),
        Power(PowerType::Negative),
        GridPosition { x: 0, y: 16 },
        MaterialMesh2dBundle {
            material: materials.add(ColorMaterial::from(Color::BLUE)),
            mesh: meshes
                .add(shape::Quad::new(Vec2 { x: 20., y: 20. }).into())
                .into(),
            transform: Transform::from_translation(Vec3::new(10., 20. * 16. + 10., 5.)),
            ..Default::default()
        },
    ))
    .set_parent(grid_origin);
}

fn convert_mouse_to_grid(pos: Vec2) -> Option<GridPosition> {
    // the 280 comes from the ui section width
    if pos.x < GRIDORIGIN.0 || pos.y < GRIDORIGIN.1 || pos.x < 280. {
        return None;
    }

    // 0, 0 in mouse space is the top left cornor
    let x = ((pos.x - 280.) / 20.) as usize;
    let y = (-(pos.y - WINDOWRESOULTION.1) / 20.) as usize;

    Some(GridPosition { x, y })
}

fn change_light_opacity(mut ui_button: Query<(&UILight, &mut BackgroundColor, &mut BorderColor)>) {
    for (ui_light, mut background_color, mut border_color) in ui_button.iter_mut() {
        if ui_light.is_lit {
            background_color.0.set_a(0.95);
            border_color.0.set_a(0.95);
        } else {
            background_color.0.set_a(0.4);
            border_color.0.set_a(0.1);
        }
    }
}

fn accept_input(
    cmd: Commands,
    mouse_button: Res<Input<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    wire_origin: Local<Option<GridPosition>>,
    wires: Query<(Entity, &Wire)>,
    lights: Query<(Entity, &Light)>,
    buttons: Query<(Entity, &ButtonSwitch)>,
    relay_switches: Query<(Entity, &RelaySwitch)>,
    relay_coils: Query<(Entity, &RelayCoil)>,
    circuit_material: Res<CircuitHandles>,
    meshes: ResMut<Assets<Mesh>>,
    grid_origin: Query<Entity, With<GridOrigin>>,
    currently_placing: ResMut<CurrentlyPlacing>,
) {
    let Some(mouse_position) = windows.single().cursor_position() else {
        return;
    };

    match currently_placing.as_ref().clone() {
        CurrentlyPlacing::Wire => handle_wire_placement(
            cmd,
            mouse_position,
            mouse_button,
            wires,
            circuit_material,
            meshes,
            grid_origin,
            wire_origin,
            lights,
            buttons,
            relay_switches,
            relay_coils,
        ),
        CurrentlyPlacing::Light { id, label } => handle_light_placement(
            cmd,
            id,
            label,
            mouse_position,
            mouse_button,
            circuit_material,
            meshes,
            grid_origin,
            currently_placing,
        ),
        CurrentlyPlacing::Button { id, label, typ } => handle_button_placement(
            cmd,
            id,
            label,
            typ,
            mouse_position,
            mouse_button,
            circuit_material,
            meshes,
            grid_origin,
            currently_placing,
        ),
        CurrentlyPlacing::RelayCoil { id, label } => handle_relay_coil_placement(
            cmd,
            id,
            label,
            mouse_position,
            mouse_button,
            circuit_material,
            meshes,
            grid_origin,
            currently_placing,
        ),
        CurrentlyPlacing::RelaySwitch { id, label, typ } => handle_relay_switch_placement(
            cmd,
            id,
            label,
            typ,
            mouse_position,
            mouse_button,
            circuit_material,
            meshes,
            grid_origin,
            currently_placing,
        ),
    }
}
// Exactly the same as buttons, but with a rectangle instead of a square
fn handle_relay_coil_placement(
    mut cmd: Commands,
    id: usize,
    label: String,
    mouse_position: Vec2,
    mouse_button: Res<Input<MouseButton>>,
    circuit_material: Res<CircuitHandles>,
    mut meshes: ResMut<Assets<Mesh>>,
    grid_origin: Query<Entity, With<GridOrigin>>,
    mut currently_placing: ResMut<CurrentlyPlacing>,
) {
    if mouse_button.just_pressed(MouseButton::Right) {
        *currently_placing = CurrentlyPlacing::Wire;
        return;
    }

    if mouse_button.just_pressed(MouseButton::Left) {
        let mouse_grid_pos = convert_mouse_to_grid(mouse_position);
        let Some(mouse_grid) = mouse_grid_pos else {
            return;
        };

        let coil = cmd
            .spawn((
                Name::new(label.clone()),
                RelayCoil {
                    id,
                    top: GridPosition {
                        x: mouse_grid.x,
                        y: mouse_grid.y + 1,
                    },
                    bottom: GridPosition {
                        x: mouse_grid.x,
                        y: mouse_grid.y - 1,
                    },
                    activated: false,
                },
                SpatialBundle::default(),
            ))
            .set_parent(grid_origin.single())
            .id();

        // Like other components, but with a rectangle instead of a square
        cmd.spawn((
            MaterialMesh2dBundle {
                mesh: meshes
                    .add(shape::Quad::new(Vec2 { x: 30., y: 20. }).into())
                    .into(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * mouse_grid.y as f32 + 10.,
                    2.5,
                )),
                ..Default::default()
            },
            Name::new("Relay Coil"),
        ))
        .set_parent(coil);

        // The two points
        cmd.spawn((
            MaterialMesh2dBundle {
                mesh: circuit_material.wire_point_mesh.clone(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * ((mouse_grid.y as f32) - 1.) + 10.,
                    2.5,
                )),
                ..Default::default()
            },
            Name::new("Relay Coil Point1"),
        ))
        .set_parent(coil);

        cmd.spawn((
            MaterialMesh2dBundle {
                mesh: circuit_material.wire_point_mesh.clone(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * ((mouse_grid.y as f32) + 1.) + 10.,
                    2.5,
                )),
                ..Default::default()
            },
            Name::new("Relay Coil Point2"),
        ))
        .set_parent(coil);

        // a wire all the way through
        let wire = cmd
            .spawn(MaterialMesh2dBundle {
                mesh: meshes
                    .add(shape::Quad::new(Vec2 { x: 4., y: 40. }).into())
                    .into(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * mouse_grid.y as f32 + 10.,
                    2.,
                )),
                ..Default::default()
            })
            .set_parent(coil)
            .id();

        cmd.spawn(Text2dBundle {
            text: Text::from_section(
                label,
                TextStyle {
                    font_size: 20.,
                    color: Color::WHITE,
                    ..Default::default()
                },
            ),
            transform: Transform::from_translation(Vec3 {
                x: 0.,
                y: 0.,
                z: 5.,
            }),
            ..Default::default()
        })
        .set_parent(wire);

        *currently_placing = CurrentlyPlacing::Wire;
    }
}

// Exactly the same as buttons, but with the label -K{id} and the relayswitch component
fn handle_relay_switch_placement(
    mut cmd: Commands,
    id: usize,
    label: String,
    typ: SwitchType,
    mouse_position: Vec2,
    mouse_button: Res<Input<MouseButton>>,
    circuit_material: Res<CircuitHandles>,
    mut meshes: ResMut<Assets<Mesh>>,
    grid_origin: Query<Entity, With<GridOrigin>>,
    mut currently_placing: ResMut<CurrentlyPlacing>,
) {
    if mouse_button.just_pressed(MouseButton::Right) {
        *currently_placing = CurrentlyPlacing::Wire;
        return;
    }

    if mouse_button.just_pressed(MouseButton::Left) {
        let mouse_grid_pos = convert_mouse_to_grid(mouse_position);
        let Some(mouse_grid) = mouse_grid_pos else {
            return;
        };

        let relay = cmd
            .spawn((
                Name::new(label.clone()),
                RelaySwitch {
                    id,
                    typ,
                    top: GridPosition {
                        x: mouse_grid.x,
                        y: mouse_grid.y + 1,
                    },
                    bottom: GridPosition {
                        x: mouse_grid.x,
                        y: mouse_grid.y - 1,
                    },
                },
                SpatialBundle::default(),
            ))
            .set_parent(grid_origin.single())
            .id();

        // Like button
        cmd.spawn((
            MaterialMesh2dBundle {
                mesh: circuit_material.wire_point_mesh.clone(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * ((mouse_grid.y as f32) - 1.) + 10.,
                    2.5,
                )),
                ..Default::default()
            },
            Name::new("Relay Point1"),
        ))
        .set_parent(relay);

        cmd.spawn((
            MaterialMesh2dBundle {
                mesh: circuit_material.wire_point_mesh.clone(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * ((mouse_grid.y as f32) + 1.) + 10.,
                    2.5,
                )),
                ..Default::default()
            },
            Name::new("Relay Point2"),
        ))
        .set_parent(relay);

        cmd.spawn((
            MaterialMesh2dBundle {
                mesh: meshes
                    .add(shape::Quad::new(Vec2 { x: 20., y: 20. }).into())
                    .into(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * mouse_grid.y as f32 + 10.,
                    2.5,
                )),
                ..Default::default()
            },
            Name::new("Relay Square"),
        ))
        .set_parent(relay)
        .with_children(|root| {
            root.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        match typ {
                            SwitchType::NormallyOpen => "NO",
                            SwitchType::NormallyClosed => "NC",
                        },
                        TextStyle {
                            font_size: 15.,
                            color: Color::WHITE,
                            ..Default::default()
                        },
                    ),
                    transform: Transform::from_translation(Vec3 {
                        x: 0.,
                        y: 0.,
                        z: 5.,
                    }),
                    ..Default::default()
                },
                Name::new("Relay Text"),
            ));
        });

        // a wire all the way through
        let wire = cmd
            .spawn(MaterialMesh2dBundle {
                mesh: meshes
                    .add(shape::Quad::new(Vec2 { x: 4., y: 40. }).into())
                    .into(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * mouse_grid.y as f32 + 10.,
                    2.,
                )),
                ..Default::default()
            })
            .set_parent(relay)
            .id();

        cmd.spawn(Text2dBundle {
            text: Text::from_section(
                label,
                TextStyle {
                    font_size: 20.,
                    color: Color::WHITE,
                    ..Default::default()
                },
            ),
            transform: Transform::from_translation(Vec3 {
                x: 20.,
                y: 0.,
                z: 5.,
            }),
            ..Default::default()
        })
        .set_parent(wire);
        *currently_placing = CurrentlyPlacing::Wire;
    }
}

fn handle_button_placement(
    mut cmd: Commands,
    id: usize,
    label: String,
    typ: SwitchType,
    mouse_position: Vec2,
    mouse_button: Res<Input<MouseButton>>,
    circuit_material: Res<CircuitHandles>,
    mut meshes: ResMut<Assets<Mesh>>,
    grid_origin: Query<Entity, With<GridOrigin>>,
    mut currently_placing: ResMut<CurrentlyPlacing>,
) {
    if mouse_button.just_pressed(MouseButton::Right) {
        *currently_placing = CurrentlyPlacing::Wire;
        return;
    }

    if mouse_button.just_pressed(MouseButton::Left) {
        let mouse_grid_pos = convert_mouse_to_grid(mouse_position);
        let Some(mouse_grid) = mouse_grid_pos else {
            return;
        };

        let button = cmd
            .spawn((
                Name::new(label.clone()),
                ButtonSwitch {
                    id,
                    typ,
                    top: GridPosition {
                        x: mouse_grid.x,
                        y: mouse_grid.y + 1,
                    },
                    bottom: GridPosition {
                        x: mouse_grid.x,
                        y: mouse_grid.y - 1,
                    },
                },
                SpatialBundle::default(),
            ))
            .set_parent(grid_origin.single())
            .id();

        // Like wire, but with label in the middle on big circle
        cmd.spawn((
            MaterialMesh2dBundle {
                mesh: circuit_material.wire_point_mesh.clone(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * ((mouse_grid.y as f32) - 1.) + 10.,
                    2.5,
                )),
                ..Default::default()
            },
            Name::new("Button Point1"),
        ))
        .set_parent(button);

        cmd.spawn((
            MaterialMesh2dBundle {
                mesh: circuit_material.wire_point_mesh.clone(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * ((mouse_grid.y as f32) + 1.) + 10.,
                    2.5,
                )),
                ..Default::default()
            },
            Name::new("Button Point2"),
        ))
        .set_parent(button);
        // The middle, for the button just a square with eiter NC or NO on it
        cmd.spawn((
            MaterialMesh2dBundle {
                mesh: meshes
                    .add(shape::Quad::new(Vec2 { x: 20., y: 20. }).into())
                    .into(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * mouse_grid.y as f32 + 10.,
                    2.5,
                )),
                ..Default::default()
            },
            Name::new("Button Square"),
        ))
        .set_parent(button)
        .with_children(|root| {
            root.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        match typ {
                            SwitchType::NormallyOpen => "NO",
                            SwitchType::NormallyClosed => "NC",
                        },
                        TextStyle {
                            font_size: 15.,
                            color: Color::WHITE,
                            ..Default::default()
                        },
                    ),
                    transform: Transform::from_translation(Vec3 {
                        x: 0.,
                        y: 0.,
                        z: 5.,
                    }),
                    ..Default::default()
                },
                Name::new("Button Text"),
            ));
        });

        // a wire all the way through
        let wire = cmd
            .spawn(MaterialMesh2dBundle {
                mesh: meshes
                    .add(shape::Quad::new(Vec2 { x: 4., y: 40. }).into())
                    .into(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * mouse_grid.y as f32 + 10.,
                    2.,
                )),
                ..Default::default()
            })
            .set_parent(button)
            .id();

        cmd.spawn(Text2dBundle {
            text: Text::from_section(
                label,
                TextStyle {
                    font_size: 20.,
                    color: Color::WHITE,
                    ..Default::default()
                },
            ),
            transform: Transform::from_translation(Vec3 {
                x: 20.,
                y: 0.,
                z: 5.,
            }),
            ..Default::default()
        })
        .set_parent(wire);
        *currently_placing = CurrentlyPlacing::Wire;
    }
}

fn handle_light_placement(
    mut cmd: Commands,
    id: usize,
    label: String,
    mouse_position: Vec2,
    mouse_button: Res<Input<MouseButton>>,
    circuit_material: Res<CircuitHandles>,
    mut meshes: ResMut<Assets<Mesh>>,
    grid_origin: Query<Entity, With<GridOrigin>>,
    mut currently_placing: ResMut<CurrentlyPlacing>,
) {
    if mouse_button.just_pressed(MouseButton::Right) {
        *currently_placing = CurrentlyPlacing::Wire;
        return;
    }

    if mouse_button.just_pressed(MouseButton::Left) {
        let mouse_grid_pos = convert_mouse_to_grid(mouse_position);
        let Some(mouse_grid) = mouse_grid_pos else {
            return;
        };

        let light = cmd
            .spawn((
                Name::new(label.clone()),
                Light {
                    id,
                    top: GridPosition {
                        x: mouse_grid.x,
                        y: mouse_grid.y + 1,
                    },
                    bottom: GridPosition {
                        x: mouse_grid.x,
                        y: mouse_grid.y - 1,
                    },
                },
                SpatialBundle::default(),
            ))
            .set_parent(grid_origin.single())
            .id();

        // Like wire, but with label in the middle on big circle
        cmd.spawn((
            MaterialMesh2dBundle {
                mesh: circuit_material.wire_point_mesh.clone(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * ((mouse_grid.y as f32) - 1.) + 10.,
                    2.5,
                )),
                ..Default::default()
            },
            Name::new("Light Point1"),
        ))
        .set_parent(light);

        cmd.spawn((
            MaterialMesh2dBundle {
                mesh: circuit_material.wire_point_mesh.clone(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * (mouse_grid.y + 1) as f32 + 10.,
                    2.5,
                )),
                ..Default::default()
            },
            Name::new("Light Point2"),
        ))
        .set_parent(light);

        cmd.spawn((
            MaterialMesh2dBundle {
                mesh: circuit_material.wire_point_mesh.clone(),
                material: circuit_material.light_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * mouse_grid.y as f32 + 10.,
                    2.5,
                )),
                ..Default::default()
            },
            Name::new("Light Point3"),
        ))
        .set_parent(light);

        // a wire all the way through, this is always the same size, so not many calculations needes

        let wire = cmd
            .spawn(MaterialMesh2dBundle {
                mesh: meshes
                    .add(shape::Quad::new(Vec2 { x: 4., y: 40. }).into())
                    .into(),
                material: circuit_material.wire_material.clone(),
                transform: Transform::from_translation(Vec3::new(
                    20. * mouse_grid.x as f32 + 10.,
                    20. * mouse_grid.y as f32 + 10.,
                    2.,
                )),
                ..Default::default()
            })
            .set_parent(light)
            .id();

        cmd.spawn(Text2dBundle {
            text: Text::from_section(
                label,
                TextStyle {
                    font_size: 20.,
                    color: Color::WHITE,
                    ..Default::default()
                },
            ),
            transform: Transform::from_translation(Vec3 {
                x: 20.,
                y: 0.,
                z: 5.,
            }),
            ..Default::default()
        })
        .set_parent(wire);

        *currently_placing = CurrentlyPlacing::Wire;
    }
}

fn handle_light_button_press(
    mut interaction: Query<(&Interaction, &mut UILight), Changed<Interaction>>,
    placed_lights: Query<&Light>,
    mut currently_placing: ResMut<CurrentlyPlacing>,
) {
    for (interaction, ui_light) in interaction.iter_mut() {
        if interaction == &Interaction::Pressed {
            if placed_lights.iter().any(|light| light.id == ui_light.id) {
                continue;
            }
            *currently_placing = CurrentlyPlacing::Light {
                id: ui_light.id,
                label: format!("-P{}", ui_light.id),
            };
        }
    }
}

fn handle_button_button_press(
    mut press_interaction: Query<(&Interaction, &mut UIButton)>,
    mut place_interaction: Query<(&Interaction, &mut ButtonSelect)>,
    placed_buttons: Query<&ButtonSwitch>,
    mut currently_placing: ResMut<CurrentlyPlacing>,
) {
    for (interaction, mut ui_button) in press_interaction.iter_mut() {
        if *interaction == Interaction::Pressed {
            ui_button.has_been_pressed = true;
        }
    }

    for (interaction, button_select) in place_interaction.iter_mut() {
        if placed_buttons
            .iter()
            .any(|button| button.id == button_select.id && button.typ == button_select.typ)
        {
            continue;
        }
        if *interaction == Interaction::Pressed {
            *currently_placing = CurrentlyPlacing::Button {
                id: button_select.id,
                label: format!("-S{}", button_select.id),
                typ: button_select.typ,
            };
        }
    }
}

fn handle_relay_switch_button_press(
    mut iteraction: Query<(&Interaction, &RelaySwitchSelect), Changed<Interaction>>,
    placed_relay_switches: Query<&RelaySwitch>,
    mut currently_placing: ResMut<CurrentlyPlacing>,
) {
    for (interaction, relay_switch_select) in iteraction.iter_mut() {
        if placed_relay_switches
            .iter()
            .filter(|relay_switch| {
                relay_switch.id == relay_switch_select.id
                    && relay_switch.typ == relay_switch_select.typ
            })
            .collect::<Vec<_>>()
            .len()
            >= 5
        {
            continue;
        }
        if *interaction == Interaction::Pressed {
            *currently_placing = CurrentlyPlacing::RelaySwitch {
                id: relay_switch_select.id,
                label: format!("-K{}", relay_switch_select.id),
                typ: relay_switch_select.typ,
            };
        }
    }
}

fn handle_relay_coil_button_press(
    mut interaction: Query<(&Interaction, &mut RelayCoilSelect), Changed<Interaction>>,
    placed_relay_coils: Query<&RelayCoil>,
    mut currently_placing: ResMut<CurrentlyPlacing>,
) {
    for (interaction, relay_coil_select) in interaction.iter_mut() {
        if placed_relay_coils
            .iter()
            .any(|relay_coil| relay_coil.id == relay_coil_select.id)
        {
            continue;
        }
        if *interaction == Interaction::Pressed {
            *currently_placing = CurrentlyPlacing::RelayCoil {
                id: relay_coil_select.id,
                label: format!("-K{}", relay_coil_select.id),
            };
        }
    }
}

fn handle_wire_placement(
    mut cmd: Commands,
    mouse_position: Vec2,
    mouse_button: Res<Input<MouseButton>>,
    wires: Query<(Entity, &Wire)>,
    circuit_material: Res<CircuitHandles>,
    mut meshes: ResMut<Assets<Mesh>>,
    grid_origin: Query<Entity, With<GridOrigin>>,
    mut wire_origin: Local<Option<GridPosition>>,
    lights: Query<(Entity, &Light)>,
    buttons: Query<(Entity, &ButtonSwitch)>,
    relay_switches: Query<(Entity, &RelaySwitch)>,
    relay_coils: Query<(Entity, &RelayCoil)>,
) {
    let mouse_grid_pos = convert_mouse_to_grid(mouse_position);
    match mouse_grid_pos {
        Some(ref mouse_grid) => {
            if mouse_button.just_pressed(MouseButton::Left) {
                let Some(ref wire_origin_position) = *wire_origin else {
                    *wire_origin = mouse_grid_pos;
                    return;
                };

                // if the mouse is on the same x or y axis as the origin, create a wire
                if mouse_grid.x == wire_origin_position.x || mouse_grid.y == wire_origin_position.y
                {
                    let wire = cmd
                        .spawn((
                            Name::new(format!(
                                "Wire {}, {} to {}, {}",
                                wire_origin_position.x,
                                wire_origin_position.y,
                                mouse_grid.x,
                                mouse_grid.y
                            )),
                            // Wire that stores position for simulation
                            Wire {
                                first: *wire_origin_position,
                                second: *mouse_grid,
                            },
                            SpatialBundle::default(),
                        ))
                        .set_parent(grid_origin.single())
                        .id();

                    // First Visual Point
                    cmd.spawn((
                        MaterialMesh2dBundle {
                            mesh: circuit_material.wire_point_mesh.clone(),
                            material: circuit_material.wire_material.clone(),
                            transform: Transform::from_translation(Vec3::new(
                                20. * mouse_grid.x as f32 + 10.,
                                20. * mouse_grid.y as f32 + 10.,
                                2.5,
                            )),
                            ..Default::default()
                        },
                        Name::new("Wire Point1"),
                    ))
                    .set_parent(wire);

                    // Second Visual Point
                    cmd.spawn((
                        MaterialMesh2dBundle {
                            mesh: circuit_material.wire_point_mesh.clone(),
                            material: circuit_material.wire_material.clone(),
                            transform: Transform::from_translation(Vec3::new(
                                20. * wire_origin_position.x as f32 + 10.,
                                20. * wire_origin_position.y as f32 + 10.,
                                2.5,
                            )),
                            ..Default::default()
                        },
                        Name::new("Wire Point2"),
                    ))
                    .set_parent(wire);

                    // Line in-between
                    let (x_extent, y_extent, x_transform, y_transform): (f32, f32, f32, f32);
                    if mouse_grid.x == wire_origin_position.x {
                        x_extent = 4.;
                        y_extent = (mouse_grid.y as f32 - wire_origin_position.y as f32) * 20.;
                        x_transform = 20. * wire_origin_position.x as f32 + 10.;
                        y_transform = 20. * wire_origin_position.y as f32 + 10. + y_extent / 2.;
                    } else {
                        x_extent = (mouse_grid.x as f32 - wire_origin_position.x as f32) * 20.;
                        y_extent = 4.;
                        x_transform = 20. * wire_origin_position.x as f32 + 10. + x_extent / 2.;
                        y_transform = 20. * wire_origin_position.y as f32 + 10.;
                    }
                    cmd.spawn((
                        MaterialMesh2dBundle {
                            mesh: meshes
                                .add(
                                    shape::Quad::new(Vec2 {
                                        x: x_extent,
                                        y: y_extent,
                                    })
                                    .into(),
                                )
                                .into(),
                            material: circuit_material.wire_material.clone(),
                            transform: Transform::from_translation(Vec3::new(
                                x_transform,
                                y_transform,
                                2.5,
                            )),
                            ..Default::default()
                        },
                        Name::new("Wire Line"),
                    ))
                    .set_parent(wire);
                }
                *wire_origin = None;
            } else if mouse_button.just_pressed(MouseButton::Right) {
                if wire_origin.is_some() {
                    *wire_origin = None;
                    return;
                }
                for (e, wire) in wires.iter() {
                    // if line between the two wire points intersects with the mouse position, remove it
                    if wire.first.x == wire.second.x {
                        if wire.first.x != mouse_grid.x {
                            continue;
                        }
                        let min = wire.first.y.min(wire.second.y);
                        let max = wire.first.y.max(wire.second.y);
                        if (min..=max).contains(&mouse_grid.y) {
                            cmd.entity(e).despawn_recursive();
                        }
                    } else if wire.first.y == wire.second.y {
                        if wire.first.y != mouse_grid.y {
                            continue;
                        }
                        let min = wire.first.x.min(wire.second.x);
                        let max = wire.first.x.max(wire.second.x);
                        if (min..=max).contains(&mouse_grid.x) {
                            cmd.entity(e).despawn_recursive();
                        }
                    }
                }

                for (e, light) in lights.iter() {
                    let mut middle = light.top;
                    middle.y -= 1;
                    if light.top == *mouse_grid
                        || light.bottom == *mouse_grid
                        || middle == *mouse_grid
                    {
                        cmd.entity(e).despawn_recursive();
                    }
                }

                for (e, button) in buttons.iter() {
                    let mut middle = button.top;
                    middle.y -= 1;
                    if button.top == *mouse_grid
                        || button.bottom == *mouse_grid
                        || middle == *mouse_grid
                    {
                        cmd.entity(e).despawn_recursive();
                    }
                }

                for (e, relay_switch) in relay_switches.iter() {
                    let mut middle = relay_switch.top;
                    middle.y -= 1;
                    if relay_switch.top == *mouse_grid
                        || relay_switch.bottom == *mouse_grid
                        || middle == *mouse_grid
                    {
                        cmd.entity(e).despawn_recursive();
                    }
                }

                for (e, relay_coil) in relay_coils.iter() {
                    let mut middle = relay_coil.top;
                    middle.y -= 1;
                    if relay_coil.top == *mouse_grid
                        || relay_coil.bottom == *mouse_grid
                        || middle == *mouse_grid
                    {
                        cmd.entity(e).despawn_recursive();
                    }
                }
            }
        }
        None => {
            if mouse_button.just_pressed(MouseButton::Left) {
                *wire_origin = None;
            }
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Visited {
    Positive,
    Negative,
    Unvisited,
}

fn simulate(
    wires: Query<&Wire>,
    mut button_input: Query<&mut UIButton>,
    button_switches: Query<&ButtonSwitch>,
    mut relay_coils: Query<&mut RelayCoil>,
    relay_switches: Query<&RelaySwitch>,
    mut ui_lights: Query<&mut UILight>,
    lights: Query<&Light>,
    power_sources: Query<(&GridPosition, &Power)>,
) {
    // CAUTION! This does not cover when there are two consumers in series, for that, extra passes are needed, but it will work for now, if a consumer finds a not yet covered wire, that could be indicated as well

    // Turn wires into 2 vectors. one with all Gridpositions, one with a tuple of indices for connections
    let max_len = wires.iter().len() + button_switches.iter().len();
    let mut wire_positions: Vec<(GridPosition, Visited)> = Vec::with_capacity(max_len);
    let mut wire_connections: Vec<(usize, usize)> = Vec::with_capacity(max_len);

    // Button prepass, resetting all ui buttons and transforming fitting buttons into wires
    let mut active_button_ids = Vec::new();
    for mut button in button_input.iter_mut() {
        if button.has_been_pressed {
            active_button_ids.push(button.id);
        }
        button.has_been_pressed = false;
    }

    let button_wires = button_switches
        .iter()
        .filter(|button| match button.typ {
            SwitchType::NormallyOpen => active_button_ids.contains(&button.id),
            SwitchType::NormallyClosed => !active_button_ids.contains(&button.id),
        })
        .map(Wire::from);

    let mut active_relay_ids = Vec::new();
    for mut relay_coil in relay_coils.iter_mut() {
        if relay_coil.activated {
            active_relay_ids.push(relay_coil.id);
        }
        relay_coil.activated = false;
    }

    let relay_wires = relay_switches
        .iter()
        .filter(|relay_switch| match relay_switch.typ {
            SwitchType::NormallyOpen => active_relay_ids.contains(&relay_switch.id),
            SwitchType::NormallyClosed => !active_relay_ids.contains(&relay_switch.id),
        })
        .map(Wire::from);

    for wire in wires
        .iter()
        .map(Clone::clone)
        .chain(button_wires)
        .chain(relay_wires)
    {
        let mut first_index = 0;
        let mut second_index = 0;
        for (pos, index) in &mut [
            (wire.first, &mut first_index),
            (wire.second, &mut second_index),
        ] {
            if let Some(idx) = wire_positions.iter().position(|p| &p.0 == pos) {
                **index = idx;
            } else {
                **index = wire_positions.len();
                wire_positions.push((*pos, Visited::Unvisited));
            }
        }
        wire_connections.push((first_index, second_index));
    }

    let power_sources = power_sources.iter().take(2).collect::<Vec<_>>();

    let source_1 = power_sources[0];
    let source_2 = power_sources[1];
    let (positive_source, negative_source) = if source_1.1 .0 == PowerType::Positive {
        (source_1.0, source_2.0)
    } else {
        (source_2.0, source_1.0)
    };

    walk_wires(
        positive_source,
        Visited::Positive,
        &mut wire_positions,
        &wire_connections,
    )
    .unwrap();

    if walk_wires(
        negative_source,
        Visited::Negative,
        &mut wire_positions,
        &wire_connections,
    )
    .is_err()
    {
        // Short Circuit
        return;
    }

    for mut ui_light in ui_lights.iter_mut() {
        ui_light.is_lit = false;
    }

    for light in lights.iter() {
        let Some(top_index) = wire_positions.iter().position(|p| p.0 == light.top) else {
            continue;
        };
        let Some(bottom_index) = wire_positions.iter().position(|p| p.0 == light.bottom) else {
            continue;
        };

        if (wire_positions[top_index].1 == Visited::Positive
            && wire_positions[bottom_index].1 == Visited::Negative)
            || (wire_positions[top_index].1 == Visited::Negative
                && wire_positions[bottom_index].1 == Visited::Positive)
        {
            ui_lights
                .iter_mut()
                .find(|ui_light| ui_light.id == light.id)
                .unwrap()
                .is_lit = true;
        } else if wire_positions[top_index].1 == Visited::Unvisited
            || wire_positions[bottom_index].1 == Visited::Unvisited
        {
            debug!("Unvisited Wire");
        }
    }

    for mut relay_coil in relay_coils.iter_mut() {
        let Some(top_index) = wire_positions.iter().position(|p| p.0 == relay_coil.top) else {
            continue;
        };
        let Some(bottom_index) = wire_positions.iter().position(|p| p.0 == relay_coil.bottom)
        else {
            continue;
        };

        if (wire_positions[top_index].1 == Visited::Positive
            && wire_positions[bottom_index].1 == Visited::Negative)
            || (wire_positions[top_index].1 == Visited::Negative
                && wire_positions[bottom_index].1 == Visited::Positive)
        {
            relay_coil.activated = true;
        } else if wire_positions[top_index].1 == Visited::Unvisited
            || wire_positions[bottom_index].1 == Visited::Unvisited
        {
            debug!("Unvisited Wire");
        }
    }
}

fn walk_wires(
    source: &GridPosition,
    mark: Visited,
    wire_positions: &mut [(GridPosition, Visited)],
    wire_connections: &[(usize, usize)],
) -> Result<(), ()> {
    let mut to_visit = vec![*source];

    while let Some(pos) = to_visit.pop() {
        let Some(index) = wire_positions.iter().position(|p| p.0 == pos) else {
            continue;
        };

        if wire_positions[index].1 == Visited::Unvisited {
            wire_positions[index].1 = mark;
        } else {
            if wire_positions[index].1 != mark {
                error!("Short Circuit");
                return Err(());
            }
            continue;
        }

        // find all occurrences of index in wire_connections
        let next_connections = wire_connections
            .iter()
            .filter_map(|(first, second)| {
                if *first == index {
                    Some(*second)
                } else if *second == index {
                    Some(*first)
                } else {
                    None
                }
            })
            .filter(|idx| wire_positions[*idx].1 != mark)
            .map(|idx| wire_positions[idx].0);

        to_visit.extend(next_connections);
    }
    Ok(())
}
