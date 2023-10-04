#![feature(iter_collect_into)]
use bevy::{ecs::schedule::ScheduleLabel, prelude::*, transform::TransformSystem, utils::HashSet};
use bevy_rapier2d::prelude::*;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Physics;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(
            0x33 as f32 / 255.0,
            0x2F as f32 / 255.0,
            0x33 as f32 / 255.0,
        )))
        .init_resource::<Simulated>()
        .add_plugins((
            DefaultPlugins,
            RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),
            // RapierDebugRenderPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (control_ball, merge_balls))
        .init_schedule(Physics)
        .add_systems(
            Physics,
            (
                RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::SyncBackend)
                    .in_set(PhysicsSet::SyncBackend),
                RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::StepSimulation)
                    .in_set(PhysicsSet::StepSimulation),
                RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::Writeback)
                    .in_set(PhysicsSet::Writeback),
                merge_balls,
                (
                    bevy::transform::systems::sync_simple_transforms,
                    bevy::transform::systems::propagate_transforms,
                )
                    .in_set(RapierTransformPropagateSet),
            ),
        )
        .add_systems(
            PostUpdate,
            simulate
                .after(PhysicsSet::Writeback)
                .before(TransformSystem::TransformPropagate),
        )
        .run();
}

fn simulate(world: &mut World) {
    const MAX_STEPS: u32 = 100;
    let mut steps = 0;
    let mut add_vec = Vec::new();
    while world.resource::<Simulated>().0.len() > 0 && steps < MAX_STEPS {
        world
            .query_filtered::<Entity, (With<RigidBody>, Without<RigidBodyDisabled>)>()
            .iter(world)
            .filter(|e| !world.resource::<Simulated>().0.contains(e))
            .map(|e| (e, RigidBodyDisabled))
            .collect_into(&mut add_vec);
        let _ = world.insert_or_spawn_batch(add_vec.iter().copied());
        add_vec.clear();
        world.run_schedule(Physics);
        steps += 1;
    }
    if steps > 0 {
        info!("Simulated {steps} physics steps");

        let remove = world
            .query_filtered::<Entity, With<RigidBodyDisabled>>()
            .iter(world)
            .collect::<Vec<_>>();
        for e in remove {
            world.entity_mut(e).remove::<RigidBodyDisabled>();
        }
    }
}

#[derive(Component, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct Ball(u32);

impl Ball {
    fn next(&self) -> Self {
        Ball(self.0 + 1)
    }

    fn scale(&self) -> f32 {
        1.0 + 0.5 * self.0 as f32
    }
}

#[derive(Component)]
struct ControlledBall;

const TEXTURES: &'static [&'static str] = &[
    "coots.png",
    "ders.png",
    "swift.png",
    "lud.png",
    "qt.png",
    "slime.png",
];

const CUP_WIDTH: f32 = 500.0;
const CUP_HEIGHT: f32 = 500.0;
const CUP_THICKNESS: f32 = 10.0;
const BALL_RADIUS: f32 = 12.0;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(0.0, 20.0, 0.0),
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

    spawn_controlled_ball(&mut commands, &asset_server, 0);
}

fn spawn_controlled_ball(commands: &mut Commands, asset_server: &AssetServer, size: u32) {
    let ball = Ball(size);
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::splat(BALL_RADIUS * 2.0)),
                ..default()
            },
            texture: asset_server.load(TEXTURES[size as usize % TEXTURES.len()]),
            transform: Transform::from_xyz(0.0, CUP_HEIGHT / 2.0 + 25.0, 0.0)
                .with_scale(Vec3::splat(ball.scale())),
            ..default()
        },
        ball,
        ControlledBall,
    ));
}

fn control_ball(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut balls: Query<(Entity, &mut Transform), With<ControlledBall>>,
    input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    if input.just_pressed(KeyCode::Space) {
        for (e, _) in balls.iter() {
            commands.entity(e).remove::<ControlledBall>().insert((
                RigidBody::Dynamic,
                Damping {
                    linear_damping: 2.0,
                    angular_damping: 1.0,
                },
                GravityScale(3.0),
                Collider::ball(BALL_RADIUS),
                ActiveEvents::COLLISION_EVENTS,
            ));
        }

        spawn_controlled_ball(&mut commands, &asset_server, 0);
    } else {
        let mov = (input.pressed(KeyCode::D) as i32 - input.pressed(KeyCode::A) as i32) as f32
            * time.delta_seconds()
            * 8.0
            * BALL_RADIUS;
        for (_, mut transform) in balls.iter_mut() {
            transform.translation.x += mov;
            transform.translation.x = transform
                .translation
                .x
                .clamp(-CUP_WIDTH / 2.0, CUP_WIDTH / 2.0);
        }
    }
}

#[derive(Resource, Default)]
struct Simulated(HashSet<Entity>);

fn merge_balls(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut simulated: ResMut<Simulated>,
    mut collision_events: EventReader<CollisionEvent>,
    balls: Query<(&Ball, &Transform)>,
) {
    let mut handled = HashSet::new();
    let mut new_simulated = HashSet::new();
    for (a, b) in collision_events.iter().filter_map(|ev| {
        if let CollisionEvent::Started(a, b, _) = ev {
            Some((*a, *b))
        } else {
            None
        }
    }) {
        if handled.contains(&a) || handled.contains(&b) {
            if simulated.0.contains(&a) {
                new_simulated.insert(a);
            }
            if simulated.0.contains(&b) {
                new_simulated.insert(b);
            }
            continue;
        }
        let Ok([(ball_a, transform_a), (ball_b, transform_b)]) = balls.get_many([a, b]) else {
            continue;
        };

        if ball_a == ball_b {
            commands.entity(a).despawn();
            commands.entity(b).despawn();
            handled.insert(a);
            handled.insert(b);

            let ball = ball_a.next();
            let e = commands
                .spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(Vec2::splat(BALL_RADIUS * 2.0)),
                            ..default()
                        },
                        texture: asset_server.load(TEXTURES[ball.0 as usize % TEXTURES.len()]),
                        transform: Transform::from_translation(
                            (transform_a.translation + transform_b.translation) / 2.0,
                        )
                        .with_rotation(Quat::lerp(transform_a.rotation, transform_b.rotation, 0.5))
                        .with_scale(Vec3::splat(ball.scale())),
                        ..default()
                    },
                    ball,
                    RigidBody::Dynamic,
                    Damping {
                        linear_damping: 2.0,
                        angular_damping: 1.0,
                    },
                    GravityScale(3.0),
                    Collider::ball(BALL_RADIUS),
                    ActiveEvents::COLLISION_EVENTS,
                ))
                .id();

            new_simulated.insert(e);
        } else {
            if simulated.0.contains(&a) {
                new_simulated.insert(a);
            }
            if simulated.0.contains(&b) {
                new_simulated.insert(b);
            }
        }
    }

    simulated.0 = new_simulated;
}
