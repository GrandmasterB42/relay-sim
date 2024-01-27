#![allow(clippy::too_many_arguments)]

use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    window::PrimaryWindow,
};

use bevy_inspector_egui::quick::WorldInspectorPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
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
        ))
        .add_plugins(WorldInspectorPlugin::new())
        .run();
}

// A Simple curcuit simulation containing only a power source, buttons, lights and relays with their coil for activation and the switch part
struct SimPlugin;

const GRIDORIGIN: (f32, f32) = (-360., -360.);
const WINDOWRESOULTION: (f32, f32) = (1280., 720.);

#[derive(Component, Debug, Clone, Copy)]
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
}

// Label for relays is -K{id}
#[derive(Component)]
struct RelaySwitch {
    id: usize,
}

// Label for buttons is -S{id}
// This is the UI part of the button
#[derive(Component)]
struct UIButton {
    id: usize,
    has_been_pressed: bool,
}

// This is the actual switch of the button
#[derive(Component)]
struct ButtonSwitch {
    id: usize,
    typ: SwitchType,
}

enum SwitchType {
    NormallyOpen,
    NormallyClosed,
}

// A Wire represented as 2 points with a line between, can only go horizontally or vertically
#[derive(Component)]
struct Wire {
    first: GridPosition,
    second: GridPosition,
}

// Label for lights is -P{id}
#[derive(Component)]
struct Light {
    id: usize,
}

#[derive(Component)]
struct UILight {
    id: usize,
    color: Color,
    is_lit: bool,
}

#[derive(Component)]
struct GridOrigin;

#[derive(Resource, Default)]
struct CurcuitHandles {
    wire_point_mesh: Mesh2dHandle,
    wire_material: Handle<ColorMaterial>,
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(20.))
            .init_resource::<CurcuitHandles>()
            .add_systems(Startup, setup)
            .add_systems(Update, accept_input)
            .add_systems(FixedUpdate, simulate);
    }
}

fn setup(
    mut cmd: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut handles: ResMut<CurcuitHandles>,
) {
    cmd.spawn(Camera2dBundle::default());

    let circle_mesh: Mesh2dHandle = meshes
        .add(
            shape::Circle {
                radius: 3.5,
                ..Default::default()
            }
            .into(),
        )
        .into();
    let wire_material = materials.add(ColorMaterial::from(Color::GRAY));
    handles.wire_point_mesh = circle_mesh;
    handles.wire_material = wire_material;

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
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                background_color: BackgroundColor(Color::rgb(0.1, 0.1, 0.1)),
                ..Default::default()
            },
            Name::new("Left Section"),
        ))
        .with_children(|root| {
            // Upper Section
            root.spawn((
                NodeBundle {
                    style: Style {
                        display: Display::Flex,
                        flex_grow: 1.,
                        height: Val::Percent(50.),
                        width: Val::Percent(100.),
                        ..Default::default()
                    },
                    border_color: BorderColor(Color::SILVER),
                    ..Default::default()
                },
                Name::new("Upper Section"),
            ))
            .with_children(|root| {
                root.spawn(TextBundle::from_section(
                    "Buttons",
                    TextStyle {
                        font_size: 40.,
                        ..Default::default()
                    },
                ));
            });

            // Lower Section
            root.spawn((
                NodeBundle {
                    style: Style {
                        display: Display::Flex,
                        flex_grow: 1.,
                        height: Val::Percent(50.),
                        width: Val::Percent(100.),
                        ..Default::default()
                    },
                    border_color: BorderColor(Color::SILVER),
                    ..Default::default()
                },
                Name::new("Lower Section"),
            ))
            .with_children(|root| {
                root.spawn(TextBundle::from_section(
                    "Lights",
                    TextStyle {
                        font_size: 40.,
                        ..Default::default()
                    },
                ));
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

fn accept_input(
    mut cmd: Commands,
    mouse_button: Res<Input<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut wire_origin: Local<Option<GridPosition>>,
    wires: Query<(Entity, &Wire)>,
    curcuit_material: Res<CurcuitHandles>,
    mut meshes: ResMut<Assets<Mesh>>,
    grid_origin: Query<Entity, With<GridOrigin>>,
) {
    let Some(mouse_position) = windows.single().cursor_position() else {
        return;
    };
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
                            mesh: curcuit_material.wire_point_mesh.clone(),
                            material: curcuit_material.wire_material.clone(),
                            transform: Transform::from_translation(Vec3::new(
                                20. * mouse_grid.x as f32 + 10.,
                                20. * mouse_grid.y as f32 + 10.,
                                1.,
                            )),
                            ..Default::default()
                        },
                        Name::new("Wire Point1"),
                    ))
                    .set_parent(wire);

                    // Second Visual Point
                    cmd.spawn((
                        MaterialMesh2dBundle {
                            mesh: curcuit_material.wire_point_mesh.clone(),
                            material: curcuit_material.wire_material.clone(),
                            transform: Transform::from_translation(Vec3::new(
                                20. * wire_origin_position.x as f32 + 10.,
                                20. * wire_origin_position.y as f32 + 10.,
                                1.,
                            )),
                            ..Default::default()
                        },
                        Name::new("Wire Point2"),
                    ))
                    .set_parent(wire);

                    // Line in-between
                    let (x_extent, y_extent, x_transform, y_transform): (f32, f32, f32, f32);
                    if mouse_grid.x == wire_origin_position.x {
                        x_extent = 3.;
                        y_extent = (mouse_grid.y as f32 - wire_origin_position.y as f32) * 20.;
                        x_transform = 20. * wire_origin_position.x as f32 + 10.;
                        y_transform = 20. * wire_origin_position.y as f32 + 10. + y_extent / 2.;
                    } else {
                        x_extent = (mouse_grid.x as f32 - wire_origin_position.x as f32) * 20.;
                        y_extent = 3.;
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
                            material: curcuit_material.wire_material.clone(),
                            transform: Transform::from_translation(Vec3::new(
                                x_transform,
                                y_transform,
                                1.,
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
                        if (min..max).contains(&mouse_grid.y) {
                            cmd.entity(e).despawn_recursive();
                        }
                    } else if wire.first.y == wire.second.y {
                        if wire.first.y != mouse_grid.y {
                            continue;
                        }
                        let min = wire.first.x.min(wire.second.x);
                        let max = wire.first.x.max(wire.second.x);
                        if (min..max).contains(&mouse_grid.x) {
                            cmd.entity(e).despawn_recursive();
                        }
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

fn simulate(
    _time: Res<Time>,
    wires: Query<&Wire>,
    button_switches: Query<(&GridPosition, &ButtonSwitch)>,
    button_input: Query<&mut UIButton>,
    relay_coils: Query<(&GridPosition, &RelayCoil)>,
    relay_switches: Query<(&GridPosition, &RelaySwitch)>,
    lights: Query<(&GridPosition, &Light)>,
    ui_lights: Query<&mut UILight>,
) {
    for wire in wires.iter() {
        //
    }
}
