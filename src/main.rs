#![feature(iter_collect_into)]
use std::ops::RangeInclusive;

use bevy::{prelude::*, render::camera::ScalingMode, utils::HashSet};
use bevy_rapier2d::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.2, 0.18, 0.2)))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),
            // RapierDebugRenderPlugin::default(),
        ))
        .init_resource::<Score>()
        .init_resource::<Textures>()
        .init_resource::<Loss>()
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(
            Update,
            (control_ball, merge_balls, update_score_text, laser_system),
        )
        .add_systems(Last, check_for_loss)
        .run();
}

#[derive(Component, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct Ball(u32);

impl Ball {
    fn next(&self) -> Self {
        Ball(self.0 + 1)
    }

    fn radius(&self) -> f32 {
        1.3f32.powi(self.0 as i32) * BALL_RADIUS
    }

    fn score(&self) -> u64 {
        5 * 2u64.pow(self.0.saturating_sub(1))
    }
}

#[derive(Resource, Default)]
struct Score(u64);

#[derive(Component)]
struct Controlled {
    current_ball: Option<Entity>,
    next_ball: Entity,
    ball_cooldown: f32,
    pog: Option<f32>,
}

#[derive(Resource)]
struct Textures {
    balls: Vec<Handle<Image>>,
    nor_lud: Handle<Image>,
    con_lud: Handle<Image>,
    pog_lud: Handle<Image>,
    hands_f: Handle<Image>,
    hands_b: Handle<Image>,
}

impl FromWorld for Textures {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Textures {
            balls: [
                "coots.png",
                "ders.png",
                "swift.png",
                "lud.png",
                "qt.png",
                "slime.png",
                "mogul_mail.png",
            ]
            .into_iter()
            .map(|p| asset_server.load(p))
            .collect(),
            nor_lud: asset_server.load("nor_lud.png"),
            con_lud: asset_server.load("con_lud.png"),
            pog_lud: asset_server.load("pog_lud.png"),
            hands_f: asset_server.load("hands_f.png"),
            hands_b: asset_server.load("hands_b.png"),
        }
    }
}

const CUP_WIDTH: f32 = 500.0;
const CUP_HEIGHT: f32 = 500.0;
const CUP_THICKNESS: f32 = 10.0;
const BALL_RADIUS: f32 = 15.0;
const FACE_DOWN: f32 = 75.0;

const LOSE_HEIGHT: f32 = -CUP_HEIGHT / 2.0 - CUP_THICKNESS - BALL_RADIUS;

fn setup(mut commands: Commands, textures: Res<Textures>) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(0.0, 20.0, 1.0),
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical(CUP_HEIGHT * 1.5),
            ..default()
        },
        ..default()
    });
    /*
     * Ground
     */

    let mut wall = |transform| {
        commands.spawn((
            SpriteBundle {
                transform,
                sprite: Sprite {
                    color: Color::rgb(0.69, 0.61, 0.85),
                    ..default()
                },
                ..default()
            },
            Collider::cuboid(0.5, 0.5),
        ));
    };

    wall(
        Transform::from_xyz(0.0, (-CUP_HEIGHT + CUP_THICKNESS) / 2.0, 0.0).with_scale(Vec3::new(
            CUP_WIDTH,
            CUP_THICKNESS,
            0.0,
        )),
    );
    wall(
        Transform::from_xyz((CUP_WIDTH - CUP_THICKNESS) / 2.0, 0.0, 0.0).with_scale(Vec3::new(
            CUP_THICKNESS,
            CUP_HEIGHT,
            0.0,
        )),
    );
    wall(
        Transform::from_xyz((-CUP_WIDTH + CUP_THICKNESS) / 2.0, 0.0, 0.0).with_scale(Vec3::new(
            CUP_THICKNESS,
            CUP_HEIGHT,
            0.0,
        )),
    );

    commands.spawn(SpriteBundle {
        transform: Transform::from_xyz(0.0, LOSE_HEIGHT, 0.0).with_scale(Vec3::new(
            CUP_WIDTH * 10.0,
            CUP_THICKNESS,
            0.0,
        )),
        sprite: Sprite {
            color: Color::rgb(0.85, 0.0, 0.0),
            ..default()
        },
        ..default()
    });

    let ball = Ball(0);
    let next = commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(2.0)),
                    ..default()
                },
                texture: textures.balls[ball.0 as usize % textures.balls.len()].clone(),
                transform: Transform::from_xyz(
                    CUP_WIDTH / 2.0 + 100.0,
                    CUP_HEIGHT / 2.0 + 25.0,
                    0.0,
                )
                .with_scale(Vec3::splat(ball.radius())),
                ..default()
            },
            ball,
        ))
        .id();

    commands
        .spawn((
            Controlled {
                current_ball: None,
                next_ball: next,
                ball_cooldown: 0.0,
                pog: None,
            },
            SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(150.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, CUP_HEIGHT / 2.0 + 100.0, -0.1)
                    .with_scale(Vec3::splat(1.0)),
                texture: textures.nor_lud.clone(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn(SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(2.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, -FACE_DOWN, 0.05)
                        .with_scale(Vec3::splat(1.0)),
                    texture: textures.hands_b.clone(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(Vec2::splat(2.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, 0.0, 0.15),
                        texture: textures.hands_f.clone(),
                        ..default()
                    });
                });
            parent.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgb(0.4, 0.37, 0.4),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 0.0, -1.0)
                        .with_scale(Vec3::new(5.0, 0.0, 1.0)),
                    ..default()
                },
                Laser,
            ));
        });
}

#[derive(Component)]
struct ScoreText;

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn((
        TextBundle::from_section(
            "Score: 0",
            TextStyle {
                font: font.clone(),
                font_size: 100.0,
                color: Color::WHITE,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(15.0),
            left: Val::Px(15.0),
            ..default()
        }),
        ScoreText,
    ));

    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::End,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(
                TextBundle::from_section(
                    "Move with 'A', 'D'.   Drop with 'Space'.   Swap ball with 'R'.",
                    TextStyle {
                        font: font.clone(),
                        font_size: 50.0,
                        color: Color::WHITE,
                    },
                )
                .with_text_alignment(TextAlignment::Center),
            );
        });
}

fn update_score_text(mut score_text: Query<&mut Text, With<ScoreText>>, score: Res<Score>) {
    if score.is_changed() {
        for mut text in score_text.iter_mut() {
            text.sections[0].value = format!("Score: {}", score.0);
        }
    }
}

#[derive(Component)]
struct Laser;

fn laser_system(
    mut lasers: Query<(&mut Transform, &Parent), With<Laser>>,
    transform: Query<&Transform, Without<Laser>>,
    ctx: Res<RapierContext>,
) {
    for (mut t, parent) in lasers.iter_mut() {
        if let Ok(gt) = transform.get(parent.get()) {
            let dir = gt.down().truncate();
            let pos = gt.translation.truncate();

            let max = CUP_HEIGHT * 2.0;

            let l = ctx
                .cast_ray(pos, dir, max, true, QueryFilter::new())
                .map_or(max, |(_, l)| l)
                + BALL_RADIUS / 2.0;

            t.scale.y = l;
            t.translation.y = -l / 2.0;
        }
    }
}

fn control_ball(
    mut commands: Commands,
    mut balls: Query<&mut Ball>,
    mut controlled: Query<(
        &Children,
        &mut Transform,
        &mut Controlled,
        &mut Handle<Image>,
    )>,
    mut transforms: Query<(&mut Transform, &mut Handle<Image>), Without<Controlled>>,
    input: Res<Input<KeyCode>>,
    time: Res<Time>,
    textures: Res<Textures>,
    loss: Res<Loss>,
) {
    if loss.0 {
        return;
    }

    let (children, mut transform, mut controlled, mut image) = controlled.single_mut();
    if let Some(ball_e) = controlled.current_ball {
        if let Ok((mut t, _)) = transforms.get_mut(ball_e) {
            t.translation.x = transform.translation.x;
        }
        if input.just_pressed(KeyCode::Space) {
            commands.entity(ball_e).insert((
                RigidBody::Dynamic,
                Damping {
                    linear_damping: 2.0,
                    angular_damping: 1.0,
                },
                GravityScale(3.0),
                Ccd::enabled(),
                Collider::ball(1.0),
                ActiveEvents::COLLISION_EVENTS,
            ));

            controlled.current_ball = None;
        } else if input.just_pressed(KeyCode::R) {
            let ball = balls.get(controlled.next_ball).unwrap();
            if let Ok((mut transform, _)) = transforms.get_mut(children[0]) {
                transform.scale = Vec2::splat(ball.radius()).extend(1.0);
            }

            let [(mut a, _), (mut b, _)] = transforms
                .get_many_mut([ball_e, controlled.next_ball])
                .unwrap();
            std::mem::swap(&mut a.translation, &mut b.translation);
            controlled.current_ball = Some(controlled.next_ball);
            controlled.next_ball = ball_e;
        }
    } else {
        controlled.ball_cooldown += time.delta_seconds();
        *image = textures.con_lud.clone();
        if controlled.ball_cooldown >= 1.0 {
            controlled.ball_cooldown = 0.0;
            let mut ball = balls.get_mut(controlled.next_ball).unwrap();

            if let Ok((mut transform, _)) = transforms.get_mut(children[0]) {
                transform.scale = Vec2::splat(ball.radius()).extend(1.0);
            }
            controlled.current_ball = Some(
                commands
                    .spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                custom_size: Some(Vec2::splat(2.0)),
                                ..default()
                            },
                            texture: textures.balls[ball.0 as usize % textures.balls.len()].clone(),
                            transform: Transform::from_xyz(
                                transform.translation.x,
                                transform.translation.y - FACE_DOWN,
                                0.0,
                            )
                            .with_scale(Vec3::splat(ball.radius())),
                            ..default()
                        },
                        *ball,
                    ))
                    .id(),
            );

            ball.0 = random(&time, 0..=2);
            if let Ok((mut transform, mut image)) = transforms.get_mut(controlled.next_ball) {
                *image = textures.balls[ball.0 as usize % textures.balls.len()].clone();
                transform.scale = Vec3::splat(ball.radius());
            }
        }
    }

    if let Some(t) = &mut controlled.pog {
        *t -= time.delta_seconds();
        if *t <= 0.0 {
            controlled.pog = None;
            *image = textures.nor_lud.clone();
        } else {
            *image = textures.pog_lud.clone();
        }
    }

    let mov = input.pressed(KeyCode::D) as i32 - input.pressed(KeyCode::A) as i32;
    if mov != 0 {
        transform.translation.x += mov as f32 * time.delta_seconds() * 8.0 * BALL_RADIUS;
    }
}

fn random(time: &Time, range: RangeInclusive<u32>) -> u32 {
    let t = (time.raw_elapsed_seconds_f64() * 14234234.123142354).sin() * 0.5 + 0.5;

    range.start() + ((range.end() + 1 - range.start()) as f64 * t) as u32
}

fn merge_balls(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut score: ResMut<Score>,
    mut controlled: Query<&mut Controlled>,
    balls: Query<(Entity, &Ball, &Transform), With<RigidBody>>,
    textures: Res<Textures>,
) {
    let mut controlled = controlled.single_mut();
    let mut handled = HashSet::new();
    let mut to_spawn = Vec::new();
    for (a, b) in collision_events.iter().filter_map(|ev| {
        if let CollisionEvent::Started(a, b, _) = ev {
            Some((*a, *b))
        } else {
            None
        }
    }) {
        if handled.contains(&a) || handled.contains(&b) {
            continue;
        }
        let Ok([(_, ball_a, transform_a), (_, ball_b, transform_b)]) = balls.get_many([a, b])
        else {
            continue;
        };

        if ball_a == ball_b {
            commands.entity(a).despawn();
            commands.entity(b).despawn();
            handled.insert(a);
            handled.insert(b);

            to_spawn.push((
                ball_a.next(),
                (transform_a.translation + transform_b.translation) / 2.0,
                Quat::lerp(transform_a.rotation, transform_b.rotation, 0.5),
            ));
        }
    }

    if to_spawn.len() > 0 {
        let t = controlled.pog.get_or_insert(0.0);
        *t += to_spawn.len() as f32;
    }

    for (ball, mut trans, rot) in to_spawn {
        let x_clamp = (CUP_WIDTH - CUP_THICKNESS) / 2.0 - ball.radius();
        let y_min = (-CUP_HEIGHT + CUP_THICKNESS) / 2.0 + ball.radius();
        trans.x = trans.x.clamp(-x_clamp, x_clamp);
        trans.y = trans.y.max(y_min);
        for _ in 0..30 {
            let mut corrected = false;
            for (_, b, t) in balls
                .iter()
                .filter(|(e, b, _)| ball != **b && !handled.contains(e))
            {
                let r = b.radius() + ball.radius();
                let d = trans.distance_squared(t.translation);
                if d < r * r {
                    trans = t.translation + (trans - t.translation).normalize_or_zero() * r;
                    trans.x = trans.x.clamp(-x_clamp, x_clamp);
                    trans.y = trans.y.max(y_min);
                    corrected = true;
                }
            }
            if !corrected {
                break;
            }
        }
        score.0 += ball.score();
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(2.0)),
                    ..default()
                },
                texture: textures.balls[ball.0 as usize % textures.balls.len()].clone(),
                transform: Transform::from_translation(trans)
                    .with_rotation(rot)
                    .with_scale(Vec3::splat(ball.radius())),
                ..default()
            },
            ball,
            RigidBody::Dynamic,
            Damping {
                linear_damping: 2.0,
                angular_damping: 1.0,
            },
            Ccd::enabled(),
            GravityScale(3.0),
            Collider::ball(1.0),
            ActiveEvents::COLLISION_EVENTS,
        ));
    }
}
#[derive(Resource, Default)]
struct Loss(bool);

fn check_for_loss(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut loss: ResMut<Loss>,
    mut score: ResMut<Score>,
    balls: Query<(Entity, &Transform), With<RigidBody>>,
    input: Res<Input<KeyCode>>,
    mut loss_screen: Local<Option<Entity>>,
) {
    if loss.0 {
        if input.just_pressed(KeyCode::Space) {
            for (e, _) in balls.iter() {
                commands.entity(e).despawn_recursive();
            }
            if let Some(e) = *loss_screen {
                commands.entity(e).despawn_recursive();
                *loss_screen = None;
            }
            score.0 = 0;
            loss.0 = false;
        }
    } else {
        for (_, transform) in balls.iter() {
            if transform.translation.y < LOSE_HEIGHT {
                loss.0 = true;
            }
        }
        if loss.0 {
            *loss_screen = Some(
                commands
                    .spawn((NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.9)),
                        ..default()
                    },))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "You Lose!\nPress 'Space' to restart",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 70.0,
                                color: Color::WHITE,
                            },
                        ));
                    })
                    .id(),
            );
        }
    }
}
